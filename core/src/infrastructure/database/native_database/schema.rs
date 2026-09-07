//! The App-supplied description of the stable native schema.
//!
//! Core owns how a candidate is finalized, how ids are allocated and how the
//! tables are queried. App owns *which* tables exist, in which order they may
//! be created, which columns are derived query keys, and which SQLite identity
//! rule each `id` column carries - the same ownership split as the ordered
//! SQLite migration set.

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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeIdentity {
  pub table: &'static str,
  pub column: &'static str,
  pub mode: NativeIdentityMode,
}

#[derive(Clone, Copy, Debug)]
pub struct NativeSchemaDefinition {
  pub version: u32,
  /// DDL creating every domain table, executed once against the empty
  /// destination.
  pub sql: &'static str,
  /// The domain tables, ordered so a table is created and filled after the
  /// tables its foreign keys reference.
  pub tables: &'static [&'static str],
  pub timestamp_columns: &'static [NativeTimestampColumn],
  pub identities: &'static [NativeIdentity],
}

impl NativeSchemaDefinition {
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
