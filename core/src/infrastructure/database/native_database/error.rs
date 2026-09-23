use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum NativeDatabaseError {
  #[error("native database file does not exist or is not a file: {path}")]
  Unavailable { path: PathBuf },
  #[error("candidate database does not exist or is not a file: {path}")]
  CandidateUnavailable { path: PathBuf },
  #[error("native database destination already exists: {path}")]
  DestinationExists { path: PathBuf },
  #[error("fresh native database creation found an existing artifact: {path}")]
  FreshInstallArtifactExists { path: PathBuf },
  #[error("native database request capacity must be greater than zero")]
  InvalidRequestCapacity,
  #[error("native database is already open in this process: {path}")]
  AlreadyOpen { path: PathBuf },
  #[error("native database is not finalized")]
  Unfinalized,
  #[error(
    "native database schema version {actual} is incompatible with expected version {expected}"
  )]
  IncompatibleSchema { expected: u32, actual: u32 },
  #[error(
    "native database storage version {actual:?} is incompatible with recorded version {expected:?}"
  )]
  StorageVersionMismatch { expected: String, actual: String },
  #[error("native database metadata does not record a storage version")]
  StorageVersionMetadataMissing,
  #[error("native database request cancellation may be attached to only one request")]
  CancellationAlreadyUsed,
  #[error("native database request was cancelled")]
  Cancelled,
  #[error("native database is closed")]
  Closed,
  #[error("native database instance was invalidated and must be reopened")]
  Invalidated,
  #[error("native database worker failed: {message}")]
  Worker { message: String },
  #[error("native database operation failed during {context}: {source}")]
  DuckDb {
    context: &'static str,
    #[source]
    source: duckdb::Error,
  },
  #[error("candidate snapshot cannot be finalized: {message}")]
  CandidateSnapshot { message: String },
  #[error("candidate table {table} does not match the supplied native schema: {detail}")]
  SchemaMismatch { table: String, detail: String },
  #[error(
    "candidate cell at {table}.{column}, source row ordinal {row_ordinal}, is {candidate} and the native column is {destination}"
  )]
  UnrepresentableCell {
    table: String,
    column: String,
    row_ordinal: u64,
    candidate: &'static str,
    destination: String,
  },
  #[error(
    "{table}.{column} is NOT NULL and the reading is NaN, which SQLite refuses to store"
  )]
  NotANumberInRequiredColumn {
    table: &'static str,
    column: &'static str,
  },
  #[error(
    "NULL in required native column {table}.{column} at source row ordinal {row_ordinal}"
  )]
  NullInRequiredColumn {
    table: String,
    column: String,
    row_ordinal: u64,
  },
  #[error("finalized native database failed verification: {message}")]
  Verification { message: String },
  #[error(
    "the exact sum of {column} for Process ({pid}, {process_name:?}) exceeds a signed 64-bit integer, where SQLite's average is no longer exact"
  )]
  IntegerSumOverflow {
    column: &'static str,
    pid: i64,
    process_name: String,
  },
  #[error(
    "stored {kind} in {table}.{column} is not a value the SQLite reader decodes: {value:?}"
  )]
  UndecodableStoredValue {
    table: &'static str,
    column: &'static str,
    kind: &'static str,
    value: String,
  },
  #[error("native archive series request is not answerable: {source}")]
  ArchiveSeries {
    #[source]
    source: crate::infrastructure::database::archive_queries::ArchiveSeriesError,
  },
  #[error(
    "SQLite cannot read the rendered timestamp {timestamp:?} as an instant, so a row stamped with it would be unreachable by range query"
  )]
  UnstampableWrite { timestamp: String },
  #[error("native database finalization failed during {context}: {message}")]
  Finalization { context: String, message: String },
  #[error("failed to capture a new source snapshot for reconciliation: {message}")]
  SourceSnapshot { message: String },
  #[error(
    "a native database in state {state} cannot be {operation}; it must be {expected}"
  )]
  UnexpectedState {
    operation: &'static str,
    state: String,
    expected: &'static str,
  },
  #[error(
    "the native database does not hold the verified conversion the caller is selecting: {detail}"
  )]
  UnverifiedSelection { detail: String },
  #[error(
    "{path} needs {required_bytes} bytes for the conversion but reports {available_bytes} available"
  )]
  InsufficientWorkspace {
    path: PathBuf,
    required_bytes: u64,
    available_bytes: u64,
  },
  #[error("free space on the volume holding {path} could not be determined")]
  WorkspaceSpaceUnknown { path: PathBuf },
  #[error("failed to record the durable authority selection: {context}: {message}")]
  Selection {
    context: &'static str,
    message: String,
  },
}

impl NativeDatabaseError {
  /// DuckDB reports a fatal checkpoint failure (including an already
  /// invalidated instance) through the error message rather than a stable
  /// error code. Keep this deliberately narrow: ordinary constraint and SQL
  /// errors must not cause the dispatcher to replace a healthy owner.
  pub(crate) fn invalidates_database_instance(&self) -> bool {
    let Self::DuckDb { source, .. } = self else {
      return matches!(self, Self::Invalidated);
    };
    let message = source.to_string().to_ascii_lowercase();
    message.contains("database has been invalidated")
  }

  pub(crate) fn duckdb(context: &'static str, source: duckdb::Error) -> Self {
    Self::DuckDb { context, source }
  }

  pub(crate) fn finalization(
    context: impl Into<String>,
    message: impl std::fmt::Display,
  ) -> Self {
    Self::Finalization {
      context: context.into(),
      message: message.to_string(),
    }
  }

  pub(crate) fn selection(
    context: &'static str,
    message: impl std::fmt::Display,
  ) -> Self {
    Self::Selection {
      context,
      message: message.to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::NativeDatabaseError;

  fn duckdb_error(message: &str) -> NativeDatabaseError {
    NativeDatabaseError::duckdb(
      "test operation",
      duckdb::Error::DuckDBFailure(
        duckdb::ffi::Error::new(duckdb::ffi::DuckDBError),
        Some(message.to_owned()),
      ),
    )
  }

  #[test]
  fn only_duckdb_invalidated_errors_mark_the_instance() {
    assert!(
      duckdb_error(
        "IO Error: Checkpoint failed for database. The database has been invalidated."
      )
      .invalidates_database_instance()
    );
    assert!(
      duckdb_error(
        "FATAL Error: database has been invalidated because of a previous fatal error"
      )
      .invalidates_database_instance()
    );
    assert!(
      !duckdb_error("IO Error: Checkpoint failed without invalidating the database")
        .invalidates_database_instance()
    );
    assert!(
      !duckdb_error("Constraint Error: duplicate key violates primary key")
        .invalidates_database_instance()
    );
    assert!(
      !NativeDatabaseError::Worker {
        message: "checkpoint failed in a worker wrapper".to_owned(),
      }
      .invalidates_database_instance()
    );
  }
}
