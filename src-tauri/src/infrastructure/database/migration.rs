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
  fn max_migration_version() {
    assert_eq!(get_max_migration_version(), 8);
  }

  #[test]
  fn migration_count() {
    let migrations = get_migrations();
    assert_eq!(migrations.len(), 8);
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

    // The exact statement shape that used to fail now succeeds.
    sqlx::query(
      "INSERT INTO storage_devices (id, display_name, first_seen_at, last_seen_at) \
       VALUES ('disk-x', 'Disk X', '2026-06-21T00:00:00Z', '2026-06-21T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .expect("insert into storage_devices must succeed after migrations");
  }
}
