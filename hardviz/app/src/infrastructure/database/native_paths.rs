//! Resolves the native database, marker and spill paths beside the SQLite
//! source.
//!
//! App owns path resolution for the same reason it resolves `hv-database.db`
//! today: Core cannot see the bundle identifier or the app-data directory.
//! Everything a conversion touches lives in that one directory: the SQLite
//! source, the finalized/selected native file, the small selection marker
//! ([`AUTHORITY_MARKER_FILE_NAME`]), and - only while a conversion runs -
//! its spill and work directories, which name themselves with a shared
//! prefix `inspect_authority` already knows to recognize.

use std::path::PathBuf;

use hardviz_core::infrastructure::database::native_database::AuthorityPaths;

use crate::utils::file::get_app_data_dir;

/// The SQLite database file name, unchanged since before native conversion
/// existed.
pub const SQLITE_DATABASE_FILE_NAME: &str = "hv-database.db";

/// The native database file name. Chosen to sort beside the SQLite file and
/// to say what it is without implying a generation number: there is ever
/// only one native file per installation.
pub const NATIVE_DATABASE_FILE_NAME: &str = "hv-database.duckdb";

/// The directory every database lifecycle file lives in.
pub fn database_directory() -> PathBuf {
  get_app_data_dir(SQLITE_DATABASE_FILE_NAME)
    .parent()
    .expect("hv-database.db always resolves under the app data directory")
    .to_path_buf()
}

/// The source, native and marker paths, resolved beside each other in the
/// app data directory.
pub fn authority_paths() -> AuthorityPaths {
  AuthorityPaths::in_directory(
    &database_directory(),
    SQLITE_DATABASE_FILE_NAME,
    NATIVE_DATABASE_FILE_NAME,
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn authority_paths_names_the_source_and_native_files_beside_each_other() {
    let paths = authority_paths();
    assert_eq!(
      paths.source_database.file_name().unwrap(),
      SQLITE_DATABASE_FILE_NAME
    );
    assert_eq!(
      paths.native_database.file_name().unwrap(),
      NATIVE_DATABASE_FILE_NAME
    );
    assert_eq!(
      paths.source_database.parent(),
      paths.native_database.parent()
    );
    assert_eq!(paths.source_database.parent(), paths.marker.parent());
  }
}
