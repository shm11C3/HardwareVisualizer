const { execFileSync } = require("node:child_process");

const sccache = process.env.SCCACHE_PATH || "sccache";

try {
  execFileSync(sccache, ["--zero-stats"], { stdio: "inherit" });
} catch (error) {
  console.log(
    `::warning title=sccache metrics::Unable to reset statistics: ${String(error.message || error)}`,
  );
}
