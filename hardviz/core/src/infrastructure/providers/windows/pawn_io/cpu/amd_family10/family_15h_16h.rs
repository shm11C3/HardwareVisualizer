//! Family 15h/16h model groups, the PkgType gate and the per-map sample.

use super::access::{AmdFamily10Access, TctlError};
use super::reported_temperature::{SelectFieldCheck, decode_tctl};

/// D0F0xBC_xD820_0CA4, Reported Temperature Control (map B).
pub(super) const SMU_REPORTED_TEMPERATURE_CONTROL: u64 = 0xD820_0CA4;

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
  pub(crate) identity: u32,
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
  // Both maps carry the select field at 17:16.
  Ok(decode_tctl(
    raw as u32,
    SelectFieldCheck::RequireHardwareTctl,
  )?)
}

#[cfg(test)]
mod tests {
  use super::super::access::fake::{Call, FakeModule, execute_error, timeout};
  use super::super::reported_temperature::{TctlDecodeError, reported_temperature};
  use super::*;

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
    use super::super::super::identity::effective_family_model;

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
}
