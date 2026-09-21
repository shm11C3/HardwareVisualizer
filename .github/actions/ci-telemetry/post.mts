import { appendFileSync, readFileSync } from "node:fs";
import * as os from "node:os";
import {
  parseSamples,
  resolveIntervalSeconds,
  summarizeSamples,
} from "./metrics.mts";

// Derived from summarizeSamples's own return type rather than importing
// schema.mts directly, so metrics.mts stays the action's single type-only
// bridge to the aggregator's contract (see metrics.mts's import comment).
type JobResourcesMarker = ReturnType<typeof summarizeSamples>;

function escapeWorkflowCommand(value: unknown): string {
  return String(value)
    .replaceAll("%", "%25")
    .replaceAll("\r", "%0D")
    .replaceAll("\n", "%0A");
}

function warn(message: string): void {
  console.log(
    `::warning title=CI telemetry::${escapeWorkflowCommand(message)}`,
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

// Accepts null so every caller below can pass a byte field straight through
// without narrowing first; `null` divides to 0 (matching plain JS's numeric
// coercion of null), same as the original untyped helper produced.
function formatGiB(bytes: number | null): string {
  return ((bytes ?? 0) / 1024 ** 3).toFixed(1);
}

// Rounded per-thread averages, index order, for <=8 threads (an unmeasured
// thread renders as "–", keeping its position visible); a min–max range
// above that, ignoring unmeasured threads (same shape rule the PR comment
// cell uses in render.mts, kept independent here since post.mts has no
// dependency on render.mts). "n/a" when every thread is unmeasured.
function cpuThreadsSummary(
  cpuThreads: JobResourcesMarker["cpu_threads"],
): string {
  if (!cpuThreads || cpuThreads.length === 0) return "n/a";
  const rounded = cpuThreads.map((thread) =>
    thread ? Math.round(thread.avg) : null,
  );
  const measured = rounded.filter((value): value is number => value !== null);
  if (measured.length === 0) return "n/a";
  if (rounded.length > 8) {
    return `${Math.min(...measured)}–${Math.max(...measured)}%`;
  }
  return rounded
    .map((value) => (value === null ? "–" : `${value}%`))
    .join(" · ");
}

function appendSummary(marker: JobResourcesMarker): void {
  const summaryPath = process.env["GITHUB_STEP_SUMMARY"];
  if (!summaryPath) return;

  const cpu = marker.cpu_pct
    ? `${marker.cpu_pct.avg}% / ${marker.cpu_pct.p95}% / ${marker.cpu_pct.max}%`
    : "n/a";
  const cpuThreads = cpuThreadsSummary(marker.cpu_threads);
  const mem = marker.mem_used_bytes
    ? `${formatGiB(marker.mem_used_bytes.max)} / ${formatGiB(marker.mem_total_bytes)} GiB`
    : "n/a";
  const disk =
    marker.disk_used_bytes && marker.disk_total_bytes
      ? `${formatGiB(marker.disk_used_bytes.start)} -> ${formatGiB(marker.disk_used_bytes.end)} / ${formatGiB(marker.disk_total_bytes)} GiB`
      : "n/a";

  const rows: [string, string][] = [
    [
      "Samples / interval",
      `${marker.sample_count} / ${marker.interval_seconds}s`,
    ],
    ["CPU avg / p95 / max", cpu],
    ["CPU per thread (avg)", cpuThreads],
    ["Memory peak", mem],
    ["Disk used (start -> end / total)", disk],
  ];

  appendFileSync(
    summaryPath,
    [
      "### CI telemetry",
      "",
      "| Metric | Value |",
      "| --- | --- |",
      ...rows.map(([name, value]) => `| ${name} | ${value} |`),
      "",
    ].join("\n"),
  );
}

try {
  const pid = process.env["STATE_sampler_pid"];
  const samplesFile = process.env["STATE_samples_file"];

  if (pid) {
    try {
      process.kill(Number(pid));
    } catch {
      // Already exited (e.g. it hit its own 6h self-terminate deadline).
    }
  }

  if (!samplesFile) {
    warn("No samples file recorded by the main step; skipping report.");
  } else {
    const samples = parseSamples(readFileSync(samplesFile, "utf8"));
    const marker = summarizeSamples(samples, {
      intervalSeconds: resolveIntervalSeconds(
        process.env["INPUT_INTERVAL-SECONDS"],
      ),
      runnerOs: process.env["RUNNER_OS"] || "unknown",
      runnerArch: process.env["RUNNER_ARCH"] || "unknown",
      cpuCount: os.cpus().length,
    });

    // Plain stdout line, not a `::notice`: workflow commands are consumed
    // by the runner and never reach raw job logs, but the aggregator
    // workflow (a separate job) reads this data back out of those logs.
    console.log(`CI_JOB_METRICS_JSON=${JSON.stringify(marker)}`);

    appendSummary(marker);
  }
} catch (error) {
  warn(`Unable to report telemetry: ${errorMessage(error)}`);
}
