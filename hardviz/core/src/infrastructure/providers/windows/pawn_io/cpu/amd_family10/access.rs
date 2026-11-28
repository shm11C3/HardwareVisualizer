//! The `AMDFamily10` module operations the Tctl paths use, the
//! `Global\Access_PCI` lock, and the D18F3x00 identity gate.

use std::time::Duration;

use super::super::super::client::{
  ACCESS_PCI_MUTEX, NamedMutex, PawnIoClient, PawnIoError,
};
use super::family_15h_16h::SMU_REPORTED_TEMPERATURE_CONTROL;
use super::reported_temperature::TctlDecodeError;

/// D18F3x00, the device/vendor identity register of node 0 function 3.
const D18F3X00: u64 = 0x00;
/// D18F3xA4, Reported Temperature Control (map A).
const D18F3XA4: u64 = 0xA4;
/// The only node the specification reads.
const NODE_0: u64 = 0;
const ACCESS_PCI_TIMEOUT: Duration = Duration::from_millis(50);

/// Why an admitted path produced no Tctl.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TctlError {
  #[error("AMD D18F3x00 identity read failed: {0}")]
  IdentityRead(PawnIoError),
  #[error(
    "AMD D18F3x00 identity 0x{actual:08X} does not match expected 0x{expected:08X}"
  )]
  IdentityMismatch { expected: u32, actual: u32 },
  #[error("AMD Tctl register read failed: {0}")]
  Read(PawnIoError),
  #[error(transparent)]
  Decode(#[from] TctlDecodeError),
}

/// The module operations this path uses, each with its fixed argument.
pub(crate) trait AmdFamily10Access {
  type PciLock;

  /// `ioctl_read_miscctl(0, 0x00)`.
  fn read_identity(&self) -> Result<u64, PawnIoError>;
  /// `ioctl_read_miscctl(0, 0xA4)`.
  fn read_d18f3xa4(&self) -> Result<u64, PawnIoError>;
  /// Take `Global\Access_PCI` with a bounded wait; dropping the guard
  /// releases it.
  fn lock_pci(&self) -> Result<Self::PciLock, PawnIoError>;
  /// `ioctl_read_smu(0xD8200CA4)`. The caller holds the PCI lock.
  fn read_smu_reported_temperature(&self) -> Result<u64, PawnIoError>;
}

impl AmdFamily10Access for PawnIoClient {
  type PciLock = NamedMutex;

  fn read_identity(&self) -> Result<u64, PawnIoError> {
    self.read_amd_family10_miscctl(NODE_0, D18F3X00)
  }

  fn read_d18f3xa4(&self) -> Result<u64, PawnIoError> {
    self.read_amd_family10_miscctl(NODE_0, D18F3XA4)
  }

  fn lock_pci(&self) -> Result<Self::PciLock, PawnIoError> {
    Ok(NamedMutex::acquire(ACCESS_PCI_MUTEX, ACCESS_PCI_TIMEOUT)?)
  }

  fn read_smu_reported_temperature(&self) -> Result<u64, PawnIoError> {
    self.read_amd_family10_smu(SMU_REPORTED_TEMPERATURE_CONTROL)
  }
}

/// The identity gate, run once per opened handle before any Tctl read.
/// `expected` is the D18F3x00 value (DeviceID 31:16, VendorID 15:0) of the
/// selected group.
pub(crate) fn admit_identity(
  access: &impl AmdFamily10Access,
  expected: u32,
) -> Result<(), TctlError> {
  let actual = access.read_identity().map_err(TctlError::IdentityRead)? as u32;
  if actual != expected {
    return Err(TctlError::IdentityMismatch { expected, actual });
  }
  Ok(())
}

/// A scripted module handle shared by the Tctl path tests.
#[cfg(test)]
pub(super) mod fake {
  use std::cell::{Cell, RefCell};
  use std::rc::Rc;

  use super::super::super::super::client::MutexError;
  use super::*;

  pub(crate) fn execute_error(function: &'static str) -> PawnIoError {
    PawnIoError::Execute {
      function,
      // STATUS_ACCESS_DENIED as an NTSTATUS-derived HRESULT.
      hresult: 0xD000_0022_u32 as i32,
    }
  }

  pub(crate) fn timeout() -> PawnIoError {
    PawnIoError::Mutex(MutexError::Timeout {
      name: ACCESS_PCI_MUTEX,
    })
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub(crate) enum Call {
    Identity,
    D18F3xA4,
    LockPci,
    Smu { lock_held: bool },
  }

  pub(crate) struct Guard(Rc<Cell<bool>>);

  impl Drop for Guard {
    fn drop(&mut self) {
      self.0.set(false);
    }
  }

  /// A scripted module handle that records every call and tracks whether
  /// the PCI lock is held.
  pub(crate) struct FakeModule {
    identity: Result<u64, PawnIoError>,
    register: Result<u64, PawnIoError>,
    pub(crate) lock_times_out: bool,
    pub(crate) held: Rc<Cell<bool>>,
    calls: RefCell<Vec<Call>>,
  }

  impl FakeModule {
    pub(crate) fn new(
      identity: Result<u64, PawnIoError>,
      register: Result<u64, PawnIoError>,
    ) -> Self {
      Self {
        identity,
        register,
        lock_times_out: false,
        held: Rc::new(Cell::new(false)),
        calls: RefCell::new(Vec::new()),
      }
    }

    pub(crate) fn reading(register: u32) -> Self {
      Self::new(Ok(0), Ok(u64::from(register)))
    }

    pub(crate) fn calls(&self) -> Vec<Call> {
      self.calls.borrow().clone()
    }
  }

  impl AmdFamily10Access for FakeModule {
    type PciLock = Guard;

    fn read_identity(&self) -> Result<u64, PawnIoError> {
      self.calls.borrow_mut().push(Call::Identity);
      self.identity.clone()
    }

    fn read_d18f3xa4(&self) -> Result<u64, PawnIoError> {
      self.calls.borrow_mut().push(Call::D18F3xA4);
      self.register.clone()
    }

    fn lock_pci(&self) -> Result<Self::PciLock, PawnIoError> {
      self.calls.borrow_mut().push(Call::LockPci);
      if self.lock_times_out {
        return Err(timeout());
      }
      assert!(!self.held.get(), "the PCI lock is taken twice");
      self.held.set(true);
      Ok(Guard(Rc::clone(&self.held)))
    }

    fn read_smu_reported_temperature(&self) -> Result<u64, PawnIoError> {
      self.calls.borrow_mut().push(Call::Smu {
        lock_held: self.held.get(),
      });
      self.register.clone()
    }
  }
}

#[cfg(test)]
mod tests {
  use super::fake::{Call, FakeModule, execute_error};
  use super::*;

  /// D18F3x00 of Family 15h models 30h-3Fh (BKDG 49125 p. 442, spec rev 2).
  const KAVERI_IDENTITY: u32 = 0x141D_1022;
  /// D18F3x00 of Family 15h models 00h-0Fh (BKDG 42301 p. 374, spec rev 2).
  const OROCHI_IDENTITY: u32 = 0x1603_1022;

  #[test]
  fn identity_gate_accepts_only_the_group_value_in_the_low_32_bits() {
    let matched = FakeModule::new(Ok(0x141D_1022), Ok(0));
    assert_eq!(admit_identity(&matched, KAVERI_IDENTITY), Ok(()));
    assert_eq!(matched.calls(), vec![Call::Identity]);

    let upper_cell_bits = FakeModule::new(Ok(0xFFFF_FFFF_141D_1022), Ok(0));
    assert_eq!(admit_identity(&upper_cell_bits, KAVERI_IDENTITY), Ok(()));

    // Another group's D18F3x00 (15h 10h-1Fh) on a 30h-3Fh CPU.
    let mismatched = FakeModule::new(Ok(0x1403_1022), Ok(0));
    assert_eq!(
      admit_identity(&mismatched, KAVERI_IDENTITY),
      Err(TctlError::IdentityMismatch {
        expected: 0x141D_1022,
        actual: 0x1403_1022,
      })
    );
  }

  #[test]
  fn identity_gate_rejects_a_failed_read_without_using_its_output() {
    let failed = FakeModule::new(Err(execute_error("ioctl_read_miscctl")), Ok(0));

    assert_eq!(
      admit_identity(&failed, OROCHI_IDENTITY),
      Err(TctlError::IdentityRead(execute_error("ioctl_read_miscctl")))
    );
  }

  #[test]
  fn failure_text_does_not_read_as_a_missing_or_misconfigured_component() {
    use crate::models::ExternalComponentReasonKind;
    use crate::models::external_component_guidance::classify_external_component_reason;

    for error in [
      TctlError::IdentityMismatch {
        expected: 0x1603_1022,
        actual: 0x1403_1022,
      },
      TctlError::Decode(TctlDecodeError::FailedTransaction(0)),
      TctlError::Decode(TctlDecodeError::NotHardwareTctl(0b11)),
      TctlError::Decode(TctlDecodeError::Implausible { cur_tmp: 961 }),
    ] {
      assert_eq!(
        classify_external_component_reason(&error.to_string()),
        ExternalComponentReasonKind::Failed,
        "{error}"
      );
    }
  }
}
