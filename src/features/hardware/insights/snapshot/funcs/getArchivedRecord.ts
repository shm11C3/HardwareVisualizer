import { type ArchiveSeriesPoint, commands } from "@/rspc/bindings";
import { isError } from "@/types/result";
import type { ProcessStat } from "../../types/processStats";

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
