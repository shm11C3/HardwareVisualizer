import { type ArchiveSeriesPoint, commands } from "@/rspc/bindings";
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

export const getArchivedRecord = async (
  hardwareType: "cpu" | "ram",
  start: Date,
  end: Date,
  bucketWidthMs: number,
): Promise<ArchiveSeriesPoint[]> => {
  const result = await commands.getDataArchiveSeries(
    hardwareType === "ram" ? "memory" : "cpu",
    "avg",
    start.toISOString(),
    end.toISOString(),
    bucketWidthMs,
    "start",
  );
  if (isError(result)) {
    throw new Error(
      `Failed to fetch archived hardware series: ${result.error}`,
    );
  }

  return result.data;
};

export const getProcessStatsInPeriod = async (
  start: Date,
  end: Date,
): Promise<ProcessStat[]> => {
  const result = await commands.getProcessStatsInPeriod(
    start.toISOString(),
    end.toISOString(),
  );
  if (isError(result)) {
    throw new Error(`Failed to fetch process stats in period: ${result.error}`);
  }

  return result.data;
};
