//! sysfs link and attribute helpers shared by the Linux DIMM routes.

use std::fs::{self, Metadata};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub(super) fn file_identity(metadata: &Metadata) -> (u64, u64) {
  (metadata.dev(), metadata.ino())
}

/// `link` is itself a symlink and canonically resolves to `target`.
pub(super) fn links_to(link: &Path, target: &Path) -> bool {
  fs::symlink_metadata(link).is_ok_and(|metadata| metadata.is_symlink())
    && fs::canonicalize(link).is_ok_and(|path| path == target)
}

/// Reads an attribute that is a regular entry of `directory`, never a link.
pub(super) fn read_hwmon_attribute(directory: &Path, name: &str) -> Option<String> {
  let path = directory.join(name);
  if fs::canonicalize(&path).ok()? != path {
    return None;
  }
  attribute(directory, name)
}

pub(super) fn attribute(directory: &Path, name: &str) -> Option<String> {
  let mut value = fs::read_to_string(directory.join(name)).ok()?;
  if value.ends_with('\n') {
    value.pop();
  }
  Some(value)
}
