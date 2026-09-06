# Native DuckDB Qualification: Initial Value and Lifecycle Probes

Status: initial measured evidence for [#2083](https://github.com/shm11C3/HardwareVisualizer/issues/2083)
and [#2084](https://github.com/shm11C3/HardwareVisualizer/issues/2084), under parent
[#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052). All remain open.
The [preceding design PR #2086](https://github.com/shm11C3/HardwareVisualizer/pull/2086)
records the accepted investigation priority. No production storage, dependency,
query path or migration is accepted or enabled here.

## Result and implication

Exact storage round trips are feasible for the tested SQLite values using
both tagged rows and typed rows with an exceptional-cell sidecar. This is
progress beyond the earlier fixed-type conversion failures, not proof that
queries over exceptional values are equivalent. Native minute-batch commits,
pinned-reader isolation, process-crash reopen and deleted-highest-ID allocation
passed the bounded lifecycle probe. Retention removed exactly eligible rows,
but the native file grew after checkpoint. Keep native DuckDB as the next
candidate; settle exceptional query semantics and reclamation before adoption.

## Provenance and reproduction

Frozen code: `0a37b01e0bcd26d26b63b36b3e53d3f18a72c897`. The [full artifact](benchmarks/hardware-archive-duckdb-qualification-2026-09-06.json)
contains both unrounded reports, exact commands/exit codes, script hashes,
retained database hashes and environment. Programs are standalone under
`core/examples/archive_engine_benchmark/`; they never open an application DB.

The final runs were sequential on the same Apple M4 macOS host as the
[engine comparison](hardware-archive-g1-engine-comparison.md), using Python
3.14.5, DuckDB 1.5.5 and pysqlite3 0.5.4 linked to SQLite 3.46.0. Small smoke
runs preceded Astra review and the measured runs. No concurrent local builds
or benchmarks ran. These are one-run diagnostic observations, not stable
performance percentiles or full-schema capacity predictions.

After preparing the isolated Python environment documented in the engine
comparison, run from the repository root with its interpreter:

```bash
python core/examples/archive_engine_benchmark/duckdb_value_preservation.py \
  --output /tmp/hv-duckdb-values --rows 20000
python core/examples/archive_engine_benchmark/duckdb_lifecycle_probe.py \
  --output /tmp/hv-duckdb-lifecycle.json --rows 100000 --repetitions 30
```

The scripts refuse other engine versions. Both exit nonzero on failed asserted
contracts. They retain synthetic databases in unique output subdirectories.
Value output uses one thread; lifecycle uses two and a 128 MB managed-memory
limit. Independent instances have distinct spill directories. The managed
limit is not a whole-process RSS bound.

## Exact value preservation

Each workload has 20,000 alternating Process/Ambient-shaped records in a
combined synthetic schema. Both representations reopen read-only before
comparison of every cell's storage class, exact payload, ordinal and original
row ID. This is not the production schema or a production migration transport.

| Exception rows | Source SQLite bytes | Tagged-row native bytes | Typed + sidecar native bytes | Exceptional cells | Exact reopen |
| --- | ---: | ---: | ---: | ---: | --- |
| 0% | 1,425,408 | 1,585,152 | 1,323,008 | 0 | Both pass |
| 1% (200 rows) | 1,454,080 | 1,585,152 | 1,847,296 | 467 | Both pass |

The tagged representation keeps one row envelope containing ordinal, storage
tag, length and exact payload. The typed representation keeps ordinary native
columns and replaces exceptional projected cells with NULL while preserving
their tag/payload in a keyed sidecar. It is invalid to query those NULL
projections as though they were the original observations. All 200 exceptional
rows affect query columns; their typed-only query path is marked disallowed.
No tag-aware SQL evaluator, production refusal path or exceptional query
contract was implemented by this storage-only prototype.

Edge probes include minimum/maximum signed i64 IDs/values, observed REAL bits,
subnormals/infinities, invalid UTF-8 TEXT, embedded NUL, identical TEXT/BLOB
bytes with distinct classes, empty values and NULL. Compare what source SQLite
actually stored: the untyped edge table retains negative zero, while an
SQLite REAL-affinity column can normalize it. No pre-storage bit guarantee is
inferred. Timestamp bytes remain untouched; predicate adapters and arithmetic
remain separate query evidence. Repeated Process tuples and Ambient sources
retain their original record multiplicity.

At this scale, adding the sidecar and its keys increases the 1% native file
above both SQLite and the fully tagged representation. File-block/index
allocation and this deliberately adversarial distribution prevent extrapolating
a real installation exception rate or large-history compression ratio. Insert
times are retained only as diagnostics: Python executemany uses default
transaction behavior, tagged timing includes CREATE and sidecar timing does
not. They are not a fair format/engine CPU or ingestion comparison.

## Native writes, snapshots and restart

Seed 100,000 Process and 10,000 Ambient rows, then execute 30 accelerated
minute-shaped commits: 15 Process records per minute, up to two Ambient sources,
nullable humidity and absent Ambient intervals. The timestamps include explicit
milliseconds. This is 30 batches, not 30 minutes of continuous monitoring.

- All 30 write transactions overlapped a measured grouped analytic query.
  The reader held one pinned transaction and retained identical counts and
  results while writes committed; the query repeats inputs eight times to
  make overlap observable.
- Reopen observed exactly 100,450 Process and 10,052 Ambient rows. Appended
  IDs and minute counts matched; seeded ID/timestamp/identity fields were
  scanned in bounded batches. This does not claim a full-field comparison of
  the 110,000 seeded lifecycle records; value round trips are a separate probe.
- Transaction median/p95/p99/max were 8.901/9.237/9.403/9.413 ms, using linear
  percentile interpolation over 30 observations. Timing includes deliberate
  reader-start synchronization; it is not production writer latency or a
  comparison against SQLite. Current production writes the two families in
  separate SQLite transactions; this candidate deliberately combines them.
- SIGKILL after commit preserved the complete batch exactly once; SIGKILL
  before commit preserved none. Complete baseline and reopened records/fields
  matched expected data in these separate small crash fixtures.
- In both crash cases, deleting the highest committed Process/Ambient IDs,
  closing/reopening and allocating through persisted sequences produced higher
  IDs without reuse and preserved all remaining fields. This does not prove
  imported SQLite sequence metadata, overflow behavior or a migration protocol.

Whole parent-process peak RSS was 116,785,152 bytes (111.375 MiB), including
setup, all probes and validation; killed worker RSS is excluded. Total wall
time was 42.459 s and parent CPU 43.923 s. Neither this total nor subtraction
from its start RSS proves incremental application idle/query/migration memory.

## Retention and physical capacity

A separate fixture held 25,000 Process and 2,500 Ambient rows. The expiry
predicate removed exactly 13,322 eligible records; every surviving full row
matched before and after reopen. This validates the predicate on these
canonical synthetic timestamps, not all endpoint timestamp semantics.

| Phase | Native DB bytes | WAL bytes |
| --- | ---: | ---: |
| Before deletion/checkpointed | 1,585,152 | 0 |
| After deletion, before checkpoint | 1,585,152 | 107,141 |
| After checkpoint | 2,109,440 | 0 |

Logical removal succeeded, while file length increased 524,288 bytes. The
probe does not measure reusable internal free space, future steady-state
reuse, long-running maintenance, rollup gating, deletion preferences or
full-file compaction. DuckDB documents that VACUUM does not reclaim deleted
space and that complete compaction can require copying to a new database;
this keeps reclamation and its temporary-space/recovery cost open in #2084.
See [official space reclamation guidance](https://duckdb.org/docs/current/operations_manual/footprint_of_duckdb/reclaiming_space).

## Remaining adoption work

- **#2083:** exceptional query semantics, complete Process fields/schema,
  timestamp predicates, numerical contracts and representative large-history
  costs. Storage preservation alone does not complete the issue.
- **#2084:** realistic minute latency/visibility, cancellation, checkpoint
  contention, sustained growth/reuse and bounded compaction. Rollup/preferences
  and partial-failure behavior need application evidence. SIGKILL is not an
  OS/power-loss test; all supported-platform durability gates remain open.
- **#2085:** isolated Rust build/integration, all 23 App migrations and query
  consumers, mutable-table conversion, native file-version policy, distribution
  packaging, true application memory and durable authority selection. Current
  progress is code/official-source inventory, not a Rust execution result.
- Complete the ten-year and full-schema matrix, ratify resource/throughput
  budgets, then seek format acceptance. Production enablement remains later.
