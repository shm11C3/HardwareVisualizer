//! The one-time startup upgrade of a selected native database to the schema
//! version this build runs (ADR 0030 decision 7).
//!
//! A selected database is the authority, so it cannot be rebuilt from SQLite
//! the way an unselected one can; a new native-only table has to be added to
//! it in place. The App supplies the ordered steps
//! ([`NativeSchemaDefinition::upgrades`]); this module runs the ones a
//! database still needs.
//!
//! # The order, and the one repairable gap
//!
//! The upgrade follows selection's write order
//! ([`super::selection`]): the database first, then the marker.
//!
//! 1. One DuckDB transaction runs every step's DDL, adds the identity rows the
//!    new native-only tables need, and records the new version in
//!    `__hv_native_metadata`. A failure anywhere rolls all of it back, and the
//!    file stays at the version it had.
//! 2. The committed file is checkpointed and synced.
//! 3. The marker is rewritten atomically with the new version.
//!
//! An interruption between 2 and 3 leaves a marker that records the older
//! version beside a database at this build's version, which
//! [`super::inspect_authority`] reports as
//! [`super::AuthorityInconsistency::MarkerBehindNativeSchema`] and
//! [`super::repair_authority_marker`] closes. The reverse order would leave a
//! marker claiming a version the database may never have committed.
//!
//! # What is upgraded
//!
//! A selected database, planned from the version the database itself
//! records, whenever that version is on this build's upgrade chain and the
//! marker is one authority inspection accepts for a database at that
//! version: in exact agreement, or behind or missing in exactly the ways the
//! marker repair closes. Planning from the database is what keeps an
//! interrupted state from stranding startup - a database left at an
//! intermediate version by an earlier build with its marker behind, or a
//! selection whose marker never landed. Inspection at this build's version
//! would refuse both as a version mismatch, which nothing repairs. Writing
//! the final marker closes the lagging or missing one as well; adopting the
//! selected file without reconciling is sound for the same reason the
//! startup marker repair is.
//!
//! # What is deliberately not upgraded
//!
//! Anything else - an unselected file, a marker ahead of its database or
//! disagreeing with it on the source facts, a version newer than this build
//! or off the upgrade chain - is left untouched and reported by
//! [`super::inspect_authority`] exactly as before: an unselected file keeps
//! its rebuild path, and a newer file stays a version mismatch, so an older
//! build never writes a database a newer one upgraded.

use duckdb::{AccessMode, Connection};

use super::NativeDatabaseError;
use super::cell::quote_identifier;
use super::compatibility::{require_storage_version_column, verify_storage_version};
use super::finalize::{
  NATIVE_IDENTITY_TABLE, NATIVE_METADATA_TABLE, SELECTED, identity_maximum,
  insert_identity, open_database_with_storage_version, require_no_wal,
};
use super::runtime::OpenClaim;
use super::schema::{
  NativeIdentityMode, NativeSchemaDefinition, NativeSchemaUpgrade, apply_upgrade_steps,
};
use super::selection::{
  AuthorityFacts, AuthorityMarker, AuthorityPaths, AuthorityRecovery, AuthorityState,
  MARKER_VERSION, NativeMetadataFacts, NativeState, file_name_of, inspect_authority,
  observe_authority, read_metadata_row, selection_spill, sync_file,
  write_marker_atomically,
};

/// What [`upgrade_selected_native_database`] did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeSchemaUpgradeOutcome {
  /// Nothing to upgrade, or nothing this upgrade may touch. Authority
  /// inspection reports any state that still needs attention.
  NotNeeded,
  /// The database and its marker now record `to`.
  Upgraded { from: u32, to: u32 },
}

/// Why [`upgrade_selected_native_database`] did not complete.
#[derive(Debug, thiserror::Error)]
pub enum NativeSchemaUpgradeError {
  /// The file's in-process open claim was refused, so the file was neither
  /// read nor changed and nothing is known about it.
  #[error("the native database could not be claimed for the schema upgrade: {0}")]
  Unclaimed(#[source] NativeDatabaseError),
  /// A selected database recorded at `from` was planned for an upgrade to
  /// `to`, and the upgrade did not complete. A failure before the commit
  /// rolled every step back; one after it leaves a marker behind the
  /// database, which the startup marker repair closes. Either way the
  /// selected native database stays the authority.
  #[error(
    "upgrading the selected native database from schema version {from} to {to} failed: \
     {source}"
  )]
  Failed {
    from: u32,
    to: u32,
    #[source]
    source: NativeDatabaseError,
  },
}

/// Bring a selected native database recorded at an older schema version up
/// to `schema.version`.
///
/// Blocking: it opens the database as a DuckDB instance of its own, so it
/// belongs at App startup, before any [`super::NativeDatabase`] owner exists.
/// The file's in-process open claim is taken before its metadata is read, so
/// a live owner in this process is refused with
/// [`NativeSchemaUpgradeError::Unclaimed`] before anything opens the file
/// beside it.
///
/// See the module documentation for the write order and what is upgraded.
pub fn upgrade_selected_native_database(
  paths: &AuthorityPaths,
  schema: NativeSchemaDefinition,
) -> Result<NativeSchemaUpgradeOutcome, NativeSchemaUpgradeError> {
  if !paths.native_database.is_file() {
    return Ok(NativeSchemaUpgradeOutcome::NotNeeded);
  }
  // Claimed before observing: reading the metadata opens the file, and beside
  // a live owner that read is refused on Windows and unsound elsewhere.
  let _claim = OpenClaim::acquire(&paths.native_database)
    .map_err(NativeSchemaUpgradeError::Unclaimed)?;
  let facts = observe_authority(paths, schema.version);
  let Some(plan) = plan_upgrade(&facts, &schema) else {
    return Ok(NativeSchemaUpgradeOutcome::NotNeeded);
  };
  run_upgrade(paths, &schema, &plan).map_err(|source| {
    NativeSchemaUpgradeError::Failed {
      from: plan.from,
      to: schema.version,
      source,
    }
  })?;
  Ok(NativeSchemaUpgradeOutcome::Upgraded {
    from: plan.from,
    to: schema.version,
  })
}

/// Carry out `plan` while the caller holds the file's open claim.
fn run_upgrade(
  paths: &AuthorityPaths,
  schema: &NativeSchemaDefinition,
  plan: &UpgradePlan,
) -> Result<(), NativeDatabaseError> {
  let spill = selection_spill()?;
  // Scoped so every DuckDB handle on the file is released before the file is
  // synced and the marker is written, as in selection.
  {
    let connection = open_database_with_storage_version(
      &paths.native_database,
      AccessMode::ReadWrite,
      spill.path(),
    )?;
    require_metadata_unchanged(&connection, plan)?;
    connection
      .execute_batch("BEGIN TRANSACTION")
      .map_err(|error| {
        NativeDatabaseError::duckdb("begin the native schema upgrade", error)
      })?;
    if let Err(error) = apply_upgrade(&connection, schema, plan.steps) {
      let _ = connection.execute_batch("ROLLBACK");
      return Err(error);
    }
    connection.execute_batch("COMMIT").map_err(|error| {
      NativeDatabaseError::duckdb("commit the native schema upgrade", error)
    })?;
    connection.execute_batch("CHECKPOINT").map_err(|error| {
      NativeDatabaseError::duckdb("checkpoint the upgraded database", error)
    })?;
  }
  require_no_wal(&paths.native_database)?;
  sync_file(&paths.native_database)?;

  // Rewritten from the database's own facts, which also closes a marker the
  // plan accepted as behind or missing.
  write_marker_atomically(
    &paths.marker,
    &AuthorityMarker {
      version: MARKER_VERSION,
      native_database_file_name: file_name_of(&paths.native_database)?,
      schema_version: schema.version,
      source_schema_sha256: plan.source_schema_sha256.clone(),
      total_rows: plan.source_rows,
    },
  )
}

/// What the database recorded when the upgrade was planned.
struct UpgradePlan {
  from: u32,
  steps: &'static [NativeSchemaUpgrade],
  source_schema_sha256: String,
  source_rows: u64,
}

/// The upgrade the observed files call for, if any. Pure, so every refusal
/// can be driven without a filesystem.
///
/// Planned from the version the database records. The marker check reuses
/// [`inspect_authority`] at that version rather than restating it: exact
/// agreement reads as selected, and a marker behind or missing reads as the
/// marker repair's state. Everything inspection stops on - a marker ahead of
/// the database, a source-fact disagreement, an unreadable marker, a storage
/// version mismatch - stops the upgrade too.
fn plan_upgrade(
  facts: &AuthorityFacts,
  schema: &NativeSchemaDefinition,
) -> Option<UpgradePlan> {
  let NativeMetadataFacts::Present {
    state: NativeState::Selected,
    schema_version: from,
    source_schema_sha256,
    source_rows,
    ..
  } = &facts.native_metadata
  else {
    return None;
  };
  if *from >= schema.version {
    return None;
  }
  let steps = schema.upgrade_path(*from)?;
  let at_recorded_version = AuthorityFacts {
    expected_schema_version: *from,
    ..facts.clone()
  };
  match inspect_authority(&at_recorded_version) {
    AuthorityState::NativeSelected
    | AuthorityState::Inconsistent {
      recovery: AuthorityRecovery::RepairSelectionMarkerFromNativeMetadata,
      ..
    } => Some(UpgradePlan {
      from: *from,
      steps,
      source_schema_sha256: source_schema_sha256.clone(),
      source_rows: *source_rows,
    }),
    _ => None,
  }
}

/// Re-read the metadata under DuckDB's writer lock: the plan was made from a
/// read-only observation, and nothing may be upgraded on a guess about a file
/// that changed since - neither its version nor the source facts the new
/// marker will record.
fn require_metadata_unchanged(
  connection: &Connection,
  plan: &UpgradePlan,
) -> Result<(), NativeDatabaseError> {
  require_storage_version_column(connection, NATIVE_METADATA_TABLE)?;
  let (state, schema_version, storage_version, source_schema_sha256, source_rows, _) =
    read_metadata_row(connection)?;
  verify_storage_version(connection, &storage_version)?;
  if state != SELECTED {
    return Err(NativeDatabaseError::UnexpectedState {
      operation: "upgraded",
      state,
      expected: SELECTED,
    });
  }
  let source_rows = u64::try_from(source_rows).unwrap_or(u64::MAX);
  if schema_version != i64::from(plan.from)
    || source_schema_sha256 != plan.source_schema_sha256
    || source_rows != plan.source_rows
  {
    return Err(NativeDatabaseError::Verification {
      message: format!(
        "the native database records schema version {schema_version}, source schema \
         {source_schema_sha256} and {source_rows} rows; the upgrade was planned from {}, \
         {} and {}",
        plan.from, plan.source_schema_sha256, plan.source_rows
      ),
    });
  }
  Ok(())
}

/// Everything inside the upgrade transaction.
fn apply_upgrade(
  connection: &Connection,
  schema: &NativeSchemaDefinition,
  steps: &[NativeSchemaUpgrade],
) -> Result<(), NativeDatabaseError> {
  apply_upgrade_steps(connection, steps)?;
  add_missing_native_only_identities(connection, schema)?;
  connection
    .execute(
      &format!(
        "UPDATE {} SET schema_version = ?",
        quote_identifier(NATIVE_METADATA_TABLE)
      ),
      [i64::from(schema.version)],
    )
    .map_err(|error| {
      NativeDatabaseError::duckdb("record the upgraded native schema version", error)
    })?;
  for table in schema.tables.iter().chain(schema.native_only_tables) {
    if !table_exists(connection, table)? {
      return Err(NativeDatabaseError::SchemaMismatch {
        table: (*table).to_owned(),
        detail: "the upgraded native schema does not contain this declared table"
          .to_owned(),
      });
    }
  }
  Ok(())
}

/// Add the identity row of every native-only table a step created. Any other
/// missing identity has a SQLite allocator the upgrade cannot reconstruct, so
/// it is refused rather than started from scratch.
fn add_missing_native_only_identities(
  connection: &Connection,
  schema: &NativeSchemaDefinition,
) -> Result<(), NativeDatabaseError> {
  let mut statement = connection
    .prepare(&format!(
      "SELECT table_name FROM {}",
      quote_identifier(NATIVE_IDENTITY_TABLE)
    ))
    .map_err(|error| {
      NativeDatabaseError::duckdb("prepare the native identity read", error)
    })?;
  let recorded = statement
    .query_map([], |row| row.get::<_, String>(0))
    .map_err(|error| NativeDatabaseError::duckdb("read the native identities", error))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|error| NativeDatabaseError::duckdb("read the native identities", error))?;
  for identity in schema.identities {
    if recorded.iter().any(|table| table == identity.table) {
      continue;
    }
    if identity.mode != NativeIdentityMode::NativeOnly {
      return Err(NativeDatabaseError::SchemaMismatch {
        table: identity.table.to_owned(),
        detail: "the database records no identity for this SQLite-sourced table, and an \
                 upgrade cannot reconstruct its SQLite allocator"
          .to_owned(),
      });
    }
    insert_identity(
      connection,
      identity,
      identity_maximum(connection, identity)?,
    )?;
  }
  Ok(())
}

fn table_exists(
  connection: &Connection,
  table: &str,
) -> Result<bool, NativeDatabaseError> {
  let present: i64 = connection
    .query_row(
      "SELECT count(*) FROM information_schema.tables WHERE table_name = ?",
      [table],
      |row| row.get(0),
    )
    .map_err(|error| NativeDatabaseError::duckdb("look for an upgraded table", error))?;
  Ok(present != 0)
}

#[cfg(test)]
mod tests {
  use super::super::selection::MarkerFacts;
  use super::*;
  use crate::infrastructure::database::native_database::{
    AuthorityInconsistency, NativeCancellation, NativeDatabase, NativeDatabaseOptions,
    NativeIdentity, create_empty_native_database, repair_authority_marker,
  };

  const V1_SQL: &str = "CREATE TABLE archive (id BIGINT PRIMARY KEY, value VARCHAR)";
  const NATIVE_ONLY_SQL: &str =
    "CREATE TABLE native_only (id BIGINT PRIMARY KEY, value VARCHAR NOT NULL)";

  static V1_IDENTITIES: &[NativeIdentity] = &[NativeIdentity {
    table: "archive",
    column: "id",
    mode: NativeIdentityMode::AutoIncrement {
      sqlite_sequence_name: "archive",
    },
  }];
  static V2_IDENTITIES: &[NativeIdentity] = &[
    NativeIdentity {
      table: "archive",
      column: "id",
      mode: NativeIdentityMode::AutoIncrement {
        sqlite_sequence_name: "archive",
      },
    },
    NativeIdentity {
      table: "native_only",
      column: "id",
      mode: NativeIdentityMode::NativeOnly,
    },
  ];
  static V1_TO_V2: &[NativeSchemaUpgrade] = &[NativeSchemaUpgrade {
    from: 1,
    to: 2,
    sql: NATIVE_ONLY_SQL,
  }];
  static FAILING_V1_TO_V2: &[NativeSchemaUpgrade] = &[NativeSchemaUpgrade {
    from: 1,
    to: 2,
    sql: "CREATE TABLE native_only (id BIGINT PRIMARY KEY); \
          SELECT * FROM a_table_that_does_not_exist",
  }];

  fn v1() -> NativeSchemaDefinition {
    NativeSchemaDefinition {
      version: 1,
      sql: V1_SQL,
      tables: &["archive"],
      native_only_tables: &[],
      timestamp_columns: &[],
      identities: V1_IDENTITIES,
      upgrades: &[],
    }
  }

  fn v2_with(upgrades: &'static [NativeSchemaUpgrade]) -> NativeSchemaDefinition {
    NativeSchemaDefinition {
      version: 2,
      sql: V1_SQL,
      tables: &["archive"],
      native_only_tables: &["native_only"],
      timestamp_columns: &[],
      identities: V2_IDENTITIES,
      upgrades,
    }
  }

  fn v2() -> NativeSchemaDefinition {
    v2_with(V1_TO_V2)
  }

  static V3_IDENTITIES: &[NativeIdentity] = &[
    NativeIdentity {
      table: "archive",
      column: "id",
      mode: NativeIdentityMode::AutoIncrement {
        sqlite_sequence_name: "archive",
      },
    },
    NativeIdentity {
      table: "native_only",
      column: "id",
      mode: NativeIdentityMode::NativeOnly,
    },
    NativeIdentity {
      table: "native_only_v3",
      column: "id",
      mode: NativeIdentityMode::NativeOnly,
    },
  ];
  static V1_TO_V3: &[NativeSchemaUpgrade] = &[
    NativeSchemaUpgrade {
      from: 1,
      to: 2,
      sql: NATIVE_ONLY_SQL,
    },
    NativeSchemaUpgrade {
      from: 2,
      to: 3,
      sql: "CREATE TABLE native_only_v3 (id BIGINT PRIMARY KEY)",
    },
  ];

  fn v3() -> NativeSchemaDefinition {
    NativeSchemaDefinition {
      version: 3,
      native_only_tables: &["native_only", "native_only_v3"],
      identities: V3_IDENTITIES,
      upgrades: V1_TO_V3,
      ..v1()
    }
  }

  fn rewrite_marker(paths: &AuthorityPaths, edit: impl FnOnce(&mut AuthorityMarker)) {
    let MarkerFacts::Present(mut marker) = observe_authority(paths, 0).marker else {
      panic!("the marker must be readable");
    };
    edit(&mut marker);
    write_marker_atomically(&paths.marker, &marker).unwrap();
  }

  fn paths(directory: &std::path::Path) -> AuthorityPaths {
    AuthorityPaths::in_directory(directory, "hv-database.db", "hv-database.duckdb")
  }

  /// A fresh v1 selected database holding two archive rows written through
  /// the native owner, so the AUTOINCREMENT high-water mark is 2.
  async fn selected_v1_with_rows(directory: &std::path::Path) -> AuthorityPaths {
    let paths = paths(directory);
    create_empty_native_database(paths.clone(), v1())
      .await
      .unwrap();
    let database =
      NativeDatabase::open(&paths.native_database, NativeDatabaseOptions::new(1))
        .await
        .unwrap();
    for value in ["first", "second"] {
      database
        .request_write(NativeCancellation::new(), move |context| {
          context.with_transaction(|transaction| {
            let id = transaction.next_id("archive")?;
            transaction
              .connection()
              .execute(
                "INSERT INTO archive VALUES (?, ?)",
                duckdb::params![id, value],
              )
              .map(|_| ())
              .map_err(|error| NativeDatabaseError::duckdb("insert test row", error))
          })
        })
        .await
        .unwrap();
    }
    database.close().await.unwrap();
    paths
  }

  fn read_only(paths: &AuthorityPaths) -> (Connection, tempfile::TempDir) {
    let spill = selection_spill().unwrap();
    let connection = super::super::finalize::open_database(
      &paths.native_database,
      AccessMode::ReadOnly,
      spill.path(),
    )
    .unwrap();
    (connection, spill)
  }

  fn recorded_version(paths: &AuthorityPaths) -> i64 {
    let (connection, _spill) = read_only(paths);
    read_metadata_row(&connection).unwrap().1
  }

  fn marker_version(paths: &AuthorityPaths) -> u32 {
    let MarkerFacts::Present(marker) = observe_authority(paths, 0).marker else {
      panic!("the marker must be readable");
    };
    marker.schema_version
  }

  fn has_table(paths: &AuthorityPaths, table: &str) -> bool {
    let (connection, _spill) = read_only(paths);
    table_exists(&connection, table).unwrap()
  }

  async fn next_native_only_id(paths: &AuthorityPaths) -> i64 {
    let database =
      NativeDatabase::open(&paths.native_database, NativeDatabaseOptions::new(2))
        .await
        .unwrap();
    let id = database
      .request_write(NativeCancellation::new(), |context| {
        context.with_transaction(|transaction| transaction.next_id("native_only"))
      })
      .await
      .unwrap();
    database.close().await.unwrap();
    id
  }

  #[tokio::test]
  async fn a_selected_v1_database_upgrades_once_and_keeps_its_rows_and_identities() {
    let directory = tempfile::tempdir().unwrap();
    let paths = selected_v1_with_rows(directory.path()).await;

    assert_eq!(
      upgrade_selected_native_database(&paths, v2()).unwrap(),
      NativeSchemaUpgradeOutcome::Upgraded { from: 1, to: 2 }
    );
    assert_eq!(recorded_version(&paths), 2);
    assert_eq!(marker_version(&paths), 2);
    assert_eq!(
      inspect_authority(&observe_authority(&paths, 2)),
      AuthorityState::NativeSelected
    );
    {
      let (connection, _spill) = read_only(&paths);
      let rows: i64 = connection
        .query_row("SELECT count(*) FROM archive", [], |row| row.get(0))
        .unwrap();
      assert_eq!(rows, 2);
      let native_only_rows: i64 = connection
        .query_row("SELECT count(*) FROM native_only", [], |row| row.get(0))
        .unwrap();
      assert_eq!(native_only_rows, 0);
      let high_water: i64 = connection
        .query_row(
          "SELECT high_water FROM __hv_native_identities WHERE table_name = 'archive'",
          [],
          |row| row.get(0),
        )
        .unwrap();
      assert_eq!(high_water, 2);
    }
    assert_eq!(next_native_only_id(&paths).await, 1);

    // A second boot finds nothing to do.
    assert_eq!(
      upgrade_selected_native_database(&paths, v2()).unwrap(),
      NativeSchemaUpgradeOutcome::NotNeeded
    );
  }

  #[tokio::test]
  async fn a_failing_step_rolls_back_to_v1_and_leaves_the_marker_alone() {
    let directory = tempfile::tempdir().unwrap();
    let paths = selected_v1_with_rows(directory.path()).await;
    let marker_before = std::fs::read(&paths.marker).unwrap();

    upgrade_selected_native_database(&paths, v2_with(FAILING_V1_TO_V2)).unwrap_err();

    assert_eq!(recorded_version(&paths), 1);
    assert!(!has_table(&paths, "native_only"));
    assert_eq!(std::fs::read(&paths.marker).unwrap(), marker_before);
    assert_eq!(
      inspect_authority(&observe_authority(&paths, 1)),
      AuthorityState::NativeSelected
    );
  }

  #[tokio::test]
  async fn a_marker_left_behind_after_the_commit_is_repaired() {
    let directory = tempfile::tempdir().unwrap();
    let paths = selected_v1_with_rows(directory.path()).await;
    let marker_before = std::fs::read(&paths.marker).unwrap();
    upgrade_selected_native_database(&paths, v2()).unwrap();
    // Simulate an interruption between the database commit and the marker.
    std::fs::write(&paths.marker, marker_before).unwrap();

    assert_eq!(
      inspect_authority(&observe_authority(&paths, 2)),
      AuthorityState::Inconsistent {
        reason: AuthorityInconsistency::MarkerBehindNativeSchema,
        recovery: AuthorityRecovery::RepairSelectionMarkerFromNativeMetadata,
      }
    );
    // The upgrade itself does not guess at it.
    assert_eq!(
      upgrade_selected_native_database(&paths, v2()).unwrap(),
      NativeSchemaUpgradeOutcome::NotNeeded
    );
    repair_authority_marker(&paths).unwrap();
    assert_eq!(marker_version(&paths), 2);
    assert_eq!(
      inspect_authority(&observe_authority(&paths, 2)),
      AuthorityState::NativeSelected
    );
  }

  #[tokio::test]
  async fn a_database_from_a_newer_build_is_never_upgraded_or_opened() {
    let directory = tempfile::tempdir().unwrap();
    let paths = selected_v1_with_rows(directory.path()).await;
    upgrade_selected_native_database(&paths, v2()).unwrap();

    // This build runs v1 and has no path down from v2.
    assert_eq!(
      upgrade_selected_native_database(&paths, v1()).unwrap(),
      NativeSchemaUpgradeOutcome::NotNeeded
    );
    assert!(matches!(
      NativeDatabase::open(&paths.native_database, NativeDatabaseOptions::new(1)).await,
      Err(NativeDatabaseError::IncompatibleSchema {
        expected: 1,
        actual: 2
      })
    ));
    assert_eq!(
      inspect_authority(&observe_authority(&paths, 1)),
      AuthorityState::Inconsistent {
        reason: AuthorityInconsistency::SchemaVersionMismatch,
        recovery: AuthorityRecovery::StopAndReport,
      }
    );
  }

  #[tokio::test]
  async fn a_live_owner_in_this_process_refuses_the_upgrade() {
    let directory = tempfile::tempdir().unwrap();
    let paths = selected_v1_with_rows(directory.path()).await;
    let owner =
      NativeDatabase::open(&paths.native_database, NativeDatabaseOptions::new(1))
        .await
        .unwrap();

    // The claim refuses before anything opens the file. Had the metadata been
    // observed first, Windows would have read it as unreadable and answered
    // `NotNeeded`, and Linux and macOS would have read it beside the owner.
    assert!(matches!(
      upgrade_selected_native_database(&paths, v2()),
      Err(NativeSchemaUpgradeError::Unclaimed(
        NativeDatabaseError::AlreadyOpen { .. }
      ))
    ));
    owner.close().await.unwrap();
    assert_eq!(recorded_version(&paths), 1);
  }

  #[test]
  fn no_native_database_needs_no_upgrade_and_creates_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let paths = paths(directory.path());

    assert_eq!(
      upgrade_selected_native_database(&paths, v2()).unwrap(),
      NativeSchemaUpgradeOutcome::NotNeeded
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
  }

  /// An earlier build's upgrade stopped between its commit and its marker:
  /// the database is at an intermediate version on the chain and the marker
  /// is behind it. Inspection at this build's version would only see a
  /// version mismatch, so the upgrade plans from the database's own version.
  #[tokio::test]
  async fn an_interrupted_earlier_upgrade_continues_to_this_builds_version() {
    let directory = tempfile::tempdir().unwrap();
    let paths = selected_v1_with_rows(directory.path()).await;
    upgrade_selected_native_database(&paths, v2()).unwrap();
    rewrite_marker(&paths, |marker| marker.schema_version = 1);

    assert_eq!(
      upgrade_selected_native_database(&paths, v3()).unwrap(),
      NativeSchemaUpgradeOutcome::Upgraded { from: 2, to: 3 }
    );
    assert_eq!(recorded_version(&paths), 3);
    assert_eq!(marker_version(&paths), 3);
    assert!(has_table(&paths, "native_only"));
    assert!(has_table(&paths, "native_only_v3"));
    assert_eq!(
      inspect_authority(&observe_authority(&paths, 3)),
      AuthorityState::NativeSelected
    );
  }

  /// A selection whose marker never landed: the upgrade writes the marker
  /// for the version it reaches, instead of leaving the repair to restore it
  /// at a version this build cannot open.
  #[tokio::test]
  async fn a_selected_database_without_its_marker_upgrades_and_gains_one() {
    let directory = tempfile::tempdir().unwrap();
    let paths = selected_v1_with_rows(directory.path()).await;
    std::fs::remove_file(&paths.marker).unwrap();

    assert_eq!(
      upgrade_selected_native_database(&paths, v2()).unwrap(),
      NativeSchemaUpgradeOutcome::Upgraded { from: 1, to: 2 }
    );
    assert_eq!(recorded_version(&paths), 2);
    assert_eq!(marker_version(&paths), 2);
    assert_eq!(
      inspect_authority(&observe_authority(&paths, 2)),
      AuthorityState::NativeSelected
    );
  }

  /// A marker ahead of its database claims an upgrade the database never
  /// committed; nothing is upgraded on that guess.
  #[tokio::test]
  async fn a_marker_ahead_of_its_database_is_not_upgraded() {
    let directory = tempfile::tempdir().unwrap();
    let paths = selected_v1_with_rows(directory.path()).await;
    rewrite_marker(&paths, |marker| marker.schema_version = 2);

    assert_eq!(
      upgrade_selected_native_database(&paths, v3()).unwrap(),
      NativeSchemaUpgradeOutcome::NotNeeded
    );
    assert_eq!(recorded_version(&paths), 1);
    assert_eq!(marker_version(&paths), 2);
  }

  /// The plan comes from a read-only observation; source facts that moved
  /// before the writer lock was taken refuse the upgrade.
  #[tokio::test]
  async fn source_facts_that_moved_after_planning_refuse_the_upgrade() {
    let directory = tempfile::tempdir().unwrap();
    let paths = selected_v1_with_rows(directory.path()).await;
    let spill = selection_spill().unwrap();
    let connection = open_database_with_storage_version(
      &paths.native_database,
      AccessMode::ReadWrite,
      spill.path(),
    )
    .unwrap();
    let planned = |source_schema_sha256: &str, source_rows| UpgradePlan {
      from: 1,
      steps: V1_TO_V2,
      source_schema_sha256: source_schema_sha256.to_owned(),
      source_rows,
    };

    require_metadata_unchanged(&connection, &planned("", 0)).unwrap();
    for plan in [planned("another-source", 0), planned("", 1)] {
      assert!(matches!(
        require_metadata_unchanged(&connection, &plan),
        Err(NativeDatabaseError::Verification { .. })
      ));
    }
  }

  #[tokio::test]
  async fn fresh_creation_builds_the_upgraded_schema_directly() {
    let directory = tempfile::tempdir().unwrap();
    let paths = paths(directory.path());
    let marker = create_empty_native_database(paths.clone(), v2())
      .await
      .unwrap();

    assert_eq!(marker.schema_version, 2);
    assert_eq!(recorded_version(&paths), 2);
    assert!(has_table(&paths, "native_only"));
    assert_eq!(
      upgrade_selected_native_database(&paths, v2()).unwrap(),
      NativeSchemaUpgradeOutcome::NotNeeded
    );
    assert_eq!(next_native_only_id(&paths).await, 1);
  }

  fn selected_facts(marker_version: u32, database_version: u32) -> AuthorityFacts {
    AuthorityFacts {
      source_database_present: false,
      native_database_present: true,
      native_database_file_name: "hv-database.duckdb".to_owned(),
      native_write_ahead_log_present: false,
      work_directory_present: false,
      marker: MarkerFacts::Present(AuthorityMarker {
        version: super::super::selection::MARKER_VERSION,
        native_database_file_name: "hv-database.duckdb".to_owned(),
        schema_version: marker_version,
        source_schema_sha256: String::new(),
        total_rows: 0,
      }),
      native_metadata: NativeMetadataFacts::Present {
        state: NativeState::Selected,
        schema_version: database_version,
        storage_version: "v1.0.0+".to_owned(),
        engine_storage_version: "v1.0.0+".to_owned(),
        source_schema_sha256: String::new(),
        source_rows: 0,
      },
      expected_schema_version: 2,
    }
  }

  #[test]
  fn a_selected_database_on_the_chain_with_an_acceptable_marker_is_planned() {
    let schema = v2();
    assert!(plan_upgrade(&selected_facts(1, 1), &schema).is_some());

    // Already current, ahead of this build, or off the chain.
    assert!(plan_upgrade(&selected_facts(2, 2), &schema).is_none());
    assert!(plan_upgrade(&selected_facts(3, 3), &schema).is_none());
    assert!(plan_upgrade(&selected_facts(0, 0), &schema).is_none());
    // A marker ahead of its database is inspection's to report.
    assert!(plan_upgrade(&selected_facts(2, 1), &schema).is_none());

    // Planned from the database's own version: a marker behind a database at
    // an intermediate version, or a missing one, is what the repair closes.
    let plan = plan_upgrade(&selected_facts(1, 2), &v3()).unwrap();
    assert_eq!((plan.from, plan.steps), (2, &V1_TO_V3[1..]));
    let mut no_marker = selected_facts(1, 1);
    no_marker.marker = MarkerFacts::Absent;
    assert!(plan_upgrade(&no_marker, &schema).is_some());

    // A marker that disagrees on the source facts is never adopted.
    let mut other_source = selected_facts(1, 1);
    if let MarkerFacts::Present(marker) = &mut other_source.marker {
      marker.total_rows = 1;
    }
    assert!(plan_upgrade(&other_source, &schema).is_none());
    let mut unreadable_marker = selected_facts(1, 1);
    unreadable_marker.marker = MarkerFacts::Unreadable;
    assert!(plan_upgrade(&unreadable_marker, &schema).is_none());

    let mut unselected = selected_facts(1, 1);
    if let NativeMetadataFacts::Present { state, .. } = &mut unselected.native_metadata {
      *state = NativeState::FinalizedUnselected;
    }
    assert!(plan_upgrade(&unselected, &schema).is_none());

    let mut other_storage = selected_facts(1, 1);
    if let NativeMetadataFacts::Present {
      engine_storage_version,
      ..
    } = &mut other_storage.native_metadata
    {
      *engine_storage_version = "v1.2.0+".to_owned();
    }
    assert!(plan_upgrade(&other_storage, &schema).is_none());
  }
}
