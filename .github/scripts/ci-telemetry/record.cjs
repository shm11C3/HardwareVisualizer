// Builds the versioned "run record" that is written to records.json and
// (in a follow-up change) uploaded to Cloudflare R2. This module is pure: it
// only shapes data already fetched by aggregate.cjs, so it can be unit
// tested with plain fixtures instead of live API responses. Job/step/
// workflow names are stored here exactly as the API returned them — they
// are untrusted PR-controlled strings, but escaping is a render-time
// concern (see render.cjs), not a storage-time one.
"use strict";

// The GitHub API represents an unset step timestamp (typically `started_at`
// on a step that was skipped by an `if:` condition) as the sentinel
// "0001-01-01T00:00:00Z" instead of `null`. Treat any timestamp before this
// app existed as that same "unset" sentinel, otherwise a duration computed
// against it comes out as millions of hours instead of null. Confirmed
// against live data: GET .../actions/runs/{id}/attempts/{n}/jobs for a real
// skipped step returns `"started_at": "0001-01-01T00:00:00Z"` alongside a
// real `completed_at`.
const SENTINEL_EPOCH_MS = Date.UTC(2000, 0, 1);

function toEpochMs(value) {
  if (!value) return null;
  const ms = Date.parse(value);
  if (Number.isNaN(ms) || ms < SENTINEL_EPOCH_MS) return null;
  return ms;
}

function durationSeconds(startIso, endIso) {
  const start = toEpochMs(startIso);
  const end = toEpochMs(endIso);
  if (start === null || end === null) return null;
  return Math.round((end - start) / 1000);
}

// Finds the latest job completion timestamp. Falls back to the run's own
// `updated_at` (handled by the caller) only when no job has a completed_at
// at all — e.g. an empty job list, which should not happen for a completed
// run but is not assumed away here.
function latestJobCompletion(jobs) {
  let latest = null;
  for (const job of jobs) {
    if (!job.completed_at) continue;
    if (latest === null || Date.parse(job.completed_at) > Date.parse(latest)) {
      latest = job.completed_at;
    }
  }
  return latest;
}

function buildStepRecord(step) {
  return {
    number: step.number,
    name: step.name,
    conclusion: step.conclusion,
    duration_seconds: durationSeconds(step.started_at, step.completed_at),
  };
}

function buildJobRecord(job, markers) {
  // A skipped (or never-started) job has no meaningful queue time: it was
  // never dispatched to a runner, so "started_at - created_at" would
  // measure how long the job sat un-run, not queue latency.
  const queueSeconds =
    job.conclusion === "skipped" || !job.started_at || !job.created_at
      ? null
      : durationSeconds(job.created_at, job.started_at);

  return {
    id: job.id,
    name: job.name,
    status: job.status,
    conclusion: job.conclusion,
    labels: job.labels || [],
    created_at: job.created_at,
    started_at: job.started_at,
    completed_at: job.completed_at,
    queue_seconds: queueSeconds,
    duration_seconds: durationSeconds(job.started_at, job.completed_at),
    steps: (job.steps || []).map(buildStepRecord),
    resources: markers?.resources ?? null,
    caches: {
      rust_target: markers?.rust_cache ?? null,
      node: markers?.node_cache ?? null,
      sccache: markers?.sccache ?? null,
    },
  };
}

function buildRunRecord({ repository, run, jobs, markersByJobId }) {
  const headRepository = run.head_repository?.full_name ?? null;
  const completedAt = latestJobCompletion(jobs) || run.updated_at || null;

  return {
    schema: 1,
    repository,
    workflow: {
      id: run.workflow_id,
      name: run.name,
      path: run.path,
    },
    run: {
      id: run.id,
      attempt: run.run_attempt,
      number: run.run_number,
      event: run.event,
      status: run.status,
      conclusion: run.conclusion,
      head_branch: run.head_branch,
      head_sha: run.head_sha,
      head_repository: headRepository,
      is_fork: headRepository !== repository,
      actor: run.actor?.login ?? null,
      created_at: run.created_at,
      started_at: run.run_started_at,
      completed_at: completedAt,
      duration_seconds: durationSeconds(run.run_started_at, completedAt),
      url: run.html_url,
    },
    jobs: jobs.map((job) =>
      buildJobRecord(job, markersByJobId?.[job.id] ?? null),
    ),
  };
}

module.exports = { buildRunRecord };
