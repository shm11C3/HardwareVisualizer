const assert = require("node:assert/strict");
const {
  parseSamples,
  summarizeSamples,
} = require("../actions/ci-telemetry/metrics.cjs");

const baseOptions = {
  intervalSeconds: 5,
  runnerOs: "Linux",
  runnerArch: "X64",
  cpuCount: 4,
};

// Whole-job cpu_pct.avg must be delta-weighted (total idle / total elapsed),
// not the mean of per-pair percentages: a short high-load interval and a
// long low-load interval must not count equally just because each is "one
// pair". Interval 1: 1s at 90% busy. Interval 2: 10s at 9% busy.
// Mean-of-pairs would give (90+9)/2=49.5; the delta-weighted figure is
// 1-(1+91)/(10+100)=16.4.
{
  const samples = [
    {
      t: 0,
      cpu_idle: 0,
      cpu_total: 0,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    {
      t: 1000,
      cpu_idle: 1,
      cpu_total: 10,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    {
      t: 11000,
      cpu_idle: 92,
      cpu_total: 110,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
  ];
  const marker = summarizeSamples(samples, baseOptions);
  assert.equal(marker.cpu_pct.avg, 16.4);
  assert.equal(marker.cpu_pct.max, 90);
  // Only two pairs (90, 9): nearest-rank p95 over 2 values takes the higher one.
  assert.equal(marker.cpu_pct.p95, 90);
}

// p95 must be the nearest-rank percentile of pair values, distinct from max.
// 100 uniform-interval pairs with pct 1..100 (sorted already): nearest-rank
// p95 = ceil(0.95*100)=95th smallest = 95, while max stays 100.
{
  const samples = [
    {
      t: 0,
      cpu_idle: 0,
      cpu_total: 0,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
  ];
  let idle = 0;
  let total = 0;
  for (let pct = 1; pct <= 100; pct++) {
    // deltaTotal=100 always; deltaIdle chosen so (1-deltaIdle/100)*100 === pct.
    total += 100;
    idle += 100 - pct;
    samples.push({
      t: pct * 1000,
      cpu_idle: idle,
      cpu_total: total,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    });
  }
  const marker = summarizeSamples(samples, baseOptions);
  assert.equal(marker.cpu_pct.max, 100);
  assert.equal(marker.cpu_pct.p95, 95);
}

// Fewer than 2 samples cannot form a pair, so cpu_pct and timeline must be
// null (unavailable), never 0 or an empty-but-present shape: a null must
// not be able to be mistaken for "runner was idle".
{
  const empty = summarizeSamples([], baseOptions);
  assert.equal(empty.cpu_pct, null);
  assert.equal(empty.timeline, null);
  assert.equal(empty.sample_count, 0);
  // An empty sample file has no memory reading either; total memory must be
  // unavailable, not a 0-byte runner.
  assert.equal(empty.mem_total_bytes, null);

  const one = summarizeSamples(
    [
      {
        t: 0,
        cpu_idle: 0,
        cpu_total: 0,
        mem_used: 100,
        mem_total: 1000,
        disk_used: 1,
        disk_total: 10,
      },
    ],
    baseOptions,
  );
  assert.equal(one.cpu_pct, null);
  assert.equal(one.timeline, null);
}

// mem_used/disk_used samples that are null (platform read failed for that
// tick) must be ignored rather than treated as 0, and when every sample is
// null the whole field must be null rather than an avg/max of zeros.
{
  const samples = [
    {
      t: 0,
      cpu_idle: 0,
      cpu_total: 100,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: 500,
    },
    {
      t: 1000,
      cpu_idle: 50,
      cpu_total: 200,
      mem_used: 200,
      mem_total: 1000,
      disk_used: 50,
      disk_total: 500,
    },
    {
      t: 2000,
      cpu_idle: 100,
      cpu_total: 300,
      mem_used: 400,
      mem_total: 1000,
      disk_used: 70,
      disk_total: 500,
    },
  ];
  const marker = summarizeSamples(samples, baseOptions);
  assert.deepEqual(marker.mem_used_bytes, { avg: 300, max: 400 });
  assert.deepEqual(marker.disk_used_bytes, { start: 50, end: 70, max: 70 });

  const allNull = samples.map((sample) => ({
    ...sample,
    mem_used: null,
    disk_used: null,
  }));
  const allNullMarker = summarizeSamples(allNull, baseOptions);
  assert.equal(allNullMarker.mem_used_bytes, null);
  assert.equal(allNullMarker.disk_used_bytes, null);
}

// The sampler can be killed mid-write, leaving a torn final JSONL line.
// parseSamples must skip it (and blank lines) instead of throwing.
{
  const valid1 = JSON.stringify({
    t: 0,
    cpu_idle: 0,
    cpu_total: 10,
    mem_used: 1,
    mem_total: 10,
    disk_used: null,
    disk_total: null,
  });
  const valid2 = JSON.stringify({
    t: 1000,
    cpu_idle: 1,
    cpu_total: 20,
    mem_used: 2,
    mem_total: 10,
    disk_used: null,
    disk_total: null,
  });
  const torn = '{"t":2000,"cpu_idle":2,"cpu_tot';
  const text = `${valid1}\n\n${valid2}\n${torn}`;
  const samples = parseSamples(text);
  assert.equal(samples.length, 2);
  assert.equal(samples[1].t, 1000);
}

// A long job (3h at a 5s interval = 2161 samples) must still collapse to at
// most 60 timeline buckets, and the whole marker line must stay comfortably
// under the ~4KB budget the aggregator/PR-comment path expects.
{
  const samples = [];
  let idle = 0;
  let total = 0;
  for (let t = 0; t <= 3 * 60 * 60 * 1000; t += 5000) {
    idle += 3;
    total += 5; // constant 40% busy
    samples.push({
      t,
      cpu_idle: idle,
      cpu_total: total,
      mem_used: 4_000_000_000 + (t % 1_000_000_000),
      mem_total: 16_000_000_000,
      disk_used: 60_000_000_000,
      disk_total: 150_000_000_000,
    });
  }
  const marker = summarizeSamples(samples, baseOptions);
  assert.equal(marker.sample_count, samples.length);
  assert.ok(marker.timeline.cpu_pct_avg.length <= 60);
  assert.equal(
    marker.timeline.cpu_pct_avg.length,
    marker.timeline.cpu_pct_max.length,
  );
  assert.equal(
    marker.timeline.cpu_pct_avg.length,
    marker.timeline.mem_used_pct_max.length,
  );
  assert.ok(marker.timeline.bucket_seconds >= baseOptions.intervalSeconds);
  assert.ok(JSON.stringify(marker).length < 4096);
}

// A counter reset (cpu_total goes backwards, e.g. a resumed/replaced VM
// clock) or two samples racing to the same total must not corrupt the
// average: pairs with deltaTotal <= 0 are dropped entirely rather than
// counted as 0% or negative load.
{
  const samples = [
    {
      t: 0,
      cpu_idle: 0,
      cpu_total: 100,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    // Reset: total drops below the previous sample.
    {
      t: 1000,
      cpu_idle: 0,
      cpu_total: 50,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    // Stalled: identical total (deltaTotal == 0).
    {
      t: 2000,
      cpu_idle: 0,
      cpu_total: 50,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    // Only this pair (50 -> 150, idle 0 -> 0) is usable: 100% busy.
    {
      t: 3000,
      cpu_idle: 0,
      cpu_total: 150,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
  ];
  const marker = summarizeSamples(samples, baseOptions);
  assert.deepEqual(marker.cpu_pct, { avg: 100, p95: 100, max: 100 });
}

console.log("ci telemetry action tests passed");
