//! RAII wrapper around a Win32 named mutex (Windows).
//!
//! The PawnIO ecosystem coordinates hardware read transactions between
//! concurrent monitoring tools through named mutexes (`Global\Access_PCI`
//! for PCI/SMN, `Global\Access_ISABUS.HTP.Method` for ISA/Super I/O).
//! The PawnIO modules acquire nothing themselves — every relevant ioctl
//! documents that the **caller** must hold the mutex — so clients in
//! this crate wrap each call in [`NamedMutex::acquire`]. See
//! `docs/specs/sensors/pawnio-interface.md` (rev 2), "Mutex conventions".
//!
//! Acquisition always uses a bounded timeout, and a timeout means the
//! caller skips its sample: proceeding unlocked is never an option.

use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
  CreateMutexW, ReleaseMutex, WaitForSingleObject,
};
use windows::core::PCWSTR;

/// An open handle to a named Win32 mutex, created (or opened, when
/// another process created it first) without initial ownership.
pub struct NamedMutex {
  handle: HANDLE,
}

impl NamedMutex {
  /// Create or open the named mutex. Returns `None` when the object
  /// cannot be created/opened (e.g. an incompatible ACL on the existing
  /// object) — callers must then skip the protected operation.
  pub fn create(name: &str) -> Option<Self> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let handle = unsafe { CreateMutexW(None, false, PCWSTR(wide.as_ptr())) }.ok()?;
    Some(Self { handle })
  }

  /// Acquire the mutex with a bounded timeout. Returns `None` on
  /// timeout or wait failure; the caller must treat that as a skipped
  /// sample and never proceed unlocked.
  ///
  /// `WAIT_ABANDONED` still grants ownership (the previous holder died
  /// while owning the mutex). The transactions guarded here are
  /// self-contained reads, so an abandoned mutex is safe to treat as
  /// acquired.
  pub fn acquire(&self, timeout: Duration) -> Option<NamedMutexGuard<'_>> {
    let millis = timeout.as_millis().min(u32::MAX as u128) as u32;
    let result = unsafe { WaitForSingleObject(self.handle, millis) };
    (result == WAIT_OBJECT_0 || result == WAIT_ABANDONED)
      .then_some(NamedMutexGuard { mutex: self })
  }
}

impl Drop for NamedMutex {
  fn drop(&mut self) {
    unsafe {
      let _ = CloseHandle(self.handle);
    }
  }
}

/// Ownership of an acquired [`NamedMutex`]; releases on drop.
pub struct NamedMutexGuard<'a> {
  mutex: &'a NamedMutex,
}

impl Drop for NamedMutexGuard<'_> {
  fn drop(&mut self) {
    unsafe {
      let _ = ReleaseMutex(self.mutex.handle);
    }
  }
}
