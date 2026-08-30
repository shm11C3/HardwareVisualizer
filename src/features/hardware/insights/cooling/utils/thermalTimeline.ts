import type {
  ArchiveSeriesPoint,
  CoolingBandTemperature,
  CoolingBaselineState,
  CoolingDailyTrendPoint,
  TemperatureUnit,
} from "@/rspc/bindings";

/**
 * One column of the synchronized thermal timeline: a single day (90d/1y) or
 * a single archive bucket (24h/7d/30d). Both lanes read the same row array,
 * which is what makes their cursors and their gaps line up.
 *
 * Every value is nullable and a period with no recording stays null all the
 * way through - never zero-filled. Recharts is rendered without
 * `connectNulls`, so an absent period draws as a gap in both lanes.
 */
export type ThermalTimelineRow = {
  /** Stable identity: the ISO date (daily) or the bucket epoch ms (archive). */
  key: string;
  /** Shared category-axis value. May repeat when buckets share a label. */
  label: string;
  /** Temperature average for the period, in the display unit. */
  temperatureAvg: number | null;
  /** Lowest recorded temperature for the period, in the display unit. */
  temperatureMin: number | null;
  /** Highest recorded temperature for the period, in the display unit. */
  temperatureMax: number | null;
  /**
   * `[min, max]` for Recharts' range `Area`. Null (rather than a collapsed
   * pair) when either end is missing, so the band breaks instead of drawing
   * a fabricated flat segment.
   */
  temperatureRange: [number, number] | null;
  /** Idle-band average temperature, in the display unit (90d/1y only). */
  idleTemperature: number | null;
  /** Bucket-average CPU usage in percent (24h/7d/30d only). */
  cpuUsage: number | null;
  /** Share of the day's samples spent in the idle band, 0-100 (90d/1y only). */
  loadIdle: number | null;
  /** Share of the day's samples spent in the low band, 0-100 (90d/1y only). */
  loadLow: number | null;
  /** Share of the day's samples spent in the mid band, 0-100 (90d/1y only). */
  loadMid: number | null;
  /** Share of the day's samples spent in the high band, 0-100 (90d/1y only). */
  loadHigh: number | null;
  /**
   * CPU package power average for the period, in watts. Watts on every
   * period and in every locale - the temperature unit preference does not
   * reach this lane.
   */
  powerAvg: number | null;
  /** Lowest recorded CPU package power for the period, in watts. */
  powerMin: number | null;
  /** Highest recorded CPU package power for the period, in watts. */
  powerMax: number | null;
  /** `[min, max]` for the power lane's range `Area`, null when either end is missing. */
  powerRange: [number, number] | null;
};

/** Smallest headroom kept above and below the data, in display degrees. */
export const TEMPERATURE_DOMAIN_MIN_PADDING = 2;
/** Extra headroom as a share of the observed span. */
export const TEMPERATURE_DOMAIN_PADDING_RATIO = 0.1;

const LOAD_PERCENT_TOTAL = 100;

/** Smallest headroom kept above the power data, in watts. */
export const POWER_DOMAIN_MIN_PADDING = 2;
/** Extra power headroom as a share of the observed peak. */
export const POWER_DOMAIN_PADDING_RATIO = 0.1;

export const toDisplayTemperature = (
  value: number | null,
  unit: TemperatureUnit,
): number | null => {
  if (value == null || !Number.isFinite(value)) {
    return null;
  }

  const converted = unit === "F" ? (value * 9) / 5 + 32 : value;
  return Number.parseFloat(converted.toFixed(1));
};

/**
 * Y-axis domain that follows the data instead of the fixed 0-100 range the
 * older per-metric charts used, so a few degrees of drift reads as a slope
 * rather than a flat line near the bottom of the plot.
 *
 * Returns null when nothing was recorded, which the caller renders as the
 * "no data for this period" state rather than an empty axis.
 */
export const computeAdaptiveTemperatureDomain = (
  values: readonly (number | null)[],
): [number, number] | null => {
  const recorded = values.filter(
    (value): value is number => value != null && Number.isFinite(value),
  );

  if (recorded.length === 0) {
    return null;
  }

  const min = Math.min(...recorded);
  const max = Math.max(...recorded);
  const padding = Math.max(
    TEMPERATURE_DOMAIN_MIN_PADDING,
    (max - min) * TEMPERATURE_DOMAIN_PADDING_RATIO,
  );

  // Temperatures never go below zero on the scales this app displays, so the
  // lower bound is clamped rather than padded into negative space.
  return [Math.max(0, Math.floor(min - padding)), Math.ceil(max + padding)];
};

/** Every temperature a row contributes to the adaptive domain. */
export const collectTemperatureDomainValues = (
  rows: readonly ThermalTimelineRow[],
  extra: readonly (number | null)[] = [],
): (number | null)[] => [
  ...rows.flatMap((row) => [
    row.temperatureAvg,
    row.temperatureMin,
    row.temperatureMax,
    row.idleTemperature,
  ]),
  ...extra,
];

/** One watt value rounded for display, or null for an absent reading. */
const toDisplayWatts = (value: number | null | undefined): number | null =>
  value == null || !Number.isFinite(value)
    ? null
    : Number.parseFloat(value.toFixed(1));

/**
 * Y-axis domain for the power lane, in watts, and the lane's capability
 * gate: null means no period in the window recorded power, and the lane is
 * then not drawn at all rather than as an empty axis reading "0 W
 * measured".
 *
 * Anchored at 0 rather than following the data the way the temperature
 * lane does: draw is meaningful against no draw, so a rise from 8 W to
 * 12 W should read as the modest change it is, not fill the lane.
 */
export const computePowerDomain = (
  values: readonly (number | null)[],
): [number, number] | null => {
  const recorded = values.filter(
    (value): value is number => value != null && Number.isFinite(value),
  );

  if (recorded.length === 0) {
    return null;
  }

  const max = Math.max(...recorded);
  const padding = Math.max(
    POWER_DOMAIN_MIN_PADDING,
    max * POWER_DOMAIN_PADDING_RATIO,
  );

  return [0, Math.ceil(max + padding)];
};

/** Every watt value a row contributes to the power lane's domain. */
export const collectPowerDomainValues = (
  rows: readonly ThermalTimelineRow[],
): (number | null)[] =>
  rows.flatMap((row) => [row.powerAvg, row.powerMin, row.powerMax]);

/**
 * Whether the currently-routed period carries any CPU package power,
 * answered straight from the fetched sources rather than from built rows.
 *
 * The pending-sensors note sits beside the timeline, not inside it, and
 * needs the same answer the power lane's gate reaches; deriving it here
 * keeps the two from disagreeing while the row builders stay a rendering
 * concern.
 */
export const hasRoutedPowerData = (
  route: { kind: "archive" | "dailyTrend" },
  series: ArchiveTimelineSeries,
  dailyTrend: readonly CoolingDailyTrendPoint[] | null,
): boolean =>
  route.kind === "archive"
    ? series.powerAvg.some((point) => point.value != null)
    : (dailyTrend ?? []).some((point) => point.power.avg != null);

/**
 * Half-width of the band drawn around the established idle baseline.
 *
 * This is a drawing width, not a verdict: it gives the dashed baseline line
 * a readable "around here" band to sit in. Whether a drift matters at all
 * (the +5/+10 degC steps and the sustain rule) stays behind Core's boundary
 * in `CoolingDeltaObservation`, and nothing here labels the band.
 */
export const BASELINE_BAND_HALF_WIDTH_CELSIUS = 2;

/** The baseline reference line and the band drawn around it, in display units. */
export type BaselineBand = {
  value: number;
  lower: number;
  upper: number;
};

/**
 * Resolve the baseline reference for the temperature lane. Returns null
 * while the baseline is still establishing - the lane then simply has no
 * reference, instead of showing a placeholder value that looks measured.
 */
export const resolveBaselineBand = (
  baseline: CoolingBaselineState | null | undefined,
  temperatureUnit: TemperatureUnit,
): BaselineBand | null => {
  if (baseline == null || baseline.status !== "established") {
    return null;
  }

  const value = toDisplayTemperature(
    baseline.idleTemperatureAvg,
    temperatureUnit,
  );
  const lower = toDisplayTemperature(
    baseline.idleTemperatureAvg - BASELINE_BAND_HALF_WIDTH_CELSIUS,
    temperatureUnit,
  );
  const upper = toDisplayTemperature(
    baseline.idleTemperatureAvg + BASELINE_BAND_HALF_WIDTH_CELSIUS,
    temperatureUnit,
  );

  if (value == null || lower == null || upper == null) {
    return null;
  }

  return { value, lower, upper };
};

const toDateKey = (date: Date): string => date.toISOString().slice(0, 10);

const bandsOf = (
  point: CoolingDailyTrendPoint,
): readonly CoolingBandTemperature[] => [
  point.idle,
  point.low,
  point.mid,
  point.high,
];

const minOf = (values: readonly (number | null)[]): number | null => {
  const recorded = values.filter((value): value is number => value != null);
  return recorded.length === 0 ? null : Math.min(...recorded);
};

const maxOf = (values: readonly (number | null)[]): number | null => {
  const recorded = values.filter((value): value is number => value != null);
  return recorded.length === 0 ? null : Math.max(...recorded);
};

/**
 * Sample-weighted daily average across the four load bands. The rollup only
 * stores per-band averages, so the day's overall average is recomposed by
 * weighting each band by the minutes it actually covered - a band with no
 * samples contributes nothing instead of dragging the average toward zero.
 */
const weightedDailyAverage = (point: CoolingDailyTrendPoint): number | null => {
  let weighted = 0;
  let minutes = 0;

  for (const band of bandsOf(point)) {
    if (band.avg == null || band.sampleMinutes <= 0) {
      continue;
    }
    weighted += band.avg * band.sampleMinutes;
    minutes += band.sampleMinutes;
  }

  return minutes === 0 ? null : weighted / minutes;
};

/**
 * Share of the day's samples spent in each load band, as percentages that
 * add up to 100. A day with no samples stays null across all four bands, so
 * the stacked lane draws a gap rather than an empty full-height column.
 */
const loadComposition = (point: CoolingDailyTrendPoint | undefined) => {
  const total = point
    ? bandsOf(point).reduce(
        (sum, band) => sum + Math.max(band.sampleMinutes, 0),
        0,
      )
    : 0;

  if (!point || total <= 0) {
    return {
      loadIdle: null,
      loadLow: null,
      loadMid: null,
      loadHigh: null,
    };
  }

  const share = (band: CoolingBandTemperature) =>
    (Math.max(band.sampleMinutes, 0) / total) * LOAD_PERCENT_TOTAL;

  return {
    loadIdle: share(point.idle),
    loadLow: share(point.low),
    loadMid: share(point.mid),
    loadHigh: share(point.high),
  };
};

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

const toRange = (
  min: number | null,
  max: number | null,
): [number, number] | null => (min == null || max == null ? null : [min, max]);

/**
 * Build one row per day in the trailing `days`-day window ending on
 * `referenceDate` (inclusive), from the `getCoolingTrend` rollup.
 *
 * A day the rollup has no row for is absent from `points` (never zero-filled
 * by Core); this turns that absence into an all-null row so both lanes show
 * the same gap at the same x position.
 */
export const buildDailyTimelineRows = (
  points: readonly CoolingDailyTrendPoint[],
  days: number,
  referenceDate: Date,
  temperatureUnit: TemperatureUnit,
  formatLabel: (isoDate: string) => string,
): ThermalTimelineRow[] => {
  const pointByDate = new Map(points.map((point) => [point.date, point]));
  const rows: ThermalTimelineRow[] = [];

  for (let offset = days - 1; offset >= 0; offset--) {
    const cellDate = new Date(referenceDate);
    cellDate.setUTCDate(cellDate.getUTCDate() - offset);
    const date = toDateKey(cellDate);
    const point = pointByDate.get(date);
    const base = { key: date, label: formatLabel(date) };

    if (!point) {
      rows.push({ ...base, ...EMPTY_ROW });
      continue;
    }

    const bands = bandsOf(point);
    const temperatureMin = toDisplayTemperature(
      minOf(bands.map((band) => band.min)),
      temperatureUnit,
    );
    const temperatureMax = toDisplayTemperature(
      maxOf(bands.map((band) => band.max)),
      temperatureUnit,
    );
    // Core folds power outside the load bands, so it is read straight off
    // the point rather than recomposed from the band weights.
    const powerMin = toDisplayWatts(point.power.min);
    const powerMax = toDisplayWatts(point.power.max);

    rows.push({
      ...base,
      ...EMPTY_ROW,
      temperatureAvg: toDisplayTemperature(
        weightedDailyAverage(point),
        temperatureUnit,
      ),
      temperatureMin,
      temperatureMax,
      temperatureRange: toRange(temperatureMin, temperatureMax),
      idleTemperature: toDisplayTemperature(point.idle.avg, temperatureUnit),
      ...loadComposition(point),
      powerAvg: toDisplayWatts(point.power.avg),
      powerMin,
      powerMax,
      powerRange: toRange(powerMin, powerMax),
    });
  }

  return rows;
};

/** The archive series the 24h/7d/30d lanes are composed from. */
export type ArchiveTimelineSeries = {
  temperatureAvg: readonly ArchiveSeriesPoint[];
  temperatureMax: readonly ArchiveSeriesPoint[];
  temperatureMin: readonly ArchiveSeriesPoint[];
  cpuUsage: readonly ArchiveSeriesPoint[];
  /**
   * CPU package power. Read from the archive's `cpuPower` columns, which
   * every platform publishing a CPU power domain writes to, so these stay
   * empty exactly on machines with no such source.
   */
  powerAvg: readonly ArchiveSeriesPoint[];
  powerMax: readonly ArchiveSeriesPoint[];
  powerMin: readonly ArchiveSeriesPoint[];
};

const valueByTimestamp = (
  points: readonly ArchiveSeriesPoint[],
): Map<number, number | null> =>
  new Map(points.map((point) => [point.timestamp, point.value]));

/**
 * Build one row per archive bucket for 24h/7d/30d, merging the avg/max/min
 * temperature series and the CPU-usage series onto a single time axis.
 *
 * The archive query omits buckets it has no rows for, so merging only the
 * returned timestamps would silently close a gap. The axis is instead the
 * full `stepMs` grid between the earliest and latest bucket any series
 * returned; buckets nobody reported become all-null rows.
 */
export const buildArchiveTimelineRows = (
  series: ArchiveTimelineSeries,
  stepMs: number,
  temperatureUnit: TemperatureUnit,
  formatLabel: (timestampMs: number) => string,
): ThermalTimelineRow[] => {
  const avg = valueByTimestamp(series.temperatureAvg);
  const max = valueByTimestamp(series.temperatureMax);
  const min = valueByTimestamp(series.temperatureMin);
  const usage = valueByTimestamp(series.cpuUsage);
  const powerAvgByBucket = valueByTimestamp(series.powerAvg);
  const powerMaxByBucket = valueByTimestamp(series.powerMax);
  const powerMinByBucket = valueByTimestamp(series.powerMin);

  const timestamps = [
    ...series.temperatureAvg,
    ...series.temperatureMax,
    ...series.temperatureMin,
    ...series.cpuUsage,
    // The power series joins the shared axis rather than getting its own:
    // the lanes must break at the same buckets, and a power-only bucket
    // would otherwise fall outside the grid entirely.
    ...series.powerAvg,
    ...series.powerMax,
    ...series.powerMin,
  ].map((point) => point.timestamp);

  if (timestamps.length === 0 || stepMs <= 0) {
    return [];
  }

  const first = Math.min(...timestamps);
  const last = Math.max(...timestamps);
  const rows: ThermalTimelineRow[] = [];

  for (let timestamp = first; timestamp <= last; timestamp += stepMs) {
    const temperatureMin = toDisplayTemperature(
      min.get(timestamp) ?? null,
      temperatureUnit,
    );
    const temperatureMax = toDisplayTemperature(
      max.get(timestamp) ?? null,
      temperatureUnit,
    );
    const cpuUsage = usage.get(timestamp) ?? null;
    const powerMin = toDisplayWatts(powerMinByBucket.get(timestamp));
    const powerMax = toDisplayWatts(powerMaxByBucket.get(timestamp));

    rows.push({
      ...EMPTY_ROW,
      key: String(timestamp),
      label: formatLabel(timestamp),
      temperatureAvg: toDisplayTemperature(
        avg.get(timestamp) ?? null,
        temperatureUnit,
      ),
      temperatureMin,
      temperatureMax,
      temperatureRange: toRange(temperatureMin, temperatureMax),
      cpuUsage:
        cpuUsage == null ? null : Number.parseFloat(cpuUsage.toFixed(1)),
      powerAvg: toDisplayWatts(powerAvgByBucket.get(timestamp)),
      powerMin,
      powerMax,
      powerRange: toRange(powerMin, powerMax),
    });
  }

  return rows;
};
