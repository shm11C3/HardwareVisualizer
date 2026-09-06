//! Creates an unselected native snapshot from the authoritative SQLite database.
//!
//! SQLite remains the source of truth. Reconciliation and native schema finalization are
//! required before a separate lifecycle owner can select the candidate.

mod cell;
mod sink;
mod source;

use std::fmt::Display;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use self::cell::TableDigest;
use self::sink::CandidateSink;
use self::source::{SourceSchema, TableSchema};
use super::migrate::SchemaMigration;

const SOURCE_ORDINAL_COLUMN: &str = "__hv_source_ordinal";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CandidateTableReport {
  pub name: String,
  pub source_rows: u64,
  pub source_sha256: String,
  pub reopened_rows: u64,
  pub reopened_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CandidateReport {
  pub candidate_path: PathBuf,
  pub snapshot_kind: String,
  pub source_sqlite_version: String,
  pub source_schema_sha256: String,
  pub source_migration_max_version: i64,
  pub source_migration_count: u64,
  pub tables: Vec<CandidateTableReport>,
  pub total_rows: u64,
  pub candidate_bytes: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum CandidateError {
  #[error("source SQLite database does not exist or is not a file: {path}")]
  SourceUnavailable { path: PathBuf },
  #[error("candidate destination already exists: {path}")]
  DestinationExists { path: PathBuf },
  #[error("failed to build the caller-supplied migration reference: {message}")]
  ReferenceMigration { message: String },
  #[error("failed to open the source SQLite database: {message}")]
  SourceOpen { message: String },
  #[error(
    "source schema does not match the caller-supplied migration reference: {detail}"
  )]
  SchemaMismatch { detail: String },
  #[error("unsupported declared type {declared_type:?} at {table}.{column}")]
  UnsupportedDeclaredType {
    table: String,
    column: String,
    declared_type: String,
  },
  #[error(
    "noncanonical SQLite cell at {table}.{column}, row ordinal {row_ordinal}: expected {expected}, found {actual}"
  )]
  NonCanonicalCell {
    table: String,
    column: String,
    row_ordinal: u64,
    expected: &'static str,
    actual: String,
  },
  #[error("NULL in required column {table}.{column} at row ordinal {row_ordinal}")]
  NullInRequiredColumn {
    table: String,
    column: String,
    row_ordinal: u64,
  },
  #[error("invalid UTF-8 in TEXT column {table}.{column} at row ordinal {row_ordinal}")]
  InvalidUtf8 {
    table: String,
    column: String,
    row_ordinal: u64,
  },
  #[error(
    "cell at {table}.{column}, row ordinal {row_ordinal}, is {bytes} bytes (limit {limit})"
  )]
  CellTooLarge {
    table: String,
    column: String,
    row_ordinal: u64,
    bytes: u64,
    limit: u64,
  },
  #[error("row in {table} at ordinal {row_ordinal} is {bytes} bytes (limit {limit})")]
  RowTooLarge {
    table: String,
    row_ordinal: u64,
    bytes: u64,
    limit: u64,
  },
  #[error("failed to read the pinned source snapshot: {message}")]
  SourceRead { message: String },
  #[error("candidate write or reopen validation failed: {message}")]
  Candidate { message: String },
  #[error("candidate copy worker failed: {message}")]
  Worker { message: String },
}

impl CandidateError {
  pub(super) fn candidate(context: &str, error: impl Display) -> Self {
    Self::Candidate {
      message: format!("{context}: {error}"),
    }
  }
}

/// Copies one pinned, canonical SQLite snapshot into a new native candidate file.
///
/// Callers must await this future to completion and act only on a successful report. The
/// blocking copy may continue if its async future is dropped; cancellation and publication
/// belong to the later lifecycle/reconciliation boundary.
pub async fn create_candidate(
  source: &Path,
  destination: &Path,
  migrations: Vec<SchemaMigration>,
) -> Result<CandidateReport, CandidateError> {
  validate_paths(source, destination)?;
  let source = source.to_owned();
  let destination = destination.to_owned();

  tokio::task::spawn_blocking(move || {
    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .map_err(|error| CandidateError::Worker {
        message: error.to_string(),
      })?;
    runtime.block_on(copy_snapshot(&source, &destination, migrations))
  })
  .await
  .map_err(|error| CandidateError::Worker {
    message: error.to_string(),
  })?
}

async fn copy_snapshot(
  source_path: &Path,
  destination: &Path,
  migrations: Vec<SchemaMigration>,
) -> Result<CandidateReport, CandidateError> {
  let reference = source::reference_schema(migrations).await?;
  let mut connection = source::open_source(source_path).await?;
  sqlx::query("BEGIN")
    .execute(&mut connection)
    .await
    .map_err(source_error)?;

  let mut schema = source::inspect_schema(&mut connection).await?;
  source::verify_schema(&schema, &reference)?;
  reject_reserved_columns(&schema)?;
  source::capture_high_waters(&mut connection, &mut schema).await?;

  let mut sink = CandidateSink::create(destination, &schema)?;
  let mut expected = Vec::with_capacity(schema.tables.len());
  if schema.tables.len() != schema.table_high_waters.len() {
    return Err(CandidateError::SourceRead {
      message: "captured table high-water metadata is incomplete".to_owned(),
    });
  }
  for (table_index, (table, high_water)) in schema
    .tables
    .iter()
    .zip(&schema.table_high_waters)
    .enumerate()
  {
    if high_water.table_index != table_index || high_water.table_name != table.name {
      return Err(CandidateError::SourceRead {
        message: "captured table high-water metadata is out of order".to_owned(),
      });
    }
    let table_digest = copy_table(
      &mut connection,
      &mut sink,
      table,
      high_water.max_rowid,
      high_water.row_count,
    )
    .await?;
    sink.finish_table(table, &table_digest)?;
    expected.push(table_digest);
  }

  sqlx::query("ROLLBACK")
    .execute(&mut connection)
    .await
    .map_err(source_error)?;
  drop(connection);

  let validation = sink.finish_and_validate(&expected)?;
  let total_rows = validation.tables.iter().try_fold(0_u64, |total, table| {
    total
      .checked_add(table.source_rows)
      .ok_or_else(|| CandidateError::Candidate {
        message: "total copied row count overflowed u64".to_owned(),
      })
  })?;

  Ok(CandidateReport {
    candidate_path: destination.to_owned(),
    snapshot_kind: "immutable_source_snapshot".to_owned(),
    source_sqlite_version: schema.sqlite_version,
    source_schema_sha256: encode_hex(&schema.digest),
    source_migration_max_version: schema.migration_provenance.max_version,
    source_migration_count: schema.migration_provenance.migration_count,
    tables: validation.tables,
    total_rows,
    candidate_bytes: validation.candidate_bytes,
  })
}

async fn copy_table(
  connection: &mut sqlx::SqliteConnection,
  sink: &mut CandidateSink,
  table: &TableSchema,
  max_rowid: Option<i64>,
  expected_row_count: u64,
) -> Result<TableDigest, CandidateError> {
  let mut after_rowid = None;
  let mut next_ordinal = 0_u64;
  let mut digest = Sha256::new();

  while let Some((batch, last_rowid)) =
    source::read_batch(connection, table, max_rowid, after_rowid, next_ordinal).await?
  {
    for row in &batch.rows {
      row.update_digest(&mut digest);
    }
    next_ordinal = next_ordinal
      .checked_add(batch.rows.len() as u64)
      .ok_or_else(|| CandidateError::SourceRead {
        message: format!("row count overflow while copying {}", table.name),
      })?;
    sink.write_batch(table, &batch)?;
    after_rowid = Some(last_rowid);
  }

  if next_ordinal != expected_row_count {
    return Err(CandidateError::SourceRead {
      message: format!(
        "{} pinned COUNT(*) was {expected_row_count}, but the bounded scan returned {next_ordinal}",
        table.name
      ),
    });
  }

  Ok(TableDigest {
    name: table.name.clone(),
    row_count: next_ordinal,
    sha256: digest.finalize().into(),
  })
}

fn reject_reserved_columns(schema: &SourceSchema) -> Result<(), CandidateError> {
  for table in &schema.tables {
    if table
      .columns
      .iter()
      .any(|column| column.name.eq_ignore_ascii_case(SOURCE_ORDINAL_COLUMN))
    {
      return Err(CandidateError::SchemaMismatch {
        detail: format!(
          "table {} uses reserved candidate validation column {SOURCE_ORDINAL_COLUMN}",
          table.name
        ),
      });
    }
    if let Some(column) = table.columns.iter().find(|column| {
      ["rowid", "_rowid_", "oid"]
        .iter()
        .any(|alias| column.name.eq_ignore_ascii_case(alias))
    }) {
      return Err(CandidateError::SchemaMismatch {
        detail: format!(
          "table {} shadows SQLite row identifier alias {}; bounded snapshot order is unavailable",
          table.name, column.name
        ),
      });
    }
  }
  Ok(())
}

fn validate_paths(source: &Path, destination: &Path) -> Result<(), CandidateError> {
  let source_metadata =
    std::fs::metadata(source).map_err(|_| CandidateError::SourceUnavailable {
      path: source.to_owned(),
    })?;
  if !source_metadata.is_file() {
    return Err(CandidateError::SourceUnavailable {
      path: source.to_owned(),
    });
  }

  match std::fs::symlink_metadata(destination) {
    Ok(_) => {
      return Err(CandidateError::DestinationExists {
        path: destination.to_owned(),
      });
    }
    Err(error) if error.kind() == ErrorKind::NotFound => {}
    Err(error) => {
      return Err(CandidateError::Candidate {
        message: format!("failed to inspect candidate destination: {error}"),
      });
    }
  }

  Ok(())
}

fn source_error(error: sqlx::Error) -> CandidateError {
  CandidateError::SourceRead {
    message: error.to_string(),
  }
}

pub(super) fn encode_hex(bytes: &[u8]) -> String {
  use std::fmt::Write;

  let mut encoded = String::with_capacity(bytes.len() * 2);
  for byte in bytes {
    write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
  }
  encoded
}
