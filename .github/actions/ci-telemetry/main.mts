import { spawn } from "node:child_process";
import { appendFileSync, mkdirSync } from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { resolveIntervalSeconds } from "./metrics.mts";

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

try {
  const intervalSeconds = resolveIntervalSeconds(
    process.env["INPUT_INTERVAL-SECONDS"],
  );
  const telemetryDir = path.join(
    process.env["RUNNER_TEMP"] || os.tmpdir(),
    "ci-telemetry",
  );
  mkdirSync(telemetryDir, { recursive: true });
  const samplesFile = path.join(telemetryDir, "samples.jsonl");
  // import.meta.dirname (not __dirname: this is ESM), so this keeps working
  // regardless of where the action checkout lands the repo on the runner.
  const samplerPath = path.join(import.meta.dirname, "sampler.mts");

  // Detached + unref'd so this step (and thus the job) does not wait on the
  // sampler; it keeps running in the background until the post step kills
  // it by pid. stdio "ignore" avoids holding open pipes that would also
  // keep the parent step alive. process.execPath re-invokes this same Node
  // binary, which strips sampler.mts's types the same way it stripped this
  // file's.
  const child = spawn(
    process.execPath,
    [samplerPath, samplesFile, String(intervalSeconds * 1000)],
    {
      detached: true,
      stdio: "ignore",
      windowsHide: true,
    },
  );
  child.unref();

  const statePath = process.env["GITHUB_STATE"];
  if (statePath) {
    appendFileSync(statePath, `sampler_pid=${child.pid}\n`);
    appendFileSync(statePath, `samples_file=${samplesFile}\n`);
  }
} catch (error) {
  warn(`Unable to start sampler: ${errorMessage(error)}`);
}
