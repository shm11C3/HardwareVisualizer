import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const workflow = readFileSync(
  new URL("../workflows/ci.yml", import.meta.url),
  "utf8",
);
const block = (name) => {
  const value = workflow.split(`\n  ${name}:\n`)[1]?.split(/\n {2}[\w-]+:/)[0];
  assert.ok(value, `Missing job: ${name}`);
  return value;
};

function runs(name, changed) {
  const condition = block(name).match(/^ {4}if: (.+)$/m)?.[1];
  assert.ok(condition, `Missing condition: ${name}`);
  // Test the changed job condition; existing paths-filter classification is
  // outside this change. Reject a new expression shape rather than misread it.
  return condition
    .split(" || ")
    .map((term) => {
      const match = term.match(
        /^needs\.change-detection\.outputs\.([\w-]+) == 'true'$/,
      );
      assert.ok(match, `Unsupported condition: ${term}`);
      return changed.includes(match[1]);
    })
    .some(Boolean);
}

test("frontend-only changes retain native coverage without regenerating bindings", () => {
  assert.equal(runs("check-tauri-bindings", ["frontend-changed"]), false);
  assert.equal(runs("test-e2e-native", ["frontend-changed"]), true);
  assert.equal(
    runs("check-tauri-bindings", ["frontend-changed", "core-changed"]),
    true,
  );
});

test("generated-file edits, producers and fail-closed checks still regenerate bindings", () => {
  assert.match(
    workflow,
    /bindings-changed: \$\{\{ steps\.changes\.outputs\.bindings \}\}/,
  );
  assert.match(
    workflow,
    /^ {12}bindings:\n {14}- 'src\/rspc\/bindings\.ts'\n/m,
  );
  for (const output of [
    "bindings-changed",
    "core-changed",
    "tauri-changed",
    "backend-deps-changed",
    "actions-changed",
    "unclassified-changed",
    "is-push",
  ]) {
    assert.equal(runs("check-tauri-bindings", [output]), true, output);
  }
  for (const name of ["check-tauri-bindings", "test-e2e-native"]) {
    assert.equal(runs(name, ["is-push"]), true);
    assert.match(block("merge-gate"), new RegExp(`^ {6}- ${name}$`, "m"));
  }
});

test("native acceleration keeps the existing driver cache and credential trust guard", () => {
  const native = block("test-e2e-native");
  assert.match(native, /cache-key: tauri-driver\n {10}cache-targets: "false"/);
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
