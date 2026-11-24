#[path = "native_schema_definitions.rs"]
mod definitions;

#[cfg(test)]
pub(crate) use definitions::{
  IDENTITIES, NATIVE_ONLY_TABLES, TABLES, TIMESTAMPS, UPGRADES,
};
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

  /// ADR 0030: the baseline DDL stays version 1, every later version is one
  /// ordered step, and a native-only table is created by exactly one step and
  /// never shares a name with a SQLite-sourced table.
  #[test]
  fn upgrade_steps_end_at_the_schema_version_and_create_native_only_tables() {
    assert_eq!(
      UPGRADES.last().map_or(1, |step| step.to),
      NATIVE_SCHEMA_VERSION
    );
    assert_eq!(UPGRADES.first().map_or(1, |step| step.from), 1);
    for pair in UPGRADES.windows(2) {
      assert_eq!(pair[0].to, pair[1].from);
    }
    for table in NATIVE_ONLY_TABLES {
      assert!(!TABLES.contains(table), "{table}");
      assert!(!NATIVE_SCHEMA_SQL.contains(&format!("CREATE TABLE {table} ")));
      assert_eq!(
        UPGRADES
          .iter()
          .filter(|step| step.sql.contains(&format!("CREATE TABLE {table} ")))
          .count(),
        1,
        "{table}"
      );
    }
  }

  #[test]
  fn every_identity_names_a_declared_table() {
    assert_eq!(IDENTITIES.len(), 7);
    for identity in IDENTITIES {
      assert!(
        TABLES.contains(&identity.table) || NATIVE_ONLY_TABLES.contains(&identity.table),
        "{}",
        identity.table
      );
      assert_eq!(
        NATIVE_ONLY_TABLES.contains(&identity.table),
        identity.mode == NativeIdentityMode::NativeOnly,
        "{}",
        identity.table
      );
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
