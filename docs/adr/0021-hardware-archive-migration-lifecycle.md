# Hardware Archive Migration Lifecycle

Status: proposed

Tracking issue: [#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052).

This is the recommended implementation design for review, dated 2026-09-05.
It refines [ADR 0019](0019-lossless-chunked-hardware-archive.md), whose accepted
information-preservation requirements remain unchanged. Acceptance of this
record would not mean the storage format has shipped.

## Context

ADR 0019 leaves migration-time writes, recovery, query arithmetic, maintenance,
and format selection open. Those decisions must precede implementation issues.
The current database has append-only minute archives, mutable daily summaries
and Storage Health records, and baselines that cannot be rebuilt from expired
minute rows. One timestamp cursor cannot preserve all of them.

Core opens a fresh pool for each operation and fixes its path once at startup.
Safe online switching requires ownership of all database access, including
reads, rollups, and Storage Health writes.

## Recommended decision

### Use family-specific chunks behind one Core read boundary

Persist minute observations immediately, then atomically replace selected tail
records with immutable chunks in the same SQLite transaction. Use family-specific
record batches with local dictionaries; retain original row identity, SQLite
values, timestamps, and duplicates. Process Stats is the first required complete
path. Keep Cooling summaries, both baselines, Storage Health, and unconverted
families relational.

The preferred format experiment is columnar record batches bounded by one hour,
row count, and decoded bytes. Compare per-series sensor batches and larger
bounded windows before freezing bytes. No codec, compression library, or size
saving is accepted without the [measurement gate](../architecture/hardware-archive-storage-design.md#measurement-gate).
This selects storage boundaries without inventing benchmark results.

Core reads tail and chunks in one generation and SQLite snapshot. Preserve
range membership, weighting, attribution, and null semantics. Add explicit
incomplete-coverage information to typed results; page Process Insight results
without dropping groups to bound IPC and memory.

### Keep the source authoritative until a verified generation switch

Build a separate destination database in the same app-data directory. Normal
reads and writes continue against the source. Suspend raw archive deletion
while copying immutable, primary-key-bounded prefixes. Transactionally track
changed keys for mutable preserved tables and reconcile current rows/deletions
into the destination. This is temporary migration bookkeeping.

Persist and validate progress per batch. Cancellation abandons the candidate
and releases the source maintenance guard; a later attempt starts again.
Crash recovery can resume only if the source generation, schema, capture guard,
and validated destination checkpoints still agree.

A Core database owner leases access to the active generation. At cutover it
drains leases and final archive writes, reconciles final changes, and durably
seals the verified destination before committing the selected generation in a
small local control database. App resolves paths and owns Tauri lifecycle;
Core owns the protocol. No data-file swap is required.

Control commits, source capture barriers, destination migration batches, and
sealing use an explicit durable policy for migration operations. Normal archive
writes keep WAL / `synchronous=NORMAL`.
Ordered sealing and selection must be tested on every supported OS; they are
not one atomic transaction across databases.

Before selection commits, the source is authoritative. Afterward, the
destination is authoritative even if opening it fails. Never silently return
to an older backup once destination writes may exist. Preserve both generations
and explain the selected database's failure.

### Make deferral, cleanup, and failure visible

Offer **Optimize now / Later**. Later leaves legacy queries and recording active,
starts no conversion, and suppresses repeated prompts for that session.
Settings retains a return entry point. Show phases, validated progress,
temporary disk needs, cancellation, and the short final recording pause.

Run bounded maintenance in long sessions. Compact independently of
`scheduledDataDeletion`; delete only fully expired chunks after required
rollups succeed. Preserve independent Retention Periods and baseline-protected
rows. Report failed or overdue maintenance rather than claiming a retention
bound that is not being met.

Keep the source recovery copy until migration validation and a subsequent
startup verification succeed. Then offer explicit **Remove recovery copy**,
showing size and date. Do not delete it on a timer. Report active-database
savings separately from the copy's disk usage. This is not a current downgrade
or export facility.

## Alternatives and consequences

| Alternative | Why it is not recommended |
| --- | --- |
| Offline conversion | Multi-GB histories would create a migration-long archive gap. |
| One long read snapshot | It can pin source WAL throughout conversion. Short prefix reads plus mutable-key reconciliation avoid that lifetime. |
| Destructive in-place rewrite | Failed conversion threatens the only copy of years of history. |
| Copy only archive tables | Retained summaries, baselines, and concurrent Storage Health changes would be lost. |
| Permanent dual writes or a generic replication framework | Neither is needed after conversion; both add continuing consistency cost. |
| Two renames or automatic fallback to an old DB | Neither identifies authority after interruption or new destination writes. |
| Source plus a full snapshot plus destination | A simpler comparison baseline, but requires another whole-DB copy. Revisit if the selected capture protocol cannot pass correctness tests. |
| Automatic recovery-copy expiry | Elapsed time does not prove recovery or user acceptance of removing the copy. |

The access owner, temporary capture, durable selector, and coverage-aware query
results are required by online conversion and honest partial reads. This work
does not change collection/ranking, add analytics, or implement downgrade.

## Decision and delivery gates

The [storage design](../architecture/hardware-archive-storage-design.md) specifies
capture, comparison rules, candidate budgets, and recovery tests. The
[implementation slices](../development/hardware-archive-implementation-plan.md)
assign remaining decisions and dependencies.

Lifecycle acceptance is separate from byte-format approval. Format adoption
requires measured evidence; product guarantees cannot be relaxed to make a
candidate pass. Revalidate the schema and query inventory after the current
scope of #1666 before implementation.

## References

- [Current database access](../../core/src/infrastructure/database/db.rs)
- [Migration definitions](../../src-tauri/src/infrastructure/database/migration.rs)
- [SQLite WAL concurrency](https://sqlite.org/wal.html#concurrency)
- [SQLite durability policies](https://sqlite.org/pragma.html#pragma_synchronous)
