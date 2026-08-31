use hardviz_core::infrastructure::database::migrate::SchemaMigration;

pub fn get_max_migration_version() -> i64 {
  get_migrations()
    .iter()
    .map(|m| m.version)
    .max()
    .unwrap_or(0)
}

/// Ordered forward migrations applied at startup by Core's migrator
/// ([`hardviz_core::infrastructure::database::migrate::run`]). Append-only:
/// the SQL of an already-released version must never change, or `sqlx`
/// will reject it as a checksum mismatch on existing databases.
pub fn get_migrations() -> Vec<SchemaMigration> {
  vec![
    SchemaMigration {
      version: 1,
      description: "create_initial_tables",
      sql: "CREATE TABLE DATA_ARCHIVE (id INTEGER PRIMARY KEY, cpu_avg INTEGER, cpu_max INTEGER, cpu_min INTEGER, ram_avg INTEGER, ram_max INTEGER, ram_min INTEGER, timestamp DATETIME);",
    },
    SchemaMigration {
      version: 2,
      description: "create_gpu_tables",
      sql: "CREATE TABLE GPU_DATA_ARCHIVE (id INTEGER PRIMARY KEY, gpu_name TEXT, usage_avg INTEGER, usage_max INTEGER, usage_min INTEGER, temperature_avg INTEGER, temperature_max INTEGER, temperature_min INTEGER, timestamp DATETIME);",
    },
    SchemaMigration {
      version: 3,
      description: "add_gpu_memory_usage_columns",
      sql: r#"
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN dedicated_memory_avg INTEGER;
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN dedicated_memory_max INTEGER;
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN dedicated_memory_min INTEGER;
      "#,
    },
    SchemaMigration {
      version: 4,
      description: "create_process_stats",
      sql: "CREATE TABLE PROCESS_STATS (id INTEGER PRIMARY KEY AUTOINCREMENT, pid INTEGER NOT NULL, process_name TEXT NOT NULL,  cpu_usage REAL NOT NULL,  memory_usage INTEGER NOT NULL, execution_sec INTEGER NOT NULL, timestamp DATETIME NOT NULL);",
    },
    SchemaMigration {
      version: 5,
      description: "add_gpu_id_column",
      sql: "ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN gpu_id TEXT;",
    },
    SchemaMigration {
      version: 6,
      description: "create_storage_smart_daily_snapshots",
      sql: r#"
        CREATE TABLE storage_devices (
          id TEXT PRIMARY KEY,
          display_name TEXT NOT NULL,
          model TEXT,
          serial_hash TEXT,
          protocol TEXT,
          capacity_bytes INTEGER,
          first_seen_at TEXT NOT NULL,
          last_seen_at TEXT NOT NULL,
          is_active INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE storage_smart_daily_snapshots (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          device_id TEXT NOT NULL,
          date TEXT NOT NULL,
          health_status TEXT NOT NULL,
          warning_level TEXT NOT NULL DEFAULT 'none',
          warning_reasons TEXT,
          temperature_celsius REAL,
          power_on_hours INTEGER,
          percentage_used REAL,
          available_spare_percent REAL,
          reallocated_sector_count INTEGER,
          current_pending_sector_count INTEGER,
          offline_uncorrectable_count INTEGER,
          media_errors INTEGER,
          error_log_entries INTEGER,
          unsafe_shutdown_count INTEGER,
          collected_at TEXT NOT NULL,
          UNIQUE(device_id, date),
          FOREIGN KEY(device_id) REFERENCES storage_devices(id)
        );
      "#,
    },
    SchemaMigration {
      version: 7,
      description: "rename_storage_smart_daily_snapshots_to_storage_health_daily_records",
      sql: r#"
        ALTER TABLE storage_smart_daily_snapshots
        RENAME TO storage_health_daily_records;
      "#,
    },
    SchemaMigration {
      version: 8,
      description: "add_process_stats_timestamp_index",
      sql: "CREATE INDEX IF NOT EXISTS idx_process_stats_timestamp ON PROCESS_STATS(timestamp);",
    },
    SchemaMigration {
      version: 9,
      description: "add_cpu_temperature_archive_columns",
      sql: r#"
        ALTER TABLE DATA_ARCHIVE ADD COLUMN cpu_temperature_avg REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN cpu_temperature_max REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN cpu_temperature_min REAL;
      "#,
    },
    SchemaMigration {
      version: 10,
      description: "add_power_archive_columns",
      sql: r#"
        ALTER TABLE DATA_ARCHIVE ADD COLUMN cpu_power_avg REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN cpu_power_max REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN cpu_power_min REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN gpu_power_avg REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN gpu_power_max REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN gpu_power_min REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN ane_power_avg REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN ane_power_max REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN ane_power_min REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN package_power_avg REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN package_power_max REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN package_power_min REAL;
      "#,
    },
    SchemaMigration {
      version: 11,
      description: "create_cooling_daily_summary",
      sql: r#"
        CREATE TABLE cooling_daily_summary (
          date TEXT PRIMARY KEY,
          idle_cpu_temperature_avg REAL,
          idle_cpu_temperature_max REAL,
          idle_cpu_temperature_min REAL,
          idle_sample_minutes INTEGER NOT NULL DEFAULT 0,
          low_cpu_temperature_avg REAL,
          low_cpu_temperature_max REAL,
          low_cpu_temperature_min REAL,
          low_sample_minutes INTEGER NOT NULL DEFAULT 0,
          mid_cpu_temperature_avg REAL,
          mid_cpu_temperature_max REAL,
          mid_cpu_temperature_min REAL,
          mid_sample_minutes INTEGER NOT NULL DEFAULT 0,
          high_cpu_temperature_avg REAL,
          high_cpu_temperature_max REAL,
          high_cpu_temperature_min REAL,
          high_sample_minutes INTEGER NOT NULL DEFAULT 0,
          coverage_minutes INTEGER NOT NULL
        );
      "#,
    },
    SchemaMigration {
      version: 12,
      description: "create_cooling_baseline",
      sql: r#"
        CREATE TABLE cooling_baseline (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          window_start_date TEXT NOT NULL,
          window_end_date TEXT NOT NULL,
          idle_temperature_avg REAL NOT NULL,
          sample_minutes INTEGER NOT NULL,
          established_at TEXT NOT NULL
        );
      "#,
    },
    SchemaMigration {
      version: 13,
      description: "create_cooling_hourly_summary",
      sql: r#"
        CREATE TABLE cooling_hourly_summary (
          hour_start TEXT PRIMARY KEY,
          cpu_usage_avg REAL,
          cpu_temperature_avg REAL,
          sample_minutes INTEGER NOT NULL
        );
      "#,
    },
    SchemaMigration {
      version: 14,
      description: "add_cooling_daily_summary_power_columns",
      // The CPU package power the timeline's power lane reads for 90d/1y
      // (#2021). Nullable so a machine with no power sampler keeps
      // reporting absent power rather than 0 W; `power_sample_minutes`
      // defaults to 0 so rows written before this migration read back as
      // "no powered minutes" instead of failing the NOT NULL constraint.
      sql: r#"
        ALTER TABLE cooling_daily_summary ADD COLUMN cpu_power_avg REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN cpu_power_max REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN cpu_power_min REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN power_sample_minutes INTEGER NOT NULL DEFAULT 0;
      "#,
    },
    SchemaMigration {
      version: 15,
      description: "create_ambient_archive",
      // Row-per-source (#2043): more than one ambient sensor in a room is
      // plausible, and each one is a distinct Sensor Source Label rather
      // than a column. `temperature` is NOT NULL because a row only
      // exists when a fresh reading backs it - a minute with no usable
      // ambient sample has no row at all, never a zeroed one (DP-02).
      // `humidity` is nullable: temperature-only sensors are common.
      sql: r#"
        CREATE TABLE AMBIENT_ARCHIVE (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          source TEXT NOT NULL,
          temperature REAL NOT NULL,
          humidity REAL,
          timestamp DATETIME NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_ambient_archive_timestamp ON AMBIENT_ARCHIVE(timestamp);
      "#,
    },
    SchemaMigration {
      version: 16,
      description: "add_cooling_daily_summary_ambient_delta_columns",
      // The per-band thermal delta (CPU package temperature minus ambient,
      // #2045) plus how many of the day's archived minutes carried an
      // ambient pair at all. Nullable delta columns and a defaulted
      // `*_delta_sample_minutes` so every row written before this
      // migration - and every row on a machine with no ambient sensor -
      // reads back as absent rather than 0 K (DP-02).
      //
      // `ambient_coverage_minutes` is counted outside the load-band gate,
      // the same way `power_sample_minutes` is: ambient availability is a
      // separate capability from CPU temperature, and the backfill cursor
      // needs a fact that a machine without a CPU temperature sensor can
      // still record.
      sql: r#"
        ALTER TABLE cooling_daily_summary ADD COLUMN idle_delta_temperature_avg REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN idle_delta_temperature_max REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN idle_delta_temperature_min REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN idle_delta_sample_minutes INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE cooling_daily_summary ADD COLUMN low_delta_temperature_avg REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN low_delta_temperature_max REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN low_delta_temperature_min REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN low_delta_sample_minutes INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE cooling_daily_summary ADD COLUMN mid_delta_temperature_avg REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN mid_delta_temperature_max REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN mid_delta_temperature_min REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN mid_delta_sample_minutes INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE cooling_daily_summary ADD COLUMN high_delta_temperature_avg REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN high_delta_temperature_max REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN high_delta_temperature_min REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN high_delta_sample_minutes INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE cooling_daily_summary ADD COLUMN ambient_coverage_minutes INTEGER NOT NULL DEFAULT 0;
      "#,
    },
    SchemaMigration {
      version: 17,
      description: "create_fan_archive",
      // The one-minute fan-speed archive behind the Cooling Insight fan
      // lane (#2022). Row-per-fan rather than fixed columns because how
      // many fans a machine exposes is configuration-dependent, and both
      // value columns are NOT NULL because a row is only written for a
      // reading that was actually taken: an unreadable fan is absent,
      // never 0 RPM (which is a real Inactive Fan Reading).
      sql: r#"
        CREATE TABLE FAN_ARCHIVE (
          id INTEGER PRIMARY KEY,
          source TEXT NOT NULL,
          rpm INTEGER NOT NULL,
          timestamp DATETIME NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_fan_archive_timestamp ON FAN_ARCHIVE(timestamp);
      "#,
    },
    SchemaMigration {
      version: 18,
      description: "create_cooling_fan_daily_summary",
      // The long-lived per-fan daily rollup the 90d/1y fan lane reads
      // (#2022). Keyed by (date, source) for the same row-per-fan reason
      // as `FAN_ARCHIVE`; a fan with no archived reading that day simply
      // has no row.
      sql: r#"
        CREATE TABLE cooling_fan_daily_summary (
          date TEXT NOT NULL,
          source TEXT NOT NULL,
          rpm_avg REAL NOT NULL,
          rpm_max INTEGER NOT NULL,
          rpm_min INTEGER NOT NULL,
          sample_minutes INTEGER NOT NULL,
          PRIMARY KEY (date, source)
        );
      "#,
    },
    SchemaMigration {
      version: 19,
      description: "add_data_archive_timestamp_index",
      // `DATA_ARCHIVE` is the one archive table that never had a
      // timestamp index, and it is also the largest: at one row per
      // minute a year of history is over half a million rows.
      //
      // Every read of it is bounded by a time range - the cooling
      // rollup's per-day fetch, the ambient pairing join (#2045), the
      // retention delete - and without this index each of those is a
      // full table scan. The pairing join made that acute, because it
      // pairs `AMBIENT_ARCHIVE` against this table per archived minute.
      //
      // `IF NOT EXISTS` matches the other index migrations, so this is a
      // no-op on a database that somehow already has it.
      sql: r#"
        CREATE INDEX IF NOT EXISTS idx_data_archive_timestamp ON DATA_ARCHIVE(timestamp);
      "#,
    },
  ]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn migration_v5_adds_gpu_id_column() {
    let migrations = get_migrations();
    let v5 = migrations
      .iter()
      .find(|m| m.version == 5)
      .expect("Version 5 up migration must exist");
    assert!(v5.sql.contains("gpu_id TEXT"));
    assert!(v5.sql.contains("GPU_DATA_ARCHIVE"));
  }

  #[test]
  fn migration_v6_adds_storage_smart_tables() {
    let migrations = get_migrations();
    let v6 = migrations
      .iter()
      .find(|m| m.version == 6)
      .expect("Version 6 up migration must exist");
    assert!(v6.sql.contains("CREATE TABLE storage_devices"));
    assert!(
      v6.sql
        .contains("CREATE TABLE storage_smart_daily_snapshots")
    );
    assert!(v6.sql.contains("UNIQUE(device_id, date)"));
  }

  #[test]
  fn migration_v7_renames_storage_health_records_table() {
    let migrations = get_migrations();
    let v7 = migrations
      .iter()
      .find(|m| m.version == 7)
      .expect("Version 7 up migration must exist");
    assert!(v7.sql.contains("ALTER TABLE storage_smart_daily_snapshots"));
    assert!(v7.sql.contains("RENAME TO storage_health_daily_records"));
  }

  #[test]
  fn migration_v8_adds_process_stats_timestamp_index() {
    let migrations = get_migrations();
    let v8 = migrations
      .iter()
      .find(|m| m.version == 8)
      .expect("Version 8 up migration must exist");
    assert!(v8.sql.contains("CREATE INDEX IF NOT EXISTS"));
    assert!(v8.sql.contains("idx_process_stats_timestamp"));
    assert!(v8.sql.contains("PROCESS_STATS(timestamp)"));
  }

  #[test]
  fn migration_v9_adds_cpu_temperature_archive_columns() {
    let migrations = get_migrations();
    let v9 = migrations
      .iter()
      .find(|m| m.version == 9)
      .expect("Version 9 up migration must exist");
    assert!(v9.sql.contains("cpu_temperature_avg REAL"));
    assert!(v9.sql.contains("cpu_temperature_max REAL"));
    assert!(v9.sql.contains("cpu_temperature_min REAL"));
    assert!(v9.sql.contains("DATA_ARCHIVE"));
  }

  #[test]
  fn migration_v10_adds_power_archive_columns() {
    let migration = get_migrations()
      .into_iter()
      .find(|migration| migration.version == 10)
      .expect("Version 10 up migration must exist");
    for component in ["cpu", "gpu", "ane", "package"] {
      for stats in ["avg", "max", "min"] {
        assert!(
          migration
            .sql
            .contains(&format!("{component}_power_{stats} REAL"))
        );
      }
    }
  }

  #[test]
  fn migration_v11_creates_cooling_daily_summary_table() {
    let migrations = get_migrations();
    let v11 = migrations
      .iter()
      .find(|m| m.version == 11)
      .expect("Version 11 up migration must exist");
    assert!(v11.sql.contains("CREATE TABLE cooling_daily_summary"));
    assert!(v11.sql.contains("date TEXT PRIMARY KEY"));
    for band in ["idle", "low", "mid", "high"] {
      assert!(
        v11
          .sql
          .contains(&format!("{band}_cpu_temperature_avg REAL"))
      );
      assert!(
        v11
          .sql
          .contains(&format!("{band}_cpu_temperature_max REAL"))
      );
      assert!(
        v11
          .sql
          .contains(&format!("{band}_cpu_temperature_min REAL"))
      );
      assert!(
        v11
          .sql
          .contains(&format!("{band}_sample_minutes INTEGER NOT NULL DEFAULT 0"))
      );
    }
    assert!(v11.sql.contains("coverage_minutes INTEGER NOT NULL"));
  }

  #[test]
  fn migration_v12_creates_the_single_row_cooling_baseline_table() {
    let migrations = get_migrations();
    let v12 = migrations
      .iter()
      .find(|m| m.version == 12)
      .expect("Version 12 up migration must exist");
    assert!(v12.sql.contains("CREATE TABLE cooling_baseline"));
    // The established baseline is a single row. The CHECK constraint is
    // what makes the insert idempotent: a second establishment can never
    // add a competing row, so the value cannot drift once pinned.
    assert!(v12.sql.contains("id INTEGER PRIMARY KEY CHECK (id = 1)"));
    assert!(v12.sql.contains("window_start_date TEXT NOT NULL"));
    assert!(v12.sql.contains("window_end_date TEXT NOT NULL"));
    assert!(v12.sql.contains("idle_temperature_avg REAL NOT NULL"));
    assert!(v12.sql.contains("sample_minutes INTEGER NOT NULL"));
    assert!(v12.sql.contains("established_at TEXT NOT NULL"));
  }

  #[test]
  fn migration_v13_creates_cooling_hourly_summary_table() {
    let migrations = get_migrations();
    let v13 = migrations
      .iter()
      .find(|m| m.version == 13)
      .expect("Version 13 up migration must exist");
    assert!(v13.sql.contains("CREATE TABLE cooling_hourly_summary"));
    // The key is the local wall-clock hour string, so it stays
    // prefix-comparable with `cooling_daily_summary.date`.
    assert!(v13.sql.contains("hour_start TEXT PRIMARY KEY"));
    assert!(v13.sql.contains("cpu_usage_avg REAL"));
    assert!(v13.sql.contains("cpu_temperature_avg REAL"));
    assert!(v13.sql.contains("sample_minutes INTEGER NOT NULL"));
  }

  #[test]
  fn migration_v14_adds_cooling_daily_summary_power_columns() {
    let migrations = get_migrations();
    let v14 = migrations
      .iter()
      .find(|m| m.version == 14)
      .expect("Version 14 up migration must exist");
    assert!(v14.sql.contains("ALTER TABLE cooling_daily_summary"));
    for stats in ["avg", "max", "min"] {
      assert!(v14.sql.contains(&format!("cpu_power_{stats} REAL")));
    }
    // Existing rows predate power collection, so the counter must default
    // rather than be NOT NULL without one.
    assert!(
      v14
        .sql
        .contains("power_sample_minutes INTEGER NOT NULL DEFAULT 0")
    );
  }

  #[test]
  fn migration_v15_creates_the_row_per_source_ambient_archive_table() {
    let migrations = get_migrations();
    let v15 = migrations
      .iter()
      .find(|m| m.version == 15)
      .expect("Version 15 up migration must exist");
    assert!(v15.sql.contains("CREATE TABLE AMBIENT_ARCHIVE"));
    // `source` identifies the row rather than being one column per
    // sensor, so several ambient sources can share one minute.
    assert!(v15.sql.contains("source TEXT NOT NULL"));
    // A row exists only when a fresh reading backs it, so the temperature
    // can never be a placeholder.
    assert!(v15.sql.contains("temperature REAL NOT NULL"));
    assert!(v15.sql.contains("humidity REAL"));
    assert!(v15.sql.contains("timestamp DATETIME NOT NULL"));
    assert!(v15.sql.contains("idx_ambient_archive_timestamp"));
  }

  #[test]
  fn migration_v16_adds_cooling_daily_summary_ambient_delta_columns() {
    let migrations = get_migrations();
    let v16 = migrations
      .iter()
      .find(|m| m.version == 16)
      .expect("Version 16 up migration must exist");
    assert!(v16.sql.contains("ALTER TABLE cooling_daily_summary"));
    for band in ["idle", "low", "mid", "high"] {
      for stats in ["avg", "max", "min"] {
        assert!(
          v16
            .sql
            .contains(&format!("{band}_delta_temperature_{stats} REAL"))
        );
      }
      // Rows written before ambient collection existed must read back as
      // "no paired minutes" rather than failing a NOT NULL constraint.
      assert!(v16.sql.contains(&format!(
        "{band}_delta_sample_minutes INTEGER NOT NULL DEFAULT 0"
      )));
    }
    assert!(
      v16
        .sql
        .contains("ambient_coverage_minutes INTEGER NOT NULL DEFAULT 0")
    );
  }

  #[test]
  fn migration_v17_creates_the_fan_archive_table() {
    let migrations = get_migrations();
    let v17 = migrations
      .iter()
      .find(|m| m.version == 17)
      .expect("Version 17 up migration must exist");
    assert!(v17.sql.contains("CREATE TABLE FAN_ARCHIVE"));
    // Row-per-fan: the identifier is a column, not one column per fan.
    assert!(v17.sql.contains("source TEXT NOT NULL"));
    // NOT NULL because a row only exists for a reading that was taken -
    // an Inactive Fan Reading is a real 0, and an absent one has no row.
    assert!(v17.sql.contains("rpm INTEGER NOT NULL"));
    assert!(v17.sql.contains("timestamp DATETIME NOT NULL"));
    assert!(v17.sql.contains("idx_fan_archive_timestamp"));
  }

  #[test]
  fn migration_v18_creates_the_cooling_fan_daily_summary_table() {
    let migrations = get_migrations();
    let v18 = migrations
      .iter()
      .find(|m| m.version == 18)
      .expect("Version 18 up migration must exist");
    assert!(v18.sql.contains("CREATE TABLE cooling_fan_daily_summary"));
    // The composite key is what makes a day carry one row per fan rather
    // than one row with a fixed fan column set.
    assert!(v18.sql.contains("PRIMARY KEY (date, source)"));
    assert!(v18.sql.contains("rpm_avg REAL NOT NULL"));
    assert!(v18.sql.contains("rpm_max INTEGER NOT NULL"));
    assert!(v18.sql.contains("rpm_min INTEGER NOT NULL"));
    assert!(v18.sql.contains("sample_minutes INTEGER NOT NULL"));
  }

  #[test]
  fn migration_v19_indexes_the_data_archive_timestamp() {
    let migrations = get_migrations();
    let v19 = migrations
      .iter()
      .find(|m| m.version == 19)
      .expect("Version 19 up migration must exist");
    assert!(v19.sql.contains("CREATE INDEX IF NOT EXISTS"));
    assert!(v19.sql.contains("idx_data_archive_timestamp"));
    assert!(v19.sql.contains("DATA_ARCHIVE(timestamp)"));
  }

  #[test]
  fn max_migration_version() {
    assert_eq!(get_max_migration_version(), 19);
  }

  #[test]
  fn migration_count() {
    let migrations = get_migrations();
    // 16 was reserved while the ambient delta columns were in flight and
    // is now filled, so 1..=19 happens to be contiguous again. The
    // assertion below still only requires unique and ascending - see it
    // for why contiguity is not something to depend on.
    assert_eq!(migrations.len(), 19);
  }

  #[test]
  fn migration_up_versions_are_unique_and_strictly_ascending() {
    // Strictly ascending and unique, deliberately *not* contiguous.
    //
    // The runner applies migrations in ascending version order and records
    // each applied version, so a gap costs nothing. Contiguity, on the
    // other hand, cannot survive parallel branches: a version reserved by
    // an in-flight PR has to stay claimed until that PR lands, and forcing
    // the numbers closed would mean two branches silently picking the same
    // version - which the checksum in `_sqlx_migrations` would only catch
    // on a user's machine, after both had shipped.
    //
    // Uniqueness and ordering are what actually protect the upgrade path,
    // and both are still asserted here.
    let versions: Vec<i64> = get_migrations().iter().map(|m| m.version).collect();

    assert!(
      versions.windows(2).all(|pair| pair[0] < pair[1]),
      "migrations must be declared in strictly ascending version order: {versions:?}"
    );
    assert!(
      versions.iter().all(|&version| version >= 1),
      "migration versions start at 1: {versions:?}"
    );
  }

  /// Regression for the reported `no such table: storage_devices` insert
  /// failure: applying the exact migration set the app ships must create the
  /// storage-health tables and let the previously-failing insert succeed.
  /// Guards against the migrations silently never running again.
  #[tokio::test]
  async fn shipped_migrations_create_storage_tables_and_allow_insert() {
    use hardviz_core::infrastructure::database::migrate;
    use sqlx::sqlite::SqlitePool;

    // A file-backed (not `:memory:`) throwaway DB so every pooled connection
    // shares the same schema the migrator just created.
    let file = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite:{}", file.path().to_string_lossy());
    let pool = SqlitePool::connect(&url).await.unwrap();

    migrate::run_on_pool(&pool, get_migrations())
      .await
      .expect("the shipped migration set must apply cleanly");

    for table in ["storage_devices", "storage_health_daily_records"] {
      let exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(1) FROM sqlite_master WHERE type = 'table' AND name = $1",
      )
      .bind(table)
      .fetch_one(&pool)
      .await
      .unwrap();
      assert_eq!(exists.0, 1, "table `{table}` must exist after migrations");
    }

    let cpu_temperature_columns: (i64,) = sqlx::query_as(
      "SELECT COUNT(*)
       FROM pragma_table_info('DATA_ARCHIVE')
       WHERE name IN (
         'cpu_temperature_avg',
         'cpu_temperature_max',
         'cpu_temperature_min'
       )",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(cpu_temperature_columns.0, 3);

    let power_columns: (i64,) = sqlx::query_as(
      "SELECT COUNT(*) FROM pragma_table_info('DATA_ARCHIVE')
       WHERE name IN (
         'cpu_power_avg', 'cpu_power_max', 'cpu_power_min',
         'gpu_power_avg', 'gpu_power_max', 'gpu_power_min',
         'ane_power_avg', 'ane_power_max', 'ane_power_min',
         'package_power_avg', 'package_power_max', 'package_power_min'
       )",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(power_columns.0, 12);

    sqlx::query(
      "INSERT INTO DATA_ARCHIVE (
         cpu_avg,
         cpu_max,
         cpu_min,
         ram_avg,
         ram_max,
         ram_min,
         cpu_temperature_avg,
         cpu_temperature_max,
         cpu_temperature_min,
         cpu_power_avg, cpu_power_max, cpu_power_min,
         gpu_power_avg, gpu_power_max, gpu_power_min,
         ane_power_avg, ane_power_max, ane_power_min,
         package_power_avg, package_power_max, package_power_min,
         timestamp
       ) VALUES (
         25, 50, 10, 40, 45, 35, 52.5, 61, 44,
         8, 12, 4, 5, 9, 2, 1, 2, 0.5, 14, 20, 8,
         '2026-06-21T00:00:00Z'
       )",
    )
    .execute(&pool)
    .await
    .expect("insert with temperature and power archive columns must succeed");

    // The exact statement shape that used to fail now succeeds.
    sqlx::query(
      "INSERT INTO storage_devices (id, display_name, first_seen_at, last_seen_at) \
       VALUES ('disk-x', 'Disk X', '2026-06-21T00:00:00Z', '2026-06-21T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .expect("insert into storage_devices must succeed after migrations");
  }

  /// Regression guard for the cooling daily rollup (#2015): the shipped
  /// migration set must create `cooling_daily_summary` with one row per
  /// local day and let a full 4-band insert succeed.
  #[tokio::test]
  async fn shipped_migrations_create_cooling_daily_summary_table_and_allow_insert() {
    use hardviz_core::infrastructure::database::migrate;
    use sqlx::sqlite::SqlitePool;

    let file = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite:{}", file.path().to_string_lossy());
    let pool = SqlitePool::connect(&url).await.unwrap();

    migrate::run_on_pool(&pool, get_migrations())
      .await
      .expect("the shipped migration set must apply cleanly");

    let exists: (i64,) = sqlx::query_as(
      "SELECT COUNT(1) FROM sqlite_master WHERE type = 'table' AND name = 'cooling_daily_summary'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
      exists.0, 1,
      "table `cooling_daily_summary` must exist after migrations"
    );

    sqlx::query(
      "INSERT INTO cooling_daily_summary (
         date,
         idle_cpu_temperature_avg, idle_cpu_temperature_max, idle_cpu_temperature_min, idle_sample_minutes,
         low_cpu_temperature_avg, low_cpu_temperature_max, low_cpu_temperature_min, low_sample_minutes,
         mid_cpu_temperature_avg, mid_cpu_temperature_max, mid_cpu_temperature_min, mid_sample_minutes,
         high_cpu_temperature_avg, high_cpu_temperature_max, high_cpu_temperature_min, high_sample_minutes,
         coverage_minutes
       ) VALUES (
         '2026-06-21',
         35.0, 40.0, 30.0, 600,
         45.0, 50.0, 40.0, 300,
         NULL, NULL, NULL, 0,
         70.0, 80.0, 60.0, 120,
         1020
       )",
    )
    .execute(&pool)
    .await
    .expect("insert into cooling_daily_summary must succeed after migrations");

    // `date` is the primary key: a second row for the same local day must
    // be rejected rather than silently duplicating the day.
    let duplicate = sqlx::query(
      "INSERT INTO cooling_daily_summary (date, coverage_minutes) VALUES ('2026-06-21', 1)",
    )
    .execute(&pool)
    .await;
    assert!(
      duplicate.is_err(),
      "date must be a primary key: one row per local day"
    );

    // A day written before #2021 (no power columns supplied) must read
    // back as absent power rather than 0 W.
    let power: (Option<f64>, i64) = sqlx::query_as(
      "SELECT cpu_power_avg, power_sample_minutes
       FROM cooling_daily_summary WHERE date = '2026-06-21'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(power, (None, 0));

    sqlx::query(
      "INSERT INTO cooling_daily_summary (
         date, coverage_minutes,
         cpu_power_avg, cpu_power_max, cpu_power_min, power_sample_minutes
       ) VALUES ('2026-06-22', 1440, 18.5, 42.0, 4.5, 1300)",
    )
    .execute(&pool)
    .await
    .expect("insert with the power columns must succeed after migrations");
  }

  /// The ambient archive (#2043) is row-per-source: the shipped migration
  /// set must let two ambient sources share one minute, accept a
  /// temperature-only sensor, and reject a row with no temperature.
  #[tokio::test]
  async fn shipped_migrations_create_ambient_archive_and_allow_row_per_source_insert() {
    use hardviz_core::infrastructure::database::migrate;
    use sqlx::sqlite::SqlitePool;

    let file = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite:{}", file.path().to_string_lossy());
    let pool = SqlitePool::connect(&url).await.unwrap();

    migrate::run_on_pool(&pool, get_migrations())
      .await
      .expect("the shipped migration set must apply cleanly");

    sqlx::query(
      "INSERT INTO AMBIENT_ARCHIVE (source, temperature, humidity, timestamp) VALUES
         ('Living Room', 24.5, 48.0, '2026-08-30T12:00:00Z'),
         ('Desk', 26.0, NULL, '2026-08-30T12:00:00Z')",
    )
    .execute(&pool)
    .await
    .expect("two ambient sources must be able to share one archive minute");

    let rows: Vec<(String, f64, Option<f64>)> = sqlx::query_as(
      "SELECT source, temperature, humidity FROM AMBIENT_ARCHIVE ORDER BY source",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
      rows,
      vec![
        ("Desk".to_string(), 26.0, None),
        ("Living Room".to_string(), 24.5, Some(48.0)),
      ]
    );

    let without_temperature = sqlx::query(
      "INSERT INTO AMBIENT_ARCHIVE (source, temperature, timestamp)
       VALUES ('Broken', NULL, '2026-08-30T12:01:00Z')",
    )
    .execute(&pool)
    .await;
    assert!(
      without_temperature.is_err(),
      "a minute without a usable ambient reading must have no row at all"
    );
  }

  /// The ambient-normalized delta columns (#2045) must be additive: a day
  /// written the way every pre-#2045 row was must read back with absent
  /// deltas and zero ambient coverage, never a fabricated 0 K.
  #[tokio::test]
  async fn shipped_migrations_leave_pre_ambient_days_with_absent_deltas() {
    use hardviz_core::infrastructure::database::migrate;
    use sqlx::sqlite::SqlitePool;

    let file = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite:{}", file.path().to_string_lossy());
    let pool = SqlitePool::connect(&url).await.unwrap();

    migrate::run_on_pool(&pool, get_migrations())
      .await
      .expect("the shipped migration set must apply cleanly");

    sqlx::query(
      "INSERT INTO cooling_daily_summary (
         date, coverage_minutes,
         idle_cpu_temperature_avg, idle_sample_minutes
       ) VALUES ('2026-06-21', 1440, 35.0, 600)",
    )
    .execute(&pool)
    .await
    .expect("a pre-#2045 shaped insert must still succeed");

    let row: (Option<f64>, i64, i64) = sqlx::query_as(
      "SELECT idle_delta_temperature_avg, idle_delta_sample_minutes,
              ambient_coverage_minutes
       FROM cooling_daily_summary WHERE date = '2026-06-21'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row, (None, 0, 0));

    sqlx::query(
      "INSERT INTO cooling_daily_summary (
         date, coverage_minutes,
         idle_delta_temperature_avg, idle_delta_temperature_max,
         idle_delta_temperature_min, idle_delta_sample_minutes,
         ambient_coverage_minutes
       ) VALUES ('2026-06-22', 1440, 22.0, 26.0, 18.0, 500, 720)",
    )
    .execute(&pool)
    .await
    .expect("insert with the ambient delta columns must succeed after migrations");
  }
}
