import type { ProcessStat } from "@/features/hardware/insights/types/processStats";
import {
  type ArchivePeriod,
  coercePeriodMinutes,
} from "@/features/hardware/insights/utils/archivePeriod";
import { commands } from "@/rspc/bindings";
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
