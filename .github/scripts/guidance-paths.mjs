export const requiredGuidanceFiles = [
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
  ".github/scripts/guidance-paths.mjs",
  ".github/scripts/test-agent-guidance.mjs",
  ".github/scripts/test-agent-hook.mjs",
  ".github/workflows/pr-branch-name.yml",
  "CONTRIBUTING.md",
];

export const cleanRoomReferenceFiles = [
  ".claude/agents/sensor-clean-room-implementer.md",
  ".claude/agents/sensor-spec-author.md",
  ".github/PULL_REQUEST_TEMPLATE/clean-room-sensor-implementation.md",
  ".github/pull_request_template.md",
  "docs/specs/sensors/README.md",
  "docs/development/sensor-handoff/01-spec-gate.md",
  "docs/development/sensor-handoff/07-phase3-nuvoton-spec.md",
  "docs/development/sensor-handoff/08-phase4-ite-spec.md",
];

const exactGuidancePaths = new Set([
  "AGENTS.md",
  "CLAUDE.md",
  "docs/design-principles.md",
  "docs/architecture/backend.md",
  "docs/README.md",
  "docs/documentation-guide.md",
  "docs/specs/sensors/README.md",
  "core/README.md",
  "src-tauri/README.md",
  "src/README.md",
  ".codex/hooks.json",
  ".claude/settings.json",
  ".github/scripts/agent-hook.mjs",
  ".github/scripts/guidance-paths.mjs",
  ".github/scripts/test-agent-guidance.mjs",
  ".github/scripts/test-agent-hook.mjs",
  ".github/scripts/check-agent-guidance.mjs",
  ".github/workflows/agent-guidance.yml",
  ".github/workflows/pr-branch-name.yml",
  ".github/pull_request_template.md",
  "CONTRIBUTING.md",
  "biome.jsonc",
  "package.json",
]);

const guidancePathPrefixes = [
  ".agents/rules/",
  ".agents/skills/",
  ".claude/agents/",
  ".github/PULL_REQUEST_TEMPLATE/",
  "docs/development/sensor-handoff/",
  "docs/agents/",
  "docs/adr/",
];

export function isGuidancePath(relativePath) {
  return (
    relativePath.endsWith("/AGENTS.md") ||
    exactGuidancePaths.has(relativePath) ||
    guidancePathPrefixes.some((prefix) => relativePath.startsWith(prefix))
  );
}
