import { chartConfig } from "@/consts/chart";
import type {
  ProcessInsightRow,
  ProcessStat,
} from "@/features/hardware/insights/types/processStats";
import type { ProcessGpuStatRecord } from "@/rspc/bindings";

// Archive identity is (pid, process_name); a PID can be reused (ADR 0019).
// Both commands group the same window by that pair, so each side has at most
// one record per key.
const processKey = (process: { pid: number; process_name: string }) =>
  JSON.stringify([process.pid, process.process_name]);

/**
 * Attach each process's archived GPU usage. A process with no GPU record in
 * the period gets null, never 0: 0 is a real reading.
 */
export const joinProcessGpuUsage = (
  processStats: readonly ProcessStat[],
  gpuRecords: readonly ProcessGpuStatRecord[],
): ProcessInsightRow[] => {
  const gpuUsage = new Map(
    gpuRecords.map((record) => [processKey(record), record.avg_gpu_usage]),
  );
  return processStats.map((process) => ({
    ...process,
    avg_gpu_usage: gpuUsage.get(processKey(process)) ?? null,
  }));
};

/**
 * The first instant `get_process_stats` and `get_process_gpu_stats` read for
 * `period` minutes ending at `endAt`: the window ends one archive interval
 * before `endAt` (the newest complete minute).
 */
export const processStatsWindowStart = (period: number, endAt: Date): Date =>
  new Date(
    endAt.getTime() - chartConfig.archiveUpdateIntervalMilSec - period * 60_000,
  );

export type ProcessGpuCoverage =
  /** GPU usage was recorded for the whole period. */
  | { kind: "complete" }
  /** The period starts before the first GPU record in the archive. */
  | { kind: "recordedSince"; recordedSince: Date }
  /** The archive has no GPU record at all yet. */
  | { kind: "notRecordedYet" };

/**
 * Whether the period reaches back before GPU usage was first recorded, so a
 * missing GPU value can mean "not recorded then" rather than "not used".
 * `recordedSince` is the earliest GPU record in the whole archive.
 */
export const processGpuCoverage = (
  recordedSince: string | null,
  windowStart: Date,
): ProcessGpuCoverage => {
  if (recordedSince == null) {
    return { kind: "notRecordedYet" };
  }
  const since = new Date(recordedSince);
  return since.getTime() > windowStart.getTime()
    ? { kind: "recordedSince", recordedSince: since }
    : { kind: "complete" };
};
