#![cfg(feature = "duckdb-archive")]

// These families do not call the process-wide `db::init`. Keeping them in a
// single integration target removes one compile/link unit while the tests
// that rely on one database path per binary remain separate targets.
mod native_support;

// These App-owned wrappers retain their unit tests in one Core target. The
// other DuckDB targets import the definitions-only siblings through
// `native_support`, so the 29 migration and 6 native-schema tests are not
// compiled once per integration binary.
#[path = "../../src-tauri/src/infrastructure/database/migration.rs"]
mod app_migrations;
#[path = "../../src-tauri/src/infrastructure/database/native_schema.rs"]
mod app_native_schema;

#[path = "duckdb/native.rs"]
mod native;
#[path = "duckdb/reconcile.rs"]
mod reconcile;
