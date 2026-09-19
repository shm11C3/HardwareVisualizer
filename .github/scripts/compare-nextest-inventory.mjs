#!/usr/bin/env node

import fs from "node:fs";

const args = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
  const argument = process.argv[index];
  if (!argument.startsWith("--")) {
    throw new Error(`unexpected argument: ${argument}`);
  }
  const value = process.argv[index + 1];
  if (!value || value.startsWith("--")) {
    throw new Error(`missing value for ${argument}`);
  }
  args.set(argument, value);
}

const baselinePath = args.get("--baseline");
const headPath = args.get("--head");
if (!baselinePath || !headPath) {
  throw new Error(
    "usage: compare-nextest-inventory.mjs --baseline FILE --head FILE",
  );
}

function readInventory(path) {
  const document = JSON.parse(fs.readFileSync(path, "utf8"));
  const tests = new Map();

  for (const suite of Object.values(document["rust-suites"] ?? {})) {
    const binary = suite["binary-name"];
    if (!binary) {
      continue;
    }
    for (const [name, testcase] of Object.entries(suite.testcases ?? {})) {
      const key = normalize(binary, name);
      const ignored = testcase.ignored === true;
      if (tests.has(key) && tests.get(key).ignored !== ignored) {
        throw new Error(`test changed ignored status in one inventory: ${key}`);
      }
      tests.set(key, { ignored });
    }
  }
  if (tests.size === 0) {
    throw new Error(`nextest inventory contains no test cases: ${path}`);
  }
  return tests;
}

function normalize(binary, name) {
  const unwrappedName = name.replace(/^native_support::/, "");

  // App-owned migration/schema tests are included by path in the Core
  // integration targets. They are intentionally owned by one Core target in
  // the refactor, so compare their semantic identity without the old binary
  // prefix; the baseline contains one copy per DuckDB binary.
  if (
    unwrappedName.startsWith("app_migrations::tests::") ||
    unwrappedName.startsWith("app_native_schema::tests::")
  ) {
    return unwrappedName;
  }

  if (binary !== "duckdb_native") {
    return `${binary}::${name}`;
  }

  // The native and reconcile families share one binary in the PoC. Preserve
  // their old binary identity so the inventory compares test semantics rather
  // than the implementation's module layout.
  const separator = unwrappedName.indexOf("::");
  if (separator !== -1) {
    const module = unwrappedName.slice(0, separator);
    if (module === "native") {
      return `duckdb_native::${unwrappedName.slice(separator + 2)}`;
    }
    if (module === "reconcile") {
      return `duckdb_reconcile::${unwrappedName.slice(separator + 2)}`;
    }
  }
  return `${binary}::${unwrappedName}`;
}

const baseline = readInventory(baselinePath);
const head = readInventory(headPath);
const missing = [...baseline.keys()].filter((key) => !head.has(key));
const unexpectedAdded = [...head.keys()].filter((key) => !baseline.has(key));
const changedIgnored = [...baseline.keys()]
  .filter(
    (key) =>
      head.has(key) && baseline.get(key).ignored !== head.get(key).ignored,
  )
  .map(
    (key) => `${key}: ${baseline.get(key).ignored} -> ${head.get(key).ignored}`,
  );

console.log(
  JSON.stringify(
    {
      baseline: baseline.size,
      head: head.size,
      missingCount: missing.length,
      addedCount: unexpectedAdded.length,
      changedIgnoredCount: changedIgnored.length,
      missingKeys: missing,
      addedKeys: unexpectedAdded,
      changedIgnoredTests: changedIgnored,
    },
    null,
    2,
  ),
);

if (missing.length || unexpectedAdded.length || changedIgnored.length) {
  process.exitCode = 1;
}
