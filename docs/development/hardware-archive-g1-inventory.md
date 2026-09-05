# Hardware Archive G1 Inventory

Status: G1 experimental-development inventory for
[#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052).

This pins the row-based oracle that G1 must benchmark before choosing a format.
It does not enable migration, approve a codec/layout, establish benchmark
results, or satisfy the G1 human-review gate.

## Pinned scope and status

- Revision: `838d08da16d60ead84df3ddb1501ca19a57f24fc` (merged #2070).
- App migration ceiling: version 23.
- [ADR 0019](../adr/0019-lossless-chunked-hardware-archive.md) is accepted.
  [ADR 0021](../adr/0021-hardware-archive-migration-lifecycle.md) and the
  [storage design](hardware-archive-storage-design.md) are proposed design
  inputs. G1 implementation is authorized; format selection and numerical
  budgets still await measurement and maintainer acceptance.
- The #1666 checkpoint includes merged #2070 at this revision, including both
  version-23 covariate tables and consumers. Starting G1 is authorized. This
  makes no claim that the #1666 epic is closed.
- G1 is isolated experimental development. Production recording and queries
  remain relational until later delivery gates change that state.

Design-review outcome: **Aligned with guardrail**. The experiment serves DP-02,
DP-04, DP-05, DP-07, DP-09, and ADR 0019 only while it preserves the stored rows
and query semantics below, keeps Core as persistence owner, and leaves format
selection and budgets pending measurement and maintainer review.

## SQLite baseline

Core opens the App-selected database with WAL, a five-second busy timeout, and
`synchronous=NORMAL`. App supplies append-only SQLx migrations; Core applies
them before persistence workers start. Existing migration SQL/checksums must
not be rewritten.

Version 23 defines 15 product tables and four explicit indexes:
`idx_process_stats_timestamp`, `idx_ambient_archive_timestamp`,
`idx_fan_archive_timestamp`, and `idx_data_archive_timestamp`. It defines no
application trigger or view. Applying the 23 SQL bodies alone with SQLite
3.50.4 produces 16 tables including `sqlite_sequence`, 12 indexes including
eight SQLite autoindexes, zero views, and zero triggers. SQLx separately creates
`_sqlx_migrations`. The benchmark must record its own SQLite version; this
inventory run is not benchmark evidence.

Preflight must inspect actual `sqlite_schema`. An unknown table, column, index,
trigger, view, storage class, or fingerprint refuses optimization while leaving
the source usable.

Sources: [App migration set](../../src-tauri/src/infrastructure/database/migration.rs),
[Core migrator](../../core/src/infrastructure/database/migrate.rs), and
[connection policy](../../core/src/infrastructure/database/db.rs).

### Raw archive tables

These are the five format-candidate families. Preserve row ID, SQLite storage
class and value (including binary64 REAL bits and TEXT bytes), nullness,
multiplicity, and exact timestamp representation. Narrower Rust producer types
do not define the migration domain.

```sql
CREATE TABLE PROCESS_STATS (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  pid INTEGER NOT NULL, process_name TEXT NOT NULL,
  cpu_usage REAL NOT NULL, memory_usage INTEGER NOT NULL,
  execution_sec INTEGER NOT NULL, timestamp DATETIME NOT NULL
);
CREATE INDEX idx_process_stats_timestamp ON PROCESS_STATS(timestamp);

CREATE TABLE DATA_ARCHIVE (
  id INTEGER PRIMARY KEY,
  cpu_avg INTEGER, cpu_max INTEGER, cpu_min INTEGER,
  ram_avg INTEGER, ram_max INTEGER, ram_min INTEGER,
  timestamp DATETIME,
  cpu_temperature_avg REAL, cpu_temperature_max REAL, cpu_temperature_min REAL,
  cpu_power_avg REAL, cpu_power_max REAL, cpu_power_min REAL,
  gpu_power_avg REAL, gpu_power_max REAL, gpu_power_min REAL,
  ane_power_avg REAL, ane_power_max REAL, ane_power_min REAL,
  package_power_avg REAL, package_power_max REAL, package_power_min REAL
);
CREATE INDEX idx_data_archive_timestamp ON DATA_ARCHIVE(timestamp);

CREATE TABLE GPU_DATA_ARCHIVE (
  id INTEGER PRIMARY KEY, gpu_name TEXT,
  usage_avg INTEGER, usage_max INTEGER, usage_min INTEGER,
  temperature_avg INTEGER, temperature_max INTEGER, temperature_min INTEGER,
  timestamp DATETIME,
  dedicated_memory_avg INTEGER, dedicated_memory_max INTEGER,
  dedicated_memory_min INTEGER, gpu_id TEXT
);

CREATE TABLE AMBIENT_ARCHIVE (
  id INTEGER PRIMARY KEY AUTOINCREMENT, source TEXT NOT NULL,
  temperature REAL NOT NULL, humidity REAL, timestamp DATETIME NOT NULL
);
CREATE INDEX idx_ambient_archive_timestamp ON AMBIENT_ARCHIVE(timestamp);

CREATE TABLE FAN_ARCHIVE (
  id INTEGER PRIMARY KEY, source TEXT NOT NULL, rpm INTEGER NOT NULL,
  timestamp DATETIME NOT NULL
);
CREATE INDEX idx_fan_archive_timestamp ON FAN_ARCHIVE(timestamp);
```

`DATA_ARCHIVE` and `GPU_DATA_ARCHIVE` retain old `INTEGER` affinity even
though current writers bind floats to several columns. Compare actual values
and storage classes, not affinity or current `f32` models.

| Family | G1 decision and required preservation |
| --- | --- |
| `PROCESS_STATS` | Required conversion, format unselected. Preserve original `id`, `(pid, process_name)`, values, timestamp, multiplicity. |
| `DATA_ARCHIVE` | Preferred candidate; measurement chooses conversion or relational preservation. Preserve every nullable metric/statistic independently. |
| `GPU_DATA_ARCHIVE` | Preferred candidate; measurement chooses. Preserve name, opaque nullable ID, existing same-name combination, all statistics. |
| `AMBIENT_ARCHIVE` | Preferred candidate; measurement chooses. Preserve source, temperature, nullable humidity, absent minutes, identity/timestamp. |
| `FAN_ARCHIVE` | Preferred candidate; measurement chooses. Preserve fan source, real zero RPM, absent minutes, identity/timestamp. |

Process Stats is mandatory for completed #2052 delivery. G1 must report a
measured conversion or preservation recommendation for every other family.

### Directly preserved relational tables

Copy every row and constraint even when the raw input has expired.

| Table and key | Version-23 shape and reason |
| --- | --- |
| `cooling_daily_summary`, PK `date` | Four bands with nullable temperature avg/max/min and counts; coverage; nullable CPU-power avg/max/min and count. Mutable, independently retained rollup. |
| `cooling_hourly_summary`, PK `hour_start` | Nullable CPU-usage/temperature averages and count. Explorer projection. |
| `cooling_fan_daily_summary`, PK `(date, source)` | Non-null RPM avg/max/min and count. Per-fan identity. |
| `cooling_thermal_delta_daily_summary`, PK `(date, source)` | Coverage plus four bands of nullable Delta avg/max/min and paired counts. Source-separated and baseline protected. |
| `cooling_covariate_daily_summary`, PK `(date, source, band)` | #2070 count/share, ambient median, nullable Delta/power medians and counts, additive power-fit `n`, sums x/y/xy/xx/yy. |
| `cooling_fan_covariate_daily_summary`, PK `(date, source, fan_source, band)` | #2070 RPM count/median and additive fan-fit `n`, sums x/y/xy/xx/yy. |
| `cooling_baseline`, fixed PK `id = 1` | Write-once window dates, idle-temperature average, count, establishment timestamp. |
| `cooling_delta_baseline`, fixed PK `id = 1` | Write-once source, window dates, Delta average, count, establishment timestamp. |
| `storage_devices`, PK `id` | Mutable device identity/display fields, nullable serial/protocol/capacity, seen times, active flag. |
| `storage_health_daily_records`, autoincrement PK `id`, unique `(device_id, date)`, FK | Mutable same-day status/warning and nullable readings/counters, independently retained. |

Exact preserved column sets:

- `cooling_daily_summary`: `date TEXT`; for each of `idle/low/mid/high`,
  nullable `*_cpu_temperature_{avg,max,min} REAL` and non-null
  `*_sample_minutes INTEGER DEFAULT 0`; non-null `coverage_minutes`;
  nullable `cpu_power_{avg,max,min} REAL`; non-null/default-zero power count.
- `cooling_hourly_summary`: `hour_start TEXT`, nullable
  `cpu_usage_avg REAL`, nullable `cpu_temperature_avg REAL`, non-null count.
- `cooling_fan_daily_summary`: non-null `date TEXT`, `source TEXT`,
  `rpm_avg REAL`, `rpm_max/min INTEGER`, and count.
- `cooling_thermal_delta_daily_summary`: non-null date, source, coverage;
  each band has nullable `*_delta_temperature_{avg,max,min} REAL` and a
  non-null/default-zero paired count.
- `cooling_covariate_daily_summary`: non-null date, source, band, sample
  count/share, ambient median; nullable Delta/power medians with non-null counts;
  non-null/default-zero power-fit `n` and sums x/y/xy/xx/yy.
- `cooling_fan_covariate_daily_summary`: non-null date, ambient source, fan
  source, band, RPM count/median; non-null/default-zero fit `n` and sums.
- `cooling_baseline`: fixed integer ID; non-null window dates, idle average,
  sample count, and establishment timestamp.
- `cooling_delta_baseline`: fixed integer ID; non-null source, window dates,
  Delta average, sample count, and establishment timestamp.
- `storage_devices`: non-null text ID/display/first-seen/last-seen and integer
  active flag; nullable model, serial hash, protocol, and integer capacity.
- `storage_health_daily_records`: integer ID; non-null device/date/status,
  warning level and collection timestamp; nullable warning JSON, temperature,
  hours, wear/spare, sector/media/error-log, and unsafe-shutdown fields.

Preserve every `_sqlx_migrations` row, including checksum/execution state;
append real migrations rather than fabricate success. Preserve
`sqlite_sequence` high-water marks for `PROCESS_STATS`, `AMBIENT_ARCHIVE`,
and `storage_health_daily_records`. Converted tails for tables without
`AUTOINCREMENT` also need monotonic non-reused record sequences. Recreate all
known indexes and constraints; unknown objects require classification or refusal.

Source for all product table and index definitions:
[App migration set](../../src-tauri/src/infrastructure/database/migration.rs).

## Producer and identity contracts

`ArchiveTracker` retains up to 60 per-second EventBus samples. It writes every
60 seconds and flushes a dirty partial interval on normal shutdown. One
`Utc::now()` is shared across system, GPU, fan, process, and ambient writes.
Instants are still irregular: delayed ticks, shutdown, restarts, clock changes,
and legacy writes remain representable. Never reconstruct
`start + interval * index` or deduplicate an instant.

### Process Stats

Sysinfo produces `ProcessSample { pid: u32, name: String, cpu_usage: f32,
memory_kb: f32, run_time_secs: u64 }`. The tracker keys by PID, clears its
rings on a name change, and retains a vanished PID for up to 60 ingest ticks.
Same-name PID reuse is intentionally indistinguishable.

At each write it requires CPU and memory samples, omits only a both-zero row,
normalizes CPU by logical-core count, rounds memory to `i32`, casts run time
to `i32`, and admits execution values through 30 days. It takes five from each
CPU/memory/execution descending ranking and deduplicates by PID, for at most 15
rows, committed in one transaction.

Persisted identity is observation `id` plus subject `(pid, process_name)`.
Neither identifies a process lifetime. Queries retain both fields; dictionaries
are only chunk-local compression keys.

### Representative sensor: ambient

`EnvironmentalSensorRegistry::fresh_readings(tick)` supplies producer Source
Labels, `f32` Celsius temperature, optional `f32` humidity, and no stored
observation time. Rows use the shared tick. Stale/unavailable/invalid/missing
readings create no row. Multiple labels at one tick are distinct identities.

Fixtures need distinct labels, nullable humidity, multiple sources at one
instant, missing intervals, duplicate instants, irregular timestamp forms, and
SQLite REAL values that do not round-trip through `f32`.

### Other families

- System has seven nullable `f32` avg/max/min triples: CPU, memory, CPU
  temperature, CPU/GPU/ANE/package power. Preserve all 21 fields.
- GPU histories use live IDs. The writer groups by reported name, combines raw
  samples with sample weighting, emits one row per name, and stores an ID only
  for one contributing live ID. Queries select by name. IDs stay opaque; live
  and inventory namespaces are disjoint, so migration performs no identity join.
  Live forms include Windows `nvapi:<id>`,
  `pci:<bus>:<device>:<function>`, `pdh:instance:<device_instance_id>` and
  fallback `pdh:<luid_high>:<luid_low>`; Linux `pci:<BDF>` and fallback
  `drm:card<n>`; and macOS `iokit:<name>`. Legacy strings and null remain
  exactly as recorded.
- Fan history keys on `MotherboardFanSpeed.name`, stored as `source`; the
  provider's separate `source` is not stored. Zero is real; invalid/missing is
  absent. The minute RPM uses integer division of a `u64` sum by present count.
- Storage Health uses an HMAC-based device ID shared by daily and Live Storage
  Health. Preserve IDs, FK/unique constraints, upserts, active changes,
  deletions, and key changes outside raw archive conversion.

Sources: [archive producer](../../core/src/persistence/archive.rs),
[stored row types](../../core/src/persistence/archive_data.rs), and
[system/process sampling](../../core/src/collector/sampling.rs).

## Query and arithmetic oracle

Semantics are intentionally non-uniform. Differential tests must preserve them.

### Hardware, GPU, and Process

System/GPU series use inclusive TEXT `BETWEEN` after App RFC-3339 parsing and
millisecond `Z` normalization; GPU filters by name. SQL casts values to REAL
and re-applies the requested statistic per bucket: avg uses `AVG`, max
`MAX`, min `MIN`. Start placement floors and end placement ceils epoch
milliseconds. Results are gap-filled and capped at 10,000 points. GPU names are
sorted distinct non-null names excluding literal `Unknown`. Consumers are
Hardware Insight, Insight Snapshot CPU/RAM, and Cooling timelines up to 30 days.

Process queries use inclusive TEXT `BETWEEN`, group by `(pid, process_name)`,
and return CPU/memory averages, max execution seconds, and max timestamp.
`get_process_stats_in_period` orders CPU descending. `get_process_stats`
does not order and subtracts 60 seconds from its supplied end before deriving
the period. The current unbounded result includes every group and serves Process
Insight and Insight Snapshot; G2 plans bounded paging without a new top-N rule.

### Fan, ambient, and Cooling

Fan series uses inclusive epoch-millisecond membership, groups by source/bucket,
averages RPM, orders, and fills gaps per source.

Ambient series reads buckets and sources in one transaction. It first averages
ambient rows per minute and CPU temperatures per minute, then left-joins CPU.
Buckets average ambient-minute means and paired per-minute `cpu - ambient`.
Ambient-only minutes affect ambient but not Delta. Sources are distinct/sorted;
downstream code must not subtract independent bucket averages.

Cooling raw reads use half-open `[start, end)` epoch-millisecond membership; a
one-day widened TEXT bound is only an index prefilter. Daily system input reads
CPU usage, temperature triple, and power triple. Temperature bands require
usage plus a complete temperature triple; power folds independently with a
complete triple. Thermal Delta joins each hardware record to ambient averaged
per `(minute, source)`. Fan rollup rows are ordered by timestamp then ID.

Catch-up probes depend on earliest hardware timestamp, summary max dates,
latest complete powered row, latest fan row, latest pairable ambient row, and
latest classifiable pairable ambient row. Route every probe through mixed
storage before emptying raw tables. One day's daily, hourly, fan, Delta, and
both covariate projections upsert in one transaction.

| App command | Core sources |
| --- | --- |
| `get_cooling_trend` | all daily system summaries |
| `get_cooling_fan_trend` | all fan summaries plus `EXISTS(FAN_ARCHIVE)` |
| `get_cooling_band_comparison` | daily system and source-specific Delta summaries |
| `get_cooling_baseline_delta` | daily system/Delta summaries and pinned baselines |
| `get_cooling_load_temperature_explorer` | daily idle and bounded hourly rows |
| `get_cooling_covariate_comparison` | Delta and both v23 covariate tables, read sequentially through one Core pool; current code does not wrap these reads in one transaction |

The complete direct raw-table read map is: `DATA_ARCHIVE` series, daily input,
Thermal Delta/ambient pairing, earliest/powered catch-up, and retention;
`GPU_DATA_ARCHIVE` series, name enumeration, and retention; `PROCESS_STATS`
grouped results and retention; `AMBIENT_ARCHIVE` timeline, pairing/catch-up,
and retention; `FAN_ARCHIVE` timeline, daily/covariate input,
capability/catch-up probes, and retention.

Sources: [archive series and Process Stats queries](../../core/src/infrastructure/database/archive_queries.rs),
[daily raw reader](../../core/src/infrastructure/database/cooling_daily_summary.rs),
[Thermal Delta raw reader](../../core/src/infrastructure/database/cooling_thermal_delta_daily_summary.rs),
[fan raw reader](../../core/src/infrastructure/database/fan_archive.rs),
[rollup coordinator](../../core/src/persistence/cooling_rollup.rs),
[App archive service](../../src-tauri/src/services/archive_history_service.rs),
[App archive commands](../../src-tauri/src/commands/hardware.rs),
[App Cooling service](../../src-tauri/src/services/cooling_insight_service.rs),
and [App Cooling commands](../../src-tauri/src/commands/cooling_insight.rs).

Cooling dates/hours are local-calendar ISO TEXT. Daily Cooling retention is
independently fixed at 400 days. Delta/covariate tables protect the pinned Delta
baseline source/window. Both baselines may outlive all contributing input and
cannot be regenerated as a copy shortcut.

## G1 evidence still required

This inventory unblocks candidate work; it does not pass G1. The report still
needs reproducible 24-hour, 30-day, one-year, and ten-year synthetic datasets;
exact Process Stats and sensor round trips; differential tests for every
boundary; and layout/limit/codec/compression/index comparisons.

Measure and ratify or revise disk targets, small-history overhead, p50/p95/p99
query and append latency, CPU, peak RSS, migration throughput, temporary space,
ten-year paging, and cutover. Record commit, seed, hardware, OS/filesystem,
SQLite/Rust versions, repetitions, and cold/warm policy. No benchmark,
compression, lifecycle acceptance, or #2052 completion claim exists until
results and maintainer review are recorded.
