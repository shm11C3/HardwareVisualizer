#[path = "../../../src-tauri/src/infrastructure/database/migration.rs"]
mod app_migration;

use std::env;
use std::path::PathBuf;

use hardviz_core::infrastructure::database::migrate;
use serde_json::{Value, json};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

fn argument(name: &str) -> Result<PathBuf, String> {
  let mut args = env::args_os().skip(1);
  while let Some(argument) = args.next() {
    if argument == name {
      return args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{name} requires a path"));
    }
  }
  Err(format!("missing required argument {name}"))
}

fn quote_identifier(identifier: &str) -> String {
  format!("\"{}\"", identifier.replace('"', "\"\""))
}

async fn pragma_rows(
  pool: &SqlitePool,
  pragma: &str,
  table: &str,
) -> Result<Vec<Value>, sqlx::Error> {
  let sql = format!("PRAGMA {pragma}({})", quote_identifier(table));
  let rows = sqlx::query(&sql).fetch_all(pool).await?;
  Ok(
    rows
      .into_iter()
      .map(|row| match pragma {
        "table_xinfo" => json!({
          "cid": row.get::<i64, _>("cid"),
          "name": row.get::<String, _>("name"),
          "declared_type": row.get::<String, _>("type"),
          "not_null": row.get::<i64, _>("notnull") != 0,
          "default_sql": row.try_get::<Option<String>, _>("dflt_value").ok().flatten(),
          "primary_key_ordinal": row.get::<i64, _>("pk"),
          "hidden": row.get::<i64, _>("hidden"),
        }),
        "index_list" => json!({
          "sequence": row.get::<i64, _>("seq"),
          "name": row.get::<String, _>("name"),
          "unique": row.get::<i64, _>("unique") != 0,
          "origin": row.get::<String, _>("origin"),
          "partial": row.get::<i64, _>("partial") != 0,
        }),
        "foreign_key_list" => json!({
          "id": row.get::<i64, _>("id"),
          "sequence": row.get::<i64, _>("seq"),
          "referenced_table": row.get::<String, _>("table"),
          "from_column": row.get::<String, _>("from"),
          "to_column": row.try_get::<Option<String>, _>("to").ok().flatten(),
          "on_update": row.get::<String, _>("on_update"),
          "on_delete": row.get::<String, _>("on_delete"),
          "match": row.get::<String, _>("match"),
        }),
        _ => unreachable!(),
      })
      .collect(),
  )
}

async fn index_columns(
  pool: &SqlitePool,
  index: &str,
) -> Result<Vec<Value>, sqlx::Error> {
  let sql = format!("PRAGMA index_xinfo({})", quote_identifier(index));
  Ok(
    sqlx::query(&sql)
      .fetch_all(pool)
      .await?
      .into_iter()
      .map(|row| {
        json!({
          "sequence": row.get::<i64, _>("seqno"),
          "column_id": row.get::<i64, _>("cid"),
          "name": row.try_get::<Option<String>, _>("name").ok().flatten(),
          "descending": row.get::<i64, _>("desc") != 0,
          "collation": row.get::<String, _>("coll"),
          "key": row.get::<i64, _>("key") != 0,
        })
      })
      .collect(),
  )
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let database = argument("--database")?;
  let output = argument("--output")?;
  if database.exists() {
    return Err(format!("refusing to overwrite database: {}", database.display()).into());
  }
  if let Some(parent) = database.parent() {
    std::fs::create_dir_all(parent)?;
  }

  let options = SqliteConnectOptions::new()
    .filename(&database)
    .create_if_missing(true)
    .foreign_keys(true);
  let pool = SqlitePoolOptions::new()
    .max_connections(1)
    .connect_with(options)
    .await?;

  let migrations = app_migration::get_migrations();
  let declared_max_migration_version = app_migration::get_max_migration_version();
  let declared_migrations: Vec<Value> = migrations
    .iter()
    .map(|migration| {
      json!({
        "version": migration.version,
        "description": migration.description,
        "sql_bytes": migration.sql.len(),
      })
    })
    .collect();
  migrate::run_on_pool(&pool, migrations).await?;

  let schema_rows = sqlx::query(
    "SELECT type, name, tbl_name, rootpage, sql FROM sqlite_schema ORDER BY type, name",
  )
  .fetch_all(&pool)
  .await?;
  let schema_objects: Vec<Value> = schema_rows
    .into_iter()
    .map(|row| {
      json!({
        "type": row.get::<String, _>("type"),
        "name": row.get::<String, _>("name"),
        "table_name": row.get::<String, _>("tbl_name"),
        "root_page": row.get::<i64, _>("rootpage"),
        "sql": row.try_get::<Option<String>, _>("sql").ok().flatten(),
      })
    })
    .collect();

  let table_names: Vec<String> = schema_objects
    .iter()
    .filter(|object| object["type"] == "table")
    .filter_map(|object| object["name"].as_str().map(ToOwned::to_owned))
    .collect();
  let mut tables = Vec::new();
  for table in &table_names {
    let indexes = pragma_rows(&pool, "index_list", table).await?;
    let mut detailed_indexes = Vec::new();
    for index in indexes {
      let name = index["name"].as_str().expect("index name");
      let schema_sql = schema_objects
        .iter()
        .find(|object| object["type"] == "index" && object["name"] == name)
        .and_then(|object| object["sql"].as_str());
      detailed_indexes.push(json!({
        "name": name,
        "unique": index["unique"],
        "origin": index["origin"],
        "partial": index["partial"],
        "sql": schema_sql,
        "columns": index_columns(&pool, name).await?,
      }));
    }
    tables.push(json!({
      "name": table,
      "columns": pragma_rows(&pool, "table_xinfo", table).await?,
      "indexes": detailed_indexes,
      "foreign_keys": pragma_rows(&pool, "foreign_key_list", table).await?,
    }));
  }

  let applied_migrations: Vec<Value> = sqlx::query(
    "SELECT version, description, CAST(installed_on AS TEXT) AS installed_on, success, hex(checksum) AS checksum_hex, execution_time FROM _sqlx_migrations ORDER BY version",
  )
  .fetch_all(&pool)
  .await?
  .into_iter()
  .map(|row| {
    json!({
      "version": row.get::<i64, _>("version"),
      "description": row.get::<String, _>("description"),
      "installed_on": row.get::<String, _>("installed_on"),
      "success": row.get::<bool, _>("success"),
      "checksum_hex": row.get::<String, _>("checksum_hex"),
      "execution_time_ns": row.get::<i64, _>("execution_time"),
    })
  })
  .collect();

  let sqlite_sequence = if table_names.iter().any(|name| name == "sqlite_sequence") {
    sqlx::query("SELECT name, seq FROM sqlite_sequence ORDER BY name")
      .fetch_all(&pool)
      .await?
      .into_iter()
      .map(|row| {
        json!({
          "name": row.get::<String, _>("name"),
          "sequence": row.get::<i64, _>("seq"),
        })
      })
      .collect()
  } else {
    Vec::new()
  };

  let sqlite_version: String = sqlx::query_scalar("SELECT sqlite_version()")
    .fetch_one(&pool)
    .await?;
  pool.close().await;

  let report = json!({
    "sqlite_version": sqlite_version,
    "database": database,
    "declared_migrations": declared_migrations,
    "declared_max_migration_version": declared_max_migration_version,
    "migration_count": applied_migrations.len(),
    "max_successful_migration_version": applied_migrations
      .iter()
      .filter(|migration| migration["success"] == true)
      .filter_map(|migration| migration["version"].as_i64())
      .max(),
    "applied_migrations": applied_migrations,
    "schema_objects": schema_objects,
    "tables": tables,
    "sqlite_sequence": sqlite_sequence,
  });
  if let Some(parent) = output.parent() {
    std::fs::create_dir_all(parent)?;
  }
  std::fs::write(output, serde_json::to_vec_pretty(&report)?)?;
  Ok(())
}
