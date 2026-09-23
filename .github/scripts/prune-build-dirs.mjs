// Remove shared Cargo build subtrees whose checkout no longer exists.
//
// `.cargo/config.toml` sets `build-dir` to
// `{cargo-cache-home}/build/shared/{workspace-path-hash}`, so every worktree
// that has ever been built owns one subtree there, and removing the worktree
// (`git worktree remove`, or Claude Code discarding an agent worktree) leaves
// the subtree behind: 15 to 65 GiB each with the bundled DuckDB build.
//
// Cargo does not record which checkout a hash stands for. The App crate's
// build script does: `tauri-build` emits `cargo:rerun-if-changed=<absolute
// path>` lines for files under `src-tauri/`, and Cargo keeps them in
// `<profile>/build/hardware_visualizer-<hash>/output`. That absolute path is
// read back here, and the subtree is deleted when its checkout is gone.
//
// A subtree that never built the App crate cannot be attributed and is left
// alone, as is every subtree whose checkout still exists: throwing away a live
// worktree's build is the user's call, not this script's.
//
// An orphan is renamed to `<subtree>.pruning` before its files are deleted, so
// a run that is interrupted part-way leaves a half-deleted directory that the
// next run recognises and finishes, and two concurrent runs (several agent
// sessions stopping at once) cannot both claim the same subtree.
//
// Usage: node .github/scripts/prune-build-dirs.mjs [--dry-run] [--quiet]
// Runs detached from the shared agent Stop hook (.github/scripts/agent-hook.mjs)
// and as `npm run prune:build-dirs`.

import { existsSync } from "node:fs";
import { readdir, readFile, rename, rm, stat } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const dryRun = process.argv.includes("--dry-run");
const quiet = process.argv.includes("--quiet");
const APP_BUILD_SCRIPT_PREFIX = "hardware_visualizer-";
const PRUNING_SUFFIX = ".pruning";
// Anchored on the App crate's own manifest so an ancestor directory that
// happens to be called `src-tauri` cannot be mistaken for the checkout.
const CHECKOUT_PATTERN =
  /^cargo:rerun-if-changed=(.+)[\\/]src-tauri[\\/]tauri\.conf\.json\r?$/m;

function sharedRoot() {
  const cargoHome = process.env.CARGO_HOME || path.join(os.homedir(), ".cargo");
  return path.join(cargoHome, "build", "shared");
}

async function directories(parent) {
  const names = await readdir(parent).catch(() => []);
  const found = [];
  for (const name of names) {
    const full = path.join(parent, name);
    const info = await stat(full).catch(() => null);
    if (info?.isDirectory()) found.push(full);
  }
  return found;
}

/// The checkout a subtree was built from, or null when nothing in it says.
async function checkoutOf(subtree) {
  for (const profile of await directories(subtree)) {
    for (const scriptDir of await directories(path.join(profile, "build"))) {
      if (!path.basename(scriptDir).startsWith(APP_BUILD_SCRIPT_PREFIX))
        continue;
      const output = await readFile(
        path.join(scriptDir, "output"),
        "utf8",
      ).catch(() => "");
      // Only an absolute path names a checkout; tauri-build also emits
      // relative `..\LICENSE`-style lines that must not be mistaken for one.
      const match = CHECKOUT_PATTERN.exec(output);
      if (match && path.isAbsolute(match[1])) return match[1];
    }
  }
  return null;
}

/// True only when the checkout is positively gone. A checkout that exists but
/// cannot be inspected (permissions, a detached drive) is treated as present,
/// because deleting on a transient error would throw away a live build.
async function checkoutMissing(checkout) {
  try {
    await stat(checkout);
    return false;
  } catch (error) {
    return error.code === "ENOENT" || error.code === "ENOTDIR";
  }
}

/// Deletes a directory tree, reporting instead of aborting the whole run when
/// one tree cannot be removed (a file held open by a build, for example).
async function removeTree(dir, bucket) {
  try {
    await rm(dir, { recursive: true, force: true });
  } catch (error) {
    console.error(`failed  ${dir}  (${error.message}); retried next run`);
    return false;
  }
  await rm(bucket, { recursive: false }).catch(() => {});
  return true;
}

async function sizeBytes(dir) {
  let total = 0;
  const entries = await readdir(dir, {
    withFileTypes: true,
    recursive: true,
  }).catch(() => []);
  for (const entry of entries) {
    if (!entry.isFile()) continue;
    const info = await stat(path.join(entry.parentPath, entry.name)).catch(
      () => null,
    );
    total += info?.size ?? 0;
  }
  return total;
}

const gib = (bytes) => (bytes / 2 ** 30).toFixed(1);

const root = sharedRoot();
if (!existsSync(root)) process.exit(0);

let reclaimed = 0;
for (const bucket of await directories(root)) {
  for (const subtree of await directories(bucket)) {
    if (subtree.endsWith(PRUNING_SUFFIX)) {
      // Left by an interrupted run; already decided, just finish the job.
      if (dryRun) {
        console.log(`would finish removing ${subtree}`);
        continue;
      }
      if (await removeTree(subtree, bucket))
        console.log(`removed ${subtree}  (interrupted earlier)`);
      continue;
    }
    const checkout = await checkoutOf(subtree);
    if (checkout === null) {
      if (!quiet)
        console.log(
          `keep    ${subtree}  (App crate never built here; cannot attribute)`,
        );
      continue;
    }
    if (!(await checkoutMissing(checkout))) {
      if (!quiet) console.log(`keep    ${subtree}  <- ${checkout}`);
      continue;
    }
    const bytes = await sizeBytes(subtree);
    reclaimed += bytes;
    if (dryRun) {
      console.log(
        `would remove ${subtree}  (${gib(bytes)} GiB; checkout gone: ${checkout})`,
      );
      continue;
    }
    // Sizing a large subtree takes a while; the checkout may have been
    // recreated at the same path (and started reusing this subtree) meanwhile.
    if (!(await checkoutMissing(checkout))) {
      if (!quiet) console.log(`keep    ${subtree}  <- ${checkout} (recreated)`);
      reclaimed -= bytes;
      continue;
    }
    const claimed = `${subtree}${PRUNING_SUFFIX}`;
    const renamed = await rename(subtree, claimed).then(
      () => true,
      () => false,
    );
    if (!renamed) {
      // Another run claimed it first, or a build still holds files open.
      if (!quiet) console.log(`skip    ${subtree}  (busy)`);
      reclaimed -= bytes;
      continue;
    }
    if (!(await removeTree(claimed, bucket))) {
      reclaimed -= bytes;
      continue;
    }
    console.log(
      `removed ${subtree}  (${gib(bytes)} GiB; checkout gone: ${checkout})`,
    );
  }
}
if (reclaimed > 0 || !quiet) {
  console.log(`${dryRun ? "reclaimable" : "reclaimed"}: ${gib(reclaimed)} GiB`);
}
