//! UTC day and month periods as they appear on the wire and in settings.

use chrono::{Datelike, NaiveDate};
use serde::{Serialize, Serializer};

const DAY_FORMAT: &str = "%Y-%m-%d";

/// A UTC calendar month. Ordered chronologically by field order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct UtcMonth {
  year: i32,
  month: u32,
}

impl UtcMonth {
  pub(crate) fn of(day: NaiveDate) -> Self {
    Self {
      year: day.year(),
      month: day.month(),
    }
  }

  /// Parses `YYYY-MM`.
  pub(crate) fn parse(value: &str) -> Option<Self> {
    let first = NaiveDate::parse_from_str(&format!("{value}-01"), DAY_FORMAT).ok()?;
    let month = Self::of(first);
    (month.to_string() == value).then_some(month)
  }

  /// The first day of the month, the form the settings writer takes.
  pub(crate) fn first_day(self) -> NaiveDate {
    NaiveDate::from_ymd_opt(self.year, self.month, 1)
      .expect("a UtcMonth is a valid month")
  }
}

impl std::fmt::Display for UtcMonth {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:04}-{:02}", self.year, self.month)
  }
}

impl Serialize for UtcMonth {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(self)
  }
}

/// Parses a `YYYY-MM-DD` UTC day.
pub(crate) fn parse_day(value: &str) -> Option<NaiveDate> {
  NaiveDate::parse_from_str(value, DAY_FORMAT)
    .ok()
    .filter(|day| day.format(DAY_FORMAT).to_string() == value)
}

/// Serializes a UTC day as `YYYY-MM-DD`.
pub(crate) fn serialize_day<S: Serializer>(
  day: &NaiveDate,
  serializer: S,
) -> Result<S::Ok, S::Error> {
  serializer.collect_str(&day.format(DAY_FORMAT))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn months_round_trip_and_order_chronologically() {
    let november = UtcMonth::parse("2026-11").unwrap();
    let december = UtcMonth::parse("2026-12").unwrap();
    let january = UtcMonth::parse("2027-01").unwrap();
    assert_eq!(november.to_string(), "2026-11");
    assert!(november < december && december < january);
    assert_eq!(november.first_day(), parse_day("2026-11-01").unwrap());
  }

  #[test]
  fn malformed_periods_are_rejected() {
    assert!(UtcMonth::parse("2026-13").is_none());
    assert!(UtcMonth::parse("2026-1").is_none());
    assert!(parse_day("2026-11-3").is_none());
    assert!(parse_day("2026-02-30").is_none());
  }
}
