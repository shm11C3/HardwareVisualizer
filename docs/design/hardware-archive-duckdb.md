# Native DuckDB Hardware Archive Design

Status: recommended design under
[ADR 0022](../adr/0022-prioritize-native-duckdb-archive-qualification.md).
Parent: [#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052).

This document explains why native DuckDB is the recommended direction and how
it would fit the application. The proposed storage is not implemented yet.
[ADR 0019](../adr/0019-lossless-chunked-hardware-archive.md) owns the existing
information-preservation, identity, retention, and recovery constraints.

## Problem and current architecture

Hardware Archive keeps minute-level Process, system, GPU, ambient, and fan
observations. Cooling summaries and baselines and Storage Health records have
their own lifetimes and include mutable rows that can outlive raw history. Long
Retention Periods make the existing row-oriented database large, and long-range
Process and Ambient reads become expensive.

The application currently has one SQLite database. App resolves
`hv-database.db`, supplies ordered schema migrations, and starts database work
before collectors. Core fixes that path once, opens fresh SQLx pools for
operations, and owns persistence and queries. SQLite runs in WAL mode with
`synchronous=NORMAL`. The frontend receives domain results and has no
storage-format routing.

~~~mermaid
flowchart LR
  Collectors[Collectors and EventBus] --> Core[Core persistence workers]
  App[App path, schema, lifecycle] --> Core
  Core --> SQLite[(SQLite: all current tables)]
  SQLite --> Queries[Core archive and insight queries]
  Queries --> IPC[Typed App IPC]
  IPC --> UI[Frontend]
~~~

This layout gives related archive, Cooling, and Storage Health work one
database-local transaction boundary. Replacing only the largest raw tables
with a second permanent database would split that boundary and add authority,
snapshot, rollup, retention, and recovery coordination.

## Experimental evidence and alternatives

The experiments are synthetic and use only their declared Process and Ambient
fixtures. Rust/SQLx produced the SQLite chunk experiments; Python bindings
produced the SQLite/DuckDB/Parquet engine comparison and initial native
qualification. Timings are comparable within each experiment, not across the
two harnesses.

| Candidate | Evidence and trade-off | Design conclusion |
| --- | --- | --- |
| Existing SQLite rows | The relational oracle preserves current behavior and transaction ownership. In the [engine comparison](https://github.com/shm11C3/HardwareVisualizer/blob/75db64c972b39d94205800cafb6ccb15d6d6029a/docs/development/hardware-archive-g1-engine-comparison.md), the one-year two-family fixture used 883.348 MiB and half-range Process p95 was 1,743.515 ms. | It remains the current implementation and migration source. |
| SQLite custom chunks with raw reads | The [initial G1 benchmark](https://github.com/shm11C3/HardwareVisualizer/blob/713add956faaca2a8d43dec8cf5c47cd43d30ebf/docs/development/hardware-archive-g1-benchmark.md) compared row/no compression, row/Deflate, columnar raw and columnar/Deflate, and 15-, 60-, and 240-minute windows. Sixty-minute columnar/Deflate was smallest for 24 hours at 0.301 MiB versus 2.367 MiB relational, but full Process decode and Ambient scan failed the 30-day and one-year query comparison. | A lossless size alternative whose raw query path does not solve long-range reads. |
| SQLite chunks with unconditional Process summaries | The [query experiment](https://github.com/shm11C3/HardwareVisualizer/blob/75db64c972b39d94205800cafb6ccb15d6d6029a/docs/development/hardware-archive-g1-query-experiment.md) used indexed per-chunk summaries, raw boundary chunks, and ranked paging. Merged binary64 sums returned 1024.0 where SQLite returned 1024.5. In the one-minute tuple-lifetime stress case, 647,535 summaries represented 647,535 finalized rows, metadata expanded a 9.727 MiB chunk DB to 70.969 MiB, and both Process ranges failed. | The measured arithmetic and unconditional high-cardinality shape are rejected. |
| SQLite chunks with Ambient bounds | Indexed chunk bounds cut one-year middle-24-hour decoding from 955,580 to 2,727 rows and produced 16.903-17.617 ms p95. Wide ranges still decoded many rows, and two measured half-range cases missed their ceilings. | Range pruning remains useful, not a complete archive design. |
| Native DuckDB | In the engine comparison, one-year storage was 204.262 MiB and half-range Process p95 was 59.039 ms. It was larger than Parquet and used more measured Python query-stage RSS than SQLite, but had faster measured native queries and an engine-local transaction boundary. | Recommended direction for the next implementation prototype. |
| Parquet queried by DuckDB | One-year storage was smallest at 83.459 MiB and half-range Process p95 was 89.383 ms. The experiment did not implement durable publication, manifest authority, concurrent replacement, or pinned-reader recovery. | Useful capacity comparison; its lifecycle is more complex than a native database. |
| Permanent SQLite/native split | No end-to-end prototype measured this topology. It could avoid converting some mutable tables, while related reads, rollups, cutoffs, and recovery would cross engines without a shared transaction. | Not recommended without a new architectural reason and evidence. |

Within the one-year stable engine comparison, whole-process query-stage RSS was
55.594 MiB for SQLite, 186.969 MiB for native DuckDB, and 127.688 MiB for
Parquet. Every Python child imported DuckDB, so subtraction does not estimate
Rust application overhead. DuckDB's configured memory limit is not a
whole-process limit. The native fixture also omitted production indexes and
constraints.

These results do not cover a complete production database, ten-year history,
controlled cold caches, Rust IPC, supported-platform packages, or power-loss
recovery.

## Recommended design

After successful conversion, one native DuckDB file becomes authoritative for
the complete application database. Raw archive families, retained summaries,
both Cooling baselines, Storage Health, schema metadata, and other persisted
objects remain together. This keeps database-local transactions available for
relationships that currently share SQLite transactions.

SQLite remains the current live implementation while this design is
unimplemented. During a future conversion it remains authoritative until a
verified DuckDB generation is durably selected. This is a migration state,
not a permanent dual-engine runtime.

~~~mermaid
flowchart LR
  Collectors[Collectors and EventBus] --> Owner[Core database owner]
  App[App paths, schema definitions, lifecycle] --> Owner
  Owner --> DuckDB[(Authoritative DuckDB)]
  DuckDB --> Queries[Core queries and maintenance]
  Queries --> IPC[Typed App IPC]
  IPC --> UI[Frontend]

  Source[(SQLite source)] -. copy and reconcile .-> Candidate[(DuckDB candidate)]
  Candidate -. verified generation selection .-> DuckDB
  Source -. retained recovery copy .-> Recovery[Explicit recovery handling]
~~~

Core continues to own engine access, persistence behavior, queries, migration
execution, and maintenance. Because duckdb-rs is synchronous, the likely Core
shape is a dedicated blocking database owner with bounded requests and a
cancellation handle. App continues to own path resolution, ordered
engine-specific schema definitions, startup and recovery lifecycle, and typed
IPC. The frontend continues to consume domain data and incomplete-coverage
information without knowing the engine.

The initial Rust candidate is [`duckdb ~1.10505.0` with `bundled`](https://docs.rs/crate/duckdb/1.10505.0/source/README.md),
matching measured DuckDB 1.5.5. Bundling avoids a separately installed runtime but adds a C++
build and native package weight not measured in the application. Storage
[compatibility](https://duckdb.org/docs/current/internals/storage) is separate
from the crate version; `v1.0.0` is an initial candidate, not a downgrade promise.

## Stored values and query meaning

DuckDB's typed columns are attractive for compression and native aggregation,
but SQLite columns can contain values outside their declared affinity. The
migration source is what SQLite returns as stored: storage class, signed i64,
binary64 bits, TEXT or BLOB bytes, nullness, IDs, and multiplicity. The current
live Rust model is not a lossless migration transport.

The [initial native qualification](https://github.com/shm11C3/HardwareVisualizer/blob/1ef7751a0cc7e0ab84f58884103402c5023a9c6c/docs/development/hardware-archive-duckdb-initial-qualification.md)
reopened every tested value exactly with a fully tagged row envelope and with
typed normal rows plus an exceptional-cell sidecar. For 20,000 synthetic
records with 1% exceptional rows, SQLite used 1,454,080 bytes, tagged DuckDB
used 1,585,152 bytes, and typed plus sidecar used 1,847,296 bytes.

That experiment tested storage, not exceptional-value queries. The sidecar
replaced exceptional projected cells with NULL, and no tag-aware evaluator was
implemented. Tagged versus typed-plus-sidecar therefore remains open. A source
that cannot yet be represented and queried honestly stays on SQLite before
selection instead of being cast or omitted.

Original timestamp bytes remain stored. Native timestamp parsing cannot replace
current membership rules: a fractional-boundary probe differed between SQLite
and DuckDB. The engine matrix used an epoch-millisecond key computed by source
SQLite for Ambient filtering. That adapter is plausible, but Process ranges,
Cooling half-open pairing, offsets, fractions, invalid text, and bucket
boundaries still need endpoint-specific treatment.

Process queries continue to group the recorded `(pid, process_name)` tuple; no
process lifetime is invented. GPU IDs remain opaque archive values rather than
inventory joins. Nulls, missing intervals, duplicate timestamps, source labels,
counts, weights, maxima, and ranking retain their domain meaning. ADR 0019 and
the current query owners define those contracts.

## Writes, snapshots, and identifiers

A bounded lifecycle probe completed 30 native transactions, each containing a
simulated minute of archive observations, while a pinned reader stayed on one snapshot. Reopen found the
expected 100,450 Process and 10,052 Ambient rows. Committed and in-flight
SIGKILL cases and allocation after deleting the highest committed IDs behaved
as expected.

This was 30 batches rather than 30 minutes of continuous monitoring. It also
combined Process and Ambient rows, whereas the current writer commits those
families separately. It does not decide whether a new writer retains that
boundary or strengthens it. Imported sequence state, cancellation, sustained
concurrency, production latency, and OS or power failure behavior remain
unknown.

The proposed database owner centralizes connection and snapshot lifetime.
Long Process results still need bounded pages and cancellation; native
vectorized execution alone does not bound IPC output or client memory.
Independent DuckDB instances use distinct spill directories because the
earlier shared-directory harness reproduced an upstream collision.

## Retention and physical storage

Logical retention remains separate for Hardware Archive, Cooling summaries,
and Storage Health. Cooling summaries and baselines can outlive raw rows,
and baseline protection affects which inputs expire. Deletion follows those
feature-owned relationships; `scheduledDataDeletion` remains the user's control
over scheduled deletion.

The initial native retention fixture removed exactly 13,322 eligible rows and
preserved every survivor after reopen. After checkpoint, the database grew from
1,585,152 to 2,109,440 bytes, an increase of 512 KiB, while its WAL returned to
zero. This demonstrates logical deletion for that fixture, not physical
reclamation or later reuse.

DuckDB may reuse internal space, while [complete compaction can require copying
to another database](https://duckdb.org/docs/current/operations_manual/footprint_of_duckdb/reclaiming_space). Internal reuse versus periodic copy-compaction remains
open because a copy adds peak disk use, cancellation, authority, and recovery
states. Checkpoint duration and compaction are maintenance concerns; neither
defines archive visibility or the recent-loss interval.

## Migration and authority

A future migration copies into a separate DuckDB candidate while SQLite remains
the source of truth. Append-only prefixes can be copied in bounded ranges;
mutable summaries, baselines, Storage Health records, deletions, and key changes
need reconciliation from a consistent source view. SQLite schema SQL and
`_sqlx_migrations` metadata need explicit DuckDB equivalents rather than replay.

Near cutover, the process reconciles remaining changes, closes and reopens the
candidate, and records an explicit authoritative generation after the candidate
is durable and verified. The selector and two database files cannot share one
transaction, so interruption order is part of the design. Once new DuckDB
writes exist, failure to open it cannot silently revive an older SQLite copy as
current data.

The SQLite source is retained as a recovery copy through a subsequent verified
startup. Removing it is a separate explicit operation. A localized readable
failure can still return usable history with incomplete-coverage information;
a failure that prevents safe database access enters file-preserving recovery.
The fault-isolation boundary for a native file remains unresolved.

## Remaining design questions

- [#2083](https://github.com/shm11C3/HardwareVisualizer/issues/2083) covers
  tagged versus typed-plus-sidecar storage, exceptional-value evaluation,
  complete Process fields, timestamp predicate adapters, and large-history
  query behavior.
- [#2084](https://github.com/shm11C3/HardwareVisualizer/issues/2084) covers the
  production transaction boundary, connection and cancellation model,
  checkpoint contention, sustained delete/reuse, possible copy-compaction,
  and crash or power-loss recovery.
- [#2085](https://github.com/shm11C3/HardwareVisualizer/issues/2085) covers all
  current schema objects and consumers, mutable-table semantics, migration
  metadata, file-version policy, supported Windows/Linux/macOS packaging,
  application memory, and durable authority selection.
