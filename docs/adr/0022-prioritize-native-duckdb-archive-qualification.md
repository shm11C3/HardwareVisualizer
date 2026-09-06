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

## Decision

1. Prioritize a native DuckDB compatibility and lifecycle prototype. Use
   Parquet plus DuckDB as the smaller-file comparison candidate. Keep the
   SQLite chunk design as an alternative if qualification fails.
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
5. Prefer testing a **single authoritative native database after conversion**
   over permanent SQLite/native dual storage. This is a topology hypothesis:
   mutable-table semantics and all readers/writers must be covered before it
   can be selected. Do not claim transactions across independent databases.
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

- Native DuckDB combines measured query/storage gains with a native transaction
  boundary. Complete conversion still needs a demonstrated mapping for SQLite
  values, SQL semantics, mutable tables, identifiers and schema metadata.
- Parquet was smaller. Publishing immutable files and switching generations
  adds a file/manifest durability and reader-lifetime protocol absent from the
  static-file benchmark; those costs remain unmeasured.
- SQLite plus custom chunks retains familiar transaction ownership. It remains
  a fallback, but the measured numerical and high-cardinality summary failures
  must be resolved before selecting that accelerator.
- A permanent split database could avoid some conversions but creates snapshot,
  rollup, retention and recovery coordination across engines. It is not the
  default and requires a separate justification if native coverage fails.
- This change does not permit lossy rollups, quantization, reduced retention,
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
