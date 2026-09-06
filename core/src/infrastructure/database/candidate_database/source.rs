use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection, SqlitePoolOptions};
use sqlx::{Connection, Row};

use super::CandidateError;
use super::cell::{
  COPY_BATCH_ROWS, CanonicalKind, Cell, MAX_BATCH_BYTES, MAX_ROW_BYTES, RowBatch,
  SourceRow, canonical_kind, validate_storage_class,
};
use crate::infrastructure::database::migrate::{self, SchemaMigration};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct SchemaObject {
  pub(super) object_type: String,
  pub(super) name: String,
  pub(super) table_name: String,
  pub(super) sql: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct ColumnSchema {
  pub(super) cid: i64,
  pub(super) name: String,
  pub(super) declared_type: String,
  pub(super) not_null: bool,
  pub(super) default_sql: Option<String>,
  pub(super) primary_key_ordinal: i64,
  pub(super) hidden: i64,
  pub(super) canonical_kind: CanonicalKind,
  pub(super) native_kind: CanonicalKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct IndexColumnSchema {
  pub(super) sequence: i64,
  pub(super) cid: i64,
  pub(super) name: Option<String>,
  pub(super) descending: bool,
  pub(super) collation: Option<String>,
  pub(super) key: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct IndexSchema {
  pub(super) sequence: i64,
  pub(super) name: String,
  pub(super) unique: bool,
  pub(super) origin: String,
  pub(super) partial: bool,
  pub(super) columns: Vec<IndexColumnSchema>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct ForeignKeySchema {
  pub(super) id: i64,
  pub(super) sequence: i64,
  pub(super) referenced_table: String,
  pub(super) from_column: String,
  pub(super) to_column: Option<String>,
  pub(super) on_update: String,
  pub(super) on_delete: String,
  pub(super) match_kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct TableSchema {
  pub(super) name: String,
  pub(super) sql: Option<String>,
  pub(super) columns: Vec<ColumnSchema>,
  pub(super) indexes: Vec<IndexSchema>,
  pub(super) foreign_keys: Vec<ForeignKeySchema>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct MigrationIdentity {
  pub(super) version: i64,
  pub(super) description: String,
  pub(super) checksum: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct MigrationProvenance {
  pub(super) max_version: i64,
  pub(super) migration_count: u64,
  pub(super) migrations: Vec<MigrationIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct TableHighWater {
  pub(super) table_index: usize,
  pub(super) table_name: String,
  pub(super) max_rowid: Option<i64>,
  pub(super) row_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct SourceSchema {
  pub(super) sqlite_version: String,
  pub(super) objects: Vec<SchemaObject>,
  pub(super) tables: Vec<TableSchema>,
  pub(super) migration_provenance: MigrationProvenance,
  pub(super) table_high_waters: Vec<TableHighWater>,
  pub(super) digest: [u8; 32],
}

pub(super) async fn open_source(path: &Path) -> Result<SqliteConnection, CandidateError> {
  let options = SqliteConnectOptions::new()
    .filename(path)
    .create_if_missing(false)
    .read_only(true)
    .row_buffer_size(1);
  let mut connection =
    SqliteConnection::connect_with(&options)
      .await
      .map_err(|error| CandidateError::SourceOpen {
        message: error.to_string(),
      })?;
  sqlx::query("PRAGMA query_only = ON")
    .execute(&mut connection)
    .await
    .map_err(source_read)?;
  Ok(connection)
}

pub(super) async fn reference_schema(
  migrations: Vec<SchemaMigration>,
) -> Result<SourceSchema, CandidateError> {
  let directory =
    tempfile::tempdir().map_err(|error| CandidateError::ReferenceMigration {
      message: error.to_string(),
    })?;
  let path = directory.path().join("reference.sqlite3");
  let options = SqliteConnectOptions::new()
    .filename(&path)
    .create_if_missing(true)
    .row_buffer_size(1);
  let pool = SqlitePoolOptions::new()
    .max_connections(1)
    .connect_with(options)
    .await
    .map_err(|error| CandidateError::ReferenceMigration {
      message: error.to_string(),
    })?;
  migrate::run_on_pool(&pool, migrations)
    .await
    .map_err(|message| CandidateError::ReferenceMigration { message })?;
  let mut connection =
    pool
      .acquire()
      .await
      .map_err(|error| CandidateError::ReferenceMigration {
        message: error.to_string(),
      })?;
  sqlx::query("BEGIN")
    .execute(&mut *connection)
    .await
    .map_err(|error| CandidateError::ReferenceMigration {
      message: error.to_string(),
    })?;
  let schema = inspect_schema(&mut connection).await.map_err(|error| {
    CandidateError::ReferenceMigration {
      message: error.to_string(),
    }
  })?;
  sqlx::query("ROLLBACK")
    .execute(&mut *connection)
    .await
    .map_err(|error| CandidateError::ReferenceMigration {
      message: error.to_string(),
    })?;
  drop(connection);
  pool.close().await;
  Ok(schema)
}

pub(super) async fn inspect_schema(
  connection: &mut SqliteConnection,
) -> Result<SourceSchema, CandidateError> {
  let sqlite_version: String = sqlx::query_scalar("SELECT sqlite_version()")
    .fetch_one(&mut *connection)
    .await
    .map_err(source_read)?;
  let object_rows = sqlx::query(
    "SELECT type, name, tbl_name, sql FROM sqlite_schema ORDER BY type, name",
  )
  .fetch_all(&mut *connection)
  .await
  .map_err(source_read)?;
  let objects = object_rows
    .into_iter()
    .map(|row| {
      Ok(SchemaObject {
        object_type: row.try_get("type")?,
        name: row.try_get("name")?,
        table_name: row.try_get("tbl_name")?,
        sql: row.try_get("sql")?,
      })
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()
    .map_err(source_read)?;

  let mut table_names = objects
    .iter()
    .filter(|object| object.object_type == "table")
    .map(|object| object.name.clone())
    .collect::<Vec<_>>();
  table_names.sort();
  let mut tables = Vec::with_capacity(table_names.len());
  for name in table_names {
    tables.push(inspect_table(connection, &objects, name).await?);
  }

  let migration_provenance = migration_provenance(connection).await?;
  let digest = schema_digest(&objects, &tables)?;
  Ok(SourceSchema {
    sqlite_version,
    objects,
    tables,
    migration_provenance,
    table_high_waters: Vec::new(),
    digest,
  })
}

pub(super) async fn capture_high_waters(
  connection: &mut SqliteConnection,
  schema: &mut SourceSchema,
) -> Result<(), CandidateError> {
  let mut high_waters = Vec::with_capacity(schema.tables.len());
  for table_index in 0..schema.tables.len() {
    let table = &schema.tables[table_index];
    let adaptive_columns = table
      .columns
      .iter()
      .enumerate()
      .filter(|(_, column)| supports_adaptive_numeric(column))
      .map(|(index, _)| index)
      .collect::<Vec<_>>();
    let mut projections = vec!["MAX(rowid)".to_owned(), "COUNT(*)".to_owned()];
    for column_index in &adaptive_columns {
      let identifier = quote_identifier(&table.columns[*column_index].name);
      projections.push(format!(
        "COALESCE(MAX(CASE WHEN typeof({identifier}) = 'integer' THEN 1 ELSE 0 END), 0)"
      ));
      projections.push(format!(
        "COALESCE(MAX(CASE WHEN typeof({identifier}) = 'real' THEN 1 ELSE 0 END), 0)"
      ));
    }
    let sql = format!(
      "SELECT {} FROM {}",
      projections.join(", "),
      quote_identifier(&table.name)
    );
    let row = sqlx::query(&sql)
      .fetch_one(&mut *connection)
      .await
      .map_err(source_read)?;
    let max_rowid: Option<i64> = row.try_get(0).map_err(source_read)?;
    let row_count: i64 = row.try_get(1).map_err(source_read)?;
    let row_count = u64::try_from(row_count).map_err(|_| CandidateError::SourceRead {
      message: format!("SQLite returned a negative row count for {}", table.name),
    })?;
    let observed_kinds = adaptive_columns
      .iter()
      .enumerate()
      .map(|(offset, column_index)| {
        let has_integer = row.try_get::<i64, _>(2 + offset * 2)? != 0;
        let has_real = row.try_get::<i64, _>(3 + offset * 2)? != 0;
        Ok((*column_index, observed_numeric_kind(has_integer, has_real)))
      })
      .collect::<Result<Vec<_>, sqlx::Error>>()
      .map_err(source_read)?;

    high_waters.push(TableHighWater {
      table_index,
      table_name: table.name.clone(),
      max_rowid,
      row_count,
    });
    for (column_index, native_kind) in observed_kinds {
      schema.tables[table_index].columns[column_index].native_kind = native_kind;
    }
  }
  schema.table_high_waters = high_waters;
  schema.digest = schema_digest(&schema.objects, &schema.tables)?;
  Ok(())
}

pub(super) async fn read_batch(
  connection: &mut SqliteConnection,
  table: &TableSchema,
  max_rowid: Option<i64>,
  after_rowid: Option<i64>,
  next_ordinal: u64,
) -> Result<Option<(RowBatch, i64)>, CandidateError> {
  let Some(max_rowid) = max_rowid else {
    return Ok(None);
  };
  let preflight_sql = preflight_sql(table);
  let preflight = sqlx::query(&preflight_sql)
    .bind(after_rowid)
    .bind(max_rowid)
    .bind(COPY_BATCH_ROWS)
    .fetch_all(&mut *connection)
    .await
    .map_err(source_read)?;
  if preflight.is_empty() {
    return Ok(None);
  }

  let mut accepted = Vec::with_capacity(preflight.len());
  let mut batch_bytes = 0_u64;
  for (row_index, row) in preflight.iter().enumerate() {
    let rowid: i64 = row.try_get(0).map_err(source_read)?;
    let ordinal = next_ordinal + row_index as u64;
    let mut row_bytes = 0_u64;
    let mut classes = Vec::with_capacity(table.columns.len());
    for (column_index, column) in table.columns.iter().enumerate() {
      let actual: String = row.try_get(1 + column_index * 2).map_err(source_read)?;
      let bytes: i64 = row.try_get(2 + column_index * 2).map_err(source_read)?;
      let bytes = u64::try_from(bytes).map_err(|_| CandidateError::SourceRead {
        message: format!(
          "SQLite returned a negative byte length for {}.{} at row ordinal {ordinal}",
          table.name, column.name
        ),
      })?;
      validate_storage_class(table, column, ordinal, &actual, bytes)?;
      row_bytes =
        row_bytes
          .checked_add(bytes)
          .ok_or_else(|| CandidateError::RowTooLarge {
            table: table.name.clone(),
            row_ordinal: ordinal,
            bytes: u64::MAX,
            limit: MAX_ROW_BYTES,
          })?;
      classes.push(actual);
    }
    if row_bytes > MAX_ROW_BYTES {
      return Err(CandidateError::RowTooLarge {
        table: table.name.clone(),
        row_ordinal: ordinal,
        bytes: row_bytes,
        limit: MAX_ROW_BYTES,
      });
    }
    if !accepted.is_empty() && batch_bytes.saturating_add(row_bytes) > MAX_BATCH_BYTES {
      break;
    }
    batch_bytes += row_bytes;
    accepted.push((rowid, ordinal, classes));
  }

  let last_rowid = accepted
    .last()
    .expect("preflight accepted at least one row")
    .0;
  let payload_sql = payload_sql(table);
  let payload = sqlx::query(&payload_sql)
    .bind(after_rowid)
    .bind(last_rowid)
    .fetch_all(&mut *connection)
    .await
    .map_err(source_read)?;
  if payload.len() != accepted.len() {
    return Err(CandidateError::SourceRead {
      message: format!(
        "{} changed inside its pinned source transaction: preflight rows {}, payload rows {}",
        table.name,
        accepted.len(),
        payload.len()
      ),
    });
  }

  let mut rows = Vec::with_capacity(payload.len());
  for (payload, (expected_rowid, ordinal, classes)) in payload.iter().zip(accepted) {
    let rowid: i64 = payload.try_get(0).map_err(source_read)?;
    if rowid != expected_rowid {
      return Err(CandidateError::SourceRead {
        message: format!(
          "{} row order changed inside the pinned source transaction",
          table.name
        ),
      });
    }
    let mut cells = Vec::with_capacity(table.columns.len());
    for (index, (column, storage_class)) in
      table.columns.iter().zip(classes.iter()).enumerate()
    {
      cells.push(decode_cell(
        payload,
        index + 1,
        column,
        storage_class,
        table,
        ordinal,
      )?);
    }
    rows.push(SourceRow { ordinal, cells });
  }
  Ok(Some((RowBatch { rows }, last_rowid)))
}

pub(super) fn verify_schema(
  source: &SourceSchema,
  reference: &SourceSchema,
) -> Result<(), CandidateError> {
  if source.objects != reference.objects || source.tables != reference.tables {
    return Err(CandidateError::SchemaMismatch {
      detail: "source sqlite_schema/table_xinfo/index/FK metadata differs from the caller-supplied migration reference".to_owned(),
    });
  }
  if source.migration_provenance != reference.migration_provenance {
    return Err(CandidateError::SchemaMismatch {
      detail: "source migration versions, descriptions, success state, or checksums differ from the caller-supplied migration reference".to_owned(),
    });
  }
  Ok(())
}

async fn inspect_table(
  connection: &mut SqliteConnection,
  objects: &[SchemaObject],
  name: String,
) -> Result<TableSchema, CandidateError> {
  let table_sql = objects
    .iter()
    .find(|object| object.object_type == "table" && object.name == name)
    .and_then(|object| object.sql.clone());
  let columns_sql = format!("PRAGMA table_xinfo({})", quote_string(&name));
  let column_rows = sqlx::query(&columns_sql)
    .fetch_all(&mut *connection)
    .await
    .map_err(source_read)?;
  let mut columns = Vec::with_capacity(column_rows.len());
  for row in column_rows {
    let column_name: String = row.try_get("name").map_err(source_read)?;
    let declared_type: String = row.try_get("type").map_err(source_read)?;
    let canonical_kind = canonical_kind(&name, &column_name, &declared_type)?;
    columns.push(ColumnSchema {
      cid: row.try_get("cid").map_err(source_read)?,
      canonical_kind,
      native_kind: canonical_kind,
      name: column_name,
      declared_type,
      not_null: row.try_get::<i64, _>("notnull").map_err(source_read)? != 0,
      default_sql: row.try_get("dflt_value").map_err(source_read)?,
      primary_key_ordinal: row.try_get("pk").map_err(source_read)?,
      hidden: row.try_get("hidden").map_err(source_read)?,
    });
  }

  let indexes_sql = format!("PRAGMA index_list({})", quote_string(&name));
  let index_rows = sqlx::query(&indexes_sql)
    .fetch_all(&mut *connection)
    .await
    .map_err(source_read)?;
  let mut indexes = Vec::with_capacity(index_rows.len());
  for row in index_rows {
    let index_name: String = row.try_get("name").map_err(source_read)?;
    let columns_sql = format!("PRAGMA index_xinfo({})", quote_string(&index_name));
    let index_column_rows = sqlx::query(&columns_sql)
      .fetch_all(&mut *connection)
      .await
      .map_err(source_read)?;
    let columns = index_column_rows
      .into_iter()
      .map(|column| {
        Ok(IndexColumnSchema {
          sequence: column.try_get("seqno")?,
          cid: column.try_get("cid")?,
          name: column.try_get("name")?,
          descending: column.try_get::<i64, _>("desc")? != 0,
          collation: column.try_get("coll")?,
          key: column.try_get::<i64, _>("key")? != 0,
        })
      })
      .collect::<Result<Vec<_>, sqlx::Error>>()
      .map_err(source_read)?;
    indexes.push(IndexSchema {
      sequence: row.try_get("seq").map_err(source_read)?,
      name: index_name,
      unique: row.try_get::<i64, _>("unique").map_err(source_read)? != 0,
      origin: row.try_get("origin").map_err(source_read)?,
      partial: row.try_get::<i64, _>("partial").map_err(source_read)? != 0,
      columns,
    });
  }

  let foreign_keys_sql = format!("PRAGMA foreign_key_list({})", quote_string(&name));
  let foreign_key_rows = sqlx::query(&foreign_keys_sql)
    .fetch_all(&mut *connection)
    .await
    .map_err(source_read)?;
  let foreign_keys = foreign_key_rows
    .into_iter()
    .map(|row| {
      Ok(ForeignKeySchema {
        id: row.try_get("id")?,
        sequence: row.try_get("seq")?,
        referenced_table: row.try_get("table")?,
        from_column: row.try_get("from")?,
        to_column: row.try_get("to")?,
        on_update: row.try_get("on_update")?,
        on_delete: row.try_get("on_delete")?,
        match_kind: row.try_get("match")?,
      })
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()
    .map_err(source_read)?;

  Ok(TableSchema {
    name,
    sql: table_sql,
    columns,
    indexes,
    foreign_keys,
  })
}

async fn migration_provenance(
  connection: &mut SqliteConnection,
) -> Result<MigrationProvenance, CandidateError> {
  let rows = sqlx::query(
    "SELECT version, description, success, checksum, typeof(success) AS success_type FROM _sqlx_migrations ORDER BY version",
  )
  .fetch_all(&mut *connection)
  .await
  .map_err(source_read)?;
  let mut migrations = Vec::with_capacity(rows.len());
  for row in rows {
    let version = row.try_get("version").map_err(source_read)?;
    let success: i64 = row.try_get("success").map_err(source_read)?;
    let success_type: String = row.try_get("success_type").map_err(source_read)?;
    if success_type != "integer" || success != 1 {
      return Err(CandidateError::SchemaMismatch {
        detail: format!(
          "source migration {version} has noncanonical success state {success} ({success_type}); expected integer 1"
        ),
      });
    }
    migrations.push(MigrationIdentity {
      version,
      description: row.try_get("description").map_err(source_read)?,
      checksum: row.try_get("checksum").map_err(source_read)?,
    });
  }
  let max_version = migrations.last().map_or(0, |migration| migration.version);
  Ok(MigrationProvenance {
    max_version,
    migration_count: migrations.len() as u64,
    migrations,
  })
}

fn supports_adaptive_numeric(column: &ColumnSchema) -> bool {
  matches!(
    column.declared_type.trim().to_ascii_uppercase().as_str(),
    "INTEGER" | "BIGINT"
  )
}

fn observed_numeric_kind(has_integer: bool, has_real: bool) -> CanonicalKind {
  match (has_integer, has_real) {
    (false, true) => CanonicalKind::Real,
    (true, true) => CanonicalKind::IntegerOrReal,
    (false, false) | (true, false) => CanonicalKind::Integer,
  }
}

fn preflight_sql(table: &TableSchema) -> String {
  let mut projections = vec!["rowid".to_owned()];
  for column in &table.columns {
    let identifier = quote_identifier(&column.name);
    projections.push(format!("typeof({identifier})"));
    projections.push(format!(
      "CASE WHEN typeof({identifier}) IN ('text','blob') THEN length(CAST({identifier} AS BLOB)) ELSE 0 END"
    ));
  }
  format!(
    "SELECT {} FROM {} WHERE (?1 IS NULL OR rowid > ?1) AND rowid <= ?2 ORDER BY rowid LIMIT ?3",
    projections.join(", "),
    quote_identifier(&table.name)
  )
}

fn payload_sql(table: &TableSchema) -> String {
  let projections = std::iter::once("rowid".to_owned())
    .chain(table.columns.iter().map(|column| {
      let identifier = quote_identifier(&column.name);
      match column.native_kind {
        CanonicalKind::Text | CanonicalKind::Blob => {
          format!("CAST({identifier} AS BLOB)")
        }
        CanonicalKind::Integer | CanonicalKind::Real | CanonicalKind::IntegerOrReal => {
          identifier
        }
      }
    }))
    .collect::<Vec<_>>();
  format!(
    "SELECT {} FROM {} WHERE (?1 IS NULL OR rowid > ?1) AND rowid <= ?2 ORDER BY rowid",
    projections.join(", "),
    quote_identifier(&table.name)
  )
}

fn decode_cell(
  row: &sqlx::sqlite::SqliteRow,
  index: usize,
  column: &ColumnSchema,
  storage_class: &str,
  table: &TableSchema,
  ordinal: u64,
) -> Result<Cell, CandidateError> {
  if storage_class == "null" {
    return Ok(Cell::Null);
  }
  match column.native_kind {
    CanonicalKind::Integer => row.try_get(index).map(Cell::Integer).map_err(source_read),
    CanonicalKind::Real => row
      .try_get::<f64, _>(index)
      .map(|value| Cell::Real(value.to_bits()))
      .map_err(source_read),
    CanonicalKind::IntegerOrReal => match storage_class {
      "integer" => row.try_get(index).map(Cell::Integer).map_err(source_read),
      "real" => row
        .try_get::<f64, _>(index)
        .map(|value| Cell::Real(value.to_bits()))
        .map_err(source_read),
      _ => Err(CandidateError::SourceRead {
        message: format!(
          "validated mixed numeric cell at {}.{} had unexpected storage class {storage_class}",
          table.name, column.name
        ),
      }),
    },
    CanonicalKind::Text => {
      let bytes: Vec<u8> = row.try_get(index).map_err(source_read)?;
      String::from_utf8(bytes)
        .map(Cell::Text)
        .map_err(|_| CandidateError::InvalidUtf8 {
          table: table.name.clone(),
          column: column.name.clone(),
          row_ordinal: ordinal,
        })
    }
    CanonicalKind::Blob => row.try_get(index).map(Cell::Blob).map_err(source_read),
  }
}

fn schema_digest(
  objects: &[SchemaObject],
  tables: &[TableSchema],
) -> Result<[u8; 32], CandidateError> {
  let bytes = serde_json::to_vec(&(objects, tables)).map_err(|error| {
    CandidateError::SourceRead {
      message: error.to_string(),
    }
  })?;
  Ok(Sha256::digest(bytes).into())
}

fn quote_identifier(value: &str) -> String {
  format!("\"{}\"", value.replace('"', "\"\""))
}

fn quote_string(value: &str) -> String {
  format!("'{}'", value.replace('\'', "''"))
}

fn source_read(error: sqlx::Error) -> CandidateError {
  CandidateError::SourceRead {
    message: error.to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_migrations() -> Vec<SchemaMigration> {
    vec![SchemaMigration {
      version: 1,
      description: "two_tables",
      sql: "CREATE TABLE alpha (id INTEGER PRIMARY KEY, value TEXT NOT NULL); CREATE TABLE beta (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
    }]
  }

  async fn source_pool(path: &Path) -> sqlx::SqlitePool {
    let options = SqliteConnectOptions::new()
      .filename(path)
      .create_if_missing(true)
      .row_buffer_size(1);
    SqlitePoolOptions::new()
      .max_connections(1)
      .connect_with(options)
      .await
      .unwrap()
  }

  #[tokio::test]
  async fn pinned_snapshot_keeps_original_schema_counts_and_rows_across_wal_writes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.sqlite3");
    let pool = source_pool(&path).await;
    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode = WAL")
      .fetch_one(&pool)
      .await
      .unwrap();
    assert_eq!(journal_mode, "wal");
    migrate::run_on_pool(&pool, test_migrations())
      .await
      .unwrap();
    sqlx::query("INSERT INTO alpha (id, value) VALUES (1, 'alpha-old')")
      .execute(&pool)
      .await
      .unwrap();
    sqlx::query("INSERT INTO beta (id, value) VALUES (1, 'beta-old')")
      .execute(&pool)
      .await
      .unwrap();

    let mut snapshot = open_source(&path).await.unwrap();
    sqlx::query("BEGIN").execute(&mut snapshot).await.unwrap();
    let mut original = inspect_schema(&mut snapshot).await.unwrap();
    capture_high_waters(&mut snapshot, &mut original)
      .await
      .unwrap();

    let mut writer = pool.begin().await.unwrap();
    sqlx::query("UPDATE alpha SET value = 'alpha-new' WHERE id = 1")
      .execute(&mut *writer)
      .await
      .unwrap();
    sqlx::query("INSERT INTO alpha (id, value) VALUES (2, 'alpha-added')")
      .execute(&mut *writer)
      .await
      .unwrap();
    sqlx::query("UPDATE beta SET value = 'beta-new' WHERE id = 1")
      .execute(&mut *writer)
      .await
      .unwrap();
    sqlx::query("INSERT INTO beta (id, value) VALUES (2, 'beta-added')")
      .execute(&mut *writer)
      .await
      .unwrap();
    writer.commit().await.unwrap();

    let mut after_commit = inspect_schema(&mut snapshot).await.unwrap();
    capture_high_waters(&mut snapshot, &mut after_commit)
      .await
      .unwrap();
    assert_eq!(after_commit, original);

    for (table_name, old_value) in [("alpha", "alpha-old"), ("beta", "beta-old")] {
      let table = original
        .tables
        .iter()
        .find(|table| table.name == table_name)
        .unwrap();
      let high_water = original
        .table_high_waters
        .iter()
        .find(|high_water| high_water.table_name == table_name)
        .unwrap();
      assert_eq!(high_water.row_count, 1);
      assert_eq!(high_water.max_rowid, Some(1));

      let (batch, last_rowid) =
        read_batch(&mut snapshot, table, high_water.max_rowid, None, 0)
          .await
          .unwrap()
          .unwrap();
      assert_eq!(last_rowid, 1);
      assert_eq!(
        batch.rows,
        vec![SourceRow {
          ordinal: 0,
          cells: vec![Cell::Integer(1), Cell::Text(old_value.to_owned())],
        }]
      );
      assert!(
        read_batch(
          &mut snapshot,
          table,
          high_water.max_rowid,
          Some(last_rowid),
          1,
        )
        .await
        .unwrap()
        .is_none()
      );
    }

    sqlx::query("ROLLBACK")
      .execute(&mut snapshot)
      .await
      .unwrap();
    pool.close().await;
  }

  #[tokio::test]
  async fn rejects_truthy_noncanonical_migration_success_value() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.sqlite3");
    let pool = source_pool(&path).await;
    migrate::run_on_pool(&pool, test_migrations())
      .await
      .unwrap();
    sqlx::query("UPDATE _sqlx_migrations SET success = 2")
      .execute(&pool)
      .await
      .unwrap();

    let mut source = open_source(&path).await.unwrap();
    sqlx::query("BEGIN").execute(&mut source).await.unwrap();
    let error = inspect_schema(&mut source).await.unwrap_err();
    assert!(matches!(error, CandidateError::SchemaMismatch { .. }));
    assert!(error.to_string().contains("expected integer 1"));
    pool.close().await;
  }
}
