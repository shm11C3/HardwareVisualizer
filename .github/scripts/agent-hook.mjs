import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { isGuidancePath } from "./guidance-paths.mjs";

const mode = process.argv[2];
if (mode !== "pre" && mode !== "post" && mode !== "stop") {
  console.error("Usage: agent-hook.mjs <pre|post|stop>");
  process.exit(2);
}

let input = "";
for await (const chunk of process.stdin) {
  input += chunk;
}

let payload;
try {
  payload = JSON.parse(input);
} catch (error) {
  console.error(`Cannot parse agent hook payload: ${error.message}`);
  process.exit(2);
}
if (typeof payload !== "object" || payload === null || Array.isArray(payload)) {
  console.error("Cannot parse agent hook payload: expected an object");
  process.exit(2);
}

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);
const rawPaths = [];
const toolInput = payload.tool_input;
const toolResponse = payload.tool_response;

for (const candidate of [
  toolInput?.file_path,
  toolInput?.filePath,
  toolInput?.path,
  toolResponse?.file_path,
  toolResponse?.filePath,
  toolResponse?.path,
]) {
  if (typeof candidate === "string") {
    rawPaths.push(candidate);
  }
}

const patchInputs = [];
const visitedInputs = new Set();

function collectPatchInputs(candidate) {
  if (typeof candidate === "string") {
    if (candidate.includes("*** Begin Patch")) {
      patchInputs.push(candidate);
    }
    return;
  }

  if (
    typeof candidate !== "object" ||
    candidate === null ||
    visitedInputs.has(candidate)
  ) {
    return;
  }

  visitedInputs.add(candidate);
  for (const value of Object.values(candidate)) {
    collectPatchInputs(value);
  }
}

collectPatchInputs(payload);

for (const patchInput of patchInputs) {
  const patchPathPattern =
    /^\*\*\* (?:(?:Add|Update|Delete) File:|Move to:) (.+)$/gm;
  for (const match of patchInput.matchAll(patchPathPattern)) {
    rawPaths.push(match[1]);
  }
}

function normalizeRepoPath(candidate) {
  const trimmed = candidate.trim().replace(/^['"]|['"]$/g, "");
  if (!trimmed) {
    return null;
  }

  const absolute = path.isAbsolute(trimmed)
    ? path.normalize(trimmed)
    : path.resolve(repoRoot, trimmed);
  const relative = path.relative(repoRoot, absolute);
  if (
    relative === "" ||
    relative === ".." ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  ) {
    return null;
  }

  return relative.split(path.sep).join("/");
}

const paths = new Set(rawPaths.map(normalizeRepoPath).filter(Boolean));

if (mode === "pre" && paths.size === 0) {
  console.error("Cannot determine edited paths from agent hook payload.");
  process.exit(2);
}

if (paths.has("src/rspc/bindings.ts")) {
  console.error(
    "src/rspc/bindings.ts is generated. Edit Rust commands/types and run npm run tauri:dev instead.",
  );
  process.exit(2);
}

if (mode === "pre") {
  process.exit(0);
}

function runValidator(args = []) {
  return spawnSync(
    process.execPath,
    [path.join(repoRoot, ".github/scripts/check-agent-guidance.mjs"), ...args],
    {
      cwd: repoRoot,
      stdio: "inherit",
    },
  );
}

function validatorExitCode(result) {
  if (result.error) {
    console.error(`Failed to run guidance validator: ${result.error.message}`);
  }
  return result.status === 0 ? 0 : 2;
}

if (mode === "stop") {
  process.exit(validatorExitCode(runValidator()));
}

const guidancePaths = [...paths].filter(isGuidancePath);
if (guidancePaths.length === 0) {
  process.exit(0);
}

for (const relativePath of guidancePaths.filter((item) =>
  item.endsWith(".mjs"),
)) {
  const absolutePath = path.join(repoRoot, relativePath);
  if (!existsSync(absolutePath)) {
    continue;
  }
  const syntax = spawnSync(process.execPath, ["--check", absolutePath], {
    cwd: repoRoot,
    stdio: "inherit",
  });
  if (syntax.status !== 0) {
    process.exit(2);
  }
}

process.exit(validatorExitCode(runValidator(["--touched", ...guidancePaths])));
