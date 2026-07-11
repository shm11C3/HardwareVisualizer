import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const root = process.cwd();
const checker = ".github/scripts/check-agent-guidance.mjs";

function read(relativePath) {
  return readFileSync(relativePath, "utf8");
}

function gitStatus() {
  const result = spawnSync("git", ["status", "--porcelain=v1", "-uall"], {
    cwd: root,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(result.stderr || "git status failed");
  }
  return result.stdout;
}

function expectFailure(name, overrides, expectedText) {
  const result = spawnSync(process.execPath, [checker], {
    cwd: root,
    env: {
      ...process.env,
      AGENT_GUIDANCE_OVERRIDES: JSON.stringify(overrides),
    },
    encoding: "utf8",
  });
  const output = `${result.stdout}${result.stderr}`;
  if (result.status !== 1 || !output.includes(expectedText)) {
    throw new Error(
      `${name}: expected validation failure containing ${JSON.stringify(expectedText)}\n${output}`,
    );
  }
}

const before = gitStatus();

expectFailure(
  "missing rule scope",
  {
    ".agents/rules/design.md": read(".agents/rules/design.md").replace(
      'scope: "**"',
      'scope: ""',
    ),
  },
  "missing a non-empty scope field",
);

const duplicateId = read(
  "docs/agents/lessons/evidence-before-conclusions.md",
).match(/^id: (.+)$/m)?.[1];
expectFailure(
  "duplicate lesson id",
  {
    "docs/agents/lessons/verify-user-visible-results.md": read(
      "docs/agents/lessons/verify-user-visible-results.md",
    ).replace(/^id: .+$/m, `id: ${duplicateId}`),
  },
  "duplicates lesson id",
);

expectFailure(
  "missing canonical reference",
  {
    "docs/agents/lessons/evidence-before-conclusions.md": read(
      "docs/agents/lessons/evidence-before-conclusions.md",
    ).replace(
      /^canonical_refs: .+$/m,
      "canonical_refs: docs/missing-principles.md",
    ),
  },
  "canonical_refs path does not exist: docs/missing-principles.md",
);

expectFailure(
  "missing superseding lesson",
  {
    "docs/agents/lessons/evidence-before-conclusions.md": read(
      "docs/agents/lessons/evidence-before-conclusions.md",
    )
      .replace("status: promoted", "status: superseded")
      .replace(
        /^(revalidate_when: .+)$/m,
        "$1\nsuperseded_by: LRN-20990101-missing-replacement",
      ),
  },
  "superseded_by does not identify an existing lesson: LRN-20990101-missing-replacement",
);

expectFailure(
  "branch policy drift",
  {
    "CONTRIBUTING.md": read("CONTRIBUTING.md").replace(
      "- Other: `chore/<short-description-or-issue-number>`",
      "- CI: `ci/<short-description-or-issue-number>`\n- Other: `chore/<short-description-or-issue-number>`",
    ),
  },
  "CONTRIBUTING prefixes missing from branch CI: ci",
);

expectFailure(
  "undocumented branch CI prefix",
  {
    ".github/workflows/pr-branch-name.yml": read(
      ".github/workflows/pr-branch-name.yml",
    ).replace("chore/*|dependabot/*", "chore/*|ci/*|dependabot/*"),
  },
  "Branch CI has undocumented project prefixes: ci",
);

expectFailure(
  "always-on branch policy drift",
  {
    "AGENTS.md": read("AGENTS.md").replace("`perf/`, ", ""),
  },
  "AGENTS.md branch prefixes differ from CONTRIBUTING (missing: perf",
);

expectFailure(
  "clean-room provider scope drift",
  {
    ".agents/rules/clean-room-sensors.md": read(
      ".agents/rules/clean-room-sensors.md",
    ).replace(",core/src/infrastructure/providers/windows/super_io*.rs", ""),
  },
  "clean-room scope is missing: core/src/infrastructure/providers/windows/super_io*.rs",
);

expectFailure(
  "retired clean-room reference",
  {
    ".claude/agents/sensor-clean-room-implementer.md": read(
      ".claude/agents/sensor-clean-room-implementer.md",
    ).replace(
      ".agents/rules/clean-room-sensors.md",
      ".github/instructions/clean-room-sensors.instructions.md",
    ),
  },
  "still refers to the retired .github/instructions directory",
);

expectFailure(
  "retired Copilot instructions file",
  {
    ".github/copilot-instructions.md": "# Retired Copilot Instructions\n",
  },
  "Retired Copilot instructions file still exists",
);

expectFailure(
  "broken local link",
  {
    "AGENTS.md": `${read("AGENTS.md")}\n[Broken fixture](missing-agent-guidance-file.md)\n`,
  },
  "has broken local link: missing-agent-guidance-file.md",
);

expectFailure(
  "stale persistence guidance",
  {
    ".agents/rules/rust.md": `${read(
      ".agents/rules/rust.md",
    )}\nApp persistence moves to Core in a future phase.\n`,
  },
  "Guidance contains stale persistence statement",
);

expectFailure(
  "stale migration terminology",
  {
    "core/README.md": `${read("core/README.md")}\nThe App brings up the Tauri SQL plugin.\n`,
  },
  "core/README.md contains stale Tauri SQL migration terminology",
);

expectFailure(
  "stale scoped migration terminology",
  {
    "src-tauri/AGENTS.md": `${read("src-tauri/AGENTS.md")}\nApp owns the Tauri SQL plugin migrations.\n`,
  },
  "src-tauri/AGENTS.md contains stale Tauri SQL migration terminology",
);

expectFailure(
  "stale binding regeneration command",
  {
    "CONTRIBUTING.md": read("CONTRIBUTING.md").replace(
      "npm run tauri:dev",
      "npm run tauri dev",
    ),
  },
  "CONTRIBUTING.md uses npm run tauri dev instead of npm run tauri:dev",
);

expectFailure(
  "stale Core settings path",
  {
    "core/README.md": read("core/README.md").replace(
      "storage_health.rs    Storage Health settings",
      "storage_smart.rs     Storage SMART settings",
    ),
  },
  "core/README.md does not point to the current storage settings module",
);

const after = gitStatus();
if (after !== before) {
  throw new Error("Agent guidance tests changed the worktree");
}

console.log(
  "Agent guidance negative tests passed without modifying the worktree.",
);
