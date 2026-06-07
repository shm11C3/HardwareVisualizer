import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";
import type { ProcessStat } from "../../types/processStats";
import {
  type ArchivePeriod,
  coercePeriodMinutes,
} from "../../utils/archivePeriod";

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
    console.error("Failed to fetch process stats:", result.error);
    return [];
  }

  return result.data;
};
