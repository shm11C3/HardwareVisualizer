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
//! # What is deliberately not upgraded
//!
//! Only a selected database whose marker agrees with it exactly. Anything
//! else - an unselected file, a disagreement, a version newer than this build
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
  AuthorityFacts, AuthorityMarker, AuthorityPaths, MarkerFacts, NativeMetadataFacts,
  NativeState, file_name_of, observe_authority, read_metadata_row, selection_spill,
  sync_file, write_marker_atomically,
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

/// Bring a selected native database recorded at an older schema version up
/// to `schema.version`.
///
/// Blocking: it opens the database as a DuckDB instance of its own, so it
/// belongs at App startup, before any [`super::NativeDatabase`] owner exists.
/// It takes the file's in-process open claim, so a live owner in this process
/// is refused with [`NativeDatabaseError::AlreadyOpen`] rather than raced.
///
/// See the module documentation for the write order and what is left alone.
pub fn upgrade_selected_native_database(
  paths: &AuthorityPaths,
  schema: NativeSchemaDefinition,
) -> Result<NativeSchemaUpgradeOutcome, NativeDatabaseError> {
  let facts = observe_authority(paths, schema.version);
  let Some(plan) = plan_upgrade(&facts, &schema) else {
    return Ok(NativeSchemaUpgradeOutcome::NotNeeded);
  };

  let _claim = OpenClaim::acquire(&paths.native_database)?;
  let spill = selection_spill()?;
  // Scoped so every DuckDB handle on the file is released before the file is
  // synced and the marker is written, as in selection.
  {
    let connection = open_database_with_storage_version(
      &paths.native_database,
      AccessMode::ReadWrite,
      spill.path(),
    )?;
    require_metadata_unchanged(&connection, plan.from)?;
    connection
      .execute_batch("BEGIN TRANSACTION")
      .map_err(|error| {
        NativeDatabaseError::duckdb("begin the native schema upgrade", error)
      })?;
    if let Err(error) = apply_upgrade(&connection, &schema, plan.steps) {
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

  write_marker_atomically(
    &paths.marker,
    &AuthorityMarker {
      native_database_file_name: file_name_of(&paths.native_database)?,
      schema_version: schema.version,
      ..plan.marker
    },
  )?;
  Ok(NativeSchemaUpgradeOutcome::Upgraded {
    from: plan.from,
    to: schema.version,
  })
}

struct UpgradePlan {
  from: u32,
  steps: &'static [NativeSchemaUpgrade],
  marker: AuthorityMarker,
}

/// The upgrade the observed files call for, if any. Pure, so every refusal
/// can be driven without a filesystem.
fn plan_upgrade(
  facts: &AuthorityFacts,
  schema: &NativeSchemaDefinition,
) -> Option<UpgradePlan> {
  let MarkerFacts::Present(marker) = &facts.marker else {
    return None;
  };
  let NativeMetadataFacts::Present {
    state: NativeState::Selected,
    schema_version: from,
    storage_version,
    engine_storage_version,
    source_schema_sha256,
    source_rows,
  } = &facts.native_metadata
  else {
    return None;
  };
  let marker_agrees = facts.native_database_present
    && marker.native_database_file_name == facts.native_database_file_name
    && marker.schema_version == *from
    && &marker.source_schema_sha256 == source_schema_sha256
    && marker.total_rows == *source_rows;
  if !marker_agrees
    || storage_version != engine_storage_version
    || *from >= schema.version
  {
    return None;
  }
  Some(UpgradePlan {
    from: *from,
    steps: schema.upgrade_path(*from)?,
    marker: marker.clone(),
  })
}

/// Re-read the metadata under DuckDB's writer lock: the plan was made from a
/// read-only observation, and nothing may be upgraded on a guess about a file
/// that changed since.
fn require_metadata_unchanged(
  connection: &Connection,
  from: u32,
) -> Result<(), NativeDatabaseError> {
  require_storage_version_column(connection, NATIVE_METADATA_TABLE)?;
  let (state, schema_version, storage_version, ..) = read_metadata_row(connection)?;
  verify_storage_version(connection, &storage_version)?;
  if state != SELECTED {
    return Err(NativeDatabaseError::UnexpectedState {
      operation: "upgraded",
      state,
      expected: SELECTED,
    });
  }
  if schema_version != i64::from(from) {
    return Err(NativeDatabaseError::Verification {
      message: format!(
        "the native database records schema version {schema_version}, but version \
         {from} was observed before the upgrade"
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
  use super::*;
  use crate::infrastructure::database::native_database::{
    AuthorityInconsistency, AuthorityRecovery, AuthorityState, NativeCancellation,
    NativeDatabase, NativeDatabaseOptions, NativeIdentity, create_empty_native_database,
    inspect_authority, repair_authority_marker,
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

    // Observation cannot read a file the owner holds on every platform, so
    // the refusal is either "nothing observed to upgrade" or the claim.
    match upgrade_selected_native_database(&paths, v2()) {
      Ok(NativeSchemaUpgradeOutcome::NotNeeded)
      | Err(NativeDatabaseError::AlreadyOpen { .. }) => {}
      other => panic!("a live owner must not be upgraded beside: {other:?}"),
    }
    owner.close().await.unwrap();
    assert_eq!(recorded_version(&paths), 1);
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
  fn only_an_agreeing_selected_database_on_the_chain_is_planned() {
    let schema = v2();
    assert!(plan_upgrade(&selected_facts(1, 1), &schema).is_some());

    // Already current, ahead of this build, or off the chain.
    assert!(plan_upgrade(&selected_facts(2, 2), &schema).is_none());
    assert!(plan_upgrade(&selected_facts(3, 3), &schema).is_none());
    assert!(plan_upgrade(&selected_facts(0, 0), &schema).is_none());
    // A marker that disagrees is inspection's to report.
    assert!(plan_upgrade(&selected_facts(2, 1), &schema).is_none());
    assert!(plan_upgrade(&selected_facts(1, 2), &schema).is_none());

    let mut unselected = selected_facts(1, 1);
    if let NativeMetadataFacts::Present { state, .. } = &mut unselected.native_metadata {
      *state = NativeState::FinalizedUnselected;
    }
    assert!(plan_upgrade(&unselected, &schema).is_none());

    let mut no_marker = selected_facts(1, 1);
    no_marker.marker = MarkerFacts::Absent;
    assert!(plan_upgrade(&no_marker, &schema).is_none());

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
