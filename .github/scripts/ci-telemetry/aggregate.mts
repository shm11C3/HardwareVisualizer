#!/usr/bin/env node
// Orchestrates the CI telemetry aggregation: reads run/job timing from the
// GitHub REST API, downloads job logs to pull out marker lines, builds
// run records, renders the PR comment, and upserts it. This is the only
// file in ci-telemetry/ that performs I/O; markers.mts, record.mts, and
// render.mts are pure so they can run identically here and in tests.
//
// Trust model: this script is invoked by a `workflow_run` workflow, which
// runs from the default branch with a write token even for a fork or
// Dependabot PR. It must never check out or execute PR code. Everything a
// PR can influence — job/step/workflow names, log contents, branch names —
// is treated as untrusted data: markers are validated against a strict
// schema (markers.mts) and names are only ever rendered through an escaping
// helper (render.mts). This script itself only ever calls the GitHub REST
// API with ids supplied via env/CLI, never interpolates untrusted values
// into a shell command, and (in --dry-run) refuses to perform any mutating
// request.

import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  includeTriggerRun,
  keepLatestRunPerWorkflow,
  parseArgs,
  resolveRunId,
} from "./cli.mts";
import { extractMarkers } from "./markers.mts";
import {
  buildRunRecord,
  type GitHubJob,
  type GitHubRun,
  type MarkersByJobId,
} from "./record.mts";
import { renderComment } from "./render.mts";
import type { PendingRunEntry, RunRecord } from "./schema.mts";

const API_BASE = "https://api.github.com";
const TELEMETRY_STEP_NAME = "Start CI telemetry";
const MAX_LOG_DOWNLOADS_PER_RUN = 40;
const MAX_CONCURRENT_LOG_DOWNLOADS = 4;
const MARKER_COMMENT_TAG = "<!-- ci-telemetry -->";

type GitHubComment = {
  id: number;
  user?: { type: string } | null;
  body?: string | null;
};

type RequestOptions = {
  query?: Record<string, string | number | undefined | null>;
  body?: unknown;
};

type HttpError = Error & { status: number };

function isHttpError(error: unknown): error is HttpError {
  return (
    error instanceof Error &&
    typeof (error as Partial<HttpError>).status === "number"
  );
}

// A tiny REST client shared between local runs and Actions runs, so a local
// --dry-run genuinely exercises the same request path CI uses. In
// --dry-run, any non-GET request throws instead of hitting the network —
// this is the enforcement mechanism behind "dry-run never writes", not just
// a convention the caller has to remember to follow.
function createClient({ token, dryRun }: { token: string; dryRun: boolean }) {
  async function rawRequest(
    method: string,
    urlPath: string,
    options: RequestOptions = {},
  ): Promise<Response> {
    const { query, body } = options;
    if (dryRun && method !== "GET") {
      throw new Error(
        `Refusing to perform ${method} ${urlPath} while --dry-run is set`,
      );
    }
    // Paths only: the bearer token must never be sent to a host other than
    // the GitHub API, so absolute URLs are not accepted here.
    const url = new URL(`${API_BASE}${urlPath}`);
    if (query) {
      for (const [key, value] of Object.entries(query)) {
        if (value !== undefined && value !== null) {
          url.searchParams.set(key, String(value));
        }
      }
    }
    const headers: Record<string, string> = {
      Accept: "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
      Authorization: `Bearer ${token}`,
    };
    if (body !== undefined) headers["Content-Type"] = "application/json";
    return fetch(url, {
      method,
      headers,
      // null (not undefined): with exactOptionalPropertyTypes, RequestInit's
      // `body` must be an explicit value when the key is present, and null
      // means "no body" exactly as undefined would have.
      body: body === undefined ? null : JSON.stringify(body),
    });
  }

  // GitHub's API shape is trusted (it is our own authenticated call to a
  // documented endpoint); only the free-text *content* inside it is
  // untrusted, and that is handled by markers.mts/render.mts, not here. `T`
  // lets each call site say what shape it expects instead of leaking `any`.
  async function requestJson<T = unknown>(
    method: string,
    urlPath: string,
    options?: RequestOptions,
  ): Promise<T> {
    const response = await rawRequest(method, urlPath, options);
    if (!response.ok) {
      const text = await response.text().catch(() => "");
      const error = new Error(
        `GitHub API ${method} ${urlPath} failed: ${response.status} ${text.slice(0, 500)}`,
      ) as HttpError;
      error.status = response.status;
      throw error;
    }
    if (response.status === 204) return null as T;
    return (await response.json()) as T;
  }

  // Fetches every page of a list endpoint at per_page=100. `extractItems`
  // pulls the array out of the page (some endpoints return a bare array,
  // others wrap it, e.g. `{ jobs: [...] }`).
  async function paginate<T>(
    urlPath: string,
    query: Record<string, string | number | undefined>,
    extractItems: (data: unknown) => T[],
  ): Promise<T[]> {
    const items: T[] = [];
    for (let page = 1; ; page += 1) {
      const data = await requestJson<unknown>("GET", urlPath, {
        query: { ...query, per_page: 100, page },
      });
      const pageItems = extractItems(data);
      items.push(...pageItems);
      if (pageItems.length < 100) break;
    }
    return items;
  }

  async function fetchLogText(jobId: number): Promise<string | null> {
    try {
      const response = await rawRequest(
        "GET",
        `/repos/${process.env["GITHUB_REPOSITORY"]}/actions/jobs/${jobId}/logs`,
      );
      if (!response.ok) return null;
      return await response.text();
    } catch {
      return null;
    }
  }

  return { requestJson, paginate, fetchLogText };
}

type RestClient = ReturnType<typeof createClient>;

async function mapWithConcurrency<T, R>(
  items: T[],
  limit: number,
  mapper: (item: T, index: number) => Promise<R>,
): Promise<R[]> {
  const results: R[] = new Array(items.length);
  let nextIndex = 0;
  async function worker(): Promise<void> {
    for (;;) {
      const current = nextIndex;
      nextIndex += 1;
      if (current >= items.length) return;
      const item = items[current];
      // current < items.length was just checked above, so item is always
      // populated; this check exists only to satisfy the type checker.
      if (item === undefined) continue;
      results[current] = await mapper(item, current);
    }
  }
  const workerCount = Math.max(1, Math.min(limit, items.length));
  await Promise.all(Array.from({ length: workerCount }, () => worker()));
  return results;
}

function toPendingEntry(run: GitHubRun): PendingRunEntry {
  return {
    workflow: { id: run.workflow_id, name: run.name, path: run.path },
    run: {
      id: run.id,
      attempt: run.run_attempt,
      number: run.run_number,
      status: run.status,
      conclusion: run.conclusion,
      url: run.html_url,
    },
  };
}

function jobNeedsLog(job: GitHubJob, allLogs: boolean): boolean {
  if (job.conclusion === "skipped") return false;
  if (allLogs) return true;
  return (job.steps || []).some((step) => step.name === TELEMETRY_STEP_NAME);
}

async function buildRecordForCompletedRun({
  client,
  repository,
  run,
  allLogs,
}: {
  client: RestClient;
  repository: string;
  run: GitHubRun;
  allLogs: boolean;
}): Promise<RunRecord> {
  const jobs = await client.paginate<GitHubJob>(
    `/repos/${repository}/actions/runs/${run.id}/attempts/${run.run_attempt}/jobs`,
    {},
    (data) => (data as { jobs: GitHubJob[] }).jobs,
  );

  const jobsToLog = jobs
    .filter((job) => jobNeedsLog(job, allLogs))
    .slice(0, MAX_LOG_DOWNLOADS_PER_RUN);
  if (
    jobsToLog.length < jobs.filter((job) => jobNeedsLog(job, allLogs)).length
  ) {
    console.log(
      `Run ${run.id} (${run.name}): capping log downloads at ${MAX_LOG_DOWNLOADS_PER_RUN} of ${jobs.length} eligible jobs`,
    );
  }

  const markersByJobId: MarkersByJobId = {};
  await mapWithConcurrency(
    jobsToLog,
    MAX_CONCURRENT_LOG_DOWNLOADS,
    async (job) => {
      const logText = await client.fetchLogText(job.id);
      // A failed/missing log download is non-fatal: that job simply has no
      // markers, same as a job that never printed any.
      markersByJobId[job.id] = logText ? extractMarkers(logText) : null;
    },
  );

  return buildRunRecord({ repository, run, jobs, markersByJobId });
}

async function resolvePullRequestNumber({
  client,
  repository,
  triggerRun,
}: {
  client: RestClient;
  repository: string;
  triggerRun: GitHubRun;
}): Promise<number | null> {
  const fromRun = triggerRun.pull_requests?.[0]?.number;
  if (fromRun) return fromRun;

  // Forks (and Dependabot) never populate `pull_requests` on the run object,
  // so fall back to searching open PRs by head ref.
  const owner = triggerRun.head_repository?.owner?.login;
  const branch = triggerRun.head_branch;
  if (!owner || !branch) return null;

  const candidates = await client.requestJson<{ number: number }[]>(
    "GET",
    `/repos/${repository}/pulls`,
    {
      query: { state: "open", head: `${owner}:${branch}` },
    },
  );
  return candidates?.[0]?.number ?? null;
}

async function findExistingComment({
  client,
  repository,
  prNumber,
}: {
  client: RestClient;
  repository: string;
  prNumber: number;
}): Promise<GitHubComment | null> {
  const comments = await client.paginate<GitHubComment>(
    `/repos/${repository}/issues/${prNumber}/comments`,
    {},
    (data) => data as GitHubComment[],
  );
  return (
    comments.find(
      (comment) =>
        comment.user?.type === "Bot" &&
        comment.body?.includes(MARKER_COMMENT_TAG),
    ) ?? null
  );
}

async function upsertPullRequestComment({
  client,
  repository,
  triggerRun,
  headSha,
  body,
  dryRun,
}: {
  client: RestClient;
  repository: string;
  triggerRun: GitHubRun;
  headSha: string;
  body: string;
  dryRun: boolean;
}): Promise<void> {
  const prNumber = await resolvePullRequestNumber({
    client,
    repository,
    triggerRun,
  });
  if (!prNumber) {
    console.log(
      "No open pull request found for this run's head branch; skipping comment.",
    );
    return;
  }

  const pr = await client.requestJson<{ head: { sha: string } }>(
    "GET",
    `/repos/${repository}/pulls/${prNumber}`,
  );
  if (pr.head.sha !== headSha) {
    console.log(
      `PR #${prNumber} head is now ${pr.head.sha}, not ${headSha}; this run was superseded, skipping comment.`,
    );
    return;
  }

  const existing = await findExistingComment({ client, repository, prNumber });

  if (dryRun) {
    console.log(
      existing
        ? `[dry-run] would update existing comment ${existing.id} on PR #${prNumber}`
        : `[dry-run] would create a new comment on PR #${prNumber}`,
    );
    return;
  }

  try {
    if (existing) {
      await client.requestJson(
        "PATCH",
        `/repos/${repository}/issues/comments/${existing.id}`,
        { body: { body } },
      );
      console.log(`Updated comment ${existing.id} on PR #${prNumber}`);
    } else {
      await client.requestJson(
        "POST",
        `/repos/${repository}/issues/${prNumber}/comments`,
        { body: { body } },
      );
      console.log(`Created a new comment on PR #${prNumber}`);
    }
  } catch (error) {
    // A read-only token (e.g. some fork/Dependabot contexts) is an expected
    // and non-fatal condition: telemetry is a nice-to-have, not a gate.
    if (isHttpError(error) && error.status === 403) {
      console.log(
        `::warning title=CI telemetry::Could not write PR comment (403 Forbidden): ${error.message}`,
      );
      return;
    }
    throw error;
  }
}

async function main(): Promise<void> {
  const args = parseArgs(process.argv.slice(2));
  const runId = resolveRunId(args.runId);
  const repository = process.env["GITHUB_REPOSITORY"];
  const token = process.env["GITHUB_TOKEN"];
  if (!repository) throw new Error("GITHUB_REPOSITORY is required");
  if (!token) throw new Error("GITHUB_TOKEN is required");

  const client = createClient({ token, dryRun: args.dryRun });

  const triggerRun = await client.requestJson<GitHubRun>(
    "GET",
    `/repos/${repository}/actions/runs/${runId}`,
  );
  const headSha = triggerRun.head_sha;
  console.log(
    `Trigger run ${triggerRun.id} (${triggerRun.name}) event=${triggerRun.event} status=${triggerRun.status} head_sha=${headSha}`,
  );

  let runSet: GitHubRun[];
  if (triggerRun.event === "pull_request") {
    const runsForSha = await client.paginate<GitHubRun>(
      `/repos/${repository}/actions/runs`,
      { head_sha: headSha, event: "pull_request" },
      (data) => (data as { workflow_runs: GitHubRun[] }).workflow_runs,
    );
    // The listing can lag behind the run that triggered this aggregation
    // (see includeTriggerRun's own comment); make sure that run is never
    // dropped from the comment just because the listing hasn't caught up.
    runSet = keepLatestRunPerWorkflow(
      includeTriggerRun(runsForSha, triggerRun),
    );
    console.log(
      `Found ${runSet.length} workflow run(s) for head_sha ${headSha}: ${runSet.map((r) => r.name).join(", ")}`,
    );
  } else {
    runSet = [triggerRun];
  }

  const completedRuns = runSet.filter((run) => run.status === "completed");
  const pendingRuns = runSet
    .filter((run) => run.status !== "completed")
    .map(toPendingEntry);

  const records: RunRecord[] = [];
  for (const run of completedRuns) {
    records.push(
      await buildRecordForCompletedRun({
        client,
        repository,
        run,
        allLogs: args.allLogs,
      }),
    );
  }

  const recordsDir = path.join(
    process.env["RUNNER_TEMP"] || os.tmpdir(),
    "ci-telemetry",
  );
  fs.mkdirSync(recordsDir, { recursive: true });
  const recordsPath = path.join(recordsDir, "records.json");
  fs.writeFileSync(recordsPath, JSON.stringify(records, null, 2));

  const body = renderComment({
    headSha,
    records,
    pendingRuns,
    now: new Date(),
  });

  const summaryPath = process.env["GITHUB_STEP_SUMMARY"];
  if (summaryPath) {
    fs.appendFileSync(summaryPath, `${body}\n`);
  }

  if (args.dryRun) {
    console.log(`\nRecords written to: ${recordsPath}\n`);
    console.log("Rendered comment:\n");
    console.log(body);
  }

  if (triggerRun.event === "pull_request") {
    await upsertPullRequestComment({
      client,
      repository,
      triggerRun,
      headSha,
      body,
      dryRun: args.dryRun,
    });
  }
}

function errorStackOrString(error: unknown): string {
  if (error instanceof Error && error.stack) return error.stack;
  return String(error);
}

// aggregate.mts is only ever run directly (`node aggregate.mts`); the pure
// helpers it used to gate behind `require.main === module` now live in
// cli.mts, which the test suite imports without triggering this file's I/O.
// import.meta.main is undefined before Node 24.2, so it cannot replace that
// guard here — this file simply has no guard and always runs main().
main().catch((error: unknown) => {
  console.error(errorStackOrString(error));
  process.exitCode = 1;
});
