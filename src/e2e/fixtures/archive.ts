import type {
  ArchiveBucketTimestamp,
  ArchiveSeriesPoint,
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
