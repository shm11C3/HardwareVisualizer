import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  chmodSync,
  existsSync,
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
  "generate-licenses.ts",
);

const validNpmData = {
  "npm-package@1.0.0": {
    licenses: "MIT",
  },
};

const workspaceId = "workspace 0.1.0";
const rustPackageId = "rust-package 1.0.0";
const validCargoData = [
  {
    name: "rust-package",
    version: "1.0.0",
    license: "MIT",
  },
];
const validMetadata = {
  packages: [
    {
      id: workspaceId,
      name: "workspace",
      version: "0.1.0",
      manifest_path: "/workspace/Cargo.toml",
    },
    {
      id: rustPackageId,
      name: "rust-package",
      version: "1.0.0",
      manifest_path: "/rust-package/Cargo.toml",
    },
  ],
  resolve: {
    nodes: [
      {
        id: workspaceId,
        deps: [
          {
            pkg: rustPackageId,
            dep_kinds: [{ kind: null }],
          },
        ],
      },
      {
        id: rustPackageId,
        deps: [],
      },
    ],
  },
  workspace_members: [workspaceId],
};

function writeExecutable(directory, name, source) {
  const filePath = join(directory, name);
  writeFileSync(filePath, `#!/usr/bin/env node\n${source}\n`);
  chmodSync(filePath, 0o755);
}

function runGenerator({
  npmData = validNpmData,
  npmExitCode = 0,
  cargoData = validCargoData,
  metadata = validMetadata,
  cargoExitCode = 0,
}) {
  const testRoot = mkdtempSync(join(tmpdir(), "generate-licenses-"));
  const binDir = join(testRoot, "bin");
  const workspaceDir = join(testRoot, "workspace");
  const outputPath = join(workspaceDir, "tmp", "THIRD_PARTY_NOTICES.md");

  try {
    mkdirSync(binDir, { recursive: true });
    mkdirSync(workspaceDir, { recursive: true });
    writeFileSync(join(workspaceDir, "package-lock.json"), "{}\n");
    writeFileSync(join(workspaceDir, "Cargo.lock"), "version = 4\n");

    writeExecutable(
      binDir,
      "npx",
      `if (${npmExitCode} !== 0) process.exit(${npmExitCode});\nconsole.log(${JSON.stringify(JSON.stringify(npmData))});`,
    );
    writeExecutable(
      binDir,
      "cargo",
      `if (${cargoExitCode} !== 0) process.exit(${cargoExitCode});\nif (process.argv[2] === "license") {\n  console.log(${JSON.stringify(JSON.stringify(cargoData))});\n} else if (process.argv[2] === "metadata") {\n  console.log(${JSON.stringify(JSON.stringify(metadata))});\n} else {\n  process.exit(2);\n}`,
    );

    const result = spawnSync(
      process.execPath,
      ["--experimental-strip-types", scriptPath, "tmp"],
      {
        cwd: workspaceDir,
        encoding: "utf8",
        env: {
          ...process.env,
          PATH: `${binDir}:${process.env.PATH ?? ""}`,
        },
      },
    );
    const outputExists = existsSync(outputPath);

    return {
      status: result.status,
      stdout: result.stdout,
      stderr: result.stderr,
      output: outputExists ? readFileSync(outputPath, "utf8") : null,
    };
  } finally {
    rmSync(testRoot, { recursive: true, force: true });
  }
}

const npmFailure = runGenerator({ npmExitCode: 1 });
assert.notEqual(npmFailure.status, 0);
assert.match(npmFailure.stderr, /Failed to collect NPM licenses/);
assert.equal(npmFailure.output, null);

const emptyNpm = runGenerator({ npmData: {} });
assert.notEqual(emptyNpm.status, 0);
assert.match(emptyNpm.stderr, /empty generated license sections: NPM/);
assert.equal(emptyNpm.output, null);

const emptyRust = runGenerator({
  cargoData: [],
  metadata: {
    packages: [validMetadata.packages[0]],
    resolve: { nodes: [{ id: workspaceId, deps: [] }] },
    workspace_members: [workspaceId],
  },
});
assert.notEqual(emptyRust.status, 0);
assert.match(emptyRust.stderr, /empty generated license sections: Rust/);
assert.equal(emptyRust.output, null);

const rustFailure = runGenerator({ cargoExitCode: 1 });
assert.notEqual(rustFailure.status, 0);
assert.match(rustFailure.stderr, /Failed to collect Rust licenses/);
assert.equal(rustFailure.output, null);

const success = runGenerator({});
assert.equal(success.status, 0, success.stderr);
assert.ok(success.output);
assert.match(success.output, /## npm-package@1\.0\.0/);
assert.match(success.output, /## rust-package/);

console.log("License notice generation tests passed.");
