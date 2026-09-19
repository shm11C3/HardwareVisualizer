#![cfg(feature = "duckdb-archive")]
//! Recurring expiry, checkpoint and space-reuse measurement for the
//! implemented native backend (#2089).
//!
//! Ignored by default: it runs minutes of simulated archive traffic and exists
//! to produce evidence, not to gate a build. Everything goes through the
//! production path - App's migrations, the SQLite writers for the converted
//! history, the #2088 candidate builder, finalization, and then the native
//! writers and `delete_old_data` of every raw archive family through the
//! blocking owner. Run it with
//!
//! ```text
//! cargo test -p hardviz-core --features duckdb-archive --release \
//!   --test duckdb_retention_probe -- --ignored --nocapture
//! ```
//!
//! Scale is steered by `HV_PROBE_*` variables (see [`Scale`]);
//! `HV_DUCKDB_RETENTION_PROBE_OUT` names the JSON artifact, otherwise the JSON
//! is printed to stdout.
//!
//! The simulated timeline ends at the wall clock, because every
//! `delete_old_data` derives its cutoff from `Utc::now()`. A day of simulated
//! time is therefore expired by shrinking the Retention Period argument by one
//! day per simulated day; the cutoff each call actually used is bracketed by
//! the texts rendered immediately before and after it.

mod native_support;

use std::path::Path;
use std::str::FromStr;
use std::time::Instant;

use chrono::{Duration, Timelike, Utc};
use hardviz_core::infrastructure::database::native_database::{
  NativeCancellation, NativeDatabase, NativeDatabaseError,
  ambient_archive as native_ambient, data_archive as native_data,
  fan_archive as native_fan, gpu_archive as native_gpu, process_stats as native_process,
};
use hardviz_core::infrastructure::database::{
  ambient_archive, db, fan_archive, gpu_archive, hardware_archive, migrate, process_stats,
};
use hardviz_core::persistence::archive_data::{
  AmbientData, FanArchiveRow, GpuData, HardwareArchiveRow, HardwareData, ProcessStatData,
};
use native_support::{NativeFixture, app_migrations};
use serde_json::{Value, json};

const MINUTES_PER_DAY: u32 = 1440;

/// The raw archive tables in the order production expires them
/// (`persistence::archive::cleanup_old_data`).
const EXPIRY_ORDER: [&str; 5] = [
  "DATA_ARCHIVE",
  "GPU_DATA_ARCHIVE",
  "FAN_ARCHIVE",
  "PROCESS_STATS",
  "AMBIENT_ARCHIVE",
];

/// Probe scale, read from the environment so one binary serves a smoke run
/// and the evidence run.
#[derive(Clone, Copy, Debug)]
struct Scale {
  /// Days of history written through the SQLite writers before conversion.
  seed_days: u32,
  /// Days of minute cycles written natively after conversion.
  native_days: u32,
  /// The Retention Period every family is expired against.
  retention_days: u32,
  /// Write cycles per simulated day; must divide 1440.
  cycles_per_day: u32,
  /// Process rows per cycle.
  processes: u32,
  /// Issue an explicit `CHECKPOINT` after each daily expiry.
  explicit_checkpoint: bool,
}

impl Scale {
  fn from_env() -> Self {
    let scale = Self {
      seed_days: env_or("HV_PROBE_SEED_DAYS", 7),
      native_days: env_or("HV_PROBE_NATIVE_DAYS", 7),
      retention_days: env_or("HV_PROBE_RETENTION_DAYS", 7),
      cycles_per_day: env_or("HV_PROBE_CYCLES_PER_DAY", MINUTES_PER_DAY),
      processes: env_or("HV_PROBE_PROCESSES", 20),
      explicit_checkpoint: env_or::<u8>("HV_PROBE_CHECKPOINT", 0) != 0,
    };
    assert!(
      scale.cycles_per_day > 0 && MINUTES_PER_DAY.is_multiple_of(scale.cycles_per_day)
    );
    assert!(scale.native_days > 0);
    scale
  }

  fn step(&self) -> Duration {
    Duration::minutes(i64::from(MINUTES_PER_DAY / self.cycles_per_day))
  }

  fn seed_cycles(&self) -> u32 {
    self.seed_days * self.cycles_per_day
  }

  fn json(&self) -> Value {
    json!({
      "seed_days": self.seed_days,
      "native_days": self.native_days,
      "retention_days": self.retention_days,
      "cycles_per_day": self.cycles_per_day,
      "processes_per_cycle": self.processes,
      "explicit_checkpoint": self.explicit_checkpoint,
    })
  }
}

fn env_or<T: FromStr>(name: &str, default: T) -> T
where
  T::Err: std::fmt::Debug,
{
  std::env::var(name)
    .ok()
    .map(|value| {
      value
        .parse()
        .unwrap_or_else(|error| panic!("{name}: {error:?}"))
    })
    .unwrap_or(default)
}

#[tokio::test]
#[ignore = "evidence probe: minutes of simulated writes, run explicitly"]
async fn measure_recurring_expiry_checkpoint_and_reuse() {
  let scale = Scale::from_env();
  let fixture = NativeFixture::new();
  assert!(db::init(fixture.source.clone()));
  migrate::run(app_migrations::get_migrations())
    .await
    .unwrap();

  let step = scale.step();
  let end = Utc::now()
    .with_second(0)
    .and_then(|instant| instant.with_nanosecond(0))
    .unwrap();
  let total_cycles = (scale.seed_days + scale.native_days) * scale.cycles_per_day;
  let start = end - step * i32::try_from(total_cycles).unwrap();

  // Converted history, through the SQLite writers.
  let seed_started = Instant::now();
  for cycle in 0..scale.seed_cycles() {
    let instant = start + step * i32::try_from(cycle).unwrap();
    process_stats::insert(process_rows(cycle, scale.processes), instant)
      .await
      .unwrap();
    hardware_archive::insert(archive_row(cycle), instant)
      .await
      .unwrap();
    gpu_archive::insert(gpu_row(cycle), instant).await.unwrap();
    ambient_archive::insert(ambient_rows(cycle), instant)
      .await
      .unwrap();
    fan_archive::insert(fan_rows(cycle), instant).await.unwrap();
  }
  let seed_seconds = seed_started.elapsed().as_secs_f64();
  let source_bytes = file_bytes(&fixture.source);
  let source_wal_bytes = file_bytes(&sidecar(&fixture.source, "-wal"));

  let conversion_started = Instant::now();
  let report = fixture.finalize().await;
  let conversion_seconds = conversion_started.elapsed().as_secs_f64();
  let candidate_bytes = file_bytes(&fixture.candidate);

  let open_started = Instant::now();
  let database = fixture.open().await;
  let open_ms = open_started.elapsed().as_secs_f64() * 1000.0;
  let engine = engine_facts(&database).await;
  let baseline = snapshot(&database, &fixture.finalized).await;

  let mut days = Vec::new();
  for day in 1..=scale.native_days {
    let mut cycle_ms = Vec::with_capacity(scale.cycles_per_day as usize);
    for offset in 0..scale.cycles_per_day {
      let cycle = scale.seed_cycles() + (day - 1) * scale.cycles_per_day + offset;
      let instant = start + step * i32::try_from(cycle).unwrap();
      let started = Instant::now();
      native_process::insert(
        &database,
        NativeCancellation::new(),
        process_rows(cycle, scale.processes),
        instant,
      )
      .await
      .unwrap();
      native_data::insert(
        &database,
        NativeCancellation::new(),
        archive_row(cycle),
        instant,
      )
      .await
      .unwrap();
      native_gpu::insert(
        &database,
        NativeCancellation::new(),
        gpu_row(cycle),
        instant,
      )
      .await
      .unwrap();
      native_ambient::insert(
        &database,
        NativeCancellation::new(),
        ambient_rows(cycle),
        instant,
      )
      .await
      .unwrap();
      native_fan::insert(
        &database,
        NativeCancellation::new(),
        fan_rows(cycle),
        instant,
      )
      .await
      .unwrap();
      cycle_ms.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let after_appends = snapshot(&database, &fixture.finalized).await;

    // Simulated time now stands `native_days - day` days before the wall
    // clock, so this argument places the cutoff exactly `retention_days` of
    // simulated time back.
    let retention_argument = scale.retention_days + scale.native_days - day;
    let cutoff_before = native_process::sqlite_timestamp_text(
      &(Utc::now() - Duration::days(i64::from(retention_argument))),
    );
    let upper_bounds = surviving_counts(&database, &cutoff_before).await;
    let mut expiry = Vec::new();
    for table in EXPIRY_ORDER {
      let started = Instant::now();
      let deleted = expire(&database, table, retention_argument).await;
      expiry.push(json!({
        "table": table,
        "deleted_rows": deleted,
        "ms": started.elapsed().as_secs_f64() * 1000.0,
      }));
    }
    let cutoff_after = native_process::sqlite_timestamp_text(
      &(Utc::now() - Duration::days(i64::from(retention_argument))),
    );
    let lower_bounds = surviving_counts(&database, &cutoff_after).await;
    let after_expiry = snapshot(&database, &fixture.finalized).await;

    let mut retained = serde_json::Map::new();
    for (index, table) in EXPIRY_ORDER.iter().enumerate() {
      let (rows, oldest) = table_rows_and_oldest(&database, table).await;
      assert!(
        lower_bounds[index] <= rows && rows <= upper_bounds[index],
        "{table}: {rows} rows retained, expected within [{}, {}]",
        lower_bounds[index],
        upper_bounds[index]
      );
      assert!(
        oldest
          .as_deref()
          .is_none_or(|oldest| oldest >= cutoff_before.as_str()),
        "{table}: a row stamped {oldest:?} survived a cutoff no later than {cutoff_before}"
      );
      retained.insert(
        (*table).to_owned(),
        json!({ "rows": rows, "oldest_timestamp": oldest }),
      );
    }

    let (checkpoint, after_checkpoint) = if scale.explicit_checkpoint {
      let started = Instant::now();
      database
        .request_write(NativeCancellation::new(), |context| {
          context
            .connection()
            .execute_batch("CHECKPOINT")
            .map_err(|error| duck_error("checkpoint", error))
        })
        .await
        .unwrap();
      (
        json!({ "ms": started.elapsed().as_secs_f64() * 1000.0 }),
        snapshot(&database, &fixture.finalized).await,
      )
    } else {
      (Value::Null, Value::Null)
    };

    days.push(json!({
      "day": day,
      "retention_argument_days": retention_argument,
      "cutoff_before": cutoff_before,
      "cutoff_after": cutoff_after,
      "append": latency(&cycle_ms),
      "after_appends": after_appends,
      "expiry": expiry,
      "after_expiry": after_expiry,
      "retained": retained,
      "checkpoint": checkpoint,
      "after_checkpoint": after_checkpoint,
    }));
  }

  let counts_before_close = all_counts(&database).await;
  let close_started = Instant::now();
  database.close().await.unwrap();
  let close_ms = close_started.elapsed().as_secs_f64() * 1000.0;
  let after_close = file_snapshot(&fixture.finalized);

  let reopen_started = Instant::now();
  let reopened = fixture.open().await;
  let reopen_ms = reopen_started.elapsed().as_secs_f64() * 1000.0;
  let counts_after_reopen = all_counts(&reopened).await;
  assert_eq!(counts_before_close, counts_after_reopen);
  let after_reopen = snapshot(&reopened, &fixture.finalized).await;
  reopened.close().await.unwrap();

  let compact = compact_copy(&fixture.finalized, fixture.directory.path());

  let output = json!({
    "schema_version": 1,
    "scope": "Recurring expiry, checkpoint and space-reuse measurement of the implemented native DuckDB backend (#2089). Converted history is written through the SQLite production writers, converted through the candidate builder and finalization, then appended to and expired through the native writers and delete_old_data of every raw archive family via the blocking owner. Synthetic fixture; not a whole-application budget.",
    "date": Utc::now().to_rfc3339(),
    "issue": 2089,
    "host": host_facts(),
    "engine": engine,
    "scale": scale.json(),
    "timeline": {
      "start": start.to_rfc3339(),
      "end": end.to_rfc3339(),
      "step_minutes": MINUTES_PER_DAY / scale.cycles_per_day,
      "seed_cycles": scale.seed_cycles(),
      "native_cycles": scale.native_days * scale.cycles_per_day,
    },
    "seed": {
      "seconds": seed_seconds,
      "source_bytes": source_bytes,
      "source_wal_bytes": source_wal_bytes,
    },
    "conversion": {
      "seconds": conversion_seconds,
      "candidate_bytes": candidate_bytes,
      "finalized_bytes": report.finalized_bytes,
      "total_rows": report.total_rows,
      "tables": report.tables.iter().map(|table| json!({
        "name": table.name,
        "rows": table.copied_rows,
      })).collect::<Vec<_>>(),
    },
    "open_ms": open_ms,
    "baseline": baseline,
    "days": days,
    "close": { "ms": close_ms, "files": after_close },
    "reopen": { "ms": reopen_ms, "snapshot": after_reopen, "counts_match": true },
    "compact_copy": compact,
    "limits": [
      "Single run; latencies include the in-memory SQLite stamp oracle each native write cycle runs and the tokio/channel hop into the blocking owner.",
      "Simulated time is compressed: a day's cycles are written back to back, so automatic checkpoint behaviour reflects WAL bytes, not elapsed time.",
      "delete_old_data derives its cutoff from the wall clock; the cutoff each call used lies between cutoff_before and cutoff_after.",
      "The compact copy is taken outside the blocking owner with a plain read-write connection and is evidence of the reclaimable bound, not a shipped maintenance operation.",
      "Resident set size is the whole test process, including the SQLite pool that seeded the history.",
    ],
  });

  // Trailing newline: the artifact is committed under `docs/` and formatted
  // there by `biome ci`.
  let rendered = format!("{}\n", serde_json::to_string_pretty(&output).unwrap());
  match std::env::var("HV_DUCKDB_RETENTION_PROBE_OUT") {
    Ok(path) => std::fs::write(&path, rendered).unwrap(),
    Err(_) => print!("{rendered}"),
  }
}

async fn expire(database: &NativeDatabase, table: &str, retention_days: u32) -> u64 {
  let cancellation = NativeCancellation::new();
  match table {
    "DATA_ARCHIVE" => {
      native_data::delete_old_data(database, cancellation, retention_days).await
    }
    "GPU_DATA_ARCHIVE" => {
      native_gpu::delete_old_data(database, cancellation, retention_days).await
    }
    "FAN_ARCHIVE" => {
      native_fan::delete_old_data(database, cancellation, retention_days).await
    }
    "PROCESS_STATS" => {
      native_process::delete_old_data(database, cancellation, retention_days).await
    }
    "AMBIENT_ARCHIVE" => {
      native_ambient::delete_old_data(database, cancellation, retention_days).await
    }
    other => panic!("no expiry for {other}"),
  }
  .unwrap()
}

/// Rows per table stamped at or after `cutoff`, in [`EXPIRY_ORDER`].
async fn surviving_counts(database: &NativeDatabase, cutoff: &str) -> Vec<u64> {
  let cutoff = cutoff.to_owned();
  database
    .request_read(NativeCancellation::new(), move |context| {
      EXPIRY_ORDER
        .iter()
        .map(|table| {
          context
            .connection()
            .query_row(
              &format!("SELECT count(*) FROM {table} WHERE timestamp >= ?"),
              [cutoff.as_str()],
              |row| row.get::<_, i64>(0),
            )
            .map(|count| count as u64)
            .map_err(|error| duck_error("count survivors", error))
        })
        .collect()
    })
    .await
    .unwrap()
}

async fn table_rows_and_oldest(
  database: &NativeDatabase,
  table: &'static str,
) -> (u64, Option<String>) {
  database
    .request_read(NativeCancellation::new(), move |context| {
      context
        .connection()
        .query_row(
          &format!("SELECT count(*), min(timestamp) FROM {table}"),
          [],
          |row| {
            Ok((
              row.get::<_, i64>(0)? as u64,
              row.get::<_, Option<String>>(1)?,
            ))
          },
        )
        .map_err(|error| duck_error("count rows", error))
    })
    .await
    .unwrap()
}

async fn all_counts(database: &NativeDatabase) -> Vec<u64> {
  let mut counts = Vec::new();
  for table in EXPIRY_ORDER {
    counts.push(table_rows_and_oldest(database, table).await.0);
  }
  counts
}

/// File bytes, the engine's own block accounting and process memory at one
/// moment, read through the owner so nothing opens a second instance.
async fn snapshot(database: &NativeDatabase, path: &Path) -> Value {
  let files = file_snapshot(path);
  let counts = all_counts(database).await;
  let accounting = database
    .request_read(NativeCancellation::new(), |context| {
      context
        .connection()
        .query_row(
          "SELECT database_size, block_size, total_blocks, used_blocks, free_blocks, \
           wal_size, memory_usage, memory_limit \
           FROM pragma_database_size() WHERE database_name = current_database()",
          [],
          |row| {
            Ok(json!({
              "database_size": row.get::<_, String>(0)?,
              "block_size": row.get::<_, i64>(1)?,
              "total_blocks": row.get::<_, i64>(2)?,
              "used_blocks": row.get::<_, i64>(3)?,
              "free_blocks": row.get::<_, i64>(4)?,
              "wal_size": row.get::<_, String>(5)?,
              "memory_usage": row.get::<_, String>(6)?,
              "memory_limit": row.get::<_, String>(7)?,
            }))
          },
        )
        .map_err(|error| duck_error("read database_size", error))
    })
    .await
    .unwrap();
  json!({
    "files": files,
    "rows": EXPIRY_ORDER
      .iter()
      .zip(counts)
      .map(|(table, count)| ((*table).to_owned(), json!(count)))
      .collect::<serde_json::Map<_, _>>(),
    "database_size": accounting,
    "resident_bytes": resident_bytes(),
  })
}

fn file_snapshot(path: &Path) -> Value {
  let database = file_bytes(path);
  let wal = file_bytes(&sidecar(path, ".wal"));
  json!({
    "database_bytes": database,
    "wal_bytes": wal,
    "total_bytes": database + wal,
  })
}

fn sidecar(path: &Path, suffix: &str) -> std::path::PathBuf {
  let mut name = path.file_name().unwrap().to_os_string();
  name.push(suffix);
  path.with_file_name(name)
}

fn file_bytes(path: &Path) -> u64 {
  std::fs::metadata(path)
    .map(|metadata| metadata.len())
    .unwrap_or(0)
}

fn resident_bytes() -> Option<u64> {
  let pid = sysinfo::get_current_pid().ok()?;
  let mut system = sysinfo::System::new();
  system.refresh_processes_specifics(
    sysinfo::ProcessesToUpdate::Some(&[pid]),
    true,
    sysinfo::ProcessRefreshKind::nothing().with_memory(),
  );
  system.process(pid).map(|process| process.memory())
}

fn latency(samples: &[f64]) -> Value {
  let mut sorted = samples.to_vec();
  sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
  let at = |quantile: f64| {
    let index = ((sorted.len() as f64 - 1.0) * quantile).round() as usize;
    sorted[index]
  };
  json!({
    "cycles": sorted.len(),
    "p50_ms": at(0.5),
    "p95_ms": at(0.95),
    "p99_ms": at(0.99),
    "max_ms": sorted.last().copied().unwrap_or(0.0),
    "mean_ms": sorted.iter().sum::<f64>() / sorted.len().max(1) as f64,
    "total_s": sorted.iter().sum::<f64>() / 1000.0,
  })
}

async fn engine_facts(database: &NativeDatabase) -> Value {
  database
    .request_read(NativeCancellation::new(), |context| {
      let connection = context.connection();
      let library: String = connection
        .query_row("SELECT library_version FROM pragma_version()", [], |row| {
          row.get(0)
        })
        .map_err(|error| duck_error("read version", error))?;
      let storage: String = connection
        .query_row(
          "SELECT storage_version FROM __hv_native_metadata",
          [],
          |row| row.get(0),
        )
        .map_err(|error| duck_error("read storage version", error))?;
      let settings: Vec<(String, String)> = connection
        .prepare(
          "SELECT name, value FROM duckdb_settings() WHERE name IN \
           ('checkpoint_threshold', 'wal_autocheckpoint', 'threads', 'max_memory', \
            'storage_compatibility_version')",
        )
        .and_then(|mut statement| {
          statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect()
        })
        .map_err(|error| duck_error("read settings", error))?;
      Ok(json!({
        "library_version": library,
        "recorded_storage_version": storage,
        "settings": settings
          .into_iter()
          .map(|(name, value)| (name, Value::String(value)))
          .collect::<serde_json::Map<_, _>>(),
      }))
    })
    .await
    .unwrap()
}

fn host_facts() -> Value {
  let mut system = sysinfo::System::new();
  system.refresh_memory();
  system.refresh_cpu_all();
  json!({
    "os": sysinfo::System::long_os_version(),
    "target_os": std::env::consts::OS,
    "target_arch": std::env::consts::ARCH,
    "cpu": system.cpus().first().map(|cpu| cpu.brand().to_owned()),
    "logical_cpus": system.cpus().len(),
    "physical_memory_bytes": system.total_memory(),
    "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
  })
}

/// A fresh copy of the closed database, as the reclaimable bound.
fn compact_copy(finalized: &Path, directory: &Path) -> Value {
  let compact = directory.join("compact.duckdb");
  let before = file_snapshot(finalized);
  let started = Instant::now();
  {
    let config = duckdb::Config::default()
      .access_mode(duckdb::AccessMode::ReadWrite)
      .unwrap()
      .with("storage_compatibility_version", "v0.10.2")
      .unwrap();
    let connection = duckdb::Connection::open_with_flags(finalized, config).unwrap();
    let name: String = connection
      .query_row("SELECT current_database()", [], |row| row.get(0))
      .unwrap();
    connection
      .execute_batch(&format!(
        "ATTACH '{}' AS compact; COPY FROM DATABASE \"{name}\" TO compact; \
         CHECKPOINT compact; DETACH compact",
        compact
          .display()
          .to_string()
          .replace('\\', "/")
          .replace('\'', "''")
      ))
      .unwrap();
  }
  json!({
    "ms": started.elapsed().as_secs_f64() * 1000.0,
    "source_before": before,
    "compact_bytes": file_bytes(&compact),
  })
}

/// The probe's own statements are not production operations, so their
/// failures are reported through the owner's generic worker error.
fn duck_error(action: &'static str, error: duckdb::Error) -> NativeDatabaseError {
  NativeDatabaseError::Worker {
    message: format!("{action}: {error}"),
  }
}

// ── Fixture rows: minute-shaped, fractional readings like the collectors ──

fn process_rows(cycle: u32, count: u32) -> Vec<ProcessStatData> {
  (0..count)
    .map(|index| ProcessStatData {
      pid: 1000 + index as i32,
      process_name: format!("process-{index:02}"),
      cpu_usage: ((cycle + index * 7) % 1000) as f32 / 10.0,
      memory_usage: 100_000 + ((cycle * 37 + index * 1013) % 500_000) as i32,
      execution_sec: (cycle % 86_400) as i32,
    })
    .collect()
}

fn archive_row(cycle: u32) -> HardwareArchiveRow {
  let seed = (cycle % 600) as f32 / 10.0;
  HardwareArchiveRow {
    cpu: reading(seed, seed + 12.5, seed / 4.0),
    memory: reading(40.0 + seed / 3.0, 45.0 + seed / 3.0, 38.0),
    cpu_temperature: reading(45.0 + seed / 2.0, 52.0 + seed / 2.0, 41.0),
    cpu_power: reading(15.0 + seed, 30.0 + seed, 8.0),
    gpu_power: absent(),
    ane_power: absent(),
    package_power: reading(25.0 + seed, 50.0 + seed, 12.0),
  }
}

fn gpu_row(cycle: u32) -> GpuData {
  let seed = (cycle % 900) as f32 / 10.0;
  GpuData {
    gpu_id: Some("gpu-probe-0".to_owned()),
    gpu_name: "Probe GPU".to_owned(),
    usage_avg: Some(seed),
    usage_max: Some(seed + 5.5),
    usage_min: Some(seed / 2.0),
    temperature_avg: Some(40.0 + seed / 10.0),
    temperature_max: Some(60),
    temperature_min: Some(30),
    dedicated_memory_avg: Some(1024 + (cycle % 512) as i32),
    dedicated_memory_max: Some(2048),
    dedicated_memory_min: Some(512),
  }
}

fn ambient_rows(cycle: u32) -> Vec<AmbientData> {
  vec![AmbientData {
    source: "Room".to_owned(),
    temperature: 20.0 + (cycle % 120) as f32 / 20.0,
    humidity: Some(45.0 + (cycle % 30) as f32 / 3.0),
  }]
}

fn fan_rows(cycle: u32) -> Vec<FanArchiveRow> {
  vec![
    FanArchiveRow {
      source: "Exhaust".to_owned(),
      rpm: 800 + cycle % 400,
    },
    FanArchiveRow {
      source: "Intake".to_owned(),
      rpm: 1100 + cycle % 300,
    },
  ]
}

fn reading(avg: f32, max: f32, min: f32) -> HardwareData {
  HardwareData {
    avg: Some(avg),
    max: Some(max),
    min: Some(min),
  }
}

fn absent() -> HardwareData {
  HardwareData {
    avg: None,
    max: None,
    min: None,
  }
}
