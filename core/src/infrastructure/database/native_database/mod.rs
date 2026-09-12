//! Explicit native DuckDB access for a finalized, unselected database.
//!
//! SQLite remains authoritative. Nothing here is reached by the running
//! application: finalization builds a separate file, the owner opens it only
//! when a caller asks, and the per-family functions run beside - never instead
//! of - their SQLite counterparts until App lifecycle selection is implemented.

mod cell;
mod epoch;
mod error;
mod finalize;
mod paging;
pub mod process_stats;
mod runtime;
mod schema;

use std::path::Path;

use duckdb::Connection;

pub use error::NativeDatabaseError;
pub use finalize::{
  NativeFinalizationReport, NativeTableReport, finalize_candidate_database,
};
pub use runtime::{
  NativeCancellation, NativeConnectionContext, NativeDatabase, NativeDatabaseOptions,
  NativeTransactionContext,
};
pub use schema::{
  NativeIdentity, NativeIdentityMode, NativeSchemaDefinition, NativeTimestampColumn,
};

/// Point one DuckDB instance at its own spill directory and close it to the
/// filesystem afterwards.
///
/// DuckDB rejects `temp_directory` changes once external access is disabled, so
/// the owned spill path has to be set first. Independent instances must not
/// share a spill directory.
fn configure_spill(
  connection: &Connection,
  spill: &Path,
) -> Result<(), NativeDatabaseError> {
  let spill = spill.to_str().ok_or_else(|| NativeDatabaseError::Worker {
    message: format!("spill path is not valid UTF-8: {}", spill.display()),
  })?;
  connection
    .execute_batch(&format!(
      "SET temp_directory = '{}'; SET enable_external_access = false",
      spill.replace('\'', "''")
    ))
    .map_err(|error| NativeDatabaseError::duckdb("configure the spill directory", error))
}
