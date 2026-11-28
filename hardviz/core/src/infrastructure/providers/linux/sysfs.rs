//! Domain-neutral sysfs topology helpers shared by the Linux hwmon providers.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// The `(device, inode)` pair that identifies one sysfs node across reads.
pub(crate) fn file_identity(metadata: &fs::Metadata) -> (u64, u64) {
  (metadata.dev(), metadata.ino())
}

/// Whether `name` is a canonical hwmon class entry: `hwmon` followed by one
/// or more ASCII digits.
pub(crate) fn is_hwmon_name(name: &str) -> bool {
  name.strip_prefix("hwmon").is_some_and(|index| {
    !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
  })
}

/// Whether `link` canonicalizes to exactly `expected`.
pub(crate) fn resolves_to(link: &Path, expected: &Path) -> bool {
  fs::canonicalize(link).is_ok_and(|target| target == expected)
}
