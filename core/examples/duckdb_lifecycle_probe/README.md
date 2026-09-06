# DuckDB Rust lifecycle probe

This standalone Cargo workspace tests duckdb-rs 1.10505.0 connection and
interrupt lifecycle against bundled DuckDB 1.5.5. It does not link to Core or
change production database ownership.

Build and run it serially, outside other builds and measurements:

    CARGO_TARGET_DIR=/path/to/rust-target cargo build --release --jobs 2 \
      --manifest-path core/examples/duckdb_lifecycle_probe/Cargo.toml
    /path/to/rust-target/release/duckdb-lifecycle-probe \
      --output /path/to/result.json \
      --database /path/to/probe.duckdb

The process has a 30-second watchdog. It creates a synthetic database, cancels
one long query out of band, reuses the same owner connection for another read
and a minute-shaped Process/Ambient commit, observes a pinned reader and
checkpoint, closes the owner, and verifies committed rows after reopen.

The synthetic minute commit deliberately combines Process and Ambient rows in
one transaction to test owner reuse. Production currently writes those raw
families through separate transaction calls, so this is not a transaction
parity claim.

The fixture preserves stored (pid, process_name) pairs as opaque archive
identity. This probe does not repeat process-crash, power-loss, migration,
capacity, or broad query-equivalence tests.

The [`evidence`](evidence/) directory contains the aggregate machine-readable
record and three raw runs, including each command, exit status, stdout, and
stderr. Hashes under `selected_source` describe the final source used for all
successful runs. Earlier failed build entries are provenance for superseded
source and do not describe the packaged final source.
