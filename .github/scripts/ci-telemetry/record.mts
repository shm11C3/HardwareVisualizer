// Builds the versioned "run record" that is written to records.json and
// (in a follow-up change) uploaded to Cloudflare R2. This module is pure: it
// only shapes data already fetched by aggregate.mts, so it can be unit
// tested with plain fixtures instead of live API responses. Job/step/
// workflow names are stored here exactly as the API returned them — they
// are untrusted PR-controlled strings, but escaping is a render-time
// concern (see render.mts), not a storage-time one.

import type {
  ExtractedMarkers,
  JobRecord,
  RunRecord,
  StepRecord,
} from "./schema.mts";

// The subset of the GitHub REST API's run/job/step shapes this module (and
// aggregate.mts, which fetches the real objects) actually reads. Optional
// fields reflect API responses that can omit them (e.g. a fork run's
// `head_repository.owner`) rather than anything this code requires.
export type GitHubActor = {
  login: string;
};

export type GitHubRepositoryRef = {
  full_name: string;
  owner?: { login: string };
};

export type GitHubStep = {
  number: number;
  name: string;
  status?: string;
  conclusion: string | null;
  started_at: string | null;
  completed_at: string | null;
};

export type GitHubJob = {
  id: number;
  name: string;
  status: string;
  conclusion: string | null;
  labels?: string[];
  created_at: string | null;
  started_at: string | null;
  completed_at: string | null;
  steps?: GitHubStep[];
};

export type GitHubRun = {
  id: number;
  name: string;
  path: string;
  workflow_id: number;
  run_attempt: number;
  run_number: number;
  event: string;
  status: string;
  conclusion: string | null;
  head_branch: string | null;
  head_sha: string;
  head_repository: GitHubRepositoryRef | null;
  actor: GitHubActor | null;
  created_at: string | null;
  run_started_at: string | null;
  updated_at: string | null;
  html_url: string;
  pull_requests?: { number: number }[];
};

export type MarkersByJobId = Record<number, ExtractedMarkers | null>;

// The GitHub API represents an unset step timestamp (typically `started_at`
// on a step that was skipped by an `if:` condition) as the sentinel
// "0001-01-01T00:00:00Z" instead of `null`. Treat any timestamp before this
// app existed as that same "unset" sentinel, otherwise a duration computed
// against it comes out as millions of hours instead of null. Confirmed
// against live data: GET .../actions/runs/{id}/attempts/{n}/jobs for a real
// skipped step returns `"started_at": "0001-01-01T00:00:00Z"` alongside a
// real `completed_at`.
const SENTINEL_EPOCH_MS = Date.UTC(2000, 0, 1);

function toEpochMs(value: string | null | undefined): number | null {
  if (!value) return null;
  const ms = Date.parse(value);
  if (Number.isNaN(ms) || ms < SENTINEL_EPOCH_MS) return null;
  return ms;
}

function durationSeconds(
  startIso: string | null | undefined,
  endIso: string | null | undefined,
): number | null {
  const start = toEpochMs(startIso);
  const end = toEpochMs(endIso);
  if (start === null || end === null) return null;
  return Math.round((end - start) / 1000);
}

// Finds the latest job completion timestamp. Falls back to the run's own
// `updated_at` (handled by the caller) only when no job has a completed_at
// at all — e.g. an empty job list, which should not happen for a completed
// run but is not assumed away here.
function latestJobCompletion(jobs: readonly GitHubJob[]): string | null {
  let latest: string | null = null;
  for (const job of jobs) {
    if (!job.completed_at) continue;
    if (latest === null || Date.parse(job.completed_at) > Date.parse(latest)) {
      latest = job.completed_at;
    }
  }
  return latest;
}

function buildStepRecord(step: GitHubStep): StepRecord {
  return {
    number: step.number,
    name: step.name,
    conclusion: step.conclusion,
    duration_seconds: durationSeconds(step.started_at, step.completed_at),
  };
}

function buildJobRecord(
  job: GitHubJob,
  markers: ExtractedMarkers | null | undefined,
): JobRecord {
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

export function buildRunRecord({
  repository,
  run,
  jobs,
  markersByJobId,
}: {
  repository: string;
  run: GitHubRun;
  jobs: GitHubJob[];
  markersByJobId?: MarkersByJobId | null;
}): RunRecord {
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
