import assert from "node:assert/strict";
import { keepLatestRunPerWorkflow, resolveRunId } from "./ci-telemetry/cli.mts";
import { extractMarkers } from "./ci-telemetry/markers.mts";
import { buildRunRecord } from "./ci-telemetry/record.mts";
import {
  formatDuration,
  formatGiB,
  renderComment,
} from "./ci-telemetry/render.mts";
import type {
  JobRecord,
  JobResourcesMarker,
  PendingRunEntry,
  RunRecord,
} from "./ci-telemetry/schema.mts";

function fixtureRecord({
  workflowName,
  jobName,
  stepName,
  resources = null,
}: {
  workflowName: string;
  jobName: string;
  stepName: string;
  resources?: JobResourcesMarker | null;
}): RunRecord {
  return {
    schema: 1,
    repository: "shm11C3/HardwareVisualizer",
    workflow: { id: 1, name: workflowName, path: ".github/workflows/ci.yml" },
    run: {
      id: 1,
      attempt: 1,
      number: 1,
      event: "pull_request",
      status: "completed",
      conclusion: "success",
      head_branch: "feat/x",
      head_sha: "abc123",
      head_repository: "shm11C3/HardwareVisualizer",
      is_fork: false,
      actor: "shm11C3",
      created_at: "2026-09-21T07:00:00Z",
      started_at: "2026-09-21T07:00:00Z",
      completed_at: "2026-09-21T07:02:00Z",
      duration_seconds: 120,
      url: "https://github.com/shm11C3/HardwareVisualizer/actions/runs/1",
    },
    jobs: [
      {
        id: 1,
        name: jobName,
        status: "completed",
        conclusion: "success",
        labels: [],
        created_at: "2026-09-21T07:00:00Z",
        started_at: "2026-09-21T07:00:05Z",
        completed_at: "2026-09-21T07:02:00Z",
        queue_seconds: 5,
        duration_seconds: 115,
        steps: [
          {
            number: 1,
            name: stepName,
            conclusion: "success",
            duration_seconds: 90,
          },
        ],
        resources,
        caches: { rust_target: null, node: null, sccache: null },
      },
    ],
  };
}

// --- markers.mts ------------------------------------------------------

// The runner echoes a step's script source (with ANSI color codes) before
// running it, which can contain the marker name followed by text that is
// not valid JSON (e.g. `console.log(\`CACHE_METRICS_JSON=${...}\`)` as
// literal source). That echoed occurrence must be skipped in favor of the
// real, later marker line the script actually printed.
function testEchoedScriptFollowedByRealMarker(): void {
  const log = [
    "2026-09-21T07:36:29.0000000Z \x1b[36;1mRun node report.cjs\x1b[0m",
    // biome-ignore lint/suspicious/noTemplateCurlyInString: simulates the runner echoing this script's own source line, `${...}` included, ahead of its real output
    "2026-09-21T07:36:29.0010000Z console.log(`CACHE_METRICS_JSON=${JSON.stringify(metrics)}`);",
    '2026-09-21T07:36:30.5473117Z CACHE_METRICS_JSON={"compile_requests":10,"compile_requests_executed":10,"rust_hits":8,"rust_misses":2,"rust_hit_rate_pct":80,"cpp_hits":0,"cpp_misses":0,"cpp_hit_rate_pct":null,"cache_errors":0,"cache_location":"s3, bucket","target_cache_exact_hit":"true"}',
  ].join("\n");
  const markers = extractMarkers(log);
  assert.ok(
    markers.sccache,
    "real marker line after an echoed source line must still be found",
  );
  assert.equal(markers.sccache.rust_hits, 8);
  assert.equal(markers.sccache.backend, "s3");
  assert.equal(markers.sccache.target_cache_exact_hit, true);
}

// A log where the ONLY occurrence of the marker name is unparseable (e.g.
// the job crashed before ever printing the real marker) must yield null,
// not throw and not accidentally pick up the source echo as data.
function testOnlyUnparseableOccurrenceYieldsNull(): void {
  const log = [
    "2026-09-21T07:36:29.0000000Z \x1b[36;1mRun node report.cjs\x1b[0m",
    // biome-ignore lint/suspicious/noTemplateCurlyInString: simulates the runner echoing this script's own source line, `${...}` included, ahead of its real output
    "2026-09-21T07:36:29.0010000Z console.log(`CACHE_METRICS_JSON=${JSON.stringify(metrics)}`);",
    "2026-09-21T07:36:30.0000000Z Error: sccache binary not found",
  ].join("\n");
  assert.equal(extractMarkers(log).sccache, null);
}

// A forged CI_JOB_METRICS_JSON marker (as a PR-controlled job could print)
// must not be able to inject free-form strings or out-of-schema data: every
// surviving field is a number, boolean, or a value from a closed enum.
function testForgedResourcesMarkerCannotInjectContent(): void {
  const forged = {
    schema: 1,
    runner_os: "<script>alert(1)</script>",
    runner_arch: "totally-fake",
    cpu_count: 4,
    interval_seconds: 5,
    sample_count: 10,
    duration_seconds: 50,
    mem_total_bytes: 1000,
    // Forged extra key that must never survive validation.
    injected_field: "'; DROP TABLE runs; --",
    cpu_pct: { avg: 999, p95: -50, max: 42 },
  };
  const log = `CI_JOB_METRICS_JSON=${JSON.stringify(forged)}`;
  const resources = extractMarkers(log).resources;
  assert.ok(resources);
  assert.equal(
    resources.runner_os,
    "unknown",
    "out-of-enum runner_os falls back to unknown",
  );
  assert.equal(
    resources.runner_arch,
    "unknown",
    "out-of-enum runner_arch falls back to unknown",
  );
  assert.equal(
    Object.hasOwn(resources, "injected_field"),
    false,
    "unknown keys are dropped",
  );
  assert.ok(resources.cpu_pct);
  assert.equal(
    resources.cpu_pct.avg,
    100,
    "over-100 percentage is clamped, not passed through",
  );
  assert.equal(
    resources.cpu_pct.p95,
    0,
    "negative percentage is clamped, not passed through",
  );
}

// Wrong schema version must reject the whole marker rather than coerce it —
// a schema-2 producer's field meanings are not guaranteed to match schema 1.
function testWrongSchemaVersionRejected(): void {
  const log = `CI_JOB_METRICS_JSON=${JSON.stringify({
    schema: 2,
    cpu_count: 1,
    interval_seconds: 1,
    sample_count: 1,
    duration_seconds: 1,
    mem_total_bytes: 1,
  })}`;
  assert.equal(extractMarkers(log).resources, null);
}

// A required numeric field that is negative or non-finite makes the whole
// marker untrustworthy (there is no sane fallback for "how many CPUs").
function testOutOfRangeRequiredNumberRejectsMarker(): void {
  const log = `CI_JOB_METRICS_JSON=${JSON.stringify({
    schema: 1,
    cpu_count: -1,
    interval_seconds: 1,
    sample_count: 1,
    duration_seconds: 1,
    mem_total_bytes: 1,
  })}`;
  assert.equal(extractMarkers(log).resources, null);
}

// A sampler that recorded nothing reports mem_total_bytes: null. That marker
// must be kept (its sample_count: 0 is the evidence the sampler failed), not
// discarded as if the required field were forged.
function testNullMemTotalIsAcceptedAsUnavailable(): void {
  const log = `CI_JOB_METRICS_JSON=${JSON.stringify({
    schema: 1,
    cpu_count: 4,
    interval_seconds: 5,
    sample_count: 0,
    duration_seconds: 0,
    cpu_pct: null,
    mem_total_bytes: null,
    mem_used_bytes: null,
    disk_total_bytes: null,
    disk_used_bytes: null,
    timeline: null,
  })}`;
  const resources = extractMarkers(log).resources;
  assert.ok(resources);
  assert.equal(resources.mem_total_bytes, null);
  assert.equal(resources.sample_count, 0);
}

// An oversized timeline array (a PR-controlled job could print thousands of
// samples to bloat the comment / records.json) must null out the whole
// timeline rather than being truncated and rendered anyway.
function testOversizedTimelineArrayNullsTimeline(): void {
  const oversized = {
    schema: 1,
    cpu_count: 4,
    interval_seconds: 5,
    sample_count: 10,
    duration_seconds: 50,
    mem_total_bytes: 1000,
    timeline: {
      bucket_seconds: 15,
      cpu_pct_avg: new Array(121).fill(10),
      cpu_pct_max: [10],
      mem_used_pct_max: [10],
    },
  };
  const log = `CI_JOB_METRICS_JSON=${JSON.stringify(oversized)}`;
  const resources = extractMarkers(log).resources;
  assert.ok(resources);
  assert.equal(resources.timeline, null);
}

// A valid, moderate timeline (with a null sample, representing a missed
// interval) is preserved as-is.
function testValidTimelinePreserved(): void {
  const valid = {
    schema: 1,
    cpu_count: 4,
    interval_seconds: 5,
    sample_count: 2,
    duration_seconds: 10,
    mem_total_bytes: 1000,
    timeline: {
      bucket_seconds: 15,
      cpu_pct_avg: [12, null],
      cpu_pct_max: [30, 100],
      mem_used_pct_max: [21, 33],
    },
  };
  const log = `CI_JOB_METRICS_JSON=${JSON.stringify(valid)}`;
  const resources = extractMarkers(log).resources;
  assert.ok(resources);
  assert.deepEqual(resources.timeline, valid.timeline);
}

// RUST_CACHE_METRICS_JSON: shared_key falls back to null on an invalid
// format, but a real type mismatch on the booleans rejects the whole
// marker (there is no safe guess for "did this restore hit exactly").
function testRustCacheKeyFallbackAndBooleanRejection(): void {
  const badKey = `RUST_CACHE_METRICS_JSON=${JSON.stringify({
    shared_key: "Not Valid!",
    exact_hit: false,
    restore_seconds: 10,
    targets: true,
  })}`;
  const parsed = extractMarkers(badKey).rust_cache;
  assert.ok(parsed);
  assert.equal(parsed.shared_key, null);
  assert.equal(parsed.exact_hit, false);

  const badBool = `RUST_CACHE_METRICS_JSON=${JSON.stringify({
    shared_key: "core-test",
    exact_hit: "false",
    restore_seconds: 10,
    targets: true,
  })}`;
  assert.equal(extractMarkers(badBool).rust_cache, null);
}

// A line containing RUST_CACHE_METRICS_JSON=... must not be mistaken for a
// CACHE_METRICS_JSON=... occurrence just because the marker name string
// contains that substring — the token-boundary check in markers.mts exists
// specifically to prevent this cross-marker leakage.
function testMarkerNameSubstringDoesNotLeakAcrossMarkers(): void {
  const log = `RUST_CACHE_METRICS_JSON=${JSON.stringify({
    shared_key: "core-test",
    exact_hit: true,
    restore_seconds: 5,
    targets: true,
  })}`;
  assert.equal(extractMarkers(log).sccache, null);
  assert.ok(extractMarkers(log).rust_cache);
}

// NODE_CACHE_METRICS_JSON: minimal shape, type-checked boolean.
function testNodeCacheMarkerShape(): void {
  assert.deepEqual(
    extractMarkers('NODE_CACHE_METRICS_JSON={"exact_hit":true}').node_cache,
    { exact_hit: true },
  );
  assert.equal(
    extractMarkers('NODE_CACHE_METRICS_JSON={"exact_hit":"true"}').node_cache,
    null,
  );
}

// The LAST valid occurrence wins when a job legitimately re-prints a marker
// (e.g. a retried step).
function testLastValidOccurrenceWins(): void {
  const log = [
    'NODE_CACHE_METRICS_JSON={"exact_hit":false}',
    'NODE_CACHE_METRICS_JSON={"exact_hit":true}',
  ].join("\n");
  const nodeCache = extractMarkers(log).node_cache;
  assert.ok(nodeCache);
  assert.equal(nodeCache.exact_hit, true);
}

// --- record.mts ---------------------------------------------------------

// buildRunRecord output for a small, hand-computed fixture must match the
// documented schema exactly, including queue/duration math.
function testBuildRunRecordMatchesDocumentedSchema(): void {
  const record = buildRunRecord({
    repository: "shm11C3/HardwareVisualizer",
    run: {
      id: 1,
      name: "CI",
      path: ".github/workflows/ci.yml",
      workflow_id: 123,
      run_attempt: 1,
      run_number: 10,
      event: "pull_request",
      status: "completed",
      conclusion: "success",
      head_branch: "feat/x",
      head_sha: "abc123",
      head_repository: { full_name: "shm11C3/HardwareVisualizer" },
      actor: { login: "shm11C3" },
      created_at: "2026-09-21T07:00:00Z",
      run_started_at: "2026-09-21T07:01:00Z",
      updated_at: "2026-09-21T07:30:05Z",
      html_url: "https://github.com/shm11C3/HardwareVisualizer/actions/runs/1",
    },
    jobs: [
      {
        id: 11,
        name: "test-core (windows-latest)",
        status: "completed",
        conclusion: "success",
        labels: ["windows-latest"],
        created_at: "2026-09-21T07:00:10Z",
        started_at: "2026-09-21T07:01:20Z",
        completed_at: "2026-09-21T07:15:26Z",
        steps: [
          {
            number: 1,
            name: "Set up job",
            status: "completed",
            conclusion: "success",
            started_at: "2026-09-21T07:01:20Z",
            completed_at: "2026-09-21T07:01:23Z",
          },
        ],
      },
    ],
    markersByJobId: {},
  });

  assert.deepEqual(record, {
    schema: 1,
    repository: "shm11C3/HardwareVisualizer",
    workflow: { id: 123, name: "CI", path: ".github/workflows/ci.yml" },
    run: {
      id: 1,
      attempt: 1,
      number: 10,
      event: "pull_request",
      status: "completed",
      conclusion: "success",
      head_branch: "feat/x",
      head_sha: "abc123",
      head_repository: "shm11C3/HardwareVisualizer",
      is_fork: false,
      actor: "shm11C3",
      created_at: "2026-09-21T07:00:00Z",
      started_at: "2026-09-21T07:01:00Z",
      completed_at: "2026-09-21T07:15:26Z",
      duration_seconds: 866,
      url: "https://github.com/shm11C3/HardwareVisualizer/actions/runs/1",
    },
    jobs: [
      {
        id: 11,
        name: "test-core (windows-latest)",
        status: "completed",
        conclusion: "success",
        labels: ["windows-latest"],
        created_at: "2026-09-21T07:00:10Z",
        started_at: "2026-09-21T07:01:20Z",
        completed_at: "2026-09-21T07:15:26Z",
        queue_seconds: 70,
        duration_seconds: 846,
        steps: [
          {
            number: 1,
            name: "Set up job",
            conclusion: "success",
            duration_seconds: 3,
          },
        ],
        resources: null,
        caches: { rust_target: null, node: null, sccache: null },
      },
    ],
  });
}

// A fork run is flagged via head_repository != repository, and when no job
// ever completed, completed_at falls back to the run's own updated_at.
function testForkFlagAndCompletedAtFallback(): void {
  const record = buildRunRecord({
    repository: "shm11C3/HardwareVisualizer",
    run: {
      id: 2,
      name: "CI",
      path: ".github/workflows/ci.yml",
      workflow_id: 123,
      run_attempt: 1,
      run_number: 11,
      event: "pull_request",
      status: "completed",
      conclusion: "success",
      head_branch: "feat/x",
      head_sha: "def456",
      head_repository: { full_name: "someone-else/HardwareVisualizer" },
      actor: { login: "someone-else" },
      created_at: "2026-09-21T07:00:00Z",
      run_started_at: "2026-09-21T07:01:00Z",
      updated_at: "2026-09-21T07:30:05Z",
      html_url: "https://github.com/shm11C3/HardwareVisualizer/actions/runs/2",
    },
    jobs: [],
    markersByJobId: {},
  });
  assert.equal(record.run.is_fork, true);
  assert.equal(record.run.completed_at, "2026-09-21T07:30:05Z");
}

// Queue time is null (not a number) for a skipped job and for a job that
// was created but never started — both cases mean "never queued for real
// work", so a numeric queue time would be misleading, not just wrong.
function testQueueSecondsNullForSkippedAndNeverStartedJobs(): void {
  const record = buildRunRecord({
    repository: "shm11C3/HardwareVisualizer",
    run: {
      id: 3,
      name: "CI",
      path: ".github/workflows/ci.yml",
      workflow_id: 123,
      run_attempt: 1,
      run_number: 12,
      event: "push",
      status: "completed",
      conclusion: "success",
      head_branch: "develop",
      head_sha: "aaa111",
      head_repository: { full_name: "shm11C3/HardwareVisualizer" },
      actor: { login: "shm11C3" },
      created_at: "2026-09-21T07:00:00Z",
      run_started_at: "2026-09-21T07:01:00Z",
      updated_at: "2026-09-21T07:30:05Z",
      html_url: "https://github.com/shm11C3/HardwareVisualizer/actions/runs/3",
    },
    jobs: [
      {
        id: 21,
        name: "skipped-job",
        status: "completed",
        conclusion: "skipped",
        labels: [],
        created_at: "2026-09-21T07:00:10Z",
        started_at: "2026-09-21T07:00:10Z",
        completed_at: "2026-09-21T07:00:11Z",
        steps: [],
      },
      {
        id: 22,
        name: "never-started-job",
        status: "completed",
        conclusion: "cancelled",
        labels: [],
        created_at: "2026-09-21T07:00:10Z",
        started_at: null,
        completed_at: "2026-09-21T07:00:11Z",
        steps: [],
      },
    ],
    markersByJobId: {},
  });
  assert.equal(record.jobs[0]?.queue_seconds, null);
  assert.equal(record.jobs[1]?.queue_seconds, null);
}

// The GitHub API represents an unset step `started_at` (observed live for a
// step skipped by an `if:` condition) as the sentinel "0001-01-01T00:00:00Z"
// rather than null. Without special-casing it, a step's duration computes as
// millions of hours instead of being recognized as missing data — this was
// caught against real production data during --dry-run verification.
function testSentinelEpochTimestampTreatedAsMissing(): void {
  const record = buildRunRecord({
    repository: "shm11C3/HardwareVisualizer",
    run: {
      id: 4,
      name: "CI",
      path: ".github/workflows/ci.yml",
      workflow_id: 123,
      run_attempt: 1,
      run_number: 13,
      event: "push",
      status: "completed",
      conclusion: "success",
      head_branch: "develop",
      head_sha: "bbb222",
      head_repository: { full_name: "shm11C3/HardwareVisualizer" },
      actor: { login: "shm11C3" },
      created_at: "2026-09-21T07:00:00Z",
      run_started_at: "2026-09-21T07:01:00Z",
      updated_at: "2026-09-21T07:30:05Z",
      html_url: "https://github.com/shm11C3/HardwareVisualizer/actions/runs/4",
    },
    jobs: [
      {
        id: 31,
        name: "test-build (ubuntu-22.04)",
        status: "completed",
        conclusion: "success",
        labels: ["ubuntu-22.04"],
        created_at: "2026-09-21T07:00:10Z",
        started_at: "2026-09-21T07:00:20Z",
        completed_at: "2026-09-21T07:10:20Z",
        steps: [
          {
            number: 13,
            name: "Setup Node.js",
            status: "completed",
            conclusion: "skipped",
            started_at: "0001-01-01T00:00:00Z",
            completed_at: "2026-09-21T07:05:00Z",
          },
        ],
      },
    ],
    markersByJobId: {},
  });
  assert.equal(record.jobs[0]?.steps[0]?.duration_seconds, null);
}

// --- render.mts: formatting helpers --------------------------------------

function testDurationAndByteFormatting(): void {
  assert.equal(formatDuration(42), "42s");
  assert.equal(formatDuration(846), "14m 06s");
  assert.equal(formatDuration(3722), "1h 02m");
  assert.equal(
    formatDuration(null),
    "–",
    "missing duration renders a dash, never 0",
  );
  assert.equal(
    formatDuration(0),
    "0s",
    "a real zero duration is not confused with missing data",
  );
  assert.equal(formatGiB(null, 1), "–");
  assert.equal(formatGiB(5.5 * 1024 ** 3, 1), "5.5");
}

// --- render.mts: hostile names cannot break the comment structure -------

function testHostileNamesCannotBreakCommentStructure(): void {
  // Kept short (well under the 80-char cap once sanitized) so the tail of
  // the payload — the fake link target — is not itself truncated away,
  // which would make the "stays inert" assertion below meaningless.
  const hostile = "a|b`c\nd <img onerror=1> @org/team [x](http://evil.example)";
  const record = fixtureRecord({
    workflowName: hostile,
    jobName: hostile,
    stepName: hostile,
  });
  const body = renderComment({
    headSha: "abc1234567",
    records: [record],
    pendingRuns: [],
    now: new Date("2026-09-21T07:40:00Z"),
  });

  // Table integrity: every table row (lines starting with "|") must keep the
  // number of pipe-delimited cells the header defines, and no raw newline
  // from a hostile name may have split a row across lines.
  const lines = body.split("\n");
  const summaryHeaderIdx = lines.indexOf(
    "| Workflow | Result | Wall time | Longest queue | Jobs |",
  );
  assert.ok(
    summaryHeaderIdx >= 0,
    "summary table header must be present and intact",
  );
  const summaryRow = lines[summaryHeaderIdx + 2];
  assert.ok(summaryRow);
  assert.equal(
    (summaryRow.match(/\|/g) || []).length,
    6,
    "hostile pipes must not add extra table columns",
  );

  // Inside a code span, literal "<img ...>" / "[x](url)" text is inert —
  // GitHub does not interpret HTML or links inside single-backtick spans —
  // so it is fine for it to appear there verbatim. What must never happen
  // is that text escaping a code span into live HTML or a live table cell.
  assert.equal(
    body.includes("](http://evil.example)"),
    true,
    "the text form is fine, it must just stay inert inside a code span",
  );

  // The <summary> line uses HTML-escaping (a code span offers no protection
  // inside a raw HTML block), so verify the raw tag never appears there
  // unescaped and that the escaped form does.
  const summaryLine = lines.find((line) => line.startsWith("<summary>"));
  assert.ok(
    summaryLine,
    "a <summary> block must be present for this long-running record",
  );
  assert.equal(
    summaryLine.includes("<img"),
    false,
    "the <summary> HTML must never contain a raw hostile tag",
  );
  assert.ok(
    summaryLine.includes("&lt;img"),
    "the <summary> block must HTML-escape the hostile name",
  );

  // Entity-escaping alone is not enough in <summary>: GitHub still turns a
  // bare "@org/team" there into a live mention that notifies people (seen
  // with GitHub's Markdown API). Its mention/autolink pass skips <code>, so
  // the hostile name must sit entirely inside one.
  const mentionIdx = summaryLine.indexOf("@org/team");
  assert.ok(mentionIdx > summaryLine.indexOf("<code>"));
  assert.ok(mentionIdx < summaryLine.indexOf("</code>"));
  assert.equal(
    (summaryLine.match(/<\/code>/g) || []).length,
    1,
    "the hostile name must not be able to close <code> early",
  );

  // No backtick from the hostile name closed a code span early: it must
  // have been stripped, not passed through.
  assert.equal(
    body.includes("`name"),
    false,
    "backticks in hostile input are stripped, not passed through",
  );
}

// --- render.mts: missing data, skipped jobs, pending runs ----------------

function testMissingDataSkippedJobsAndPendingRuns(): void {
  const record = fixtureRecord({
    workflowName: "CI",
    jobName: "test-core (windows-latest)",
    stepName: "Run Core tests",
  });
  // Add a skipped job that must be excluded from the Jobs count and from
  // the per-job table entirely.
  record.jobs.push({
    id: 2,
    name: "skipped-job",
    status: "completed",
    conclusion: "skipped",
    labels: [],
    created_at: "2026-09-21T07:00:00Z",
    started_at: null,
    completed_at: null,
    queue_seconds: null,
    duration_seconds: null,
    steps: [],
    resources: null,
    caches: { rust_target: null, node: null, sccache: null },
  });

  const pendingRuns: PendingRunEntry[] = [
    {
      workflow: {
        id: 2,
        name: "CodeQL Advanced",
        path: ".github/workflows/codeql.yml",
      },
      run: {
        id: 2,
        attempt: 1,
        number: 2,
        status: "in_progress",
        conclusion: null,
        url: "https://github.com/shm11C3/HardwareVisualizer/actions/runs/2",
      },
    },
  ];

  const body = renderComment({
    headSha: "abc1234567",
    records: [record],
    pendingRuns,
    now: new Date("2026-09-21T07:40:00Z"),
  });

  const lines = body.split("\n");
  const headerIdx = lines.indexOf(
    "| Workflow | Result | Wall time | Longest queue | Jobs |",
  );
  // Completed row (CI) must sort before the pending row (CodeQL Advanced).
  const completedRow = lines[headerIdx + 2];
  const pendingRow = lines[headerIdx + 3];
  assert.ok(completedRow);
  assert.ok(pendingRow);
  assert.match(completedRow, /`CI`/);
  assert.equal(
    completedRow.includes("| 1 |"),
    true,
    "Jobs column excludes the skipped job",
  );
  assert.match(pendingRow, /CodeQL Advanced/);
  assert.match(pendingRow, /⏳ in progress/);
  assert.ok(
    pendingRow.endsWith("– | – | – |"),
    "pending row has dashes, not 0, for unknown metrics",
  );

  // Missing resources/caches render as dashes in the details table, not 0.
  assert.match(
    body,
    /\| `test-core \(windows-latest\)` \| ✅ \| 5s \| 1m 55s \| – \| – \| – \| – \|/,
  );

  // The skipped job never appears as a row in the details table.
  assert.equal(body.includes("skipped-job"), false);
}

// --- render.mts: 60000-character cap -------------------------------------

// The rendered comment must respect GitHub's hard 65536-char limit even for
// a run with hundreds of long-named jobs across several workflows; the
// "Slowest steps" tables must be dropped first, before job rows are cut.
function testLengthCapHoldsForHundredsOfLongNamedJobs(): void {
  const manyJobRecords: RunRecord[] = [];
  for (let w = 0; w < 15; w += 1) {
    const jobs: JobRecord[] = [];
    for (let j = 0; j < 200; j += 1) {
      jobs.push({
        id: w * 1000 + j,
        name: `a-very-long-descriptive-job-name-that-eats-space (matrix-target-${j}, extra-long-suffix-here)`,
        status: "completed",
        conclusion: "success",
        labels: ["ubuntu-latest"],
        created_at: "2026-09-21T07:00:00Z",
        started_at: "2026-09-21T07:00:05Z",
        completed_at: "2026-09-21T07:05:00Z",
        queue_seconds: 5,
        duration_seconds: 295,
        steps: [
          {
            number: 1,
            name: "Run a fairly long step name that also eats space",
            conclusion: "success",
            duration_seconds: 200,
          },
        ],
        resources: null,
        caches: { rust_target: null, node: null, sccache: null },
      });
    }
    manyJobRecords.push({
      schema: 1,
      repository: "shm11C3/HardwareVisualizer",
      workflow: {
        id: w,
        name: `Workflow number ${w} with a moderately long name`,
        path: `.github/workflows/w${w}.yml`,
      },
      run: {
        id: w,
        attempt: 1,
        number: w,
        event: "pull_request",
        status: "completed",
        conclusion: "success",
        head_branch: "feat/x",
        head_sha: "abc123",
        head_repository: "shm11C3/HardwareVisualizer",
        is_fork: false,
        actor: "shm11C3",
        created_at: "2026-09-21T07:00:00Z",
        started_at: "2026-09-21T07:00:00Z",
        completed_at: "2026-09-21T08:00:00Z",
        duration_seconds: 3600,
        url: "https://github.com/shm11C3/HardwareVisualizer/actions/runs/1",
      },
      jobs,
    });
  }

  const body = renderComment({
    headSha: "abc1234567",
    records: manyJobRecords,
    pendingRuns: [],
    now: new Date("2026-09-21T07:40:00Z"),
  });

  assert.ok(
    body.length <= 65536,
    `comment body must respect GitHub's hard limit, got ${body.length}`,
  );
  assert.equal(
    body.includes("Slowest steps"),
    false,
    "slowest-steps tables are dropped first once over the cap",
  );
}

// --- cli.mts: pure orchestration helpers ---------------------------

// RUN_ID must be a plain 1-20 digit numeric string: it is interpolated into
// GitHub API URL paths, so anything else must be rejected up front rather
// than trusted as a path segment.
function testResolveRunIdValidatesFormat(): void {
  assert.equal(resolveRunId("12345"), "12345");
  assert.throws(() => resolveRunId("12345; rm -rf /"));
  assert.throws(() => resolveRunId(""));
  assert.throws(() => resolveRunId(undefined));
}

// When multiple workflow runs share a head_sha (e.g. a re-run), only the
// most recently created run per workflow_id is kept — otherwise the comment
// would show a stale, superseded run alongside the current one.
function testKeepLatestRunPerWorkflow(): void {
  const runs = [
    { workflow_id: 1, created_at: "2026-09-21T07:00:00Z", id: "old" },
    { workflow_id: 1, created_at: "2026-09-21T07:05:00Z", id: "new" },
    { workflow_id: 2, created_at: "2026-09-21T07:00:00Z", id: "only" },
  ];
  const kept = keepLatestRunPerWorkflow(runs);
  assert.equal(kept.length, 2);
  assert.ok(kept.find((r) => r.id === "new"));
  assert.ok(!kept.find((r) => r.id === "old"));
  assert.ok(kept.find((r) => r.id === "only"));
}

testEchoedScriptFollowedByRealMarker();
testOnlyUnparseableOccurrenceYieldsNull();
testForgedResourcesMarkerCannotInjectContent();
testWrongSchemaVersionRejected();
testOutOfRangeRequiredNumberRejectsMarker();
testNullMemTotalIsAcceptedAsUnavailable();
testOversizedTimelineArrayNullsTimeline();
testValidTimelinePreserved();
testRustCacheKeyFallbackAndBooleanRejection();
testMarkerNameSubstringDoesNotLeakAcrossMarkers();
testNodeCacheMarkerShape();
testLastValidOccurrenceWins();
testBuildRunRecordMatchesDocumentedSchema();
testForkFlagAndCompletedAtFallback();
testQueueSecondsNullForSkippedAndNeverStartedJobs();
testSentinelEpochTimestampTreatedAsMissing();
testDurationAndByteFormatting();
testHostileNamesCannotBreakCommentStructure();
testMissingDataSkippedJobsAndPendingRuns();
testLengthCapHoldsForHundredsOfLongNamedJobs();
testResolveRunIdValidatesFormat();
testKeepLatestRunPerWorkflow();

console.log("ci-telemetry aggregate tests passed");
