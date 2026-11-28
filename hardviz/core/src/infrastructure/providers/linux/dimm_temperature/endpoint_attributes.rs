//! DIMM-route sysfs reads on top of the shared `providers::linux::sysfs`
//! checks: kernel-owned symlinks and attributes that are regular entries.

use std::fs;
use std::path::Path;

use super::super::sysfs::resolves_to;

/// `link` is itself a symlink and canonically resolves to `target`.
pub(super) fn links_to(link: &Path, target: &Path) -> bool {
  fs::symlink_metadata(link).is_ok_and(|metadata| metadata.is_symlink())
    && resolves_to(link, target)
}

/// Reads an attribute that is a regular entry of `directory`, never a link.
pub(super) fn read_hwmon_attribute(directory: &Path, name: &str) -> Option<String> {
  let path = directory.join(name);
  if !resolves_to(&path, &path) {
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
