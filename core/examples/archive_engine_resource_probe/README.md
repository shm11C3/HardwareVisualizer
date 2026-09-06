# Archive Engine Resource Probe

This standalone Cargo workspace compares a common Tokio/serde JSON process
baseline with SQLx 0.8.6 SQLite and duckdb-rs 1.10505.0 bundled. It does not
change or model the complete Tauri application.

All three binaries use the repository release profile: size optimization,
debug disabled, overflow checks enabled, LTO enabled, symbols stripped, and one
codegen unit. Tokio uses its default multi-thread worker count and every stage
reports the host parallelism used by that default.

SQLite uses the current Core connection policy and SQLx feature set: WAL,
synchronous NORMAL, five-second busy timeout, and the default pool sizing.
The fixture follows current AUTOINCREMENT/DATETIME affinities. Its additional
Ambient epoch column/index is the measured timestamp compatibility adapter, not
a current production column. DuckDB enables only bundled, then sets two query
threads, a 128 MB engine-managed limit, and a per-process spill directory.

The engine binaries create deterministic, identical Process/Ambient fixtures
outside measurement:

    resource-probe-sqlite --prepare --database PATH --seed-rows 100000
    resource-probe-duckdb --prepare --database PATH --seed-rows 100000

Measured runs omit --prepare. They emit one JSON line and wait for one stdin
line at each stage: before_open, open_empty or open_seeded, post_query, and
after_close. The baseline emits before_open and after_close. Query rows and
statements are dropped before post_query; connections are dropped or closed
before after_close. The source accepts at most 400,000 Process rows so its
January timestamp fixture remains a valid calendar timestamp.

The query digest covers a CPU-ranked Process aggregate and an ID-ordered
inclusive Ambient range. Process CPU values use binary-exact quarter steps.
The digest detects fixture/query drift between the two engines; broader
production query equivalence remains outside this resource probe.
