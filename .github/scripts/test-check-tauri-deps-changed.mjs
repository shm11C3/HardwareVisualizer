import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = join(
  dirname(fileURLToPath(import.meta.url)),
  "check-tauri-deps-changed.ts",
);
const testRepo = mkdtempSync(join(tmpdir(), "tauri-deps-check-"));

function git(...args) {
  return execFileSync("git", args, {
    cwd: testRepo,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function check(baseRef, headRef) {
  const outputPath = join(testRepo, "github-output");
  rmSync(outputPath, { force: true });
  execFileSync(process.execPath, [scriptPath, baseRef, headRef], {
    cwd: testRepo,
    encoding: "utf8",
    env: { ...process.env, GITHUB_OUTPUT: outputPath },
  });
  return Object.fromEntries(
    readFileSync(outputPath, "utf8")
      .trim()
      .split("\n")
      .map((line) => line.split("=")),
  );
}

try {
  git("init", "--initial-branch=main");
  git("config", "user.name", "Tauri dependency check test");
  git("config", "user.email", "tauri-deps-check@example.invalid");
  mkdirSync(join(testRepo, "src-tauri"));

  writeFileSync(
    join(testRepo, "package.json"),
    `${JSON.stringify(
      {
        dependencies: {
          "@tauri-apps/api": "^2.11.0",
          react: "19.2.0",
        },
      },
      null,
      2,
    )}\n`,
  );
  writeFileSync(
    join(testRepo, "package-lock.json"),
    `${JSON.stringify(
      {
        lockfileVersion: 3,
        packages: {
          "": {
            dependencies: {
              "@tauri-apps/api": "^2.11.0",
              react: "19.2.0",
            },
          },
          "node_modules/@tauri-apps/api": { version: "2.11.0" },
          "node_modules/react": { version: "19.2.0" },
        },
      },
      null,
      2,
    )}\n`,
  );
  writeFileSync(
    join(testRepo, "Cargo.lock"),
    `version = 4

[[package]]
name = "tauri"
version = "2.11.0"
dependencies = [
 "os_pipe",
 "tauri-runtime",
]

[[package]]
name = "tauri-runtime"
version = "2.11.0"

[[package]]
name = "os_pipe"
version = "1.2.3"
dependencies = [
 "windows-sys 0.61.2",
]
`,
  );
  writeFileSync(
    join(testRepo, "src-tauri/Cargo.toml"),
    `[dependencies]
tauri = "2.11.0"
serde = "1"
`,
  );

  git("add", ".");
  git("commit", "-m", "test: add baseline dependencies");

  assert.deepEqual(check("HEAD", "--worktree"), {
    changed: "false",
    npm_changed: "false",
    cargo_changed: "false",
  });

  const cargoLockPath = join(testRepo, "Cargo.lock");
  writeFileSync(
    cargoLockPath,
    readFileSync(cargoLockPath, "utf8").replace(
      "windows-sys 0.61.2",
      "windows-sys 0.48.0",
    ),
  );
  assert.deepEqual(
    check("HEAD", "--worktree"),
    {
      changed: "false",
      npm_changed: "false",
      cargo_changed: "false",
    },
    "transitive lockfile rewrites alone must not open a Tauri update PR",
  );

  writeFileSync(
    cargoLockPath,
    readFileSync(cargoLockPath, "utf8").replace(
      'name = "tauri-runtime"\nversion = "2.11.0"',
      'name = "tauri-runtime"\nversion = "2.12.0"',
    ),
  );
  assert.deepEqual(
    check("HEAD", "--worktree"),
    {
      changed: "true",
      npm_changed: "false",
      cargo_changed: "true",
    },
    "a Tauri crate update must remain eligible for a PR",
  );

  git("restore", "Cargo.lock");
  const packageLockPath = join(testRepo, "package-lock.json");
  writeFileSync(
    packageLockPath,
    readFileSync(packageLockPath, "utf8").replace(
      '"node_modules/@tauri-apps/api": {\n      "version": "2.11.0"',
      '"node_modules/@tauri-apps/api": {\n      "version": "2.12.0"',
    ),
  );
  assert.deepEqual(
    check("HEAD", "--worktree"),
    {
      changed: "true",
      npm_changed: "true",
      cargo_changed: "false",
    },
    "a Tauri npm update must remain eligible for a PR",
  );

  console.log("Tauri dependency change detection tests passed.");
} finally {
  rmSync(testRepo, { recursive: true, force: true });
}
