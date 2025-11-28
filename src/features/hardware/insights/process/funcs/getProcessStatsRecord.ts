import { chartConfig } from "@/features/hardware/consts/chart";
import { sqlitePromise } from "@/lib/sqlite";
import type { ProcessStat } from "../../types/processStats";

/**
 *
 * @param period
 * @param endAt
 * @returns
 * @todo Also do sorting in SQL
 */
export const getProcessStats = async (
  period: number,
  endAt: Date,
): Promise<ProcessStat[]> => {
  const adjustedEndAt = new Date(
    endAt.getTime() - chartConfig.archiveUpdateIntervalMilSec,
  );
  const startTime = new Date(adjustedEndAt.getTime() - period * 60 * 1000);
  const db = await sqlitePromise;

  const sql = `
    SELECT
      pid,
      process_name,
      AVG(cpu_usage) AS avg_cpu_usage,
      AVG(memory_usage) AS avg_memory_usage,
      MAX(execution_sec) AS total_execution_sec,
      MAX(timestamp) AS latest_timestamp
    FROM process_stats
    WHERE timestamp BETWEEN '${startTime.toISOString()}'
    AND '${adjustedEndAt.toISOString()}'
    GROUP BY pid, process_name
  `;

  return db.load(sql);
};
