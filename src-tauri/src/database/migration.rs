use tauri_plugin_sql::{Migration, MigrationKind};

pub fn get_migrations() -> Vec<Migration> {
  vec![
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
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN memory_dedicated_avg INTEGER;
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN memory_dedicated_max INTEGER;
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN memory_dedicated_min INTEGER;
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN memory_shared_avg INTEGER;
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN memory_shared_max INTEGER;
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN memory_shared_min INTEGER;
      "#,
      kind: MigrationKind::Up,
    },
  ]
}
