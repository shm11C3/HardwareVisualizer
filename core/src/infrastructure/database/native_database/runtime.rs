//! The dedicated blocking owner of one finalized native database.
//!
//! duckdb-rs is synchronous, so connections are never handed out. Two owned
//! threads - one read lane, one write lane over `try_clone`d connections to the
//! same instance - execute closures sent through bounded channels, so a slow
//! reader can neither block a commit nor let callers queue unbounded work.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use duckdb::{AccessMode, Connection, OptionalExt, Transaction, params};
use tempfile::TempDir;
use tokio::sync::{Mutex as AsyncMutex, RwLock, mpsc, oneshot};

use super::NativeDatabaseError;
use super::cell::quote_identifier;
use super::compatibility::{
  native_config, require_storage_version_column, verify_storage_version,
};
use super::finalize::{
  FINALIZED_UNSELECTED, NATIVE_IDENTITY_TABLE, NATIVE_METADATA_TABLE, SELECTED,
};

/// The runtime owner's spill directory must not look like conversion work to
/// authority inspection, which recognizes `.hardwarevisualizer-duckdb-*`.
pub(super) const RUNTIME_SPILL_DIRECTORY_PREFIX: &str =
  ".hardwarevisualizer-runtime-spill";

const RUNTIME_SPILL_ENTRY_PREFIX: &str = "spill-";

#[derive(Clone, Copy, Debug)]
pub struct NativeDatabaseOptions {
  pub expected_schema_version: u32,
  /// How many requests may wait per lane before a caller is made to wait.
  pub request_capacity: usize,
}

impl NativeDatabaseOptions {
  pub fn new(expected_schema_version: u32) -> Self {
    Self {
      expected_schema_version,
      request_capacity: 32,
    }
  }
}

/// A one-shot cancellation token for exactly one request.
///
/// Claimed when it is attached to a request so two requests can never share an
/// interrupt handle, which would let one caller's cancellation abort another's
/// statement.
#[derive(Clone, Debug)]
pub struct NativeCancellation {
  inner: Arc<CancellationState>,
}

struct CancellationState {
  claimed: AtomicBool,
  cancelled: AtomicBool,
  interrupt: Mutex<Option<Arc<duckdb::InterruptHandle>>>,
}

impl std::fmt::Debug for CancellationState {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter
      .debug_struct("CancellationState")
      .field("claimed", &self.claimed)
      .field("cancelled", &self.cancelled)
      .finish_non_exhaustive()
  }
}

impl NativeCancellation {
  pub fn new() -> Self {
    Self {
      inner: Arc::new(CancellationState {
        claimed: AtomicBool::new(false),
        cancelled: AtomicBool::new(false),
        interrupt: Mutex::new(None),
      }),
    }
  }

  pub fn cancel(&self) {
    self.inner.cancelled.store(true, Ordering::Release);
    let interrupt = self
      .inner
      .interrupt
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(interrupt) = interrupt.as_ref() {
      interrupt.interrupt();
    }
  }

  pub fn is_cancelled(&self) -> bool {
    self.inner.cancelled.load(Ordering::Acquire)
  }

  fn claim(&self) -> Result<(), NativeDatabaseError> {
    self
      .inner
      .claimed
      .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
      .map(|_| ())
      .map_err(|_| NativeDatabaseError::CancellationAlreadyUsed)
  }
}

impl Default for NativeCancellation {
  fn default() -> Self {
    Self::new()
  }
}

pub struct NativeConnectionContext<'a> {
  connection: &'a mut Connection,
  cancellation: NativeCancellation,
  healthy: bool,
}

impl NativeConnectionContext<'_> {
  pub fn connection(&mut self) -> &mut Connection {
    self.connection
  }

  pub fn check_cancelled(&self) -> Result<(), NativeDatabaseError> {
    if self.cancellation.is_cancelled() {
      Err(NativeDatabaseError::Cancelled)
    } else {
      Ok(())
    }
  }

  /// Run `operation` inside one DuckDB transaction, committing only if it
  /// succeeded and was not cancelled.
  ///
  /// A rollback that itself fails marks the lane unhealthy and stops it: a
  /// connection whose transaction state is unknown must not serve the next
  /// request.
  pub fn with_transaction<T, F>(&mut self, operation: F) -> Result<T, NativeDatabaseError>
  where
    F: FnOnce(&NativeTransactionContext<'_, '_>) -> Result<T, NativeDatabaseError>,
  {
    self.check_cancelled()?;
    let transaction = self
      .connection
      .transaction()
      .map_err(|error| NativeDatabaseError::duckdb("begin transaction", error))?;
    let context = NativeTransactionContext {
      transaction: &transaction,
      cancellation: &self.cancellation,
    };
    let result = operation(&context);
    let cancelled = self.cancellation.is_cancelled();
    match result {
      Ok(value) if !cancelled => transaction
        .commit()
        .map(|_| value)
        .map_err(|error| NativeDatabaseError::duckdb("commit transaction", error)),
      result => {
        if let Err(rollback_error) = transaction.rollback() {
          self.healthy = false;
          return Err(NativeDatabaseError::Worker {
            message: format!("failed to roll back native transaction: {rollback_error}"),
          });
        }
        if cancelled {
          Err(NativeDatabaseError::Cancelled)
        } else {
          result
        }
      }
    }
  }
}

pub struct NativeTransactionContext<'transaction, 'connection> {
  transaction: &'transaction Transaction<'connection>,
  cancellation: &'transaction NativeCancellation,
}

impl NativeTransactionContext<'_, '_> {
  pub fn connection(&self) -> &Connection {
    self.transaction
  }

  pub fn check_cancelled(&self) -> Result<(), NativeDatabaseError> {
    if self.cancellation.is_cancelled() {
      Err(NativeDatabaseError::Cancelled)
    } else {
      Ok(())
    }
  }

  /// Allocate the next `id` for `table`.
  ///
  /// **Identity contract.** These ids are record identities, not domain
  /// identities: a Process row is identified by its recorded `(pid,
  /// process_name)` tuple, a Storage Health record by its producer-supplied
  /// `storage:hmac-sha256:v1:...` device key, and an archived GPU row by its
  /// opaque `gpu_id`. Nothing joins on the values produced here, so the only
  /// contract they owe is the one SQLite gave them, which finalization recorded
  /// per table in `__hv_native_identities`:
  ///
  /// - `autoincrement` reproduces `INTEGER PRIMARY KEY AUTOINCREMENT`. The next
  ///   id is the stored high-water mark plus one, and the mark advances in the
  ///   same transaction as the insert. Deleting the highest row therefore does
  ///   **not** release its id, and a rolled-back insert releases it exactly as
  ///   SQLite's `sqlite_sequence` update would.
  /// - `rowid` reproduces plain `INTEGER PRIMARY KEY`. The next id is the
  ///   current maximum plus one, so deleting the highest row does release its
  ///   id - SQLite's behavior, deliberately preserved rather than "fixed",
  ///   because an archive migrated to a stricter rule would start handing out
  ///   ids the source would not have.
  ///
  /// Both modes read and write inside the caller's transaction, so two
  /// concurrent writers cannot be handed the same id: the write lane is the
  /// only lane that commits.
  pub fn next_id(&self, table: &str) -> Result<i64, NativeDatabaseError> {
    self.check_cancelled()?;
    let metadata: Option<(String, String, i64)> = self
      .transaction
      .query_row(
        &format!(
          "SELECT column_name, mode, high_water FROM {NATIVE_IDENTITY_TABLE} WHERE table_name = ?"
        ),
        [table],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
      )
      .optional()
      .map_err(|error| {
        NativeDatabaseError::duckdb("read native identity metadata", error)
      })?;
    let Some((column, mode, high_water)) = metadata else {
      return Err(NativeDatabaseError::finalization(
        "allocate a native id",
        format!("table {table} has no native identity definition"),
      ));
    };
    let next = match mode.as_str() {
      "rowid" => {
        let sql = format!(
          "SELECT COALESCE(MAX({}), 0) FROM {}",
          quote_identifier(&column),
          quote_identifier(table)
        );
        let maximum: i64 = self
          .transaction
          .query_row(&sql, [], |row| row.get(0))
          .map_err(|error| {
            NativeDatabaseError::duckdb("read native rowid maximum", error)
          })?;
        maximum.checked_add(1)
      }
      "autoincrement" => high_water.checked_add(1),
      other => {
        return Err(NativeDatabaseError::finalization(
          "allocate a native id",
          format!("table {table} has unsupported identity mode {other}"),
        ));
      }
    }
    .ok_or_else(|| {
      NativeDatabaseError::finalization(
        "allocate a native id",
        format!("native identity for {table} exhausted signed 64-bit ids"),
      )
    })?;
    if mode == "autoincrement" {
      self
        .transaction
        .execute(
          &format!(
            "UPDATE {NATIVE_IDENTITY_TABLE} SET high_water = ? WHERE table_name = ?"
          ),
          params![next, table],
        )
        .map_err(|error| NativeDatabaseError::duckdb("advance native identity", error))?;
    }
    Ok(next)
  }
}

type Operation = Box<dyn FnOnce(&mut NativeConnectionContext<'_>) + Send + 'static>;

enum LaneMessage {
  Run {
    cancellation: NativeCancellation,
    operation: Operation,
  },
  Shutdown,
}

struct NativeDatabaseInner {
  read_sender: mpsc::Sender<LaneMessage>,
  write_sender: mpsc::Sender<LaneMessage>,
  lifecycle: RwLock<()>,
  close_lock: AsyncMutex<()>,
  closed: AtomicBool,
  joins: AsyncMutex<Option<(JoinHandle<()>, JoinHandle<()>)>>,
}

#[derive(Clone)]
pub struct NativeDatabase {
  inner: Arc<NativeDatabaseInner>,
}

impl std::fmt::Debug for NativeDatabase {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter
      .debug_struct("NativeDatabase")
      .field("closed", &self.inner.closed.load(Ordering::Acquire))
      .finish_non_exhaustive()
  }
}

impl NativeDatabase {
  /// Open a finalized, unselected native database.
  ///
  /// Refuses a file that finalization never produced, and one whose recorded
  /// schema version is not the version the caller was built against.
  ///
  /// Also refuses, with [`NativeDatabaseError::AlreadyOpen`], a file another
  /// `NativeDatabase` in this process still holds. DuckDB's own lock cannot be
  /// relied on for that: on Linux and macOS it is an `fcntl` lock, which POSIX
  /// scopes to the process, so a second open from the same process succeeds and
  /// two owners would write the same file. The claim is released only after
  /// both lanes have dropped their connections, so a successful
  /// [`NativeDatabase::close`] makes the file openable again.
  pub async fn open(
    path: impl AsRef<Path>,
    options: NativeDatabaseOptions,
  ) -> Result<Self, NativeDatabaseError> {
    if options.request_capacity == 0 {
      return Err(NativeDatabaseError::InvalidRequestCapacity);
    }
    let path = path.as_ref().to_owned();
    let metadata = std::fs::metadata(&path)
      .map_err(|_| NativeDatabaseError::Unavailable { path: path.clone() })?;
    if !metadata.is_file() {
      return Err(NativeDatabaseError::Unavailable { path });
    }
    let expected_version = options.expected_schema_version;
    // The claim is taken and carried inside the blocking task. If this future
    // is dropped while the task runs, Tokio lets the task finish anyway; a
    // claim held out here would be released while those connections were
    // still opening, and a second owner could slip in beside them.
    let (writer, reader, spill, claim) = tokio::task::spawn_blocking(move || {
      let claim = OpenClaim::acquire(&path)?;
      let (writer, reader, spill) = open_connections(&path, expected_version)?;
      Ok::<_, NativeDatabaseError>((writer, reader, spill, claim))
    })
    .await
    .map_err(|error| NativeDatabaseError::Worker {
      message: error.to_string(),
    })??;
    let spill = Arc::new(LaneResources {
      _spill: spill,
      _claim: claim,
    });
    let (read_sender, read_receiver) = mpsc::channel(options.request_capacity);
    let (write_sender, write_receiver) = mpsc::channel(options.request_capacity);
    let read_spill = Arc::clone(&spill);
    let read_join = std::thread::Builder::new()
      .name("hardviz-duckdb-read".to_owned())
      .spawn(move || run_lane(read_spill, reader, read_receiver))
      .map_err(|error| NativeDatabaseError::Worker {
        message: format!("failed to start native read owner: {error}"),
      })?;
    let write_join = std::thread::Builder::new()
      .name("hardviz-duckdb-write".to_owned())
      .spawn(move || run_lane(spill, writer, write_receiver))
      .map_err(|error| NativeDatabaseError::Worker {
        message: format!("failed to start native write owner: {error}"),
      })?;
    Ok(Self {
      inner: Arc::new(NativeDatabaseInner {
        read_sender,
        write_sender,
        lifecycle: RwLock::new(()),
        close_lock: AsyncMutex::new(()),
        closed: AtomicBool::new(false),
        joins: AsyncMutex::new(Some((read_join, write_join))),
      }),
    })
  }

  /// Stop both lanes and wait for them. Idempotent; every later request fails
  /// with [`NativeDatabaseError::Closed`].
  pub async fn close(&self) -> Result<(), NativeDatabaseError> {
    let _close = self.inner.close_lock.lock().await;
    let mut joins = self.inner.joins.lock().await;
    let Some((read_join, write_join)) = joins.take() else {
      return Ok(());
    };
    {
      let _lifecycle = self.inner.lifecycle.write().await;
      self.inner.closed.store(true, Ordering::Release);
      let _ = self.inner.read_sender.send(LaneMessage::Shutdown).await;
      let _ = self.inner.write_sender.send(LaneMessage::Shutdown).await;
    }
    tokio::task::spawn_blocking(move || {
      read_join.join().map_err(|_| NativeDatabaseError::Worker {
        message: "native read owner panicked".to_owned(),
      })?;
      write_join.join().map_err(|_| NativeDatabaseError::Worker {
        message: "native write owner panicked".to_owned(),
      })?;
      Ok(())
    })
    .await
    .map_err(|error| NativeDatabaseError::Worker {
      message: error.to_string(),
    })?
  }

  pub async fn request_read<T, F>(
    &self,
    cancellation: NativeCancellation,
    operation: F,
  ) -> Result<T, NativeDatabaseError>
  where
    T: Send + 'static,
    F: FnOnce(&mut NativeConnectionContext<'_>) -> Result<T, NativeDatabaseError>
      + Send
      + 'static,
  {
    self
      .request_on_lane(&self.inner.read_sender, cancellation, operation)
      .await
  }

  pub async fn request_write<T, F>(
    &self,
    cancellation: NativeCancellation,
    operation: F,
  ) -> Result<T, NativeDatabaseError>
  where
    T: Send + 'static,
    F: FnOnce(&mut NativeConnectionContext<'_>) -> Result<T, NativeDatabaseError>
      + Send
      + 'static,
  {
    self
      .request_on_lane(&self.inner.write_sender, cancellation, operation)
      .await
  }

  /// Issue one explicit `CHECKPOINT` on the write lane.
  ///
  /// Measured after a daily expiry pass on the implemented backend
  /// (`docs/development/hardware-archive-duckdb-retention-evidence.md`):
  /// 111-220 ms, keeping the WAL under 7 MiB and the engine's own reported
  /// memory at 10-20 MiB afterward. Left to the engine's own threshold
  /// checkpoint instead, the same measurement found 177-677 ms landing
  /// inside a random write cycle and 30-40 MiB more resident memory held
  /// between checkpoints. The App lifecycle owner (#2135) is expected to
  /// call this once after each daily expiry pass on the native backend;
  /// nothing here schedules it.
  pub async fn checkpoint(
    &self,
    cancellation: NativeCancellation,
  ) -> Result<(), NativeDatabaseError> {
    self
      .request_write(cancellation, |context| {
        context
          .connection()
          .execute_batch("CHECKPOINT")
          .map_err(|error| {
            NativeDatabaseError::duckdb("checkpoint the native database", error)
          })
      })
      .await
  }

  async fn request_on_lane<T, F>(
    &self,
    sender: &mpsc::Sender<LaneMessage>,
    cancellation: NativeCancellation,
    operation: F,
  ) -> Result<T, NativeDatabaseError>
  where
    T: Send + 'static,
    F: FnOnce(&mut NativeConnectionContext<'_>) -> Result<T, NativeDatabaseError>
      + Send
      + 'static,
  {
    cancellation.claim()?;
    let (result_sender, result_receiver) = oneshot::channel();
    let request_cancellation = cancellation.clone();
    let operation = Box::new(move |context: &mut NativeConnectionContext<'_>| {
      let result = context.check_cancelled().and_then(|_| operation(context));
      // A DuckDB interrupt surfaces as an ordinary statement error; the token
      // is what says the error was asked for.
      let result = if request_cancellation.is_cancelled() && result.is_err() {
        Err(NativeDatabaseError::Cancelled)
      } else {
        result
      };
      let _ = result_sender.send(result);
    });
    {
      let _lifecycle = self.inner.lifecycle.read().await;
      if self.inner.closed.load(Ordering::Acquire) {
        return Err(NativeDatabaseError::Closed);
      }
      sender
        .send(LaneMessage::Run {
          cancellation,
          operation,
        })
        .await
        .map_err(|_| NativeDatabaseError::Closed)?;
    }
    result_receiver
      .await
      .map_err(|_| NativeDatabaseError::Worker {
        message: "native database owner ended before returning a request".to_owned(),
      })?
  }
}

fn open_connections(
  path: &Path,
  expected_version: u32,
) -> Result<(Connection, Connection, TempDir), NativeDatabaseError> {
  let parent = path
    .parent()
    .filter(|parent| !parent.as_os_str().is_empty())
    .ok_or_else(|| NativeDatabaseError::Unavailable {
      path: path.to_owned(),
    })?;
  let spill_parent = runtime_spill_parent(
    parent,
    path
      .file_name()
      .ok_or_else(|| NativeDatabaseError::Unavailable {
        path: path.to_owned(),
      })?,
  )?;
  let spill = tempfile::Builder::new()
    .prefix(RUNTIME_SPILL_ENTRY_PREFIX)
    .tempdir_in(&spill_parent)
    .map_err(|error| NativeDatabaseError::Worker {
      message: format!("failed to create native spill directory: {error}"),
    })?;
  let config = native_config(AccessMode::ReadWrite, true)?;
  let writer = Connection::open_with_flags(path, config)
    .map_err(|error| NativeDatabaseError::duckdb("open native database", error))?;
  // DuckDB's writer lock now proves no other process owns this exact database.
  // The database-specific scope prevents cleanup from reaching another file's spill.
  discard_stale_runtime_spill_directories(&spill_parent, spill.path());
  super::configure_spill(&writer, spill.path())?;
  validate_native_metadata(&writer, expected_version)?;
  let reader = writer
    .try_clone()
    .map_err(|error| NativeDatabaseError::duckdb("open native read connection", error))?;
  Ok((writer, reader, spill))
}

/// Prepare a spill scope scoped to the exact database file.
///
/// The database filename remains an OS path component, so this also works for
/// filenames which cannot be represented as UTF-8. Existing reparse points and
/// symlinks are refused before any nested directory is created.
fn runtime_spill_parent(
  parent: &Path,
  database_file_name: &std::ffi::OsStr,
) -> Result<PathBuf, NativeDatabaseError> {
  let canonical_parent =
    fs::canonicalize(parent).map_err(|error| NativeDatabaseError::Worker {
      message: format!("failed to resolve native database parent: {error}"),
    })?;
  let spill_root = canonical_parent.join(RUNTIME_SPILL_DIRECTORY_PREFIX);
  ensure_normal_directory(&spill_root)?;
  ensure_directory_is_contained(&spill_root, &canonical_parent).map_err(|error| {
    NativeDatabaseError::Worker {
      message: format!("native spill root is outside its verified parent: {error}"),
    }
  })?;
  let canonical_spill_root =
    fs::canonicalize(&spill_root).map_err(|error| NativeDatabaseError::Worker {
      message: format!("failed to resolve native spill root: {error}"),
    })?;
  if canonical_spill_root.parent() != Some(canonical_parent.as_path()) {
    return Err(NativeDatabaseError::Worker {
      message: "native spill root resolved outside the database parent".to_owned(),
    });
  }

  let spill_parent = spill_root.join(database_file_name);
  ensure_normal_directory(&spill_parent)?;
  ensure_directory_is_contained(&spill_parent, &canonical_spill_root).map_err(
    |error| NativeDatabaseError::Worker {
      message: format!(
        "native database spill scope is outside its verified root: {error}"
      ),
    },
  )?;
  let canonical_spill_parent =
    fs::canonicalize(&spill_parent).map_err(|error| NativeDatabaseError::Worker {
      message: format!("failed to resolve native database spill scope: {error}"),
    })?;
  if canonical_spill_parent.parent() != Some(canonical_spill_root.as_path()) {
    return Err(NativeDatabaseError::Worker {
      message: "native database spill scope resolved outside its spill root".to_owned(),
    });
  }
  Ok(spill_parent)
}

fn ensure_normal_directory(path: &Path) -> Result<(), NativeDatabaseError> {
  match fs::symlink_metadata(path) {
    Ok(metadata) if is_normal_directory(&metadata) => Ok(()),
    Ok(_) => Err(NativeDatabaseError::Worker {
      message: format!(
        "native spill path is not a normal directory: {}",
        path.display()
      ),
    }),
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
      match fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
          let metadata = fs::symlink_metadata(path).map_err(|inspect_error| {
            NativeDatabaseError::Worker {
              message: format!(
                "failed to inspect existing native spill path {}: {inspect_error}",
                path.display()
              ),
            }
          })?;
          if is_normal_directory(&metadata) {
            Ok(())
          } else {
            Err(NativeDatabaseError::Worker {
              message: format!(
                "native spill path is not a normal directory: {}",
                path.display()
              ),
            })
          }
        }
        Err(error) => Err(NativeDatabaseError::Worker {
          message: format!(
            "failed to create native spill path {}: {error}",
            path.display()
          ),
        }),
      }
    }
    Err(error) => Err(NativeDatabaseError::Worker {
      message: format!(
        "failed to inspect native spill path {}: {error}",
        path.display()
      ),
    }),
  }
}

fn is_normal_directory(metadata: &fs::Metadata) -> bool {
  metadata.is_dir() && !metadata.file_type().is_symlink() && !is_reparse_point(metadata)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
  use std::os::windows::fs::MetadataExt;

  const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
  metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
  false
}

/// Remove spill directories left by a runtime owner that did not close cleanly.
///
/// This runs only after DuckDB opens the target database and acquires its
/// cross-process writer lock. It only considers direct spill children in that
/// database's verified scope and never follows symlinks or Windows reparse points.
fn discard_stale_runtime_spill_directories(spill_parent: &Path, active_spill: &Path) {
  let Some(spill_root) = spill_parent.parent() else {
    return;
  };
  let canonical_root = match fs::canonicalize(spill_root) {
    Ok(path) => path,
    Err(error) => {
      crate::log_warn!(
        "failed to resolve native runtime spill root",
        "native_database::discard_stale_runtime_spill_directories",
        Some(format!("{}: {error}", spill_root.display()))
      );
      return;
    }
  };
  if let Err(error) = ensure_directory_is_contained(spill_parent, &canonical_root) {
    crate::log_warn!(
      "native runtime spill directory is outside its verified scope",
      "native_database::discard_stale_runtime_spill_directories",
      Some(error)
    );
    return;
  }
  let canonical_scope = match fs::canonicalize(spill_parent) {
    Ok(path) => path,
    Err(error) => {
      crate::log_warn!(
        "failed to resolve native runtime spill directory",
        "native_database::discard_stale_runtime_spill_directories",
        Some(format!("{}: {error}", spill_parent.display()))
      );
      return;
    }
  };

  let entries = match fs::read_dir(spill_parent) {
    Ok(entries) => entries,
    Err(error) => {
      crate::log_warn!(
        "failed to inspect stale native runtime spill directories",
        "native_database::discard_stale_runtime_spill_directories",
        Some(format!("{}: {error}", spill_parent.display()))
      );
      return;
    }
  };

  for entry in entries {
    let entry = match entry {
      Ok(entry) => entry,
      Err(error) => {
        crate::log_warn!(
          "failed to inspect a native runtime spill directory entry",
          "native_database::discard_stale_runtime_spill_directories",
          Some(error.to_string())
        );
        continue;
      }
    };
    let file_name = entry.file_name();
    if entry.path() == active_spill
      || !file_name
        .to_str()
        .is_some_and(|name| name.starts_with(RUNTIME_SPILL_ENTRY_PREFIX))
    {
      continue;
    }
    let path = entry.path();
    let metadata = match fs::symlink_metadata(&path) {
      Ok(metadata) => metadata,
      Err(error) => {
        crate::log_warn!(
          "failed to inspect a native runtime spill directory",
          "native_database::discard_stale_runtime_spill_directories",
          Some(format!("{}: {error}", path.display()))
        );
        continue;
      }
    };
    if !is_normal_directory(&metadata) {
      continue;
    }
    if let Err(error) = ensure_directory_is_contained(&path, &canonical_scope) {
      crate::log_warn!(
        "native runtime spill candidate is outside its verified scope",
        "native_database::discard_stale_runtime_spill_directories",
        Some(format!("{}: {error}", path.display()))
      );
      continue;
    }
    if let Err(error) = fs::remove_dir_all(&path) {
      crate::log_warn!(
        "failed to discard a stale native runtime spill directory",
        "native_database::discard_stale_runtime_spill_directories",
        Some(format!("{}: {error}", path.display()))
      );
    }
  }
}

fn ensure_directory_is_contained(
  path: &Path,
  canonical_parent: &Path,
) -> Result<(), String> {
  let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
  if !is_normal_directory(&metadata) {
    return Err("entry is a symlink, reparse point, or non-directory".to_owned());
  }
  let canonical_path = fs::canonicalize(path).map_err(|error| error.to_string())?;
  if canonical_path.parent() != Some(canonical_parent) {
    return Err(
      "resolved entry is not an immediate child of its verified parent".to_owned(),
    );
  }
  Ok(())
}

/// What identifies a native database file for the in-process claim.
///
/// On Unix it is the file itself, `(device, inode)`, so a symlink, a `.`
/// component or a hard link all resolve to one entry. On Windows it is the
/// canonical path: the stable library exposes no file index there, and the
/// platform's own file lock already refuses a second handle to the same file
/// under any name, which is why this claim was only ever missing on Unix.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum FileIdentity {
  #[cfg(unix)]
  Inode { device: u64, inode: u64 },
  #[cfg(not(unix))]
  CanonicalPath(std::path::PathBuf),
}

impl FileIdentity {
  fn of(path: &Path) -> Result<Self, NativeDatabaseError> {
    let unavailable = || NativeDatabaseError::Unavailable {
      path: path.to_owned(),
    };
    #[cfg(unix)]
    {
      use std::os::unix::fs::MetadataExt;
      let metadata = std::fs::metadata(path).map_err(|_| unavailable())?;
      Ok(Self::Inode {
        device: metadata.dev(),
        inode: metadata.ino(),
      })
    }
    #[cfg(not(unix))]
    {
      std::fs::canonicalize(path)
        .map(Self::CanonicalPath)
        .map_err(|_| unavailable())
    }
  }
}

/// Files a `NativeDatabase` in this process currently holds.
static OPEN_NATIVE_DATABASES: std::sync::LazyLock<
  Mutex<std::collections::HashSet<FileIdentity>>,
> = std::sync::LazyLock::new(Default::default);

/// This process's claim on one native database file, released on drop.
struct OpenClaim {
  identity: FileIdentity,
}

impl OpenClaim {
  fn acquire(path: &Path) -> Result<Self, NativeDatabaseError> {
    let identity = FileIdentity::of(path)?;
    let mut open = OPEN_NATIVE_DATABASES
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    if !open.insert(identity.clone()) {
      return Err(NativeDatabaseError::AlreadyOpen {
        path: path.to_owned(),
      });
    }
    Ok(Self { identity })
  }
}

impl Drop for OpenClaim {
  fn drop(&mut self) {
    OPEN_NATIVE_DATABASES
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .remove(&self.identity);
  }
}

/// What both lanes keep alive for the owner's lifetime. Each lane drops its
/// connection before its reference to this, so the claim is released only
/// once no connection to the file is left.
struct LaneResources {
  _spill: TempDir,
  _claim: OpenClaim,
}

fn run_lane(
  resources: Arc<LaneResources>,
  mut connection: Connection,
  mut receiver: mpsc::Receiver<LaneMessage>,
) {
  let interrupt = connection.interrupt_handle();
  while let Some(message) = receiver.blocking_recv() {
    let LaneMessage::Run {
      cancellation,
      operation,
    } = message
    else {
      break;
    };
    {
      let mut active = cancellation
        .inner
        .interrupt
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
      if !cancellation.is_cancelled() {
        *active = Some(Arc::clone(&interrupt));
      }
    }
    let mut context = NativeConnectionContext {
      connection: &mut connection,
      cancellation: cancellation.clone(),
      healthy: true,
    };
    operation(&mut context);
    let healthy = context.healthy;
    *cancellation
      .inner
      .interrupt
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    if !healthy {
      break;
    }
  }
  // The connection goes before this lane's share of the claim, so the file is
  // never reported free while a connection to it is still open.
  drop(connection);
  drop(resources);
}

fn validate_native_metadata(
  connection: &Connection,
  expected_version: u32,
) -> Result<(), NativeDatabaseError> {
  let present: i64 = connection
    .query_row(
      "SELECT count(*) FROM information_schema.tables WHERE table_name = ?",
      [NATIVE_METADATA_TABLE],
      |row| row.get(0),
    )
    .map_err(|error| {
      NativeDatabaseError::duckdb("look for the native schema metadata", error)
    })?;
  if present == 0 {
    return Err(NativeDatabaseError::Unfinalized);
  }
  let row: Option<(String, i64)> = connection
    .query_row(
      &format!("SELECT state, schema_version FROM {NATIVE_METADATA_TABLE}"),
      [],
      |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(|error| {
      NativeDatabaseError::duckdb("read the native schema metadata", error)
    })?;
  let Some((state, actual)) = row else {
    return Err(NativeDatabaseError::Unfinalized);
  };
  // A selected database is the same finalized file with its authority
  // recorded, so the owner serves it on the same terms. Refusing it here would
  // make the backend unopenable exactly once it became authoritative.
  if state != FINALIZED_UNSELECTED && state != SELECTED {
    return Err(NativeDatabaseError::Unfinalized);
  }
  let actual = u32::try_from(actual).map_err(|_| NativeDatabaseError::Unfinalized)?;
  if actual != expected_version {
    return Err(NativeDatabaseError::IncompatibleSchema {
      expected: expected_version,
      actual,
    });
  }
  require_storage_version_column(connection, NATIVE_METADATA_TABLE)?;
  let storage_version: String = connection
    .query_row(
      &format!("SELECT storage_version FROM {NATIVE_METADATA_TABLE}"),
      [],
      |row| row.get(0),
    )
    .map_err(|error| {
      NativeDatabaseError::duckdb("read the native schema metadata", error)
    })?;
  verify_storage_version(connection, &storage_version)?;
  Ok(())
}
