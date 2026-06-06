use tauri_plugin_sql::{Migration, MigrationKind};

pub fn get_max_migration_version() -> i64 {
  get_migrations()
    .iter()
    .filter(|m| matches!(m.kind, MigrationKind::Up))
    .map(|m| m.version)
    .max()
    .unwrap_or(0)
}

pub fn get_migrations() -> Vec<Migration> {
  vec![
    // Up Migrations
    Migration {
      version: 1,
      description: "create_initial_tables",
      sql: "CREATE TABLE DATA_ARCHIVE (id INTEGER PRIMARY KEY, cpu_avg INTEGER, cpu_max INTEGER, cpu_min INTEGER, ram_avg INTEGER, ram_max INTEGER, ram_min INTEGER, timestamp DATETIME);",
      kind: MigrationKind::Up,
    },
    Migration {
      version: 2,
      description: "create_gpu_tables",
      sql: "CREATE TABLE GPU_DATA_ARCHIVE (id INTEGER PRIMARY KEY, gpu_name TEXT, usage_avg INTEGER, usage_max INTEGER, usage_min INTEGER, temperature_avg INTEGER, temperature_max INTEGER, temperature_min INTEGER, timestamp DATETIME);",
      kind: MigrationKind::Up,
    },
    Migration {
      version: 3,
      description: "add_gpu_memory_usage_columns",
      sql: r#"
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN dedicated_memory_avg INTEGER;
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN dedicated_memory_max INTEGER;
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN dedicated_memory_min INTEGER;
      "#,
      kind: MigrationKind::Up,
    },
    Migration {
      version: 4,
      description: "create_process_stats",
      sql: "CREATE TABLE PROCESS_STATS (id INTEGER PRIMARY KEY AUTOINCREMENT, pid INTEGER NOT NULL, process_name TEXT NOT NULL,  cpu_usage REAL NOT NULL,  memory_usage INTEGER NOT NULL, execution_sec INTEGER NOT NULL, timestamp DATETIME NOT NULL);",
      kind: MigrationKind::Up,
    },
    Migration {
      version: 5,
      description: "add_gpu_id_column",
      sql: "ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN gpu_id TEXT;",
      kind: MigrationKind::Up,
    },
    Migration {
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
      kind: MigrationKind::Up,
    },
    Migration {
      version: 7,
      description: "rename_storage_smart_daily_snapshots_to_storage_health_daily_records",
      sql: r#"
        ALTER TABLE storage_smart_daily_snapshots
        RENAME TO storage_health_daily_records;
      "#,
      kind: MigrationKind::Up,
    },
    // Down Migrations
    Migration {
      version: 4,
      description: "drop_process_stats",
      sql: "DROP TABLE IF EXISTS PROCESS_STATS;",
      kind: MigrationKind::Down,
    },
    Migration {
      version: 6,
      description: "drop_storage_smart_daily_snapshots",
      sql: r#"
        DROP TABLE IF EXISTS storage_smart_daily_snapshots;
        DROP TABLE IF EXISTS storage_devices;
      "#,
      kind: MigrationKind::Down,
    },
    Migration {
      version: 7,
      description: "rename_storage_health_daily_records_to_storage_smart_daily_snapshots",
      sql: r#"
        ALTER TABLE storage_health_daily_records
        RENAME TO storage_smart_daily_snapshots;
      "#,
      kind: MigrationKind::Down,
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
      .find(|m| m.version == 5 && matches!(m.kind, MigrationKind::Up))
      .expect("Version 5 up migration must exist");
    assert!(v5.sql.contains("gpu_id TEXT"));
    assert!(v5.sql.contains("GPU_DATA_ARCHIVE"));
  }

  #[test]
  fn migration_v6_adds_storage_smart_tables() {
    let migrations = get_migrations();
    let v6 = migrations
      .iter()
      .find(|m| m.version == 6 && matches!(m.kind, MigrationKind::Up))
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
      .find(|m| m.version == 7 && matches!(m.kind, MigrationKind::Up))
      .expect("Version 7 up migration must exist");
    assert!(v7.sql.contains("ALTER TABLE storage_smart_daily_snapshots"));
    assert!(v7.sql.contains("RENAME TO storage_health_daily_records"));
  }

  #[test]
  fn max_migration_version() {
    assert_eq!(get_max_migration_version(), 7);
  }

  #[test]
  fn migration_count() {
    let migrations = get_migrations();
    let up_count = migrations
      .iter()
      .filter(|m| matches!(m.kind, MigrationKind::Up))
      .count();
    assert_eq!(up_count, 7);
  }

  #[test]
  fn migration_up_versions_sequential() {
    let migrations = get_migrations();
    let mut up_versions: Vec<_> = migrations
      .iter()
      .filter(|m| matches!(m.kind, MigrationKind::Up))
      .map(|m| m.version)
      .collect();
    up_versions.sort();
    let expected: Vec<i64> = (1..=up_versions.len() as i64).collect();
    assert_eq!(up_versions, expected);
  }
}
