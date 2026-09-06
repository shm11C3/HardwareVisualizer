# Hardware Archive G1 Engine Comparison

Status: measured experiment for [#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052). No production storage format is accepted by this report. The accepted SQLite decision in ADR 0019 remains in force.

## Recommendation

Continue evaluating standard columnar engines. Prioritize **DuckDB native storage** for the next compatibility and lifecycle prototype: it reduces storage and query cost while providing a single-database transactional boundary. Keep **Parquet queried by DuckDB** as the capacity-oriented alternative; it is smaller in these fixtures, but requires a separately proven publication and recovery protocol for immutable files. These recommendations concern the next experiment, not replacement of the application's SQLite database.

Resolve lossless representation of SQLite's mixed storage classes and timestamp membership before selecting either format. Do not relax retained samples, reinterpret legacy timestamps, or adopt the previously rejected binary64 chunk summaries. Tagged values or a validated legacy-row preservation path are possible designs, but neither their correctness nor their storage cost has been tested here. The working-memory, complete-database migration, maintenance, and cross-platform bundle gates also remain open.

## Method and provenance

The corrected matrix uses source commit `8610bacde374f31db91ef86de8f207f7153a8068`, built on the prior query experiment and develop `838d08da16d60ead84df3ddb1501ca19a57f24fc`. Inputs are the unchanged relational databases from query-fixture source `7fae38b38116d82d771d6fc3705f67f618218732`. The full artifact contains hashes of every script, binding, SQLite source, source database, and input range report, plus exact commands and every measurement. The current source inputs were hashed again after all runs.

The host is an Apple M4, 10 logical CPUs, 24 GiB RAM, macOS 26.6.2 build 25G83. This uses Python 3.14.5, DuckDB 1.5.5 and pysqlite3 0.5.4 linked to SQLite 3.46.0. SQLite has the same engine version as Core, but a different binding and build; compare strategies within this report, not its absolute times against the earlier Rust/SQLx report. CPU/RAM reads blocked in the sandbox are retained as unavailable in the matrix and supplemented by separate successful host reads.

Seven cases cover continuous 24-hour, 30-day, and one-year history, with 15 process samples per minute and seed 2052. Stable cases repeat 45 ordinary PID/name groups plus a sparse numeric sentinel. Churn reuses a bounded 120-PID pool with generated changing names and tuple lifetimes of 30 minutes; the 30-day stress case uses one minute. This is a cardinality stress distribution, not a measured real-world executable-name frequency. The existing unaligned latter-half and middle-24-hour ranges are reused; the latter is a middle-third range for 24-hour datasets.

Each case prepares candidates once, reopens them, and stream-compares every stored row against SQLite. Queries run in separate serial children for each strategy; strategy order rotates by case. Each of seven repetitions opens a fresh connection outside timing, runs both families and ranges, then closes it. Source hashing and preparation prime caches; no controlled cold-cache or cache-purge claim is made, and connections/prepared plans are not reused across repetitions. No concurrent builds or benchmarks ran during the matrix. p50 uses the middle sample and nearest-rank p95 equals the maximum of seven observations; these are initial comparisons, not stable tail-latency estimates.

DuckDB uses two threads and a **128 MB managed-memory limit per database instance**. SQLite uses a 2,000 KiB main-page cache, `temp_store=FILE`, and a 16 MiB temporary-page cache. Native and in-memory Parquet instances have separate spill directories. Results are drained in 500-row client batches; full differential validation is a separate, untimed phase. Process queries preserve `(pid, process_name)`, inclusive original-text ranges, averages, counts, maxima, latest original timestamp, and CPU-descending rank with PID/name ties. Ambient returns the five original fields ordered by original ID. Its candidates additionally store the exact epoch-millisecond expression computed by **source SQLite** during export, so their fast filter includes a semantic adapter and its storage cost.

## Query results

Cells show **p50 / p95 milliseconds** for complete result drain. Both candidates are compared with `max(1.2 * SQLite p95, SQLite p95 + 20 ms)`. These measured ranges do not establish a ten-year IPC pagination or chart-bucketing contract.

### Process Stats

| Case / range | Rows or groups | SQLite | DuckDB native | Parquet + DuckDB | Ceiling | Result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| 24h-stable / half | 46 | 2.729 / 2.937 | 0.660 / 1.236 | 1.158 / 1.729 | 22.936 | Both pass |
| 24h-stable / middle third | 46 | 1.787 / 1.830 | 0.530 / 0.570 | 1.038 / 1.125 | 21.830 | Both pass |
| 24h-churn / half | 360 | 2.791 / 2.869 | 0.846 / 1.440 | 1.308 / 1.957 | 22.869 | Both pass |
| 24h-churn / middle third | 255 | 1.817 / 1.832 | 0.638 / 0.703 | 1.137 / 1.309 | 21.832 | Both pass |
| 30d-stable / half | 46 | 116.336 / 120.031 | 4.984 / 5.592 | 8.925 / 9.727 | 144.037 | Both pass |
| 30d-stable / middle 24h | 46 | 5.568 / 5.712 | 1.885 / 1.917 | 3.853 / 4.002 | 25.712 | Both pass |
| 30d-churn / half | 10,800 | 127.321 / 129.351 | 9.660 / 10.805 | 14.105 / 15.253 | 155.221 | Both pass |
| 30d-churn / middle 24h | 735 | 8.223 / 8.376 | 2.326 / 2.718 | 4.357 / 4.427 | 28.376 | Both pass |
| 1y-stable / half | 46 | 1716.172 / 1743.515 | 50.211 / 59.039 | 81.506 / 89.383 | 2092.218 | Both pass |
| 1y-stable / middle 24h | 46 | 5.900 / 6.005 | 2.813 / 2.926 | 4.584 / 4.786 | 26.005 | Both pass |
| 1y-churn / half | 131,400 | 1872.511 / 1899.054 | 110.213 / 115.804 | 143.273 / 153.527 | 2278.864 | Both pass |
| 1y-churn / middle 24h | 735 | 8.438 / 8.824 | 2.967 / 3.052 | 5.072 / 5.184 | 28.824 | Both pass |
| 30d-churn-1m / half | 323,850 | 462.092 / 484.037 | 166.264 / 169.791 | 172.458 / 175.289 | 580.845 | Both pass |
| 30d-churn-1m / middle 24h | 21,600 | 28.624 / 29.460 | 11.977 / 12.358 | 15.883 / 16.222 | 49.460 | Both pass |

### Ambient raw ranges

| Case / range | Rows or groups | SQLite | DuckDB native | Parquet + DuckDB | Ceiling | Result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| 24h-stable / half | 1,290 | 1.230 / 1.382 | 0.585 / 0.779 | 0.706 / 0.906 | 21.382 | Both pass |
| 24h-stable / middle third | 874 | 1.021 / 1.052 | 0.416 / 0.460 | 0.559 / 0.613 | 21.052 | Both pass |
| 24h-churn / half | 1,290 | 1.226 / 1.416 | 0.600 / 0.759 | 0.713 / 0.914 | 21.416 | Both pass |
| 24h-churn / middle third | 874 | 1.027 / 1.053 | 0.427 / 0.444 | 0.578 / 0.626 | 21.053 | Both pass |
| 30d-stable / half | 39,254 | 37.124 / 37.938 | 10.992 / 11.417 | 12.388 / 13.312 | 57.938 | Both pass |
| 30d-stable / middle 24h | 2,618 | 21.884 / 22.491 | 1.046 / 1.105 | 2.681 / 2.781 | 42.491 | Both pass |
| 30d-churn / half | 39,254 | 36.956 / 37.782 | 10.937 / 11.120 | 12.332 / 12.578 | 57.782 | Both pass |
| 30d-churn / middle 24h | 2,618 | 21.765 / 22.333 | 1.110 / 1.136 | 2.679 / 2.727 | 42.333 | Both pass |
| 1y-stable / half | 477,800 | 450.242 / 461.338 | 124.897 / 126.950 | 128.380 / 130.314 | 553.606 | Both pass |
| 1y-stable / middle 24h | 2,619 | 256.780 / 259.951 | 1.298 / 1.363 | 4.061 / 4.193 | 311.941 | Both pass |
| 1y-churn / half | 477,800 | 449.961 / 453.705 | 122.903 / 125.056 | 129.172 / 130.707 | 544.446 | Both pass |
| 1y-churn / middle 24h | 2,619 | 260.885 / 264.264 | 1.276 / 1.330 | 4.069 / 4.180 | 317.117 | Both pass |
| 30d-churn-1m / half | 39,254 | 37.336 / 37.736 | 11.184 / 11.432 | 12.716 / 12.919 | 57.736 | Both pass |
| 30d-churn-1m / middle 24h | 2,618 | 21.817 / 22.545 | 1.130 / 1.203 | 2.747 / 2.798 | 42.545 | Both pass |

## Storage and resources

| Case | Process / Ambient rows | SQLite MiB | Native MiB | Parquet MiB | Native / Parquet reduction |
| --- | ---: | ---: | ---: | ---: | ---: |
| 24h-stable | 21,615 / 2,618 | 2.367 | 1.012 | 0.241 | 57.26% / 89.83% |
| 24h-churn | 21,600 / 2,618 | 2.762 | 1.012 | 0.240 | 63.37% / 91.31% |
| 30d-stable | 648,446 / 78,546 | 72.027 | 17.012 | 6.926 | 76.38% / 90.38% |
| 30d-churn | 648,000 / 78,546 | 83.547 | 13.012 | 6.657 | 84.43% / 92.03% |
| 1y-stable | 7,889,419 / 955,636 | 883.348 | 204.262 | 83.459 | 76.88% / 90.55% |
| 1y-churn | 7,884,000 / 955,636 | 1027.512 | 149.512 | 80.131 | 85.45% / 92.20% |
| 30d-churn-1m | 648,000 / 78,546 | 83.547 | 17.012 | 6.544 | 79.64% / 92.17% |

Sizes are logical file lengths after closing/checkpointing: the original SQLite database including indexes, the complete native database including metadata/WAL, or both Parquet files including their metadata. Only Process and Ambient fixtures are compared. This is not the whole production database, and per-family native allocation, preserved families, publisher metadata, active tails, recovery copies, and production retention overhead are not established. Native has no explicit primary-key/index constraints in this immutable query fixture; the concurrency probe tests a different small primary-key schema. Do not extrapolate these reductions into an accepted complete-database budget.

| Case | Query-stage peak RSS: SQLite / native / Parquet MiB | Preparation peak RSS MiB | CSV staging MiB | Export / native import / Parquet write seconds |
| --- | ---: | ---: | ---: | ---: |
| 24h-stable | 54.406 / 61.312 / 61.234 | 91.344 | 1.821 | 0.037 / 0.055 / 0.005 |
| 24h-churn | 54.812 / 61.500 / 61.516 | 93.109 | 2.240 | 0.041 / 0.056 / 0.005 |
| 30d-stable | 55.141 / 76.250 / 74.484 | 312.703 | 56.616 | 1.126 / 0.309 / 0.116 |
| 30d-churn | 56.484 / 77.641 / 79.766 | 340.766 | 69.186 | 1.217 / 0.289 / 0.128 |
| 1y-stable | 55.594 / 186.969 / 127.688 | 351.594 | 697.438 | 13.944 / 3.023 / 2.596 |
| 1y-churn | 58.016 / 200.000 / 143.797 | 378.266 | 850.368 | 15.099 / 2.282 / 2.784 |
| 30d-churn-1m | 57.375 / 154.625 / 160.969 | 362.312 | 69.186 | 1.233 / 0.331 / 0.143 |

Query-stage peak RSS is captured in each strategy's fresh child **before** its untimed SQLite-oracle validation, but includes Python, loaded engine modules, all repetitions, and startup. All strategy children import DuckDB, including the SQLite baseline, so subtracting these peaks does not estimate the application cost of adding DuckDB. It is not incremental per-query allocation; the independent-of-history 256 MiB extra-memory gate remains unproven. The artifact also retains final whole-child high-water RSS and user/system CPU, separately from query process CPU. Preparation simultaneously validates two DuckDB instances and includes export, import, writes, sorting, and exact digests. Its peak is not production migration memory.

CSV staging is retained and measured. Native import and Parquet export are separate operations, with Parquet produced from the prepared native table. A production migration would also retain the source and recovery data. Temporary file lengths are observed at phase boundaries only; the in-query/transformation peak and free-space preflight are not validated. Engine-managed memory limits do not bound whole-process RSS. The installed `_duckdb` Python extension is 43.300 MiB; this is neither a Rust dependency-size measurement nor a Tauri bundle-size estimate.

## Compatibility and concurrency

| Probe | Result | Interpretation |
| --- | --- | --- |
| Stored ordinary typed values | Native and Parquet pass | Exact signed i64 values, SQLite-read binary64 bits, nulls, BLOBs, original IDs, names including NUL, timestamps, and duplicates in the focused fixture; full generated matrices also match after reopen. |
| Seeded negative zero | SQLite reads positive zero | Candidates preserve SQLite-observed bits. This does not prove preservation of negative zero that SQLite did not retain. |
| SQLite mixed storage classes / invalid UTF-8 | Fixed BIGINT/VARCHAR mapping fails | Of 10 native projections, 4 are exact, 2 change, and 4 are rejected; Parquet checks only the exact subset. This rejects the tested mapping, not every possible tagged DuckDB/Parquet representation. |
| Process query semantics | Focused and full generated comparisons pass | Discrete fields/rank match exactly; averages meet `max(1e-9, 1e-12 * abs(reference))`. |
| Signed-integer cancellation counterexample | Both return SQLite's `1024.5` | Direct engine AVG passes the four-integer case that defeated merging binary64 chunk sums. It is not a proof for every floating reduction. |
| Native timestamp conversion | Fails SQLite membership | UTC normalization alone is insufficient. For `2026-01-01T00:00:00.999500Z`, SQLite produces `1767225601000`, while DuckDB produces `1767225600999`. Raw text is retained; the main matrix uses the source-computed key. |

The complete matrix compares every source row after reopening both candidates. All 56 candidate family/range comparisons pass against SQLite, and all 588 timed queries return the validated counts. The separate compatibility probes retain their negative findings. Tests do not turn a failed broad compatibility probe into a passing production gate. `production_format_accepted` remains false even when every measurement completes successfully.

In a separate generic 200,000-row/64-group fixture, both SQLite WAL/NORMAL and native DuckDB completed 30 one-row append transactions while analytic reads overlapped. Every append transaction overlapped a measured query interval; the pinned reader saw 200,000 rows throughout, and reopening saw exactly 200,030 with every appended ID once. Nearest-rank transaction p95 was **0.017 ms for SQLite and 0.570 ms for DuckDB**; p99/max was 0.124/0.800 ms. These timings include BEGIN/INSERT/COMMIT, not just COMMIT, and do not establish the production minute-batch append budget. Both engines retained a committed append exactly once and exposed no in-flight append after SIGKILL. OS/power failure, a hard one-minute durability bound, corruption recovery, and production cutover were not tested.

Parquet transactional publication was not implemented. A candidate design must write and validate new files, durably publish an authoritative generation, keep pinned readers on their generation, and recover interruptions before/after publication. [DuckDB's concurrency documentation](https://duckdb.org/docs/current/connect/concurrency) describes the single-process read/write boundary used by the native probe. [Parquet filtering documentation](https://duckdb.org/docs/current/data/parquet/overview) explains the filter/projection pushdown used by that alternative. [Memory and spilling documentation](https://duckdb.org/docs/current/guides/performance/how_to_tune_workloads) describes why a configured memory limit is not sufficient evidence of a complete working-memory bound.

## Resolved harness failure

The first matrix (`1dc633f9`) used one spill directory for independent native and in-memory DuckDB instances. Its one-year preparations both terminated with SIGBUS during `fetchmany`, while five smaller cases completed. Over 60 GiB of disk space remained. Isolated ordered reads of both candidates each drained 7,889,419 rows successfully. Keeping the same data, two threads, 128 MB limits, and simultaneous three-way validation, but assigning distinct spill directories, produced matching full-row digests. This matches [DuckDB's independent-instance temporary-file collision report](https://github.com/duckdb/duckdb/issues/15173).

Commit `8610bacd` fixes temporary-directory ownership; it changes no data, query, thread count, or memory setting. The final seven-case matrix was then run from a clean checkout. The failed run is retained in a separate superseded matrix linked from the main artifact; the representative native crash stack, isolated-read diagnostics, and successful three-way diagnostic remain under its diagnostic keys. The SIGBUS is evidence of the faulty harness, not rejection of DuckDB as a history engine. The [learning record](../agents/lessons/isolate-duckdb-spill-directories.md) links the regression boundary to the owning code.

## Reproduction and evidence

The full artifact is [hardware-archive-g1-engines-2026-09-06.json](benchmarks/hardware-archive-g1-engines-2026-09-06.json). SHA-256: `56efc8ed7ee72e84a1c67c74cad9e6301de77c0380a3e0d56bcbf0b51a9d3b12`. The [superseded matrix](benchmarks/hardware-archive-g1-engines-superseded-2026-09-06.json) is a separate artifact (SHA-256 `04dfa4dde0d0135e3b8ec57ff03838ee9ae757c653eb8fb3efdd57aff221e201`). Together, these files retain every parsed value of the original combined evidence; each matrix and probe was compared exactly using decimal numbers after splitting and repository JSON formatting. Original absolute paths and commands remain unchanged.

First generate the immutable SQLite inputs using the [prior query experiment's exact reproduction](hardware-archive-g1-query-experiment.md#exact-reproduction). Use a clean checkout of `8610bacde374f31db91ef86de8f207f7153a8068` for the commands below. Setup currently supports macOS and was verified through the equivalent pinned manual build; the setup wrapper received AST/help checks, not a second clean installation.

```bash
python3 core/examples/archive_engine_benchmark/setup_macos.py \
  --sqlite-source /path/to/libsqlite3-sys-0.30.1/sqlite3 \
  --output /tmp/hardviz-engine-runtime
PYTHONDONTWRITEBYTECODE=1 /tmp/hardviz-engine-runtime/venv/bin/python \
  core/examples/archive_engine_benchmark/engine_contracts.py \
  --output /tmp/hardviz-engine-contracts
PYTHONDONTWRITEBYTECODE=1 /tmp/hardviz-engine-runtime/venv/bin/python \
  core/examples/archive_engine_benchmark/concurrency_probe.py \
  --output /tmp/hardviz-engine-concurrency --rows 200000 --repetitions 30 \
  > /tmp/hardviz-engine-concurrency.json
PYTHONDONTWRITEBYTECODE=1 /tmp/hardviz-engine-runtime/venv/bin/python \
  core/examples/archive_engine_benchmark/run_matrix.py \
  --source-root /tmp/hardviz-query-matrix-2026-09-06 \
  --output /tmp/hardviz-engine-matrix \
  --repetitions 7 --threads 2 --memory-limit 128MB
```

Output directories should be new. A nonzero matrix exit means at least one preparation/query phase failed, and its causal stderr is retained. Contract-probe exit zero means execution completed; inspect each probe's status separately. No application dependency, migration, collection, IPC, retention, or generated binding is changed by this experiment. Rust/Tauri integration, all supported operating systems, real application append concurrency, ten-year histories, maintenance CPU, cancellation, finalization, recovery, and retained nonconverted families remain separate gates before revising ADR 0019 or selecting a production format.
