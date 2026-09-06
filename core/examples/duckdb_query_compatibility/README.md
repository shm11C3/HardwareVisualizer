# DuckDB query compatibility probe

This bounded investigation compares the current Process and ambient archive
query shapes using SQLite 3.46.0 and DuckDB 1.5.5. It checks exact cell
round trips for the typed plus exception-sidecar candidate, then either runs
the covered query or refuses it before selecting DuckDB. The tagged
representation has no query evaluator and is also refused.

Create or activate a Python environment containing `pysqlite3==0.5.4` linked
to SQLite 3.46.0 and `duckdb==1.5.5`, then run from the repository root:

```bash
PYTHONDONTWRITEBYTECODE=1 python \
  core/examples/duckdb_query_compatibility/probe.py \
  --output target/archive-engine-validation/duckdb-query-compatibility-smoke
```

The output directory must not exist. Exit zero means the diagnostic and exact
round-trip checks completed. Acceptance is represented only by the JSON flags.
The encoding is reproduced locally from the immutable
[`duckdb_value_preservation.py` source at commit `58fad7ad54263079c1c2a74aa1b8396fcdcbd344`](https://github.com/shm11C3/HardwareVisualizer/blob/58fad7ad54263079c1c2a74aa1b8396fcdcbd344/core/examples/archive_engine_benchmark/duckdb_value_preservation.py);
this adds no production dependency or migration.

The retained final smoke evidence is
[`result-2026-09-07.json`](result-2026-09-07.json). Its captured process outcome
is in [`validation-2026-09-07.json`](validation-2026-09-07.json).
