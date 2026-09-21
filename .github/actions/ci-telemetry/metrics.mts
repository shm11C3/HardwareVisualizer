// Pure functions only: no fs/process access, so these are unit-testable
// without a real job environment. main.mts, sampler.mts, and post.mts do the
// I/O and pass this module plain data.

// Type-only: erased at runtime, so this stays the action's only dependency
// outside its own directory. A *runtime* import reaching outside
// .github/actions/ci-telemetry/ is not allowed (see action.yml/AGENTS.md on
// keeping local actions dependency-free) — this import proves at compile
// time that the marker this file builds still matches what
// .github/scripts/ci-telemetry/markers.mts validates, without costing
// anything once types are stripped.
import type {
  ByteAvgMax,
  CpuPct,
  CpuThreadStat,
  DiskUsedBytes,
  JobResourcesMarker,
  ResourceTimeline,
} from "../../scripts/ci-telemetry/schema.mts";

type Sample = {
  t: number;
  cpu_idle: number;
  cpu_total: number;
  // null covers both a pre-change sample (sampler.mts didn't produce this
  // field yet) and a malformed value in an otherwise-parseable sample line;
  // parseSamples collapses both to null rather than dropping the sample.
  cpu_threads: [number, number][] | null;
  mem_used: number | null;
  mem_total: number;
  disk_used: number | null;
  disk_total: number | null;
};

type CpuPair = {
  t: number;
  pct: number;
  deltaIdle: number;
  deltaTotal: number;
};

// Per-thread pairs, keyed by thread index, in the same shape as the
// whole-VM CpuPair list. Reused by both the marker's cpu_threads summary
// and its per-thread timeline so the two stay derived from identical data.
type CpuThreadPairs = CpuPair[][];

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

// A torn write or a future sampler change could leave cpu_threads absent or
// misshapen on an otherwise-valid sample line; that must not sink the whole
// sample (parseSamples already tolerates a torn *line*, this tolerates a
// torn *field*), so an invalid value here becomes null rather than a reason
// to reject the sample.
function parseCpuThreads(value: unknown): [number, number][] | null {
  if (!Array.isArray(value)) return null;
  const pairs: [number, number][] = [];
  for (const entry of value) {
    if (
      !Array.isArray(entry) ||
      entry.length !== 2 ||
      typeof entry[0] !== "number" ||
      typeof entry[1] !== "number"
    ) {
      return null;
    }
    pairs.push([entry[0], entry[1]]);
  }
  return pairs;
}

export function resolveIntervalSeconds(rawValue: string | undefined): number {
  // Number.parseInt does its own ToString coercion, so this matches the
  // original's untyped `Number.parseInt(rawValue, 10)` even when rawValue is
  // undefined (parseInt("undefined", 10) is NaN, same as before).
  const parsed = Number.parseInt(String(rawValue), 10);
  if (!Number.isFinite(parsed)) return 5;
  return Math.min(60, Math.max(1, parsed));
}

// The sampler process can be killed mid-write (job cancelled, runner
// recycled), which leaves a torn final line. Skip anything that is not a
// complete, valid sample rather than failing the whole report.
export function parseSamples(text: string): Sample[] {
  const samples: Sample[] = [];
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    let parsed: unknown;
    try {
      parsed = JSON.parse(trimmed);
    } catch {
      continue;
    }
    if (!isPlainObject(parsed)) continue;
    if (
      typeof parsed["t"] !== "number" ||
      typeof parsed["cpu_idle"] !== "number" ||
      typeof parsed["cpu_total"] !== "number" ||
      typeof parsed["mem_total"] !== "number"
    ) {
      continue;
    }
    const memUsed = parsed["mem_used"];
    const diskUsed = parsed["disk_used"];
    const diskTotal = parsed["disk_total"];
    samples.push({
      t: parsed["t"],
      cpu_idle: parsed["cpu_idle"],
      cpu_total: parsed["cpu_total"],
      cpu_threads: parseCpuThreads(parsed["cpu_threads"]),
      mem_used: typeof memUsed === "number" ? memUsed : null,
      mem_total: parsed["mem_total"],
      disk_used: typeof diskUsed === "number" ? diskUsed : null,
      disk_total: typeof diskTotal === "number" ? diskTotal : null,
    });
  }
  return samples;
}

function round1(value: number): number {
  return Math.round(value * 10) / 10;
}

function clampPct(value: number): number {
  return Math.min(100, Math.max(0, value));
}

// Nearest-rank percentile over an already-sorted-ascending array.
function nearestRank(
  sortedAscending: number[],
  percentile: number,
): number | null {
  const n = sortedAscending.length;
  if (n === 0) return null;
  const rank = Math.min(n, Math.max(1, Math.ceil((percentile / 100) * n)));
  // rank is always in [1, n] by construction, so this index is always
  // populated; the `?? null` only matters to the type checker.
  return sortedAscending[rank - 1] ?? null;
}

// CPU % per consecutive sample pair, from cumulative os.cpus() counters.
// Pairs with a non-positive total delta (counter reset, or two samples that
// raced to the same tick) carry no usable information and are dropped
// rather than distorting the average with a divide-by-zero or negative rate.
function summarizeCpuPairs(samples: Sample[]): CpuPair[] {
  const pairs: CpuPair[] = [];
  for (let i = 1; i < samples.length; i++) {
    const prev = samples[i - 1];
    const cur = samples[i];
    // i ranges over [1, samples.length - 1], so both indices are always
    // populated; this check exists only to satisfy the type checker.
    if (prev === undefined || cur === undefined) continue;
    const deltaTotal = cur.cpu_total - prev.cpu_total;
    if (deltaTotal <= 0) continue;
    const deltaIdle = cur.cpu_idle - prev.cpu_idle;
    const pct = clampPct((1 - deltaIdle / deltaTotal) * 100);
    pairs.push({ t: cur.t, pct, deltaIdle, deltaTotal });
  }
  return pairs;
}

// avg is the whole-job figure (total idle time over total elapsed time), not
// the mean of the pair percentages, so a few long intervals cannot be
// out-voted by many short ones covering less wall-clock time.
function summarizeCpuPct(pairs: CpuPair[]): CpuPct | null {
  if (pairs.length === 0) return null;

  let sumIdle = 0;
  let sumTotal = 0;
  const sortedPct: number[] = [];
  for (const pair of pairs) {
    sumIdle += pair.deltaIdle;
    sumTotal += pair.deltaTotal;
    sortedPct.push(pair.pct);
  }
  sortedPct.sort((a, b) => a - b);

  return {
    avg: round1((1 - sumIdle / sumTotal) * 100),
    // sortedPct is non-empty here (pairs.length > 0), so nearestRank never
    // actually returns null; the `?? 0` mirrors what the untyped original
    // would have computed (null coerces to 0 in arithmetic) if it somehow did.
    p95: round1(nearestRank(sortedPct, 95) ?? 0),
    // sortedPct is sorted ascending, so its max is also its last element;
    // Math.max avoids indexing it a second time.
    max: round1(Math.max(...sortedPct)),
  };
}

const MAX_CPU_THREADS = 64; // marker size budget; see summarizeSamples.
const MAX_TIMELINE_THREADS = 8; // a 64-thread timeline would blow the budget.

// Same delta logic as summarizeCpuPairs, but for one logical CPU's own
// [idle, total] counters instead of the whole-VM sum, so per-thread avg is
// independent of how busy the other threads were.
function summarizeCpuThreadPairs(
  samples: Sample[],
  threadIndex: number,
): CpuPair[] {
  const pairs: CpuPair[] = [];
  for (let i = 1; i < samples.length; i++) {
    const prev = samples[i - 1];
    const cur = samples[i];
    if (prev === undefined || cur === undefined) continue;
    const prevPair = prev.cpu_threads?.[threadIndex];
    const curPair = cur.cpu_threads?.[threadIndex];
    if (prevPair === undefined || curPair === undefined) continue;
    const deltaTotal = curPair[1] - prevPair[1];
    if (deltaTotal <= 0) continue;
    const deltaIdle = curPair[0] - prevPair[0];
    const pct = clampPct((1 - deltaIdle / deltaTotal) * 100);
    pairs.push({ t: cur.t, pct, deltaIdle, deltaTotal });
  }
  return pairs;
}

// avg is delta-weighted over the whole job, same method as summarizeCpuPct;
// max is the largest single-pair percentage. A thread with zero usable
// pairs (every delta total <= 0 for its whole lifetime — practically only
// an offline CPU) is null: that thread was never actually measured, so it
// must not be indistinguishable from a thread that was measured and found
// idle (see docs/development/ci-telemetry.md: unavailable is null, never 0).
// Every pair that does reach here already has deltaTotal > 0 (dropped
// earlier by summarizeCpuThreadPairs), so sumTotal is always > 0 once
// pairs.length > 0.
function summarizeCpuThreadStat(pairs: CpuPair[]): CpuThreadStat | null {
  if (pairs.length === 0) return null;
  let sumIdle = 0;
  let sumTotal = 0;
  let max = 0;
  for (const pair of pairs) {
    sumIdle += pair.deltaIdle;
    sumTotal += pair.deltaTotal;
    if (pair.pct > max) max = pair.pct;
  }
  return {
    avg: round1(clampPct((1 - sumIdle / sumTotal) * 100)),
    max: round1(max),
  };
}

// Resolves per-thread stats plus the pair lists a per-thread timeline can be
// bucketed from. Both come back null together whenever cpu_threads is
// unavailable: fewer than 2 samples, any sample missing cpu_threads (a
// pre-change job, or one torn field parseSamples nulled out), a thread
// count that is not constant across every sample, or more threads than the
// marker's size budget allows. pairsByThread is additionally null when the
// (valid) thread count exceeds MAX_TIMELINE_THREADS, independent of stats.
// An individual stats entry can independently be null — see
// summarizeCpuThreadStat — without nulling the rest of the array.
function summarizeCpuThreads(samples: Sample[]): {
  stats: (CpuThreadStat | null)[] | null;
  pairsByThread: CpuThreadPairs | null;
} {
  if (samples.length < 2) return { stats: null, pairsByThread: null };

  let threadCount: number | null = null;
  for (const sample of samples) {
    const threads = sample.cpu_threads;
    if (!threads) return { stats: null, pairsByThread: null };
    if (threadCount === null) {
      threadCount = threads.length;
    } else if (threads.length !== threadCount) {
      return { stats: null, pairsByThread: null };
    }
  }
  if (!threadCount || threadCount > MAX_CPU_THREADS) {
    return { stats: null, pairsByThread: null };
  }

  const pairsByThread: CpuThreadPairs = Array.from(
    { length: threadCount },
    (_, idx) => summarizeCpuThreadPairs(samples, idx),
  );
  const stats = pairsByThread.map(summarizeCpuThreadStat);

  return {
    stats,
    pairsByThread: threadCount <= MAX_TIMELINE_THREADS ? pairsByThread : null,
  };
}

function summarizeByteAvgMax(samples: Sample[]): ByteAvgMax | null {
  const values = samples
    .map((sample) => sample.mem_used)
    .filter((value): value is number => typeof value === "number");
  if (values.length === 0) return null;
  return {
    avg: Math.round(
      values.reduce((sum, value) => sum + value, 0) / values.length,
    ),
    max: Math.round(Math.max(...values)),
  };
}

function summarizeDiskUsed(samples: Sample[]): DiskUsedBytes | null {
  const withValues = samples.filter(
    (sample): sample is Sample & { disk_used: number } =>
      typeof sample.disk_used === "number",
  );
  const first = withValues[0];
  const last = withValues[withValues.length - 1];
  // Equivalent to the withValues.length === 0 check this replaces: both are
  // undefined exactly when the array is empty.
  if (first === undefined || last === undefined) return null;
  return {
    start: Math.round(first.disk_used),
    end: Math.round(last.disk_used),
    max: Math.round(Math.max(...withValues.map((sample) => sample.disk_used))),
  };
}

function firstNonNull(
  samples: Sample[],
  key: "mem_total" | "disk_total",
): number | null {
  for (const sample of samples) {
    const value = sample[key];
    if (typeof value === "number") return Math.round(value);
  }
  return null;
}

// At most 60 equal-width buckets spanning the first..last sample, so the
// timeline stays a small, fixed-size shape regardless of job length instead
// of growing with sample_count (a multi-hour job at a short interval would
// otherwise serialize thousands of points).
function buildTimeline(
  samples: Sample[],
  cpuPairs: CpuPair[],
  intervalSeconds: number,
  cpuThreadPairs: CpuThreadPairs | null,
): ResourceTimeline | null {
  if (samples.length < 2) return null;

  const firstSample = samples[0];
  const lastSample = samples[samples.length - 1];
  // samples.length >= 2 here, so both indices are always populated; this
  // check exists only to satisfy the type checker.
  if (firstSample === undefined || lastSample === undefined) return null;
  const firstT = firstSample.t;
  const lastT = lastSample.t;
  const spanSeconds = Math.max(0, (lastT - firstT) / 1000);
  const naturalBucketCount =
    spanSeconds > 0 ? Math.ceil(spanSeconds / intervalSeconds) : 1;
  const bucketCount = Math.min(60, Math.max(1, naturalBucketCount));
  const bucketSeconds = Math.max(
    intervalSeconds,
    Math.ceil(spanSeconds / bucketCount) || intervalSeconds,
  );

  function bucketIndex(t: number): number {
    const idx = Math.floor((t - firstT) / 1000 / bucketSeconds);
    return Math.min(bucketCount - 1, Math.max(0, idx));
  }

  const cpuBuckets: number[][] = Array.from({ length: bucketCount }, () => []);
  for (const pair of cpuPairs) {
    // bucketIndex always returns an index in [0, bucketCount - 1], so this
    // bucket always exists; a plain array is truthy regardless of contents.
    const bucket = cpuBuckets[bucketIndex(pair.t)];
    if (bucket) bucket.push(pair.pct);
  }

  const memTotal = firstNonNull(samples, "mem_total");
  const memBuckets: number[][] = Array.from({ length: bucketCount }, () => []);
  if (memTotal) {
    for (const sample of samples) {
      if (typeof sample.mem_used !== "number") continue;
      const bucket = memBuckets[bucketIndex(sample.t)];
      if (bucket) bucket.push((sample.mem_used / memTotal) * 100);
    }
  }

  const bucketAvg = (values: number[]): number | null =>
    values.length === 0
      ? null
      : Math.round(
          values.reduce((sum, value) => sum + value, 0) / values.length,
        );
  const bucketMax = (values: number[]): number | null =>
    values.length === 0 ? null : Math.round(Math.max(...values));

  // Bucketed with the exact same bucketIndex/bucketCount as cpu_pct_avg
  // above, so a future dashboard can overlay per-thread series on the
  // whole-VM one without re-deriving bucket boundaries.
  const cpuThreadPctAvg =
    cpuThreadPairs === null
      ? null
      : cpuThreadPairs.map((pairs) => {
          const buckets: number[][] = Array.from(
            { length: bucketCount },
            () => [],
          );
          for (const pair of pairs) {
            const bucket = buckets[bucketIndex(pair.t)];
            if (bucket) bucket.push(pair.pct);
          }
          return buckets.map(bucketAvg);
        });

  return {
    bucket_seconds: Math.round(bucketSeconds),
    cpu_pct_avg: cpuBuckets.map(bucketAvg),
    cpu_pct_max: cpuBuckets.map(bucketMax),
    mem_used_pct_max: memBuckets.map(bucketMax),
    cpu_thread_pct_avg: cpuThreadPctAvg,
  };
}

// Builds the CI_JOB_METRICS_JSON marker (schema 1). The aggregator workflow
// validates job logs against this exact shape, so field names/nesting must
// not change without updating that contract too.
export function summarizeSamples(
  samples: Sample[],
  options: {
    intervalSeconds: number;
    runnerOs: string;
    runnerArch: string;
    cpuCount: number;
  },
): JobResourcesMarker {
  const { intervalSeconds, runnerOs, runnerArch, cpuCount } = options;
  const sampleCount = samples.length;
  const lastSample = samples[sampleCount - 1];
  const firstSample = samples[0];
  const durationSeconds =
    sampleCount >= 2 && lastSample !== undefined && firstSample !== undefined
      ? Math.round((lastSample.t - firstSample.t) / 1000)
      : 0;

  const cpuPairs = summarizeCpuPairs(samples);
  const cpuThreads = summarizeCpuThreads(samples);

  return {
    schema: 1,
    runner_os: runnerOs,
    runner_arch: runnerArch,
    cpu_count: cpuCount,
    interval_seconds: intervalSeconds,
    sample_count: sampleCount,
    duration_seconds: durationSeconds,
    cpu_pct: summarizeCpuPct(cpuPairs),
    cpu_threads: cpuThreads.stats,
    mem_total_bytes: firstNonNull(samples, "mem_total"),
    mem_used_bytes: summarizeByteAvgMax(samples),
    disk_total_bytes: firstNonNull(samples, "disk_total"),
    disk_used_bytes: summarizeDiskUsed(samples),
    timeline: buildTimeline(
      samples,
      cpuPairs,
      intervalSeconds,
      cpuThreads.pairsByThread,
    ),
  };
}
