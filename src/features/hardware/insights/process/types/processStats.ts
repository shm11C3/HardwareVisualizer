export type ProcessStat = {
  pid: number;
  process_name: string;
  avg_cpu_usage: number;
  avg_memory_usage: number;
  total_execution_sec: number;
  latest_timestamp: string;
};
