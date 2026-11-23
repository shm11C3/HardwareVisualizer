import type { ProcessStat } from "@/features/hardware/insights/types/processStats";

const PROCESSES_PER_METRIC = 250;

const processKey = (stat: ProcessStat) =>
  JSON.stringify([stat.pid, stat.process_name]);

const compareIdentity = (a: ProcessStat, b: ProcessStat) =>
  a.pid - b.pid ||
  (a.process_name < b.process_name
    ? -1
    : a.process_name > b.process_name
      ? 1
      : 0);

export const getProcessBubbleData = (processStats: ProcessStat[]) => {
  // Archive queries group by (pid, process_name), not PID alone (ADR 0019).
  // Reused PIDs must remain distinct when combining the two rankings.
  const selected = new Set(
    (["avg_cpu_usage", "avg_memory_usage"] as const).flatMap((metric) =>
      [...processStats]
        .sort((a, b) => b[metric] - a[metric] || compareIdentity(a, b))
        .slice(0, PROCESSES_PER_METRIC)
        .map(processKey),
    ),
  );

  const allData = processStats.map((stat) => {
    const rawZ = (stat.avg_cpu_usage ?? 0) * stat.total_execution_sec;
    return {
      x: stat.total_execution_sec / 60,
      y: stat.avg_cpu_usage,
      z: Math.sqrt(rawZ),
      name: stat.process_name,
      pid: stat.pid,
      zRaw: rawZ,
      ram: stat.avg_memory_usage,
    };
  });

  // Keep scales and zoom based on the whole result, including omitted points.
  const data = allData.filter((_, index) =>
    selected.has(processKey(processStats[index])),
  );
  let maxX = 0;
  let maxZ = 0;
  for (const point of allData) {
    maxX = Math.max(maxX, point.x);
    maxZ = Math.max(maxZ, point.z);
  }

  return {
    allData,
    data,
    omittedCount: processStats.length - data.length,
    maxX,
    maxZ,
  };
};
