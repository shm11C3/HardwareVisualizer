/// <reference types="node" />

import { execFileSync } from "node:child_process";
import { appendFileSync } from "node:fs";

type JsonObject = Record<string, unknown>;
type Extractor = (text: string) => string;

const baseRef = process.argv[2] ?? "HEAD^";
const outputPath = process.env.GITHUB_OUTPUT;
const tauriCratePattern = /^(tauri|tauri-build|tauri-specta|tauri-plugin-.+)$/;

const jsonSections = [
  "dependencies",
  "devDependencies",
  "optionalDependencies",
  "peerDependencies",
] as const;

function readFileAt(ref: string, path: string): string {
  try {
    return execFileSync("git", ["show", `${ref}:${path}`], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
  } catch {
    return "";
  }
}

function isObject(value: unknown): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asObject(value: unknown): JsonObject {
  return isObject(value) ? value : {};
}

function stableJson(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(stableJson);
  }

  if (isObject(value)) {
    return Object.fromEntries(
      Object.entries(value)
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([key, child]) => [key, stableJson(child)]),
    );
  }

  return value;
}

function parseJson(text: string): JsonObject {
  try {
    return asObject(JSON.parse(text));
  } catch {
    return {};
  }
}

function extractPackageJson(text: string): string {
  const json = parseJson(text);
  const entries: Record<string, unknown> = {};

  for (const section of jsonSections) {
    for (const [name, version] of Object.entries(asObject(json[section]))) {
      if (name.startsWith("@tauri-apps/")) {
        entries[`${section}:${name}`] = version;
      }
    }
  }

  return JSON.stringify(stableJson(entries));
}

function extractPackageLock(text: string): string {
  const json = parseJson(text);
  const entries: Record<string, unknown> = {};

  for (const [path, metadata] of Object.entries(asObject(json.packages))) {
    if (path === "") {
      for (const section of jsonSections) {
        for (const [name, version] of Object.entries(
          asObject(asObject(metadata)[section]),
        )) {
          if (name.startsWith("@tauri-apps/")) {
            entries[`root:${section}:${name}`] = version;
          }
        }
      }
      continue;
    }

    if (path.startsWith("node_modules/@tauri-apps/")) {
      entries[path] = metadata;
    }
  }

  for (const [name, metadata] of Object.entries(asObject(json.dependencies))) {
    if (name.startsWith("@tauri-apps/")) {
      entries[`dependency:${name}`] = metadata;
    }
  }

  return JSON.stringify(stableJson(entries));
}

function extractCargoLock(text: string): string {
  const entries: string[] = [];

  for (const block of text.split(/\n\s*\n/)) {
    const name = block.match(/^name = "([^"]+)"/m)?.[1];
    if (name && tauriCratePattern.test(name)) {
      entries.push(block.trim());
    }
  }

  return entries.sort().join("\n\n");
}

function extractCargoToml(text: string): string {
  const entries: string[] = [];
  let currentTable = "";
  let currentTableBody: string[] = [];
  let collectTable = false;

  function flushTable() {
    if (collectTable) {
      entries.push([currentTable, ...currentTableBody].join("\n"));
    }
    currentTableBody = [];
    collectTable = false;
  }

  for (const line of text.split("\n")) {
    const table = line.match(/^\s*\[([^\]]+)\]\s*$/)?.[1];
    if (table) {
      flushTable();
      currentTable = line.trim();
      collectTable = tauriCratePattern.test(table.split(".").at(-1) ?? "");
      continue;
    }

    if (collectTable) {
      currentTableBody.push(line);
    }

    const dependency = line.match(/^\s*([A-Za-z0-9_-]+)\s*=/)?.[1];
    if (dependency && tauriCratePattern.test(dependency)) {
      entries.push(line.trim());
    }
  }

  flushTable();
  return entries.sort().join("\n");
}

const checks: [path: string, extract: Extractor][] = [
  ["package.json", extractPackageJson],
  ["package-lock.json", extractPackageLock],
  ["Cargo.lock", extractCargoLock],
  ["src-tauri/Cargo.toml", extractCargoToml],
];

const changed = checks.some(([path, extract]) => {
  const base = extract(readFileAt(baseRef, path));
  const head = extract(readFileAt("HEAD", path));
  return base !== head;
});

if (outputPath) {
  appendFileSync(outputPath, `changed=${changed}\n`);
} else {
  console.log(`changed=${changed}`);
}
