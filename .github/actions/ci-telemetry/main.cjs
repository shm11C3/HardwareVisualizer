const { spawn } = require("node:child_process");
const { mkdirSync, appendFileSync } = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { resolveIntervalSeconds } = require("./metrics.cjs");

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

try {
  const intervalSeconds = resolveIntervalSeconds(
    process.env["INPUT_INTERVAL-SECONDS"],
  );
  const telemetryDir = path.join(
    process.env.RUNNER_TEMP || os.tmpdir(),
    "ci-telemetry",
  );
  mkdirSync(telemetryDir, { recursive: true });
  const samplesFile = path.join(telemetryDir, "samples.jsonl");
  const samplerPath = path.join(__dirname, "sampler.cjs");

  // Detached + unref'd so this step (and thus the job) does not wait on the
  // sampler; it keeps running in the background until the post step kills
  // it by pid. stdio "ignore" avoids holding open pipes that would also
  // keep the parent step alive.
  const child = spawn(
    process.execPath,
    [samplerPath, samplesFile, String(intervalSeconds * 1000)],
    { detached: true, stdio: "ignore", windowsHide: true },
  );
  child.unref();

  const statePath = process.env.GITHUB_STATE;
  if (statePath) {
    appendFileSync(statePath, `sampler_pid=${child.pid}\n`);
    appendFileSync(statePath, `samples_file=${samplesFile}\n`);
  }
} catch (error) {
  warn(`Unable to start sampler: ${String(error.message || error)}`);
}
