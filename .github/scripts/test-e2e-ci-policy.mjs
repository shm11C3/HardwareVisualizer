import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { matchesGlob } from "node:path";
import { test } from "node:test";

const workflow = readFileSync(
  new URL("../workflows/ci.yml", import.meta.url),
  "utf8",
);
// Read the actual paths-filter blocks without installing dependencies in the
// cheap change-detection job. Reject unfamiliar syntax rather than skipping it.
const filters = new Map();
for (const block of workflow.matchAll(
  / {10}filters: \|\n((?: {12}.*\n|\n)+)/g,
)) {
  let name;
  for (const line of block[1].split("\n").filter((value) => value.trim())) {
    if (line.trimStart().startsWith("#")) continue;
    const heading = line.match(/^ {12}([\w-]+):$/);
    if (heading) {
      name = heading[1];
      filters.set(name, []);
      continue;
    }
    const pattern = line.match(/^ {14}- '([^']+)'$/);
    assert.ok(pattern && name, `Unsupported filter line: ${line}`);
    filters.get(name).push(pattern[1]);
  }
}
assert.ok(filters.has("unclassified"));

function outputsFor(files, push = false) {
  const outputs = { "is-push": String(push) };
  for (const [name, patterns] of filters) {
    const positives = patterns.filter((pattern) => !pattern.startsWith("!"));
    const negatives = patterns.filter((pattern) => pattern.startsWith("!"));
    outputs[`${name}-changed`] = String(
      files.some(
        (file) =>
          positives.some((pattern) => matchesGlob(file, pattern)) &&
          !negatives.some((pattern) => matchesGlob(file, pattern.slice(1))),
      ),
    );
  }
  return outputs;
}

function jobBlock(name) {
  const block = workflow.match(
    new RegExp(
      `^  ${name}:\\n([\\s\\S]*?)(?=^  [a-z][\\w-]*:|(?![\\s\\S]))`,
      "m",
    ),
  );
  assert.ok(block, `Missing job: ${name}`);
  return block[1];
}

function runs(name, files, push = false) {
  const outputs = outputsFor(files, push);
  const condition = jobBlock(name).match(/^ {4}if: (.+)$/m)?.[1];
  assert.ok(condition, `Missing condition: ${name}`);
  return condition.split(" || ").some((term) => {
    const match = term.match(
      /^needs\.change-detection\.outputs\.([\w-]+) == 'true'$/,
    );
    assert.ok(match, `Unsupported condition: ${term}`);
    assert.ok(match[1] in outputs, `Missing output: ${match[1]}`);
    return outputs[match[1]] === "true";
  });
}

test("frontend consumers keep web/native coverage without regenerating bindings", () => {
  for (const file of [
    "src/app/App.tsx",
    "src/types/i18next.d.ts",
    "src/lang/en.json",
    "e2e/settings.spec.ts",
    "package-lock.json",
    "vite.config.ts",
    "playwright.config.ts",
  ]) {
    assert.equal(runs("check-tauri-bindings", [file]), false, file);
    assert.equal(runs("test-e2e-web", [file]), true, file);
    assert.equal(runs("test-e2e-native", [file]), true, file);
  }
});

test("binding producers, generated edits, unknown inputs and pushes stay checked", () => {
  assert.match(
    workflow,
    /bindings-changed: \$\{\{ steps\.changes\.outputs\.bindings \}\}/,
  );
  for (const file of [
    "src/rspc/bindings.ts",
    "core/src/models/hardware.rs",
    "src-tauri/src/commands/system.rs",
    "Cargo.lock",
    ".cargo/config.toml",
    "rust-toolchain.toml",
    ".github/workflows/ci.yml",
    ".github/actions/setup-rust/action.yml",
    ".github/scripts/merge-gate.ts",
    "unclassified-input",
  ]) {
    assert.equal(runs("check-tauri-bindings", [file]), true, file);
  }
  assert.equal(
    runs("check-tauri-bindings", ["src/app/App.tsx", "core/src/lib.rs"]),
    true,
  );
  for (const name of [
    "check-tauri-bindings",
    "test-e2e-web",
    "test-e2e-native",
  ]) {
    assert.equal(runs(name, [], true), true);
    assert.match(jobBlock("merge-gate"), new RegExp(`^      - ${name}$`, "m"));
  }
});

test("native cache keeps app targets and the existing credential trust guard", () => {
  const native = jobBlock("test-e2e-native");
  assert.match(native, /cache-key: tauri-native-e2e/);
  assert.doesNotMatch(native, /cache-targets: "false"/);
  assert.match(
    native,
    /if: env\.R2_CACHE_ALLOWED == 'true'\n {8}uses: \.\/\.github\/actions\/cache-sccache/,
  );
  assert.match(native, /expect-cpp: "true"/);
  assert.match(native, /xvfb-run npm run test:e2e:native/);
  assert.match(
    workflow,
    /R2_CACHE_ALLOWED:.*dependabot\[bot\].*head\.repo\.full_name == github\.repository/,
  );
});
