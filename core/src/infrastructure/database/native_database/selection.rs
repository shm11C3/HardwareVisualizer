//! Durable authority selection, and what to conclude from the files left on
//! disk.
//!
//! Selecting a backend is the one step of the conversion that cannot be undone
//! by deleting a file: afterwards the native database, not SQLite, holds the
//! rows the application has written. It therefore has to survive a crash at any
//! point, and - more importantly - a crash must never leave a state where two
//! files both look authoritative.
//!
//! # The order, and the single repairable gap
//!
//! The record is written twice: once inside the native database's own metadata
//! (`state = 'selected'`, committed, checkpointed and synced) and once in a
//! small marker file beside it. The native database is written **first**. The
//! only state a crash can leave between the two writes is therefore "the
//! database says selected, the marker is missing", and that state is
//! repairable without guessing: the database that says `selected` is the one
//! that was chosen, and the marker is rewritten from it
//! ([`repair_authority_marker`]). The reverse order would leave a marker
//! claiming a selection the database never recorded, which is not repairable -
//! the marker alone cannot say whether the transaction committed.
//!
//! Every other disagreement between the two is reported rather than repaired.
//! Recovery code that guesses which of two files is authoritative is how
//! history gets lost silently, and the numbers needed to decide are in the
//! report.
//!
//! # What is deliberately not here
//!
//! Retiring the SQLite source once a later startup has verified the selection
//! is App lifecycle work: the source is renamed in place (decided 2026-09-13;
//! [`super::preflight`] budgets no second copy of it). Nothing in this module
//! removes or renames the source.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use duckdb::{AccessMode, Connection, params};
use serde::{Deserialize, Serialize};

use super::NativeDatabaseError;
use super::cell::quote_identifier;
use super::compatibility::{
  engine_storage_version, has_storage_version_column, require_storage_version_column,
  verify_storage_version,
};
use super::finalize::{
  FINALIZED_UNSELECTED, NATIVE_IDENTITY_TABLE, NATIVE_METADATA_TABLE, SELECTED,
  open_database, open_database_with_storage_version, require_no_wal,
};
use super::reconcile::NativeReconciliationReport;
use super::schema::{NativeIdentityMode, NativeSchemaDefinition};

/// The marker file name, resolved by the caller against the directory that
/// holds the databases.
pub const AUTHORITY_MARKER_FILE_NAME: &str = "hv-database.authority.json";

/// The prefix every conversion work directory shares, so leftover debris is
/// recognizable without knowing which step produced it.
const WORK_PREFIX: &str = ".hardwarevisualizer-duckdb-";
const FRESH_WORK_PREFIX: &str = ".hardwarevisualizer-duckdb-fresh-";

const MARKER_VERSION: u32 = 1;

/// Proof that a reconciliation caught this file up to its source and read it
/// back afterwards.
///
/// It has no public constructor: the only way to obtain one is
/// [`super::reconcile_native_database`], so "select whatever is lying there"
/// cannot be expressed, and neither can "select the file finalization just
/// produced" - a finalized file copies one snapshot taken while the
/// application kept writing, so it is stale by construction.
///
/// # Precondition the caller owns
///
/// Reconciliation makes the native database equal the source *at the moment it
/// captured its candidate*. Rows written to SQLite after that are not in the
/// file, and nothing here can see them. The App lifecycle owner must therefore
/// quiesce every SQLite writer before the final reconciliation and keep them
/// quiesced until [`select_native_database`] returns; selecting after a
/// reconciliation that ran against a live writer silently drops whatever was
/// written in between.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedNativeDatabase {
  path: PathBuf,
  schema_version: u32,
  source_schema_sha256: String,
  total_rows: u64,
}

impl VerifiedNativeDatabase {
  /// Only [`super::reconcile`] may mint proof, and only from a report it has
  /// just verified against the reopened file.
  pub(super) fn from_reconciliation(report: &NativeReconciliationReport) -> Self {
    Self {
      path: report.native_database_path.clone(),
      schema_version: report.schema_version,
      source_schema_sha256: report.source_schema_sha256.clone(),
      total_rows: report.total_rows,
    }
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  pub fn schema_version(&self) -> u32 {
    self.schema_version
  }

  pub fn source_schema_sha256(&self) -> &str {
    &self.source_schema_sha256
  }

  pub fn total_rows(&self) -> u64 {
    self.total_rows
  }
}

/// The on-disk marker. Deliberately small: every field is one the native
/// database records too, so the two can be compared rather than trusted.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityMarker {
  pub version: u32,
  /// The file name only. An absolute path would be wrong the moment the
  /// application data directory moves, and the directory is the caller's.
  pub native_database_file_name: String,
  pub schema_version: u32,
  pub source_schema_sha256: String,
  pub total_rows: u64,
}

/// The two databases, the marker beside them, and the directory conversion
/// debris would appear in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorityPaths {
  pub source_database: PathBuf,
  pub native_database: PathBuf,
  pub marker: PathBuf,
}

impl AuthorityPaths {
  /// The conventional layout: both databases and the marker in one directory.
  pub fn in_directory(
    directory: &Path,
    source_file_name: &str,
    native_file_name: &str,
  ) -> Self {
    Self {
      source_database: directory.join(source_file_name),
      native_database: directory.join(native_file_name),
      marker: directory.join(AUTHORITY_MARKER_FILE_NAME),
    }
  }
}

/// The two states a native database's metadata may record. Anything else is
/// treated as unreadable rather than mapped onto one of them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeState {
  FinalizedUnselected,
  Selected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarkerFacts {
  Absent,
  /// Present but unusable: unreadable bytes, invalid JSON, or a version this
  /// build does not know.
  Unreadable,
  Present(AuthorityMarker),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeMetadataFacts {
  /// No native database file at all.
  Absent,
  /// The file exists but its metadata could not be read as a finalized native
  /// database.
  Unreadable,
  Present {
    state: NativeState,
    schema_version: u32,
    storage_version: String,
    engine_storage_version: String,
    source_schema_sha256: String,
    /// The row count the file records, compared against the marker's so a
    /// database restored from a different backup than its marker is reported
    /// rather than trusted.
    source_rows: u64,
  },
  /// A finalized file from before storage-version metadata was introduced.
  Legacy {
    state: NativeState,
    schema_version: u32,
  },
}

/// Everything [`inspect_authority`] is allowed to look at, gathered by
/// [`observe_authority`]. Separating the two keeps the decision a pure function
/// that a test can drive through every state without a filesystem.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorityFacts {
  pub source_database_present: bool,
  pub native_database_present: bool,
  pub native_database_file_name: String,
  /// A `.wal` beside the native database: a file that was not checkpointed and
  /// closed cleanly.
  pub native_write_ahead_log_present: bool,
  /// A `.hardwarevisualizer-duckdb-*` directory: an interrupted conversion.
  pub work_directory_present: bool,
  pub marker: MarkerFacts,
  pub native_metadata: NativeMetadataFacts,
  pub expected_schema_version: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityInconsistency {
  MarkerUnreadable,
  MarkerWithoutNativeDatabase,
  MarkerNamesAnotherDatabase,
  NativeMetadataUnreadable,
  /// The marker claims a selection the database did not record.
  MarkerAheadOfNativeState,
  /// A selected database built for a schema version this build does not run -
  /// the downgrade case.
  SchemaVersionMismatch,
  /// A finalized file predating the storage-version metadata contract.
  StorageVersionMetadataMissing,
  StorageVersionMismatch,
  MarkerDisagreesWithNativeDatabase,
  /// The repairable gap: the selection committed, the marker did not land.
  SelectedWithoutMarker,
  /// Native files exist but the SQLite source they were built from is gone.
  SourceDatabaseMissing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityRecovery {
  /// Report the numbers and change nothing. Guessing here loses history.
  StopAndReport,
  /// Rewrite the marker from the native database's own committed metadata.
  RepairSelectionMarkerFromNativeMetadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityState {
  /// The ordinary state before native creation or conversion: SQLite is
  /// authoritative and no native artifact exists.
  SqliteAuthoritative,
  /// Debris from an interrupted conversion. `resumable` means a complete
  /// finalized file is present, so the conversion resumes at reconciliation;
  /// otherwise the debris is discarded and the copy restarts.
  ConversionInProgress {
    resumable: bool,
  },
  /// A verified native database exists, and SQLite is still authoritative.
  FinalizedUnselected,
  NativeSelected,
  Inconsistent {
    reason: AuthorityInconsistency,
    recovery: AuthorityRecovery,
  },
}

/// Record `native_database` as the authoritative backend.
///
/// Opens the database itself, so no [`super::NativeDatabase`] owner may be live
/// on the same file: DuckDB refuses a second instance, and on Windows the file
/// could not be synced afterwards either.
///
/// The database's own metadata is committed, checkpointed and synced before the
/// marker is written, so the only interruption window leaves the one state
/// [`repair_authority_marker`] can close.
pub async fn select_native_database(
  paths: AuthorityPaths,
  verified: VerifiedNativeDatabase,
) -> Result<AuthorityMarker, NativeDatabaseError> {
  tokio::task::spawn_blocking(move || select(&paths, &verified))
    .await
    .map_err(|error| NativeDatabaseError::Worker {
      message: error.to_string(),
    })?
}

fn select(
  paths: &AuthorityPaths,
  verified: &VerifiedNativeDatabase,
) -> Result<AuthorityMarker, NativeDatabaseError> {
  if paths.native_database != verified.path {
    return Err(NativeDatabaseError::UnverifiedSelection {
      detail: format!(
        "the verified conversion is {} but {} is being selected",
        verified.path.display(),
        paths.native_database.display()
      ),
    });
  }
  let spill = selection_spill()?;

  // Scoped so every DuckDB handle on the file is released before the file is
  // synced and the marker is written: Windows refuses to reopen, rename or
  // delete a file another instance still holds.
  {
    let connection = open_database_with_storage_version(
      &paths.native_database,
      AccessMode::ReadWrite,
      spill.path(),
    )?;
    require_storage_version_column(&connection, NATIVE_METADATA_TABLE)?;
    let (
      state,
      schema_version,
      storage_version,
      source_schema_sha256,
      source_rows,
      reconciled,
    ) = read_metadata_row(&connection)?;
    verify_storage_version(&connection, &storage_version)?;
    if state != FINALIZED_UNSELECTED {
      return Err(NativeDatabaseError::UnexpectedState {
        operation: "selected",
        state,
        expected: FINALIZED_UNSELECTED,
      });
    }
    // The file says for itself whether a reconciliation committed into it, so
    // a proof that happens to describe a look-alike file - one re-finalized
    // from the same unchanged source, say - still cannot select it.
    if !reconciled {
      return Err(NativeDatabaseError::UnverifiedSelection {
        detail: format!(
          "{} records no committed reconciliation, so it holds one snapshot \
           taken while the source was still being written",
          paths.native_database.display()
        ),
      });
    }
    let schema_version = u32::try_from(schema_version).unwrap_or(u32::MAX);
    let source_rows = u64::try_from(source_rows).unwrap_or(u64::MAX);
    if schema_version != verified.schema_version
      || source_schema_sha256 != verified.source_schema_sha256
      || source_rows != verified.total_rows
    {
      return Err(NativeDatabaseError::UnverifiedSelection {
        detail: format!(
          "the file records schema version {schema_version}, source schema \
           {source_schema_sha256} and {source_rows} rows; the verified \
           conversion recorded {}, {} and {}",
          verified.schema_version, verified.source_schema_sha256, verified.total_rows
        ),
      });
    }

    connection
      .execute_batch(&format!(
        "BEGIN TRANSACTION; UPDATE {} SET state = '{SELECTED}'; COMMIT",
        quote_identifier(NATIVE_METADATA_TABLE)
      ))
      .map_err(|error| {
        NativeDatabaseError::duckdb("record the native selection", error)
      })?;
    connection.execute_batch("CHECKPOINT").map_err(|error| {
      NativeDatabaseError::duckdb("checkpoint the selected database", error)
    })?;
  }
  require_no_wal(&paths.native_database)?;
  sync_file(&paths.native_database)?;

  let marker = AuthorityMarker {
    version: MARKER_VERSION,
    native_database_file_name: file_name_of(&paths.native_database)?,
    schema_version: verified.schema_version,
    source_schema_sha256: verified.source_schema_sha256.clone(),
    total_rows: verified.total_rows,
  };
  write_marker_atomically(&paths.marker, &marker)?;
  Ok(marker)
}

/// Create and durably select the native database for a profile that has no
/// database yet.
///
/// The operation is intentionally stricter than conversion: any source,
/// native file, marker, sidecar or conversion work directory makes it refuse
/// rather than overwrite an artifact whose meaning belongs to another
/// lifecycle. The database is built in a fresh-install work directory and is
/// published without replacing an existing path only after its stable schema,
/// selected metadata and identity high-water marks have committed. That means
/// an interruption before publication leaves only disposable work; an
/// interruption after publication but before the marker is the existing
/// `SelectedWithoutMarker` repair case.
pub async fn create_empty_native_database(
  paths: AuthorityPaths,
  schema: NativeSchemaDefinition,
) -> Result<AuthorityMarker, NativeDatabaseError> {
  tokio::task::spawn_blocking(move || create_empty(&paths, schema))
    .await
    .map_err(|error| NativeDatabaseError::Worker {
      message: error.to_string(),
    })?
}

fn create_empty(
  paths: &AuthorityPaths,
  schema: NativeSchemaDefinition,
) -> Result<AuthorityMarker, NativeDatabaseError> {
  refuse_existing_fresh_artifacts(paths)?;
  let parent = paths
    .native_database
    .parent()
    .filter(|path| !path.as_os_str().is_empty())
    .ok_or_else(|| {
      NativeDatabaseError::selection(
        "reserve fresh native database storage",
        "the native database path has no parent directory",
      )
    })?;
  if !parent.is_dir() {
    return Err(NativeDatabaseError::selection(
      "reserve fresh native database storage",
      format!("database parent does not exist: {}", parent.display()),
    ));
  }

  let work = tempfile::Builder::new()
    .prefix(FRESH_WORK_PREFIX)
    .tempdir_in(parent)
    .map_err(|error| {
      NativeDatabaseError::selection("reserve fresh native database storage", error)
    })?;
  let database_path = work.path().join("fresh.duckdb");
  let spill = work.path().join("spill");
  fs::create_dir(&spill).map_err(|error| {
    NativeDatabaseError::selection("create fresh native database spill storage", error)
  })?;

  {
    let connection =
      open_database_with_storage_version(&database_path, AccessMode::ReadWrite, &spill)?;
    let storage_version = engine_storage_version(&connection)?;
    connection
      .execute_batch("BEGIN TRANSACTION")
      .map_err(|error| {
        NativeDatabaseError::duckdb("begin fresh native database creation", error)
      })?;
    if let Err(error) = write_empty_schema(&connection, schema, &storage_version) {
      let _ = connection.execute_batch("ROLLBACK");
      return Err(error);
    }
    connection
      .execute_batch("COMMIT; CHECKPOINT")
      .map_err(|error| {
        NativeDatabaseError::duckdb("commit fresh native database creation", error)
      })?;
  }
  require_no_wal(&database_path)?;
  sync_file(&database_path)?;

  // `rename` replaces an existing destination on Unix. The initial artifact
  // check cannot by itself serialize two first launches, so publish through a
  // hard link whose destination creation fails atomically when another
  // process won the race.
  fs::hard_link(&database_path, &paths.native_database).map_err(|error| {
    NativeDatabaseError::selection(
      "publish the fresh native database",
      format!("{}: {error}", paths.native_database.display()),
    )
  })?;
  fs::remove_file(&database_path).map_err(|error| {
    NativeDatabaseError::selection(
      "clean up the published fresh native database work file",
      error,
    )
  })?;
  sync_directory(parent)?;

  let marker = AuthorityMarker {
    version: MARKER_VERSION,
    native_database_file_name: file_name_of(&paths.native_database)?,
    schema_version: schema.version,
    // A fresh profile has no SQLite source schema to hash. Empty is an
    // explicit "no source" value; consumers only compare it with the marker.
    source_schema_sha256: String::new(),
    total_rows: 0,
  };
  write_marker_atomically_noclobber(&paths.marker, &marker)?;
  Ok(marker)
}

fn write_empty_schema(
  connection: &Connection,
  schema: NativeSchemaDefinition,
  storage_version: &str,
) -> Result<(), NativeDatabaseError> {
  connection.execute_batch(schema.sql).map_err(|error| {
    NativeDatabaseError::duckdb("create the fresh native schema", error)
  })?;
  connection
    .execute_batch(&format!(
      "CREATE TABLE {} (state VARCHAR NOT NULL, schema_version BIGINT NOT NULL, \
       storage_version VARCHAR NOT NULL, source_candidate_path VARCHAR NOT NULL, \
       source_schema_sha256 VARCHAR NOT NULL, source_rows BIGINT NOT NULL, \
       reconciled BOOLEAN NOT NULL); \
       CREATE TABLE {} (table_name VARCHAR PRIMARY KEY, column_name VARCHAR NOT NULL, \
       mode VARCHAR NOT NULL, high_water BIGINT NOT NULL)",
      quote_identifier(NATIVE_METADATA_TABLE),
      quote_identifier(NATIVE_IDENTITY_TABLE)
    ))
    .map_err(|error| {
      NativeDatabaseError::duckdb("create fresh native metadata tables", error)
    })?;

  for identity in schema.identities {
    let mode = match identity.mode {
      NativeIdentityMode::RowId => "rowid",
      NativeIdentityMode::AutoIncrement { .. } => "autoincrement",
    };
    connection
      .execute(
        &format!(
          "INSERT INTO {} VALUES (?, ?, ?, 0)",
          quote_identifier(NATIVE_IDENTITY_TABLE)
        ),
        params![identity.table, identity.column, mode],
      )
      .map_err(|error| {
        NativeDatabaseError::duckdb("initialize fresh native identities", error)
      })?;
  }

  connection
    .execute(
      &format!(
        "INSERT INTO {} VALUES (?, ?, ?, '', '', 0, true)",
        quote_identifier(NATIVE_METADATA_TABLE)
      ),
      params![SELECTED, i64::from(schema.version), storage_version],
    )
    .map(|_| ())
    .map_err(|error| NativeDatabaseError::duckdb("record fresh native authority", error))
}

/// Remove only interrupted fresh-install work after the caller has established
/// that no source, native database or marker exists. Without one of those
/// authoritative artifacts, a work directory cannot contain user history that
/// is still recoverable; deleting it is the one safe discard in this lifecycle.
pub fn discard_interrupted_fresh_creation_work(
  paths: &AuthorityPaths,
) -> Result<bool, NativeDatabaseError> {
  if path_exists(&paths.source_database)
    || path_exists(&paths.native_database)
    || path_exists(&paths.marker)
    || path_exists(&sidecar_path(&paths.source_database, "-wal"))
    || path_exists(&sidecar_path(&paths.source_database, "-shm"))
    || path_exists(&sidecar_path(&paths.native_database, ".wal"))
  {
    return Ok(false);
  }
  let Some(parent) = paths.native_database.parent() else {
    return Ok(false);
  };
  let entries = fs::read_dir(parent).map_err(|error| {
    NativeDatabaseError::selection("inspect fresh native database work", error)
  })?;
  let mut discarded = false;
  for entry in entries {
    let entry = entry.map_err(|error| {
      NativeDatabaseError::selection("inspect fresh native database work", error)
    })?;
    if entry
      .file_name()
      .to_string_lossy()
      .starts_with(FRESH_WORK_PREFIX)
      && entry
        .file_type()
        .map_err(|error| {
          NativeDatabaseError::selection("inspect fresh native database work", error)
        })?
        .is_dir()
    {
      fs::remove_dir_all(entry.path()).map_err(|error| {
        NativeDatabaseError::selection(
          "discard interrupted fresh native database work",
          error,
        )
      })?;
      discarded = true;
    }
  }
  Ok(discarded)
}

fn refuse_existing_fresh_artifacts(
  paths: &AuthorityPaths,
) -> Result<(), NativeDatabaseError> {
  for path in [
    &paths.source_database,
    &paths.native_database,
    &paths.marker,
    &sidecar_path(&paths.source_database, "-wal"),
    &sidecar_path(&paths.source_database, "-shm"),
    &sidecar_path(&paths.native_database, ".wal"),
  ] {
    if path_exists(path) {
      return Err(NativeDatabaseError::FreshInstallArtifactExists { path: path.clone() });
    }
  }
  if work_directory_present(&paths.native_database) {
    return Err(NativeDatabaseError::FreshInstallArtifactExists {
      path: paths
        .native_database
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_owned(),
    });
  }
  Ok(())
}

fn path_exists(path: &Path) -> bool {
  fs::symlink_metadata(path).is_ok()
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
  let mut sidecar = path.as_os_str().to_os_string();
  sidecar.push(suffix);
  PathBuf::from(sidecar)
}

/// Close the one repairable gap: a database that committed `selected` while its
/// marker never landed.
///
/// Refuses anything else, including a database that is merely finalized: the
/// marker is a record of a decision, never the decision itself.
///
/// Opens the database, so it carries the same "no live owner" precondition as
/// [`observe_authority`].
pub fn repair_authority_marker(
  paths: &AuthorityPaths,
) -> Result<AuthorityMarker, NativeDatabaseError> {
  let spill = selection_spill()?;
  // Scoped so the database is closed before the marker is renamed over.
  let marker = {
    let connection =
      open_database(&paths.native_database, AccessMode::ReadOnly, spill.path())?;
    require_storage_version_column(&connection, NATIVE_METADATA_TABLE)?;
    let (state, schema_version, storage_version, source_schema_sha256, source_rows, _) =
      read_metadata_row(&connection)?;
    verify_storage_version(&connection, &storage_version)?;
    if state != SELECTED {
      return Err(NativeDatabaseError::UnexpectedState {
        operation: "repaired into a selection marker",
        state,
        expected: SELECTED,
      });
    }
    AuthorityMarker {
      version: MARKER_VERSION,
      native_database_file_name: file_name_of(&paths.native_database)?,
      schema_version: u32::try_from(schema_version).unwrap_or(u32::MAX),
      source_schema_sha256,
      total_rows: u64::try_from(source_rows).unwrap_or(u64::MAX),
    }
  };
  write_marker_atomically(&paths.marker, &marker)?;
  Ok(marker)
}

/// The metadata row, as `(state, schema_version, storage_version,
/// source_schema_sha256, source_rows, reconciled)`.
/// Selection reads and updates one metadata row, so its spill never holds
/// anything. It goes in the system temporary directory rather than beside the
/// databases: a directory named with the conversion work prefix, left behind by
/// an interrupted selection, would read as interrupted *conversion* debris to
/// [`inspect_authority`].
fn selection_spill() -> Result<tempfile::TempDir, NativeDatabaseError> {
  tempfile::Builder::new()
    .prefix(WORK_PREFIX)
    .tempdir()
    .map_err(|error| {
      NativeDatabaseError::selection("reserve the selection spill directory", error)
    })
}

fn read_metadata_row(
  connection: &Connection,
) -> Result<(String, i64, String, String, i64, bool), NativeDatabaseError> {
  connection
    .query_row(
      &format!(
        "SELECT state, schema_version, storage_version, source_schema_sha256, source_rows, \
         reconciled FROM {}",
        quote_identifier(NATIVE_METADATA_TABLE)
      ),
      [],
      |row| {
        Ok((
          row.get(0)?,
          row.get(1)?,
          row.get(2)?,
          row.get(3)?,
          row.get(4)?,
          row.get(5)?,
        ))
      },
    )
    .map_err(|error| {
      NativeDatabaseError::duckdb("read the native metadata to select", error)
    })
}

fn file_name_of(path: &Path) -> Result<String, NativeDatabaseError> {
  path
    .file_name()
    .map(|name| name.to_string_lossy().into_owned())
    .ok_or_else(|| {
      NativeDatabaseError::selection(
        "name the selected database",
        "the native database path has no file name",
      )
    })
}

/// Write to a temporary file, sync it, rename it over the marker, then sync the
/// directory: a reader sees either the old marker or the whole new one.
fn write_marker_atomically(
  marker_path: &Path,
  marker: &AuthorityMarker,
) -> Result<(), NativeDatabaseError> {
  write_marker_atomically_with(marker_path, marker, false)
}

fn write_marker_atomically_noclobber(
  marker_path: &Path,
  marker: &AuthorityMarker,
) -> Result<(), NativeDatabaseError> {
  write_marker_atomically_with(marker_path, marker, true)
}

fn write_marker_atomically_with(
  marker_path: &Path,
  marker: &AuthorityMarker,
  no_clobber: bool,
) -> Result<(), NativeDatabaseError> {
  let directory = marker_path
    .parent()
    .filter(|path| !path.as_os_str().is_empty())
    .ok_or_else(|| {
      NativeDatabaseError::selection(
        "resolve the selection marker directory",
        "the marker must have a parent directory",
      )
    })?;
  let encoded = serde_json::to_vec_pretty(marker).map_err(|error| {
    NativeDatabaseError::selection("encode the selection marker", error)
  })?;
  let mut temporary = tempfile::Builder::new()
    .prefix(".hv-database.authority.")
    .suffix(".json")
    .tempfile_in(directory)
    .map_err(|error| {
      NativeDatabaseError::selection("create the selection marker", error)
    })?;
  temporary.write_all(&encoded).map_err(|error| {
    NativeDatabaseError::selection("write the selection marker", error)
  })?;
  temporary.as_file().sync_all().map_err(|error| {
    NativeDatabaseError::selection("sync the selection marker", error)
  })?;
  if no_clobber {
    temporary.persist_noclobber(marker_path).map_err(|error| {
      NativeDatabaseError::selection("publish the selection marker", error.error)
    })?;
  } else {
    temporary.persist(marker_path).map_err(|error| {
      NativeDatabaseError::selection("publish the selection marker", error.error)
    })?;
  }
  sync_directory(directory)
}

fn sync_file(path: &Path) -> Result<(), NativeDatabaseError> {
  OpenOptions::new()
    .read(true)
    .write(true)
    .open(path)
    .and_then(|file| file.sync_all())
    .map_err(|error| NativeDatabaseError::selection("sync the selected database", error))
}

/// Renames are only durable once the directory entry itself is synced. Windows
/// has no directory handle to sync, and its rename is already ordered, so the
/// call is skipped rather than faked.
fn sync_directory(directory: &Path) -> Result<(), NativeDatabaseError> {
  match File::open(directory).and_then(|handle| handle.sync_all()) {
    Ok(()) => Ok(()),
    Err(_) if cfg!(windows) => Ok(()),
    Err(error) => Err(NativeDatabaseError::selection(
      "sync the selection marker directory",
      error,
    )),
  }
}

/// Read everything on disk that the authority decision depends on.
///
/// Never writes, and never fails: an unreadable marker or database is a fact
/// about the state, not an error, and [`inspect_authority`] has an arm for it.
///
/// # Call this before the backend is opened, never beside it
///
/// Reading the native metadata means opening the file as a second DuckDB
/// instance beside the owner. On Windows DuckDB refuses that; on Linux and
/// macOS its lock is process-scoped, so the read goes through against a file
/// another connection is writing. Neither is a sound answer. Called while a
/// [`super::NativeDatabase`] owner is live, this can therefore report the
/// metadata as *unreadable* - which [`inspect_authority`] turns into
/// `ConversionInProgress`, an alarming answer about a perfectly healthy
/// database. It belongs at startup, before any owner is opened, and after one
/// has been closed.
pub fn observe_authority(
  paths: &AuthorityPaths,
  expected_schema_version: u32,
) -> AuthorityFacts {
  let native_database_present = paths.native_database.is_file();
  let mut write_ahead_log = paths.native_database.as_os_str().to_os_string();
  write_ahead_log.push(".wal");

  AuthorityFacts {
    source_database_present: paths.source_database.is_file(),
    native_database_present,
    native_database_file_name: paths
      .native_database
      .file_name()
      .map(|name| name.to_string_lossy().into_owned())
      .unwrap_or_default(),
    native_write_ahead_log_present: PathBuf::from(write_ahead_log).is_file(),
    work_directory_present: work_directory_present(&paths.native_database),
    marker: observe_marker(&paths.marker),
    native_metadata: if native_database_present {
      observe_native_metadata(&paths.native_database)
    } else {
      NativeMetadataFacts::Absent
    },
    expected_schema_version,
  }
}

fn work_directory_present(native_database: &Path) -> bool {
  let Some(directory) = native_database.parent() else {
    return false;
  };
  let Ok(entries) = fs::read_dir(directory) else {
    return false;
  };
  entries.filter_map(Result::ok).any(|entry| {
    let name = entry.file_name();
    let name = name.to_string_lossy();
    name.starts_with(WORK_PREFIX)
      && !name.starts_with(super::LEGACY_RUNTIME_SPILL_DIRECTORY_PREFIX)
      && entry.file_type().is_ok_and(|kind| kind.is_dir())
  })
}

fn observe_marker(path: &Path) -> MarkerFacts {
  match fs::read(path) {
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => MarkerFacts::Absent,
    Err(_) => MarkerFacts::Unreadable,
    Ok(bytes) => match serde_json::from_slice::<AuthorityMarker>(&bytes) {
      Ok(marker) if marker.version == MARKER_VERSION => MarkerFacts::Present(marker),
      _ => MarkerFacts::Unreadable,
    },
  }
}

fn observe_native_metadata(path: &Path) -> NativeMetadataFacts {
  let Ok(spill) = selection_spill() else {
    return NativeMetadataFacts::Unreadable;
  };
  let Ok(connection) = open_database(path, AccessMode::ReadOnly, spill.path()) else {
    return NativeMetadataFacts::Unreadable;
  };
  let Ok(has_storage_version) =
    has_storage_version_column(&connection, NATIVE_METADATA_TABLE)
  else {
    return NativeMetadataFacts::Unreadable;
  };
  if !has_storage_version {
    let legacy_metadata: Result<(String, i64), _> = connection.query_row(
      &format!(
        "SELECT state, schema_version FROM {}",
        quote_identifier(NATIVE_METADATA_TABLE)
      ),
      [],
      |row| Ok((row.get(0)?, row.get(1)?)),
    );
    let Ok((state, schema_version)) = legacy_metadata else {
      return NativeMetadataFacts::Unreadable;
    };
    let state = match state.as_str() {
      FINALIZED_UNSELECTED => NativeState::FinalizedUnselected,
      SELECTED => NativeState::Selected,
      _ => return NativeMetadataFacts::Unreadable,
    };
    let Ok(schema_version) = u32::try_from(schema_version) else {
      return NativeMetadataFacts::Unreadable;
    };
    return NativeMetadataFacts::Legacy {
      state,
      schema_version,
    };
  }
  let Ok((state, schema_version, storage_version, source_schema_sha256, source_rows, _)) =
    read_metadata_row(&connection)
  else {
    return NativeMetadataFacts::Unreadable;
  };
  let Ok(engine_storage_version) = engine_storage_version(&connection) else {
    return NativeMetadataFacts::Unreadable;
  };
  let state = match state.as_str() {
    FINALIZED_UNSELECTED => NativeState::FinalizedUnselected,
    SELECTED => NativeState::Selected,
    _ => return NativeMetadataFacts::Unreadable,
  };
  let Ok(schema_version) = u32::try_from(schema_version) else {
    return NativeMetadataFacts::Unreadable;
  };
  let Ok(source_rows) = u64::try_from(source_rows) else {
    return NativeMetadataFacts::Unreadable;
  };
  NativeMetadataFacts::Present {
    state,
    schema_version,
    storage_version,
    engine_storage_version,
    source_schema_sha256,
    source_rows,
  }
}

/// Decide what the observed files mean. Pure: the same facts always give the
/// same answer, and no arm creates, empties or reverts a database.
pub fn inspect_authority(facts: &AuthorityFacts) -> AuthorityState {
  let stop = |reason| AuthorityState::Inconsistent {
    reason,
    recovery: AuthorityRecovery::StopAndReport,
  };

  if let NativeMetadataFacts::Present {
    storage_version,
    engine_storage_version,
    ..
  } = &facts.native_metadata
    && storage_version != engine_storage_version
  {
    return stop(AuthorityInconsistency::StorageVersionMismatch);
  }
  match &facts.marker {
    MarkerFacts::Unreadable => stop(AuthorityInconsistency::MarkerUnreadable),
    MarkerFacts::Present(marker) => {
      if !facts.native_database_present {
        return stop(AuthorityInconsistency::MarkerWithoutNativeDatabase);
      }
      if marker.native_database_file_name != facts.native_database_file_name {
        return stop(AuthorityInconsistency::MarkerNamesAnotherDatabase);
      }
      if let NativeMetadataFacts::Legacy {
        state,
        schema_version,
      } = &facts.native_metadata
      {
        if *state == NativeState::FinalizedUnselected {
          return stop(AuthorityInconsistency::MarkerAheadOfNativeState);
        }
        if *schema_version != facts.expected_schema_version {
          return stop(AuthorityInconsistency::SchemaVersionMismatch);
        }
        return stop(AuthorityInconsistency::StorageVersionMetadataMissing);
      }
      let NativeMetadataFacts::Present {
        state,
        schema_version,
        source_schema_sha256,
        source_rows,
        ..
      } = &facts.native_metadata
      else {
        return stop(AuthorityInconsistency::NativeMetadataUnreadable);
      };
      if *state == NativeState::FinalizedUnselected {
        return stop(AuthorityInconsistency::MarkerAheadOfNativeState);
      }
      if *schema_version != facts.expected_schema_version {
        return stop(AuthorityInconsistency::SchemaVersionMismatch);
      }
      // Every field the marker carries is one the database records too, so all
      // of them are compared. A restore that put back a database and a marker
      // from different backups agrees on the file name and the schema but not
      // on how many rows were selected.
      if marker.schema_version != *schema_version
        || &marker.source_schema_sha256 != source_schema_sha256
        || marker.total_rows != *source_rows
      {
        return stop(AuthorityInconsistency::MarkerDisagreesWithNativeDatabase);
      }
      AuthorityState::NativeSelected
    }
    MarkerFacts::Absent => {
      if matches!(
        facts.native_metadata,
        NativeMetadataFacts::Present {
          state: NativeState::Selected,
          ..
        }
      ) {
        return AuthorityState::Inconsistent {
          reason: AuthorityInconsistency::SelectedWithoutMarker,
          recovery: AuthorityRecovery::RepairSelectionMarkerFromNativeMetadata,
        };
      }
      if !facts.source_database_present
        && (facts.native_database_present || facts.work_directory_present)
      {
        return stop(AuthorityInconsistency::SourceDatabaseMissing);
      }
      match &facts.native_metadata {
        NativeMetadataFacts::Unreadable => {
          // A native file can be selected even when its marker is absent;
          // the marker is written after the database records selection. If
          // the database cannot be read, treating it as partial conversion
          // output sends retries into a conversion with an existing native file.
          stop(AuthorityInconsistency::NativeMetadataUnreadable)
        }
        NativeMetadataFacts::Present { .. } => {
          // Finalized and unselected. A write-ahead log or leftover work
          // directory means the conversion was interrupted after the file was
          // complete, so it resumes at reconciliation rather than recopying.
          if facts.native_write_ahead_log_present || facts.work_directory_present {
            AuthorityState::ConversionInProgress { resumable: true }
          } else {
            AuthorityState::FinalizedUnselected
          }
        }
        NativeMetadataFacts::Legacy { schema_version, .. } => {
          if *schema_version != facts.expected_schema_version {
            stop(AuthorityInconsistency::SchemaVersionMismatch)
          } else {
            stop(AuthorityInconsistency::StorageVersionMetadataMissing)
          }
        }
        NativeMetadataFacts::Absent => {
          if facts.work_directory_present {
            AuthorityState::ConversionInProgress { resumable: false }
          } else {
            AuthorityState::SqliteAuthoritative
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn marker() -> AuthorityMarker {
    AuthorityMarker {
      version: MARKER_VERSION,
      native_database_file_name: "hv-database.duckdb".to_owned(),
      schema_version: 1,
      source_schema_sha256: "abc".to_owned(),
      total_rows: 10,
    }
  }

  fn facts() -> AuthorityFacts {
    AuthorityFacts {
      source_database_present: true,
      native_database_present: true,
      native_database_file_name: "hv-database.duckdb".to_owned(),
      native_write_ahead_log_present: false,
      work_directory_present: false,
      marker: MarkerFacts::Present(marker()),
      native_metadata: NativeMetadataFacts::Present {
        state: NativeState::Selected,
        schema_version: 1,
        storage_version: "v1.0.0+".to_owned(),
        engine_storage_version: "v1.0.0+".to_owned(),
        source_schema_sha256: "abc".to_owned(),
        source_rows: 10,
      },
      expected_schema_version: 1,
    }
  }

  fn inconsistent(reason: AuthorityInconsistency) -> AuthorityState {
    AuthorityState::Inconsistent {
      reason,
      recovery: AuthorityRecovery::StopAndReport,
    }
  }

  #[test]
  fn a_marker_and_an_agreeing_selected_database_is_the_only_selected_state() {
    assert_eq!(inspect_authority(&facts()), AuthorityState::NativeSelected);
  }

  #[test]
  fn a_native_storage_version_disagreement_stops_authority_inspection() {
    let mut observed = facts();
    if let NativeMetadataFacts::Present {
      engine_storage_version,
      ..
    } = &mut observed.native_metadata
    {
      *engine_storage_version = "v1.2.0+".to_owned();
    }
    assert_eq!(
      inspect_authority(&observed),
      inconsistent(AuthorityInconsistency::StorageVersionMismatch)
    );
  }

  #[test]
  fn a_fresh_install_and_a_plain_sqlite_installation_stay_on_sqlite() {
    let mut observed = facts();
    observed.marker = MarkerFacts::Absent;
    observed.native_database_present = false;
    observed.native_metadata = NativeMetadataFacts::Absent;
    assert_eq!(
      inspect_authority(&observed),
      AuthorityState::SqliteAuthoritative
    );

    observed.source_database_present = false;
    assert_eq!(
      inspect_authority(&observed),
      AuthorityState::SqliteAuthoritative
    );
  }

  #[test]
  fn a_finalized_database_alone_leaves_sqlite_authoritative() {
    let mut observed = facts();
    observed.marker = MarkerFacts::Absent;
    observed.native_metadata = NativeMetadataFacts::Present {
      state: NativeState::FinalizedUnselected,
      schema_version: 1,
      storage_version: "v1.0.0+".to_owned(),
      engine_storage_version: "v1.0.0+".to_owned(),
      source_schema_sha256: "abc".to_owned(),
      source_rows: 10,
    };
    assert_eq!(
      inspect_authority(&observed),
      AuthorityState::FinalizedUnselected
    );

    // Debris beside a complete file means the conversion stopped after
    // finalization, so it resumes at reconciliation.
    observed.work_directory_present = true;
    assert_eq!(
      inspect_authority(&observed),
      AuthorityState::ConversionInProgress { resumable: true }
    );
    observed.work_directory_present = false;
    observed.native_write_ahead_log_present = true;
    assert_eq!(
      inspect_authority(&observed),
      AuthorityState::ConversionInProgress { resumable: true }
    );
  }

  #[test]
  fn an_unreadable_native_file_is_not_treated_as_interrupted_conversion() {
    let mut observed = facts();
    observed.marker = MarkerFacts::Absent;
    observed.native_metadata = NativeMetadataFacts::Unreadable;
    assert_eq!(
      inspect_authority(&observed),
      inconsistent(AuthorityInconsistency::NativeMetadataUnreadable)
    );

    // A bare work directory with no native database is still interrupted
    // conversion output and can safely restart from the SQLite source.
    observed.native_database_present = false;
    observed.native_metadata = NativeMetadataFacts::Absent;
    observed.work_directory_present = true;
    assert_eq!(
      inspect_authority(&observed),
      AuthorityState::ConversionInProgress { resumable: false }
    );
  }

  #[test]
  fn a_committed_selection_without_its_marker_is_the_one_repairable_state() {
    let mut observed = facts();
    observed.marker = MarkerFacts::Absent;
    assert_eq!(
      inspect_authority(&observed),
      AuthorityState::Inconsistent {
        reason: AuthorityInconsistency::SelectedWithoutMarker,
        recovery: AuthorityRecovery::RepairSelectionMarkerFromNativeMetadata,
      }
    );
  }

  #[test]
  fn every_other_disagreement_stops_rather_than_guessing() {
    let mut observed = facts();
    observed.marker = MarkerFacts::Unreadable;
    assert_eq!(
      inspect_authority(&observed),
      inconsistent(AuthorityInconsistency::MarkerUnreadable)
    );

    let mut observed = facts();
    observed.native_database_present = false;
    observed.native_metadata = NativeMetadataFacts::Absent;
    assert_eq!(
      inspect_authority(&observed),
      inconsistent(AuthorityInconsistency::MarkerWithoutNativeDatabase)
    );

    let mut observed = facts();
    observed.native_database_file_name = "other.duckdb".to_owned();
    assert_eq!(
      inspect_authority(&observed),
      inconsistent(AuthorityInconsistency::MarkerNamesAnotherDatabase)
    );

    let mut observed = facts();
    observed.native_metadata = NativeMetadataFacts::Unreadable;
    assert_eq!(
      inspect_authority(&observed),
      inconsistent(AuthorityInconsistency::NativeMetadataUnreadable)
    );

    let mut observed = facts();
    observed.native_metadata = NativeMetadataFacts::Present {
      state: NativeState::FinalizedUnselected,
      schema_version: 1,
      storage_version: "v1.0.0+".to_owned(),
      engine_storage_version: "v1.0.0+".to_owned(),
      source_schema_sha256: "abc".to_owned(),
      source_rows: 10,
    };
    assert_eq!(
      inspect_authority(&observed),
      inconsistent(AuthorityInconsistency::MarkerAheadOfNativeState)
    );

    // The downgrade case: a selected database this build cannot run.
    let mut observed = facts();
    observed.expected_schema_version = 2;
    assert_eq!(
      inspect_authority(&observed),
      inconsistent(AuthorityInconsistency::SchemaVersionMismatch)
    );

    let mut observed = facts();
    observed.native_metadata = NativeMetadataFacts::Present {
      state: NativeState::Selected,
      schema_version: 1,
      storage_version: "v1.0.0+".to_owned(),
      engine_storage_version: "v1.0.0+".to_owned(),
      source_schema_sha256: "different".to_owned(),
      source_rows: 10,
    };
    assert_eq!(
      inspect_authority(&observed),
      inconsistent(AuthorityInconsistency::MarkerDisagreesWithNativeDatabase)
    );

    // A restore that put back a database and a marker from different backups:
    // everything agrees except how many rows were selected.
    let mut observed = facts();
    observed.native_metadata = NativeMetadataFacts::Present {
      state: NativeState::Selected,
      schema_version: 1,
      storage_version: "v1.0.0+".to_owned(),
      engine_storage_version: "v1.0.0+".to_owned(),
      source_schema_sha256: "abc".to_owned(),
      source_rows: 11,
    };
    assert_eq!(
      inspect_authority(&observed),
      inconsistent(AuthorityInconsistency::MarkerDisagreesWithNativeDatabase)
    );

    let mut observed = facts();
    observed.marker = MarkerFacts::Absent;
    observed.source_database_present = false;
    observed.native_metadata = NativeMetadataFacts::Present {
      state: NativeState::FinalizedUnselected,
      schema_version: 1,
      storage_version: "v1.0.0+".to_owned(),
      engine_storage_version: "v1.0.0+".to_owned(),
      source_schema_sha256: "abc".to_owned(),
      source_rows: 10,
    };
    assert_eq!(
      inspect_authority(&observed),
      inconsistent(AuthorityInconsistency::SourceDatabaseMissing)
    );
  }

  #[test]
  fn the_marker_round_trips_and_an_unknown_version_is_unreadable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(AUTHORITY_MARKER_FILE_NAME);
    write_marker_atomically(&path, &marker()).unwrap();
    assert_eq!(observe_marker(&path), MarkerFacts::Present(marker()));

    std::fs::write(&path, br#"{"version":99}"#).unwrap();
    assert_eq!(observe_marker(&path), MarkerFacts::Unreadable);
    std::fs::write(&path, b"not json").unwrap();
    assert_eq!(observe_marker(&path), MarkerFacts::Unreadable);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(observe_marker(&path), MarkerFacts::Absent);
  }

  fn fresh_schema() -> NativeSchemaDefinition {
    static IDENTITIES:
      &[crate::infrastructure::database::native_database::NativeIdentity] = &[
      crate::infrastructure::database::native_database::NativeIdentity {
        table: "smoke",
        column: "id",
        mode: NativeIdentityMode::RowId,
      },
    ];
    NativeSchemaDefinition {
      version: 7,
      sql: "CREATE TABLE smoke (id BIGINT PRIMARY KEY)",
      tables: &["smoke"],
      timestamp_columns: &[],
      identities: IDENTITIES,
    }
  }

  fn fresh_paths(directory: &std::path::Path) -> AuthorityPaths {
    AuthorityPaths::in_directory(directory, "hv-database.db", "hv-database.duckdb")
  }

  #[tokio::test]
  async fn fresh_creation_records_selected_metadata_and_zero_identity_watermarks() {
    let directory = tempfile::tempdir().unwrap();
    let paths = fresh_paths(directory.path());
    let marker = create_empty_native_database(paths.clone(), fresh_schema())
      .await
      .unwrap();

    assert_eq!(marker.schema_version, 7);
    assert!(marker.source_schema_sha256.is_empty());
    assert_eq!(marker.total_rows, 0);
    assert!(paths.native_database.is_file());
    assert!(paths.marker.is_file());
    assert_eq!(
      inspect_authority(&observe_authority(&paths, 7)),
      AuthorityState::NativeSelected
    );

    let spill = selection_spill().unwrap();
    let connection =
      open_database(&paths.native_database, AccessMode::ReadOnly, spill.path()).unwrap();
    let (state, schema_version, _storage_version, source_hash, source_rows, reconciled) =
      read_metadata_row(&connection).unwrap();
    assert_eq!(state, SELECTED);
    assert_eq!(schema_version, 7);
    assert!(source_hash.is_empty());
    assert_eq!(source_rows, 0);
    assert!(reconciled);
    let high_water: i64 = connection
      .query_row(
        "SELECT high_water FROM __hv_native_identities WHERE table_name = ?",
        [&"smoke"],
        |row| row.get(0),
      )
      .unwrap();
    assert_eq!(high_water, 0);
  }

  async fn assert_fresh_creation_refuses(setup: impl FnOnce(&AuthorityPaths)) {
    let directory = tempfile::tempdir().unwrap();
    let paths = fresh_paths(directory.path());
    setup(&paths);
    let error = create_empty_native_database(paths, fresh_schema())
      .await
      .unwrap_err();
    assert!(matches!(
      error,
      NativeDatabaseError::FreshInstallArtifactExists { .. }
    ));
  }

  #[tokio::test]
  async fn fresh_creation_refuses_source_native_marker_and_work_artifacts() {
    assert_fresh_creation_refuses(|paths| {
      std::fs::write(&paths.source_database, b"sqlite artifact").unwrap();
    })
    .await;
    assert_fresh_creation_refuses(|paths| {
      std::fs::write(&paths.native_database, b"duckdb artifact").unwrap();
    })
    .await;
    assert_fresh_creation_refuses(|paths| {
      std::fs::write(&paths.marker, b"marker artifact").unwrap();
    })
    .await;
    assert_fresh_creation_refuses(|paths| {
      std::fs::create_dir(
        paths
          .native_database
          .parent()
          .unwrap()
          .join(format!("{WORK_PREFIX}interrupted")),
      )
      .unwrap();
    })
    .await;
  }

  #[test]
  fn interrupted_fresh_work_is_discarded_only_without_authoritative_artifacts() {
    let directory = tempfile::tempdir().unwrap();
    let paths = fresh_paths(directory.path());
    let work = directory
      .path()
      .join(format!("{FRESH_WORK_PREFIX}interrupted"));
    std::fs::create_dir(&work).unwrap();
    assert!(discard_interrupted_fresh_creation_work(&paths).unwrap());
    assert!(!work.exists());

    std::fs::create_dir(&work).unwrap();
    std::fs::write(&paths.source_database, b"sqlite artifact").unwrap();
    assert!(!discard_interrupted_fresh_creation_work(&paths).unwrap());
    assert!(work.exists());

    std::fs::remove_file(&paths.source_database).unwrap();
    assert!(discard_interrupted_fresh_creation_work(&paths).unwrap());
    assert!(!work.exists());
    let conversion_work = directory.path().join(format!("{WORK_PREFIX}conversion"));
    std::fs::create_dir(&conversion_work).unwrap();
    assert!(!discard_interrupted_fresh_creation_work(&paths).unwrap());
    assert!(conversion_work.exists());
  }

  #[tokio::test]
  async fn fresh_selection_without_marker_uses_the_existing_repair_path() {
    let directory = tempfile::tempdir().unwrap();
    let paths = fresh_paths(directory.path());
    create_empty_native_database(paths.clone(), fresh_schema())
      .await
      .unwrap();
    std::fs::remove_file(&paths.marker).unwrap();

    assert_eq!(
      inspect_authority(&observe_authority(&paths, 7)),
      AuthorityState::Inconsistent {
        reason: AuthorityInconsistency::SelectedWithoutMarker,
        recovery: AuthorityRecovery::RepairSelectionMarkerFromNativeMetadata,
      }
    );
    repair_authority_marker(&paths).unwrap();
    assert_eq!(
      inspect_authority(&observe_authority(&paths, 7)),
      AuthorityState::NativeSelected
    );
  }

  #[tokio::test]
  async fn legacy_spill_is_ignored_by_authority_and_new_stale_spills_are_removed_after_owner_opens()
  {
    let directory = tempfile::tempdir().unwrap();
    let paths = fresh_paths(directory.path());
    create_empty_native_database(paths.clone(), fresh_schema())
      .await
      .unwrap();
    std::fs::remove_file(&paths.marker).unwrap();

    let legacy_runtime_spill = directory.path().join(format!(
      "{}interrupted",
      super::super::LEGACY_RUNTIME_SPILL_DIRECTORY_PREFIX
    ));
    std::fs::create_dir(&legacy_runtime_spill).unwrap();
    assert!(
      !observe_authority(&paths, 7).work_directory_present,
      "legacy runtime spills must not be observed as conversion work"
    );

    let runtime_spill_parent = directory
      .path()
      .join(super::super::runtime::RUNTIME_SPILL_DIRECTORY_PREFIX)
      .join(paths.native_database.file_name().unwrap());
    std::fs::create_dir_all(&runtime_spill_parent).unwrap();
    let runtime_spill = runtime_spill_parent.join("spill-interrupted");
    std::fs::create_dir(&runtime_spill).unwrap();
    assert_eq!(
      inspect_authority(&observe_authority(&paths, 7)),
      AuthorityState::Inconsistent {
        reason: AuthorityInconsistency::SelectedWithoutMarker,
        recovery: AuthorityRecovery::RepairSelectionMarkerFromNativeMetadata,
      }
    );

    let conversion_work = directory.path().join(format!("{WORK_PREFIX}interrupted"));
    std::fs::create_dir(&conversion_work).unwrap();
    assert!(observe_authority(&paths, 7).work_directory_present);
    let owner = crate::infrastructure::database::native_database::NativeDatabase::open(
      &paths.native_database,
      crate::infrastructure::database::native_database::NativeDatabaseOptions::new(7),
    )
    .await
    .unwrap();
    assert!(!runtime_spill.exists());
    assert!(legacy_runtime_spill.is_dir());
    assert!(conversion_work.is_dir());
    owner.close().await.unwrap();
  }

  #[tokio::test]
  async fn a_checkpointed_native_file_can_be_renamed_while_its_writer_lock_is_held() {
    let directory = tempfile::tempdir().unwrap();
    let paths = fresh_paths(directory.path());
    create_empty_native_database(paths.clone(), fresh_schema())
      .await
      .unwrap();

    let spill = directory.path().join("rename-spill");
    std::fs::create_dir(&spill).unwrap();
    let connection =
      open_database(&paths.native_database, AccessMode::ReadWrite, &spill).unwrap();
    connection
      .execute_batch(
        "CREATE TABLE recovery_rename_probe(value INTEGER); \
         INSERT INTO recovery_rename_probe VALUES (1)",
      )
      .unwrap();

    let mut wal_path = paths.native_database.as_os_str().to_os_string();
    wal_path.push(".wal");
    let wal_path = PathBuf::from(wal_path);
    assert!(
      wal_path.is_file(),
      "the write should leave a checkpointable WAL"
    );
    connection.execute_batch("CHECKPOINT").unwrap();
    assert!(
      !wal_path.exists(),
      "CHECKPOINT must flush and remove the WAL"
    );

    let recovery_directory = directory.path().join("recovery");
    std::fs::create_dir(&recovery_directory).unwrap();
    let recovered_database = recovery_directory.join("hv-database.duckdb");
    std::fs::rename(&paths.native_database, &recovered_database)
      .expect("DuckDB's writer lock must allow same-volume rename while held");
    let row_count: i64 = connection
      .query_row("SELECT count(*) FROM recovery_rename_probe", [], |row| {
        row.get(0)
      })
      .unwrap();
    assert_eq!(row_count, 1);
    drop(connection);

    assert!(!paths.native_database.exists());
    assert!(recovered_database.is_file());
    assert!(!wal_path.exists());
    let recovered_spill = directory.path().join("recovered-spill");
    std::fs::create_dir(&recovered_spill).unwrap();
    let recovered =
      open_database(&recovered_database, AccessMode::ReadOnly, &recovered_spill).unwrap();
    let recovered_count: i64 = recovered
      .query_row("SELECT count(*) FROM recovery_rename_probe", [], |row| {
        row.get(0)
      })
      .unwrap();
    assert_eq!(recovered_count, 1);
  }
}
