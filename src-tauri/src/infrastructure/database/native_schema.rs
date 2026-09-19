#[path = "native_schema_definitions.rs"]
mod definitions;

#[cfg(test)]
pub(crate) use definitions::{IDENTITIES, TABLES, TIMESTAMPS};
#[allow(unused_imports)]
pub use definitions::{NATIVE_SCHEMA_SQL, NATIVE_SCHEMA_VERSION, get_native_schema};
#[cfg(test)]
use hardviz_core::infrastructure::database::native_database::NativeIdentityMode;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn stable_schema_names_every_current_domain_table_once() {
    assert_eq!(TABLES.len(), 15);
    for table in TABLES {
      assert_eq!(
        NATIVE_SCHEMA_SQL
          .matches(&format!("CREATE TABLE {table} "))
          .count(),
        1,
        "{table}"
      );
    }
    assert_eq!(
      NATIVE_SCHEMA_SQL.matches("CREATE TABLE ").count(),
      TABLES.len()
    );
  }

  #[test]
  fn storage_devices_is_created_before_the_table_referencing_it() {
    let parent = TABLES.iter().position(|t| *t == "storage_devices").unwrap();
    let child = TABLES
      .iter()
      .position(|t| *t == "storage_health_daily_records")
      .unwrap();
    assert!(parent < child);
  }

  #[test]
  fn only_writer_proven_mixed_measurements_use_tagged_unions() {
    assert_eq!(
      NATIVE_SCHEMA_SQL
        .matches("UNION(i BIGINT, r DOUBLE)")
        .count(),
      10
    );
  }

  #[test]
  fn derived_epoch_columns_stay_nullable_so_an_unconvertible_stamp_reads_absent() {
    assert_eq!(TIMESTAMPS.len(), 4);
    assert_eq!(
      NATIVE_SCHEMA_SQL
        .matches("__hv_timestamp_epoch_ms BIGINT\n")
        .count(),
      4
    );
    assert!(!NATIVE_SCHEMA_SQL.contains("__hv_timestamp_epoch_ms BIGINT NOT NULL"));
  }

  #[test]
  fn mutable_domain_keys_have_native_constraints() {
    for constraint in [
      "UNIQUE(device_id, date)",
      "PRIMARY KEY (date, source)",
      "PRIMARY KEY (date, source, band)",
      "PRIMARY KEY (date, source, fan_source, band)",
      "CHECK (id = 1)",
    ] {
      assert!(
        NATIVE_SCHEMA_SQL.contains(constraint),
        "missing {constraint}"
      );
    }
  }

  #[test]
  fn every_identity_names_a_declared_table() {
    assert_eq!(IDENTITIES.len(), 6);
    for identity in IDENTITIES {
      assert!(TABLES.contains(&identity.table), "{}", identity.table);
    }
    let autoincrement = IDENTITIES
      .iter()
      .filter(|identity| {
        matches!(identity.mode, NativeIdentityMode::AutoIncrement { .. })
      })
      .count();
    assert_eq!(autoincrement, 3);
  }
}
