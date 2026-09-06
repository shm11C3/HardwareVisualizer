# Hardware Archive G1 Query Strategy Experiment

Status: experimental verification for [#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052). The Process binary64 summary candidate fails the retained numerical contract and is not accepted for production. Performance evidence below does not override that failure or complete G1.

## Decision

The interval metadata experiment tests whether Ambient can avoid decoding unrelated history while retaining SQLite timestamp membership. Process summaries test the performance and storage cost of reading per-chunk aggregates, with raw reads for range boundaries and the active tail. Both operate only on synthetic databases; no production query, migration, collection, or retention behavior changes.

## Numerical counterexample

The existing tolerance is `abs(actual - reference) <= max(1e-9, 1e-12 * abs(reference))`. These four stored INTEGER values are split into two chunks:

| Chunk | Memory values |
| --- | --- |
| 1 | `9223372036854775805`, `9223372036854775804` |
| 2 | `-9223372036854773756`, `-9223372036854773755` |

SQLite's row-based average is **1024.5**. Merging binary64 chunk sums yields **1024.0**. The absolute error is **0.5**, exceeding the allowed **0.0000000010245**. A focused test exercises the actual accelerator and confirms this difference; every measurement report also includes an isolated SQLite partial-sum reproducer. The stored codec values remain exact. The failed boundary is derived arithmetic.

The regression test asserts that this counterexample remains detected; a passing test suite therefore does not mean the candidate meets the numerical contract. `numerical_contract_passed` and `production_candidate_accepted` remain false. Generated performance-workload comparisons have separate result flags. A successful executable exit means the measurements completed, not production acceptance.

A follow-up [SQLite, DuckDB native, and Parquet comparison](hardware-archive-g1-engine-comparison.md) evaluates standard engines on the same immutable source fixtures. It retains the numerical and exact-sample requirements below.

## Design recommendation after this experiment

Keep range pruning as the next Ambient query building block. It sharply reduces decoded rows for a narrow request inside long history, while retaining the current SQLite predicate. Wide requests need a separate bounded output path: perform the actual consumer's bucket reduction while reading chunks, or stream raw results when a raw endpoint needs every row. Daily rollups cannot replace arbitrary raw queries without proving the existing endpoint's semantics.

Reject unconditional per-chunk Process summaries as measured here, as well as
the binary64 `sum/count` representation. The 30-day one-minute-lifetime stress
case produced 647,535 summary rows from 647,535 finalized rows, expanded the
post-VACUUM candidate from 9.727 MiB to 70.969 MiB, and made the accelerated
half-range query slower than both the relational and current readers. Before
production implementation, choose and measure both a selection rule and an
accumulation representation or exact read path. They must preserve the same
numerical tolerance against the row-based oracle, including the signed-integer
counterexample. Disk-backed aggregation and a ranked result snapshot remain
useful experimental pieces for paging every group, but neither is accepted by
this result.

These are next experimental boundaries. They do not approve either accelerator for production or change the retained query contract.

## Method

Source commit: `7fae38b3` (full hash in the measurement artifact), based on benchmark commit `713add956faaca2a8d43dec8cf5c47cd43d30ebf` and develop `838d08da16d60ead84df3ddb1501ca19a57f24fc`. The experiment is opt-in with `--query-experiment`. Continuous synthetic workloads use 15 process observations per minute, 60-minute / 4096-row chunks, columnar Deflate, seed 2052, and seven repetitions per query range. No elapsed time is skipped.

The complete full-precision artifact is
[hardware-archive-g1-query-2026-09-06.json](benchmarks/hardware-archive-g1-query-2026-09-06.json).
Its SHA-256 is
`291b055a457eea789ac5e672510f0f5e33434baa5f9b2410f1ef6730d9f2f21a`;
the measured binary SHA-256 recorded inside it is
`d4782a603c2b481c19219900f724a698ca93c3fa771b8a4f72e12d12ebee2989`.
The artifact contains the complete matrix, including the original absolute
`--output` arguments. Only repository JSON formatting was applied; parsed
values were compared exactly with the original matrix using decimal numbers.
No path normalization was applied. The unformatted matrix SHA-256 was
`1e280067c2087dee774b8fdef0ba80cf81678d052f366fb613a63cdf34c03d94`.

Each case compares relational rows, the previous chunk reader, and the experimental accelerator on the same records. Strategy order rotates across repetitions. Process results use average-CPU descending order with PID/name tie-breakers; canonical copies used for full-result validation are created after timing. The standard benchmark retains its original PID/name canonical query separately. Each candidate read observes one SQLite snapshot.

The two ranges are an unaligned latter-half range and a middle 24-hour range (the middle third for the 24-hour dataset). Original timestamp predicates, tuple identities, row multiplicity, nullable Ambient humidity, and original-ID digest ordering are preserved. Ambient uses SQLite's existing epoch-millisecond expression, not Chrono normalization. Non-UTF8 TEXT timestamps are explicitly unsupported by the experimental metadata builder.

Metadata is built once offline after exact source/candidate validation and explicit VACUUM. Reported storage growth is the subsequent database plus WAL length, including auxiliary tables and indexes. It is not a production maintenance or concurrent-finalization measurement. Both connections use `temp_store=FILE` and a 16 MiB temporary-page cache policy. This policy alone does not establish a memory bound.

Queries are cache-primed, with no controlled cache purge. Seven observations make p95 equal the maximum; these are initial comparisons, not statistically stable tail-latency estimates. The runner executes cases serially with no concurrent builds/benchmarks and records direct-child wall time, user/system CPU, peak RSS, and swaps through macOS `os.wait4`. Peak RSS covers setup and full validation as well as query execution.

The stable fixture repeats 45 ordinary PID/name groups plus a sparse numeric sentinel. Churn is a synthetic cardinality stress case: a bounded 120-PID pool reuses PIDs while generated process names change; each tuple lasts 30 minutes, or one minute for the stress case. It does not establish real-world executable-name frequency. All strategies within a case see exactly the same data.

The host was an Apple M4 with 10 logical CPUs and 24 GiB RAM, running verified
macOS 26.6.2 build 25G83. The executable reports Rust 1.98.0 and SQLite 3.46.0
in WAL mode with `synchronous=NORMAL`. Filesystem detection is `unavailable` in
every report because the executable does not detect the macOS filesystem. A
separate `diskutil` attempt also failed; this run makes no filesystem type
claim.

### Exact reproduction

The committed runner validates its arguments before creating the output
directory, refuses a dirty source checkout, runs cases serially, and currently
requires macOS because `os.wait4().ru_maxrss` units are treated as bytes.

```bash
git worktree add /tmp/hardviz-query-source \
  7fae38b38116d82d771d6fc3705f67f618218732
cargo build --manifest-path /tmp/hardviz-query-source/Cargo.toml \
  --target-dir /tmp/hardviz-query-source/target \
  -p hardviz-core --release --example archive_format_benchmark
python3 core/examples/archive_format_benchmark/run_query_matrix.py \
  --binary /tmp/hardviz-query-source/target/release/examples/archive_format_benchmark \
  --cwd /tmp/hardviz-query-source \
  --output /tmp/hardviz-query-matrix-2026-09-06
```

The output directory must not exist. The default runner matrix is 24-hour,
30-day, and one-year stable and 30-minute churn, plus the 30-day one-minute
churn stress case. It passes `--group-cap 1000000` and seven repetitions to
every case. Use `--cases NAME...` only for a clearly labeled partial rerun.

## Measured results

The comparison is `C p95 <= max(1.2 * A p95, A p95 + 20 ms)`. Seven samples
make p95 equal the maximum. Cells show `p50 / p95` milliseconds rounded to
three decimals; the artifact retains every input value and full-precision
measurement.

### Process Stats

| Case / range | Groups | A relational | B current chunks | C summaries | Ceiling | Result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| 24h stable / half | 46 | 3.203 / 3.228 | 8.995 / 9.606 | 1.427 / 1.572 | 23.228 | Pass |
| 24h stable / middle third | 46 | 2.181 / 2.244 | 6.595 / 6.856 | 1.905 / 2.004 | 22.244 | Pass |
| 24h churn / half | 360 | 3.424 / 3.447 | 9.993 / 10.319 | 2.188 / 2.350 | 23.447 | Pass |
| 24h churn / middle third | 255 | 2.271 / 2.524 | 7.255 / 7.335 | 2.519 / 2.741 | 22.524 | Pass |
| 30d stable / half | 46 | 140.100 / 145.207 | 271.870 / 287.920 | 6.158 / 6.221 | 174.248 | Pass |
| 30d stable / middle 24h | 46 | 6.537 / 6.878 | 18.473 / 20.247 | 2.490 / 2.607 | 26.878 | Pass |
| 30d churn / half | 10,800 | 158.106 / 167.709 | 300.729 / 318.302 | 31.679 / 32.932 | 201.251 | Pass |
| 30d churn / middle 24h | 735 | 9.736 / 10.544 | 20.519 / 23.346 | 4.308 / 7.669 | 30.544 | Pass |
| 1y stable / half | 46 | 2045.992 / 2092.565 | 3173.324 / 3222.043 | 54.540 / 63.677 | 2511.077 | Pass |
| 1y stable / middle 24h | 46 | 6.931 / 7.393 | 20.281 / 20.468 | 5.232 / 5.405 | 27.393 | Pass |
| 1y churn / half | 131,400 | 2211.280 / 2233.772 | 3611.655 / 3634.386 | 439.966 / 451.477 | 2680.527 | Pass |
| 1y churn / middle 24h | 735 | 10.222 / 12.850 | 22.415 / 22.926 | 6.987 / 13.729 | 32.850 | Pass |
| 30d churn 1m / half | 323,850 | 665.542 / 675.118 | 439.557 / 443.941 | 1055.550 / 1090.628 | 810.141 | **Fail** |
| 30d churn 1m / middle 24h | 21,600 | 42.172 / 47.169 | 26.473 / 26.650 | 69.106 / 83.337 | 67.169 | **Fail** |

The stress case is not a representative executable-name distribution, but it
is a valid counterexample to unconditional summaries: C was 2.46 times B for
the half range and 3.13 times B for the middle range at p95. Its accelerated
first-page p95 was 738.539 ms and 61.673 ms respectively because ranking still
scales with the materialized group count.

### Ambient raw range

| Case / range | B decoded rows | C decoded rows | A relational | B current scan | C bounds | Ceiling | Result |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 24h stable / half | 2,562 | 1,253 | 3.142 / 3.241 | 6.423 / 6.512 | 4.032 / 4.231 | 23.241 | Pass |
| 24h stable / middle third | 2,562 | 982 | 2.665 / 2.894 | 6.128 / 6.190 | 2.982 / 3.221 | 22.894 | Pass |
| 24h churn / half | 2,562 | 1,253 | 3.133 / 3.274 | 6.363 / 6.519 | 4.133 / 4.280 | 23.274 | Pass |
| 24h churn / middle third | 2,562 | 982 | 2.655 / 2.726 | 6.205 / 6.265 | 3.090 / 3.144 | 22.726 | Pass |
| 30d stable / half | 78,489 | 39,217 | 95.432 / 98.158 | 183.739 / 184.501 | 116.227 / 117.790 | 118.158 | Pass |
| 30d stable / middle 24h | 78,489 | 2,726 | 40.058 / 44.778 | 172.115 / 183.206 | 8.702 / 8.809 | 64.778 | Pass |
| 30d churn / half | 78,489 | 39,217 | 97.551 / 106.701 | 192.590 / 239.272 | 124.215 / 127.823 | 128.041 | Pass |
| 30d churn / middle 24h | 78,489 | 2,726 | 40.093 / 40.613 | 178.756 / 179.463 | 9.438 / 9.613 | 60.613 | Pass |
| 1y stable / half | 955,580 | 477,762 | 1143.058 / 1147.794 | 2299.146 / 2325.829 | 1409.080 / 1431.034 | 1377.353 | **Fail** |
| 1y stable / middle 24h | 955,580 | 2,727 | 455.337 / 459.335 | 2156.374 / 2167.152 | 11.141 / 16.903 | 551.202 | Pass |
| 1y churn / half | 955,580 | 477,762 | 1293.361 / 1314.298 | 2399.245 / 2441.892 | 1522.152 / 1542.509 | 1577.158 | Pass |
| 1y churn / middle 24h | 955,580 | 2,727 | 451.657 / 467.735 | 2245.912 / 2269.108 | 11.689 / 17.617 | 561.282 | Pass |
| 30d churn 1m / half | 78,489 | 39,217 | 96.406 / 100.313 | 197.665 / 200.435 | 125.672 / 134.257 | 120.376 | **Fail** |
| 30d churn 1m / middle 24h | 78,489 | 2,726 | 42.106 / 43.112 | 184.295 / 190.586 | 9.377 / 11.256 | 63.112 | Pass |

Ambient bounds reduce narrow-window decoding from 955,580 rows to 2,727 in
the one-year cases and bring C p95 to 16.903–17.617 ms. A half-history request
still selects about half the chunks and rows. The one-year stable half range
missed its comparison ceiling by 53.681 ms, so this evidence supports pruning
as a building block rather than a complete wide-range query design.

### Storage, construction, cardinality, and resources

Sizes use database plus WAL file length in MiB after checkpointing. Auxiliary
growth includes both Process summary/metadata tables and Ambient bounds.

| Case | Process rows | Ambient rows | Summary rows | Relational | Chunks | Auxiliary growth | With metadata | Reduction | Peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 24h-stable | 21,615 | 2,618 | 735 | 2.367 | 0.301 | 0.066 | 0.367 | 84.49% | 29.000 |
| 24h-churn | 21,600 | 2,618 | 705 | 2.762 | 0.309 | 0.078 | 0.387 | 86.00% | 28.984 |
| 30d-stable | 648,446 | 78,546 | 22,198 | 72.027 | 6.906 | 1.824 | 8.730 | 87.88% | 53.016 |
| 30d-churn | 648,000 | 78,546 | 21,585 | 83.547 | 6.914 | 2.250 | 9.164 | 89.03% | 69.609 |
| 1y-stable | 7,889,419 | 955,636 | 270,187 | 883.348 | 83.223 | 22.082 | 105.305 | 88.08% | 68.188 |
| 1y-churn | 7,884,000 | 955,636 | 262,785 | 1027.512 | 83.227 | 27.336 | 110.562 | 89.24% | 305.469 |
| 30d-churn-1m | 648,000 | 78,546 | 647,535 | 83.547 | 9.727 | 61.242 | 70.969 | 15.06% | 676.719 |

The size columns and peak RSS are MiB. Peak RSS is the entire benchmark child
across generation, finalization, validation, metadata construction, and all
queries. It is not incremental query memory and cannot prove or disprove the
256 MiB query-memory gate. The 1y churn and 30d one-minute stress observations
do show that the whole validation harness can exceed 256 MiB at process level;
they do not locate that memory to one query phase. No case swapped.

| Case | Process build | Ambient build | Wall time | User + system CPU |
| --- | ---: | ---: | ---: | ---: |
| 24h-stable | 17.974 ms | 5.703 ms | 1.278 s | 0.964 s |
| 24h-churn | 19.432 ms | 5.680 ms | 0.947 s | 0.993 s |
| 30d-stable | 552.360 ms | 159.668 ms | 22.174 s | 23.153 s |
| 30d-churn | 590.180 ms | 159.275 ms | 23.726 s | 24.890 s |
| 1y-stable | 6568.423 ms | 1966.022 ms | 280.747 s | 282.433 s |
| 1y-churn | 7242.951 ms | 1938.397 ms | 304.308 s | 304.071 s |
| 30d-churn-1m | 1695.104 ms | 161.314 ms | 44.537 s | 48.381 s |

Construction is offline and untimed numerical-probe work is excluded from the
Process build duration. These values do not claim atomic online maintenance.

### Correctness gates

All seven executable cases exited zero. Across them, all seven original
correctness flags passed: exact decoder output, exact persisted records after
reopen, Process query equivalence, Ambient raw-range equivalence, rollback on a
changed selection, exact multiplicity after retry, and a before-or-after
concurrent snapshot. All 98 measured range repetitions also passed exact
generated-workload tuple/count/max/timestamp or digest validation, including
CPU-ranked first pages.

The separate binary64 cancellation probe failed in every case: SQLite's direct
average is `1024.5`, while merging per-chunk binary64 sums yields `1024.0`.
The absolute error `0.5` exceeds the allowed `0.0000000010245`. Therefore all
seven `numerical_contract_passed` and `production_candidate_accepted` flags are
false. Successful generated-workload comparisons and zero exits record a
completed experiment; they do not accept the Process candidate.

## Completed verification

The measured source passed these checks before the matrix was recorded:

```bash
cargo fmt --all -- --check
cargo clippy -p hardviz-core --all-targets -- -D warnings
cargo test -p hardviz-core --example archive_format_benchmark -- --test-threads=1
```

The example target ran 35 tests. The committed matrix has seven zero exit
codes, one source commit across all embedded reports, 98 passing measured-range
comparisons, and false numerical/production acceptance flags in every case.
The documentation-only handoff also parsed the runner with Python `ast`, ran
its `--help` path, parsed the JSON artifact, and compared every value exactly with the original
completed matrix after repository formatting.

## Remaining boundaries

- The simple Process summary representation must be replaced or otherwise proven to preserve the agreed arithmetic contract before acceptance. The tolerance was not loosened.
- Metadata integrity, atomic online construction, invalidation, and maintenance remain production work. These accelerators are built once on a fixed synthetic candidate.
- Process aggregation and ranked paging use SQLite temporary tables, but all pages and canonical copies are materialized for validation. This run cannot certify a history-independent 256 MiB query-memory bound.
- Ambient still materializes every matching row before its first page. Pruning unrelated history does not make first-page work independent of the selected range size.
- Ambient coverage is the raw inclusive endpoint. Actual chart buckets, half-open Cooling reads, paired-minute weights, and source attribution require their own differential tests.
- Ten-year continuous history, controlled cold-cache runs, repeat runs across hosts, and native Windows/Linux performance remain unmeasured.
- This verification does not ratify a production format, enable G2, or complete the full-database storage/migration gates.
