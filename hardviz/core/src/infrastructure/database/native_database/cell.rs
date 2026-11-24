//! Tagged cell values moved between the candidate and the finalized database.
//!
//! The tagged encoding deliberately matches the candidate builder's
//! ([`crate::infrastructure::database::candidate_database`]) so a value that
//! survived the SQLite -> candidate copy is still recognizable here: the tag
//! byte distinguishes an integer from a real that happens to be integral, and
//! both text and blob bytes are length-prefixed so `"ab"` and `"a"` + `"b"`
//! cannot collide. It is duplicated rather than shared because the candidate's
//! copy also carries snapshot-validation concerns this module has no use for.

use sha2::{Digest, Sha256};

/// Bounded copy limits for finalization. Buffer-safety limits, not
/// whole-process memory caps - the same role they have in the candidate
/// builder.
pub(super) const COPY_BATCH_ROWS: u64 = 512;
pub(super) const MAX_BATCH_BYTES: u64 = 8 * 1024 * 1024;

/// The DuckDB storage types the candidate and the stable schema may use.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NativeColumnKind {
  BigInt,
  /// Only the candidate's `__hv_source_ordinal` uses it; the stable schema
  /// never declares an unsigned column.
  UBigInt,
  Double,
  Varchar,
  Blob,
  /// `UNION(i BIGINT, r DOUBLE)`: a column holding both SQLite storage classes.
  TaggedNumeric,
}

impl NativeColumnKind {
  pub(super) fn parse(data_type: &str) -> Option<Self> {
    match data_type.trim() {
      "BIGINT" => Some(Self::BigInt),
      "UBIGINT" => Some(Self::UBigInt),
      "DOUBLE" => Some(Self::Double),
      "VARCHAR" => Some(Self::Varchar),
      "BLOB" => Some(Self::Blob),
      "UNION(i BIGINT, r DOUBLE)" => Some(Self::TaggedNumeric),
      _ => None,
    }
  }

  pub(super) fn sql(self) -> &'static str {
    match self {
      Self::BigInt => "BIGINT",
      Self::UBigInt => "UBIGINT",
      Self::Double => "DOUBLE",
      Self::Varchar => "VARCHAR",
      Self::Blob => "BLOB",
      Self::TaggedNumeric => "UNION(i BIGINT, r DOUBLE)",
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Cell {
  Null,
  Integer(i64),
  /// binary64 bits, so a value round-trips without NaN or -0.0 comparison
  /// surprises.
  Real(u64),
  Text(String),
  Blob(Vec<u8>),
}

impl Cell {
  pub(super) fn describe(&self) -> &'static str {
    match self {
      Self::Null => "NULL",
      Self::Integer(_) => "an integer",
      Self::Real(_) => "a real",
      Self::Text(_) => "text",
      Self::Blob(_) => "a blob",
    }
  }

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

fn update_bytes(digest: &mut Sha256, bytes: &[u8]) {
  digest.update((bytes.len() as u64).to_le_bytes());
  digest.update(bytes);
}

/// A digest of a table's rows that does not depend on the order they are read
/// back in.
///
/// The candidate keeps a `__hv_source_ordinal` column, so its digest can be
/// ordered. The finalized schema deliberately does not carry that validation
/// column, and DuckDB does not promise a scan order, so the comparison here is
/// over the multiset of rows instead: each row's SHA-256 is added into a
/// 256-bit accumulator. Addition (not XOR) keeps duplicate rows visible, and
/// the row count is compared separately.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct RowMultisetDigest {
  limbs: [u64; 4],
  rows: u64,
}

impl RowMultisetDigest {
  pub(super) fn add_row(&mut self, cells: &[Cell]) {
    let mut digest = Sha256::new();
    digest.update((cells.len() as u64).to_le_bytes());
    for cell in cells {
      cell.update_digest(&mut digest);
    }
    let row: [u8; 32] = digest.finalize().into();
    let mut carry = 0_u64;
    for (index, limb) in self.limbs.iter_mut().enumerate() {
      let mut chunk = [0_u8; 8];
      chunk.copy_from_slice(&row[index * 8..index * 8 + 8]);
      let addend = u64::from_le_bytes(chunk);
      let (sum, overflow_a) = limb.overflowing_add(addend);
      let (sum, overflow_b) = sum.overflowing_add(carry);
      *limb = sum;
      carry = u64::from(overflow_a) + u64::from(overflow_b);
    }
    self.rows = self.rows.wrapping_add(1);
  }

  pub(super) fn rows(self) -> u64 {
    self.rows
  }

  pub(super) fn encode(self) -> String {
    use std::fmt::Write;

    let mut encoded = String::with_capacity(64);
    for limb in self.limbs.iter().rev() {
      write!(&mut encoded, "{limb:016x}").expect("writing to String cannot fail");
    }
    encoded
  }
}

pub(super) fn quote_identifier(identifier: &str) -> String {
  format!("\"{}\"", identifier.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_only_the_storage_types_the_candidate_can_produce() {
    assert_eq!(
      NativeColumnKind::parse("BIGINT"),
      Some(NativeColumnKind::BigInt)
    );
    assert_eq!(
      NativeColumnKind::parse("UNION(i BIGINT, r DOUBLE)"),
      Some(NativeColumnKind::TaggedNumeric)
    );
    assert_eq!(
      NativeColumnKind::parse("UBIGINT"),
      Some(NativeColumnKind::UBigInt)
    );
    assert_eq!(NativeColumnKind::parse("DECIMAL(18,3)"), None);
    assert_eq!(NativeColumnKind::parse("TIMESTAMP"), None);
  }

  #[test]
  fn multiset_digest_ignores_row_order_but_not_multiplicity_or_tags() {
    let a = vec![Cell::Integer(1), Cell::Text("x".to_owned())];
    let b = vec![Cell::Real(2.0_f64.to_bits()), Cell::Null];

    let mut forward = RowMultisetDigest::default();
    forward.add_row(&a);
    forward.add_row(&b);
    let mut reverse = RowMultisetDigest::default();
    reverse.add_row(&b);
    reverse.add_row(&a);
    assert_eq!(forward, reverse);
    assert_eq!(forward.rows(), 2);
    assert_eq!(forward.encode().len(), 64);

    let mut duplicated = RowMultisetDigest::default();
    duplicated.add_row(&a);
    duplicated.add_row(&a);
    assert_ne!(duplicated, forward);

    let mut integral_real = RowMultisetDigest::default();
    integral_real.add_row(&[Cell::Real(2.0_f64.to_bits())]);
    let mut integer = RowMultisetDigest::default();
    integer.add_row(&[Cell::Integer(2)]);
    assert_ne!(integral_real, integer);
  }

  #[test]
  fn multiset_digest_carries_across_limbs() {
    let mut all_ones = RowMultisetDigest {
      limbs: [u64::MAX; 4],
      rows: 0,
    };
    all_ones.add_row(&[Cell::Integer(7)]);
    let mut from_zero = RowMultisetDigest::default();
    from_zero.add_row(&[Cell::Integer(7)]);
    assert_ne!(all_ones, from_zero);
  }
}
