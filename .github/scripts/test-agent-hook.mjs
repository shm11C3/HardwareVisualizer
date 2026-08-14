import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);
const hook = path.join(root, ".github/scripts/agent-hook.mjs");

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

function runHook(name, mode, payload, expectedStatus, options = {}) {
  const input = typeof payload === "string" ? payload : JSON.stringify(payload);
  const result = spawnSync(process.execPath, [hook, mode], {
    cwd: options.cwd ?? root,
    env: { ...process.env, ...options.env },
    input,
    encoding: "utf8",
  });
  if (result.status !== expectedStatus) {
    throw new Error(
      `${name}: expected exit ${expectedStatus}, got ${result.status}\n${result.stdout}${result.stderr}`,
    );
  }
}

const before = gitStatus();

runHook("normal edit", "pre", { tool_input: { file_path: "src/App.tsx" } }, 0);
runHook(
  "absolute generated binding",
  "pre",
  { tool_input: { file_path: `${root}/src/rspc/bindings.ts` } },
  2,
);
runHook(
  "normalized generated binding",
  "pre",
  { tool_input: { file_path: "src/../src/rspc/bindings.ts" } },
  2,
);
runHook(
  "apply patch generated binding",
  "pre",
  {
    tool_input:
      "*** Begin Patch\n*** Update File: src/rspc/bindings.ts\n*** End Patch",
  },
  2,
);
runHook("null payload", "pre", "null", 2);
runHook("scalar payload", "pre", "42", 2);
runHook(
  "move to generated binding",
  "pre",
  {
    tool_input:
      "*** Begin Patch\n*** Update File: src/tmp.ts\n*** Move to: src/rspc/bindings.ts\n*** End Patch",
  },
  2,
);
runHook("malformed payload", "pre", "{", 2);
runHook(
  "subdirectory invocation",
  "pre",
  { tool_input: { file_path: "src/App.tsx" } },
  0,
  { cwd: path.join(root, "src") },
);
runHook(
  "touched guidance validation",
  "post",
  { tool_input: { file_path: "AGENTS.md" } },
  0,
);
runHook(
  "touched guidance schema failure",
  "post",
  {
    tool_input: {
      file_path: ".agents/rules/design.md",
    },
  },
  2,
  {
    env: {
      AGENT_GUIDANCE_OVERRIDES: JSON.stringify({
        ".agents/rules/design.md": readFileSync(
          path.join(root, ".agents/rules/design.md"),
          "utf8",
        ).replace('scope: "**"', 'scope: ""'),
      }),
    },
  },
);
runHook("complete guidance validation", "stop", {}, 0);

const incompleteGuidance = readFileSync(
  path.join(root, "AGENTS.md"),
  "utf8",
).replaceAll("docs/design-principles.md", "docs/pending-design-principles.md");
const incompleteEnv = {
  AGENT_GUIDANCE_OVERRIDES: JSON.stringify({
    "AGENTS.md": incompleteGuidance,
  }),
};
runHook(
  "post defers cross-file consistency",
  "post",
  { tool_input: { file_path: "AGENTS.md" } },
  0,
  { env: incompleteEnv },
);
runHook("stop catches cross-file consistency", "stop", {}, 2, {
  env: incompleteEnv,
});

for (const configPath of [".codex/hooks.json", ".claude/settings.json"]) {
  const config = JSON.parse(readFileSync(path.join(root, configPath), "utf8"));
  const command = config.hooks.PreToolUse[0].hooks[0].command;
  const result = spawnSync(command, {
    cwd: path.join(root, "src"),
    input: JSON.stringify({ tool_input: { file_path: "src/App.tsx" } }),
    encoding: "utf8",
    shell: true,
  });
  if (result.status !== 0) {
    throw new Error(
      `${configPath}: configured hook failed from a subdirectory\n${result.stdout}${result.stderr}`,
    );
  }
}

runHook(
  "nested apply_patch payload",
  "pre",
  {
    tool_input: {
      arguments: {
        input: "*** Begin Patch\n*** Update File: src/App.tsx\n*** End Patch",
      },
    },
  },
  0,
);

const after = gitStatus();
if (after !== before) {
  throw new Error("Agent hooks changed the worktree");
}

console.log("Agent hook tests passed without modifying the worktree.");
