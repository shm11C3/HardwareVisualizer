const { appendFileSync, readFileSync } = require("node:fs");
const os = require("node:os");
const {
  parseSamples,
  resolveIntervalSeconds,
  summarizeSamples,
} = require("./metrics.cjs");

function escapeWorkflowCommand(value) {
  return String(value)
    .replaceAll("%", "%25")
    .replaceAll("\r", "%0D")
    .replaceAll("\n", "%0A");
}

function warn(message) {
  console.log(
    `::warning title=CI telemetry::${escapeWorkflowCommand(message)}`,
  );
}

function formatGiB(bytes) {
  return (bytes / 1024 ** 3).toFixed(1);
}

function appendSummary(marker) {
  if (!process.env.GITHUB_STEP_SUMMARY) return;

  const cpu = marker.cpu_pct
    ? `${marker.cpu_pct.avg}% / ${marker.cpu_pct.p95}% / ${marker.cpu_pct.max}%`
    : "n/a";
  const mem = marker.mem_used_bytes
    ? `${formatGiB(marker.mem_used_bytes.max)} / ${formatGiB(marker.mem_total_bytes)} GiB`
    : "n/a";
  const disk =
    marker.disk_used_bytes && marker.disk_total_bytes
      ? `${formatGiB(marker.disk_used_bytes.start)} -> ${formatGiB(marker.disk_used_bytes.end)} / ${formatGiB(marker.disk_total_bytes)} GiB`
      : "n/a";

  const rows = [
    [
      "Samples / interval",
      `${marker.sample_count} / ${marker.interval_seconds}s`,
    ],
    ["CPU avg / p95 / max", cpu],
    ["Memory peak", mem],
    ["Disk used (start -> end / total)", disk],
  ];

  appendFileSync(
    process.env.GITHUB_STEP_SUMMARY,
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
  const pid = process.env.STATE_sampler_pid;
  const samplesFile = process.env.STATE_samples_file;

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
      runnerOs: process.env.RUNNER_OS || "unknown",
      runnerArch: process.env.RUNNER_ARCH || "unknown",
      cpuCount: os.cpus().length,
    });

    // Plain stdout line, not a `::notice`: workflow commands are consumed
    // by the runner and never reach raw job logs, but the aggregator
    // workflow (a separate job) reads this data back out of those logs.
    console.log(`CI_JOB_METRICS_JSON=${JSON.stringify(marker)}`);

    appendSummary(marker);
  }
} catch (error) {
  warn(`Unable to report telemetry: ${String(error.message || error)}`);
}
