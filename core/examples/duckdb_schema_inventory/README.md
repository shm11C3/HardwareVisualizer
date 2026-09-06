# DuckDB schema inventory probe

This research probe applies the App migration list through the Core migrator to
a temporary synthetic SQLite database, then records its runtime schema and
bounded source-reference inventory.

From the repository root:

```sh
gh api repos/shm11C3/HardwareVisualizer/issues/1666 > /tmp/hardviz-issue-1666.json
PYTHONDONTWRITEBYTECODE=1 python3 core/examples/duckdb_schema_inventory/inventory.py \
  --issue-1666-json /tmp/hardviz-issue-1666.json \
  --cargo-target-dir target/archive-engine-validation/schema-inventory-target \
  --output core/examples/duckdb_schema_inventory/inventory-2026-09-07.json
```

The measured artifact is
[`inventory-2026-09-07.json`](inventory-2026-09-07.json).

The owner, consumer, and transaction annotations are manually reviewed evidence,
not a complete call graph. Their freshness is pinned to hashes of the reviewed
production source files. A changed source hash makes the run incomplete until
the annotation and pinned hash are reviewed again. The probe uses an empty
synthetic database and does not measure user rows, production cardinalities, or
production `sqlite_sequence` high-water state.
