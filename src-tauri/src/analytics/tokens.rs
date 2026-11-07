//! Installation Tokens: rotating per-UTC-day and per-UTC-month values
//! derived from the local seed, which itself never leaves the machine.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use super::period::UtcMonth;

const DAY_LABEL: &str = "hardviz-analytics/v1/day/";
const MONTH_LABEL: &str = "hardviz-analytics/v1/month/";
const SEED_BYTES: usize = 32;
const TOKEN_HEX_CHARS: usize = 32;

/// The persisted analytics seed. `Debug` never prints it, so it cannot reach
/// a log through a derived `Debug` of a containing type.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Seed([u8; SEED_BYTES]);

impl std::fmt::Debug for Seed {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("Seed(..)")
  }
}

impl Seed {
  /// Parses the settings representation: 32 bytes as lowercase hex.
  pub(crate) fn from_hex(value: &str) -> Option<Self> {
    if value.len() != SEED_BYTES * 2 {
      return None;
    }
    let mut bytes = [0u8; SEED_BYTES];
    for (byte, pair) in bytes.iter_mut().zip(value.as_bytes().as_chunks::<2>().0) {
      *byte = (hex_digit(pair[0])? << 4) | hex_digit(pair[1])?;
    }
    Some(Self(bytes))
  }

  pub(crate) fn day_token(&self, day: chrono::NaiveDate) -> String {
    self.token(DAY_LABEL, &day.format("%Y-%m-%d").to_string())
  }

  pub(crate) fn month_token(&self, month: UtcMonth) -> String {
    self.token(MONTH_LABEL, &month.to_string())
  }

  fn token(&self, label: &str, period: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(&self.0)
      .expect("HMAC-SHA-256 accepts any key length");
    mac.update(label.as_bytes());
    mac.update(period.as_bytes());
    let digest = mac.finalize().into_bytes();
    let mut token: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    token.truncate(TOKEN_HEX_CHARS);
    token
  }
}

fn hex_digit(digit: u8) -> Option<u8> {
  match digit {
    b'0'..=b'9' => Some(digit - b'0'),
    b'a'..=b'f' => Some(digit - b'a' + 10),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const SEED: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

  fn day(value: &str) -> chrono::NaiveDate {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
  }

  /// Expected values were computed independently with Node's
  /// `crypto.createHmac("sha256", seed).update(label + period)`.
  #[test]
  fn tokens_match_independent_hmac_vectors() {
    let seed = Seed::from_hex(SEED).unwrap();
    assert_eq!(
      seed.day_token(day("2026-11-03")),
      "2ca1f2b7672934d3f30ff1f906d5ff4d"
    );
    assert_eq!(
      seed.month_token(UtcMonth::of(day("2026-11-03"))),
      "b6a3ae032d2ace6d4dbe487bba0ed1ab"
    );
  }

  #[test]
  fn day_and_month_labels_give_independent_tokens() {
    let seed = Seed::from_hex(SEED).unwrap();
    let tokens = [
      seed.day_token(day("2026-11-03")),
      seed.day_token(day("2026-11-04")),
      seed.month_token(UtcMonth::of(day("2026-11-03"))),
      seed.month_token(UtcMonth::of(day("2026-12-01"))),
    ];
    for (index, token) in tokens.iter().enumerate() {
      assert_eq!(token.len(), TOKEN_HEX_CHARS);
      assert!(!tokens[index + 1..].contains(token));
    }
  }

  #[test]
  fn malformed_seeds_are_rejected() {
    assert!(Seed::from_hex(&SEED.to_uppercase()).is_none());
    assert!(Seed::from_hex(&SEED[..62]).is_none());
    assert!(Seed::from_hex(&format!("{}zz", &SEED[..62])).is_none());
  }

  #[test]
  fn debug_output_hides_the_seed() {
    assert_eq!(format!("{:?}", Seed::from_hex(SEED).unwrap()), "Seed(..)");
  }
}
