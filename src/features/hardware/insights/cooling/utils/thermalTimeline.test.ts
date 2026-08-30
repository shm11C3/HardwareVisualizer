import { describe, expect, it } from "vitest";
import type {
  CoolingBandTemperature,
  CoolingDailyTrendPoint,
} from "@/rspc/bindings";
import {
  buildArchiveTimelineRows,
  buildDailyTimelineRows,
  collectPowerDomainValues,
  collectTemperatureDomainValues,
  computeAdaptiveTemperatureDomain,
  computePowerDomain,
  hasRoutedPowerData,
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
  power: { avg: null, max: null, min: null, sampleMinutes: 0 },
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

describe("buildDailyTimelineRows power lane", () => {
  const referenceDate = new Date("2026-01-15T12:00:00Z");

  it("carries the day's power summary onto the row", () => {
    const rows = buildDailyTimelineRows(
      [
        trendPoint("2026-01-15", {
          power: { avg: 18.5, max: 42, min: 4.5, sampleMinutes: 1200 },
        }),
      ],
      1,
      referenceDate,
      "C",
      identityLabel,
    );

    expect(rows[0]).toMatchObject({
      powerAvg: 18.5,
      powerMin: 4.5,
      powerMax: 42,
      powerRange: [4.5, 42],
    });
  });

  it("keeps a day without power readings absent rather than zeroed", () => {
    const rows = buildDailyTimelineRows(
      [trendPoint("2026-01-15", { idle: band(30, 28, 33, 600) })],
      1,
      referenceDate,
      "C",
      identityLabel,
    );

    expect(rows[0]).toMatchObject({
      temperatureAvg: 30,
      powerAvg: null,
      powerMin: null,
      powerMax: null,
      powerRange: null,
    });
  });

  it("breaks the power band when only one edge was recorded", () => {
    const rows = buildDailyTimelineRows(
      [
        trendPoint("2026-01-15", {
          power: { avg: 18.5, max: 42, min: null, sampleMinutes: 1200 },
        }),
      ],
      1,
      referenceDate,
      "C",
      identityLabel,
    );

    expect(rows[0].powerAvg).toBe(18.5);
    expect(rows[0].powerRange).toBeNull();
  });

  it("keeps power in watts regardless of the temperature display unit", () => {
    const rows = buildDailyTimelineRows(
      [
        trendPoint("2026-01-15", {
          idle: band(30, 28, 33, 600),
          power: { avg: 18.5, max: 42, min: 4.5, sampleMinutes: 1200 },
        }),
      ],
      1,
      referenceDate,
      "F",
      identityLabel,
    );

    expect(rows[0].temperatureAvg).toBe(86);
    expect(rows[0].powerAvg).toBe(18.5);
  });
});

describe("hasRoutedPowerData", () => {
  const NO_SERIES = {
    temperatureAvg: [],
    temperatureMax: [],
    temperatureMin: [],
    cpuUsage: [],
    powerAvg: [],
    powerMax: [],
    powerMin: [],
  };
  const ARCHIVE = { kind: "archive" } as const;
  const DAILY = { kind: "dailyTrend" } as const;

  it("reads the archive power series on the archive routes", () => {
    expect(
      hasRoutedPowerData(
        ARCHIVE,
        { ...NO_SERIES, powerAvg: [{ timestamp: 0, value: 18 }] },
        null,
      ),
    ).toBe(true);
  });

  it("treats an archive bucket with no value as no power", () => {
    expect(
      hasRoutedPowerData(
        ARCHIVE,
        { ...NO_SERIES, powerAvg: [{ timestamp: 0, value: null }] },
        null,
      ),
    ).toBe(false);
  });

  it("ignores the daily trend while an archive route is selected", () => {
    // Otherwise a 24h window on a machine that only ever recorded power
    // months ago would claim the lane is available.
    expect(
      hasRoutedPowerData(ARCHIVE, NO_SERIES, [
        trendPoint("2026-01-15", {
          power: { avg: 18, max: 42, min: 4, sampleMinutes: 900 },
        }),
      ]),
    ).toBe(false);
  });

  it("reads the daily trend on the long-range routes", () => {
    expect(
      hasRoutedPowerData(DAILY, NO_SERIES, [
        trendPoint("2026-01-15", {
          power: { avg: 18, max: 42, min: 4, sampleMinutes: 900 },
        }),
      ]),
    ).toBe(true);
  });

  it("is false for a daily window whose days recorded no power", () => {
    expect(
      hasRoutedPowerData(DAILY, NO_SERIES, [trendPoint("2026-01-15")]),
    ).toBe(false);
  });

  it("is false while the daily trend is still loading", () => {
    expect(hasRoutedPowerData(DAILY, NO_SERIES, null)).toBe(false);
  });
});

describe("computePowerDomain", () => {
  it("returns null when nothing was recorded", () => {
    expect(computePowerDomain([null, null])).toBeNull();
  });

  it("returns null for an empty window rather than assuming support", () => {
    expect(computePowerDomain([])).toBeNull();
  });

  it("anchors the lower bound at zero so draw reads against no load", () => {
    expect(computePowerDomain([12, 40])).toEqual([0, 44]);
  });

  it("keeps headroom above a flat series instead of a zero-height lane", () => {
    expect(computePowerDomain([20, 20])).toEqual([0, 22]);
  });
});

describe("buildArchiveTimelineRows", () => {
  const identityBucketLabel = (timestamp: number) => String(timestamp);

  const EMPTY_POWER = {
    powerAvg: [],
    powerMax: [],
    powerMin: [],
  } as const;

  it("merges the CPU power series onto the same bucket axis", () => {
    const rows = buildArchiveTimelineRows(
      {
        temperatureAvg: [{ timestamp: 0, value: 50 }],
        temperatureMax: [],
        temperatureMin: [],
        cpuUsage: [],
        powerAvg: [{ timestamp: 0, value: 18.55 }],
        powerMax: [{ timestamp: 0, value: 42 }],
        powerMin: [{ timestamp: 0, value: 4.5 }],
      },
      60_000,
      "C",
      identityBucketLabel,
    );

    expect(rows[0]).toMatchObject({
      powerAvg: 18.6,
      powerMin: 4.5,
      powerMax: 42,
      powerRange: [4.5, 42],
    });
  });

  it("keeps a machine without a power source at absent power, not 0 W", () => {
    const rows = buildArchiveTimelineRows(
      {
        temperatureAvg: [{ timestamp: 0, value: 50 }],
        temperatureMax: [],
        temperatureMin: [],
        cpuUsage: [],
        ...EMPTY_POWER,
      },
      60_000,
      "C",
      identityBucketLabel,
    );

    expect(rows[0].powerAvg).toBeNull();
    expect(rows[0].powerRange).toBeNull();
    // Which is exactly what makes the lane's gate close.
    expect(computePowerDomain(collectPowerDomainValues(rows))).toBeNull();
  });

  it("extends the bucket axis to cover a power-only bucket", () => {
    // The power series can outlive the temperature series on a machine
    // whose temperature sensor dropped out; the shared axis must still
    // cover it or the power lane would silently lose its newest bucket.
    const rows = buildArchiveTimelineRows(
      {
        temperatureAvg: [{ timestamp: 0, value: 50 }],
        temperatureMax: [],
        temperatureMin: [],
        cpuUsage: [],
        powerAvg: [{ timestamp: 60_000, value: 20 }],
        powerMax: [],
        powerMin: [],
      },
      60_000,
      "C",
      identityBucketLabel,
    );

    expect(rows.map((r) => r.key)).toEqual(["0", "60000"]);
    expect(rows[1].powerAvg).toBe(20);
  });

  it("returns no rows when every series is empty", () => {
    expect(
      buildArchiveTimelineRows(
        {
          temperatureAvg: [],
          temperatureMax: [],
          temperatureMin: [],
          cpuUsage: [],
          ...EMPTY_POWER,
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
        ...EMPTY_POWER,
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
        ...EMPTY_POWER,
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
        ...EMPTY_POWER,
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
        ...EMPTY_POWER,
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
          ...EMPTY_POWER,
        },
        0,
        "C",
        identityBucketLabel,
      ),
    ).toEqual([]);
  });
});
