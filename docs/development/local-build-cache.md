# Local Build Cache Across Git Worktrees

HardwareVisualizer development routinely runs several git worktrees at once
(review branches, parallel implementation lanes, Codex or Claude sessions). By
default Cargo gives every checkout its own `target/`, so each worktree rebuilds
and stores the same dependency graph again. With the `duckdb-archive` feature
enabled a single debug `target/` is roughly 5 to 7 GB, of which about 3 GB is
the bundled DuckDB C++ build under `target/debug/build/libduckdb-sys-*`. Four
concurrent worktrees therefore cost 20 to 30 GB for identical bytes.

## What the repository configures

`.cargo/config.toml` sets Cargo's `build.build-dir` to a single machine-wide
directory:

```toml
[build]
build-dir = "{cargo-cache-home}/build/shared"
```

`{cargo-cache-home}` resolves to `$CARGO_HOME` (usually `~/.cargo`), so on a
developer machine the shared directory is
`~/.cargo/build/shared`. Cargo 1.91 or newer is required; the
pinned toolchain in `rust-toolchain.toml` satisfies this.

The split is:

- **`build-dir` (shared)** holds intermediate artifacts: dependency `.rlib`
  files, build-script output (including the DuckDB build), incremental caches,
  fingerprints, and test executables.
- **`target/` (per checkout)** keeps only final artifacts: the application
  binary, examples, and Tauri bundles under `target/release/bundle`. Every path
  that CI, `tauri dev`, `tauri build`, and the perf workflows read stays where it
  was.

Cargo keys shared artifacts by package id, features, profile, and compiler
version, so worktrees on different commits or with different `Cargo.lock`
contents coexist in the same directory without invalidating each other.

Measured on 2026-09-12 with cargo 1.98.1: after one worktree built
`hardviz-core` (default features) into the shared directory, a second worktree
on the same commit reported `Finished` in 0.6 s with zero crates recompiled, and
each worktree's own `target/` held 77 MB.

The directory name is project-neutral on purpose: Cargo keys every artifact by
package id, features, profile, and compiler version, so other Rust projects on
the same machine can share it safely, and a user-level `~/.cargo/config.toml`
with the same key gives worktrees on older branches (which predate this file)
the same behavior. The repository setting and the user setting point at the
same path, so the two never split the cache.

## What changes for you

- The first build after upgrading repopulates the shared directory; existing
  per-worktree `target/` directories are no longer read and can be deleted.
- Two worktrees building the **same profile** at the same time serialize on
  Cargo's build-directory lock (`Blocking waiting for file lock on build
  directory`). Total work is lower than before because each crate compiles
  once, but a lane may wait for another lane's build to finish. Debug and
  release builds of different worktrees still run in parallel.
- `cargo clean` from one worktree removes the shared intermediate artifacts for
  every worktree. Prefer deleting only the checkout's `target/` when you want a
  local reset, and reserve `cargo clean` or `rm -rf
  ~/.cargo/build/shared` for reclaiming disk space.
- To opt out for one command, set the environment variable
  `CARGO_BUILD_BUILD_DIR` (for example to `target`), which overrides the config
  file.

## Keeping disk usage bounded

The shared directory still accumulates artifacts across compiler upgrades and
dependency bumps. Reclaim space occasionally with:

```bash
rm -rf ~/.cargo/build/shared
```

Do not run `rm -rf target` in the main checkout without looking first: some
tools (Codex, for example) create their worktrees under `target/codex-worktrees/`,
and deleting `target/` there deletes those checkouts, including uncommitted
work. Delete `target/debug` and `target/release` instead, or move the worktrees
out first.

Worktree hygiene matters as much as the cache. `git worktree remove` deletes the
checkout but leaves nothing behind only if the checkout no longer owns a large
`target/`; with the shared build directory that is now the normal case. Drop
registrations whose directories were deleted by hand with:

```bash
git worktree prune
```

To list every worktree together with what its checkout still occupies:

```bash
git worktree list --porcelain | awk '/^worktree /{print $2}' | xargs -I{} du -sh {}
```
