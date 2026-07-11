import { access, readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);
const errors = [];
let contentOverrides = {};
if (process.env.AGENT_GUIDANCE_OVERRIDES) {
  try {
    contentOverrides = JSON.parse(process.env.AGENT_GUIDANCE_OVERRIDES);
  } catch (error) {
    errors.push(`Invalid AGENT_GUIDANCE_OVERRIDES: ${error.message}`);
  }
}

const requiredFiles = [
  "AGENTS.md",
  "core/AGENTS.md",
  "src-tauri/AGENTS.md",
  "src/AGENTS.md",
  "docs/AGENTS.md",
  "docs/design-principles.md",
  "docs/agents/lessons/README.md",
  ".agents/skills/change-kind-naming/SKILL.md",
  ".agents/skills/hardwarevisualizer-design-review/SKILL.md",
  ".agents/skills/hardwarevisualizer-design-review/agents/openai.yaml",
  ".agents/skills/capture-project-learning/SKILL.md",
  ".agents/skills/capture-project-learning/agents/openai.yaml",
  ".agents/rules/README.md",
  ".agents/rules/clean-room-sensors.md",
  ".agents/rules/design.md",
  ".agents/rules/documentation.md",
  ".agents/rules/frontend.md",
  ".agents/rules/rust.md",
  ".agents/rules/settings.md",
  ".github/scripts/agent-hook.mjs",
  ".github/scripts/test-agent-guidance.mjs",
  ".github/scripts/test-agent-hook.mjs",
  ".github/workflows/pr-branch-name.yml",
  "CONTRIBUTING.md",
];
const cleanRoomReferenceFiles = [
  ".claude/agents/sensor-clean-room-implementer.md",
  ".claude/agents/sensor-spec-author.md",
  ".github/PULL_REQUEST_TEMPLATE/clean-room-sensor-implementation.md",
  ".github/pull_request_template.md",
  "docs/specs/sensors/README.md",
  "docs/development/sensor-handoff/01-spec-gate.md",
  "docs/development/sensor-handoff/07-phase3-nuvoton-spec.md",
  "docs/development/sensor-handoff/08-phase4-ite-spec.md",
];

const lessonFields = new Set([
  "id",
  "status",
  "cause_status",
  "scope",
  "trigger",
  "failure_signature",
  "root_cause",
  "guardrail",
  "canonical_refs",
  "verification",
  "evidence",
  "revalidate_when",
  "superseded_by",
]);
const requiredLessonFields = [...lessonFields].filter(
  (field) => field !== "superseded_by",
);
const skillFields = new Set(["name", "description"]);
const ruleFields = new Set(["scope"]);
const touchedMode = process.argv[2] === "--touched";
const touchedPaths = touchedMode ? process.argv.slice(3) : [];

function fail(message) {
  errors.push(message);
}

async function exists(relativePath) {
  if (Object.hasOwn(contentOverrides, relativePath)) {
    return true;
  }
  try {
    await access(path.join(root, relativePath));
    return true;
  } catch {
    return false;
  }
}

async function read(relativePath) {
  if (Object.hasOwn(contentOverrides, relativePath)) {
    return contentOverrides[relativePath];
  }
  return readFile(path.join(root, relativePath), "utf8");
}

function parseScalar(rawValue, relativePath, key) {
  const value = rawValue.trim();
  if (!value) {
    return "";
  }

  if (value.startsWith('"')) {
    try {
      const parsed = JSON.parse(value);
      if (typeof parsed !== "string") {
        fail(`${relativePath} frontmatter ${key} must be a string`);
        return "";
      }
      return parsed;
    } catch {
      fail(`${relativePath} has an invalid quoted value for ${key}`);
      return "";
    }
  }

  if (value.startsWith("'")) {
    if (!value.endsWith("'")) {
      fail(`${relativePath} has an unmatched quote for ${key}`);
      return "";
    }
    return value.slice(1, -1).replaceAll("''", "'");
  }

  if (/\s#/.test(value)) {
    fail(`${relativePath} must quote the frontmatter value for ${key}`);
  }
  if (value.endsWith('"') || value.endsWith("'")) {
    fail(`${relativePath} has an unmatched trailing quote for ${key}`);
  }
  if (/^[|>]/.test(value)) {
    fail(`${relativePath} does not allow multiline frontmatter field ${key}`);
  }
  return value;
}

function commaSeparatedPaths(value) {
  return value
    .split(",")
    .map((item) => item.trim().replace(/^and\s+/, ""))
    .filter(Boolean);
}

function frontmatter(content, relativePath, allowedFields) {
  const normalized = content.replaceAll("\r\n", "\n");
  const match = normalized.match(/^---\n([\s\S]*?)\n---(?:\n|$)/);
  if (!match) {
    fail(`${relativePath} is missing YAML frontmatter`);
    return null;
  }

  const fields = new Map();
  for (const [index, line] of match[1].split("\n").entries()) {
    if (!line.trim()) {
      continue;
    }
    if (/^\s/.test(line)) {
      fail(
        `${relativePath} frontmatter line ${index + 2} must be a single-line scalar`,
      );
      continue;
    }

    const field = line.match(/^([a-zA-Z_][a-zA-Z0-9_-]*):\s*(.*)$/);
    if (!field) {
      fail(`${relativePath} has invalid frontmatter line: ${line}`);
      continue;
    }

    const [, key, rawValue] = field;
    if (fields.has(key)) {
      fail(`${relativePath} duplicates frontmatter field: ${key}`);
      continue;
    }
    if (!allowedFields.has(key)) {
      fail(`${relativePath} has unknown frontmatter field: ${key}`);
      continue;
    }
    fields.set(key, parseScalar(rawValue, relativePath, key));
  }
  return fields;
}

function validateLessonShape(fields, relativePath) {
  for (const field of requiredLessonFields) {
    if (!fields.get(field)) {
      fail(`${relativePath} is missing required field: ${field}`);
    }
  }

  const id = fields.get("id");
  const status = fields.get("status");
  const causeStatus = fields.get("cause_status");
  const supersededBy = fields.get("superseded_by") ?? "";
  if (id && !/^LRN-\d{8}-[a-z0-9-]+$/.test(id)) {
    fail(`${relativePath} has invalid lesson id: ${id}`);
  }
  if (status && !new Set(["candidate", "promoted", "superseded"]).has(status)) {
    fail(`${relativePath} has invalid status: ${status}`);
  }
  if (causeStatus && !new Set(["hypothesis", "confirmed"]).has(causeStatus)) {
    fail(`${relativePath} has invalid cause_status: ${causeStatus}`);
  }
  if (status === "promoted") {
    if (causeStatus !== "confirmed") {
      fail(`${relativePath} must confirm its cause before promotion`);
    }
    if ((fields.get("canonical_refs") ?? "").startsWith("pending")) {
      fail(`${relativePath} is promoted but canonical_refs is pending`);
    }
  }
  if (status === "superseded") {
    if (!supersededBy) {
      fail(`${relativePath} is superseded but has no superseded_by`);
    } else if (!/^LRN-\d{8}-[a-z0-9-]+$/.test(supersededBy)) {
      fail(`${relativePath} has invalid superseded_by ID`);
    }
  }

  return {
    relativePath,
    id,
    status,
    canonicalRefs: fields.get("canonical_refs") ?? "",
    supersededBy,
  };
}

async function checkRequiredFiles() {
  for (const relativePath of [...requiredFiles, ...cleanRoomReferenceFiles]) {
    if (!(await exists(relativePath))) {
      fail(`Missing required guidance file: ${relativePath}`);
    }
  }
}

async function checkRules() {
  const directory = ".agents/rules";
  const files = (await readdir(path.join(root, directory)))
    .filter((file) => file.endsWith(".md") && file !== "README.md")
    .sort();
  const parsedByFile = new Map();

  for (const file of files) {
    const relativePath = path.join(directory, file);
    const fields = frontmatter(
      await read(relativePath),
      relativePath,
      ruleFields,
    );
    parsedByFile.set(file, fields);
    if (fields && !fields.get("scope")) {
      fail(`${relativePath} is missing a non-empty scope field`);
    }
  }

  const cleanRoomScope =
    parsedByFile.get("clean-room-sensors.md")?.get("scope") ?? "";
  for (const requiredPath of [
    "core/src/infrastructure/providers/windows/pawn_io.rs",
    "core/src/infrastructure/providers/windows/cpu_temperature.rs",
    "core/src/infrastructure/providers/windows/cpu_temperature_decode.rs",
    "core/src/infrastructure/providers/windows/super_io*.rs",
    "core/src/platform/windows/motherboard.rs",
    "core/src/platform/windows/sensors.rs",
    "core/src/utils/super_io.rs",
  ]) {
    if (!cleanRoomScope.split(",").includes(requiredPath)) {
      fail(`clean-room scope is missing: ${requiredPath}`);
    }
  }

  return files;
}

function checkOpenAiYaml(content, relativePath) {
  const normalized = content.replaceAll("\r\n", "\n");
  const lines = normalized.split("\n").filter((line) => line.trim());
  if (lines[0] !== "interface:") {
    fail(`${relativePath} must start with interface:`);
    return;
  }

  const required = new Set([
    "display_name",
    "short_description",
    "default_prompt",
  ]);
  const allowed = new Set([
    ...required,
    "icon_small",
    "icon_large",
    "brand_color",
  ]);
  const fields = new Map();
  for (const line of lines.slice(1)) {
    const match = line.match(/^ {2}([a-z_]+):\s*(.*)$/);
    if (!match || !allowed.has(match[1])) {
      fail(`${relativePath} has invalid interface line: ${line}`);
      continue;
    }
    if (fields.has(match[1])) {
      fail(`${relativePath} duplicates interface field: ${match[1]}`);
      continue;
    }
    fields.set(match[1], parseScalar(match[2], relativePath, match[1]));
  }
  for (const field of required) {
    if (!fields.get(field)) {
      fail(`${relativePath} is missing interface field: ${field}`);
    }
  }
}

async function checkSkills() {
  const directory = ".agents/skills";
  const entries = await readdir(path.join(root, directory), {
    withFileTypes: true,
  });
  const skillFiles = [];
  const metadataFiles = [];

  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }
    const relativePath = path.join(directory, entry.name, "SKILL.md");
    if (!(await exists(relativePath))) {
      fail(`${path.join(directory, entry.name)} is missing SKILL.md`);
      continue;
    }

    skillFiles.push(relativePath);
    const fields = frontmatter(
      await read(relativePath),
      relativePath,
      skillFields,
    );
    if (!fields) {
      continue;
    }
    if (fields.get("name") !== entry.name) {
      fail(`${relativePath} name must match its directory (${entry.name})`);
    }
    if (!/^[a-z0-9-]{1,64}$/.test(fields.get("name") ?? "")) {
      fail(`${relativePath} has an invalid skill name`);
    }
    if (!fields.get("description")) {
      fail(`${relativePath} is missing a description`);
    }

    const metadataPath = path.join(directory, entry.name, "agents/openai.yaml");
    if (await exists(metadataPath)) {
      metadataFiles.push(metadataPath);
      checkOpenAiYaml(await read(metadataPath), metadataPath);
    }
  }

  return { skillFiles, metadataFiles };
}

async function checkLessons() {
  const directory = "docs/agents/lessons";
  const files = (await readdir(path.join(root, directory)))
    .filter((file) => file.endsWith(".md") && file !== "README.md")
    .sort();
  const index = await read(path.join(directory, "README.md"));
  const ids = new Set();
  const records = [];

  for (const file of files) {
    const relativePath = path.join(directory, file);
    const fields = frontmatter(
      await read(relativePath),
      relativePath,
      lessonFields,
    );
    if (!fields) {
      continue;
    }

    const record = validateLessonShape(fields, relativePath);
    const { id } = record;
    if (id && ids.has(id)) {
      fail(`${relativePath} duplicates lesson id: ${id}`);
    } else if (id) {
      ids.add(id);
    }
    records.push(record);

    if (
      (
        index.match(new RegExp(`\\(${file.replaceAll(".", "\\.")}\\)`, "g")) ??
        []
      ).length !== 1
    ) {
      fail(`${relativePath} must appear exactly once in the lessons index`);
    }
  }

  for (const record of records) {
    if (record.status === "promoted") {
      for (const reference of commaSeparatedPaths(record.canonicalRefs)) {
        const normalized = path.normalize(reference);
        if (
          path.isAbsolute(reference) ||
          normalized === ".." ||
          normalized.startsWith(`..${path.sep}`) ||
          !(await exists(normalized))
        ) {
          fail(
            `${record.relativePath} canonical_refs path does not exist: ${reference}`,
          );
        }
      }
    }

    if (record.status === "superseded") {
      if (record.supersededBy === record.id) {
        fail(`${record.relativePath} cannot supersede itself`);
      } else if (!ids.has(record.supersededBy)) {
        fail(
          `${record.relativePath} superseded_by does not identify an existing lesson: ${record.supersededBy}`,
        );
      }
    }
  }

  return files;
}

async function checkAdrs() {
  const directory = "docs/adr";
  const files = (await readdir(path.join(root, directory)))
    .filter((file) => /^\d{4}-.*\.md$/.test(file))
    .sort();
  const index = await read(path.join(directory, "README.md"));
  const allowed = new Set(["proposed", "accepted", "superseded"]);

  for (const file of files) {
    const content = await read(path.join(directory, file));
    const status = content.match(/^Status: ([a-z]+)$/m)?.[1];
    if (!status || !allowed.has(status)) {
      fail(`${path.join(directory, file)} has missing or invalid ADR status`);
    }
    if (!index.includes(`(${file})`)) {
      fail(`${path.join(directory, file)} is missing from docs/adr/README.md`);
    }
  }

  return files;
}

function setDifference(left, right) {
  return [...left].filter((value) => !right.has(value));
}

async function checkBranchPolicy() {
  const contributing = await read("CONTRIBUTING.md");
  const workflow = await read(".github/workflows/pr-branch-name.yml");
  const changeKind = await read(".agents/skills/change-kind-naming/SKILL.md");
  const rootAgents = await read("AGENTS.md");

  const documented = new Set(
    [...contributing.matchAll(/`([a-z][a-z0-9-]*)\/(?:<[^`]+>|\.\.\.)`/g)].map(
      (match) => match[1],
    ),
  );
  const caseMatch = workflow.match(
    /^\s*((?:[a-z0-9-]+\/\*\|)*[a-z0-9-]+\/\*)\)\s*$/m,
  );
  const enforced = new Set(
    (caseMatch?.[1] ?? "")
      .split("|")
      .filter(Boolean)
      .map((item) => item.slice(0, -2)),
  );
  const allowedSkillBlock = changeKind
    .split("Use branch prefixes that match the final classification:")[1]
    ?.split("This repository's CI")[0];
  const skill = new Set(
    [...(allowedSkillBlock ?? "").matchAll(/^- `([a-z0-9-]+)\/\.\.\.`$/gm)].map(
      (match) => match[1],
    ),
  );

  if (documented.size === 0 || enforced.size === 0 || skill.size === 0) {
    fail("Could not parse branch policy from CONTRIBUTING, CI, and skill");
    return;
  }

  const missingFromCi = setDifference(documented, enforced);
  const automatedPrefixes = new Set(["dependabot", "renovate"]);
  const extraInCi = setDifference(enforced, documented).filter(
    (prefix) => !automatedPrefixes.has(prefix),
  );
  const missingFromSkill = setDifference(documented, skill);
  const extraInSkill = setDifference(skill, documented);
  if (missingFromCi.length > 0) {
    fail(
      `CONTRIBUTING prefixes missing from branch CI: ${missingFromCi.join(", ")}`,
    );
  }
  if (extraInCi.length > 0) {
    fail(
      `Branch CI has undocumented project prefixes: ${extraInCi.join(", ")}`,
    );
  }
  if (missingFromSkill.length > 0 || extraInSkill.length > 0) {
    fail(
      `change-kind skill differs from CONTRIBUTING (missing: ${missingFromSkill.join(", ") || "none"}; extra: ${extraInSkill.join(", ") || "none"})`,
    );
  }

  for (const [name, content] of [["AGENTS.md", rootAgents]]) {
    const policyBlock = content.match(
      /(?:Allowed project branch prefixes are|Branches must use)([\s\S]*?)Never use/,
    )?.[1];
    const copied = new Set(
      [...(policyBlock ?? "").matchAll(/`([a-z][a-z0-9-]*)\/`/g)].map(
        (match) => match[1],
      ),
    );
    const missing = setDifference(documented, copied);
    const extra = setDifference(copied, documented);
    if (!policyBlock || missing.length > 0 || extra.length > 0) {
      fail(
        `${name} branch prefixes differ from CONTRIBUTING (missing: ${missing.join(", ") || "none"}; extra: ${extra.join(", ") || "none"})`,
      );
    }

    const mapsTestAndCiToChore =
      content.includes("`test:`") &&
      content.includes("`ci:`") &&
      content.includes("`chore/` branch");
    if (!mapsTestAndCiToChore) {
      fail(`${name} must map test and CI changes to a chore/ branch`);
    }
  }
}

async function checkKnownDriftPoints() {
  const rootAgents = await read("AGENTS.md");
  const rustRule = await read(".agents/rules/rust.md");
  const cleanRoom = await read(".agents/rules/clean-room-sensors.md");
  const backend = await read("docs/architecture/backend.md");
  const coreReadme = await read("core/README.md");
  const appAgents = await read("src-tauri/AGENTS.md");
  const appReadme = await read("src-tauri/README.md");
  const frontendReadme = await read("src/README.md");
  const settings = await read(".agents/rules/settings.md");
  const prTemplate = await read(".github/pull_request_template.md");
  const packageJson = JSON.parse(await read("package.json"));

  for (const stalePath of [
    "src-tauri/src/platform/",
    "crate::platform::PlatformFactory",
  ]) {
    if (rootAgents.includes(stalePath)) {
      fail(`Always-on guidance contains stale architecture path: ${stalePath}`);
    }
  }
  for (const staleStatement of [
    "moves to Core in a future phase",
    "App registers Tauri SQL migrations",
    "App owns Tauri SQL plugin migrations",
  ]) {
    if (
      rootAgents.includes(staleStatement) ||
      rustRule.includes(staleStatement) ||
      backend.includes(staleStatement)
    ) {
      fail(`Guidance contains stale persistence statement: ${staleStatement}`);
    }
  }
  for (const [name, content] of [
    ["AGENTS.md", rootAgents],
    [".agents/rules/rust.md", rustRule],
    ["docs/architecture/backend.md", backend],
    ["core/README.md", coreReadme],
    ["src-tauri/AGENTS.md", appAgents],
    ["src-tauri/README.md", appReadme],
  ]) {
    if (/Tauri SQL (?:plugin|migration)/.test(content)) {
      fail(`${name} contains stale Tauri SQL migration terminology`);
    }
  }
  for (const [name, content] of [
    ["AGENTS.md", rootAgents],
    ["docs/architecture/backend.md", backend],
    ["src-tauri/AGENTS.md", appAgents],
    ["src-tauri/README.md", appReadme],
    ["src/README.md", frontendReadme],
    ["CONTRIBUTING.md", await read("CONTRIBUTING.md")],
  ]) {
    if (content.includes("npm run tauri dev")) {
      fail(`${name} uses npm run tauri dev instead of npm run tauri:dev`);
    }
  }
  if (!coreReadme.includes("storage_health.rs    Storage Health settings")) {
    fail(
      "core/README.md does not point to the current storage settings module",
    );
  }

  for (const requiredReference of [
    "docs/design-principles.md",
    ".agents/rules/README.md",
    "hardwarevisualizer-design-review",
    "capture-project-learning",
    "docs/agents/lessons/",
  ]) {
    if (!rootAgents.includes(requiredReference)) {
      fail(`AGENTS.md is missing required reference: ${requiredReference}`);
    }
  }
  if (rootAgents.includes(".github/instructions/")) {
    fail(
      "AGENTS.md still refers to the retired .github/instructions directory",
    );
  }
  if (await exists(".github/copilot-instructions.md")) {
    fail("Retired Copilot instructions file still exists");
  }
  for (const relativePath of cleanRoomReferenceFiles) {
    const content = await read(relativePath);
    if (content.includes(".github/instructions/")) {
      fail(
        `${relativePath} still refers to the retired .github/instructions directory`,
      );
    }
    if (!content.includes(".agents/rules/clean-room-sensors.md")) {
      fail(`${relativePath} is missing the shared clean-room rule reference`);
    }
  }

  if (cleanRoom.includes("whose status is **not**")) {
    fail("Clean-room rule weakens the exact implementation-ready status gate");
  }
  for (const requiredText of [
    "Implementation-ready (rev N)",
    "Sensor access is read-only",
    "fan control",
    "power state",
  ]) {
    if (!cleanRoom.includes(requiredText)) {
      fail(`Clean-room rule is missing safety/readiness text: ${requiredText}`);
    }
  }

  if (!settings.includes("legacy Tauri Store exception")) {
    fail("Settings rule is missing the showGpuUsageSource legacy exception");
  }
  if (!prTemplate.includes("Performance (`perf/` branch)")) {
    fail("PR template is missing the perf/ change type");
  }
  for (const [script, command] of Object.entries({
    "check:agent-guidance": "node .github/scripts/check-agent-guidance.mjs",
    "test:agent-guidance": "node .github/scripts/test-agent-guidance.mjs",
    "test:agent-hooks": "node .github/scripts/test-agent-hook.mjs",
  })) {
    if (packageJson.scripts?.[script] !== command) {
      fail(`package.json ${script} script points to the wrong command`);
    }
  }
}

function commandHooks(parsed) {
  return Object.values(parsed.hooks ?? {})
    .flat()
    .flatMap((group) => group.hooks ?? [])
    .filter((hook) => hook.type === "command")
    .map((hook) => hook.command);
}

async function checkHooks() {
  const script =
    'node "$(git rev-parse --show-toplevel)/.github/scripts/agent-hook.mjs"';
  const expected = [`${script} pre`, `${script} post`, `${script} stop`];

  for (const relativePath of [".codex/hooks.json", ".claude/settings.json"]) {
    let parsed;
    try {
      parsed = JSON.parse(await read(relativePath));
    } catch (error) {
      fail(`${relativePath} is invalid JSON: ${error.message}`);
      continue;
    }

    const commands = commandHooks(parsed);
    if (
      commands.length !== expected.length ||
      expected.some((command) => !commands.includes(command))
    ) {
      fail(
        `${relativePath} must contain only the shared pre/post/stop hook commands`,
      );
    }
  }
}

async function checkLocalMarkdownLinks(relativePaths) {
  const linkPattern = /\[[^\]]*\]\(([^)]+)\)/g;

  for (const relativePath of relativePaths) {
    const content = await read(relativePath);
    for (const match of content.matchAll(linkPattern)) {
      const rawTarget = match[1].trim().replace(/^<|>$/g, "");
      if (rawTarget.startsWith("#") || /^(?:https?:|mailto:)/.test(rawTarget)) {
        continue;
      }

      const targetWithoutAnchor = rawTarget.split("#", 1)[0];
      if (!targetWithoutAnchor) {
        continue;
      }
      const resolved = path.normalize(
        path.join(path.dirname(relativePath), targetWithoutAnchor),
      );
      if (!(await exists(resolved))) {
        fail(`${relativePath} has broken local link: ${rawTarget}`);
      }
    }
  }
}

async function checkForSensitiveContent(relativePaths) {
  const patterns = [
    {
      name: "GitHub token",
      pattern: /(?:gh[pousr]|github_pat)_[A-Za-z0-9_]{20,}/,
    },
    { name: "personal absolute path", pattern: /\/Users\/[^/\s]+\// },
  ];

  for (const relativePath of relativePaths) {
    const content = await read(relativePath);
    for (const { name, pattern } of patterns) {
      if (pattern.test(content)) {
        fail(`${relativePath} contains a ${name}`);
      }
    }
  }
}

async function checkTouchedFiles(relativePaths) {
  if (relativePaths.length === 0) {
    fail("--touched requires at least one repository-relative path");
    return;
  }

  const existingPaths = [];
  for (const candidate of new Set(relativePaths)) {
    const normalized = path.normalize(candidate);
    if (
      path.isAbsolute(candidate) ||
      normalized === ".." ||
      normalized.startsWith(`..${path.sep}`)
    ) {
      fail(`Touched guidance path is outside the repository: ${candidate}`);
      continue;
    }
    if (!(await exists(normalized))) {
      continue;
    }

    existingPaths.push(normalized);
    const content = await read(normalized);
    if (
      /^\.agents\/rules\/[^/]+\.md$/.test(normalized) &&
      normalized !== ".agents/rules/README.md"
    ) {
      const fields = frontmatter(content, normalized, ruleFields);
      if (fields && !fields.get("scope")) {
        fail(`${normalized} is missing a non-empty scope field`);
      }
      continue;
    }

    const skillMatch = normalized.match(
      /^\.agents\/skills\/([^/]+)\/SKILL\.md$/,
    );
    if (skillMatch) {
      const fields = frontmatter(content, normalized, skillFields);
      if (fields?.get("name") !== skillMatch[1]) {
        fail(`${normalized} name must match its directory (${skillMatch[1]})`);
      }
      if (!fields?.get("description")) {
        fail(`${normalized} is missing a description`);
      }
      continue;
    }

    if (/^\.agents\/skills\/[^/]+\/agents\/openai\.yaml$/.test(normalized)) {
      checkOpenAiYaml(content, normalized);
      continue;
    }

    if (
      /^docs\/agents\/lessons\/[^/]+\.md$/.test(normalized) &&
      normalized !== "docs/agents/lessons/README.md"
    ) {
      const fields = frontmatter(content, normalized, lessonFields);
      if (fields) {
        validateLessonShape(fields, normalized);
      }
      continue;
    }

    if (/^docs\/adr\/\d{4}-.*\.md$/.test(normalized)) {
      const status = content.match(/^Status: ([a-z]+)$/m)?.[1];
      if (
        !status ||
        !new Set(["proposed", "accepted", "superseded"]).has(status)
      ) {
        fail(`${normalized} has missing or invalid ADR status`);
      }
      continue;
    }

    if (
      normalized === ".codex/hooks.json" ||
      normalized === ".claude/settings.json" ||
      normalized === "package.json"
    ) {
      try {
        JSON.parse(content);
      } catch (error) {
        fail(`${normalized} is invalid JSON: ${error.message}`);
      }
    }
  }

  await checkForSensitiveContent(existingPaths);
}

function reportErrors() {
  if (errors.length === 0) {
    return false;
  }
  console.error("Agent guidance validation failed:");
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exitCode = 1;
  return true;
}

async function main() {
  await checkRequiredFiles();
  const ruleFiles = await checkRules();
  const { skillFiles, metadataFiles } = await checkSkills();
  const lessonFiles = await checkLessons();
  const adrFiles = await checkAdrs();
  await checkBranchPolicy();
  await checkKnownDriftPoints();
  await checkHooks();

  const guidanceFiles = [
    "AGENTS.md",
    "core/AGENTS.md",
    "src-tauri/AGENTS.md",
    "src/AGENTS.md",
    "docs/AGENTS.md",
    "CLAUDE.md",
    "docs/design-principles.md",
    "docs/README.md",
    "docs/documentation-guide.md",
    ".agents/rules/README.md",
    ...cleanRoomReferenceFiles,
    "docs/agents/lessons/README.md",
    ...lessonFiles.map((file) => path.join("docs/agents/lessons", file)),
    ...adrFiles.map((file) => path.join("docs/adr", file)),
    ...ruleFiles.map((file) => path.join(".agents/rules", file)),
    ...skillFiles,
    ...metadataFiles,
  ];

  await checkLocalMarkdownLinks(guidanceFiles);
  await checkForSensitiveContent(guidanceFiles);

  if (reportErrors()) {
    return;
  }

  console.log(
    `Agent guidance validation passed (${lessonFiles.length} lessons, ${skillFiles.length} skills, ${ruleFiles.length} rules, ${adrFiles.length} ADRs).`,
  );
}

if (touchedMode) {
  await checkTouchedFiles(touchedPaths);
  if (!reportErrors()) {
    console.log(
      `Touched agent guidance validation passed (${touchedPaths.length} paths).`,
    );
  }
} else {
  await main();
}
