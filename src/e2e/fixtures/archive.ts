import type { ArchiveRecord, ProcessStatRecord } from "@/rspc/bindings";

/**
 * Synthesize deterministic archive records for the requested time range.
 * Values follow a fixed sine wave; the step is chosen so a range yields at
 * most ~120 points. Timestamps are derived purely from the requested
 * start/end args, so results are stable for a fixed test clock.
 */
export const buildArchiveRecords = (
  start: string,
  end: string,
  base: number,
  amplitude: number,
): ArchiveRecord[] => {
  const startMs = Date.parse(start);
  const endMs = Date.parse(end);

  if (Number.isNaN(startMs) || Number.isNaN(endMs) || endMs <= startMs) {
    return [];
  }

  const stepMs = Math.max(60_000, Math.ceil((endMs - startMs) / 120));
  const records: ArchiveRecord[] = [];

  for (let t = startMs, i = 0; t <= endMs; t += stepMs, i++) {
    records.push({
      id: i + 1,
      value: Math.round((base + amplitude * Math.sin(i / 5)) * 10) / 10,
      timestamp: new Date(t).toISOString(),
    });
  }

  return records;
};

export const buildProcessStats = (
  latestTimestamp: string,
): ProcessStatRecord[] => [
  {
    pid: 100,
    process_name: "hv-fixture-app",
    avg_cpu_usage: 12.5,
    avg_memory_usage: 262_144,
    total_execution_sec: 5400,
    latest_timestamp: latestTimestamp,
  },
  {
    pid: 200,
    process_name: "fixture-browser",
    avg_cpu_usage: 8.1,
    avg_memory_usage: 1_258_291,
    total_execution_sec: 3600,
    latest_timestamp: latestTimestamp,
  },
  {
    pid: 300,
    process_name: "fixture-editor",
    avg_cpu_usage: 4.4,
    avg_memory_usage: 524_288,
    total_execution_sec: 1800,
    latest_timestamp: latestTimestamp,
  },
  {
    pid: 400,
    process_name: "fixture-daemon",
    avg_cpu_usage: 1.2,
    avg_memory_usage: 65_536,
    total_execution_sec: 86_400,
    latest_timestamp: latestTimestamp,
  },
];
