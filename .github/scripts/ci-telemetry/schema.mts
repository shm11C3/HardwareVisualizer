// Type-only contract shared by the ci-telemetry action (producer) and the
// aggregator (validator/consumer). No runtime code lives here: type
// stripping erases this whole file, so importing from it costs nothing at
// runtime while letting `tsc` prove both sides agree on one shape. See
// metrics.mts and markers.mts for why their import of this file is
// `import type`.

/** Whole-job CPU utilization, delta-weighted (see metrics.mts). */
export type CpuPct = {
  avg: number;
  p95: number;
  max: number;
};

/** Average/peak of a byte-valued sample series (e.g. memory used). */
export type ByteAvgMax = {
  avg: number;
  max: number;
};

/** Start/end/peak of a byte-valued series that trends over the job (disk). */
export type DiskUsedBytes = {
  start: number;
  end: number;
  max: number;
};

/** Fixed-size (<=60 bucket) resource timeline; see metrics.mts's buildTimeline. */
export type ResourceTimeline = {
  bucket_seconds: number;
  cpu_pct_avg: (number | null)[];
  cpu_pct_max: (number | null)[];
  mem_used_pct_max: (number | null)[];
};

/**
 * The CI_JOB_METRICS_JSON marker (schema 1): runner CPU/memory/disk sampled
 * by the ci-telemetry action for the lifetime of a job. Both the producer
 * (metrics.mts's summarizeSamples) and the consumer (markers.mts's
 * validateResources, which re-derives this from untrusted log text) conform
 * to this exact type, so a field rename on either side is a compile error on
 * the other. runner_os/runner_arch stay `string` here (not a closed union):
 * the producer writes through whatever GITHUB_ACTIONS gives it, and only the
 * validator narrows unrecognized values down to "unknown" for untrusted
 * input — narrowing on the producer side would change what a self-hosted
 * runner with a nonstandard value prints.
 */
export type JobResourcesMarker = {
  schema: 1;
  runner_os: string;
  runner_arch: string;
  cpu_count: number;
  interval_seconds: number;
  sample_count: number;
  duration_seconds: number;
  cpu_pct: CpuPct | null;
  mem_total_bytes: number | null;
  mem_used_bytes: ByteAvgMax | null;
  disk_total_bytes: number | null;
  disk_used_bytes: DiskUsedBytes | null;
  timeline: ResourceTimeline | null;
};

/** The CACHE_METRICS_JSON marker emitted by cache-sccache/report. */
export type SccacheMarker = {
  compile_requests: number;
  compile_requests_executed: number;
  rust_hits: number;
  rust_misses: number;
  rust_hit_rate_pct: number | null;
  cpp_hits: number;
  cpp_misses: number;
  cpp_hit_rate_pct: number | null;
  cache_errors: number;
  backend: "s3" | "disk" | "other";
  target_cache_exact_hit: boolean | null;
};

/** The RUST_CACHE_METRICS_JSON marker emitted by setup-rust. */
export type RustCacheMarker = {
  shared_key: string | null;
  exact_hit: boolean;
  restore_seconds: number;
  targets: boolean;
};

/** The NODE_CACHE_METRICS_JSON marker emitted by setup-node. */
export type NodeCacheMarker = {
  exact_hit: boolean;
};

/** Everything markers.mts's extractMarkers can pull out of one job log. */
export type ExtractedMarkers = {
  resources: JobResourcesMarker | null;
  sccache: SccacheMarker | null;
  rust_cache: RustCacheMarker | null;
  node_cache: NodeCacheMarker | null;
};

export type StepRecord = {
  number: number;
  name: string;
  conclusion: string | null;
  duration_seconds: number | null;
};

export type JobRecord = {
  id: number;
  name: string;
  status: string;
  conclusion: string | null;
  labels: string[];
  created_at: string | null;
  started_at: string | null;
  completed_at: string | null;
  queue_seconds: number | null;
  duration_seconds: number | null;
  steps: StepRecord[];
  resources: JobResourcesMarker | null;
  caches: {
    rust_target: RustCacheMarker | null;
    node: NodeCacheMarker | null;
    sccache: SccacheMarker | null;
  };
};

/** The versioned "run record" written to records.json (schema 1). */
export type RunRecord = {
  schema: 1;
  repository: string;
  workflow: {
    id: number;
    name: string;
    path: string;
  };
  run: {
    id: number;
    attempt: number;
    number: number;
    event: string;
    status: string;
    conclusion: string | null;
    head_branch: string | null;
    head_sha: string;
    head_repository: string | null;
    is_fork: boolean;
    actor: string | null;
    created_at: string | null;
    started_at: string | null;
    completed_at: string | null;
    duration_seconds: number | null;
    url: string;
  };
  jobs: JobRecord[];
};

/** A still-running workflow run: shown in the summary table without a details block. */
export type PendingRunEntry = {
  workflow: {
    id: number;
    name: string;
    path: string;
  };
  run: {
    id: number;
    attempt: number;
    number: number;
    status: string;
    conclusion: string | null;
    url: string;
  };
};
