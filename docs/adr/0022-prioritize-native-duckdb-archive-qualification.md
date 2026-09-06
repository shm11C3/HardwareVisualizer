# Prioritize Native DuckDB Archive Qualification

Status: accepted

Tracking issue: [#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052).

Accepted direction on 2026-09-06: prioritize **native DuckDB** as the next
Hardware Archive prototype. This accepts an investigation order, not a
production format, migration implementation, dependency, or release switch.

## Context

The maintainer reopened the SQLite-only choice after long-range query
experiments. Custom binary64 chunk summaries failed an integer cancellation
case and unconditional summaries expanded high-cardinality Process data.
The subsequent [engine comparison](https://github.com/shm11C3/HardwareVisualizer/blob/75db64c972b39d94205800cafb6ccb15d6d6029a/docs/development/hardware-archive-g1-engine-comparison.md) measured SQLite rows, native
DuckDB, and Parquet queried by DuckDB on the same synthetic source records.

For the one-year stable fixture, total Process/Ambient storage was 883.348 MiB
in SQLite, 204.262 MiB in native DuckDB, and 83.459 MiB in Parquet. Complete
half-range Process-query p95 was 1,743.515 / 59.039 / 89.383 ms respectively.
All seven generated cases passed exact reopened-row comparisons and query
comparisons under the declared numerical tolerance.
The fixed typed mapping still failed SQLite mixed-storage-class/invalid-UTF8
probes, and native timestamp conversion differed at fractional boundaries.
The measured Ambient candidate stored a source-SQLite-derived epoch key.
Python results do not establish Rust/App memory, packaging, migration, or
power-loss behavior.

The later [initial native qualification](https://github.com/shm11C3/HardwareVisualizer/blob/1ef7751a0cc7e0ab84f58884103402c5023a9c6c/docs/development/hardware-archive-duckdb-initial-qualification.md)
proved exact storage round trips for two exceptional-value representations,
but did not implement their query semantics. Thirty native transactions, each
shaped as one minute batch, pinned-reader isolation and bounded process-crash
cases passed. A separate retention probe removed 13,322 eligible rows, while checkpointing
increased the database by 512 KiB; sustained reuse and compaction remain open.

## Decision

1. Select **native DuckDB with one authoritative database after verified
   conversion** as the recommended adoption direction to qualify. Keep
   production SQLite authoritative until every adoption gate passes. Use
   Parquet plus DuckDB as the smaller-file comparison candidate and keep the
   SQLite chunk design as an alternative if native qualification fails. This
   does not accept a production format.
2. Reopen ADR 0019's **SQLite-only, custom chunk/tail, and whole-chunk expiry
   implementation choices**. They are no longer required shapes for the next
   prototype. Native tables may use engine-managed columnar storage; a
   checkpoint is not an application chunk or a visibility interval.
3. Preserve ADR 0019's product guarantees: Process Stats is mandatory; stored
   observations, IDs, timestamp bytes, nulls, multiplicity, attribution,
   statistics, user-selected Retention Periods, and existing summaries,
   baselines, Storage Health and unconverted data survive. Query arithmetic
   has a separate tolerance; it cannot excuse changed membership or weights.
   Retention may remove no unexpired sample, must follow successful required
   rollups, and must preserve independent lifetimes and deletion preferences.
4. Keep production SQLite recording and reads unchanged until explicit format
   acceptance and the complete delivery gate. Qualification may refuse an
   unsupported source while keeping it usable; it must not cast, drop, or
   silently exclude exceptional records to make a benchmark pass.
5. Qualify the selected single-database direction before production adoption:
   mutable-table semantics and all readers/writers must be covered. Do not
   claim transactions across independent databases.
6. Preserve migration authority rules: source remains authoritative through
   verified copying/catch-up; activation requires a durable destination and
   explicit generation selection. After destination writes may exist, a
   failed open must not silently select an old recovery copy. A backup is not
   a current downgrade path. Later, cancellation, readable partial results,
   and explicit recovery-copy cleanup remain required.
7. Keep Core/App/frontend ownership unchanged. Core owns database behavior,
   migration execution, and queries; App owns paths, ordered schema definitions,
   lifecycle and typed IPC; frontend does not branch by storage engine.

[ADR 0019](0019-lossless-chunked-hardware-archive.md) remains authoritative for
its retained product guarantees. This record takes precedence over only the
engine/chunk/expiry-shape clauses identified above. [ADR 0021](0021-hardware-archive-migration-lifecycle.md)
remains a proposed SQLite-specific protocol; its transaction, trigger,
checkpoint, integrity, sequence and control-file assumptions are not accepted
for DuckDB by analogy. The [qualification design](../development/hardware-archive-duckdb-qualification.md)
and revised [delivery plan](../development/hardware-archive-implementation-plan.md)
own the next evidence gates.

## Alternatives and consequences

| Alternative evaluated | Evidence and trade-off | Decision |
| --- | --- | --- |
| Existing SQLite rows | Preserves current behavior and transaction ownership, but the measured one-year Process/Ambient fixture occupied 883.348 MiB and its half-range Process p95 was 1,743.515 ms in the engine comparison. | Remains production authority until another format passes every gate. |
| [SQLite custom raw chunks](https://github.com/shm11C3/HardwareVisualizer/blob/713add956faaca2a8d43dec8cf5c47cd43d30ebf/docs/development/hardware-archive-g1-benchmark.md) | Row/no-compression, row/Deflate, columnar raw and columnar/Deflate layouts plus 15/60/240-minute windows were measured. Sixty-minute columnar/Deflate was smallest at 0.301 MiB for 24 hours, but its full-decode/full-scan queries failed the 30-day and one-year comparison. | Retained fallback, not selected. |
| [SQLite chunk accelerators](https://github.com/shm11C3/HardwareVisualizer/blob/75db64c972b39d94205800cafb6ccb15d6d6029a/docs/development/hardware-archive-g1-query-experiment.md) | Ambient bounds helped narrow reads. Unconditional Process summaries expanded under high cardinality, and merging binary64 sums failed the accepted arithmetic tolerance. | Rejected as measured; a different selection and exact arithmetic design would need new evidence. |
| Native DuckDB | Lower measured one-year query latency than the other engine candidates, with larger files than Parquet, higher measured Python query-stage RSS than SQLite, and an engine-local transaction boundary. Exceptional-value query behavior, reclamation, full schema, Rust integration and recovery remain open. | **Selected direction for qualification with one authoritative database.** |
| Parquet queried by DuckDB | Smallest static files in the engine comparison. Durable publication, manifest/generation switching and pinned-reader recovery were not implemented. | Retain as capacity-oriented comparison, not selected. |
| Permanent SQLite/native split | Could avoid converting some mutable tables, but no cross-engine snapshot or transaction was measured; rollup, retention, recovery and authority would span files. | Unmeasured and not selected; requires a separate ADR-level trade-off if reconsidered. |

The representation inside native DuckDB remains conditional. Both fully tagged
rows and typed rows with an exceptional-cell sidecar preserved the tested
stored values. At a synthetic 1% exception rate, SQLite used 1,454,080 bytes,
tagged DuckDB used 1,585,152 bytes, and typed plus sidecar used 1,847,296 bytes.
That storage-only probe did not implement exceptional-value queries, so this
ADR selects neither representation.

This change does not permit lossy rollups, quantization, reduced retention,
outbound telemetry, a server dependency, or automatic migration enablement.

## Acceptance boundary

The next decision requires evidence for exact storage and endpoint semantics,
production-shaped minute writes and shutdown/restart, query cancellation and
memory, retention/reclamation, complete schema/mutable-table migration, and
Rust/native packaging and file-version compatibility on supported systems.
Test localized read failures with usable remainder versus whole-database
recovery errors, and test application crashes separately from OS/power failures
under the proposed native durability policy. Preserve the existing proposed
budgets until measurements justify a reviewed revision. Neither a configured
DuckDB memory limit nor a process-kill test proves an application memory or
power-loss guarantee.
