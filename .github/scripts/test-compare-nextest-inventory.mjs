#!/usr/bin/env node

import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const script = path.join(import.meta.dirname, "compare-nextest-inventory.mjs");
const directory = fs.mkdtempSync(path.join(os.tmpdir(), "hardviz-nextest-inventory-"));

function suite(binary, testcases) {
  return {
    "binary-name": binary,
    testcases: Object.fromEntries(
      Object.entries(testcases).map(([name, ignored]) => [name, { ignored }]),
    ),
  };
}

const baseline = {
  "rust-suites": {
    ambient: suite("duckdb_ambient_fan", {
      "native_support::app_migrations::tests::migration_count": false,
      "native_support::app_native_schema::tests::schema_names": false,
      ambient_case: false,
    }),
    native: suite("duckdb_native", { native_case: false }),
    reconcile: suite("duckdb_reconcile", { reconcile_case: false }),
    duplicate: suite("duckdb_storage_health", {
      "native_support::app_migrations::tests::migration_count": false,
      "native_support::app_native_schema::tests::schema_names": false,
    }),
  },
};

const head = {
  "rust-suites": {
    ambient: suite("duckdb_ambient_fan", { ambient_case: false }),
    native: suite("duckdb_native", {
      "app_migrations::tests::migration_count": false,
      "app_native_schema::tests::schema_names": false,
      "native::native_case": false,
      "reconcile::reconcile_case": false,
    }),
  },
};

function run(baselineDocument, headDocument) {
  const baselinePath = path.join(directory, "baseline.json");
  const headPath = path.join(directory, "head.json");
  fs.writeFileSync(baselinePath, JSON.stringify(baselineDocument));
  fs.writeFileSync(headPath, JSON.stringify(headDocument));
  return spawnSync(process.execPath, [
    script,
    "--baseline",
    baselinePath,
    "--head",
    headPath,
  ], { encoding: "utf8" });
}

try {
  assert.equal(run(baseline, head).status, 0, "equivalent inventories must pass");

  const addedDomainTest = structuredClone(head);
  addedDomainTest["rust-suites"].ambient.testcases.new_case = false;
  assert.notEqual(run(baseline, addedDomainTest).status, 0, "unexpected added tests must fail");

  const droppedAppTest = structuredClone(head);
  delete droppedAppTest["rust-suites"].native.testcases["app_native_schema::tests::schema_names"];
  assert.notEqual(run(baseline, droppedAppTest).status, 0, "dropped App tests must fail");

  const droppedDomainTest = structuredClone(head);
  delete droppedDomainTest["rust-suites"].ambient.testcases.ambient_case;
  assert.notEqual(run(baseline, droppedDomainTest).status, 0, "dropped domain tests must fail");

  const changedIgnored = structuredClone(head);
  changedIgnored["rust-suites"].native.testcases["native::native_case"].ignored = true;
  assert.notEqual(run(baseline, changedIgnored).status, 0, "ignored changes must fail");

  const empty = { "rust-suites": {} };
  assert.notEqual(run(empty, head).status, 0, "empty inventories must fail");

  console.log("nextest inventory comparator fixtures passed");
} finally {
  fs.rmSync(directory, { recursive: true, force: true });
}
