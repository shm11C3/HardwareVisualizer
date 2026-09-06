# Prefer Native DuckDB for Hardware Archives

Status: accepted

Tracking issue: [#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052).

This records the storage direction selected for investigation on 2026-09-06.
The application still uses SQLite; this decision is not a claim that the
replacement or migration is implemented.

## Context

Hardware Archive stores minute observations and related longer-lived data.
SQLite chunks reduced file size, but their measured long-range readers were
slow. An attempted Process summary accelerator introduced unacceptable
numerical error and excessive metadata under high cardinality.

The [engine comparison](https://github.com/shm11C3/HardwareVisualizer/blob/75db64c972b39d94205800cafb6ccb15d6d6029a/docs/development/hardware-archive-g1-engine-comparison.md)
found a better query/storage balance in native DuckDB. In its one-year stable
Process/Ambient fixture, SQLite/native/Parquet files were
883.348/204.262/83.459 MiB and half-range Process p95 was
1,743.515/59.039/89.383 ms. These are synthetic Python-client results, not
full-application capacity, memory or migration measurements.

## Decision

Use **native DuckDB with one authoritative database after conversion** as the
recommended architecture to investigate. Native tables provide columnar
storage and a shared transaction boundary for history, summaries and mutable
records. This avoids building a custom query engine over compressed blobs or
coordinating permanently split SQLite/DuckDB data.

This reopens ADR 0019's SQLite-only, custom tail/chunk and whole-chunk expiry
choices. Its lossless-history, mandatory Process Stats, retention, partial-data
and recovery constraints still explain the design. Core owns database behavior;
App owns paths, lifecycle, schema definitions and IPC; views consume domain
results. The source remains authoritative during conversion, and a stale
recovery copy cannot replace newer destination writes.

## Alternatives and trade-offs

| Alternative | Why selected or not selected |
| --- | --- |
| Existing SQLite rows | Lowest migration/integration cost and the current production implementation; it retains the measured long-history storage/query cost. |
| SQLite custom chunks | Strong measured compression, but full-decode readers missed long-range latency targets. Retain as a fallback design rather than the preferred direction. |
| SQLite chunk summaries/bounds | Ambient pruning helps narrow ranges. The tested unconditional Process summaries failed arithmetic and high-cardinality cases, so that implementation is rejected. |
| Native DuckDB | Preferred query/storage/transaction balance. Files were larger than Parquet and Python query-stage RSS higher than SQLite; value compatibility, Rust integration and maintenance still need investigation. |
| Parquet queried by DuckDB | Smallest measured static files; durable publication, generation switching and concurrent reader recovery add a separate file protocol. Keep as a comparison candidate. |
| Permanent SQLite/native split | Could reduce conversion work, but gives up a shared transaction for related records and adds snapshot/retention/recovery coordination. Not selected. |

## Consequences

Native DuckDB does not remove the work of preserving SQLite storage classes,
original timestamp semantics or all mutable and longer-lived data. The initial
value probes restored tagged and exceptional-cell representations exactly,
but did not implement equivalent exceptional-value queries. Neither
representation is selected yet. Resource probes show a larger executable,
higher clean build cost and more idle process memory than SQLite. Closing the
connection does not immediately restore the before-open RSS in the measured
window. Expiry can initially grow the native file;
later checkpoints reuse space and partially shrink it, while copying produces
a smaller file at the cost of temporary space and a safe replacement lifecycle.
These measurements support distinguishing ordinary retention from explicit
compaction. Full-application resource use and long-session maintenance remain
open. Process-kill results do not establish power-loss recovery.

The [Design Doc](../design/hardware-archive-duckdb.md) explains the
proposed structure, accumulated experiments and unresolved trade-offs.
[#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052) tracks
work and verification through its linked investigation Issues.
[ADR 0021](0021-hardware-archive-migration-lifecycle.md)
remains the historical SQLite migration proposal; its mechanics are not a
DuckDB implementation plan.
