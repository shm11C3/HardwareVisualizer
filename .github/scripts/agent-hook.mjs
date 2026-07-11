import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

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

function isGuidancePath(relativePath) {
  return (
    relativePath === "AGENTS.md" ||
    relativePath.endsWith("/AGENTS.md") ||
    relativePath === "CLAUDE.md" ||
    relativePath.startsWith(".agents/rules/") ||
    relativePath.startsWith(".agents/skills/") ||
    relativePath.startsWith(".claude/agents/") ||
    relativePath.startsWith(".github/PULL_REQUEST_TEMPLATE/") ||
    relativePath === "docs/design-principles.md" ||
    relativePath === "docs/architecture/backend.md" ||
    relativePath === "docs/README.md" ||
    relativePath === "docs/documentation-guide.md" ||
    relativePath === "docs/specs/sensors/README.md" ||
    relativePath.startsWith("docs/development/sensor-handoff/") ||
    relativePath === "core/README.md" ||
    relativePath === "src-tauri/README.md" ||
    relativePath === "src/README.md" ||
    relativePath.startsWith("docs/agents/") ||
    relativePath.startsWith("docs/adr/") ||
    relativePath === ".codex/hooks.json" ||
    relativePath === ".claude/settings.json" ||
    relativePath === ".github/scripts/agent-hook.mjs" ||
    relativePath === ".github/scripts/test-agent-guidance.mjs" ||
    relativePath === ".github/scripts/test-agent-hook.mjs" ||
    relativePath === ".github/scripts/check-agent-guidance.mjs" ||
    relativePath === ".github/workflows/agent-guidance.yml" ||
    relativePath === ".github/workflows/pr-branch-name.yml" ||
    relativePath === ".github/pull_request_template.md" ||
    relativePath === "CONTRIBUTING.md" ||
    relativePath === "biome.jsonc" ||
    relativePath === "package.json"
  );
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

if (mode === "stop") {
  const result = runValidator();
  process.exit(result.status === 0 ? 0 : 2);
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

const result = runValidator(["--touched", ...guidancePaths]);
process.exit(result.status === 0 ? 0 : 2);
