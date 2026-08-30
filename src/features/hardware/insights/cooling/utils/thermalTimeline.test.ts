import { describe, expect, it } from "vitest";
import type {
  CoolingBandTemperature,
  CoolingDailyTrendPoint,
} from "@/rspc/bindings";
import {
  buildArchiveTimelineRows,
  buildDailyTimelineRows,
  collectTemperatureDomainValues,
  computeAdaptiveTemperatureDomain,
  resolveBaselineBand,
  type ThermalTimelineRow,
  toDisplayTemperature,
} from "./thermalTimeline";

const EMPTY_BAND: CoolingBandTemperature = {
  avg: null,
  max: null,
  min: null,
  sampleMinutes: 0,
};

const band = (
  avg: number,
  min: number,
  max: number,
  sampleMinutes: number,
): CoolingBandTemperature => ({ avg, min, max, sampleMinutes });

const trendPoint = (
  date: string,
  overrides: Partial<CoolingDailyTrendPoint> = {},
): CoolingDailyTrendPoint => ({
  date,
  coverageMinutes: 1440,
  idle: EMPTY_BAND,
  low: EMPTY_BAND,
  mid: EMPTY_BAND,
  high: EMPTY_BAND,
  ...overrides,
});

const identityLabel = (value: string) => value;

describe("toDisplayTemperature", () => {
  it("keeps Celsius values as recorded", () => {
    expect(toDisplayTemperature(42.25, "C")).toBe(42.3);
  });

  it("converts to Fahrenheit for display only", () => {
    expect(toDisplayTemperature(30, "F")).toBe(86);
  });

  it("keeps a missing reading missing instead of zeroing it", () => {
    expect(toDisplayTemperature(null, "C")).toBeNull();
  });
});

describe("computeAdaptiveTemperatureDomain", () => {
  it("returns null when nothing was recorded", () => {
    expect(computeAdaptiveTemperatureDomain([null, null])).toBeNull();
  });

  it("pads a narrow span so a few degrees of drift reads as a slope", () => {
    expect(computeAdaptiveTemperatureDomain([40, 41, 42])).toEqual([38, 44]);
  });

  it("scales the padding with the observed span", () => {
    expect(computeAdaptiveTemperatureDomain([20, 120])).toEqual([10, 130]);
  });

  it("keeps a single reading visible with the minimum padding", () => {
    expect(computeAdaptiveTemperatureDomain([50])).toEqual([48, 52]);
  });

  it("clamps the lower bound at zero rather than padding into negatives", () => {
    expect(computeAdaptiveTemperatureDomain([1, 2])).toEqual([0, 4]);
  });

  it("does not use the fixed 0-100 range", () => {
    const domain = computeAdaptiveTemperatureDomain([55, 58]);

    expect(domain).not.toEqual([0, 100]);
  });
});

describe("collectTemperatureDomainValues", () => {
  it("collects every lane series plus the extra reference values", () => {
    const rows: ThermalTimelineRow[] = buildDailyTimelineRows(
      [
        trendPoint("2026-01-15", {
          idle: band(30, 28, 33, 600),
          high: band(60, 55, 70, 60),
        }),
      ],
      1,
      new Date("2026-01-15T12:00:00Z"),
      "C",
      identityLabel,
    );

    const values = collectTemperatureDomainValues(rows, [90]);

    expect(values).toContain(90);
    expect(values).toContain(28);
    expect(values).toContain(70);
  });
});

describe("resolveBaselineBand", () => {
  it("has no reference while the baseline is still establishing", () => {
    expect(
      resolveBaselineBand(
        { status: "establishing", qualifyingDays: 4, requiredDays: 7 },
        "C",
      ),
    ).toBeNull();
  });

  it("has no reference before the baseline has been fetched", () => {
    expect(resolveBaselineBand(null, "C")).toBeNull();
  });

  it("brackets the established baseline by the display half-width", () => {
    expect(
      resolveBaselineBand(
        {
          status: "established",
          idleTemperatureAvg: 32,
          windowStartDate: "2025-11-01",
          windowEndDate: "2025-11-14",
          sampleMinutes: 12_600,
        },
        "C",
      ),
    ).toEqual({ value: 32, lower: 30, upper: 34 });
  });

  it("converts both band edges when Fahrenheit is displayed", () => {
    expect(
      resolveBaselineBand(
        {
          status: "established",
          idleTemperatureAvg: 30,
          windowStartDate: "2025-11-01",
          windowEndDate: "2025-11-14",
          sampleMinutes: 12_600,
        },
        "F",
      ),
    ).toEqual({ value: 86, lower: 82.4, upper: 89.6 });
  });
});

describe("buildDailyTimelineRows", () => {
  const referenceDate = new Date("2026-01-15T12:00:00Z");

  it("builds one row per day in the trailing window, oldest first", () => {
    const rows = buildDailyTimelineRows(
      [],
      3,
      referenceDate,
      "C",
      identityLabel,
    );

    expect(rows.map((row) => row.key)).toEqual([
      "2026-01-13",
      "2026-01-14",
      "2026-01-15",
    ]);
  });

  it("keeps a day the rollup skipped as an all-null gap row", () => {
    const rows = buildDailyTimelineRows(
      [
        trendPoint("2026-01-15", {
          idle: band(30, 28, 33, 600),
        }),
      ],
      2,
      referenceDate,
      "C",
      identityLabel,
    );

    expect(rows[0]).toMatchObject({
      key: "2026-01-14",
      temperatureAvg: null,
      temperatureMin: null,
      temperatureMax: null,
      temperatureRange: null,
      idleTemperature: null,
      loadIdle: null,
      loadLow: null,
      loadMid: null,
      loadHigh: null,
    });
    expect(rows[1].temperatureAvg).toBe(30);
  });

  it("spans the min-max range across every band that recorded", () => {
    const rows = buildDailyTimelineRows(
      [
        trendPoint("2026-01-15", {
          idle: band(30, 26, 34, 600),
          high: band(60, 55, 72, 60),
        }),
      ],
      1,
      referenceDate,
      "C",
      identityLabel,
    );

    expect(rows[0].temperatureMin).toBe(26);
    expect(rows[0].temperatureMax).toBe(72);
    expect(rows[0].temperatureRange).toEqual([26, 72]);
  });

  it("weights the daily average by the minutes each band covered", () => {
    const rows = buildDailyTimelineRows(
      [
        trendPoint("2026-01-15", {
          idle: band(30, 28, 32, 900),
          high: band(70, 65, 75, 100),
        }),
      ],
      1,
      referenceDate,
      "C",
      identityLabel,
    );

    // (30 * 900 + 70 * 100) / 1000
    expect(rows[0].temperatureAvg).toBe(34);
  });

  it("ignores bands with no samples instead of averaging them in as zero", () => {
    const rows = buildDailyTimelineRows(
      [trendPoint("2026-01-15", { idle: band(30, 28, 32, 900) })],
      1,
      referenceDate,
      "C",
      identityLabel,
    );

    expect(rows[0].temperatureAvg).toBe(30);
  });

  it("splits the day into load-band shares that add up to a full day", () => {
    const rows = buildDailyTimelineRows(
      [
        trendPoint("2026-01-15", {
          idle: band(30, 28, 32, 600),
          low: band(40, 36, 44, 200),
          mid: band(50, 46, 54, 150),
          high: band(60, 56, 64, 50),
        }),
      ],
      1,
      referenceDate,
      "C",
      identityLabel,
    );

    const row = rows[0];
    expect(row.loadIdle).toBeCloseTo(60);
    expect(row.loadLow).toBeCloseTo(20);
    expect(row.loadMid).toBeCloseTo(15);
    expect(row.loadHigh).toBeCloseTo(5);
    expect(
      (row.loadIdle ?? 0) +
        (row.loadLow ?? 0) +
        (row.loadMid ?? 0) +
        (row.loadHigh ?? 0),
    ).toBeCloseTo(100);
  });

  it("keeps a recorded day with no band samples out of the load lane", () => {
    const rows = buildDailyTimelineRows(
      [trendPoint("2026-01-15", { coverageMinutes: 30 })],
      1,
      referenceDate,
      "C",
      identityLabel,
    );

    expect(rows[0].loadIdle).toBeNull();
    expect(rows[0].loadHigh).toBeNull();
  });

  it("converts every temperature series to the display unit", () => {
    const rows = buildDailyTimelineRows(
      [trendPoint("2026-01-15", { idle: band(30, 20, 40, 600) })],
      1,
      referenceDate,
      "F",
      identityLabel,
    );

    expect(rows[0]).toMatchObject({
      temperatureAvg: 86,
      temperatureMin: 68,
      temperatureMax: 104,
      idleTemperature: 86,
    });
  });

  it("labels rows through the caller's formatter", () => {
    const rows = buildDailyTimelineRows(
      [],
      1,
      referenceDate,
      "C",
      (isoDate) => `day:${isoDate}`,
    );

    expect(rows[0].label).toBe("day:2026-01-15");
  });
});

describe("buildArchiveTimelineRows", () => {
  const identityBucketLabel = (timestamp: number) => String(timestamp);

  it("returns no rows when every series is empty", () => {
    expect(
      buildArchiveTimelineRows(
        {
          temperatureAvg: [],
          temperatureMax: [],
          temperatureMin: [],
          cpuUsage: [],
        },
        60_000,
        "C",
        identityBucketLabel,
      ),
    ).toEqual([]);
  });

  it("merges the avg/max/min and CPU series onto one bucket axis", () => {
    const rows = buildArchiveTimelineRows(
      {
        temperatureAvg: [{ timestamp: 0, value: 50 }],
        temperatureMax: [{ timestamp: 0, value: 60 }],
        temperatureMin: [{ timestamp: 0, value: 40 }],
        cpuUsage: [{ timestamp: 0, value: 25.55 }],
      },
      60_000,
      "C",
      identityBucketLabel,
    );

    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({
      key: "0",
      temperatureAvg: 50,
      temperatureMin: 40,
      temperatureMax: 60,
      temperatureRange: [40, 60],
      cpuUsage: 25.6,
    });
  });

  it("fills a bucket no series reported with an all-null gap row", () => {
    const rows = buildArchiveTimelineRows(
      {
        temperatureAvg: [
          { timestamp: 0, value: 50 },
          { timestamp: 120_000, value: 52 },
        ],
        temperatureMax: [],
        temperatureMin: [],
        cpuUsage: [],
      },
      60_000,
      "C",
      identityBucketLabel,
    );

    expect(rows.map((row) => row.key)).toEqual(["0", "60000", "120000"]);
    expect(rows[1]).toMatchObject({
      temperatureAvg: null,
      temperatureRange: null,
      cpuUsage: null,
    });
  });

  it("breaks the band when only one edge of the range was recorded", () => {
    const rows = buildArchiveTimelineRows(
      {
        temperatureAvg: [{ timestamp: 0, value: 50 }],
        temperatureMax: [{ timestamp: 0, value: 60 }],
        temperatureMin: [{ timestamp: 0, value: null }],
        cpuUsage: [],
      },
      60_000,
      "C",
      identityBucketLabel,
    );

    expect(rows[0].temperatureRange).toBeNull();
  });

  it("converts temperatures but leaves CPU usage as a percentage", () => {
    const rows = buildArchiveTimelineRows(
      {
        temperatureAvg: [{ timestamp: 0, value: 30 }],
        temperatureMax: [],
        temperatureMin: [],
        cpuUsage: [{ timestamp: 0, value: 30 }],
      },
      60_000,
      "F",
      identityBucketLabel,
    );

    expect(rows[0].temperatureAvg).toBe(86);
    expect(rows[0].cpuUsage).toBe(30);
  });

  it("returns no rows for a non-positive bucket width", () => {
    expect(
      buildArchiveTimelineRows(
        {
          temperatureAvg: [{ timestamp: 0, value: 50 }],
          temperatureMax: [],
          temperatureMin: [],
          cpuUsage: [],
        },
        0,
        "C",
        identityBucketLabel,
      ),
    ).toEqual([]);
  });
});
