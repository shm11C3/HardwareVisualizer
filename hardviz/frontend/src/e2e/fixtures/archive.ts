import type {
  ArchiveBucketTimestamp,
  ArchiveSeriesPoint,
  ProcessGpuStats,
  ProcessStatRecord,
} from "@/rspc/bindings";

export type ArchiveSeriesOptions = {
  /**
   * Drop every Nth bucket, the way the archive omits a bucket it recorded
   * nothing for. Lets captures show that a gap stays a gap.
   */
  gapEvery?: number;
};

/**
 * Synthesize the compact series returned by the Core archive API.
 * Values follow a fixed sine wave and timestamps honor the requested bucket
 * width/alignment, so results remain stable for a fixed test clock.
 */
export const buildArchiveSeries = (
  start: string,
  end: string,
  bucketWidthMs: number,
  bucketTimestamp: ArchiveBucketTimestamp,
  base: number,
  amplitude: number,
  options: ArchiveSeriesOptions = {},
): ArchiveSeriesPoint[] => {
  const startMs = Date.parse(start);
  const endMs = Date.parse(end);

  if (
    Number.isNaN(startMs) ||
    Number.isNaN(endMs) ||
    endMs < startMs ||
    bucketWidthMs <= 0
  ) {
    return [];
  }

  const firstBucket =
    bucketTimestamp === "start"
      ? Math.floor(startMs / bucketWidthMs) * bucketWidthMs
      : Math.ceil((startMs - 60_000) / bucketWidthMs) * bucketWidthMs;
  const lastBucket =
    bucketTimestamp === "start"
      ? Math.floor(endMs / bucketWidthMs) * bucketWidthMs
      : Math.ceil(endMs / bucketWidthMs) * bucketWidthMs;
  const series: ArchiveSeriesPoint[] = [];

  const { gapEvery } = options;

  for (let t = firstBucket, i = 0; t <= lastBucket; t += bucketWidthMs, i++) {
    if (gapEvery != null && gapEvery > 0 && i % gapEvery === gapEvery - 1) {
      continue;
    }

    series.push({
      value: Math.round((base + amplitude * Math.sin(i / 5)) * 10) / 10,
      timestamp: t,
    });
  }

  return series;
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

const GPU_RECORDED_FOR_MS = 2 * 24 * 60 * 60 * 1000;

/**
 * The `get_process_gpu_stats` companion of `buildProcessStats`, keyed by the
 * same (pid, process_name) identities: one ordinary reading, one above 100
 * (summed across GPUs and engines), a real 0, and no record for
 * `fixture-daemon`. `fixture-gpu-helper` reuses the editor's PID under its
 * own name and has no process stats row, so it must not attach to the
 * editor. GPU usage was first recorded two days before `endAt`, so periods
 * longer than that show the coverage note.
 */
export const buildProcessGpuStats = (endAt: string): ProcessGpuStats => ({
  kind: "available",
  records: [
    {
      pid: 100,
      process_name: "hv-fixture-app",
      avg_gpu_usage: 3.42,
      covered_seconds: 5400,
    },
    {
      pid: 200,
      process_name: "fixture-browser",
      avg_gpu_usage: 112.48,
      covered_seconds: 3600,
    },
    {
      pid: 300,
      process_name: "fixture-editor",
      avg_gpu_usage: 0,
      covered_seconds: 1800,
    },
    {
      pid: 300,
      process_name: "fixture-gpu-helper",
      avg_gpu_usage: 55,
      covered_seconds: 600,
    },
  ],
  recorded_since: new Date(
    new Date(endAt).getTime() - GPU_RECORDED_FOR_MS,
  ).toISOString(),
});
