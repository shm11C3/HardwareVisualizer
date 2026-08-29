/**
 * Single top-of-view period selector for the Cooling Insight tab (#2018).
 * Distinct from `ArchivePeriod` (minutes, used by the per-chart selectors on
 * the other Insights tabs): 90d/1y have no archive-bucket equivalent and
 * route to the daily rollup instead (see `resolveCoolingPeriodRoute`).
 */
export const coolingInsightPeriods = ["24h", "7d", "30d", "90d", "1y"] as const;

export type CoolingInsightPeriod = (typeof coolingInsightPeriods)[number];

export const isCoolingInsightPeriod = (
  value: unknown,
): value is CoolingInsightPeriod =>
  typeof value === "string" &&
  (coolingInsightPeriods as readonly string[]).includes(value);
