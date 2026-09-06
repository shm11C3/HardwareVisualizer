# Archive engine comparison experiment

These standalone Python programs compare the original SQLite row format with
DuckDB native storage and Parquet read by DuckDB. They use synthetic databases;
none is imported by the application or the Rust benchmark. A completed run does
not approve a production storage format.

- `setup_macos.py` builds an isolated Python environment with DuckDB 1.5.5 and
  pysqlite3 0.5.4 linked to SQLite 3.46.0. Supply the SQLite amalgamation used by
  `libsqlite3-sys` 0.30.1. The output directory must be new. This is a Python
  binding/build comparison, not the production SQLx or Tauri build.
- `engine_contracts.py` probes stored-value preservation, SQLite storage classes,
  Process identity and aggregates, timestamp membership, and cancellation.
  Negative findings are retained in JSON even when the program exits zero.
- `concurrency_probe.py` tests one-process analytic reads with one-row appends
  and committed/in-flight transactions after a killed process. It does not
  prove power-loss durability or the production migration protocol. Parquet
  publication and recovery are explicitly untested.
- `engine_benchmark.py` prepares immutable candidates, validates all source rows
  after reopening, and measures/validates both query families and ranges. Its
  CLI supports prepare and isolated-strategy query phases.
- `run_matrix.py` executes all seven source cases serially in fresh child
  processes and retains complete phase reports, failures, and resource counters.

Use `--help` for arguments. Do not run builds, tests, or another benchmark during
measurement. Source databases are opened read-only and verified by SHA-256.
DuckDB uses two threads and a 128 MB managed-memory limit by default; this is
not a whole-process RSS cap. Results are fetched in batches of 500 rows.
Ambient candidates store the source SQLite epoch-millisecond expression as an
extra column; this is a semantic adapter, not native timestamp-parser parity.

The [measurement report](../../../docs/development/hardware-archive-g1-engine-comparison.md)
contains the complete commands, results, and limits of the experiment.

## Native qualification follow-up

- `duckdb_value_preservation.py` compares fully tagged and typed/exceptional
  representations on 0%/1% mixed-value fixtures, reopening before exact checks.
- `duckdb_lifecycle_probe.py` exercises minute-shaped commits with pinned reads,
  process interruption, sequence high-water preservation and eligible deletion.

Both are synthetic diagnostics; use the [initial qualification report](../../../docs/development/hardware-archive-duckdb-initial-qualification.md)
for commands, results and open gates. They do not establish exceptional query
semantics, production migration or power-loss durability.
