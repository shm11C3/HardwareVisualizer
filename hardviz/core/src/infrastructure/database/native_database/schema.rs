//! The App-supplied description of the stable native schema.
//!
//! Core owns how a candidate is finalized, how ids are allocated and how the
//! tables are queried. App owns *which* tables exist, in which order they may
//! be created, which columns are derived query keys, and which SQLite identity
//! rule each `id` column carries - the same ownership split as the ordered
//! SQLite migration set.

use duckdb::Connection;

use super::NativeDatabaseError;

/// One derived epoch-millisecond column and the stored timestamp text it is
/// computed from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeTimestampColumn {
  pub table: &'static str,
  pub source_column: &'static str,
  pub epoch_milliseconds_column: &'static str,
}

/// How SQLite allocated the table's `id`, so the native allocator can keep
/// producing the same ids. See `NativeTransactionContext::next_id` for the
/// contract each mode has to reproduce.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeIdentityMode {
  /// `INTEGER PRIMARY KEY` without AUTOINCREMENT.
  RowId,
  /// `INTEGER PRIMARY KEY AUTOINCREMENT`, whose high-water mark lives in the
  /// named `sqlite_sequence` row.
  AutoIncrement { sqlite_sequence_name: &'static str },
  /// A native-only table (ADR 0030): no SQLite allocator exists to import.
  /// It is recorded as `autoincrement`, so `next_id` allocates it unchanged
  /// and a deleted highest row never releases its id, starting from the
  /// table's current maximum (zero for an empty table).
  NativeOnly,
}

impl NativeIdentityMode {
  /// The allocation rule `__hv_native_identities.mode` records for this mode.
  pub(super) fn stored_mode(self) -> &'static str {
    match self {
      Self::RowId => "rowid",
      Self::AutoIncrement { .. } | Self::NativeOnly => "autoincrement",
    }
  }
}

/// One ordered native schema step: the DDL that takes a database recorded at
/// `from` to `to`.
///
/// A step only adds what a native-only family needs (ADR 0030 decision 3):
/// it never changes a SQLite-sourced table, so conversion can keep planning
/// every one of those from the SQLite candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeSchemaUpgrade {
  pub from: u32,
  pub to: u32,
  pub sql: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeIdentity {
  pub table: &'static str,
  pub column: &'static str,
  pub mode: NativeIdentityMode,
}

#[derive(Clone, Copy, Debug)]
pub struct NativeSchemaDefinition {
  /// The version every path below produces: fresh creation, conversion and
  /// the startup upgrade of an existing selected database.
  pub version: u32,
  /// Baseline DDL, executed once against the empty destination. It describes
  /// the first version of the upgrade chain (`upgrades[0].from`), or
  /// `version` itself when there are no upgrades.
  pub sql: &'static str,
  /// The SQLite-sourced domain tables, ordered so a table is created and
  /// filled after the tables its foreign keys reference. Conversion copies,
  /// reconciles and counts exactly these.
  pub tables: &'static [&'static str],
  /// Tables with no SQLite source (ADR 0030). Conversion creates them empty
  /// and leaves them out of the copy, the reconciliation and the row counts.
  pub native_only_tables: &'static [&'static str],
  pub timestamp_columns: &'static [NativeTimestampColumn],
  pub identities: &'static [NativeIdentity],
  /// Contiguous ordered steps after the baseline: each step's `from` is the
  /// previous step's `to`, and the last `to` is `version`. Every path runs
  /// `sql` and then these steps, so fresh, converted and upgraded files share
  /// one DDL source.
  pub upgrades: &'static [NativeSchemaUpgrade],
}

impl NativeSchemaDefinition {
  /// The steps that take a database recorded at `from` to [`Self::version`],
  /// or `None` when `from` is not on a contiguous chain that ends there. An
  /// empty slice means `from` already is the current version.
  pub(super) fn upgrade_path(&self, from: u32) -> Option<&'static [NativeSchemaUpgrade]> {
    let upgrades = self.upgrades;
    let contiguous = upgrades.windows(2).all(|pair| pair[0].to == pair[1].from)
      && upgrades.iter().all(|step| step.from < step.to)
      && upgrades.last().is_none_or(|last| last.to == self.version);
    if !contiguous {
      return None;
    }
    if from == self.version {
      return Some(&[]);
    }
    let start = upgrades.iter().position(|step| step.from == from)?;
    Some(&upgrades[start..])
  }

  /// Every step after the baseline DDL, for a path that builds the schema
  /// from nothing (fresh creation and finalization).
  pub(super) fn steps_after_baseline(
    &self,
  ) -> Result<&'static [NativeSchemaUpgrade], NativeDatabaseError> {
    let baseline = self.upgrades.first().map_or(self.version, |step| step.from);
    self.upgrade_path(baseline).ok_or_else(|| {
      NativeDatabaseError::InvalidSchemaDefinition {
        detail: format!(
          "the upgrade steps do not form a contiguous chain from {baseline} to {}",
          self.version
        ),
      }
    })
  }

  /// Run the baseline DDL and then every upgrade step, so a schema built from
  /// nothing is the same one an upgraded database ends up with.
  pub(super) fn create_all(
    &self,
    connection: &Connection,
  ) -> Result<(), NativeDatabaseError> {
    let steps = self.steps_after_baseline()?;
    connection
      .execute_batch(self.sql)
      .map_err(|error| NativeDatabaseError::duckdb("create the native schema", error))?;
    apply_upgrade_steps(connection, steps)
  }

  pub(super) fn derived_column_for(
    &self,
    table: &str,
    column: &str,
  ) -> Option<&NativeTimestampColumn> {
    self.timestamp_columns.iter().find(|timestamp| {
      timestamp.table == table && timestamp.epoch_milliseconds_column == column
    })
  }
}

/// Execute `steps` in order on `connection`, inside whatever transaction the
/// caller holds.
pub(super) fn apply_upgrade_steps(
  connection: &Connection,
  steps: &[NativeSchemaUpgrade],
) -> Result<(), NativeDatabaseError> {
  for step in steps {
    connection.execute_batch(step.sql).map_err(|error| {
      NativeDatabaseError::duckdb("apply a native schema upgrade step", error)
    })?;
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn definition(
    version: u32,
    upgrades: &'static [NativeSchemaUpgrade],
  ) -> NativeSchemaDefinition {
    NativeSchemaDefinition {
      version,
      sql: "",
      tables: &[],
      native_only_tables: &[],
      timestamp_columns: &[],
      identities: &[],
      upgrades,
    }
  }

  const fn step(from: u32, to: u32) -> NativeSchemaUpgrade {
    NativeSchemaUpgrade { from, to, sql: "" }
  }

  #[test]
  fn a_contiguous_chain_upgrades_from_any_version_on_it() {
    static CHAIN: &[NativeSchemaUpgrade] = &[step(1, 2), step(2, 3)];
    let schema = definition(3, CHAIN);

    assert_eq!(schema.upgrade_path(1), Some(CHAIN));
    assert_eq!(schema.upgrade_path(2), Some(&CHAIN[1..]));
    assert_eq!(schema.upgrade_path(3), Some(&[][..]));
    assert_eq!(schema.steps_after_baseline().unwrap(), CHAIN);
  }

  #[test]
  fn a_version_off_the_chain_has_no_upgrade_path() {
    static CHAIN: &[NativeSchemaUpgrade] = &[step(1, 2)];
    let schema = definition(2, CHAIN);

    // Older than the baseline, and newer than this build.
    assert_eq!(schema.upgrade_path(0), None);
    assert_eq!(schema.upgrade_path(4), None);
  }

  #[test]
  fn a_chain_with_a_gap_or_a_wrong_end_has_no_upgrade_path() {
    static GAP: &[NativeSchemaUpgrade] = &[step(1, 2), step(3, 4)];
    static SHORT: &[NativeSchemaUpgrade] = &[step(1, 2)];
    static BACKWARDS: &[NativeSchemaUpgrade] = &[step(2, 1)];

    for schema in [
      definition(4, GAP),
      definition(3, SHORT),
      definition(1, BACKWARDS),
    ] {
      assert_eq!(schema.upgrade_path(1), None);
      assert!(matches!(
        schema.steps_after_baseline(),
        Err(NativeDatabaseError::InvalidSchemaDefinition { .. })
      ));
    }
  }

  #[test]
  fn a_definition_without_upgrades_is_its_own_baseline() {
    let schema = definition(1, &[]);

    assert_eq!(schema.upgrade_path(1), Some(&[][..]));
    assert_eq!(schema.upgrade_path(0), None);
    assert_eq!(schema.steps_after_baseline().unwrap(), &[][..]);
  }
}
