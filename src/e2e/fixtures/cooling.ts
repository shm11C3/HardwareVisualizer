import type {
  AmbientArchiveSeries,
  ArchiveBucketTimestamp,
  CoolingBandComparison,
  CoolingBandMedian,
  CoolingBandMedianDelta,
  CoolingBandTemperature,
  CoolingBaselineDelta,
  CoolingDailyTrendPoint,
  CoolingDeltaBaselineState,
  CoolingExplorerWindow,
  CoolingFanTrendSeries,
  CoolingLoadBand,
  CoolingLoadTemperatureExplorer,
  CoolingLoadTemperaturePoint,
  FanArchiveSeries,
} from "@/rspc/bindings";
import { buildArchiveSeries } from "./archive";

const band = (
  avg: number,
  max: number,
  min: number,
  sampleMinutes: number,
): CoolingBandTemperature => ({ avg, max, min, sampleMinutes });

/**
 * Deterministic daily rollup for the 90d/1y Cooling Insight routes.
 * Every 13th day is skipped to exercise the coverage strip's gap rendering,
 * and the series follows a fixed sine wave so captures stay stable.
 *
 * `endDate` anchors the series so it lines up with the fixed E2E clock
 * (`FIXED_TIME` in `e2e/insights.spec.ts`) regardless of which `days` window
 * the frontend requested.
 */
/** A day with no CPU package power source: absent, never 0 W. */
const NO_POWER = {
  avg: null,
  max: null,
  min: null,
  sampleMinutes: 0,
} as const satisfies CoolingDailyTrendPoint["power"];

export const buildCoolingDailyTrendFixture = (
  days: number,
  endDate = new Date("2026-01-15T12:00:00Z"),
  /**
   * `false` simulates a machine whose platform publishes no CPU package
   * power, so the timeline draws no power lane and the pending-sensors
   * note keeps naming power (#2021).
   */
  hasPower = true,
): CoolingDailyTrendPoint[] => {
  const points: CoolingDailyTrendPoint[] = [];

  for (let offset = days - 1; offset >= 0; offset--) {
    const date = new Date(endDate);
    date.setUTCDate(date.getUTCDate() - offset);
    const dayIndex = days - 1 - offset;

    // Skip every 13th day so the coverage strip has visible gaps.
    if (dayIndex % 13 === 12) {
      continue;
    }

    const idleAvg = 32 + 2 * Math.sin(dayIndex / 9);
    points.push({
      date: date.toISOString().slice(0, 10),
      coverageMinutes: 1440,
      idle: band(idleAvg, idleAvg + 3, idleAvg - 3, 900),
      low: band(idleAvg + 8, idleAvg + 12, idleAvg + 4, 300),
      mid: band(idleAvg + 18, idleAvg + 24, idleAvg + 12, 180),
      high: band(idleAvg + 30, idleAvg + 38, idleAvg + 22, 60),
      power: hasPower
        ? {
            // Tracks the temperature wave so a capture shows the two lanes
            // rising together, which is what the lane is there to reveal.
            avg: Math.round((18 + 6 * Math.sin(dayIndex / 9)) * 10) / 10,
            max: Math.round((42 + 6 * Math.sin(dayIndex / 9)) * 10) / 10,
            min: 4.5,
            sampleMinutes: 1380,
          }
        : NO_POWER,
    });
  }

  return points;
};

/**
 * The fans a fan-enabled fixture machine reports, mirroring the stable
 * channel-derived labels the Super I/O provider archives (#2022).
 *
 * The third fan is deliberately an Inactive Fan Reading throughout: 0 RPM
 * is a real observation, so a capture must show it as a line on the floor
 * rather than as a gap.
 */
const FAN_FIXTURE_SOURCES = [
  { source: "Fan 1", base: 900, amplitude: 220 },
  { source: "Fan 2", base: 1450, amplitude: 320 },
  { source: "Fan 3", base: 0, amplitude: 0 },
] as const;

/**
 * Deterministic per-fan daily rollup for the 90d/1y routes, skipping the
 * same every-13th day as `buildCoolingDailyTrendFixture` so the fan lane
 * breaks where the lanes above it do.
 */
export const buildCoolingFanTrendFixture = (
  days: number,
  endDate = new Date("2026-01-15T12:00:00Z"),
): CoolingFanTrendSeries[] =>
  FAN_FIXTURE_SOURCES.map(({ source, base, amplitude }) => {
    const entries: CoolingFanTrendSeries["days"] = [];

    for (let offset = days - 1; offset >= 0; offset--) {
      const date = new Date(endDate);
      date.setUTCDate(date.getUTCDate() - offset);
      const dayIndex = days - 1 - offset;

      if (dayIndex % 13 === 12) {
        continue;
      }

      const avg = Math.round(base + amplitude * Math.sin(dayIndex / 9));
      entries.push({
        date: date.toISOString().slice(0, 10),
        rpmAvg: avg,
        rpmMax: avg + amplitude / 2,
        rpmMin: Math.max(0, avg - amplitude / 2),
        sampleMinutes: 1380,
      });
    }

    return { source, days: entries };
  });

/**
 * The archived per-fan series for the 24h/7d/30d routes, on the same bucket
 * grid and with the same gaps as the CPU series so every lane breaks
 * together.
 */
export const buildFanArchiveSeriesFixture = (
  start: string,
  end: string,
  bucketWidthMs: number,
  bucketTimestamp: ArchiveBucketTimestamp,
): FanArchiveSeries[] =>
  FAN_FIXTURE_SOURCES.map(({ source, base, amplitude }) => ({
    source,
    points: buildArchiveSeries(
      start,
      end,
      bucketWidthMs,
      bucketTimestamp,
      base,
      amplitude,
      { gapEvery: 17 },
    ),
  }));

/**
 * The archived ambient series for the 24h/7d/30d routes, on the same bucket
 * grid and with the same gaps as the CPU series so every lane breaks
 * together (#2046).
 *
 * `deltaAvg` is generated as its own series rather than as
 * `cpuAvg - ambientAvg`: Core pairs each archived minute before averaging,
 * so a bucket's delta genuinely is not the difference of the two bucket
 * averages, and a fixture that derived it that way would let a frontend
 * regression to subtraction pass unnoticed.
 */
export const buildAmbientArchiveSeriesFixture = (
  start: string,
  end: string,
  bucketWidthMs: number,
  bucketTimestamp: ArchiveBucketTimestamp,
): AmbientArchiveSeries => {
  const ambient = buildArchiveSeries(
    start,
    end,
    bucketWidthMs,
    bucketTimestamp,
    22,
    3,
    { gapEvery: 17 },
  );
  const delta = new Map(
    buildArchiveSeries(start, end, bucketWidthMs, bucketTimestamp, 36, 4, {
      gapEvery: 17,
    }).map((point) => [point.timestamp, point.value]),
  );

  return {
    // Two sensors in one room, which is what the row-per-source archive
    // exists for; the lane draws their per-minute mean.
    sources: ["Desk sensor", "Window sensor"],
    buckets: ambient.map((point) => ({
      timestamp: point.timestamp,
      ambientAvg: point.value,
      deltaAvg: delta.get(point.timestamp) ?? null,
    })),
  };
};

export const coolingBaselineDeltaEstablishingFixture: CoolingBaselineDelta = {
  baseline: { status: "establishing", qualifyingDays: 4, requiredDays: 7 },
  recent: {
    windowStartDate: "2026-01-09",
    windowEndDate: "2026-01-15",
    idleTemperatureAvg: null,
    sampleMinutes: 0,
  },
  delta: null,
  observation: "establishing",
  dailyDeltas: [],
  sustainedDays: 0,
  ambientAdjusted: {
    baseline: { status: "establishing", qualifyingDays: 0, requiredDays: 7 },
    recent: { deltaAvg: null, sampleMinutes: 0 },
    delta: null,
    comparable: false,
  },
};

export const coolingBaselineDeltaFixture: CoolingBaselineDelta = {
  baseline: {
    status: "established",
    idleTemperatureAvg: 32,
    windowStartDate: "2025-11-01",
    windowEndDate: "2025-11-14",
    sampleMinutes: 12_600,
  },
  recent: {
    windowStartDate: "2026-01-09",
    windowEndDate: "2026-01-15",
    idleTemperatureAvg: 33.5,
    sampleMinutes: 6_300,
  },
  delta: 1.5,
  observation: "withinRange",
  dailyDeltas: [
    { date: "2026-01-13", delta: 1.2 },
    { date: "2026-01-14", delta: 1.4 },
    { date: "2026-01-15", delta: 1.5 },
  ],
  sustainedDays: 3,
  ambientAdjusted: {
    baseline: { status: "establishing", qualifyingDays: 0, requiredDays: 7 },
    recent: { deltaAvg: null, sampleMinutes: 0 },
    delta: null,
    comparable: false,
  },
};

/**
 * `observation: "notComparable"` - baseline established, but the trailing
 * recent window does not carry enough idle evidence to compare against it.
 */
export const coolingBaselineDeltaNotComparableFixture: CoolingBaselineDelta = {
  baseline: {
    status: "established",
    idleTemperatureAvg: 32,
    windowStartDate: "2025-11-01",
    windowEndDate: "2025-11-14",
    sampleMinutes: 12_600,
  },
  recent: {
    windowStartDate: "2026-01-09",
    windowEndDate: "2026-01-15",
    idleTemperatureAvg: null,
    sampleMinutes: 12,
  },
  delta: null,
  observation: "notComparable",
  dailyDeltas: [],
  sustainedDays: 0,
  ambientAdjusted: {
    baseline: { status: "establishing", qualifyingDays: 0, requiredDays: 7 },
    recent: { deltaAvg: null, sampleMinutes: 0 },
    delta: null,
    comparable: false,
  },
};

/** `observation: "sustainedMildRise"` - a 3-day streak at a +5..10degC drift. */
export const coolingBaselineDeltaMildRiseFixture: CoolingBaselineDelta = {
  baseline: {
    status: "established",
    idleTemperatureAvg: 32,
    windowStartDate: "2025-11-01",
    windowEndDate: "2025-11-14",
    sampleMinutes: 12_600,
  },
  recent: {
    windowStartDate: "2026-01-09",
    windowEndDate: "2026-01-15",
    idleTemperatureAvg: 38.2,
    sampleMinutes: 6_300,
  },
  delta: 6.2,
  observation: "sustainedMildRise",
  dailyDeltas: [
    { date: "2026-01-13", delta: 5.6 },
    { date: "2026-01-14", delta: 5.9 },
    { date: "2026-01-15", delta: 6.2 },
  ],
  sustainedDays: 3,
  ambientAdjusted: {
    baseline: { status: "establishing", qualifyingDays: 0, requiredDays: 7 },
    recent: { deltaAvg: null, sampleMinutes: 0 },
    delta: null,
    comparable: false,
  },
};

/** `observation: "sustainedLargeRise"` - a 3-day streak at a +10degC+ drift. */
export const coolingBaselineDeltaLargeRiseFixture: CoolingBaselineDelta = {
  baseline: {
    status: "established",
    idleTemperatureAvg: 32,
    windowStartDate: "2025-11-01",
    windowEndDate: "2025-11-14",
    sampleMinutes: 12_600,
  },
  recent: {
    windowStartDate: "2026-01-09",
    windowEndDate: "2026-01-15",
    idleTemperatureAvg: 43.5,
    sampleMinutes: 6_300,
  },
  delta: 11.5,
  observation: "sustainedLargeRise",
  dailyDeltas: [
    { date: "2026-01-13", delta: 10.2 },
    { date: "2026-01-14", delta: 10.8 },
    { date: "2026-01-15", delta: 11.5 },
  ],
  sustainedDays: 3,
  ambientAdjusted: {
    baseline: { status: "establishing", qualifyingDays: 0, requiredDays: 7 },
    recent: { deltaAvg: null, sampleMinutes: 0 },
    delta: null,
    comparable: false,
  },
};

/**
 * Deterministic hourly (load, temperature) pairs for one Explorer window.
 * Loads walk a fixed cycle and the temperature follows the load plus a
 * fixed per-window offset, so a capture shows two visibly separated
 * clouds.
 *
 * `extraPoints` appends points the cycle would not produce - used to give
 * the recent window a single, under-sampled high-band hour.
 */
const buildExplorerWindow = (
  startDate: string,
  endDate: string,
  loads: number[],
  temperatureOffset: number,
  extraPoints: CoolingLoadTemperaturePoint[] = [],
): CoolingExplorerWindow => {
  const points: CoolingLoadTemperaturePoint[] = [];

  for (let index = 0; index < 48; index++) {
    const cpuUsageAvg = loads[index % loads.length];
    points.push({
      hourStart: `${endDate} ${String(index % 24).padStart(2, "0")}:00`,
      cpuUsageAvg,
      cpuTemperatureAvg:
        30 + cpuUsageAvg * 0.45 + temperatureOffset + (index % 3),
      sampleMinutes: 60,
    });
  }

  return { startDate, endDate, points: [...points, ...extraPoints] };
};

/** The CPU-load band a usage percentage falls in (mirrors Core). */
const classifyLoadBand = (cpuUsageAvg: number): CoolingLoadBand => {
  if (cpuUsageAvg < 10) return "idle";
  if (cpuUsageAvg < 30) return "low";
  if (cpuUsageAvg < 60) return "mid";
  return "high";
};

const median = (values: number[]): number | null => {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1
    ? sorted[middle]
    : (sorted[middle - 1] + sorted[middle]) / 2;
};

/**
 * Summarize one band from the window's own points, so the band deltas can
 * never contradict the scatter they are shown beside. Core derives these
 * the same way; the fixture mirrors it rather than hardcoding numbers that
 * drift out of step with the generated points.
 */
const summarizeBand = (
  window: CoolingExplorerWindow,
  band: CoolingLoadBand,
): CoolingBandMedian => {
  const inBand = window.points.filter(
    (point) => classifyLoadBand(point.cpuUsageAvg) === band,
  );
  return {
    temperatureMedian: median(inBand.map((point) => point.cpuTemperatureAvg)),
    pointCount: inBand.length,
    sampleMinutes: inBand.reduce((sum, point) => sum + point.sampleMinutes, 0),
  };
};

/** Core's `COOLING_BAND_COMPARISON_MINIMUM_SAMPLE_MINUTES`. */
const MINIMUM_COMPARABLE_SAMPLE_MINUTES = 30;

const buildBandDeltas = (
  baseline: CoolingExplorerWindow,
  recent: CoolingExplorerWindow,
): CoolingBandMedianDelta[] =>
  (["idle", "low", "mid", "high"] as const).map((band) => {
    const baselineMedian = summarizeBand(baseline, band);
    const recentMedian = summarizeBand(recent, band);
    const comparable =
      baselineMedian.sampleMinutes >= MINIMUM_COMPARABLE_SAMPLE_MINUTES &&
      recentMedian.sampleMinutes >= MINIMUM_COMPARABLE_SAMPLE_MINUTES;

    return {
      band,
      baseline: baselineMedian,
      recent: recentMedian,
      delta:
        comparable &&
        baselineMedian.temperatureMedian != null &&
        recentMedian.temperatureMedian != null
          ? recentMedian.temperatureMedian - baselineMedian.temperatureMedian
          : null,
      comparable,
    };
  });

const explorerBaselineWindow = buildExplorerWindow(
  "2025-11-01",
  "2025-11-14",
  [3, 6, 14, 22, 38, 47, 66, 82],
  0,
);

/**
 * The recent window's cycle stays below the high band, and a single
 * under-sampled high-band hour is appended, so Core's comparability bar
 * genuinely rejects that band - the capture shows a real not-comparable
 * row rather than one asserted against contradicting points.
 */
const explorerRecentWindow = buildExplorerWindow(
  "2025-12-19",
  "2026-01-15",
  [3, 6, 14, 22, 38, 47, 52, 58],
  4,
  [
    {
      hourStart: "2026-01-15 23:00",
      cpuUsageAvg: 74,
      cpuTemperatureAvg: 71,
      sampleMinutes: 12,
    },
  ],
);

export const coolingLoadTemperatureExplorerFixture: CoolingLoadTemperatureExplorer =
  {
    status: "established",
    baseline: explorerBaselineWindow,
    recent: explorerRecentWindow,
    bandDeltas: buildBandDeltas(explorerBaselineWindow, explorerRecentWindow),
  };

export const coolingLoadTemperatureExplorerEstablishingFixture: CoolingLoadTemperatureExplorer =
  { status: "establishing", qualifyingDays: 4, requiredDays: 7 };

export const coolingBandComparisonEstablishingFixture: CoolingBandComparison = {
  status: "establishing",
  qualifyingDays: 4,
  requiredDays: 7,
};

export const coolingBandComparisonFixture: CoolingBandComparison = {
  status: "established",
  baselineWindowStartDate: "2025-11-01",
  baselineWindowEndDate: "2025-11-14",
  recentWindowStartDate: "2026-01-09",
  recentWindowEndDate: "2026-01-15",
  bands: [
    {
      band: "idle",
      baseline: { temperatureAvg: 32, sampleMinutes: 12_600 },
      recent: { temperatureAvg: 33.5, sampleMinutes: 6_300 },
      comparable: true,
      ambientAdjusted: null,
    },
    {
      band: "low",
      baseline: { temperatureAvg: 40, sampleMinutes: 4_200 },
      recent: { temperatureAvg: 41, sampleMinutes: 2_100 },
      comparable: true,
      ambientAdjusted: null,
    },
    {
      band: "mid",
      baseline: { temperatureAvg: 50, sampleMinutes: 2_500 },
      recent: { temperatureAvg: null, sampleMinutes: 40 },
      comparable: false,
      ambientAdjusted: null,
    },
    {
      band: "high",
      baseline: { temperatureAvg: 62, sampleMinutes: 800 },
      recent: { temperatureAvg: null, sampleMinutes: 0 },
      comparable: false,
      ambientAdjusted: null,
    },
  ],
  ambientAdjustedBaseline: {
    status: "establishing",
    qualifyingDays: 0,
    requiredDays: 7,
  },
};

/**
 * The ΔT baseline's own window, deliberately a different range than the
 * absolute baseline's: ambient collection commonly starts long after a
 * machine did, so the two baselines establish over different days and the
 * comparison panel has to label each one's window (#2046).
 */
const AMBIENT_BASELINE = {
  status: "established",
  deltaTemperatureAvg: 28,
  windowStartDate: "2025-12-01",
  windowEndDate: "2025-12-14",
  sampleMinutes: 9_800,
} as const satisfies CoolingDeltaBaselineState;

/**
 * A machine whose environmental sensor explains the absolute rise: idle
 * temperature is 6.2 degC above baseline, but only 0.3 degC of that survives
 * ambient normalization - the confounder #1666's own issue text names.
 */
export const coolingBaselineDeltaAmbientFixture: CoolingBaselineDelta = {
  ...coolingBaselineDeltaMildRiseFixture,
  ambientAdjusted: {
    baseline: AMBIENT_BASELINE,
    recent: { deltaAvg: 28.3, sampleMinutes: 5_400 },
    delta: 0.3,
    comparable: true,
  },
};

/**
 * The same band comparison with an ambient reading on top, exercising all
 * three states a band can be in: paired and comparable, paired but too thin
 * on one side, and never paired at all.
 */
export const coolingBandComparisonAmbientFixture: CoolingBandComparison = {
  status: "established",
  baselineWindowStartDate: "2025-11-01",
  baselineWindowEndDate: "2025-11-14",
  recentWindowStartDate: "2026-01-09",
  recentWindowEndDate: "2026-01-15",
  bands: [
    {
      band: "idle",
      baseline: { temperatureAvg: 32, sampleMinutes: 12_600 },
      recent: { temperatureAvg: 33.5, sampleMinutes: 6_300 },
      comparable: true,
      ambientAdjusted: {
        baseline: { deltaAvg: 28, sampleMinutes: 9_800 },
        recent: { deltaAvg: 28.3, sampleMinutes: 5_400 },
        comparable: true,
      },
    },
    {
      band: "low",
      baseline: { temperatureAvg: 40, sampleMinutes: 4_200 },
      recent: { temperatureAvg: 41, sampleMinutes: 2_100 },
      comparable: true,
      ambientAdjusted: {
        baseline: { deltaAvg: 36, sampleMinutes: 3_100 },
        recent: { deltaAvg: 36.4, sampleMinutes: 1_800 },
        comparable: true,
      },
    },
    {
      band: "mid",
      baseline: { temperatureAvg: 50, sampleMinutes: 2_500 },
      recent: { temperatureAvg: null, sampleMinutes: 40 },
      comparable: false,
      // Ambient data exists for this band, but one window is too thin:
      // `comparable: false` on a present value, not a null.
      ambientAdjusted: {
        baseline: { deltaAvg: 46, sampleMinutes: 1_900 },
        recent: { deltaAvg: null, sampleMinutes: 22 },
        comparable: false,
      },
    },
    {
      band: "high",
      baseline: { temperatureAvg: 62, sampleMinutes: 800 },
      recent: { temperatureAvg: null, sampleMinutes: 0 },
      comparable: false,
      // Never a paired minute at all: null, not a zero ΔT.
      ambientAdjusted: null,
    },
  ],
  ambientAdjustedBaseline: AMBIENT_BASELINE,
};
