# Native DuckDB Retention, Checkpoint And Reuse Evidence

Status: evidence for the last unmeasured item of
[#2089](https://github.com/shm11C3/HardwareVisualizer/issues/2089), "measure
recurring expiry/checkpoint/reuse using the implemented backend", carrying
forward the retention questions left open by
[#2084](https://github.com/shm11C3/HardwareVisualizer/issues/2084). Unlike the
2026-09-06 Python probes, everything here runs the production code: App's
migrations, the SQLite writers for the converted history, the #2088 candidate
builder, finalization into App's stable schema, and then the native writers and
`delete_old_data` of every raw archive family through the blocking owner. No
product code changed; the probe is the ignored integration test
[`core/tests/duckdb_retention_probe.rs`](../../core/tests/duckdb_retention_probe.rs).

Measured on Windows 11 Pro x64 (AMD Ryzen 7 7800X3D, 16 logical CPUs, 64 GiB)
at develop `eb0f04dc`, rustc 1.98.1, release profile, `duckdb 1.10505.0` with
bundled DuckDB 1.5.5, storage compatibility `v0.10.2`, `threads = 2`,
`max_memory = 128MB`, and the engine's default `checkpoint_threshold` of
16 MiB. Two runs, one per configuration, with nothing else compiling on the
host:

- [as implemented](benchmarks/hardware-archive-duckdb-retention-2026-09-17-as-implemented.json):
  the owner issues no `CHECKPOINT`; only the engine's automatic threshold
  checkpoint and `checkpoint_on_shutdown` run.
- [explicit checkpoint](benchmarks/hardware-archive-duckdb-retention-2026-09-17-explicit-checkpoint.json):
  one `CHECKPOINT` on the write lane after each daily expiry pass.

**Implication.** Expiry is cheap and exact, and the file settles at roughly
twice its fresh-copy size under steady churn, as the 2026-09-06 probe already
suggested. What the implemented backend adds is a checkpoint question: left to
the engine, checkpoints land inside a random write cycle and cost 177 to 677 ms
on the write lane, and the data still in the WAL is also held in memory, which
adds 30 to 40 MiB to the process between checkpoints. One explicit checkpoint
after the daily expiry pass costs 111 to 220 ms, keeps the WAL under 7 MiB, and
returns the engine to 10 to 20 MiB. The App lifecycle owner (#2135) should
schedule that checkpoint; the owner does not need a new operation to do it.

## Fixture

Simulated minute cycles, each writing 20 Process rows, one `DATA_ARCHIVE` row,
one `GPU_DATA_ARCHIVE` row, one `AMBIENT_ARCHIVE` row and two `FAN_ARCHIVE`
rows, all stamped with the cycle's instant. Seven days (10,080 cycles) are
written through the SQLite production writers and converted; seven more days
(10,080 cycles) are written through the native writers. After each native day,
every family is expired against a 7-day Retention Period in production order
(`persistence::archive::cleanup_old_data`), so the retained row count is held
steady while one day's rows leave and one day's rows arrive.

`delete_old_data` derives its cutoff from the wall clock, so simulated days are
expired by shrinking the Retention Period argument by one per day; the cutoff
each call used lies between the two texts the artifact records around it. A
day's cycles are written back to back, so automatic checkpoints are driven by
WAL bytes, not elapsed time.

| Stage | Value |
| --- | ---: |
| SQLite source after seeding | 25.145 MiB, 252,000 domain rows |
| Candidate | 6.512 / 7.012 MiB |
| Finalized | 9.012 MiB, 36 blocks of 256 KiB, no free blocks |
| Candidate + finalization | 20.1 / 20.4 s |
| Open through the owner | 26.1 / 20.8 ms |

The SQLite seeding took 512.5 / 617.8 s for 10,080 cycles, about 51 to 61 ms
per cycle through the sqlx writers, against 29 to 34 ms mean per native cycle
below. Both are the same five family transactions in the same process, so this
is an observation about this host, not a benchmark of either engine.

## Expiry: exact, tens of milliseconds, no physical shrink

Every daily pass deleted exactly one simulated day per family (1,440 or 1,441
rows for the one-row families, twice that for fans, 28,800 or 28,820 for
Process; the extra minute is wall-clock drift during the run). In all fourteen
passes the surviving row count sat inside the bracket the two cutoff texts
allow and the oldest surviving stamp was at or after the earlier cutoff. The
row counts before close and after reopen were identical in both runs.

| Family | Rows deleted per pass | Delete latency |
| --- | ---: | ---: |
| `DATA_ARCHIVE` | 1,440 to 1,453 | 4.3 to 5.6 ms |
| `GPU_DATA_ARCHIVE` | 1,440 to 1,453 | 4.2 to 5.2 ms |
| `FAN_ARCHIVE` | 2,880 to 2,906 | 5.9 to 7.8 ms |
| `PROCESS_STATS` | 28,800 to 29,060 | 33.0 to 43.1 ms |
| `AMBIENT_ARCHIVE` | 1,440 to 1,453 | 4.1 to 6.9 ms |

A whole pass is about 60 ms. Neither the file nor the block accounting changed
on deletion: the same `used_blocks` and `free_blocks` are reported before and
after every expiry, and the WAL grows by the deletion's own entries (about
0.28 MiB per pass). Reclamation happens only at a checkpoint, and then only as
`free_blocks` that later writes reuse.

## File growth under steady churn

Retained rows were constant within a few minutes of drift (about 201,300
Process and 10,065 rows per one-row family). The file still grew from its
finalized size, because the converted history was written as bulk row groups
and the churned data arrives as 1,440 small transactions a day.

| After native day | As implemented: file / WAL MiB | Explicit checkpoint: file MiB (free blocks) |
| --- | ---: | ---: |
| 1 | 9.012 / 6.671 | 16.262 (20) |
| 2 | 9.012 / 13.340 | 19.512 (28) |
| 3 | 17.262 / 5.028 | 22.262 (24) |
| 4 | 17.262 / 11.699 | 23.262 (39) |
| 5 | 20.012 / 3.387 | 24.512 (29) |
| 6 | 20.012 / 10.056 | 25.762 (44) |
| 7 | 23.512 / 1.748 | 24.762 (24) |
| Close | 23.762 / 0 | 24.762 / 0 |
| Fresh compact copy | 11.762 | 12.012 |

Growth slowed each day and the explicit-checkpoint file shrank for the first
time on day 7 as free blocks were reused, so the file is approaching a plateau
near twice the compact size rather than growing without bound. Seven days is
too short to name the plateau; a longer session is the open measurement. The
compact copy (`COPY FROM DATABASE` into a fresh file, taken outside the owner)
took 605 / 423 ms and reclaimed about half, at the cost of a second file on
disk while it runs. That remains the optional maintenance trade-off the Design
Doc already describes, not something expiry needs.

## Checkpoint placement and memory

Per-cycle append latency, five family transactions with the in-memory SQLite
stamp oracle and the owner's channel hop included:

| Run | p50 ms | p95 ms | Daily max ms |
| --- | ---: | ---: | --- |
| As implemented | 29.5 to 32.1 | 34.9 to 36.8 | 44.3, 43.5, 226.7, 40.5, 176.7, 500.4, 677.0 |
| Explicit checkpoint | 29.2 to 31.8 | 34.1 to 35.9 | 64.0, 48.0, 45.8, 44.0, 41.1, 40.4, 40.6 |

In the as-implemented run the three daily maxima above 100 ms are the days on
which the WAL crossed the 16 MiB threshold and the engine checkpointed inside a
write cycle (the file grew and the WAL fell on days 3, 5 and 7). The 500 ms
maximum on day 6 has no matching file change and is unexplained. With one
explicit checkpoint per day the WAL never exceeded 6.67 MiB, no automatic
checkpoint fired, and no cycle exceeded 64 ms. The explicit checkpoints took
111.0 to 219.5 ms each.

Memory follows the WAL. The engine's own `memory_usage` reads 54 to 69 MiB at
the end of a day's appends in both runs and 10.0 to 19.5 MiB immediately after
an explicit checkpoint; the test process's resident set moved from 79 to 96 MiB
before a checkpoint to 49 to 61 MiB after one. Closing the owner with 1.7 MiB
of WAL pending took 135.3 ms (`checkpoint_on_shutdown`), against 6.0 ms with
nothing pending; reopen took 24 to 29 ms and the reopened process sat at 40 to
43 MiB resident. The resident figures include the sqlx pool that seeded the
history and are not a whole-application budget.

## What this settles and what it leaves open

Settled for #2089:

- Recurring expiry through the implemented `delete_old_data` functions removes
  exactly the expired rows, costs tens of milliseconds per family, and survives
  close and reopen.
- Expiry frees nothing physically; checkpoints turn deletions into reusable
  blocks, and ordinary appends reuse them. The file plateaus near twice its
  compact size under steady churn, so normal retention does not need a
  compaction step.
- Checkpoint scheduling is a lifecycle decision with a measured answer: one
  checkpoint after the daily expiry pass bounds WAL, memory, close time and
  write-lane latency at a cost of about 0.1 to 0.2 s.

Still open, and not claimed here:

- Checkpoint contention with a concurrent long reader, and cancellation of a
  running expiry; the probe had no concurrent reader.
- The plateau beyond seven days, and multi-year histories.
- Linux and both macOS targets, and power-loss durability
  (see the [durability plan](hardware-archive-duckdb-distribution-evidence.md#durability-plan-for-2084)).
- Whole-application memory with the collectors and the frontend running.

Reproduce with:

```bash
HV_PROBE_CHECKPOINT=1 HV_DUCKDB_RETENTION_PROBE_OUT=/tmp/retention.json cargo test -p hardviz-core --features duckdb-archive --release --test duckdb_retention_probe -- --ignored --nocapture
```

`HV_PROBE_SEED_DAYS`, `HV_PROBE_NATIVE_DAYS`, `HV_PROBE_RETENTION_DAYS`,
`HV_PROBE_CYCLES_PER_DAY` and `HV_PROBE_PROCESSES` scale the fixture; the
defaults are the values measured here. Each run took about 15 minutes.
