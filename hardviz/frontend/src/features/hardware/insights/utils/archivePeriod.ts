import { chartConfig } from "@/consts/chart";
import {
  archivePeriods,
  type processStatsPeriods,
} from "@/features/hardware/consts/chart";

export type ArchivePeriod = (typeof archivePeriods)[number];
export type ProcessStatsPeriod = (typeof processStatsPeriods)[number];

/**
 * How many archive minutes one chart point covers for each period. The long
 * periods use 6 h and 12 h buckets rather than a day: Core floors buckets on
 * the UTC epoch grid, so a 1-day bucket would be a UTC day, not the local day
 * Cooling Insight's daily rollup means (DP-05).
 */
const bucketMinutesByPeriod: Record<ArchivePeriod, number> = {
  10: 1,
  30: 1,
  60: 1,
  180: 1,
  720: 10,
  1440: 30,
  10080: 60,
  20160: 180,
  43200: 720,
  129600: 360,
  525600: 720,
};

export const insightBucketWidthMs = (period: ArchivePeriod): number =>
  bucketMinutesByPeriod[period] * chartConfig.archiveUpdateIntervalMilSec;

const MIN_REFRESH_MS = chartConfig.archiveUpdateIntervalMilSec;
const MAX_REFRESH_MS = 5 * chartConfig.archiveUpdateIntervalMilSec;

/**
 * How often the current window re-reads the archive: at most once per new
 * archive minute, and no more often than every five minutes once a bucket is
 * wider than that, because a wide bucket barely moves within a minute and a
 * year-long read should not repeat every minute for every chart (DP-04).
 */
export const insightRefreshIntervalMs = (period: ArchivePeriod): number =>
  Math.min(
    Math.max(insightBucketWidthMs(period), MIN_REFRESH_MS),
    MAX_REFRESH_MS,
  );

const archivePeriodValues = new Set<number>(archivePeriods);

export const normalizeArchivePeriod = (
  value: ArchivePeriod | number | string | null | undefined,
): ArchivePeriod | null => {
  if (value == null) {
    return null;
  }

  const numericValue = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numericValue)) {
    return null;
  }

  return archivePeriodValues.has(numericValue)
    ? (numericValue as ArchivePeriod)
    : null;
};

export const coercePeriodMinutes = (value: number | string): number => {
  const numericValue = typeof value === "number" ? value : Number(value);
  if (
    !Number.isInteger(numericValue) ||
    numericValue < 0 ||
    numericValue > 4_294_967_295
  ) {
    throw new Error(`Invalid period minutes: ${String(value)}`);
  }

  return numericValue;
};
