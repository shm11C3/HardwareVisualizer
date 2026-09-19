const { execFileSync } = require("node:child_process");
const { appendFileSync } = require("node:fs");
const { buildCacheReport } = require("./metrics.cjs");

const sccache = process.env.SCCACHE_PATH || "sccache";

function escapeWorkflowCommand(value) {
  return String(value)
    .replaceAll("%", "%25")
    .replaceAll("\r", "%0D")
    .replaceAll("\n", "%0A");
}

function warning(message) {
  console.log(
    `::warning title=sccache cache health::${escapeWorkflowCommand(message)}`,
  );
}

try {
  const output = execFileSync(
    sccache,
    ["--show-stats", "--stats-format=json"],
    {
      encoding: "utf8",
    },
  );
  const result = JSON.parse(output);
  const { metrics, warnings } = buildCacheReport(result, process.env);
  const rustRequests = metrics.rust_hits + metrics.rust_misses;
  const cppRequests = metrics.cpp_hits + metrics.cpp_misses;
  const rustRate = metrics.rust_hit_rate_pct ?? "n/a";
  const cppRate = metrics.cpp_hit_rate_pct ?? "n/a";

  console.log(`CACHE_METRICS_JSON=${JSON.stringify(metrics)}`);
  console.log(
    `::notice title=sccache metrics::backend=${escapeWorkflowCommand(metrics.cache_location)} rust=${metrics.rust_hits}/${rustRequests} (${rustRate}%) cpp=${metrics.cpp_hits}/${cppRequests} (${cppRate}%) errors=${metrics.cache_errors}`,
  );

  if (process.env.GITHUB_STEP_SUMMARY) {
    const rows = [
      ["Backend", `\`${metrics.cache_location.replaceAll("|", "\\|")}\``],
      ["Target cache exact hit", `\`${metrics.target_cache_exact_hit}\``],
      ["Compile requests", String(metrics.compile_requests)],
      [
        "Rust hits / misses / rate",
        `${metrics.rust_hits} / ${metrics.rust_misses} / ${rustRate}%`,
      ],
      [
        "C/C++ hits / misses / rate",
        `${metrics.cpp_hits} / ${metrics.cpp_misses} / ${cppRate}%`,
      ],
      ["Cache errors", String(metrics.cache_errors)],
    ];
    appendFileSync(
      process.env.GITHUB_STEP_SUMMARY,
      [
        "### sccache",
        "",
        "| Metric | Value |",
        "| --- | --- |",
        ...rows.map(([name, value]) => `| ${name} | ${value} |`),
        "",
      ].join("\n"),
    );
  }

  for (const message of warnings) {
    warning(message);
  }
} catch (error) {
  warning(`Unable to collect statistics: ${String(error.message || error)}`);
}
