---
id: LRN-20260906-isolate-duckdb-spill-directories
status: promoted
cause_status: confirmed
scope: core/examples/archive_engine_benchmark
trigger: comparing independent DuckDB database instances with memory-limited queries
failure_signature: one-year simultaneous native and Parquet round-trip reads terminate with SIGBUS
root_cause: the experimental harness assigned the same spill directory to independent native and in-memory DuckDB instances
guardrail: separate native and Parquet spill subdirectories in the experiment connection owner
canonical_refs: core/examples/archive_engine_benchmark/engine_benchmark.py
verification: the corrected seven-case engine matrix retains exact source/native/Parquet streaming digests and child exit codes
evidence: docs/development/hardware-archive-g1-engine-comparison.md
revalidate_when: DuckDB version, database-instance ownership, temporary-directory policy, or the verification harness changes
---

# Isolate DuckDB Spill Directories

The first SQLite/DuckDB/Parquet matrix opened independent native and in-memory
DuckDB instances with the same `temp_directory`. Both one-year fixtures exited
with SIGBUS during `fetchmany`, after smaller fixtures had passed. Each
candidate drained all 7,889,419 Process rows in isolation. Changing only the
temporary-directory ownership allowed the simultaneous three-way comparison
to finish with identical source, native, and Parquet digests at the same
128 MB per-instance memory limit and two-thread setting.

This matches the independent-instance temporary-file collision described in
[DuckDB issue 15173](https://github.com/duckdb/duckdb/issues/15173). The experiment
now uses separate native and Parquet subdirectories. Connections sharing one
native database instance use that instance's directory. The full matrix is the
regression surface because the failure requires spilling; a tiny smoke test
did not reveal it.

The observed SIGBUS belongs to the faulty experimental harness and is not
evidence that HardwareVisualizer's database is corrupt or that DuckDB cannot
read one year of history. Preserve the failed run as diagnostic evidence,
and use the corrected run for format comparisons. Revalidate this condition
when the engine, instance ownership, or temporary-directory policy changes.
