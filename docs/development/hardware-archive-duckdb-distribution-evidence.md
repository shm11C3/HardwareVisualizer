# Native DuckDB Distribution And Durability Evidence

Status: evidence and plans for adoption blockers still listed in
[#2085](https://github.com/shm11C3/HardwareVisualizer/issues/2085) and
[#2084](https://github.com/shm11C3/HardwareVisualizer/issues/2084), under
[#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052). The
`duckdb-archive` feature is not enabled in any production build, no runtime
behavior changed, and no product code was added. Commands, exit codes,
environment, per-run load averages and the full third-party inventory are in the
[artifact](benchmarks/hardware-archive-duckdb-distribution-2026-09-12.json).
Measured on macOS 26.6.2 arm64 (Apple M4, 10 cores, 24 GiB) at
`1839986f6e29422420d5fcb4d49337e7bed1e31e`, rustc 1.98.1, Apple clang 17.0.0,
`duckdb 1.10505.0` with bundled DuckDB 1.5.5.

**Implication.** The remaining distribution blockers are narrower than #2085
implies: the feature already compiles and runs in CI on Windows x64, Linux x64
and macOS arm64, so what is left is macOS x64 (in no CI matrix), the absence of
any job that links an application binary or installer with the feature, and a
license gate and notice generator that both run with default features and so
never see the DuckDB tree. Durability is unchanged — the engine issues a real
per-commit flush on all three platforms, but only app-crash evidence exists.

## Measured

Both builds used `--release --locked --offline` with a fresh `CARGO_TARGET_DIR`
and exited 0. `Cargo.lock` was not modified.

```bash
cargo build --release --locked --offline -p hardviz-core --lib
cargo build --release --locked --offline -p hardviz-core --lib --features duckdb-archive
```

| Measurement | Without | With feature | Delta |
| --- | ---: | ---: | ---: |
| `hardviz-core` rlib bytes | 18,349,744 | 23,281,552 | +4,931,808 |
| rlib gzip -9 bytes | 4,507,872 | 5,715,878 | +1,208,006 |
| `target/` KiB | 548,796 | 1,038,660 | +489,864 |
| `libduckdb-sys` static archive bytes | 0 | 70,018,256 | +70,018,256 |
| Clean build s | 159.35 | 233.30 | +73.95 |

The rlib is not shipped and the compiled DuckDB C++ sits in the separate static
archive, before linking, LTO or stripping, so **no row here is the installed
size delta**. **The timings are not usable**: three other agent lanes compiled
this workspace concurrently (load 6.9–102 on 10 cores), and the with-feature
build reports a *faster* no-op rebuild (0.53 s vs 1.60 s) and touched-source
rebuild (9.34 s vs 37.00 s) than the strictly smaller baseline — physically
implausible, and itself the evidence that contention dominated.

Dependency growth: 432 → 473 crates, **41 added**. `ureq`, `zip`, `zopfli` and
`zlib-rs` enter only as *build* dependencies of `libduckdb-sys`; the `arrow-*`
crates link in. Because `core/Cargo.toml` uses `default-features = false,
features = ["bundled"]`, only `core_functions` is linked (not `json` or
`parquet`): 280 translation units, 12 vendored C/C++ libraries.

**Not measured.** `cargo build --release -p hardware_visualizer --features
custom-protocol,duckdb-archive` failed with `No space left on device (os error
28)` after the shared volume fell from 24 GiB to 132 MiB under four concurrent
lanes. It never reached a link step — an environment failure, not a product one
— and was not retried because a contended retry would produce another unusable
timing. The baseline half completed: 13,487,440 bytes, gzip 6,306,422, `target/`
2.25 GiB. No Tauri bundle was produced, so no installer size delta exists, and
Windows x64, Linux x64 and macOS x64 are unmeasured on every axis.

## License and packaging

`libduckdb-sys` and `duckdb` are MIT (Stichting DuckDB Foundation), and all 41
added crates use expressions already on the `src-tauri/deny.toml` allow list — so
the check would pass **if it ran**. It does not: `deny.toml` sets `[graph]
all-features = false`, and with default features `cargo metadata` returns no
`duckdb` node at all. `.github/scripts/generate-licenses.ts` shares the blind
spot, invoking `cargo license` and `cargo metadata` without features.

Notices do reach users: `publish.yml` regenerates `tmp/THIRD_PARTY_NOTICES.md`
just before `tauri-action`, and `tauri.conf.json` bundles it as a resource
beside `LICENSE`; the same file is also a release asset. The committed copy is a
29-byte placeholder.

The deeper gap is that crate metadata cannot describe the vendored C++. The
`duckdb.tar.gz` ships **no** `LICENSE`, `COPYING` or `NOTICE` file, yet 12
libraries compile into the binary — MIT, Apache-2.0, BSD-3-Clause, the
PostgreSQL License, a Bison skeleton under GPL-2.0-or-later *with* its special
exception, and two dual-licensed libraries (`mbedtls`, `zstd`). Each offers a
GPL-3.0-compatible arm, so nothing conflicts with the application's
GPL-3.0-or-later licence, but all carry attribution obligations that no tool
here satisfies. This is a reading of source headers, not a legal review.

Required changes, **not applied**: evaluate the feature in `deny.toml`; collect
notice metadata with the feature set the release builds; add a manual notice
covering DuckDB and those 12 libraries. Separately, the bundled build compiles
with `DUCKDB_EXTENSION_AUTOINSTALL_DEFAULT=1` and `..._AUTOLOAD_DEFAULT=1`,
countered only at open time by `enable_autoload_extension(false)` and `SET
enable_external_access = false` — worth an explicit decision against the
no-outbound-telemetry principle.

## Supported-target matrix

| Target | Compiled | Executed | Binary linked | Installer |
| --- | --- | --- | --- | --- |
| Windows x64 | yes (`lint-core`, `lint-tauri`) | yes (`test-core`) | no | no |
| Linux x64 | yes | yes | no | no |
| macOS arm64 | yes | yes | no | no |
| macOS x64 | **no** | no | no | no |

`test-core` runs `cargo test -p hardviz-core --features duckdb-archive` on all
three runner platforms. macOS x64 appears in no CI matrix; `publish.yml` only
cross-compiles it, without the feature. CI never bundles an installer
(`--no-bundle`). `Swatinem/rust-cache` uses `shared-key: "rust-workspace"`,
`add-job-id-key: false` and `save-if` restricted to `develop`, so every Rust job
shares one cache entry and no pull request saves; because rust-cache prunes
artifacts outside the current job's dependency set, a `develop` job *without*
the feature can save a cache lacking the DuckDB objects, and the compile cost is
not reliably amortised.

Smallest change closing the linked-binary gap. **Not applied.**

```diff
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
       - name: Build Tauri Rust crate (CI optimized for PR)
-        run: cargo build -p hardware_visualizer --profile ci --features custom-protocol
+        run: cargo build -p hardware_visualizer --profile ci --features custom-protocol,duckdb-archive
```

Estimated cost: the local cold delta was ~74 s on 10 M4 cores under load, and
GitHub runners have 4 slower cores, so budget roughly 5–12 minutes of added cold
wall time, Windows MSVC at the upper end. The three platforms run in parallel,
so that is added critical-path time, not three times the cost. This is
arithmetic over a contended local measurement, not an observed CI run. macOS x64
needs a separate matrix entry with `targets: x86_64-apple-darwin`; the installer
gap needs a bundling job that does not exist.

## Durability plan for #2084

**Engine behavior.** Every commit runs
`SingleFileStorageCommitState::FlushCommit()` → `WriteAheadLog::Flush()`
(`src/storage/write_ahead_log.cpp:542`), appending a `WAL_FLUSH` marker then
calling `writer->Sync()`. `LocalFileSystem::FileSync` uses
`fcntl(fd, F_FULLFSYNC)` on macOS with an fsync fallback, `fdatasync`/`fsync` on
Linux, `FlushFileBuffers` on Windows. Recovery (`wal_replay.cpp`) commits only
at `WAL_FLUSH` markers, checksums each entry, and — `abort_on_wal_failure` being
off by default — silently truncates a torn tail while rethrowing genuine
corruption. Defaults: `checkpoint_threshold` 16 MiB,
`wal_autocheckpoint_entries` disabled, `checkpoint_on_shutdown` true.
`native_config()` sets only `ReadWrite`, `threads(2)`, `max_memory("128MB")`,
`enable_autoload_extension(false)`, an owned spill directory and
`enable_external_access = false` — **no** checkpoint or WAL option, so those
defaults apply unreviewed.

**App-crash evidence is not power-loss evidence.** The 2026-09-06/07 SIGKILL
probes leave the page cache and kernel intact; they exercise WAL replay, not
stable storage. Proposed validation, in increasing cost:

1. On this macOS host, run the existing lifecycle probe under `sudo fs_usage -w
   -f filesys` and confirm one `F_FULLFSYNC` against the `.wal` per committed
   batch. Proves the call is issued; proves nothing about the hardware.
2. In a Linux guest, place the database on `dm-log-writes` (better than
   `dm-flakey`: cut points are enumerated deterministically), replay to each cut
   point and reopen, asserting every acknowledged commit survives, no batch is
   partial, and the identity high-water never regresses. Proves crash
   consistency against a device that loses unflushed writes.
3. Windows and Linux QEMU guests with `cache=writeback`, killed host-side to
   discard the emulated cache. Proves OS-crash survival per platform, including
   the `FlushFileBuffers` path.

A physical power cut is the only proof that a drive honours the flush and is out
of scope; the adopted target should say so. None of this covers disk exhaustion
during checkpoint or compaction, corruption outside the WAL, or a second
concurrent instance.

## File format compatibility

`VERSION_NUMBER` is 64 with a readable range of `[64, 68]`.
`SerializationCompatibility::Default()` returns `v0.10.2` (serialization version
1) unless `DUCKDB_LATEST_STORAGE` or `DUCKDB_ALTERNATIVE_VERIFY` is defined, and
`build_bundled_cc.rs` defines neither, so `GetVersionNumber()` returns 64. Files
this application creates are storage version 64 — the oldest format this engine
emits, readable by DuckDB v0.9.0 through v1.5.5.

The crate version therefore does **not** determine the file format, and pinning
`=1.10505.0` does not pin it. The hazard is the reverse direction:
`single_file_block_manager.cpp:1321` silently rewrites the header from 64 to 65
once the storage version reaches 4, so anything raising it — an explicit
`STORAGE_VERSION`, a future crate default, or a feature requiring it —
permanently makes the archive unreadable by older builds. The policy should
assert the written storage version explicitly instead of inheriting the default,
record it beside `schema_version` in the native metadata table, and treat a
crate bump as a format review rather than a routine dependency update.

## Limits

Single run per configuration, macOS arm64 only, all timings unusable (heavy
concurrent load; two of three deltas physically implausible). Unmeasured: the
with-feature application binary, every installer, and all other targets. The CI
estimate is arithmetic, not an observed run. Licences were read from source
headers in a tarball that ships no licence files — not a legal review. The
durability work is a plan; no OS-crash or power-loss test ran. Storage-version
findings come from source constants, since no with-feature binary existed to
create and inspect a DuckDB file with.
