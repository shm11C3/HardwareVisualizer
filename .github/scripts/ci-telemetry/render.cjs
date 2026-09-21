// Renders the sticky PR comment from run records that aggregate.cjs built.
// This module is pure (no network, no fs) so it can be tested with plain
// fixtures and so the same rendering always happens regardless of which
// invocation of the (stateless, idempotent) aggregator produced the data.
//
// Every job/step/workflow name here originates from the GitHub API but its
// *content* was chosen by whatever produced the run (including a fork or
// Dependabot PR): a job name, step name, or run-name override is
// PR-controlled text. It is rendered only through `codeSpan` (Markdown code
// span, backticks/pipes/newlines stripped, length-capped) or
// `escapeHtmlInline` (HTML-entity escaped, for the one spot — the
// `<summary>` — where a code span does not protect us, because GitHub does
// not render Markdown code spans inside raw HTML blocks). Never interpolate
// an untrusted name any other way.
"use strict";

const MAX_INLINE_LENGTH = 80;
const BODY_LENGTH_CAP = 60000;
const HARD_LENGTH_CAP = 65536; // GitHub's issue/PR comment body limit.
const MAX_JOB_ROWS = 40;
const MIN_JOB_ROWS = 1;
const MAX_SLOWEST_STEPS = 5;
const MIN_STEP_DURATION_SECONDS = 10;
const DETAILS_MIN_DURATION_SECONDS = 60;

const RESULT_ICONS = {
  success: "✅",
  failure: "❌",
  cancelled: "🚫",
  skipped: "⏭️",
  timed_out: "⌛",
};

function resultIcon(conclusion) {
  return RESULT_ICONS[conclusion] ?? "⚠️";
}

function resultCell(conclusion) {
  const word = conclusion ?? "unknown";
  return `${resultIcon(conclusion)} ${word}`;
}

// Strips the characters that could break out of a Markdown code span (`) or
// a table cell (|, newlines), then caps length. This is deliberately a
// deletion, not a substitution, per the trust-model contract: untrusted
// names are "stripped and length capped", not re-encoded.
function sanitizeInline(text) {
  const value = typeof text === "string" ? text : String(text ?? "");
  const cleaned = value.replace(/[`|\r\n]/g, "");
  return cleaned.length > MAX_INLINE_LENGTH
    ? `${cleaned.slice(0, MAX_INLINE_LENGTH - 1)}…`
    : cleaned;
}

function codeSpan(text) {
  return `\`${sanitizeInline(text)}\``;
}

// Used only inside the <summary><b>...</b></summary> HTML block, where a
// Markdown code span would render as literal backticks instead of being
// interpreted, so it offers no protection there.
function escapeHtmlInline(text) {
  const value = typeof text === "string" ? text : String(text ?? "");
  const capped =
    value.length > MAX_INLINE_LENGTH
      ? `${value.slice(0, MAX_INLINE_LENGTH - 1)}…`
      : value;
  return capped
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;")
    .replace(/[\r\n]/g, " ");
}

function formatDuration(seconds) {
  if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) {
    return "–";
  }
  const total = Math.max(0, Math.round(seconds));
  if (total < 60) return `${total}s`;
  if (total < 3600) {
    const minutes = Math.floor(total / 60);
    const remainder = total % 60;
    return `${minutes}m ${String(remainder).padStart(2, "0")}s`;
  }
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  return `${hours}h ${String(minutes).padStart(2, "0")}m`;
}

function formatGiB(bytes, decimals) {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes)) {
    return "–";
  }
  return (bytes / 1024 ** 3).toFixed(decimals);
}

function formatNow(now) {
  const date = now instanceof Date ? now : new Date(now);
  const iso = date.toISOString();
  return `${iso.slice(0, 10)} ${iso.slice(11, 16)} UTC`;
}

function cpuCell(resources) {
  if (!resources?.cpu_pct) return "–";
  const { avg, p95 } = resources.cpu_pct;
  return `${Math.round(avg)}% / ${Math.round(p95)}%`;
}

function memCell(resources) {
  if (!resources?.mem_used_bytes) return "–";
  const used = formatGiB(resources.mem_used_bytes.max, 1);
  const total = formatGiB(resources.mem_total_bytes, 1);
  return `${used} / ${total} GiB`;
}

function diskCell(resources) {
  if (!resources?.disk_used_bytes || resources.disk_total_bytes === null) {
    return "–";
  }
  const used = formatGiB(resources.disk_used_bytes.max, 0);
  const total = formatGiB(resources.disk_total_bytes, 0);
  return `${used} / ${total} GiB`;
}

function sccachePart(label, hits, misses, ratePct) {
  const total = hits + misses;
  if (total === 0) return `${label} n/a`;
  const pct =
    ratePct !== null ? Math.round(ratePct) : Math.round((hits / total) * 100);
  return `${label} ${pct}% (${hits}/${total})`;
}

// rust-cache reports only "was the restore key an exact match", so a
// non-exact restore is labelled "partial/miss": we cannot tell a useful
// prefix restore apart from a full cache miss from that one boolean.
function cachesCell(caches) {
  const segments = [];
  if (caches?.rust_target) {
    segments.push(
      caches.rust_target.exact_hit ? "target exact" : "target partial/miss",
    );
  }
  if (caches?.node) {
    segments.push(caches.node.exact_hit ? "npm exact" : "npm miss");
  }
  if (caches?.sccache) {
    const s = caches.sccache;
    segments.push(
      `sccache ${sccachePart("Rust", s.rust_hits, s.rust_misses, s.rust_hit_rate_pct)}`,
    );
    segments.push(
      sccachePart("C/C++", s.cpp_hits, s.cpp_misses, s.cpp_hit_rate_pct),
    );
    if (s.cache_errors > 0) {
      segments.push(`⚠️ ${s.cache_errors} cache errors`);
    }
  }
  return segments.length > 0 ? segments.join(" · ") : "–";
}

function executedJobs(jobs) {
  return jobs.filter((job) => job.conclusion !== "skipped");
}

function longestQueueSeconds(jobs) {
  let max = null;
  for (const job of executedJobs(jobs)) {
    if (job.queue_seconds === null) continue;
    if (max === null || job.queue_seconds > max) max = job.queue_seconds;
  }
  return max;
}

function workflowLinkCell(name, url, attempt) {
  const label = `[${codeSpan(name)}](${url})`;
  return attempt > 1 ? `${label} (attempt ${attempt})` : label;
}

function buildSummaryRows(records, pendingRuns) {
  const completedRows = records
    .slice()
    .sort(
      (a, b) => (b.run.duration_seconds ?? -1) - (a.run.duration_seconds ?? -1),
    )
    .map((record) => [
      workflowLinkCell(
        record.workflow.name,
        record.run.url,
        record.run.attempt,
      ),
      resultCell(record.run.conclusion),
      formatDuration(record.run.duration_seconds),
      formatDuration(longestQueueSeconds(record.jobs)),
      String(executedJobs(record.jobs).length),
    ]);

  const pendingRows = pendingRuns.map((pending) => [
    workflowLinkCell(
      pending.workflow.name,
      pending.run.url,
      pending.run.attempt,
    ),
    "⏳ in progress",
    "–",
    "–",
    "–",
  ]);

  return [...completedRows, ...pendingRows];
}

// A run's telemetry is worth expanding into a <details> block only when
// there is something to look at: real wall-clock cost, or resource samples
// collected by the sampler action.
function shouldShowDetails(record) {
  const executed = executedJobs(record.jobs);
  const totalDuration = executed.reduce(
    (sum, job) => sum + (job.duration_seconds ?? 0),
    0,
  );
  const hasResources = executed.some((job) => job.resources !== null);
  return totalDuration >= DETAILS_MIN_DURATION_SECONDS || hasResources;
}

function buildJobRows(jobs, maxRows) {
  const sorted = executedJobs(jobs)
    .slice()
    .sort((a, b) => (b.duration_seconds ?? -1) - (a.duration_seconds ?? -1));
  const shown = sorted.slice(0, maxRows);
  const rows = shown.map((job) => [
    codeSpan(job.name),
    resultIcon(job.conclusion),
    formatDuration(job.queue_seconds),
    formatDuration(job.duration_seconds),
    cpuCell(job.resources),
    memCell(job.resources),
    diskCell(job.resources),
    cachesCell(job.caches),
  ]);
  return { rows, remaining: sorted.length - shown.length };
}

function buildSlowestSteps(jobs) {
  const steps = [];
  for (const job of executedJobs(jobs)) {
    for (const step of job.steps) {
      if (
        step.duration_seconds === null ||
        step.duration_seconds < MIN_STEP_DURATION_SECONDS
      ) {
        continue;
      }
      steps.push({
        jobName: job.name,
        stepName: step.name,
        duration: step.duration_seconds,
      });
    }
  }
  return steps
    .sort((a, b) => b.duration - a.duration)
    .slice(0, MAX_SLOWEST_STEPS);
}

function renderDetails(record, { includeSlowestSteps, maxJobRows }) {
  const executed = executedJobs(record.jobs);
  const skippedCount = record.jobs.length - executed.length;
  // <code> is load-bearing, not styling: GitHub turns @mentions, #refs and
  // bare URLs into live links (and notifications) even inside
  // entity-escaped HTML, and skips only text inside <code>/<pre>/<a>.
  // Verified against GitHub's Markdown API with a hostile workflow name.
  const nameHtml = `<b><code>${escapeHtmlInline(record.workflow.name)}</code></b>`;
  const summaryLine =
    skippedCount > 0
      ? `<summary>${nameHtml} · ${executed.length} jobs · ${skippedCount} skipped</summary>`
      : `<summary>${nameHtml} · ${executed.length} jobs</summary>`;

  const { rows, remaining } = buildJobRows(record.jobs, maxJobRows);
  const jobTableLines = [
    "| Job | Result | Queue | Duration | CPU avg / p95 | Mem peak | Disk peak | Caches |",
    "| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |",
    ...rows.map((cells) => `| ${cells.join(" | ")} |`),
  ];
  if (remaining > 0) {
    jobTableLines.push(`| … and ${remaining} more | | | | | | | |`);
  }

  const slowest = includeSlowestSteps ? buildSlowestSteps(record.jobs) : [];
  const slowestLines =
    slowest.length > 0
      ? [
          "",
          "**Slowest steps**",
          "",
          "| Step | Duration |",
          "| --- | ---: |",
          ...slowest.map(
            (step) =>
              `| ${codeSpan(step.jobName)} › ${codeSpan(step.stepName)} | ${formatDuration(step.duration)} |`,
          ),
        ]
      : [];

  return [
    "<details>",
    summaryLine,
    "",
    ...jobTableLines,
    ...slowestLines,
    "",
    "</details>",
  ].join("\n");
}

function renderBody({
  headSha,
  records,
  pendingRuns,
  now,
  includeSlowestSteps,
  maxJobRows,
}) {
  const shortSha = (headSha || "").slice(0, 7);
  const lines = [
    "<!-- ci-telemetry -->",
    "## CI telemetry",
    "",
    `Commit \`${shortSha}\` · updated ${formatNow(now)}`,
    "",
    "| Workflow | Result | Wall time | Longest queue | Jobs |",
    "| --- | --- | ---: | ---: | ---: |",
    ...buildSummaryRows(records, pendingRuns).map(
      (cells) => `| ${cells.join(" | ")} |`,
    ),
  ];

  const detailBlocks = records
    .filter(shouldShowDetails)
    .map((record) =>
      renderDetails(record, { includeSlowestSteps, maxJobRows }),
    );
  if (detailBlocks.length > 0) {
    lines.push("");
    lines.push(detailBlocks.join("\n\n"));
  }

  lines.push("");
  lines.push(
    "<sub>Generated by the CI telemetry workflow from the GitHub API and job log markers. Queue = time from job creation to start.</sub>",
  );

  return lines.join("\n");
}

// The aggregator is stateless and idempotent per head SHA: every invocation
// re-renders the complete comment from the API rather than patching a prior
// version. That is deliberate — GitHub cancels older *pending* runs sharing
// a concurrency group, so any single invocation may be the only one that
// ever runs for a given commit, and it must produce a complete picture on
// its own.
//
// GitHub caps a comment body at 65536 characters. If the full render
// exceeds a smaller working cap, drop the "Slowest steps" tables first
// (least essential detail), then progressively shrink how many job rows
// each details block shows, before falling back to a hard truncation as a
// last resort so the cap is never exceeded regardless of input size.
function renderComment({ headSha, records, pendingRuns, now }) {
  let body = renderBody({
    headSha,
    records,
    pendingRuns,
    now,
    includeSlowestSteps: true,
    maxJobRows: MAX_JOB_ROWS,
  });

  if (body.length > BODY_LENGTH_CAP) {
    body = renderBody({
      headSha,
      records,
      pendingRuns,
      now,
      includeSlowestSteps: false,
      maxJobRows: MAX_JOB_ROWS,
    });
  }

  let maxJobRows = MAX_JOB_ROWS;
  while (body.length > BODY_LENGTH_CAP && maxJobRows > MIN_JOB_ROWS) {
    maxJobRows = Math.max(MIN_JOB_ROWS, Math.floor(maxJobRows / 2));
    body = renderBody({
      headSha,
      records,
      pendingRuns,
      now,
      includeSlowestSteps: false,
      maxJobRows,
    });
  }

  if (body.length > HARD_LENGTH_CAP) {
    body = `${body.slice(0, HARD_LENGTH_CAP - 200)}\n\n<!-- truncated: comment exceeded GitHub's length limit -->`;
  }

  return body;
}

module.exports = {
  renderComment,
  formatDuration,
  formatGiB,
  formatNow,
  codeSpan,
  sanitizeInline,
  escapeHtmlInline,
};
