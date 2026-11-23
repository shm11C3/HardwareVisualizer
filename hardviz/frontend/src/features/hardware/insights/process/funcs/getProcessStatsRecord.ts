import type { ProcessStat } from "@/features/hardware/insights/types/processStats";
import {
  type ArchivePeriod,
  coercePeriodMinutes,
} from "@/features/hardware/insights/utils/archivePeriod";
import { commands, type ProcessGpuStats } from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 *
 * @param period
 * @param endAt
 * @returns
 * @todo Also do sorting in SQL
 */
export const getProcessStats = async (
  period: ArchivePeriod | number | string,
  endAt: Date,
): Promise<ProcessStat[]> => {
  const result = await commands.getProcessStats(
    coercePeriodMinutes(period),
    endAt.toISOString(),
  );
  if (isError(result)) {
    throw new Error(`Failed to fetch process stats: ${result.error}`);
  }

  return result.data;
};

/**
 * Archived per-process GPU usage for the same window `getProcessStats` reads
 * for `period` and `endAt`. `needsNativeDatabase` is an answer, not an error.
 */
export const getProcessGpuStats = async (
  period: ArchivePeriod | number | string,
  endAt: Date,
): Promise<ProcessGpuStats> => {
  const result = await commands.getProcessGpuStats(
    coercePeriodMinutes(period),
    endAt.toISOString(),
  );
  if (isError(result)) {
    throw new Error(`Failed to fetch process GPU stats: ${result.error}`);
  }

  return result.data;
};
