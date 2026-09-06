use sha2::{Digest, Sha256};

use super::CandidateError;
use super::source::{ColumnSchema, TableSchema};

pub(super) const COPY_BATCH_ROWS: i64 = 512;
// Buffer-safety limits for the snapshot copier, not whole-process memory caps.
pub(super) const MAX_CELL_BYTES: u64 = 1024 * 1024;
pub(super) const MAX_ROW_BYTES: u64 = 4 * 1024 * 1024;
pub(super) const MAX_BATCH_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub(super) enum CanonicalKind {
  Integer,
  Real,
  Text,
  Blob,
}

impl CanonicalKind {
  pub(super) fn sqlite_storage_class(self) -> &'static str {
    match self {
      Self::Integer => "integer",
      Self::Real => "real",
      Self::Text => "text",
      Self::Blob => "blob",
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Cell {
  Null,
  Integer(i64),
  Real(u64),
  Text(String),
  Blob(Vec<u8>),
}

impl Cell {
  pub(super) fn update_digest(&self, digest: &mut Sha256) {
    match self {
      Self::Null => digest.update([0]),
      Self::Integer(value) => {
        digest.update([1]);
        digest.update(value.to_le_bytes());
      }
      Self::Real(bits) => {
        digest.update([2]);
        digest.update(bits.to_le_bytes());
      }
      Self::Text(value) => {
        digest.update([3]);
        update_bytes(digest, value.as_bytes());
      }
      Self::Blob(value) => {
        digest.update([4]);
        update_bytes(digest, value);
      }
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SourceRow {
  pub(super) ordinal: u64,
  pub(super) cells: Vec<Cell>,
}

impl SourceRow {
  pub(super) fn update_digest(&self, digest: &mut Sha256) {
    digest.update(self.ordinal.to_le_bytes());
    digest.update((self.cells.len() as u64).to_le_bytes());
    for cell in &self.cells {
      cell.update_digest(digest);
    }
  }
}

#[derive(Clone, Debug)]
pub(super) struct RowBatch {
  pub(super) rows: Vec<SourceRow>,
}

#[derive(Clone, Debug)]
pub(super) struct TableDigest {
  pub(super) name: String,
  pub(super) row_count: u64,
  pub(super) sha256: [u8; 32],
}

pub(super) fn canonical_kind(
  table: &str,
  column: &str,
  declared_type: &str,
) -> Result<CanonicalKind, CandidateError> {
  let normalized = declared_type.trim().to_ascii_uppercase();
  match normalized.as_str() {
    "INTEGER" | "BIGINT" | "BOOLEAN" => Ok(CanonicalKind::Integer),
    "REAL" => Ok(CanonicalKind::Real),
    "TEXT" | "DATETIME" | "TIMESTAMP" => Ok(CanonicalKind::Text),
    "BLOB" => Ok(CanonicalKind::Blob),
    "" if table == "sqlite_sequence" && column == "name" => Ok(CanonicalKind::Text),
    "" if table == "sqlite_sequence" && column == "seq" => Ok(CanonicalKind::Integer),
    _ => Err(CandidateError::UnsupportedDeclaredType {
      table: table.to_owned(),
      column: column.to_owned(),
      declared_type: declared_type.to_owned(),
    }),
  }
}

pub(super) fn validate_storage_class(
  table: &TableSchema,
  column: &ColumnSchema,
  row_ordinal: u64,
  actual: &str,
  bytes: u64,
) -> Result<(), CandidateError> {
  if bytes > MAX_CELL_BYTES {
    return Err(CandidateError::CellTooLarge {
      table: table.name.clone(),
      column: column.name.clone(),
      row_ordinal,
      bytes,
      limit: MAX_CELL_BYTES,
    });
  }

  if actual == "null" {
    if column.not_null {
      return Err(CandidateError::NullInRequiredColumn {
        table: table.name.clone(),
        column: column.name.clone(),
        row_ordinal,
      });
    }
    return Ok(());
  }

  let expected = column.canonical_kind.sqlite_storage_class();
  if actual != expected {
    return Err(CandidateError::NonCanonicalCell {
      table: table.name.clone(),
      column: column.name.clone(),
      row_ordinal,
      expected,
      actual: actual.to_owned(),
    });
  }
  Ok(())
}

fn update_bytes(digest: &mut Sha256, bytes: &[u8]) {
  digest.update((bytes.len() as u64).to_le_bytes());
  digest.update(bytes);
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn maps_only_the_supported_declared_types() {
    assert_eq!(
      canonical_kind("PROCESS_STATS", "pid", "INTEGER").unwrap(),
      CanonicalKind::Integer
    );
    assert_eq!(
      canonical_kind("_sqlx_migrations", "installed_on", "TIMESTAMP").unwrap(),
      CanonicalKind::Text
    );
    assert_eq!(
      canonical_kind("sqlite_sequence", "name", "").unwrap(),
      CanonicalKind::Text
    );
    assert!(canonical_kind("other", "value", "").is_err());
    assert!(canonical_kind("other", "value", "NUMERIC").is_err());
  }
}
