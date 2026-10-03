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
//! volume that supports it - and falls back to a copy staged under a
//! temporary name and published with a no-replace rename, which keeps the
//! same "the destination is either absent or one complete file, and a second
//! publisher always loses" guarantee without needing hard-link support.

use std::fs;
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

/// The fallback publish: copy `source`'s bytes into a temporary file beside
/// `destination`, fsync it while it still has no name anyone else can open,
/// and only then give it the destination name with a no-replace rename.
///
/// # Why the copy must stay unnamed until it is complete
///
/// `destination` must never be observable half-written. Claiming the
/// destination name first (for example with `create_new`) and copying into it
/// afterward would let a crash, a power loss, or a concurrent reader between
/// those two steps see or keep a truncated file at the name every other part
/// of this codebase treats as either absent or a complete, verified database -
/// `inspect_authority` has no state for "present but partial", so that would
/// turn into a stuck `ActionRequired` exactly like the bug this fallback
/// fixes. Staging the copy under a temporary name and publishing it with one
/// rename keeps the same "the destination is either absent or complete"
/// invariant the hard link gave: nothing ever names an incomplete file
/// `destination`.
///
/// # Why a rename, and why `tempfile`'s
///
/// `std::fs::rename` maps to `MoveFileExW` *with* `MOVEFILE_REPLACE_EXISTING`
/// on Windows and to POSIX `rename(2)` on Unix, both of which silently
/// replace an existing destination - exactly the clobber the hard link was
/// chosen to prevent. [`NamedTempFile::persist_noclobber`] is the same
/// no-replace rename this module already trusts for the authority marker
/// (see [`super::selection::write_marker_atomically_noclobber`]): a true
/// `renameat2(RENAME_NOREPLACE)` on Linux and macOS, `MoveFileExW` without the
/// replace flag on Windows, refusing with `AlreadyExists` rather than
/// clobbering when `destination` appeared in the meantime.
fn publish_by_copy_without_replacing(
  source: &Path,
  destination: &Path,
) -> io::Result<()> {
  let directory = destination.parent().ok_or_else(|| {
    io::Error::new(
      ErrorKind::InvalidInput,
      "publish destination must have a parent directory",
    )
  })?;
  let mut staged = tempfile::NamedTempFile::new_in(directory)?;
  let mut source_file = fs::File::open(source)?;
  io::copy(&mut source_file, staged.as_file_mut())?;
  staged.as_file().sync_all()?;
  staged
    .persist_noclobber(destination)
    .map(|_file| ())
    .map_err(|error| error.error)
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
  fn a_failed_fallback_copy_never_names_destination_and_leaves_no_stray_file() {
    let directory = tempfile::tempdir().unwrap();
    // A source that cannot be opened makes the copy fail after the temporary
    // file has already been created beside `destination`, exercising the
    // property that matters most: `destination` must never be observable
    // half-written, and a failed attempt must not block a retry with its own
    // leftovers.
    let source = directory.path().join("missing-source");
    let destination = directory.path().join("destination");

    let error = publish_without_replacing_with(
      |_, _| Err(unsupported_error()),
      &source,
      &destination,
    )
    .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::NotFound);
    assert!(
      !destination.exists(),
      "destination must not exist until the copy is complete"
    );
    assert!(
      fs::read_dir(directory.path()).unwrap().next().is_none(),
      "a failed fallback must not leave a partial file behind"
    );
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
