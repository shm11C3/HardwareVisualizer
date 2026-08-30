import { describe, expect, it } from "vitest";
import type {
  CoolingBandTemperature,
  CoolingDailyTrendPoint,
} from "@/rspc/bindings";
import { buildCoverageCells } from "./coverageStrip";

const EMPTY_BAND: CoolingBandTemperature = {
  avg: null,
  max: null,
  min: null,
  sampleMinutes: 0,
};

const trendPoint = (
  date: string,
  coverageMinutes: number,
): CoolingDailyTrendPoint => ({
  date,
  coverageMinutes,
  idle: EMPTY_BAND,
  low: EMPTY_BAND,
  mid: EMPTY_BAND,
  high: EMPTY_BAND,
  power: { avg: null, max: null, min: null, sampleMinutes: 0 },
});

describe("buildCoverageCells", () => {
  it("returns no cells when the backend returned no points - there is no anchor to fabricate a window from", () => {
    expect(buildCoverageCells([], 3)).toEqual([]);
  });

  it("anchors the trailing window to the latest returned day, oldest first", () => {
    const points = [
      trendPoint("2026-01-13", 1440),
      trendPoint("2026-01-15", 1440),
    ];

    const cells = buildCoverageCells(points, 3);

    expect(cells.map((cell) => cell.date)).toEqual([
      "2026-01-13",
      "2026-01-14",
      "2026-01-15",
    ]);
  });

  it("never extends the window past the latest summarized day onto the frontend's own clock", () => {
    // Core ends its window on yesterday (local); a strip built through
    // "today" would add a false zero cell and drop the oldest day.
    const points = [trendPoint("2026-01-14", 1440)];

    const cells = buildCoverageCells(points, 2);

    expect(cells.map((cell) => cell.date)).toEqual([
      "2026-01-13",
      "2026-01-14",
    ]);
  });

  it("fills a day absent from the rollup with zero coverage instead of dropping it", () => {
    const points = [
      trendPoint("2026-01-13", 1440),
      trendPoint("2026-01-15", 1440),
    ];

    const cells = buildCoverageCells(points, 3);

    expect(cells).toEqual([
      { date: "2026-01-13", coverageRatio: 1 },
      { date: "2026-01-14", coverageRatio: 0 },
      { date: "2026-01-15", coverageRatio: 1 },
    ]);
  });

  it("scales partial-day coverage as a fraction of a full day", () => {
    const points = [trendPoint("2026-01-15", 360)];

    const cells = buildCoverageCells(points, 1);

    expect(cells[0].coverageRatio).toBeCloseTo(0.25);
  });

  it("caps coverage at 1 even if the rollup reports more than a full day", () => {
    const points = [trendPoint("2026-01-15", 2000)];

    const cells = buildCoverageCells(points, 1);

    expect(cells[0].coverageRatio).toBe(1);
  });

  it("ignores rollup rows older than the requested window", () => {
    const points = [
      trendPoint("2020-01-01", 1440),
      trendPoint("2026-01-15", 720),
    ];

    const cells = buildCoverageCells(points, 1);

    expect(cells).toEqual([{ date: "2026-01-15", coverageRatio: 0.5 }]);
  });
});
