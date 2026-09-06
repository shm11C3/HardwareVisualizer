use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use duckdb::types::Value;
use duckdb::{AccessMode, Config, Connection, appender_params_from_iter, params};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use super::CandidateError;
use super::CandidateTableReport;
use super::cell::{
  CanonicalKind, Cell, MAX_BATCH_BYTES, RowBatch, SourceRow, TableDigest,
};
use super::encode_hex;
use super::source::{SourceSchema, TableSchema};

const METADATA_TABLE: &str = "__hv_snapshot_metadata";
const SOURCE_ORDINAL_COLUMN: &str = "__hv_source_ordinal";
const VERIFY_ROWS: u64 = 512;

pub(super) struct CandidateSink {
  destination: PathBuf,
  database_path: PathBuf,
  connection: Option<Connection>,
  // Declared after `connection` so failure-path drop closes DuckDB before
  // TempDir removes the database and spill directory on Windows.
  directory: TempDir,
  schema_json: Vec<u8>,
  schema_digest: [u8; 32],
  tables: Vec<TableState>,
  current_table: usize,
}

pub(super) struct SinkValidation {
  pub(super) tables: Vec<CandidateTableReport>,
  pub(super) candidate_bytes: u64,
}

struct TableState {
  schema: TableSchema,
  next_ordinal: u64,
  row_count: u64,
  digest: Sha256,
  finished: bool,
}

impl CandidateSink {
  pub(super) fn create(
    destination: &Path,
    schema: &SourceSchema,
  ) -> Result<Self, CandidateError> {
    if destination.exists() {
      return Err(candidate_error(
        "reserve candidate destination",
        format!("destination already exists: {}", destination.display()),
      ));
    }
    let parent = destination
      .parent()
      .filter(|path| !path.as_os_str().is_empty())
      .ok_or_else(|| {
        candidate_error(
          "reserve candidate destination",
          "destination must have a parent",
        )
      })?;
    if !parent.is_dir() {
      return Err(candidate_error(
        "reserve candidate destination",
        format!("destination parent does not exist: {}", parent.display()),
      ));
    }
    validate_reserved_names(schema)?;

    let directory = tempfile::Builder::new()
      .prefix(".hardwarevisualizer-duckdb-candidate-")
      .tempdir_in(parent)
      .map_err(|error| candidate_error("reserve candidate work directory", error))?;
    let database_path = directory.path().join("snapshot.duckdb");
    let spill_path = directory.path().join("spill");
    fs::create_dir(&spill_path)
      .map_err(|error| candidate_error("create candidate spill directory", error))?;
    let config = candidate_config(AccessMode::ReadWrite)?;
    let connection = Connection::open_with_flags(&database_path, config)
      .map_err(|error| candidate_error("open DuckDB candidate", error))?;
    configure_spill(&connection, &spill_path)?;
    let version: String = connection
      .query_row("SELECT version()", [], |row| row.get(0))
      .map_err(|error| candidate_error("read DuckDB candidate version", error))?;
    if version.trim_start_matches('v') != "1.5.5" {
      return Err(candidate_error(
        "verify DuckDB candidate version",
        format!("expected 1.5.5, got {version}"),
      ));
    }

    let schema_json = serde_json::to_vec(schema)
      .map_err(|error| candidate_error("encode source schema metadata", error))?;
    connection
      .execute_batch("BEGIN TRANSACTION")
      .map_err(|error| candidate_error("begin candidate transaction", error))?;
    connection
      .execute_batch(&format!(
        "CREATE TABLE {} (snapshot_kind VARCHAR NOT NULL, source_schema_json BLOB NOT NULL, source_schema_digest BLOB NOT NULL)",
        quote_identifier(METADATA_TABLE)
      ))
      .map_err(|error| candidate_error("create candidate metadata table", error))?;
    connection
      .execute(
        &format!(
          "INSERT INTO {} VALUES (?, ?, ?)",
          quote_identifier(METADATA_TABLE)
        ),
        params![
          "immutable_source_snapshot",
          &schema_json,
          schema.digest.as_slice()
        ],
      )
      .map_err(|error| candidate_error("write candidate metadata", error))?;

    for table in &schema.tables {
      connection
        .execute_batch(&create_table_sql(table))
        .map_err(|error| {
          candidate_error(format!("create candidate table {}", table.name), error)
        })?;
    }

    Ok(Self {
      destination: destination.to_owned(),
      database_path,
      connection: Some(connection),
      directory,
      schema_json,
      schema_digest: schema.digest,
      tables: schema
        .tables
        .iter()
        .cloned()
        .map(|schema| TableState {
          schema,
          next_ordinal: 0,
          row_count: 0,
          digest: Sha256::new(),
          finished: false,
        })
        .collect(),
      current_table: 0,
    })
  }

  pub(super) fn write_batch(
    &mut self,
    table: &TableSchema,
    batch: &RowBatch,
  ) -> Result<(), CandidateError> {
    let state_index = self.current_state_index(table)?;
    let expected_ordinal = self.tables[state_index].next_ordinal;
    if batch.rows.len() > 512 {
      return Err(candidate_error(
        format!("write candidate table {}", table.name),
        format!("batch contains {} rows; maximum is 512", batch.rows.len()),
      ));
    }
    for (offset, row) in batch.rows.iter().enumerate() {
      let ordinal = expected_ordinal.checked_add(offset as u64).ok_or_else(|| {
        candidate_error(
          format!("write candidate table {}", table.name),
          "source ordinal overflowed u64",
        )
      })?;
      if row.ordinal != ordinal {
        return Err(candidate_error(
          format!("write candidate table {}", table.name),
          format!(
            "source ordinal {} is not the expected contiguous ordinal {}",
            row.ordinal, ordinal
          ),
        ));
      }
      if row.cells.len() != table.columns.len() {
        return Err(candidate_error(
          format!("write candidate table {}", table.name),
          format!(
            "row {} has {} cells for {} columns",
            row.ordinal,
            row.cells.len(),
            table.columns.len()
          ),
        ));
      }
    }

    let connection = self.connection.as_ref().ok_or_else(|| {
      candidate_error("write candidate batch", "candidate connection is closed")
    })?;
    let mut appender = connection.appender(&table.name).map_err(|error| {
      candidate_error(format!("open appender for {}", table.name), error)
    })?;
    for row in &batch.rows {
      let mut values = row.cells.iter().map(cell_value).collect::<Vec<_>>();
      values.push(Value::UBigInt(row.ordinal));
      appender
        .append_row(appender_params_from_iter(values))
        .map_err(|error| {
          candidate_error(format!("append candidate row into {}", table.name), error)
        })?;
    }
    appender.flush().map_err(|error| {
      candidate_error(format!("flush appender for {}", table.name), error)
    })?;
    drop(appender);
    let state = &mut self.tables[state_index];
    for row in &batch.rows {
      row.update_digest(&mut state.digest);
      state.next_ordinal = state.next_ordinal.checked_add(1).ok_or_else(|| {
        candidate_error(
          format!("write candidate table {}", table.name),
          "source ordinal overflowed u64",
        )
      })?;
      state.row_count = state.row_count.checked_add(1).ok_or_else(|| {
        candidate_error(
          format!("write candidate table {}", table.name),
          "candidate row count overflowed u64",
        )
      })?;
    }
    Ok(())
  }

  pub(super) fn finish_table(
    &mut self,
    table: &TableSchema,
    expected: &TableDigest,
  ) -> Result<(), CandidateError> {
    let state_index = self.current_state_index(table)?;
    let state = &mut self.tables[state_index];
    let actual_digest: [u8; 32] = state.digest.clone().finalize().into();
    if expected.name != table.name
      || expected.row_count != state.row_count
      || expected.sha256 != actual_digest
    {
      return Err(candidate_error(
        format!("finish candidate table {}", table.name),
        "source count or tagged-cell digest changed between source and sink",
      ));
    }
    state.finished = true;
    self.current_table += 1;
    Ok(())
  }

  pub(super) fn finish_and_validate(
    mut self,
    expected: &[TableDigest],
  ) -> Result<SinkValidation, CandidateError> {
    if self.current_table != self.tables.len() || expected.len() != self.tables.len() {
      return Err(candidate_error(
        "finish candidate snapshot",
        format!(
          "finished {} of {} tables with {} expected digests",
          self.current_table,
          self.tables.len(),
          expected.len()
        ),
      ));
    }
    for (state, expected) in self.tables.iter().zip(expected) {
      if !state.finished || state.schema.name != expected.name {
        return Err(candidate_error(
          "finish candidate snapshot",
          "table completion order does not match the source schema",
        ));
      }
    }

    let connection = self.connection.take().ok_or_else(|| {
      candidate_error(
        "finish candidate snapshot",
        "candidate connection is closed",
      )
    })?;
    connection
      .execute_batch("COMMIT; CHECKPOINT")
      .map_err(|error| {
        candidate_error("commit and checkpoint candidate snapshot", error)
      })?;
    drop(connection);
    require_no_wal(&self.database_path)?;

    let spill_path = self.directory.path().join("spill");
    let config = candidate_config(AccessMode::ReadOnly)?;
    let reopened = Connection::open_with_flags(&self.database_path, config)
      .map_err(|error| candidate_error("reopen candidate read-only", error))?;
    configure_spill(&reopened, &spill_path)?;
    validate_metadata(&reopened, &self.schema_json, &self.schema_digest)?;
    let mut table_reports = Vec::with_capacity(self.tables.len());
    for (state, expected) in self.tables.iter().zip(expected) {
      let reopened = validate_table(&reopened, &state.schema, expected)?;
      table_reports.push(CandidateTableReport {
        name: state.schema.name.clone(),
        source_rows: expected.row_count,
        source_sha256: encode_hex(&expected.sha256),
        reopened_rows: reopened.row_count,
        reopened_sha256: encode_hex(&reopened.sha256),
      });
    }
    drop(reopened);
    require_no_wal(&self.database_path)?;

    let candidate_file = OpenOptions::new()
      .read(true)
      .write(true)
      .open(&self.database_path)
      .map_err(|error| candidate_error("open validated candidate for sync", error))?;
    candidate_file
      .sync_all()
      .map_err(|error| candidate_error("sync validated candidate", error))?;
    let candidate_bytes = candidate_file
      .metadata()
      .map_err(|error| candidate_error("measure validated candidate", error))?
      .len();
    drop(candidate_file);
    fs::hard_link(&self.database_path, &self.destination).map_err(|error| {
      candidate_error(
        "publish validated candidate without replacement",
        format!("{}: {error}", self.destination.display()),
      )
    })?;
    drop(self.directory);
    Ok(SinkValidation {
      tables: table_reports,
      candidate_bytes,
    })
  }

  fn current_state_index(&self, table: &TableSchema) -> Result<usize, CandidateError> {
    let state = self.tables.get(self.current_table).ok_or_else(|| {
      candidate_error(
        "write candidate snapshot",
        "all source tables are already finished",
      )
    })?;
    if state.finished || state.schema != *table {
      return Err(candidate_error(
        "write candidate snapshot",
        format!(
          "received table {} while expecting {}",
          table.name, state.schema.name
        ),
      ));
    }
    Ok(self.current_table)
  }
}

fn validate_reserved_names(schema: &SourceSchema) -> Result<(), CandidateError> {
  for table in &schema.tables {
    if table.name.eq_ignore_ascii_case(METADATA_TABLE) {
      return Err(candidate_error(
        "validate candidate schema",
        format!("source table uses reserved name {METADATA_TABLE}"),
      ));
    }
    if table
      .columns
      .iter()
      .any(|column| column.name.eq_ignore_ascii_case(SOURCE_ORDINAL_COLUMN))
    {
      return Err(candidate_error(
        "validate candidate schema",
        format!(
          "source table {} uses reserved column name {SOURCE_ORDINAL_COLUMN}",
          table.name
        ),
      ));
    }
  }
  Ok(())
}

fn create_table_sql(table: &TableSchema) -> String {
  let mut columns = table
    .columns
    .iter()
    .map(|column| {
      format!(
        "{} {}",
        quote_identifier(&column.name),
        duckdb_type(column.canonical_kind)
      )
    })
    .collect::<Vec<_>>();
  // Source rowid order is validation metadata for this immutable candidate.
  // It is deliberately not a source key or part of the future authoritative schema.
  columns.push(format!(
    "{} UBIGINT NOT NULL",
    quote_identifier(SOURCE_ORDINAL_COLUMN)
  ));
  format!(
    "CREATE TABLE {} ({})",
    quote_identifier(&table.name),
    columns.join(", ")
  )
}

fn validate_metadata(
  connection: &Connection,
  expected_json: &[u8],
  expected_digest: &[u8; 32],
) -> Result<(), CandidateError> {
  let (kind, schema_json, schema_digest): (String, Vec<u8>, Vec<u8>) = connection
    .query_row(
      &format!(
        "SELECT snapshot_kind, source_schema_json, source_schema_digest FROM {}",
        quote_identifier(METADATA_TABLE)
      ),
      [],
      |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .map_err(|error| {
      candidate_error("validate candidate metadata after reopen", error)
    })?;
  if kind != "immutable_source_snapshot"
    || schema_json != expected_json
    || schema_digest.as_slice() != expected_digest
  {
    return Err(candidate_error(
      "validate candidate metadata after reopen",
      "snapshot kind, source schema metadata, or schema digest differs",
    ));
  }
  Ok(())
}

fn validate_table(
  connection: &Connection,
  table: &TableSchema,
  expected: &TableDigest,
) -> Result<TableDigest, CandidateError> {
  let count: i64 = connection
    .query_row(
      &format!("SELECT COUNT(*) FROM {}", quote_identifier(&table.name)),
      [],
      |row| row.get(0),
    )
    .map_err(|error| {
      candidate_error(format!("count reopened table {}", table.name), error)
    })?;
  let count = u64::try_from(count).map_err(|_| {
    candidate_error(
      format!("count reopened table {}", table.name),
      "DuckDB returned a negative row count",
    )
  })?;
  if expected.name != table.name || count != expected.row_count {
    return Err(candidate_error(
      format!("validate reopened table {}", table.name),
      format!("expected {} rows, found {count}", expected.row_count),
    ));
  }

  let mut digest = Sha256::new();
  let mut next_ordinal = 0_u64;
  while next_ordinal < count {
    let ordinals = readback_preflight(connection, table, next_ordinal)?;
    if ordinals.is_empty() {
      return Err(candidate_error(
        format!("validate reopened table {}", table.name),
        format!("missing source ordinal {next_ordinal}"),
      ));
    }
    let last = *ordinals.last().expect("non-empty preflight");
    let sql = readback_payload_sql(table);
    let mut statement = connection.prepare(&sql).map_err(|error| {
      candidate_error(format!("prepare reopened read for {}", table.name), error)
    })?;
    let mut rows = statement
      .query(params![next_ordinal, last])
      .map_err(|error| {
        candidate_error(format!("read reopened table {}", table.name), error)
      })?;
    let mut read = 0_usize;
    while let Some(row) = rows.next().map_err(|error| {
      candidate_error(format!("read reopened table {}", table.name), error)
    })? {
      let ordinal: u64 = row.get(0).map_err(|error| {
        candidate_error(format!("decode reopened ordinal for {}", table.name), error)
      })?;
      if ordinal != next_ordinal + read as u64 {
        return Err(candidate_error(
          format!("validate reopened table {}", table.name),
          format!("non-contiguous source ordinal {ordinal}"),
        ));
      }
      let cells = table
        .columns
        .iter()
        .enumerate()
        .map(|(index, column)| read_cell(row, index + 1, column.canonical_kind))
        .collect::<Result<Vec<_>, _>>()?;
      SourceRow { ordinal, cells }.update_digest(&mut digest);
      read += 1;
    }
    if read != ordinals.len() {
      return Err(candidate_error(
        format!("validate reopened table {}", table.name),
        "readback preflight and payload row counts differ",
      ));
    }
    next_ordinal = last.checked_add(1).ok_or_else(|| {
      candidate_error(
        format!("validate reopened table {}", table.name),
        "source ordinal overflowed u64",
      )
    })?;
  }
  let actual: [u8; 32] = digest.finalize().into();
  if actual != expected.sha256 {
    return Err(candidate_error(
      format!("validate reopened table {}", table.name),
      "tagged full-cell digest differs after close and reopen",
    ));
  }
  Ok(TableDigest {
    name: table.name.clone(),
    row_count: count,
    sha256: actual,
  })
}

fn readback_preflight(
  connection: &Connection,
  table: &TableSchema,
  next_ordinal: u64,
) -> Result<Vec<u64>, CandidateError> {
  let end_ordinal = next_ordinal.checked_add(VERIFY_ROWS).ok_or_else(|| {
    candidate_error(
      format!("bound readback preflight for {}", table.name),
      "source ordinal window overflowed u64",
    )
  })?;
  let byte_expression = table
    .columns
    .iter()
    .map(|column| match column.canonical_kind {
      CanonicalKind::Integer | CanonicalKind::Real => format!(
        "CASE WHEN {} IS NULL THEN 0 ELSE 8 END",
        quote_identifier(&column.name)
      ),
      CanonicalKind::Text => format!(
        "CASE WHEN {} IS NULL THEN 0 ELSE octet_length(encode({})) END",
        quote_identifier(&column.name),
        quote_identifier(&column.name)
      ),
      CanonicalKind::Blob => format!(
        "CASE WHEN {} IS NULL THEN 0 ELSE octet_length({}) END",
        quote_identifier(&column.name),
        quote_identifier(&column.name)
      ),
    })
    .collect::<Vec<_>>()
    .join(" + ");
  let sql = format!(
    "SELECT {}, {byte_expression} FROM {} WHERE {} >= ? AND {} < ? ORDER BY {} LIMIT {VERIFY_ROWS}",
    quote_identifier(SOURCE_ORDINAL_COLUMN),
    quote_identifier(&table.name),
    quote_identifier(SOURCE_ORDINAL_COLUMN),
    quote_identifier(SOURCE_ORDINAL_COLUMN),
    quote_identifier(SOURCE_ORDINAL_COLUMN),
  );
  let mut statement = connection.prepare(&sql).map_err(|error| {
    candidate_error(
      format!("prepare readback preflight for {}", table.name),
      error,
    )
  })?;
  let mut rows = statement
    .query(params![next_ordinal, end_ordinal])
    .map_err(|error| {
      candidate_error(format!("readback preflight for {}", table.name), error)
    })?;
  let mut accepted = Vec::new();
  let mut bytes = 0_u64;
  while let Some(row) = rows.next().map_err(|error| {
    candidate_error(format!("readback preflight for {}", table.name), error)
  })? {
    let ordinal: u64 = row.get(0).map_err(|error| {
      candidate_error(format!("decode readback ordinal for {}", table.name), error)
    })?;
    let row_bytes: i64 = row.get(1).map_err(|error| {
      candidate_error(format!("decode readback size for {}", table.name), error)
    })?;
    let row_bytes = u64::try_from(row_bytes).map_err(|_| {
      candidate_error(
        format!("decode readback size for {}", table.name),
        "DuckDB returned a negative row byte count",
      )
    })?;
    if !accepted.is_empty() && bytes.saturating_add(row_bytes) > MAX_BATCH_BYTES {
      break;
    }
    bytes = bytes.saturating_add(row_bytes);
    accepted.push(ordinal);
  }
  Ok(accepted)
}

fn readback_payload_sql(table: &TableSchema) -> String {
  let mut columns = vec![quote_identifier(SOURCE_ORDINAL_COLUMN)];
  columns.extend(
    table
      .columns
      .iter()
      .map(|column| quote_identifier(&column.name)),
  );
  format!(
    "SELECT {} FROM {} WHERE {} BETWEEN ? AND ? ORDER BY {}",
    columns.join(", "),
    quote_identifier(&table.name),
    quote_identifier(SOURCE_ORDINAL_COLUMN),
    quote_identifier(SOURCE_ORDINAL_COLUMN),
  )
}

fn read_cell(
  row: &duckdb::Row<'_>,
  index: usize,
  kind: CanonicalKind,
) -> Result<Cell, CandidateError> {
  match kind {
    CanonicalKind::Integer => row
      .get::<_, Option<i64>>(index)
      .map(|value| value.map_or(Cell::Null, Cell::Integer)),
    CanonicalKind::Real => row
      .get::<_, Option<f64>>(index)
      .map(|value| value.map_or(Cell::Null, |value| Cell::Real(value.to_bits()))),
    CanonicalKind::Text => row
      .get::<_, Option<String>>(index)
      .map(|value| value.map_or(Cell::Null, Cell::Text)),
    CanonicalKind::Blob => row
      .get::<_, Option<Vec<u8>>>(index)
      .map(|value| value.map_or(Cell::Null, Cell::Blob)),
  }
  .map_err(|error| candidate_error("decode reopened candidate cell", error))
}

fn cell_value(cell: &Cell) -> Value {
  match cell {
    Cell::Null => Value::Null,
    Cell::Integer(value) => Value::BigInt(*value),
    Cell::Real(bits) => Value::Double(f64::from_bits(*bits)),
    Cell::Text(value) => Value::Text(value.clone()),
    Cell::Blob(value) => Value::Blob(value.clone()),
  }
}

fn duckdb_type(kind: CanonicalKind) -> &'static str {
  match kind {
    CanonicalKind::Integer => "BIGINT",
    CanonicalKind::Real => "DOUBLE",
    CanonicalKind::Text => "VARCHAR",
    CanonicalKind::Blob => "BLOB",
  }
}

fn candidate_config(access_mode: AccessMode) -> Result<Config, CandidateError> {
  Config::default()
    .access_mode(access_mode)
    .and_then(|config| config.threads(2))
    .and_then(|config| config.max_memory("128MB"))
    .and_then(|config| config.enable_autoload_extension(false))
    .map_err(|error| candidate_error("configure DuckDB candidate", error))
}

fn configure_spill(
  connection: &Connection,
  spill_path: &Path,
) -> Result<(), CandidateError> {
  let spill = spill_path.to_str().ok_or_else(|| {
    candidate_error(
      "configure candidate spill directory",
      format!("path is not valid UTF-8: {}", spill_path.display()),
    )
  })?;
  // DuckDB rejects temp_directory changes after external access is disabled.
  // Set the owned spill path first, before any copy or validation query runs.
  connection
    .execute_batch(&format!(
      "SET temp_directory = '{}'; SET enable_external_access = false",
      spill.replace('\'', "''")
    ))
    .map_err(|error| candidate_error("configure candidate spill directory", error))
}

fn quote_identifier(identifier: &str) -> String {
  format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn require_no_wal(database: &Path) -> Result<(), CandidateError> {
  let mut wal = database.as_os_str().to_os_string();
  wal.push(".wal");
  let wal = PathBuf::from(wal);
  if wal.exists() {
    return Err(candidate_error(
      "verify closed candidate has no WAL",
      format!("WAL remains at {}", wal.display()),
    ));
  }
  Ok(())
}

fn candidate_error(
  context: impl Into<String>,
  error: impl std::fmt::Display,
) -> CandidateError {
  let context = context.into();
  CandidateError::candidate(&context, error)
}
