import type { CoolingFanTrendSeries, FanArchiveSeries } from "@/rspc/bindings";
import type {
  ArchiveTimelineSeries,
  ThermalTimelineRow,
} from "./thermalTimeline";
import {
  archiveWindowRecordedAnything,
  hasFiniteValue,
} from "./thermalTimeline";

/**
 * One fan's series as the lane addresses it.
 *
 * `key` is positional (`fan0`, `fan1`, …) rather than the source name
 * itself: the name is a free-form label from the sensor provider, and
 * Recharts resolves a `dataKey` as a lodash-style path, so a name
 * containing a dot would silently address a nested property that does not
 * exist. Deriving the key from the sorted source order keeps each fan's
 * color and legend position stable across refreshes.
 */
export type FanSeries = {
  source: string;
  key: string;
};

/**
 * One column of the fan lane, aligned to the timeline's shared axis.
 *
 * RPM lives under `values` rather than flat on the row so the row type
 * stays closed: how many fans a machine exposes is configuration-dependent,
 * and an index signature on the row would make every other lane's field
 * access unchecked.
 */
export type FanLaneRow = {
  /** The matching `ThermalTimelineRow.key`, so both lanes break together. */
  key: string;
  label: string;
  /** RPM per [`FanSeries.key`]; null for a period this fan did not record. */
  values: Record<string, number | null>;
};

/** One fan's RPM keyed by the timeline row it belongs to. */
export type FanTimelineSeries = {
  source: string;
  valueByRowKey: ReadonlyMap<string, number | null>;
};

/** Smallest headroom kept above the fan data, in RPM. */
export const FAN_DOMAIN_MIN_PADDING = 100;
/** Extra fan headroom as a share of the observed peak. */
export const FAN_DOMAIN_PADDING_RATIO = 0.1;

/** The `values` path Recharts reads one fan series from. */
export const fanDataKey = (series: FanSeries): string => `values.${series.key}`;

/**
 * Assign each fan a stable positional series key, ordered by source.
 *
 * Core already returns its series ordered by source, but the ordering is
 * re-applied here because the lane's colors are assigned by position: a
 * caller that merged two sources in a different order would otherwise
 * recolor every fan.
 */
export const resolveFanSeries = (sources: readonly string[]): FanSeries[] =>
  [...sources]
    .sort((a, b) => a.localeCompare(b))
    .map((source, index) => ({ source, key: `fan${index}` }));

const toRowKeyedSeries = <T>(
  source: string,
  entries: readonly T[],
  rowKeyOf: (entry: T) => string,
  rpmOf: (entry: T) => number | null,
): FanTimelineSeries => ({
  source,
  valueByRowKey: new Map(
    entries.map((entry) => [rowKeyOf(entry), rpmOf(entry)]),
  ),
});

/**
 * The 24h/7d/30d fan series, keyed by archive bucket timestamp - which is
 * exactly `ThermalTimelineRow.key` for those routes.
 */
export const toArchiveFanSeries = (
  series: readonly FanArchiveSeries[],
): FanTimelineSeries[] =>
  series.map((entry) =>
    toRowKeyedSeries(
      entry.source,
      entry.points,
      (point) => String(point.timestamp),
      (point) => point.value,
    ),
  );

/**
 * The 90d/1y fan series, keyed by ISO date - `ThermalTimelineRow.key` for
 * the daily routes. Only `rpmAvg` is carried: the lane draws one line per
 * fan, and a min-max band per fan would overplot into noise once a machine
 * reports six of them.
 */
export const toDailyFanSeries = (
  series: readonly CoolingFanTrendSeries[],
): FanTimelineSeries[] =>
  series.map((entry) =>
    toRowKeyedSeries(
      entry.source,
      entry.days,
      (day) => day.date,
      (day) => day.rpmAvg,
    ),
  );

/**
 * Project the fan series onto the timeline's own rows.
 *
 * Driven by `rows` rather than by the fan data's own timestamps, so the
 * fan lane always has the same length, labels, and gaps as the lanes above
 * it - which is what keeps the synchronized cursor honest. A period a fan
 * did not record stays null and draws as a break.
 */
export const buildFanLaneRows = (
  rows: readonly ThermalTimelineRow[],
  series: readonly FanTimelineSeries[],
  fanSeries: readonly FanSeries[],
): FanLaneRow[] => {
  const seriesBySource = new Map(
    series.map((entry) => [entry.source, entry.valueByRowKey]),
  );

  return rows.map((row) => {
    const values: Record<string, number | null> = {};
    for (const fan of fanSeries) {
      const recorded = seriesBySource.get(fan.source)?.get(row.key);
      values[fan.key] =
        recorded == null || !Number.isFinite(recorded) ? null : recorded;
    }
    return { key: row.key, label: row.label, values };
  });
};

/**
 * Y-axis domain for the fan lane, in RPM, and the lane's capability gate:
 * null means nothing in the window recorded a fan, and the lane is then not
 * drawn at all rather than as an empty axis reading "0 RPM measured".
 *
 * Anchored at 0 rather than following the data the way the temperature lane
 * does: a fan's speed is meaningful against a stop, and an Inactive Fan
 * Reading of 0 RPM must sit on the floor rather than off the bottom of a
 * data-fitted axis.
 */
export const computeFanDomain = (
  rows: readonly FanLaneRow[],
): [number, number] | null => {
  const recorded = rows.flatMap((row) =>
    Object.values(row.values).filter(
      (value): value is number => value != null && Number.isFinite(value),
    ),
  );

  if (recorded.length === 0) {
    return null;
  }

  const max = Math.max(...recorded);
  const padding = Math.max(
    FAN_DOMAIN_MIN_PADDING,
    max * FAN_DOMAIN_PADDING_RATIO,
  );

  // Rounded up to a whole hundred so the axis reads 0 / 1000 / 2000 rather
  // than 0 / 974 / 1947: RPM is read as a magnitude, and a tick derived
  // from whatever the fastest fan happened to hit reads as a measurement
  // of its own.
  return [
    0,
    Math.ceil((max + padding) / FAN_DOMAIN_MIN_PADDING) *
      FAN_DOMAIN_MIN_PADDING,
  ];
};

/**
 * What is known about this machine's fan readings, from the currently
 * routed period.
 *
 * Three states rather than a boolean, for the same reason
 * [`resolveRoutedPowerCapability`] needs them:
 * - `present`: the window carries fan readings. The lane renders.
 * - `absent`: the window recorded *something* and none of it was a fan
 *   reading, which is real evidence of no readable fan source.
 * - `unknown`: the fetch has not resolved, it failed, or the window
 *   recorded nothing at all. Nothing may be claimed either way.
 */
export type FanCapability = "unknown" | "present" | "absent";

const archiveFanSeriesHasReadings = (
  series: readonly FanArchiveSeries[],
): boolean => series.some((entry) => hasFiniteValue(entry.points));

/**
 * Resolve [`FanCapability`] for the currently routed period, answered from
 * the fetched sources rather than from built rows - the same split the
 * power capability uses, so the pending-sensors note beside the timeline
 * and the lane's own gate cannot disagree.
 *
 * Each route reads only its own fan source. A 24h window on a machine whose
 * fan sensor stopped months ago must not inherit `present` from the daily
 * trend: the lane it would be describing is the one for *this* window.
 */
export const resolveRoutedFanCapability = (
  route: { kind: "archive" | "dailyTrend" },
  archive: {
    fanSeries: readonly FanArchiveSeries[];
    cpuSeries: ArchiveTimelineSeries;
    hasLoaded: boolean;
    hasError: boolean;
  },
  daily: {
    fanSeries: readonly CoolingFanTrendSeries[] | null;
    /** The CPU-side trend, the evidence that the window recorded at all. */
    recordedDays: number | null;
    hasError: boolean;
  },
): FanCapability => {
  if (route.kind === "archive") {
    if (archive.hasError || !archive.hasLoaded) {
      return "unknown";
    }
    if (archiveFanSeriesHasReadings(archive.fanSeries)) {
      return "present";
    }
    // A window that recorded nothing at all says nothing about the
    // machine's sensors - only that the app was not running.
    return archiveWindowRecordedAnything(archive.cpuSeries)
      ? "absent"
      : "unknown";
  }

  if (daily.hasError || daily.fanSeries == null || daily.recordedDays == null) {
    return "unknown";
  }
  if (daily.fanSeries.some((entry) => entry.days.length > 0)) {
    return "present";
  }
  // A rollup row exists only for a day that was actually recorded, so any
  // summarized day is evidence; an empty trend is not.
  return daily.recordedDays > 0 ? "absent" : "unknown";
};

/**
 * Whether the pending-sensors note and the data-state row may name the fan
 * as unsupported.
 *
 * Only `absent` licenses that claim. `unknown` deliberately reads the same
 * as `present`: both leave the fan unmentioned, which under-claims rather
 * than telling a user with working fan sensors that their machine has none
 * while the fetch is still in flight.
 */
export const claimsFanUnsupported = (capability: FanCapability): boolean =>
  capability === "absent";
