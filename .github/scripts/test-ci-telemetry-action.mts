import assert from "node:assert/strict";
import {
  parseSamples,
  summarizeSamples,
} from "../actions/ci-telemetry/metrics.mts";

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
      cpu_threads: null,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    {
      t: 1000,
      cpu_idle: 1,
      cpu_total: 10,
      cpu_threads: null,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    {
      t: 11000,
      cpu_idle: 92,
      cpu_total: 110,
      cpu_threads: null,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
  ];
  const marker = summarizeSamples(samples, baseOptions);
  assert.ok(marker.cpu_pct);
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
      cpu_threads: null,
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
      cpu_threads: null,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    });
  }
  const marker = summarizeSamples(samples, baseOptions);
  assert.ok(marker.cpu_pct);
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
        cpu_threads: null,
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
      cpu_threads: null,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: 500,
    },
    {
      t: 1000,
      cpu_idle: 50,
      cpu_total: 200,
      cpu_threads: null,
      mem_used: 200,
      mem_total: 1000,
      disk_used: 50,
      disk_total: 500,
    },
    {
      t: 2000,
      cpu_idle: 100,
      cpu_total: 300,
      cpu_threads: null,
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
    cpu_threads: null,
    mem_used: 1,
    mem_total: 10,
    disk_used: null,
    disk_total: null,
  });
  const valid2 = JSON.stringify({
    t: 1000,
    cpu_idle: 1,
    cpu_total: 20,
    cpu_threads: null,
    mem_used: 2,
    mem_total: 10,
    disk_used: null,
    disk_total: null,
  });
  const torn = '{"t":2000,"cpu_idle":2,"cpu_tot';
  const text = `${valid1}\n\n${valid2}\n${torn}`;
  const samples = parseSamples(text);
  assert.equal(samples.length, 2);
  assert.equal(samples[1]?.t, 1000);
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
      cpu_threads: null,
      mem_used: 4_000_000_000 + (t % 1_000_000_000),
      mem_total: 16_000_000_000,
      disk_used: 60_000_000_000,
      disk_total: 150_000_000_000,
    });
  }
  const marker = summarizeSamples(samples, baseOptions);
  assert.equal(marker.sample_count, samples.length);
  assert.ok(marker.timeline);
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
      cpu_threads: null,
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
      cpu_threads: null,
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
      cpu_threads: null,
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
      cpu_threads: null,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
  ];
  const marker = summarizeSamples(samples, baseOptions);
  assert.deepEqual(marker.cpu_pct, { avg: 100, p95: 100, max: 100 });
}

// Per-thread avg must be delta-weighted per thread, independent of how busy
// the other threads were: one thread pegged 100% busy (idle never advances)
// alongside three fully-idle threads (idle advances exactly as fast as
// total) must read [100, 0, 0, 0] even though the whole-VM figure — which
// sums all four threads together — comes out around 25.
{
  const samples = [
    {
      t: 0,
      cpu_idle: 0,
      cpu_total: 0,
      cpu_threads: [
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
      ] as [number, number][],
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    {
      t: 1000,
      cpu_idle: 300,
      cpu_total: 400,
      cpu_threads: [
        [0, 100],
        [100, 100],
        [100, 100],
        [100, 100],
      ] as [number, number][],
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
  ];
  const marker = summarizeSamples(samples, baseOptions);
  assert.ok(marker.cpu_pct);
  assert.equal(marker.cpu_pct.avg, 25);
  assert.ok(marker.cpu_threads);
  // All four threads had a usable pair (every delta total > 0), so none of
  // these should be the "unmeasured" null; `?? null` (not `!`) surfaces an
  // unexpected null as a value the deepEqual below can still fail on.
  assert.deepEqual(
    marker.cpu_threads.map((thread) => thread?.avg ?? null),
    [100, 0, 0, 0],
  );
}

// A thread that never advances between samples (every delta total <= 0 for
// its whole lifetime, e.g. an offline CPU) has no usable pair at all, and
// must report null — not 0 — so it cannot be mistaken for a thread that was
// measured and found idle. The other three threads, which ARE measured,
// keep their real values.
{
  const samples = [
    {
      t: 0,
      cpu_idle: 0,
      cpu_total: 0,
      cpu_threads: [
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
      ] as [number, number][],
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    {
      t: 1000,
      cpu_idle: 300,
      cpu_total: 400,
      cpu_threads: [
        // Thread 0 never advances: idle and total both stay at 0.
        [0, 0],
        [100, 100],
        [100, 100],
        [100, 100],
      ] as [number, number][],
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
  ];
  const marker = summarizeSamples(samples, baseOptions);
  assert.ok(marker.cpu_threads);
  assert.equal(
    marker.cpu_threads[0],
    null,
    "an unmeasured thread must be null, never {avg:0, max:0}",
  );
  assert.deepEqual(
    marker.cpu_threads.slice(1).map((thread) => thread?.avg ?? null),
    [0, 0, 0],
    "the other, genuinely-measured threads must keep reporting real (non-null) values",
  );
}

// Per-thread avg must be delta-weighted (total idle / total elapsed for that
// thread), not the mean of its own pair percentages — the same distinction
// the file's very first test makes for the whole-VM figure, reproduced here
// per-thread using the identical two-interval shape (1s at 90% busy, then
// 10s at 9% busy) so a naive per-thread mean (49.5) is clearly wrong and the
// correct delta-weighted figure (16.4) is clearly right. The other three
// threads never move, so the whole-VM figure equals thread 0's figure
// exactly, which also re-confirms independence from the total.
{
  const samples = [
    {
      t: 0,
      cpu_idle: 0,
      cpu_total: 0,
      cpu_threads: [
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
      ] as [number, number][],
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    {
      t: 1000,
      cpu_idle: 1,
      cpu_total: 10,
      cpu_threads: [
        [1, 10],
        // idle === total each interval: genuinely measured (deltaTotal > 0)
        // and 0% busy, distinct from a thread whose counters never move at
        // all (that case is covered by the dedicated "unmeasured" test
        // above and would be null, not 0, here).
        [10, 10],
        [10, 10],
        [10, 10],
      ] as [number, number][],
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    {
      t: 11000,
      cpu_idle: 92,
      cpu_total: 110,
      cpu_threads: [
        [92, 110],
        [110, 110],
        [110, 110],
        [110, 110],
      ] as [number, number][],
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
  ];
  const marker = summarizeSamples(samples, baseOptions);
  assert.ok(marker.cpu_pct);
  assert.equal(marker.cpu_pct.avg, 16.4);
  assert.ok(marker.cpu_threads);
  assert.equal(
    marker.cpu_threads[0]?.avg,
    16.4,
    "delta-weighted, not the naive mean of (90, 9) = 49.5",
  );
  assert.deepEqual(
    marker.cpu_threads.slice(1).map((thread) => thread?.avg ?? null),
    [0, 0, 0],
    "threads that were measured and found idle must read 0, not be pulled up by thread 0",
  );
}

// Unavailable must be null, never zeros, for three independent reasons: no
// sample carries cpu_threads (a pre-change job), the thread count is not
// constant across samples (untrustworthy), and fewer than 2 samples (no
// delta can be formed at all, even though cpu_threads is present).
{
  const noThreads = [
    {
      t: 0,
      cpu_idle: 0,
      cpu_total: 0,
      cpu_threads: null,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    {
      t: 1000,
      cpu_idle: 1,
      cpu_total: 10,
      cpu_threads: null,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
  ];
  const noThreadsMarker = summarizeSamples(noThreads, baseOptions);
  assert.equal(noThreadsMarker.cpu_threads, null);
  assert.equal(noThreadsMarker.timeline?.cpu_thread_pct_avg ?? null, null);

  const countChanges = [
    {
      t: 0,
      cpu_idle: 0,
      cpu_total: 0,
      cpu_threads: [
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
      ] as [number, number][],
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    {
      t: 1000,
      cpu_idle: 0,
      cpu_total: 100,
      // Only 3 pairs this time: the thread count changed mid-job.
      cpu_threads: [
        [0, 100],
        [0, 100],
        [0, 100],
      ] as [number, number][],
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
  ];
  const countChangesMarker = summarizeSamples(countChanges, baseOptions);
  assert.equal(countChangesMarker.cpu_threads, null);
  assert.equal(countChangesMarker.timeline?.cpu_thread_pct_avg ?? null, null);

  const single = summarizeSamples(
    [
      {
        t: 0,
        cpu_idle: 0,
        cpu_total: 0,
        cpu_threads: [
          [0, 0],
          [0, 0],
        ] as [number, number][],
        mem_used: null,
        mem_total: 1000,
        disk_used: null,
        disk_total: null,
      },
    ],
    baseOptions,
  );
  assert.equal(single.cpu_threads, null);
}

// parseSamples must not drop a whole sample just because its cpu_threads
// value is malformed: the sample's other fields (used for the whole-VM
// figures) are still usable, with only cpu_threads becoming null for that
// one sample — mirroring how a torn *line* is tolerated, just for a torn
// *field* instead.
{
  const malformed = JSON.stringify({
    t: 0,
    cpu_idle: 5,
    cpu_total: 10,
    cpu_threads: ["not", "pairs"],
    mem_used: 1,
    mem_total: 100,
    disk_used: null,
    disk_total: null,
  });
  const samples = parseSamples(malformed);
  assert.equal(samples.length, 1);
  assert.equal(samples[0]?.cpu_idle, 5);
  assert.equal(samples[0]?.cpu_total, 10);
  assert.equal(samples[0]?.mem_used, 1);
  assert.equal(samples[0]?.cpu_threads, null);
}

// Size budget with a per-thread timeline: 8 threads (<= the timeline's own
// cap) over a 3h job at 5s must still fit comfortably under the ~4KB budget
// the aggregator/PR-comment path expects, timeline included.
{
  const threadCount = 8;
  const idleAcc = new Array(threadCount).fill(0);
  const totalAcc = new Array(threadCount).fill(0);
  const samples = [];
  for (let t = 0; t <= 3 * 60 * 60 * 1000; t += 5000) {
    const cpuThreads: [number, number][] = [];
    let idleSum = 0;
    let totalSum = 0;
    for (let i = 0; i < threadCount; i++) {
      idleAcc[i] += 3;
      totalAcc[i] += 5; // constant 40% busy per thread
      cpuThreads.push([idleAcc[i], totalAcc[i]]);
      idleSum += idleAcc[i];
      totalSum += totalAcc[i];
    }
    samples.push({
      t,
      cpu_idle: idleSum,
      cpu_total: totalSum,
      cpu_threads: cpuThreads,
      mem_used: 4_000_000_000 + (t % 1_000_000_000),
      mem_total: 16_000_000_000,
      disk_used: 60_000_000_000,
      disk_total: 150_000_000_000,
    });
  }
  const marker = summarizeSamples(samples, {
    ...baseOptions,
    cpuCount: threadCount,
  });
  assert.ok(marker.cpu_threads);
  assert.equal(marker.cpu_threads.length, 8);
  assert.ok(
    marker.timeline?.cpu_thread_pct_avg,
    "8 threads is within the timeline's own cap",
  );
  assert.equal(marker.timeline.cpu_thread_pct_avg.length, 8);
  const size = JSON.stringify(marker).length;
  assert.ok(
    size < 4096,
    `8-thread marker with a per-thread timeline must stay under 4096 bytes, got ${size}`,
  );
}

// Size budget without a per-thread timeline: 64 threads (the cpu_threads
// cap) over the same 3h/5s job must still fit under 4096 bytes once the
// timeline is dropped for exceeding its own, smaller 8-thread cap.
{
  const threadCount = 64;
  const idleAcc = new Array(threadCount).fill(0);
  const totalAcc = new Array(threadCount).fill(0);
  const samples = [];
  for (let t = 0; t <= 3 * 60 * 60 * 1000; t += 5000) {
    const cpuThreads: [number, number][] = [];
    let idleSum = 0;
    let totalSum = 0;
    for (let i = 0; i < threadCount; i++) {
      idleAcc[i] += 3;
      totalAcc[i] += 5;
      cpuThreads.push([idleAcc[i], totalAcc[i]]);
      idleSum += idleAcc[i];
      totalSum += totalAcc[i];
    }
    samples.push({
      t,
      cpu_idle: idleSum,
      cpu_total: totalSum,
      cpu_threads: cpuThreads,
      mem_used: 4_000_000_000 + (t % 1_000_000_000),
      mem_total: 16_000_000_000,
      disk_used: 60_000_000_000,
      disk_total: 150_000_000_000,
    });
  }
  const marker = summarizeSamples(samples, {
    ...baseOptions,
    cpuCount: threadCount,
  });
  assert.ok(marker.cpu_threads);
  assert.equal(marker.cpu_threads.length, 64);
  assert.equal(
    marker.timeline?.cpu_thread_pct_avg ?? null,
    null,
    "64 threads exceeds the timeline's own cap even though cpu_threads itself is populated",
  );
  const size = JSON.stringify(marker).length;
  assert.ok(
    size < 4096,
    `64-thread marker without a per-thread timeline must stay under 4096 bytes, got ${size}`,
  );
}

// 65 threads exceeds the marker's own size budget (not just the timeline's
// smaller cap), so cpu_threads itself must be null, not truncated to 64.
{
  const cpuThreadsA: [number, number][] = Array.from({ length: 65 }, (_, i) => [
    i,
    i + 100,
  ]);
  const cpuThreadsB: [number, number][] = Array.from({ length: 65 }, (_, i) => [
    i,
    i + 200,
  ]);
  const samples = [
    {
      t: 0,
      cpu_idle: 0,
      cpu_total: 0,
      cpu_threads: cpuThreadsA,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
    {
      t: 1000,
      cpu_idle: 100,
      cpu_total: 200,
      cpu_threads: cpuThreadsB,
      mem_used: null,
      mem_total: 1000,
      disk_used: null,
      disk_total: null,
    },
  ];
  const marker = summarizeSamples(samples, { ...baseOptions, cpuCount: 65 });
  assert.equal(marker.cpu_threads, null);
}

console.log("ci telemetry action tests passed");
