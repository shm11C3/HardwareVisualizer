import { spawn, spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  githubScriptsDir,
  githubScriptsIndex,
  isGuidancePath,
  listedGithubScripts,
} from "./guidance-paths.mjs";

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

const patchInput =
  typeof toolInput === "string"
    ? toolInput
    : typeof toolInput === "object" && toolInput !== null
      ? (toolInput.patch ?? toolInput.input ?? "")
      : "";

if (typeof patchInput === "string") {
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
  // Paths outside the repository (user settings, scratchpad files) cannot be
  // the generated file this guard protects, so let them through. Only a
  // verified non-empty path counts: an empty or whitespace candidate is not
  // an outside path, and treating it as one would fail open for payload
  // shapes that store the real target elsewhere.
  const hasOutsideRepoPath = rawPaths.some(
    (candidate) =>
      candidate
        .trim()
        .replace(/^['"]|['"]$/g, "")
        .trim().length > 0,
  );
  if (hasOutsideRepoPath) {
    process.exit(0);
  }
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
  // Creating a script under .github/scripts is the placement mistake this
  // guards: scripts belong with the owner of what they operate on, and only
  // GitHub Actions plumbing listed in the index lives here.
  const indexPath = path.join(repoRoot, githubScriptsIndex);
  const listed = existsSync(indexPath)
    ? listedGithubScripts(readFileSync(indexPath, "utf8"))
    : new Set();
  for (const relativePath of paths) {
    if (
      !relativePath.startsWith(`${githubScriptsDir}/`) ||
      relativePath === githubScriptsIndex ||
      existsSync(path.join(repoRoot, relativePath))
    ) {
      continue;
    }
    const scriptPath = relativePath.slice(githubScriptsDir.length + 1);
    if (!listed.has(scriptPath)) {
      console.error(
        `${relativePath} is not listed in ${githubScriptsIndex}. Put the script next to the owner of what it checks or produces (or under scripts/<area>/). Only GitHub Actions plumbing belongs in ${githubScriptsDir}; if this is that, add its index row first.`,
      );
      process.exit(2);
    }
  }
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
  // Reclaim the shared Cargo build subtrees of worktrees that no longer
  // exist (see docs/development/local-build-cache.md). Detached, so a
  // multi-gigabyte deletion never runs into this hook's timeout; the script
  // itself is safe to interrupt and to run concurrently with itself.
  spawn(
    process.execPath,
    [path.join(repoRoot, ".github/scripts/prune-build-dirs.mjs"), "--quiet"],
    { cwd: repoRoot, detached: true, stdio: "ignore" },
  ).unref();
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
