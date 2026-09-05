# Hardware Archive G1 Initial Benchmark

Status: experimental evidence for [#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052).
The [G1 acceptance gate](hardware-archive-implementation-plan.md#g1--prove-process-stats-and-sensor-format-candidates)
remains open. These measurements establish a reproducible two-family path;
they do not select a production format or authorize G2.

## Recommendation

Continue evaluating one-hour columnar chunks with Deflate as a **size
candidate**. Keep Process Stats mandatory. The first experiment demonstrates
exact preservation and substantial reclaimed-file savings, but the measured
query paths do not meet the proposed latency comparison. Optimize and remeasure
the query strategy before accepting a format; do not relax the data-preservation
contract to obtain a passing result.

The next experiment should resolve bounded reads and aggregation for Process
Stats, and an efficient Ambient query that preserves SQLite's timestamp
membership semantics. Complete high-cardinality process workloads and the
remaining families before deciding per-family storage. There is no evidence
here to freeze a ten-year paging budget or a migration throughput floor.

## Reproduce

The measured executable is from commit
`f4afeb9acc5b4c6a3206375db2b87b2b834fec2e`, based on develop
`838d08da16d60ead84df3ddb1501ca19a57f24fc`. The
[G1 inventory](hardware-archive-g1-inventory.md) pins all schema-v23 objects and
query consumers, including the Cooling covariate summaries introduced by
#2070. The executable copies only the current Process Stats and Ambient raw
schemas and their timestamp indexes into a synthetic database.

```bash
cargo build -p hardviz-core --release --example archive_format_benchmark
/usr/bin/time -l target/release/examples/archive_format_benchmark \
  --output /tmp/hardviz-g1-24h-columnar-deflate \
  --days 1 --seed 2052 --repetitions 7 \
  --layout columnar --compression deflate
```

The output directory must not exist. The command creates its own relational
oracle, candidate, and isolated transaction probes there, then writes
`report.json`. It never locates or opens an application database. Run each case
serially after building; use distinct directories. `/usr/bin/time -l` is the
macOS wrapper for the external CPU/RSS observations below. Use an equivalent
wrapper on other systems. See the committed
[measurement artifact](benchmarks/hardware-archive-g1-2026-09-05.json) for every
case's arguments, complete report, and external resource observations.

Default limits are 60 represented minutes, 4,096 rows, and 4 MiB of decoded
value bytes per chunk. The retained relational tail spans the newest half
chunk duration. The time-limit comparisons therefore change tail length too;
they are whole-configuration comparisons, not an isolated time-cap experiment.
The 15-minute comparison also uses a 256-row cap. All cases use 15 ordinary
process observations per minute, seed 2052, and duty cycle 1. No elapsed time
is skipped to simulate longer histories.

## Environment and method

Measured on 2026-09-05 using Apple M4 (10 logical CPUs), 24 GiB RAM,
macOS 26.6.2 (25G83), and APFS. The Rust binary reports SQLite 3.46.0, WAL mode,
`synchronous=NORMAL`, 4,096-byte pages, and Rust 1.98.0
(`88d9e12ae`, 2026-08-18). Full SQLite compile options are in the artifact.
APFS was checked independently; the portable harness reports filesystem as
unavailable on macOS. The inventory's system SQLite version is not substituted
for the benchmark's bundled SQLite version.

Synthetic identities rotate through 45 PID/name groups, with an additional
sparse sentinel group. Ordinary rows use the producer's numeric ranges and
bounded process lifetimes. Separate sentinels exercise exact i64 and binary64
storage. Two Ambient source labels include gaps and nullable humidity.
Unicode/NUL text is deliberate fixture data. This is not a realistic process
ranking distribution or a high-churn workload. Duplicate instants, offset and
submillisecond timestamp spellings, noncontiguous IDs, backward clock steps,
and incomplete/corrupt chunks receive separate contract tests.

Each case is one complete executable run with seven repetitions of each query.
The source and candidate have already been populated and validated; the
candidate has also been closed and reopened. Queries alternate baseline then
candidate on the same host without cache purging. These are cache-primed measurements, including the first query, not
controlled cold-cache results. With seven observations, the reported p95 and
p99 both equal the maximum; use the raw p50 as additional context and do not
interpret these as stable tail-latency estimates.

Both queries cover the latter half of the represented history. Process Stats
compares grouped averages/counts/maxima against SQLite, with exact group keys,
counts, and timestamps and the design's separate float tolerance. Results are
canonicalized by PID/name for comparison; the experiment does not implement
production CPU-order pagination. Ambient compares an inclusive raw range using
the current SQLite epoch-millisecond expression. The candidate decodes all
Ambient chunks into a temporary SQLite timestamp/digest relation to preserve
that predicate and original-ID ordering exactly. Its timing includes this
work. This representative query does not prove every production Ambient bucket
or half-open Cooling pairing query.

## Measured results

### Size and query comparison

Sizes are MiB (`bytes / 1,048,576`). Query values are p95 milliseconds.

| Configuration | Relational DB | DB after VACUUM | Reduction | Process row / chunk | Ambient row / chunk |
| --- | ---: | ---: | ---: | ---: | ---: |
| 24h-row-none | 2.367 | 2.004 | 15.35% | 3.345 / 11.655 | 2.708 / 19.360 |
| 24h-row-deflate | 2.367 | 0.477 | 79.87% | 3.334 / 9.322 | 2.736 / 18.902 |
| 24h-columnar-none | 2.367 | 0.812 | 65.68% | 3.153 / 8.615 | 2.677 / 18.851 |
| 24h-columnar-deflate | 2.367 | 0.301 | 87.29% | 3.283 / 8.210 | 2.730 / 18.841 |
| 24h-columnar-deflate-15m | 2.367 | 0.465 | 80.36% | 3.171 / 8.286 | 2.734 / 19.359 |
| 24h-columnar-deflate-240m | 2.367 | 0.426 | 82.01% | 3.253 / 8.384 | 2.685 / 18.098 |
| 30d-columnar-deflate | 72.027 | 6.906 | 90.41% | 134.360 / 244.489 | 85.041 / 536.850 |
| 1y-columnar-deflate | 883.348 | 83.223 | 90.58% | 2468.620 / 2999.155 | 1572.645 / 6595.218 |

These are logical file lengths for two raw families, including their indexes.
They are not full-application savings or filesystem physical-block measurements.
All measured WAL lengths were zero after explicit checkpointing; SHM sizes and
index/live/free-page bytes are recorded separately in the artifact.

### Longer-history comparison

The proposed comparison is `candidate p95 <= max(1.2 * baseline p95,
baseline p95 + 20 ms)`. The following uses the cache-primed measurements above
as an initial comparison; it does not ratify a statistically stable warm budget.

| Duration / query | Baseline p95 | Candidate p95 | Proposed ceiling | Result |
| --- | ---: | ---: | ---: | --- |
| 30d / Process Stats | 134.360 ms | 244.489 ms | 161.232 ms | Fail |
| 30d / Ambient raw | 85.041 ms | 536.850 ms | 105.041 ms | Fail |
| 1y / Process Stats | 2468.620 ms | 2999.155 ms | 2962.344 ms | Fail |
| 1y / Ambient raw | 1572.645 ms | 6595.218 ms | 1887.174 ms | Fail |

### Reclamation and resources

| Default columnar/Deflate case | Process / Ambient rows | DB before VACUUM (MiB) | VACUUM (ms) | Append p99 (ms) | Total wall / CPU (s) | External peak RSS (MiB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 24h | 21,615 / 2,618 | 2.383 | 1.234 | 0.113 | 0.560 / 0.586 | 29.00 |
| 30d | 648,446 / 78,546 | 72.043 | 25.234 | 0.139 | 15.006 / 15.778 | 29.03 |
| 1y | 7,889,419 / 955,636 | 883.363 | 316.211 | 1.332 | 200.460 / 197.950 | unavailable |

Finalization frees pages for reuse but does not itself shorten the database
file: the candidate before reclamation is slightly larger than the source.
VACUUM is explicit here and is not a recommendation to run blocking reclamation
in the application. The source copy is retained. Source-plus-candidate and
SQLite temporary-file peak space were not instrumented.

Total CPU is user plus system CPU across fixture generation, copying,
validation, repeated queries, and reclamation. It is not steady-state maintenance
CPU. Shorter runs use Python `os.wait4` resource usage for the benchmark child;
macOS reports `ru_maxrss` in bytes. The one-year `/usr/bin/time -l` wrapper
printed wall/user/system time, then failed with
`sysctl kern.clockrate: Operation not permitted`. Its exit status was 1; the
benchmark had already emitted matching complete file/stdout reports with every
correctness flag true. Peak RSS for that case is therefore unavailable, not
zero. The report's end-of-run RSS is a separate observation, not peak RSS.

## What the results establish

All recorded runs produced complete reports only after the seven required
correctness flags passed: exact codec decoding, exact records after
closing/reopening the persisted candidate, Process Stats aggregate equivalence, Ambient raw range
equivalence, rollback when a selected row disappears before commit, unchanged
multiplicity on post-commit retry, and a consistent read snapshot while another
connection finalizes the tail. Exact validation checks the actual persisted IDs
and values against the source and uses a temporary primary-key manifest to
reject duplicates and missing coverage. These probes establish the implemented
experimental boundaries, not the full online migration/recovery state machine.

The lossless frame preserves SQLite value classes and exact integer, float-bit,
text-byte, blob-byte, and null values. It has a version, integrity digest, and
bounded decoding. Columnar encoding adds checked integer deltas, float XORs, and
local text dictionaries. Row layout and uncompressed variants are controls.
Deflate uses an already locked workspace dependency, added only as a Core
development dependency; it is not a production codec commitment. Truncated
compressed streams must reach neither a successful decode nor a correctness
report, even when the outer hash has been recomputed.

The catalog index is `(family, id)` because the measured readers consume one
chunk at a time with `id > cursor ORDER BY id LIMIT 1`. An earlier timestamp
index caused a repeated temporary sort for this access path. The corrected
index is covered by an `EXPLAIN QUERY PLAN` regression test. It does not solve
all interval-index selection or per-series partitioning questions.

## Open acceptance gates

| Gate | Evidence and remaining work |
| --- | --- |
| Exact preservation | Passes the two representative paths and focused corruption/boundary tests. Add the other raw families and preserved relational tables to full-database validation. |
| Disk benefit | Strong two-family savings after explicit reclamation. Per-family Process Stats index accounting and a complete representative database are still needed for the 50%/30% gates. |
| Query latency | The candidate misses the proposed comparison for longer ranges. Optimize the real query boundary and repeat these measurements before format acceptance. |
| Ten-year history | Not measured in this initial report. Existing-range query failures must be addressed; continuous ten-year data and paged/aggregate budgets remain required by G1. |
| Process workload | The generator has 46 aggregate groups at the default density. High PID/name churn, realistic ranking, bounded paging, spills, and partial errors remain unproven. |
| Other families/layouts | System, GPU, fan, mixed metadata, per-series partitioning, byte-cap boundary configurations, and alternative compression candidates still need comparison. |
| Append/visibility | Standalone minute-append transactions are measured, including commit. They do not establish append latency under concurrent maintenance or absence of missed intervals. |
| CPU/memory | External total CPU/RSS are recorded for the entire synthetic run. They do not prove additional steady-state maintenance cost over 30 minutes or history-independent query/migration overhead. |
| Migration/reclamation | Per-chunk finalization and explicit VACUUM are measured. Source growth, online capture, catch-up, full validation cost, temporary-space peak, recovery-copy protection, and cutover pauses need the later migration experiment. |
| Platform and repeatability | Local measurements use one macOS host and one run per configuration. Native Linux/Windows measurements and repeated/cold-cache trials remain open. |
| Human decision | No production format, numerical migration floor, or ten-year budget is ratified by this report. Maintainer acceptance remains required before G2. |

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p hardviz-core --all-targets -- -D warnings
cargo test -p hardviz-core -- --test-threads=1
npm run check:agent-guidance
```

The benchmark example has `test = true`, so the ordinary Core test command also
runs its codec and SQLite contract tests. Production collection, migration
selection, retention, and application queries are not wired to this executable.
