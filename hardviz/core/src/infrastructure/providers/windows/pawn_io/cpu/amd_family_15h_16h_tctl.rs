//! AMD Family 15h/16h control temperature (Tctl) through the signed
//! `AMDFamily10` module.
//!
//! Implemented clean-room from
//! `docs/specs/sensors/cpu-amd-family-15h-16h-tctl.md` rev 2 and the
//! `AMDFamily10` contract in `docs/specs/sensors/pawnio-interface.md` rev 8.
//! Only `ioctl_read_miscctl` (node 0, offsets `0x00` and `0xA4`) and
//! `ioctl_read_smu` (address `0xD8200CA4`) are ever invoked. The module's
//! TSC-measurement ioctl has upstream freeze reports and its C-state ioctl
//! writes an I/O port without a documented mutex, so neither has a caller
//! here; adding one needs a specification revision.

use std::time::Duration;

use super::super::client::{ACCESS_PCI_MUTEX, NamedMutex, PawnIoClient, PawnIoError};
use super::identity::cpuid_leaf;

/// D18F3x00, the device/vendor identity register of node 0 function 3.
const D18F3X00: u64 = 0x00;
/// D18F3xA4, Reported Temperature Control (map A).
const D18F3XA4: u64 = 0xA4;
/// D0F0xBC_xD820_0CA4, Reported Temperature Control (map B).
const SMU_REPORTED_TEMPERATURE_CONTROL: u64 = 0xD820_0CA4;
/// The only node the specification reads.
const NODE_0: u64 = 0;
const ACCESS_PCI_TIMEOUT: Duration = Duration::from_millis(50);
/// `CurTmp` counts eighths of a degree; 120 is the plausibility ceiling.
const MAX_PLAUSIBLE_CUR_TMP: u16 = 120 * 8;

/// Where a group's Tctl lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TctlRegisterMap {
  /// Map A: D18F3xA4 through `ioctl_read_miscctl`. No caller mutex.
  D18F3xA4,
  /// Map B: D0F0xBC_xD820_0CA4 through `ioctl_read_smu` under
  /// `Global\Access_PCI`. These groups have no D18F3xA4.
  SmuD8200Ca4,
}

/// One model group of the specification's Detection table.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TctlGroup {
  family: u32,
  first_model: u32,
  last_model: u32,
  pub(crate) map: TctlRegisterMap,
  /// The D18F3x00 value (DeviceID 31:16, VendorID 15:0) of this group.
  identity: u32,
  /// CPUID Fn8000_0001_EBX[31:28] `PkgType` values this group admits, or
  /// `None` when the group has no package-type gate.
  package_types: Option<&'static [u32]>,
}

/// AM3r2 (`0001b`) and C32r1 (`0101b`), the single-node packages of Family
/// 15h models 00h-0Fh (BKDG 42301 p. 482). G34r1 (`0011b`) is always
/// dual-node, and device 18h is not documented to be its internal node 0,
/// the only node that reports the processor temperature (pp. 100, 393).
const SINGLE_NODE_PACKAGE_TYPES: &[u32] = &[0b0001, 0b0101];

const TCTL_GROUPS: [TctlGroup; 7] = [
  // Family 15h models 00h-0Fh (BKDG 42301 p. 374).
  TctlGroup {
    family: 0x15,
    first_model: 0x00,
    last_model: 0x0F,
    map: TctlRegisterMap::D18F3xA4,
    identity: 0x1603_1022,
    package_types: Some(SINGLE_NODE_PACKAGE_TYPES),
  },
  // Family 15h models 10h-1Fh (BKDG 42300 p. 419).
  TctlGroup {
    family: 0x15,
    first_model: 0x10,
    last_model: 0x1F,
    map: TctlRegisterMap::D18F3xA4,
    identity: 0x1403_1022,
    package_types: None,
  },
  // Family 15h models 30h-3Fh (BKDG 49125 p. 442).
  TctlGroup {
    family: 0x15,
    first_model: 0x30,
    last_model: 0x3F,
    map: TctlRegisterMap::D18F3xA4,
    identity: 0x141D_1022,
    package_types: None,
  },
  // Family 15h models 60h-6Fh (BKDG 50742 p. 478).
  TctlGroup {
    family: 0x15,
    first_model: 0x60,
    last_model: 0x6F,
    map: TctlRegisterMap::SmuD8200Ca4,
    identity: 0x1573_1022,
    package_types: None,
  },
  // Family 15h models 70h-7Fh (BKDG 55072 p. 451).
  TctlGroup {
    family: 0x15,
    first_model: 0x70,
    last_model: 0x7F,
    map: TctlRegisterMap::SmuD8200Ca4,
    identity: 0x15B3_1022,
    package_types: None,
  },
  // Family 16h models 00h-0Fh (BKDG 48751 p. 395).
  TctlGroup {
    family: 0x16,
    first_model: 0x00,
    last_model: 0x0F,
    map: TctlRegisterMap::D18F3xA4,
    identity: 0x1533_1022,
    package_types: None,
  },
  // Family 16h models 30h-3Fh (BKDG 52740 p. 437).
  TctlGroup {
    family: 0x16,
    first_model: 0x30,
    last_model: 0x3F,
    map: TctlRegisterMap::D18F3xA4,
    identity: 0x1583_1022,
    package_types: None,
  },
];

/// Whether the effective family belongs to this specification, so its
/// unsupported models are not handed to the Zen path.
pub(crate) const fn is_family_15h_16h(family: u32) -> bool {
  matches!(family, 0x15 | 0x16)
}

/// The group of an AMD processor's effective family and model, if the
/// specification defines its register map.
pub(crate) fn select_tctl_group(family: u32, model: u32) -> Option<&'static TctlGroup> {
  TCTL_GROUPS.iter().find(|group| {
    group.family == family && (group.first_model..=group.last_model).contains(&model)
  })
}

impl TctlGroup {
  /// The package-type gate. `package_type` is read only for a group that
  /// has the gate; an unreadable CPUID leaf is not admitted.
  pub(crate) fn admits_package_type(
    &self,
    package_type: impl FnOnce() -> Option<u32>,
  ) -> Result<(), Option<u32>> {
    let Some(allowed) = self.package_types else {
      return Ok(());
    };
    match package_type() {
      Some(value) if allowed.contains(&value) => Ok(()),
      other => Err(other),
    }
  }
}

/// CPUID Fn8000_0001_EBX[31:28] `PkgType`.
pub(crate) fn cpuid_package_type() -> Option<u32> {
  cpuid_leaf(0x8000_0001).map(|leaf| leaf.ebx >> 28)
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TctlDecodeError {
  #[error("AMD Tctl register read 0x{0:08X}, a failed transaction")]
  FailedTransaction(u32),
  #[error("AMD Tctl select field is {0:02b}, not the hardware-updated Tctl")]
  NotHardwareTctl(u8),
  #[error("AMD Tctl {cur_tmp}/8 degrees is outside 0 to 120")]
  Implausible { cur_tmp: u16 },
}

/// Decode a Reported Temperature Control value of either map. Both maps share
/// the `CurTmp` (31:21) and select (17:16) fields and the 0.125 scale.
pub(crate) fn decode_tctl(raw: u32) -> Result<f32, TctlDecodeError> {
  if raw == 0 || raw == u32::MAX {
    return Err(TctlDecodeError::FailedTransaction(raw));
  }
  // 01b and 10b are reserved; 11b holds a software-specified value.
  let select = ((raw >> 16) & 0b11) as u8;
  if select != 0 {
    return Err(TctlDecodeError::NotHardwareTctl(select));
  }
  let cur_tmp = (raw >> 21) as u16;
  if cur_tmp > MAX_PLAUSIBLE_CUR_TMP {
    return Err(TctlDecodeError::Implausible { cur_tmp });
  }
  Ok(f32::from(cur_tmp) * 0.125)
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
pub(crate) fn admit_identity(
  access: &impl AmdFamily10Access,
  group: &TctlGroup,
) -> Result<(), TctlError> {
  let actual = access.read_identity().map_err(TctlError::IdentityRead)? as u32;
  if actual != group.identity {
    return Err(TctlError::IdentityMismatch {
      expected: group.identity,
      actual,
    });
  }
  Ok(())
}

/// One Tctl sample in degrees on the Tctl scale. A failed call or a lock
/// timeout skips the sample; no value is read from a failed call.
pub(crate) fn sample_tctl(
  access: &impl AmdFamily10Access,
  map: TctlRegisterMap,
) -> Result<f32, TctlError> {
  let raw = match map {
    TctlRegisterMap::D18F3xA4 => access.read_d18f3xa4(),
    TctlRegisterMap::SmuD8200Ca4 => {
      // The module writes the SMU index and does not restore it. The guard
      // is dropped after the call on success and on failure alike.
      let _lock = access.lock_pci().map_err(TctlError::Read)?;
      access.read_smu_reported_temperature()
    }
  }
  .map_err(TctlError::Read)?;
  Ok(decode_tctl(raw as u32)?)
}

#[cfg(test)]
mod tests {
  use std::cell::{Cell, RefCell};
  use std::rc::Rc;

  use super::super::super::client::MutexError;
  use super::*;

  /// Synthetic register value built from the specification's field layout
  /// (rev 2, Register map): `CurTmp` at 31:21, select at 17:16, plus
  /// arbitrary slew-control bits 12:0 that the decode must ignore.
  fn reported_temperature(cur_tmp: u32, select: u32, slew: u32) -> u32 {
    (cur_tmp << 21) | (select << 16) | (slew & 0x1FFF)
  }

  fn execute_error(function: &'static str) -> PawnIoError {
    PawnIoError::Execute {
      function,
      // STATUS_ACCESS_DENIED as an NTSTATUS-derived HRESULT.
      hresult: 0xD000_0022_u32 as i32,
    }
  }

  fn timeout() -> PawnIoError {
    PawnIoError::Mutex(MutexError::Timeout {
      name: ACCESS_PCI_MUTEX,
    })
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  enum Call {
    Identity,
    D18F3xA4,
    LockPci,
    Smu { lock_held: bool },
  }

  struct Guard(Rc<Cell<bool>>);

  impl Drop for Guard {
    fn drop(&mut self) {
      self.0.set(false);
    }
  }

  /// A scripted module handle that records every call and tracks whether
  /// the PCI lock is held.
  struct FakeModule {
    identity: Result<u64, PawnIoError>,
    register: Result<u64, PawnIoError>,
    lock_times_out: bool,
    held: Rc<Cell<bool>>,
    calls: RefCell<Vec<Call>>,
  }

  impl FakeModule {
    fn new(
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

    fn reading(register: u32) -> Self {
      Self::new(Ok(0), Ok(u64::from(register)))
    }

    fn calls(&self) -> Vec<Call> {
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

  fn group(family: u32, model: u32) -> &'static TctlGroup {
    select_tctl_group(family, model).expect("a documented group")
  }

  #[test]
  fn every_documented_group_selects_its_map_and_identity_at_both_bounds() {
    use TctlRegisterMap::{D18F3xA4, SmuD8200Ca4};

    // (family, first model, last model, map, D18F3x00) from the spec's
    // Detection group table, rev 2.
    let expected = [
      (0x15, 0x00, 0x0F, D18F3xA4, 0x1603_1022),
      (0x15, 0x10, 0x1F, D18F3xA4, 0x1403_1022),
      (0x15, 0x30, 0x3F, D18F3xA4, 0x141D_1022),
      (0x15, 0x60, 0x6F, SmuD8200Ca4, 0x1573_1022),
      (0x15, 0x70, 0x7F, SmuD8200Ca4, 0x15B3_1022),
      (0x16, 0x00, 0x0F, D18F3xA4, 0x1533_1022),
      (0x16, 0x30, 0x3F, D18F3xA4, 0x1583_1022),
    ];

    for (family, first, last, map, identity) in expected {
      for model in [first, last] {
        let group = group(family, model);
        assert_eq!(group.map, map, "{family:#x}/{model:#x}");
        assert_eq!(group.identity, identity, "{family:#x}/{model:#x}");
      }
    }
  }

  #[test]
  fn revision_guide_cpuid_identities_select_their_groups() {
    use super::super::identity::effective_family_model;

    // CPUID Fn0000_0001_EAX evidence identities from the spec's S8 (rev 2,
    // Scoped enablement). Evidence only; admission keys on family/model.
    let evidence = [
      (0x0060_0F12, 0x1603_1022), // OR-B2
      (0x0060_0F20, 0x1603_1022), // OR-C0
      (0x0061_0F01, 0x1403_1022), // TN-A1
      (0x0063_0F01, 0x141D_1022), // KV-A1
      (0x0067_0F00, 0x15B3_1022), // ST-A0
      (0x0070_0F01, 0x1533_1022), // KB-A1
      (0x0073_0F01, 0x1583_1022), // ML-A1
    ];

    for (signature, identity) in evidence {
      let (family, model) = effective_family_model(signature);
      assert_eq!(group(family, model).identity, identity, "{signature:#010x}");
    }
  }

  #[test]
  fn family_15h_models_00h_0fh_admit_only_single_node_package_types() {
    // PkgType encoding from BKDG 42301 p. 482 via the spec's Detection
    // table (rev 2).
    let orochi = group(0x15, 0x01);

    assert_eq!(orochi.admits_package_type(|| Some(0b0001)), Ok(())); // AM3r2
    assert_eq!(orochi.admits_package_type(|| Some(0b0101)), Ok(())); // C32r1
    assert_eq!(
      orochi.admits_package_type(|| Some(0b0011)), // G34r1, dual-node
      Err(Some(0b0011))
    );
    for reserved in [0b0000, 0b0010, 0b0100, 0b0110, 0b1111] {
      assert_eq!(
        orochi.admits_package_type(|| Some(reserved)),
        Err(Some(reserved))
      );
    }
    assert_eq!(orochi.admits_package_type(|| None), Err(None));
  }

  #[test]
  fn other_groups_have_no_package_type_gate() {
    for (family, model) in [
      (0x15, 0x10),
      (0x15, 0x30),
      (0x15, 0x60),
      (0x15, 0x70),
      (0x16, 0x00),
      (0x16, 0x30),
    ] {
      assert_eq!(
        group(family, model).admits_package_type(|| panic!("not queried")),
        Ok(()),
        "{family:#x}/{model:#x}"
      );
    }
  }

  #[test]
  fn undocumented_models_and_other_families_have_no_group() {
    for (family, model) in [
      (0x15, 0x20),
      (0x15, 0x2F),
      (0x15, 0x40),
      (0x15, 0x5F),
      (0x15, 0x80),
      (0x15, 0xFF),
      (0x16, 0x10),
      (0x16, 0x2F),
      (0x16, 0x40),
      (0x16, 0xFF),
      (0x14, 0x00),
      (0x10, 0x04),
      (0x17, 0x01),
    ] {
      assert_eq!(
        select_tctl_group(family, model),
        None,
        "{family:#x}/{model:#x}"
      );
    }
    assert!(is_family_15h_16h(0x15) && is_family_15h_16h(0x16));
    assert!(!is_family_15h_16h(0x14) && !is_family_15h_16h(0x17));
  }

  #[test]
  fn decode_scales_cur_tmp_by_one_eighth_and_ignores_slew_bits() {
    // Synthetic: CurTmp 0x1A4 (420) = 52.5, slew controls all set.
    assert_eq!(
      decode_tctl(reported_temperature(420, 0b00, 0x1FFF)),
      Ok(52.5)
    );
    // Synthetic: a low idle value on the Tctl scale is not an error.
    assert_eq!(decode_tctl(reported_temperature(1, 0b00, 0x0A5)), Ok(0.125));
  }

  #[test]
  fn decode_rejects_every_select_field_other_than_hardware_tctl() {
    for select in [0b01, 0b10, 0b11] {
      assert_eq!(
        decode_tctl(reported_temperature(400, select, 0)),
        Err(TctlDecodeError::NotHardwareTctl(select as u8))
      );
    }
  }

  #[test]
  fn decode_rejects_all_zero_and_all_ones_as_failed_transactions() {
    assert_eq!(decode_tctl(0), Err(TctlDecodeError::FailedTransaction(0)));
    assert_eq!(
      decode_tctl(u32::MAX),
      Err(TctlDecodeError::FailedTransaction(u32::MAX))
    );
  }

  #[test]
  fn decode_publishes_only_zero_to_120_degrees() {
    // CurTmp 0 with a nonzero slew field is a real 0 on the Tctl scale.
    assert_eq!(decode_tctl(reported_temperature(0, 0b00, 0x10)), Ok(0.0));
    assert_eq!(decode_tctl(reported_temperature(960, 0b00, 0)), Ok(120.0));
    assert_eq!(
      decode_tctl(reported_temperature(961, 0b00, 0)),
      Err(TctlDecodeError::Implausible { cur_tmp: 961 })
    );
    assert_eq!(
      decode_tctl(reported_temperature(0x7FF, 0b00, 0)),
      Err(TctlDecodeError::Implausible { cur_tmp: 0x7FF })
    );
  }

  #[test]
  fn identity_gate_accepts_only_the_group_value_in_the_low_32_bits() {
    let kaveri = group(0x15, 0x30);

    let matched = FakeModule::new(Ok(0x141D_1022), Ok(0));
    assert_eq!(admit_identity(&matched, kaveri), Ok(()));
    assert_eq!(matched.calls(), vec![Call::Identity]);

    let upper_cell_bits = FakeModule::new(Ok(0xFFFF_FFFF_141D_1022), Ok(0));
    assert_eq!(admit_identity(&upper_cell_bits, kaveri), Ok(()));

    // Another group's D18F3x00 (15h 10h-1Fh) on a 30h-3Fh CPU.
    let mismatched = FakeModule::new(Ok(0x1403_1022), Ok(0));
    assert_eq!(
      admit_identity(&mismatched, kaveri),
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
      admit_identity(&failed, group(0x15, 0x01)),
      Err(TctlError::IdentityRead(execute_error("ioctl_read_miscctl")))
    );
  }

  #[test]
  fn map_a_reads_d18f3xa4_without_the_pci_lock() {
    let module = FakeModule::reading(reported_temperature(352, 0b00, 0));

    assert_eq!(sample_tctl(&module, TctlRegisterMap::D18F3xA4), Ok(44.0));
    assert_eq!(module.calls(), vec![Call::D18F3xA4]);
  }

  #[test]
  fn map_b_holds_the_pci_lock_around_the_smu_read_and_releases_it() {
    let module = FakeModule::reading(reported_temperature(512, 0b00, 0));

    assert_eq!(sample_tctl(&module, TctlRegisterMap::SmuD8200Ca4), Ok(64.0));
    assert_eq!(
      module.calls(),
      vec![Call::LockPci, Call::Smu { lock_held: true }]
    );
    assert!(!module.held.get());
  }

  #[test]
  fn map_b_releases_the_pci_lock_after_a_failed_call() {
    let module = FakeModule::new(Ok(0), Err(execute_error("ioctl_read_smu")));

    assert_eq!(
      sample_tctl(&module, TctlRegisterMap::SmuD8200Ca4),
      Err(TctlError::Read(execute_error("ioctl_read_smu")))
    );
    assert_eq!(
      module.calls(),
      vec![Call::LockPci, Call::Smu { lock_held: true }]
    );
    assert!(!module.held.get());
  }

  #[test]
  fn map_b_releases_the_pci_lock_after_a_rejected_value() {
    let module = FakeModule::reading(reported_temperature(400, 0b11, 0));

    assert_eq!(
      sample_tctl(&module, TctlRegisterMap::SmuD8200Ca4),
      Err(TctlError::Decode(TctlDecodeError::NotHardwareTctl(0b11)))
    );
    assert!(!module.held.get());
  }

  #[test]
  fn map_b_skips_the_sample_on_a_lock_timeout_without_calling_the_module() {
    let mut module = FakeModule::reading(reported_temperature(512, 0b00, 0));
    module.lock_times_out = true;

    assert_eq!(
      sample_tctl(&module, TctlRegisterMap::SmuD8200Ca4),
      Err(TctlError::Read(timeout()))
    );
    assert_eq!(module.calls(), vec![Call::LockPci]);
    assert!(!module.held.get());
  }

  #[test]
  fn a_failed_map_a_read_skips_the_sample() {
    let module = FakeModule::new(Ok(0), Err(execute_error("ioctl_read_miscctl")));

    assert_eq!(
      sample_tctl(&module, TctlRegisterMap::D18F3xA4),
      Err(TctlError::Read(execute_error("ioctl_read_miscctl")))
    );
  }

  #[test]
  fn sample_decodes_only_the_low_32_bits_of_the_output_cell() {
    let module = FakeModule::new(
      Ok(0),
      Ok(0xFFFF_FFFF_0000_0000 | u64::from(reported_temperature(8, 0, 0))),
    );

    assert_eq!(sample_tctl(&module, TctlRegisterMap::D18F3xA4), Ok(1.0));
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
