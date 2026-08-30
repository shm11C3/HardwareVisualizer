import { describe, expect, it } from "vitest";
import type {
  CoolingBandMedianDelta,
  CoolingExplorerWindow,
} from "@/rspc/bindings";
import {
  bandMedianPositions,
  buildExplorerBandDeltaRows,
  buildExplorerMedianTrend,
  buildExplorerMinimapSegments,
  buildExplorerScatterPoints,
  cpuLoadBandDividers,
  defaultExplorerRecentDays,
  explorerRecentDayPresets,
  isExplorerRecentDays,
} from "./loadTemperatureExplorer";

const window = (
  overrides: Partial<CoolingExplorerWindow> = {},
): CoolingExplorerWindow => ({
  startDate: "2026-08-14",
  endDate: "2026-08-20",
  points: [
    {
      hourStart: "2026-08-20 09:00",
      cpuUsageAvg: 5,
      cpuTemperatureAvg: 40,
      sampleMinutes: 60,
    },
  ],
  ...overrides,
});

const delta = (
  overrides: Partial<CoolingBandMedianDelta> = {},
): CoolingBandMedianDelta => ({
  band: "idle",
  baseline: { temperatureMedian: 32, pointCount: 12, sampleMinutes: 720 },
  recent: { temperatureMedian: 33.5, pointCount: 20, sampleMinutes: 1_200 },
  delta: 1.5,
  comparable: true,
  ...overrides,
});

describe("explorer recent-day presets", () => {
  it("stays inside the range Core clamps to, so no preset is silently narrowed", () => {
    expect(Math.min(...explorerRecentDayPresets)).toBe(7);
    expect(Math.max(...explorerRecentDayPresets)).toBe(90);
  });

  it("defaults to a preset that is actually offered", () => {
    expect(explorerRecentDayPresets).toContain(defaultExplorerRecentDays);
  });

  it("recognizes only the offered presets", () => {
    expect(isExplorerRecentDays(28)).toBe(true);
    expect(isExplorerRecentDays(30)).toBe(false);
    expect(isExplorerRecentDays("28")).toBe(false);
    expect(isExplorerRecentDays(null)).toBe(false);
  });
});

describe("band geometry", () => {
  it("uses the same dividers Core classifies load bands on", () => {
    expect(cpuLoadBandDividers).toEqual([10, 30, 60]);
  });

  it("places each band's median at the center of that band's usage range", () => {
    expect(bandMedianPositions.idle).toBe(5);
    expect(bandMedianPositions.low).toBe(20);
    expect(bandMedianPositions.mid).toBe(45);
    expect(bandMedianPositions.high).toBe(80);
  });
});

describe("buildExplorerScatterPoints", () => {
  it("maps each hour onto a load/temperature coordinate", () => {
    expect(buildExplorerScatterPoints(window(), "C")).toEqual([
      { hourStart: "2026-08-20 09:00", x: 5, y: 40, sampleMinutes: 60 },
    ]);
  });

  it("converts the temperature axis but never the load axis", () => {
    const [point] = buildExplorerScatterPoints(window(), "F");

    expect(point.y).toBeCloseTo(40 * 1.8 + 32);
    expect(point.x).toBe(5);
  });

  it("returns no points for a window that has not loaded yet", () => {
    expect(buildExplorerScatterPoints(null, "C")).toEqual([]);
  });

  it("returns no points for an established but empty window", () => {
    expect(buildExplorerScatterPoints(window({ points: [] }), "C")).toEqual([]);
  });
});

describe("buildExplorerMedianTrend", () => {
  it("positions each side's medians across the bands in order", () => {
    const trend = buildExplorerMedianTrend(
      [
        delta({ band: "idle" }),
        delta({ band: "low" }),
        delta({ band: "mid" }),
        delta({ band: "high" }),
      ],
      "recent",
      "C",
    );

    expect(trend.map((point) => point.x)).toEqual([5, 20, 45, 80]);
    expect(trend.every((point) => point.y === 33.5)).toBe(true);
  });

  it("reads the requested side rather than blending the two", () => {
    const [point] = buildExplorerMedianTrend([delta()], "baseline", "C");

    expect(point.y).toBe(32);
  });

  it("omits a band with no median instead of dropping the line to zero", () => {
    const trend = buildExplorerMedianTrend(
      [
        delta({ band: "idle" }),
        delta({
          band: "low",
          recent: { temperatureMedian: null, pointCount: 0, sampleMinutes: 0 },
        }),
        delta({ band: "mid" }),
      ],
      "recent",
      "C",
    );

    expect(trend.map((point) => point.band)).toEqual(["idle", "mid"]);
  });
});

describe("buildExplorerBandDeltaRows", () => {
  it("converts a comparable band's medians and delta in Celsius", () => {
    const [row] = buildExplorerBandDeltaRows([delta()], "C");

    expect(row).toEqual({
      band: "idle",
      comparable: true,
      baseline: 32,
      recent: 33.5,
      delta: 1.5,
      baselinePointCount: 12,
      recentPointCount: 20,
    });
  });

  it("scales the delta by 9/5 with no +32 offset in Fahrenheit", () => {
    const [row] = buildExplorerBandDeltaRows([delta()], "F");

    if (!row.comparable) {
      throw new Error("expected a comparable row");
    }
    expect(row.baseline).toBeCloseTo(32 * 1.8 + 32);
    expect(row.delta).toBeCloseTo(1.5 * 1.8);
  });

  it("marks a band Core reported as not comparable, keeping its point counts", () => {
    const [row] = buildExplorerBandDeltaRows(
      [
        delta({
          comparable: false,
          delta: null,
          recent: { temperatureMedian: null, pointCount: 2, sampleMinutes: 12 },
        }),
      ],
      "C",
    );

    expect(row).toEqual({
      band: "idle",
      comparable: false,
      baselinePointCount: 12,
      recentPointCount: 2,
    });
  });

  it("falls back to not-comparable if a median is missing despite the flag", () => {
    const [row] = buildExplorerBandDeltaRows(
      [
        delta({
          comparable: true,
          baseline: {
            temperatureMedian: null,
            pointCount: 0,
            sampleMinutes: 0,
          },
        }),
      ],
      "C",
    );

    expect(row.comparable).toBe(false);
  });

  it("preserves band order across multiple entries", () => {
    const rows = buildExplorerBandDeltaRows(
      [
        delta({ band: "idle" }),
        delta({ band: "low" }),
        delta({ band: "high" }),
      ],
      "C",
    );

    expect(rows.map((row) => row.band)).toEqual(["idle", "low", "high"]);
  });
});

describe("buildExplorerMinimapSegments", () => {
  const baseline = { startDate: "2026-01-01", endDate: "2026-01-07" };
  const recent = { startDate: "2026-07-25", endDate: "2026-08-20" };

  it("anchors the earlier window at the left edge and the later one at the right", () => {
    const [baselineSegment, recentSegment] = buildExplorerMinimapSegments(
      baseline,
      recent,
    );

    expect(baselineSegment.kind).toBe("baseline");
    expect(baselineSegment.offsetPercent).toBe(0);
    expect(recentSegment.kind).toBe("recent");
    expect(
      recentSegment.offsetPercent + recentSegment.widthPercent,
    ).toBeCloseTo(100);
  });

  it("keeps a very short window visible rather than collapsing it to nothing", () => {
    const [baselineSegment] = buildExplorerMinimapSegments(
      { startDate: "2026-01-01", endDate: "2026-01-01" },
      recent,
    );

    expect(baselineSegment.widthPercent).toBeGreaterThan(0);
  });

  it("measures each window inclusively, so a 1-day window is a seventh of a 7-day one", () => {
    // A 10-day total span holding a 1-day window and a 7-day window: the
    // widths must be 1/10 and 7/10, not the 0/10 and 6/10 an exclusive
    // end-date subtraction would give.
    const [oneDay, sevenDay] = buildExplorerMinimapSegments(
      { startDate: "2026-01-01", endDate: "2026-01-01" },
      { startDate: "2026-01-04", endDate: "2026-01-10" },
    );

    expect(oneDay.widthPercent).toBeCloseTo(10);
    expect(sevenDay.widthPercent).toBeCloseTo(70);
    expect(sevenDay.offsetPercent).toBeCloseTo(30);
  });

  it("lays both windows across the full width when they share a single day", () => {
    const segments = buildExplorerMinimapSegments(
      { startDate: "2026-01-01", endDate: "2026-01-01" },
      { startDate: "2026-01-01", endDate: "2026-01-01" },
    );

    expect(segments.map((segment) => segment.widthPercent)).toEqual([100, 100]);
  });

  it("omits the minimap entirely when a window's dates cannot be read", () => {
    expect(
      buildExplorerMinimapSegments(
        { startDate: "not-a-date", endDate: "2026-01-07" },
        recent,
      ),
    ).toEqual([]);
  });

  it("carries each window's dates through for labeling", () => {
    const [baselineSegment, recentSegment] = buildExplorerMinimapSegments(
      baseline,
      recent,
    );

    expect(baselineSegment.startDate).toBe("2026-01-01");
    expect(recentSegment.endDate).toBe("2026-08-20");
  });
});
