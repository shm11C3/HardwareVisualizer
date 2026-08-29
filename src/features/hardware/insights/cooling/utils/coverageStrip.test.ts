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
});

describe("buildCoverageCells", () => {
  const referenceDate = new Date("2026-01-15T12:00:00Z");

  it("builds one cell per day in the trailing window, oldest first", () => {
    const cells = buildCoverageCells([], 3, referenceDate);

    expect(cells.map((cell) => cell.date)).toEqual([
      "2026-01-13",
      "2026-01-14",
      "2026-01-15",
    ]);
  });

  it("fills a day absent from the rollup with zero coverage instead of dropping it", () => {
    const points = [trendPoint("2026-01-14", 1440)];

    const cells = buildCoverageCells(points, 3, referenceDate);

    expect(cells).toEqual([
      { date: "2026-01-13", coverageRatio: 0 },
      { date: "2026-01-14", coverageRatio: 1 },
      { date: "2026-01-15", coverageRatio: 0 },
    ]);
  });

  it("scales partial-day coverage as a fraction of a full day", () => {
    const points = [trendPoint("2026-01-15", 360)];

    const cells = buildCoverageCells(points, 1, referenceDate);

    expect(cells[0].coverageRatio).toBeCloseTo(0.25);
  });

  it("caps coverage at 1 even if the rollup reports more than a full day", () => {
    const points = [trendPoint("2026-01-15", 2000)];

    const cells = buildCoverageCells(points, 1, referenceDate);

    expect(cells[0].coverageRatio).toBe(1);
  });

  it("ignores rollup rows outside the requested window", () => {
    const points = [trendPoint("2020-01-01", 1440)];

    const cells = buildCoverageCells(points, 1, referenceDate);

    expect(cells).toEqual([{ date: "2026-01-15", coverageRatio: 0 }]);
  });
});
