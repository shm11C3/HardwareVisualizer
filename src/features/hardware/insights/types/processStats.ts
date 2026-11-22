import type { ProcessGpuStats } from "@/rspc/bindings";

export type ProcessStat = {
  pid: number;
  process_name: string;
  avg_cpu_usage: number;
  avg_memory_usage: number;
  total_execution_sec: number;
  latest_timestamp: string;
};

/**
 * A Process Insight table row: the archived process stats joined with the
 * process's archived GPU usage for the same period. `avg_gpu_usage` is null
 * when the period has no GPU record for this process identity; 0 is a real
 * reading.
 */
export type ProcessInsightRow = ProcessStat & {
  avg_gpu_usage: number | null;
};

/**
 * The query window a Process Insight read answers: `period` minutes ending at
 * `endAt`. The table joins two reads only when their windows are equal.
 */
export type ProcessStatsWindow = {
  period: number;
  endAt: Date;
};

/** The latest per-process GPU read and the window it answers. */
export type ProcessGpuRead = { window: ProcessStatsWindow } & (
  | { status: "ready"; stats: ProcessGpuStats }
  | { status: "failed" }
);
