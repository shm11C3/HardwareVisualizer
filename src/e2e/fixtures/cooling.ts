import type {
  CoolingBandComparison,
  CoolingBandMedian,
  CoolingBandMedianDelta,
  CoolingBandTemperature,
  CoolingBaselineDelta,
  CoolingDailyTrendPoint,
  CoolingExplorerWindow,
  CoolingLoadBand,
  CoolingLoadTemperatureExplorer,
  CoolingLoadTemperaturePoint,
} from "@/rspc/bindings";

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
    },
    {
      band: "low",
      baseline: { temperatureAvg: 40, sampleMinutes: 4_200 },
      recent: { temperatureAvg: 41, sampleMinutes: 2_100 },
      comparable: true,
    },
    {
      band: "mid",
      baseline: { temperatureAvg: 50, sampleMinutes: 2_500 },
      recent: { temperatureAvg: null, sampleMinutes: 40 },
      comparable: false,
    },
    {
      band: "high",
      baseline: { temperatureAvg: 62, sampleMinutes: 800 },
      recent: { temperatureAvg: null, sampleMinutes: 0 },
      comparable: false,
    },
  ],
};
