# Lossless Chunked Hardware Archive

Status: accepted

Tracking issue: [#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052).

This records the product and architectural constraints agreed on 2026-09-01.
It is a decision for planned work, not a claim that chunk storage is implemented.
The proposed [ADR 0021](0021-hardware-archive-migration-lifecycle.md) and
[storage design](../development/hardware-archive-storage-design.md) now provide
a concrete lifecycle recommendation and decision inventory. Binary formats and
performance budgets remain subject to the design and benchmark gate.

Implementation is planned to follow the current Cooling Insight scope tracked
by [#1666](https://github.com/shm11C3/HardwareVisualizer/issues/1666). This is a
sequencing plan, not a requirement for Cooling Insight to adopt chunked storage.
Before starting #2052, refresh the table, retained-summary, baseline, and query
inventory against the then-current implementation and benchmark that baseline.

## Context

Hardware Archive rows retain approximately one-minute summaries. Repeated
timestamps, source labels, and process records make long Retention Periods an
important storage workload. Process Stats is a required optimization target,
not an optional follow-up to sensor compression; its actual contribution and
the total improvement must be measured.

The existing archive already contains summarized and ranked observations, not
all original per-second readings or a complete process audit. Storage
optimization must not reduce the information that was actually retained.

## Decision

### Keep SQLite and separate freshness from compaction

Keep the realtime collector/EventBus path independent of database work.
Persist each new archive interval in an append-oriented active tail and
periodically finalize bounded windows into immutable, independently decodable,
losslessly encoded chunks. Queries combine the tail, chunks, and any retained
legacy representation in Core. Chunk duration determines neither history
visibility nor the amount of recent history exposed to a crash.

Process Stats conversion is mandatory for the first completed delivery.
Optimize system, GPU, ambient, and fan archives wherever practical, recording
each family's conversion decision. A separate process codec is acceptable;
forcing every record into scalar sensor series is not required. Preserve
non-converted families in their existing representation rather than dropping
them. This work does not add sensor providers or change collection/ranking.

Core owns codecs, persistence, queries, migration execution, and maintenance.
App keeps database-path resolution, ordered migration definitions, Tauri
lifecycle, typed IPC, and presentation conversion. The frontend owns
interaction and rendering, not storage-format routing. This preserves
[ADR 0002](0002-core-app-split.md).

### Preserve samples exactly; assess query arithmetic separately

A lossless round trip preserves stored values, timestamp precision, nullable
fields, record presence and multiplicity, existing statistics, and attribution.
Legacy records cannot be rounded onto a regular minute grid: actual write
instants include delayed ticks and shutdown flushes. Do not narrow stored
numeric values merely to match a convenient codec or current frontend type.

Exact decoded-sample comparison is separate from the numerical criteria for
derived query results. Those criteria must be defined before replacing the
query path; tolerance must not excuse changed grouping, bucket/range boundaries,
missing-data rules, or quantization of source samples. Metadata and rollups are
accelerators, not replacements for retained samples. Partial-chunk queries
must retain their exact requested range.

Migration preserves the existing identity contract rather than repairing or
inventing history:

| Record family | Existing archived subject | Required preservation |
| --- | --- | --- |
| System | Metric and statistic columns in `DATA_ARCHIVE` | Keep average, minimum, maximum and nullability independently. |
| GPU | `gpu_name` and optional `gpu_id` | Preserve existing name-based queries and already-combined same-name rows; do not reconstruct separate devices. |
| Process | Per-observation `pid` and `process_name` | Preserve records and current tuple-based query grouping, not an inferred process lifetime. |
| Fan | `source` copied from the live fan's `name` | Keep the archived label and real 0 RPM; do not substitute the live provider `source`. |
| Ambient | Environmental Sensor Source Label in `source` | Keep source, temperature, optional humidity, and absent intervals. |

These contracts come from the [archive producer](../../core/src/persistence/archive.rs),
[stored record types](../../core/src/persistence/archive_data.rs), and
[query owner](../../core/src/infrastructure/database/archive_queries.rs).
Process queries currently average CPU/memory and take the maximum execution
seconds and latest timestamp within the range; a name dictionary cannot turn
those observations into an execution log.

GPU ids must be retained as opaque values. Current live producers use
`nvapi:<id>`, `pci:<bus>:<device>:<function>`, and
`pdh:instance:<device_instance_id>` (fallback `pdh:<luid_high>:<luid_low>`) on
[Windows](../../core/src/platform/windows/gpu.rs), `pci:<BDF>` (fallback
`drm:card<n>`) on [Linux](../../core/src/platform/linux/gpu.rs), and
`iokit:<name>` on [macOS](../../core/src/platform/macos/gpu.rs). These are not
the inventory namespace; already-written legacy forms and absent ids remain
as recorded. [ADR 0016](0016-gpu-attribution-on-the-performance-screen.md)
explains the live/archive attribution boundary. No new physical-identity join
or historical reverse translation is introduced by this decision.

### Keep recent crash loss small, not proportional to chunk size

The accepted runtime loss budget is on the order of the newest archive
interval, approximately one minute, not zero loss under every power failure.
Normal shutdown must flush pending observations and retain persisted tail
samples. Restart must recover and finalize stale incomplete windows.

Finalization and active-row removal must be atomic or recoverably equivalent,
including concurrent reads and retries: neither sample loss nor double
counting is acceptable. An interrupted finalization may revert to its active
rows; it must not lose the older samples they represent. Migration must not
turn a small recent loss window into loss of an existing archive.

The [current pool policy](../../core/src/infrastructure/database/db.rs) uses
WAL with `synchronous=NORMAL`. That setting alone does not establish a hard
one-minute bound for OS crashes or power loss; verify the write/checkpoint
strategy against the agreed target rather than asserting a guarantee from
the pragma. Application crashes and OS/power failures require distinct tests.
See [SQLite durability](https://sqlite.org/pragma.html#pragma_synchronous).

### Expire whole chunks and preserve independent retention

Delete a finalized chunk only once its latest actual sample has expired.
Accept a small extension of physical retention rather than rewriting a
boundary chunk or removing any unexpired sample. Bound elapsed chunk duration
as well as sample count, so sparse sources cannot prolong retention
indefinitely. With healthy maintenance enabled, extra retention is bounded
by that duration plus the maintenance interval.

Maintenance must also run in long-lived app sessions and preserve the user's
`scheduledDataDeletion` choice; disabling deletion does not disable compaction.
Required rollups must succeed before their source history is removed.
Keep Hardware Archive, Cooling summary, and Storage Health Retention Periods
separate. This refines the startup-only cleanup trigger in
[ADR 0018](0018-cooling-daily-rollup-retention.md) for the planned storage,
without changing its independent summary-retention decision or
[ADR 0004](0004-separate-storage-health-history.md).

### Render the readable remainder and report errors

For a localized read/decode failure, return usable history with error
information and render what can be read. Missing readings, absent rows and
unreadable portions may all appear as chart gaps: a separate visual category
for each is not required. Report the read error separately so incomplete
coverage is not silently represented as a successful, complete result.

Do not replace unavailable values with zero, interpolate through an unreadable
chunk, or automatically delete its payload. Format/codec versions and
corruption detection must prevent unrecognized or damaged payloads from being
treated as valid readings. A whole-database failure still needs a recovery
error; partial results are only possible when some history remains readable.

### Migrate explicitly and preserve the whole database

Offer optimization now or later and a settings entry point to return to it.
Prefer conversion into a separate temporary database, validation, and a
recoverable cutover while retaining the source as a recovery copy. Support
years of existing data without waiting for default retention to expire.

A whole-database replacement must preserve non-converted tables, Storage
Health history, Cooling baseline data, and retained summaries whose source
minute rows have already expired. Their schema need not be redesigned.
Aggregate equality is not enough to validate migration: compare decoded
samples against source records as well as coverage and database integrity.

Check temporary capacity conservatively, stream conversion, and handle disk
exhaustion and interruption without sacrificing original history. Account for
[SQLite WAL state](https://sqlite.org/wal.html#the_wal_file). Define consistent
source capture, concurrent writes, reader/writer shutdown, restart selection,
and every cutover failure state before implementing file replacement. Merely
renaming two files does not define recovery. Backup deletion needs a verified
destination and an explicit policy.

## Alternatives and consequences

- Lossy rollups, quantization, and shortened retention violate the information
  and user-intent contract. Query-only rollups remain useful.
- RAM-only tails or hour-sized uncommitted writes make the loss/visibility
  window too large. Persisting the active tail costs some temporary row overhead.
- Whole-chunk deletion trades a bounded amount of extra retained data for
  immutable payloads and cheaper maintenance.
- Independent chunks add metadata and bounded random-access decode cost.
  Measure total storage, CPU, and query behavior before selecting a format.
- Copy/validate/switch migration needs extra disk space and recovery states,
  but protects years of history from a failed conversion.
- Process-specific encoding is allowed because process observations have a
  different shape; shared storage infrastructure must not erase that meaning.
- Partial display preserves useful results while an error explains unavailable
  portions, without requiring a more complicated chart-gap vocabulary.

## Deferred decisions

The proposed [ADR 0021](0021-hardware-archive-migration-lifecycle.md) recommends
Later behavior, migration-time writes, capture/resume, generation selection,
maintenance, and recovery-copy removal. Its
[storage design](../development/hardware-archive-storage-design.md) records
comparison criteria and candidate budgets; the
[implementation plan](../development/hardware-archive-implementation-plan.md)
assigns their measurement and validation gates. Until that proposal is accepted,
these recommendations do not replace this ADR's accepted constraints. Codecs,
chunk sizes, and calibrated performance thresholds still require evidence.

Downgrade support remains on hold. An option to evaluate is an on-demand export
into a separate legacy-compatible DB while preserving v2 as authoritative.
This can include compatible post-migration observations without permanent
dual writes. It is not an accepted implementation requirement: supported old
schemas, unsupported metrics, extra space, safe file selection, and records
created during subsequent old-version use all need a policy. A stale migration
backup is not a lossless downgrade of current history, and automatic merging
of old-version additions is not promised.

The accepted constraints above can guide implementation planning without
silently settling these open choices or claiming the new behavior is shipped.
