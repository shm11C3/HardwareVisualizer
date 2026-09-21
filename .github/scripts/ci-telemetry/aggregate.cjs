#!/usr/bin/env node
// Orchestrates the CI telemetry aggregation: reads run/job timing from the
// GitHub REST API, downloads job logs to pull out marker lines, builds
// run records, renders the PR comment, and upserts it. This is the only
// file in ci-telemetry/ that performs I/O; markers.cjs, record.cjs, and
// render.cjs are pure so they can run identically here and in tests.
//
// Trust model: this script is invoked by a `workflow_run` workflow, which
// runs from the default branch with a write token even for a fork or
// Dependabot PR. It must never check out or execute PR code. Everything a
// PR can influence — job/step/workflow names, log contents, branch names —
// is treated as untrusted data: markers are validated against a strict
// schema (markers.cjs) and names are only ever rendered through an escaping
// helper (render.cjs). This script itself only ever calls the GitHub REST
// API with ids supplied via env/CLI, never interpolates untrusted values
// into a shell command, and (in --dry-run) refuses to perform any mutating
// request.
"use strict";

const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");

const { extractMarkers } = require("./markers.cjs");
const { buildRunRecord } = require("./record.cjs");
const { renderComment } = require("./render.cjs");

const API_BASE = "https://api.github.com";
const TELEMETRY_STEP_NAME = "Start CI telemetry";
const MAX_LOG_DOWNLOADS_PER_RUN = 40;
const MAX_CONCURRENT_LOG_DOWNLOADS = 4;
const MARKER_COMMENT_TAG = "<!-- ci-telemetry -->";

function parseArgs(argv) {
  const args = { dryRun: false, allLogs: false, runId: null };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--dry-run") {
      args.dryRun = true;
    } else if (arg === "--all-logs") {
      args.allLogs = true;
    } else if (arg === "--run-id") {
      args.runId = argv[i + 1];
      i += 1;
    }
  }
  return args;
}

function resolveRunId(cliRunId) {
  const value = cliRunId || process.env.RUN_ID;
  if (!value || !/^\d{1,20}$/.test(value)) {
    throw new Error(
      `RUN_ID must be a 1-20 digit numeric string (env RUN_ID or --run-id), got: ${JSON.stringify(value)}`,
    );
  }
  return value;
}

// A tiny REST client shared between local runs and Actions runs, so a local
// --dry-run genuinely exercises the same request path CI uses. In
// --dry-run, any non-GET request throws instead of hitting the network —
// this is the enforcement mechanism behind "dry-run never writes", not just
// a convention the caller has to remember to follow.
function createClient({ token, dryRun }) {
  async function rawRequest(method, urlPath, { query, body } = {}) {
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
    const headers = {
      Accept: "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
      Authorization: `Bearer ${token}`,
    };
    if (body !== undefined) headers["Content-Type"] = "application/json";
    return fetch(url, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  }

  async function requestJson(method, urlPath, options) {
    const response = await rawRequest(method, urlPath, options);
    if (!response.ok) {
      const text = await response.text().catch(() => "");
      const error = new Error(
        `GitHub API ${method} ${urlPath} failed: ${response.status} ${text.slice(0, 500)}`,
      );
      error.status = response.status;
      throw error;
    }
    if (response.status === 204) return null;
    return response.json();
  }

  // Fetches every page of a list endpoint at per_page=100. `extractItems`
  // pulls the array out of the page (some endpoints return a bare array,
  // others wrap it, e.g. `{ jobs: [...] }`).
  async function paginate(urlPath, query, extractItems) {
    const items = [];
    for (let page = 1; ; page += 1) {
      const data = await requestJson("GET", urlPath, {
        query: { ...query, per_page: 100, page },
      });
      const pageItems = extractItems ? extractItems(data) : data;
      items.push(...pageItems);
      if (pageItems.length < 100) break;
    }
    return items;
  }

  async function fetchLogText(jobId) {
    try {
      const response = await rawRequest(
        "GET",
        `/repos/${process.env.GITHUB_REPOSITORY}/actions/jobs/${jobId}/logs`,
      );
      if (!response.ok) return null;
      return await response.text();
    } catch {
      return null;
    }
  }

  return { requestJson, paginate, fetchLogText };
}

async function mapWithConcurrency(items, limit, mapper) {
  const results = new Array(items.length);
  let nextIndex = 0;
  async function worker() {
    for (;;) {
      const current = nextIndex;
      nextIndex += 1;
      if (current >= items.length) return;
      results[current] = await mapper(items[current], current);
    }
  }
  const workerCount = Math.max(1, Math.min(limit, items.length));
  await Promise.all(Array.from({ length: workerCount }, () => worker()));
  return results;
}

function keepLatestRunPerWorkflow(runs) {
  const latestByWorkflow = new Map();
  for (const run of runs) {
    const existing = latestByWorkflow.get(run.workflow_id);
    if (
      !existing ||
      Date.parse(run.created_at) > Date.parse(existing.created_at)
    ) {
      latestByWorkflow.set(run.workflow_id, run);
    }
  }
  return [...latestByWorkflow.values()];
}

function toPendingEntry(run) {
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

function jobNeedsLog(job, allLogs) {
  if (job.conclusion === "skipped") return false;
  if (allLogs) return true;
  return (job.steps || []).some((step) => step.name === TELEMETRY_STEP_NAME);
}

async function buildRecordForCompletedRun({
  client,
  repository,
  run,
  allLogs,
}) {
  const jobs = await client.paginate(
    `/repos/${repository}/actions/runs/${run.id}/attempts/${run.run_attempt}/jobs`,
    {},
    (data) => data.jobs,
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

  const markersByJobId = {};
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

async function resolvePullRequestNumber({ client, repository, triggerRun }) {
  const fromRun = triggerRun.pull_requests?.[0]?.number;
  if (fromRun) return fromRun;

  // Forks (and Dependabot) never populate `pull_requests` on the run object,
  // so fall back to searching open PRs by head ref.
  const owner = triggerRun.head_repository?.owner?.login;
  const branch = triggerRun.head_branch;
  if (!owner || !branch) return null;

  const candidates = await client.requestJson(
    "GET",
    `/repos/${repository}/pulls`,
    { query: { state: "open", head: `${owner}:${branch}` } },
  );
  return candidates?.[0]?.number ?? null;
}

async function findExistingComment({ client, repository, prNumber }) {
  const comments = await client.paginate(
    `/repos/${repository}/issues/${prNumber}/comments`,
    {},
    (data) => data,
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
}) {
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

  const pr = await client.requestJson(
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
    if (error.status === 403) {
      console.log(
        `::warning title=CI telemetry::Could not write PR comment (403 Forbidden): ${error.message}`,
      );
      return;
    }
    throw error;
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const runId = resolveRunId(args.runId);
  const repository = process.env.GITHUB_REPOSITORY;
  const token = process.env.GITHUB_TOKEN;
  if (!repository) throw new Error("GITHUB_REPOSITORY is required");
  if (!token) throw new Error("GITHUB_TOKEN is required");

  const client = createClient({ token, dryRun: args.dryRun });

  const triggerRun = await client.requestJson(
    "GET",
    `/repos/${repository}/actions/runs/${runId}`,
  );
  const headSha = triggerRun.head_sha;
  console.log(
    `Trigger run ${triggerRun.id} (${triggerRun.name}) event=${triggerRun.event} status=${triggerRun.status} head_sha=${headSha}`,
  );

  let runSet;
  if (triggerRun.event === "pull_request") {
    const runsForSha = await client.paginate(
      `/repos/${repository}/actions/runs`,
      { head_sha: headSha, event: "pull_request" },
      (data) => data.workflow_runs,
    );
    runSet = keepLatestRunPerWorkflow(runsForSha);
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

  const records = [];
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
    process.env.RUNNER_TEMP || os.tmpdir(),
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

  if (process.env.GITHUB_STEP_SUMMARY) {
    fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${body}\n`);
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

// Only run when invoked directly (`node aggregate.cjs`), not when required
// by the test suite to exercise the pure helpers below.
if (require.main === module) {
  main().catch((error) => {
    console.error(error.stack || String(error));
    process.exitCode = 1;
  });
}

module.exports = { parseArgs, resolveRunId, keepLatestRunPerWorkflow };
