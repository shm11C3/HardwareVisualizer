import type {
  CoolingBandComparison,
  CoolingBandTemperature,
  CoolingBaselineDelta,
  CoolingDailyTrendPoint,
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
export const buildCoolingDailyTrendFixture = (
  days: number,
  endDate = new Date("2026-01-15T12:00:00Z"),
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
