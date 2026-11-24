import type { HardwareArchiveSettings } from "@/rspc/bindings";

const MINUTES_PER_DAY = 24 * 60;

/**
 * Whether part of a selected period lies before what the Hardware Archive
 * still keeps. Those buckets render as gaps, which is honest but unexplained,
 * so the view names the Retention Period beside them (DP-02). Nothing is
 * deleted while scheduled deletion is off, and an unknown Retention Period
 * proves nothing, so neither case claims missing history.
 */
export const periodExceedsRetention = (
  periodMinutes: number,
  hardwareArchive: Pick<
    HardwareArchiveSettings,
    "scheduledDataDeletion" | "retentionDays"
  >,
): boolean => {
  const { scheduledDataDeletion, retentionDays } = hardwareArchive;
  if (!scheduledDataDeletion || retentionDays == null) {
    return false;
  }
  return periodMinutes > retentionDays * MINUTES_PER_DAY;
};
