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
qualification. Separate Rust release probes measure native process memory and
build costs. Timings are comparable within each experiment, not across
harnesses.

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
matching measured DuckDB 1.5.5. Bundling avoids a separately installed runtime.
The isolated build measurements below quantify its C++ build and executable
cost; full application packages remain unmeasured. Storage
[compatibility](https://duckdb.org/docs/current/internals/storage) is separate
from the crate version; `v1.0.0` is an initial candidate, not a downgrade promise.

## Native runtime and build costs

The [resource measurements](https://github.com/shm11C3/HardwareVisualizer/blob/58fad7ad54263079c1c2a74aa1b8396fcdcbd344/docs/development/benchmarks/hardware-archive-duckdb-resources-2026-09-06.json) use macOS arm64 on a
Mac16,1 with 24 GiB RAM, Rust 1.98.0, SQLx 0.8.6 and bundled DuckDB 1.5.5.
Three standalone executables share Tokio and the JSON measurement protocol.
Each clean build uses a separate empty target directory and two build jobs,
after dependency fetch, with the application's release profile: size
optimization, LTO, stripped symbols and one codegen unit.

| Executable | File MiB | gzip-9 MiB | Clean build s | Unchanged rebuild s | Build target allocated MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| Common Tokio/JSON baseline | 0.401 | 0.190 | 5.49 | 0.06 | 40.660 |
| SQLx SQLite | 1.800 | 0.925 | 44.30 | 0.23 | 439.148 |
| Bundled DuckDB | 24.354 | 7.805 | 273.97 | 0.20 | 686.172 |

The native executable adds 22.554 MiB over the SQLite control, and the clean
build takes about 6.2 times as long. Its dynamic dependency list contains no
separate DuckDB library. Target allocation includes dependencies and native
build outputs and excludes the generated gzip comparison. gzip sizes are
compression comparisons, not installer sizes. Each build timing is one local
observation; none estimates a full Tauri package or a Windows/Linux build.
The recorded lockfile pins the resolved dependencies; the measured commands
did not pass `--offline` or `--locked`.

Idle memory was measured from the same release executables, with no concurrent
build or retention probe. Each engine/fixture case uses three fresh processes;
each stage settles for one second, then takes six samples one second apart.
The macOS `proc_pid_rusage` current resident-size and physical-footprint fields
are sampled while the child waits for input. They are not peak RSS. The fixture
contains 100,000 normal-value Process and 10,000 Ambient rows; fixture creation
happens outside the measured child. Process and Ambient queries return 2,926
rows with matching digests across engines and repetitions, and release their
results before sampling.

The table gives the median of the three per-process medians as **RSS / physical
footprint**, in MiB. Raw samples and the range of process medians are in the
resource artifact. These are standalone process totals, not incremental app
memory.

| Stage | SQLx SQLite MiB | Bundled DuckDB MiB |
| --- | ---: | ---: |
| Before database open | 6.312 / 1.797 | 7.922 / 2.172 |
| Open seeded database | 7.594 / 2.250 | 20.922 / 6.000 |
| Idle after queries | 14.578 / 9.235 | 28.578 / 11.954 |
| Idle after connection/pool close | 14.516 / 7.235 | 28.562 / 11.735 |

The common runtime baseline stays at 1.797 MiB RSS / 1.235 MiB footprint.
Opening an empty database gives 7.578 / 2.235 MiB for SQLite and
20.984 / 6.094 MiB for DuckDB, close to the seeded-open case. Both controls use
the same ten-thread Tokio runtime. SQLite uses current SQLx features and pool
defaults with WAL/NORMAL; DuckDB uses two engine threads and a 128 MB managed
memory limit. The Ambient fixture's extra epoch-millisecond key/index is the
previous query adapter, not a column in the current production schema.

Closing the connection/pool does not return RSS to the before-open level in
this short observation. The median within-process increase after close is
8.172 MiB RSS / 5.406 MiB footprint for SQLite and 20.641 / 9.562 MiB for
DuckDB. Repeated open/close should therefore not be assumed to release process
memory immediately. The bounded Core database owner remains the recommended
shape; this evidence does not select an idle timeout or establish a memory
leak, a steady-state ceiling, or a full-application budget. Long sessions,
full-schema workloads, query peaks and migration still need application-level
measurement. The native query and storage benefits justify continuing the
prototype while accepting the measured executable, build and idle-memory
costs as tradeoffs to validate in the application.

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

The earlier small fixture removed 13,322 eligible rows, preserved every
survivor and grew by 512 KiB after checkpoint. A larger
[retention experiment](https://github.com/shm11C3/HardwareVisualizer/blob/58fad7ad54263079c1c2a74aa1b8396fcdcbd344/docs/development/benchmarks/hardware-archive-duckdb-resources-2026-09-06.json) now separates that immediate
behavior from later reuse. It starts with 630,000 Process and 72,000 Ambient
rows, deletes 80%, then runs eight append/expiry cycles at a stable 126,000 and
14,400 retained rows. Primary keys and both current timestamp indexes are
included. Every reopen compares every surviving field and ID with the expected
records; source and compact copies also pass those checks.

The table reports **database file bytes / filesystem-allocated bytes**, in MiB.
WAL is zero at these checkpoints; the complete artifact separately records WAL,
spill files and SQLite SHM, so a shrinking SHM is not mistaken for a shrinking
SQLite database.

| State | SQLite file / allocated MiB | DuckDB file / allocated MiB |
| --- | ---: | ---: |
| Before expiry | 73.824 / 73.824 | 28.262 / 29.012 |
| After 80% expiry and checkpoint | 73.824 / 73.824 | 31.512 / 32.012 |
| After eight append/expiry cycles | 73.824 / 73.824 | 18.262 / 18.262 |
| Fresh compact copy | 14.203 / 14.203 | 6.762 / 6.762 |

DuckDB initially grows despite expiry: 71 of 126 blocks are free after the
purge. Subsequent writes reuse space and checkpoints shrink the file in steps;
the eighth cycle leaves 15 free blocks of 73 total. SQLite keeps its database
allocation while reusing free pages, with 15,101 of 18,899 pages free at the end.
These bounded observations support reuse and partial shrink, not an immediate
size reduction proportional to the Retention Period or a lifetime size bound.

DuckDB's fresh copy takes 93.61 ms and the sampled combined source/output disk
allocation reaches at least 28.758 MiB. SQLite's `VACUUM INTO` takes 26.04 ms
and reaches at least 88.062 MiB. Both retain the source. These are single small
copy observations with a 2 ms sampler, not exact peak-space or application
pause budgets. The artifact preserves a prior control-affected run and the
selected run's shell bookkeeping error after complete passing probe output;
the latter did not alter the measurements or validations.

The recommended maintenance shape starts with expiry plus checkpoints and
reuse. [Copy compaction](https://duckdb.org/docs/current/operations_manual/footprint_of_duckdb/reclaiming_space)
can recover more space when an explicit optimization warrants its extra disk
use and rewrite work. It needs the same durable selection, cancellation and
recovery ownership as migration. The full-schema policy and when to offer
compaction remain open; neither compaction nor checkpoint timing defines
archive visibility or the recent-loss interval.

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
  checkpoint contention, full-schema and long-session retention, copy-compaction,
  and crash or power-loss recovery.
- [#2085](https://github.com/shm11C3/HardwareVisualizer/issues/2085) covers all
  current schema objects and consumers, mutable-table semantics, migration
  metadata, file-version policy, supported Windows/Linux/macOS packaging,
  application memory, and durable authority selection.
