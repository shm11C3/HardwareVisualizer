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
        v11.sql
          .contains(&format!("{band}_cpu_temperature_avg REAL"))
      );
      assert!(
        v11.sql
          .contains(&format!("{band}_cpu_temperature_max REAL"))
      );
      assert!(
        v11.sql
          .contains(&format!("{band}_cpu_temperature_min REAL"))
      );
      assert!(
        v11.sql
          .contains(&format!("{band}_sample_minutes INTEGER NOT NULL DEFAULT 0"))
      );
    }
    assert!(v11.sql.contains("coverage_minutes INTEGER NOT NULL"));
  }

  #[test]
  fn max_migration_version() {
    assert_eq!(get_max_migration_version(), 11);
  }

  #[test]
  fn migration_count() {
    let migrations = get_migrations();
    assert_eq!(migrations.len(), 11);
  }

  #[test]
  fn migration_up_versions_sequential() {
    let migrations = get_migrations();
    let mut up_versions: Vec<_> = migrations.iter().map(|m| m.version).collect();
    up_versions.sort();
    let expected: Vec<i64> = (1..=up_versions.len() as i64).collect();
    assert_eq!(up_versions, expected);
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
    assert_eq!(exists.0, 1, "table `cooling_daily_summary` must exist after migrations");

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
  }
}
