//! Hand a fully-written file its real name without ever replacing a file
//! already there.
//!
//! [`finalize`](super::finalize) and fresh installation
//! ([`super::selection::create_empty_native_database`]) both build a
//! finished database next to its destination in a work directory and then
//! have to publish it exactly once. They used a hard link for that: it is
//! free (no second copy of a database that can be gigabytes), and because the
//! link and the original already name the same complete, closed file, the
//! filesystem itself refuses to let two publishers both succeed - whichever
//! `hard_link` call loses a race gets `AlreadyExists`, and the destination it
//! sees is always the winner's *finished* file, never a partial one.
//!
//! Not every volume can make hard links. FAT, exFAT, some ReFS
//! configurations, and some network shares reject them outright, which
//! turned a fresh install into `FreshCreationFailed` on those volumes (#2272).
//! This module tries the hard link first - unchanged fast path on every
//! volume that supports it - and falls back to a copy-based publish that
//! keeps the same "second writer loses, and the loser never sees a partial
//! file" guarantee without needing hard-link support.

use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind};
use std::path::Path;

/// Publish `source` (a finished, already-synced file) at `destination`,
/// refusing to replace anything already there.
pub(super) fn publish_without_replacing(
  source: &Path,
  destination: &Path,
) -> io::Result<()> {
  publish_without_replacing_with(
    |source, destination| fs::hard_link(source, destination),
    source,
    destination,
  )
}

/// Same as [`publish_without_replacing`], with the hard-link step injected so
/// tests can force the fallback deterministically instead of needing an
/// actual FAT/exFAT/ReFS volume.
pub(super) fn publish_without_replacing_with(
  hard_link: impl Fn(&Path, &Path) -> io::Result<()>,
  source: &Path,
  destination: &Path,
) -> io::Result<()> {
  match hard_link(source, destination) {
    Ok(()) => Ok(()),
    Err(error) if hard_link_unsupported(&error) => {
      publish_by_copy_without_replacing(source, destination)
    }
    Err(error) => Err(error),
  }
}

/// Whether `error` means "this volume cannot create hard links at all", as
/// opposed to any other `hard_link` failure - destination already exists,
/// permission denied, source missing, and so on - which must keep failing the
/// publish rather than silently degrade into a slower fallback.
///
/// `ErrorKind::Unsupported` alone is necessary but not sufficient on Windows.
/// Checked against the standard library's own Windows error mapping
/// (`library/std/src/sys/io/error/windows.rs` in the 1.98 toolchain used to
/// build this crate): the only Win32 code it maps to `Unsupported` is
/// `ERROR_CALL_NOT_IMPLEMENTED` (120). What `CreateHardLinkW` actually returns
/// when the destination volume has no hard-link support -
/// `ERROR_INVALID_FUNCTION` (raw code 1) on FAT/exFAT, `ERROR_NOT_SUPPORTED`
/// (raw code 50) reported by some ReFS configurations - is not in that
/// mapping at all, so `std::io::Error::kind()` reports it as
/// `ErrorKind::Uncategorized` and the raw OS error has to be read directly
/// instead. `ErrorKind::CrossesDevices` (`ERROR_NOT_SAME_DEVICE`) is included
/// defensively: a network share can make the work directory and the
/// destination resolve to different volumes even though both sit under the
/// same profile directory.
fn hard_link_unsupported(error: &io::Error) -> bool {
  if matches!(
    error.kind(),
    ErrorKind::Unsupported | ErrorKind::CrossesDevices
  ) {
    return true;
  }
  #[cfg(windows)]
  {
    const ERROR_INVALID_FUNCTION: i32 = 1;
    const ERROR_NOT_SUPPORTED: i32 = 50;
    matches!(
      error.raw_os_error(),
      Some(ERROR_INVALID_FUNCTION) | Some(ERROR_NOT_SUPPORTED)
    )
  }
  #[cfg(not(windows))]
  {
    false
  }
}

/// The fallback publish: claim `destination` exclusively, copy `source`'s
/// bytes into it, and fsync before returning.
///
/// # Why not an atomic rename
///
/// `std::fs::rename` maps to `MoveFileExW` *with* `MOVEFILE_REPLACE_EXISTING`
/// on Windows and to POSIX `rename(2)` on Unix, both of which silently
/// replace an existing destination - exactly the clobber the hard link was
/// chosen to prevent. A true no-replace rename exists on some platforms
/// (`renameat2(RENAME_NOREPLACE)` on Linux, `MoveFileExW` without the replace
/// flag on Windows) but not portably through `std`, and this crate already
/// depends on the `windows` crate for other Win32 access, so it would be free
/// to reach for it here - but only for one platform, which would leave every
/// other target on this same fallback anyway. Publishing happens once per
/// finalize or fresh install, not on a hot path, so `create_new` gives the
/// same "second writer loses" atomicity `hard_link` did, using only `std`,
/// uniformly across platforms, at the cost of one file copy.
///
/// A destination left partially written by a failed copy is removed so a
/// retry is not permanently blocked by this attempt's own leftovers.
fn publish_by_copy_without_replacing(
  source: &Path,
  destination: &Path,
) -> io::Result<()> {
  let mut destination_file = OpenOptions::new()
    .write(true)
    .create_new(true)
    .open(destination)?;
  let result = (|| {
    let mut source_file = fs::File::open(source)?;
    io::copy(&mut source_file, &mut destination_file)?;
    destination_file.sync_all()
  })();
  if result.is_err() {
    drop(destination_file);
    let _ = fs::remove_file(destination);
  }
  result
}

#[cfg(test)]
mod tests {
  use super::*;

  fn unsupported_error() -> io::Error {
    io::Error::new(ErrorKind::Unsupported, "hard links are not supported here")
  }

  #[test]
  fn publishes_via_hard_link_when_supported() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    let destination = directory.path().join("destination");
    fs::write(&source, b"native database bytes").unwrap();

    publish_without_replacing(&source, &destination).unwrap();

    assert_eq!(fs::read(&destination).unwrap(), b"native database bytes");
  }

  #[test]
  fn falls_back_to_copy_when_hard_link_is_unsupported() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    let destination = directory.path().join("destination");
    fs::write(&source, b"native database bytes").unwrap();

    publish_without_replacing_with(
      |_, _| Err(unsupported_error()),
      &source,
      &destination,
    )
    .unwrap();

    assert_eq!(fs::read(&destination).unwrap(), b"native database bytes");
    // The fallback must not consume the source the way a real hard link
    // leaves it - callers (finalize, fresh install) remove their own work
    // file afterward.
    assert_eq!(fs::read(&source).unwrap(), b"native database bytes");
  }

  #[cfg(windows)]
  #[test]
  fn falls_back_for_the_raw_windows_error_codes_fat_and_refs_actually_report() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    let destination = directory.path().join("destination");
    fs::write(&source, b"native database bytes").unwrap();

    for raw_code in [1, 50] {
      fs::remove_file(&destination).ok();
      publish_without_replacing_with(
        move |_, _| Err(io::Error::from_raw_os_error(raw_code)),
        &source,
        &destination,
      )
      .unwrap();
      assert_eq!(fs::read(&destination).unwrap(), b"native database bytes");
    }
  }

  #[test]
  fn fallback_does_not_overwrite_an_existing_destination() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    let destination = directory.path().join("destination");
    fs::write(&source, b"new bytes").unwrap();
    fs::write(&destination, b"already published").unwrap();

    let error = publish_without_replacing_with(
      |_, _| Err(unsupported_error()),
      &source,
      &destination,
    )
    .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::AlreadyExists);
    assert_eq!(fs::read(&destination).unwrap(), b"already published");
  }

  #[test]
  fn other_hard_link_failures_are_not_treated_as_missing_support() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    let destination = directory.path().join("destination");
    fs::write(&source, b"native database bytes").unwrap();

    let error = publish_without_replacing_with(
      |_, _| Err(io::Error::new(ErrorKind::PermissionDenied, "denied")),
      &source,
      &destination,
    )
    .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::PermissionDenied);
    assert!(
      !destination.exists(),
      "a non-fallback error must not publish anything"
    );
  }
}
