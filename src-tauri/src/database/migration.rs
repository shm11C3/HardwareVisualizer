use tauri_plugin_sql::{Migration, MigrationKind};

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
    // Down Migrations
    Migration {
      version: 4,
      description: "drop_process_stats",
      sql: "DROP TABLE IF EXISTS PROCESS_STATS;",
      kind: MigrationKind::Down,
    },
  ]
}
