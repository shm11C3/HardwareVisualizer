// Pure functions only: no fs/process access, so these are unit-testable
// without a real job environment. main.cjs, sampler.cjs, and post.cjs do the
// I/O and pass this module plain data.

function resolveIntervalSeconds(rawValue) {
  const parsed = Number.parseInt(rawValue, 10);
  if (!Number.isFinite(parsed)) return 5;
  return Math.min(60, Math.max(1, parsed));
}

// The sampler process can be killed mid-write (job cancelled, runner
// recycled), which leaves a torn final line. Skip anything that is not a
// complete, valid sample rather than failing the whole report.
function parseSamples(text) {
  const samples = [];
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    let sample;
    try {
      sample = JSON.parse(trimmed);
    } catch {
      continue;
    }
    if (
      typeof sample.t !== "number" ||
      typeof sample.cpu_idle !== "number" ||
      typeof sample.cpu_total !== "number" ||
      typeof sample.mem_total !== "number"
    ) {
      continue;
    }
    samples.push(sample);
  }
  return samples;
}

function round1(value) {
  return Math.round(value * 10) / 10;
}

function clampPct(value) {
  return Math.min(100, Math.max(0, value));
}

// Nearest-rank percentile over an already-sorted-ascending array.
function nearestRank(sortedAscending, percentile) {
  const n = sortedAscending.length;
  if (n === 0) return null;
  const rank = Math.min(n, Math.max(1, Math.ceil((percentile / 100) * n)));
  return sortedAscending[rank - 1];
}

// CPU % per consecutive sample pair, from cumulative os.cpus() counters.
// Pairs with a non-positive total delta (counter reset, or two samples that
// raced to the same tick) carry no usable information and are dropped
// rather than distorting the average with a divide-by-zero or negative rate.
function summarizeCpuPairs(samples) {
  const pairs = [];
  for (let i = 1; i < samples.length; i++) {
    const prev = samples[i - 1];
    const cur = samples[i];
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
function summarizeCpuPct(pairs) {
  if (pairs.length === 0) return null;

  let sumIdle = 0;
  let sumTotal = 0;
  const sortedPct = [];
  for (const pair of pairs) {
    sumIdle += pair.deltaIdle;
    sumTotal += pair.deltaTotal;
    sortedPct.push(pair.pct);
  }
  sortedPct.sort((a, b) => a - b);

  return {
    avg: round1((1 - sumIdle / sumTotal) * 100),
    p95: round1(nearestRank(sortedPct, 95)),
    max: round1(sortedPct[sortedPct.length - 1]),
  };
}

function summarizeByteAvgMax(samples, key) {
  const values = samples
    .map((sample) => sample[key])
    .filter((value) => typeof value === "number");
  if (values.length === 0) return null;
  return {
    avg: Math.round(
      values.reduce((sum, value) => sum + value, 0) / values.length,
    ),
    max: Math.round(Math.max(...values)),
  };
}

function summarizeDiskUsed(samples) {
  const withValues = samples.filter(
    (sample) => typeof sample.disk_used === "number",
  );
  if (withValues.length === 0) return null;
  return {
    start: Math.round(withValues[0].disk_used),
    end: Math.round(withValues[withValues.length - 1].disk_used),
    max: Math.round(Math.max(...withValues.map((sample) => sample.disk_used))),
  };
}

function firstNonNull(samples, key) {
  for (const sample of samples) {
    if (typeof sample[key] === "number") return Math.round(sample[key]);
  }
  return null;
}

// At most 60 equal-width buckets spanning the first..last sample, so the
// timeline stays a small, fixed-size shape regardless of job length instead
// of growing with sample_count (a multi-hour job at a short interval would
// otherwise serialize thousands of points).
function buildTimeline(samples, cpuPairs, intervalSeconds) {
  if (samples.length < 2) return null;

  const firstT = samples[0].t;
  const lastT = samples[samples.length - 1].t;
  const spanSeconds = Math.max(0, (lastT - firstT) / 1000);
  const naturalBucketCount =
    spanSeconds > 0 ? Math.ceil(spanSeconds / intervalSeconds) : 1;
  const bucketCount = Math.min(60, Math.max(1, naturalBucketCount));
  const bucketSeconds = Math.max(
    intervalSeconds,
    Math.ceil(spanSeconds / bucketCount) || intervalSeconds,
  );

  function bucketIndex(t) {
    const idx = Math.floor((t - firstT) / 1000 / bucketSeconds);
    return Math.min(bucketCount - 1, Math.max(0, idx));
  }

  const cpuBuckets = Array.from({ length: bucketCount }, () => []);
  for (const pair of cpuPairs) {
    cpuBuckets[bucketIndex(pair.t)].push(pair.pct);
  }

  const memTotal = firstNonNull(samples, "mem_total");
  const memBuckets = Array.from({ length: bucketCount }, () => []);
  if (memTotal) {
    for (const sample of samples) {
      if (typeof sample.mem_used !== "number") continue;
      memBuckets[bucketIndex(sample.t)].push(
        (sample.mem_used / memTotal) * 100,
      );
    }
  }

  const bucketAvg = (values) =>
    values.length === 0
      ? null
      : Math.round(
          values.reduce((sum, value) => sum + value, 0) / values.length,
        );
  const bucketMax = (values) =>
    values.length === 0 ? null : Math.round(Math.max(...values));

  return {
    bucket_seconds: Math.round(bucketSeconds),
    cpu_pct_avg: cpuBuckets.map(bucketAvg),
    cpu_pct_max: cpuBuckets.map(bucketMax),
    mem_used_pct_max: memBuckets.map(bucketMax),
  };
}

// Builds the CI_JOB_METRICS_JSON marker (schema 1). The aggregator workflow
// validates job logs against this exact shape, so field names/nesting must
// not change without updating that contract too.
function summarizeSamples(
  samples,
  { intervalSeconds, runnerOs, runnerArch, cpuCount },
) {
  const sampleCount = samples.length;
  const durationSeconds =
    sampleCount >= 2
      ? Math.round((samples[sampleCount - 1].t - samples[0].t) / 1000)
      : 0;

  const cpuPairs = summarizeCpuPairs(samples);

  return {
    schema: 1,
    runner_os: runnerOs,
    runner_arch: runnerArch,
    cpu_count: cpuCount,
    interval_seconds: intervalSeconds,
    sample_count: sampleCount,
    duration_seconds: durationSeconds,
    cpu_pct: summarizeCpuPct(cpuPairs),
    mem_total_bytes: firstNonNull(samples, "mem_total"),
    mem_used_bytes: summarizeByteAvgMax(samples, "mem_used"),
    disk_total_bytes: firstNonNull(samples, "disk_total"),
    disk_used_bytes: summarizeDiskUsed(samples),
    timeline: buildTimeline(samples, cpuPairs, intervalSeconds),
  };
}

module.exports = { resolveIntervalSeconds, parseSamples, summarizeSamples };
