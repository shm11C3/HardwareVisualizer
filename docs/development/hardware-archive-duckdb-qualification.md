# Native DuckDB Hardware Archive Qualification

Status: active investigation under [ADR 0022](../adr/0022-prioritize-native-duckdb-archive-qualification.md).
Parent: [#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052).
This is the current design and adoption-gate inventory; production still uses
SQLite. The [SQLite chunk candidate](hardware-archive-storage-design.md) and
[ADR 0021](../adr/0021-hardware-archive-migration-lifecycle.md) remain alternatives.

## Claim and evidence

Reduce local Hardware Archive storage and long-range query cost while retaining
all stored information and monitoring during migration. This follows DP-02,
DP-04, DP-05, DP-06, DP-07 and DP-09. The [engine comparison](https://github.com/shm11C3/HardwareVisualizer/blob/75db64c972b39d94205800cafb6ccb15d6d6029a/docs/development/hardware-archive-g1-engine-comparison.md)
uses SQLite 3.46.0, DuckDB 1.5.5 and Python clients. It proves promising
synthetic behavior, not a Rust/App replacement. Keep its raw artifact and
negative probes; do not relabel them as a completed production gate.

The inventory baseline is develop `838d08da16d60ead84df3ddb1501ca19a57f24fc`,
including the merged Cooling correlation work from #2070. #1666 remains open;
its open state does not forbid isolated research or establish that its
current scope is finished. Recheck schema and query changes before format
adoption and each application integration slice.

## Accumulated alternatives and selection

The selected adoption direction to qualify is **native DuckDB as one
authoritative database after verified conversion**. Production remains on the
existing SQLite database until all value, query, lifecycle, migration, Rust,
platform and recovery gates pass. This selection does not choose an
exceptional-value representation or accept a production format.

| Candidate | What was tested | Measured benefit or cost | Current decision |
| --- | --- | --- | --- |
| Existing SQLite rows | Original SQLite storage and query paths are the relational oracle in every experiment. | It retains current semantics and transaction ownership. In the engine comparison, the one-year two-family fixture used 883.348 MiB and half-range Process p95 was 1,743.515 ms. | Keep as production authority and recovery source until adoption completes. |
| SQLite custom chunks, raw query path | The [initial G1 benchmark](https://github.com/shm11C3/HardwareVisualizer/blob/713add956faaca2a8d43dec8cf5c47cd43d30ebf/docs/development/hardware-archive-g1-benchmark.md) compared row/no compression, row/Deflate, columnar raw and columnar/Deflate, plus 15-, 60- and 240-minute windows. | Sixty-minute columnar/Deflate was smallest for 24 hours at 0.301 MiB versus 2.367 MiB relational. Full Process decode and Ambient scan failed the proposed 30-day and one-year query ceilings. | Useful lossless size fallback; raw scans do not satisfy the long-range query gate. |
| SQLite chunks with unconditional Process summaries | The [query-strategy experiment](https://github.com/shm11C3/HardwareVisualizer/blob/75db64c972b39d94205800cafb6ccb15d6d6029a/docs/development/hardware-archive-g1-query-experiment.md) measured indexed per-chunk summary/metadata tables, raw boundary chunks and ranked paging. | Fast on ordinary cases, but merged binary64 sums produced 1024.0 versus SQLite 1024.5. At one-minute tuple lifetime, 647,535 summaries represented 647,535 finalized rows, metadata expanded the 9.727 MiB chunk DB to 70.969 MiB (83.547 MiB relational), and both measured Process ranges failed. | Reject this arithmetic and unconditional high-cardinality shape. A replacement needs a new exact accumulation and selection proof. |
| SQLite chunks with Ambient bounds | The same query experiment indexed per-chunk timestamp bounds and decoded candidate chunks before applying the original SQLite predicate. | One-year middle-24-hour decoding fell from 955,580 to 2,727 rows and p95 to 16.903–17.617 ms. Wide ranges still decode many rows; one-year stable half-range and 30-day one-minute-churn half-range missed their ceilings. | Retain range pruning as a possible building block, not a complete storage/query choice. |
| Native DuckDB | The [engine comparison](https://github.com/shm11C3/HardwareVisualizer/blob/75db64c972b39d94205800cafb6ccb15d6d6029a/docs/development/hardware-archive-g1-engine-comparison.md) tested the same generated rows and ranges after exact reopen. The [initial qualification](https://github.com/shm11C3/HardwareVisualizer/blob/1ef7751a0cc7e0ab84f58884103402c5023a9c6c/docs/development/hardware-archive-duckdb-initial-qualification.md) added exceptional-value storage and bounded lifecycle probes. | One-year storage was 204.262 MiB and half-range Process p95 59.039 ms. Ordinary typed rows passed; fixed typing failed mixed classes. Both later exact representations passed storage round trip, but exceptional queries remain unimplemented. Thirty native transactions, each shaped as one minute batch, and bounded crash/snapshot checks passed. | **Selected next qualification direction**, with one authoritative native database only after every gate passes. |
| Parquet queried by DuckDB | The engine comparison queried two immutable Parquet files through DuckDB. | One-year storage was 83.459 MiB and half-range Process p95 89.383 ms. Publication, manifest authority, concurrent replacement and reader recovery were not implemented. | Capacity-oriented comparison; smaller static files do not outweigh the unproven lifecycle. |
| Permanent SQLite/native split | No end-to-end prototype or benchmark exists. | Avoids some conversion, but mutable summaries/baselines and Storage Health would need coherent snapshots, rollup/retention ordering and recovery across engines without a shared transaction. | Not selected. Reconsider only with a separate explicit trade-off and evidence. |

Within the one-year stable **engine comparison**, SQLite/native/Parquet query-stage
whole-process RSS was 55.594/186.969/127.688 MiB. Native trades more measured
working memory and disk than Parquet for faster Process queries in that matrix
and a database-local write/read transaction boundary. These Python processes
all import DuckDB, even the SQLite baseline; subtraction does not estimate
added application RAM. The native static fixture lacks production keys/indexes,
so its 204.262 MiB does not include their required cost.

The initial/query-strategy experiments use Rust/SQLx, while the engine matrix
uses Python bindings. Compare query timings within each experiment, not as a
single cross-harness speed ranking. The measurements are synthetic and cover
only their declared fixtures. They do not compare a complete production DB,
ten-year history, controlled cold caches, Rust IPC, supported-platform packages
or power-loss recovery.

Adopt the native direction only if its query and transaction benefits survive
full-schema, exceptional-value, memory, packaging and recovery qualification.
If a gate fails, keep SQLite authoritative and revisit the measured alternatives;
a smaller Parquet file or a prior SQLite chunk result is not automatic fallback
approval. The typed-versus-tagged representation and internal-reuse-versus-copy
compaction choices remain open until their specific query and sustained-growth
measurements support a decision.

## Decisions and open gates

| Area | Next design to test | Adoption gate |
| --- | --- | --- |
| Engine | Native DuckDB first; Parquet comparison; SQLite fallback | Same-value, same-query measured benefit plus supported native integration |
| Authority topology | One native database after verified conversion | Every raw/daily/baseline/Storage Health/schema object accounted for; no permanent split assumed |
| Stored representation | Typed normal rows with explicit lossless handling for exceptional SQLite storage classes; compare tagged representation | Exact reopened values including class and bytes; exception-path storage/CPU cost and honest query behavior |
| Timestamp predicates | Preserve original bytes; materialize source-computed predicate keys where needed | Endpoint-specific SQLite membership and bucket equivalence for offsets, fractions, invalid text, duplicates and negative epochs |
| Recent writes | Bounded minute-batch native transactions, independent of checkpoint/compaction | Production-shaped writer, monotonic IDs, restart/retry, live cadence and actual visibility |
| Long queries | Native aggregation/sort with bounded fetches, explicit cancellation and snapshot lifetime | All groups accessible, ranking/tolerance preserved, measured incremental memory and spill peak |
| Retention | Bounded native row deletion after required rollups; checkpoint/reclamation measured separately | No unexpired or baseline-protected row removed; deletion preference and independent lifetimes preserved |
| Migration | Source capture/copy/reconciliation, verified destination, durable explicit selection | All schema/mutable-table mutations, interruption/cancel/restart, disk margins and authority states covered |
| Read failure | Domain-readable remainder with explicit incomplete coverage; whole-DB failures enter recovery | Fault injection proves which failures remain local and which prevent open/query; no false completeness |
| Durability | Qualify native normal writes and rare authority selection separately | Separate application-crash and OS/power-failure evidence on supported systems; checkpoint/WAL policy meets the accepted recent-loss target |
| Packaging | Isolated Rust/native prototype before application dependency | Pin client/engine and storage versions; upgrade/reopen policy, licensing, build, launch, notice/resource and size evidence on supported targets |

No conditional fallback, dual-write service, generic storage framework, new UI
preference, or production migration is introduced merely to support this
investigation. A failed candidate returns to its owning decision.

## Ownership and preserved records

Core owns engine access, value conversion, queries, ID allocation, migration
execution and maintenance. App resolves paths, provides engine-appropriate
ordered schema definitions, controls lifecycle, and exposes typed IPC.
The frontend continues to consume domain results and incomplete-coverage
information. Current `open_pool`/direct SQL call sites must be inventoried
before any active-generation switch; changing a filename alone is insufficient.

| Family | Required preservation |
| --- | --- |
| Process Stats | Original ID; `(pid, process_name)` observations; CPU/memory, execution seconds, timestamp, null/class/byte behavior and multiplicity. No invented process lifetime. |
| System/GPU | Each metric/statistic/null independently; recorded GPU name and optional opaque ID, including already-combined names. No inventory-ID join. |
| Ambient/fan | Recorded source label; original timestamp; nullable humidity, absent intervals, real zero RPM and duplicate observations. Fan source remains live fan name. |
| Cooling | Daily, fan, Thermal Delta and covariate summaries, both baselines, coverage/weighting and protected inputs; some outlive source minute rows. |
| Storage Health | Devices, activity state, history, natural keys, conflicts/upserts and independent retention. |
| Schema and other objects | Full table/object inventory, constraints, sequences/high-water marks and migration history; explicitly preserve or translate each. Unknown objects block activation while source remains usable. |

The ID contracts are sourced from the archive producer, stored-record helpers,
Process query owner and GPU attribution ADRs. Fixtures must not replace opaque
GPU IDs or group processes only by PID. This investigation adds no new entity
namespace or cross-provider join.

## Value and query qualification

The source is **what SQLite returns as stored**, including storage class,
signed i64 values, binary64 bits, TEXT/BLOB bytes, nullness, IDs and duplicates.
Current live `f32`/`i32` models are not a migration transport. A declared INTEGER
column can contain REAL, TEXT or BLOB. A native BIGINT cast can round, reject,
or reinterpret it. Invalid UTF-8 TEXT cannot be treated as an ordinary VARCHAR.

Prototype two concrete representations: a tagged exact-value envelope, and a
typed normal-row representation with explicit exceptional data. Measure their
cost on normal and exceptional fixtures, including zero and nonzero exception
rates. Exact payload round trips alone do not authorize typed projections to
exclude exceptions from groups, averages, ranks, or ranges. Until those
semantics are proven, reject activation of an unsupported source and keep it
queryable by its existing SQLite path. This is preflight refusal, not silent
loss or a permanent cross-engine query design.

The initial storage-only probe reopened every tested value exactly under both
representations. With 20,000 records and a synthetic 1% exceptional-row rate,
source SQLite used 1,454,080 bytes, fully tagged DuckDB used 1,585,152 bytes,
and typed DuckDB plus a 467-cell sidecar used 1,847,296 bytes. The sidecar made
exceptional projected cells NULL, and no tag-aware query evaluator was built.
These sizes are allocation-sensitive single-scale observations, not a query or
large-history result. Keep both representations conditional until query
semantics and representative exception rates are measured.

Keep the query contracts from the [earlier design](hardware-archive-storage-design.md#query-contract):
exact membership, buckets, source/group identity, null masks, counts and
weighting; finite derived values use `abs(actual-reference) <=
max(1e-9, 1e-12*abs(reference))` before display. Treat nonfinite/null classification
separately. Do not widen tolerance or substitute unweighted averages.
The rejected binary64 sum/count chunk accelerator remains rejected.

Native timestamp parsing is not the existing predicate. The prior matrix
stored SQLite-derived epoch milliseconds alongside raw Ambient timestamp text.
The next prototype must retain this adapter's cost and semantics, cover
fractional rounding and offsets, and inventory every other endpoint's
TEXT/inclusive/half-open/bucket predicates. One epoch key does not prove all
Cooling or Process ranges equivalent.

Native vectorized execution does not bound IPC output. Keep all Process groups
accessible in a ranked query snapshot with bounded pages, cancellation,
expiry and cleanup; perform sensor endpoint bucket reduction before crossing
IPC when required by the consumer. A memory limit controls only part of the
engine's allocation. Measure startup, idle, query, validation and migration
memory separately, and sample spill peaks instead of reading only final sizes.

Inject localized record/query failures and native-file corruption separately.
Prove when readable observations can be returned with explicit incomplete
coverage and when the engine cannot safely open/query the database. The latter
must enter file-preserving recovery, never masquerade as empty or complete
history. Native storage does not by itself prove the former custom chunk
candidate's fault-isolation behavior; this is an adoption gate.

## Writes, maintenance and recovery

Test a process-shaped minute batch and representative sensor batch, including
shutdown flushes and duplicate timestamps. Preserve the actual current writer's
atomicity before strengthening any multi-family boundary. New IDs must not
reuse expired/deleted IDs or collide after restart/retry; do not derive them
from `MAX(id)+1` without a proven serialized high-water mechanism.

Measure append while a real long read holds a pinned snapshot. Verify the
reader's before/after view and exact committed IDs after reopening. Kill the
process at committed and in-flight minute-batch boundaries; keep this distinct
from OS/power loss and cross-database migration recovery. Transactions must not
turn checkpoint duration into archive visibility or the accepted recent-loss
window. Different DuckDB database instances own distinct spill directories.
Before adoption, separately test application crashes and OS/power failures on
supported systems under an explicit native write/checkpoint/WAL durability
policy. The accepted newest-interval loss target and protection of older data
remain requirements. A successful process-kill probe leaves the OS/power gate
open; the current SQLite WAL/NORMAL setting is not a native configuration.

The bounded lifecycle probe completed 30 minute-shaped transactions while a
pinned reader remained on one snapshot, then reopened the expected 100,450
Process and 10,052 Ambient rows. This was 30 batches, not 30 minutes of
continuous monitoring, and it deliberately combined Process and Ambient where
the current writer uses separate SQLite transactions. Committed and in-flight
SIGKILL cases and deleted-highest-ID allocation passed; imported sequence state,
production latency, cancellation and OS/power failure remain open.

Delete only eligible rows after required rollups succeed, with bounded work in
long sessions. Preserve `scheduledDataDeletion`, separate Retention Periods,
and baseline protection. Native table deletion replaces whole application
chunk expiry only if its measured cost is acceptable. Report logically expired
rows, internal reusable space and filesystem bytes separately; do not assume
`DELETE`, `VACUUM`, or checkpoint reduces the file. Preserve source/recovery
copies while any replacement/reclamation operation is unverified.

The initial retention fixture removed exactly 13,322 eligible records and
preserved every survivor after reopen. The checkpointed database then grew from
1,585,152 to 2,109,440 bytes, an increase of 512 KiB, while the WAL returned to
zero. This proves the tested logical predicate, not physical reclamation or
future reuse. Selection between accepting internal reuse and performing a
full-copy compaction remains conditional on sustained-growth, temporary-space,
cancellation and recovery evidence.

Prefer a single native authoritative database after conversion because raw
history, summaries/baselines and Storage Health are related transactional data.
This does not prove conversion is feasible. A retained SQLite/native split
would require coherent generation snapshots, rollup/cutoff ordering and
recovery across files, and needs a new explicit trade-off if selected.

For migration, preserve the source until a verified destination is selected.
Capture append-only prefixes and every mutable upsert/delete/key change; prove
idempotent catch-up, validation progress, cancellation and crash resume. Source
SQLite triggers may assist capture, but cannot be assumed to execute in DuckDB.
Translate schema/migration metadata explicitly; never replay arbitrary SQLite
DDL as DuckDB SQL. The destination must be durable before selector commit;
the selector protocol has a separate engine/format decision and must be tested
on each OS. A native data transaction does not span the selector or source DB.

Before selection, errors return to the source. After selection, failed open or
post-selection writes must not silently fall back to an older source copy.
Preserve files and expose retry, live-only continuation or exit. Keep Later,
cancel/retry, short recording pause and explicit verified recovery-copy removal
in the complete delivery scope; no production Optimize now entry appears until
all gates pass.

## Rust and file compatibility investigation

The initial isolated Rust candidate is `duckdb = "~1.10505.0"` with `bundled`,
matching the measured DuckDB 1.5.5. Pin the resolved version in the prototype's
lockfile. The upstream [Rust binding README](https://docs.rs/crate/duckdb/1.10505.0/source/README.md)
explains the version mapping and bundled C++ build. Measure build time and
package growth before adding any production dependency. Keep external
extension install/autoload disabled in the application candidate; no query
requires downloading an extension or hardware data leaving the device.

Qualify blocking connection ownership and an interrupt handle within Core;
async IPC must not run the engine on a runtime worker or hold a global query
lock indefinitely. A dedicated blocking owner is a prototype option, not a
new generic storage framework. Check actual constraints, upserts, collations,
integer widths and mutable-table transactions against the full schema.

Choose a storage compatibility version separately from the client/engine pin,
record it in artifacts, and test reopen across each supported application
upgrade. A `v1.0.0` storage target is the initial compatibility candidate, not
an accepted downgrade guarantee. DuckDB documents backward compatibility goals
and best-effort forward compatibility; do not select `latest` implicitly or
claim an older app can read a newer file. Refuse unsupported versions while
preserving files. This native update policy does not promise legacy SQLite
export. See [storage versions](https://duckdb.org/docs/current/internals/storage).

Build, launch, write/query/cancel/reopen and inspect the actual distribution
on Windows x64, Linux x64, macOS arm64 and macOS x64. Include native library
loading, licensing/notices, App startup/reset and file paths in the evidence.
Static Python probes and upstream binary availability do not complete this gate.

## Measurement and adoption

Retain the [proposed budgets](hardware-archive-storage-design.md#measurement-gate):
50% Process / 30% representative total storage reduction, existing-range warm
p95 at most `max(1.2*baseline, baseline+20ms)`, append p99 below 100ms, at most
64 MiB extra steady-state and 256 MiB extra query/migration memory independent
of history, and a 5s target final pause. These remain proposed, not ratified by
Python results. Measure maintenance CPU, source growth, temporary peaks,
recovery copies and ten-year behavior. Do not loosen product guarantees to
satisfy a resource target.

The [delivery plan](hardware-archive-implementation-plan.md) links qualification
Issues and the later vertical application slices. Format acceptance requires
review of all three qualification tracks and unresolved long-history/platform
gates. Application enablement remains a later complete migration decision;
Process Stats and all preserved families are required, not optional follow-ups.
