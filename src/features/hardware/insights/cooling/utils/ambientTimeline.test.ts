import { describe, expect, it } from "vitest";
import type { AmbientArchiveSeries } from "@/rspc/bindings";
import {
  buildAmbientLaneRows,
  computeAmbientDomain,
  namedAmbientSources,
  resolveRoutedAmbientCapability,
} from "./ambientTimeline";
import type {
  ArchiveTimelineSeries,
  ThermalTimelineRow,
} from "./thermalTimeline";

const EMPTY_ROW = {
  temperatureAvg: null,
  temperatureMin: null,
  temperatureMax: null,
  temperatureRange: null,
  idleTemperature: null,
  cpuUsage: null,
  loadIdle: null,
  loadLow: null,
  loadMid: null,
  loadHigh: null,
  powerAvg: null,
  powerMin: null,
  powerMax: null,
  powerRange: null,
} as const satisfies Omit<ThermalTimelineRow, "key" | "label">;

const row = (key: string): ThermalTimelineRow => ({
  ...EMPTY_ROW,
  key,
  label: key,
});

const EMPTY_CPU_SERIES: ArchiveTimelineSeries = {
  temperatureAvg: [],
  temperatureMax: [],
  temperatureMin: [],
  cpuUsage: [],
  powerAvg: [],
  powerMax: [],
  powerMin: [],
};

const RECORDED_CPU_SERIES: ArchiveTimelineSeries = {
  ...EMPTY_CPU_SERIES,
  temperatureAvg: [{ timestamp: 0, value: 50 }],
};

const series = (
  overrides: Partial<AmbientArchiveSeries> = {},
): AmbientArchiveSeries => ({
  sources: ["Living room"],
  buckets: [{ timestamp: 0, ambientAvg: 22, deltaAvg: 28 }],
  ...overrides,
});

describe("buildAmbientLaneRows", () => {
  it("projects buckets onto the timeline's own keys and labels", () => {
    // Driven by the shared rows, so the lane cannot drift out of step with
    // the lanes above it even if the ambient archive returned other buckets.
    const rows = buildAmbientLaneRows(
      [row("0"), row("60000")],
      series({
        buckets: [{ timestamp: 60_000, ambientAvg: 22, deltaAvg: 28 }],
      }),
      "C",
    );

    expect(rows).toEqual([
      { key: "0", label: "0", ambient: null, delta: null },
      { key: "60000", label: "60000", ambient: 22, delta: 28 },
    ]);
  });

  it("converts the ambient point with the offset and the delta without it", () => {
    const rows = buildAmbientLaneRows([row("0")], series(), "F");

    // 22 degC is 71.6 degF; a 28 K span is 50.4 R, not 82.4.
    expect(rows[0].ambient).toBeCloseTo(71.6);
    expect(rows[0].delta).toBeCloseTo(50.4);
  });

  it("keeps a bucket that recorded ambient but no pairing at a null delta", () => {
    // A minute with an ambient row and no CPU package temperature yields no
    // ΔT at all - never a delta standing in for one (DP-02).
    const rows = buildAmbientLaneRows(
      [row("0")],
      series({ buckets: [{ timestamp: 0, ambientAvg: 22, deltaAvg: null }] }),
      "C",
    );

    expect(rows[0]).toEqual({
      key: "0",
      label: "0",
      ambient: 22,
      delta: null,
    });
  });

  it("leaves every row null on a route with no ambient series", () => {
    const rows = buildAmbientLaneRows([row("0"), row("60000")], null, "C");

    expect(rows.every((entry) => entry.ambient == null)).toBe(true);
    expect(computeAmbientDomain(rows)).toBeNull();
  });
});

describe("computeAmbientDomain", () => {
  it("follows the data rather than anchoring at zero", () => {
    // A room sits in a narrow band well above zero; a 0-anchored axis would
    // flatten the movement the lane exists to show.
    const domain = computeAmbientDomain([
      { key: "0", label: "0", ambient: 21, delta: 28 },
      { key: "1", label: "1", ambient: 27, delta: 28 },
    ]);

    expect(domain).not.toBeNull();
    expect(domain?.[0]).toBeGreaterThan(0);
  });

  it("keeps a sub-zero window in ascending order", () => {
    // A garage or an unheated room in winter. Clamping the lower bound at
    // zero the way the CPU temperature lane does would answer [0, -3] here
    // - a descending domain, which renders as a broken axis.
    const domain = computeAmbientDomain([
      { key: "0", label: "0", ambient: -5, delta: 44 },
    ]);

    expect(domain).toEqual([-7, -3]);
  });

  it("keeps a window that crosses freezing in order", () => {
    const domain = computeAmbientDomain([
      { key: "0", label: "0", ambient: -3, delta: 44 },
      { key: "1", label: "1", ambient: 6, delta: 38 },
    ]);

    expect(domain).toEqual([-5, 8]);
  });

  it("closes the lane's gate when nothing recorded ambient", () => {
    expect(
      computeAmbientDomain([
        { key: "0", label: "0", ambient: null, delta: 28 },
      ]),
    ).toBeNull();
  });
});

describe("resolveRoutedAmbientCapability", () => {
  const archiveRoute = { kind: "archive" } as const;

  it("reports present once the window carries an ambient reading", () => {
    expect(
      resolveRoutedAmbientCapability(archiveRoute, {
        ambientSeries: series(),
        cpuSeries: RECORDED_CPU_SERIES,
        hasLoaded: true,
        hasError: false,
        ambientHasError: false,
      }),
    ).toBe("present");
  });

  it("reports absent when the window recorded something and no ambient", () => {
    expect(
      resolveRoutedAmbientCapability(archiveRoute, {
        ambientSeries: { sources: [], buckets: [] },
        cpuSeries: RECORDED_CPU_SERIES,
        hasLoaded: true,
        hasError: false,
        ambientHasError: false,
      }),
    ).toBe("absent");
  });

  it("stays unknown when the window recorded nothing at all", () => {
    // The app simply was not running; that is not evidence about sensors.
    expect(
      resolveRoutedAmbientCapability(archiveRoute, {
        ambientSeries: { sources: [], buckets: [] },
        cpuSeries: EMPTY_CPU_SERIES,
        hasLoaded: true,
        hasError: false,
        ambientHasError: false,
      }),
    ).toBe("unknown");
  });

  it("stays unknown while the fetch is still in flight", () => {
    expect(
      resolveRoutedAmbientCapability(archiveRoute, {
        ambientSeries: null,
        cpuSeries: RECORDED_CPU_SERIES,
        hasLoaded: false,
        hasError: false,
        ambientHasError: false,
      }),
    ).toBe("unknown");
  });

  it("stays unknown when only the ambient read failed", () => {
    expect(
      resolveRoutedAmbientCapability(archiveRoute, {
        ambientSeries: null,
        cpuSeries: RECORDED_CPU_SERIES,
        hasLoaded: true,
        hasError: false,
        ambientHasError: true,
      }),
    ).toBe("unknown");
  });

  it("stays unknown on the long-range routes, which have no ambient source", () => {
    // The daily rollup stores the per-band ΔT and an ambient coverage count
    // but no ambient temperature, so a 90-day window can neither draw the
    // lane nor prove that no sensor exists.
    expect(
      resolveRoutedAmbientCapability(
        { kind: "dailyTrend" },
        {
          ambientSeries: series(),
          cpuSeries: RECORDED_CPU_SERIES,
          hasLoaded: true,
          hasError: false,
          ambientHasError: false,
        },
      ),
    ).toBe("unknown");
  });

  it("treats a bucketed window with only null ambient values as absent", () => {
    expect(
      resolveRoutedAmbientCapability(archiveRoute, {
        ambientSeries: {
          sources: [],
          buckets: [{ timestamp: 0, ambientAvg: null, deltaAvg: null }],
        },
        cpuSeries: RECORDED_CPU_SERIES,
        hasLoaded: true,
        hasError: false,
        ambientHasError: false,
      }),
    ).toBe("absent");
  });
});

describe("namedAmbientSources", () => {
  it("names the window's sources once ambient is present", () => {
    expect(namedAmbientSources("present", series())).toEqual(["Living room"]);
  });

  it("names nothing while the answer is unknown", () => {
    // Under-claims rather than telling a user whose sensor is still loading
    // that their window has none.
    expect(namedAmbientSources("unknown", series())).toEqual([]);
    expect(namedAmbientSources("absent", series())).toEqual([]);
  });
});
