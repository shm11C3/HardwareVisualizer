const MINUTES_PER_HOUR = 60;
const MINUTES_PER_DAY = 24 * MINUTES_PER_HOUR;

export type ArchivePeriodUnit = "minutes" | "hours" | "days";

export type ArchivePeriodDescription = {
  count: number;
  unit: ArchivePeriodUnit;
};

/**
 * The largest whole unit a period length can be read in, so the period
 * selector can say "7 days" instead of "10080 minutes". Pure so the label
 * rule is testable without i18n; the owning hook translates the unit.
 */
export const describeArchivePeriod = (
  minutes: number,
): ArchivePeriodDescription => {
  if (minutes % MINUTES_PER_DAY === 0) {
    return { count: minutes / MINUTES_PER_DAY, unit: "days" };
  }
  if (minutes % MINUTES_PER_HOUR === 0) {
    return { count: minutes / MINUTES_PER_HOUR, unit: "hours" };
  }
  return { count: minutes, unit: "minutes" };
};
