//! Family 10h, 11h, 12h and 14h models 00h-0Fh: model groups, the Family
//! 10h erratum 319 and `PkgType` gates, the DDR3 gate and the sample.
//!
//! Implemented clean-room from
//! `docs/specs/sensors/cpu-amd-family-10h-14h-tctl.md` rev 1 and the
//! `AMDFamily10` contract in `docs/specs/sensors/pawnio-interface.md` rev 9.
//! Every group reads D18F3xA4 on node 0 through `ioctl_read_miscctl`; no
//! other module function is called on this path.

use std::time::Duration;

use super::access::{AmdFamily10Access, TctlError};
use super::reported_temperature::{SelectFieldCheck, decode_tctl};
use crate::infrastructure::providers::windows::wmi_provider::{
  Win32PhysicalMemoryType, query_physical_memory_types,
};
use crate::models::SensorEnablement;

/// Family 10h CPUID Fn8000_0001_EBX[31:28] `PkgType` encodings (BKDG 31116
/// p. 387).
const PKG_TYPE_SOCKET_F: u32 = 0b0000;
const PKG_TYPE_AM2R2_AM3: u32 = 0b0001;
const PKG_TYPE_S1G3_S1G4: u32 = 0b0010;
const PKG_TYPE_G34: u32 = 0b0011;
const PKG_TYPE_ASB2: u32 = 0b0100;
const PKG_TYPE_C32: u32 = 0b0101;

/// SMBIOS Type 17 Memory Type `18h`, DDR3 (DSP0134 3.9.0 Table 77).
const SMBIOS_MEMORY_TYPE_DDR3: u32 = 0x18;
/// Keeps the one-time WMI query inside the sensor service's two-second pipe
/// exchange. A query that does not finish in time fails the gate.
const DDR3_GATE_QUERY_TIMEOUT: Duration = Duration::from_millis(750);

/// One group row of the specification's Detection group table.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Family10h14hGroup {
  family: u32,
  first_model: u32,
  last_model: u32,
  /// The D18F3x00 value (DeviceID 31:16, VendorID 15:0) of this group.
  pub(crate) identity: u32,
  /// Only Family 10h has `CurTmpSel` at 17:16; the other groups leave those
  /// bits Reserved.
  select_field: SelectFieldCheck,
}

const FAMILY_10H_14H_GROUPS: [Family10h14hGroup; 4] = [
  // Family 10h, any model (BKDG 31116 F3x00 p. 287, F3xA4 p. 320).
  Family10h14hGroup {
    family: 0x10,
    first_model: 0x00,
    last_model: 0xFF,
    identity: 0x1203_1022,
    select_field: SelectFieldCheck::RequireHardwareTctl,
  },
  // Family 11h, any model (BKDG 41256 p. 140, pp. 156-157).
  Family10h14hGroup {
    family: 0x11,
    first_model: 0x00,
    last_model: 0xFF,
    identity: 0x1303_1022,
    select_field: SelectFieldCheck::NotExamined,
  },
  // Family 12h, any model (BKDG 41131 p. 303, p. 316).
  Family10h14hGroup {
    family: 0x12,
    first_model: 0x00,
    last_model: 0xFF,
    identity: 0x1703_1022,
    select_field: SelectFieldCheck::NotExamined,
  },
  // Family 14h models 00h-0Fh (BKDG 43170 p. 282, pp. 294-295). Shares the
  // 12h device ID; CPUID family tells them apart.
  Family10h14hGroup {
    family: 0x14,
    first_model: 0x00,
    last_model: 0x0F,
    identity: 0x1703_1022,
    select_field: SelectFieldCheck::NotExamined,
  },
];

/// Whether the effective family belongs to this specification, so its
/// unsupported models (Family 13h, Family 14h models 10h and above) are not
/// handed to the Zen path.
pub(crate) const fn is_family_10h_14h(family: u32) -> bool {
  matches!(family, 0x10..=0x14)
}

/// The group of an AMD processor's effective family and model, if the
/// specification defines its register map.
pub(crate) fn select_family_10h_14h_group(
  family: u32,
  model: u32,
) -> Option<&'static Family10h14hGroup> {
  FAMILY_10H_14H_GROUPS.iter().find(|group| {
    group.family == family && (group.first_model..=group.last_model).contains(&model)
  })
}

/// Why a CPU of this specification's families is not admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Family10h14hRejection {
  #[error(
    "AMD Family 10h model 0x{model:x} stepping 0x{stepping:x} is in the erratum 319 \
     revision set"
  )]
  Erratum319Revision { model: u32, stepping: u32 },
  #[error("AMD Family 10h stepping could not be read")]
  SteppingUnreadable,
  #[error("AMD Family 10h socket F package type 0x0 is excluded by erratum 319")]
  SocketF,
  #[error("AMD Family 10h G34 package type 0x3 is a dual-node package")]
  G34,
  #[error("AMD Family 10h package type 0x{0:x} is reserved")]
  ReservedPackageType(u32),
  #[error("AMD Family 10h package type could not be read")]
  PackageTypeUnreadable,
  #[error("the AMD Family 10h-14h Tctl path requires exactly one physical package")]
  SinglePackageUnconfirmed,
  #[error(
    "AMD Family 10h AM2r2/AM3 package: Win32_PhysicalMemory does not show DDR3 on \
     every installed module (erratum 319)"
  )]
  Ddr3Unconfirmed,
  #[error(
    "AMD Family 10h AM2r2/AM3 package: the Win32_PhysicalMemory query for the \
     DDR3 gate did not complete, so the path stays closed until a later query \
     confirms DDR3"
  )]
  Ddr3QueryFailed,
}

impl Family10h14hRejection {
  /// Whether a later evaluation may admit this CPU. Only a DDR3 query that
  /// did not complete qualifies; every other rejection is a fact of the
  /// hardware or the topology.
  pub(crate) const fn is_retryable(self) -> bool {
    matches!(self, Self::Ddr3QueryFailed)
  }
}

/// The result of the CPU gates for an admitted group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CpuAdmission {
  pub(crate) enablement: SensorEnablement,
  /// Family 10h `PkgType` `0001b` (AM2r2 or AM3) only: the DDR3 gate must
  /// pass before the module is loaded.
  pub(crate) requires_ddr3: bool,
}

const VERIFIED: CpuAdmission = CpuAdmission {
  enablement: SensorEnablement::Verified,
  requires_ddr3: false,
};

/// Erratum 319 (Revision Guide 41322 Table 27 p. 31, p. 74): every model
/// below 4, model 4 steppings 0-2 (RB-C2 and below) and model 8 stepping 0
/// (HY-D0).
const fn in_erratum_319_revision_set(model: u32, stepping: u32) -> bool {
  match model {
    0x00..=0x03 => true,
    0x04 => stepping <= 0x2,
    0x08 => stepping == 0x0,
    _ => false,
  }
}

impl Family10h14hGroup {
  /// The Family 10h revision and `PkgType` gates, in the specification's
  /// order. Families 11h, 12h and 14h have neither, so `stepping` and
  /// `package_type` are read only for Family 10h. An unreadable CPUID value
  /// is not admitted.
  pub(crate) fn admit_cpu(
    &self,
    model: u32,
    stepping: impl FnOnce() -> Option<u32>,
    package_type: impl FnOnce() -> Option<u32>,
  ) -> Result<CpuAdmission, Family10h14hRejection> {
    if self.family != 0x10 {
      return Ok(VERIFIED);
    }
    let stepping = stepping().ok_or(Family10h14hRejection::SteppingUnreadable)?;
    if in_erratum_319_revision_set(model, stepping) {
      return Err(Family10h14hRejection::Erratum319Revision { model, stepping });
    }
    match package_type() {
      Some(PKG_TYPE_S1G3_S1G4 | PKG_TYPE_ASB2 | PKG_TYPE_C32) => Ok(VERIFIED),
      Some(PKG_TYPE_AM2R2_AM3) => Ok(CpuAdmission {
        enablement: SensorEnablement::Experimental,
        requires_ddr3: true,
      }),
      Some(PKG_TYPE_SOCKET_F) => Err(Family10h14hRejection::SocketF),
      Some(PKG_TYPE_G34) => Err(Family10h14hRejection::G34),
      Some(reserved) => Err(Family10h14hRejection::ReservedPackageType(reserved)),
      None => Err(Family10h14hRejection::PackageTypeUnreadable),
    }
  }
}

/// The outcome of one DDR3 gate evaluation. Only `Confirmed` admits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ddr3Gate {
  /// The query completed and every installed instance reports DDR3.
  Confirmed,
  /// The query completed and does not confirm DDR3: an empty result, an
  /// absent or null property, or another memory type.
  NotConfirmed,
  /// The query failed or did not finish in time. It may be evaluated again
  /// later; the path stays closed until a query confirms DDR3.
  QueryFailed,
}

/// The DDR3 gate over a `Win32_PhysicalMemory` query of `Capacity` and
/// `SMBIOSMemoryType`: the query succeeds, at least one instance has
/// `Capacity` > 0, and every such instance reports DDR3. An absent or null
/// property on any instance, a failed or empty query, or any other memory
/// type fails the gate. A failed query is kept apart from a completed one
/// so that only the former is retried.
pub(crate) fn evaluate_ddr3_gate(
  query: Result<Vec<Win32PhysicalMemoryType>, String>,
) -> Ddr3Gate {
  let Ok(records) = query else {
    return Ddr3Gate::QueryFailed;
  };
  let mut installed = 0_usize;
  for record in &records {
    let (Some(capacity), Some(memory_type)) =
      (record.capacity, record.smbios_memory_type)
    else {
      return Ddr3Gate::NotConfirmed;
    };
    if capacity > 0 {
      if memory_type != SMBIOS_MEMORY_TYPE_DDR3 {
        return Ddr3Gate::NotConfirmed;
      }
      installed += 1;
    }
  }
  if installed > 0 {
    Ddr3Gate::Confirmed
  } else {
    Ddr3Gate::NotConfirmed
  }
}

/// Evaluate the DDR3 gate through a bounded, read-only WMI query.
pub(crate) fn query_ddr3_gate() -> Ddr3Gate {
  evaluate_ddr3_gate(query_physical_memory_types(DDR3_GATE_QUERY_TIMEOUT))
}

/// One Tctl sample from D18F3xA4, decoded with the group's select-field
/// rule. A failed call skips the sample; no value is read from it.
pub(crate) fn sample_family_10h_14h_tctl(
  access: &impl AmdFamily10Access,
  group: &Family10h14hGroup,
) -> Result<f32, TctlError> {
  let raw = access.read_d18f3xa4().map_err(TctlError::Read)?;
  Ok(decode_tctl(raw as u32, group.select_field)?)
}

#[cfg(test)]
mod tests {
  use super::super::super::identity::{effective_family_model, stepping_from_signature};
  use super::super::access::admit_identity;
  use super::super::access::fake::{Call, FakeModule, execute_error};
  use super::super::reported_temperature::TctlDecodeError;
  use super::*;

  const EXPERIMENTAL_AFTER_DDR3: CpuAdmission = CpuAdmission {
    enablement: SensorEnablement::Experimental,
    requires_ddr3: true,
  };

  const GIB: u64 = 1 << 30;
  /// SMBIOS Type 17 Memory Type `13h` (DDR2) and `02h` (Unknown), DSP0134
  /// Table 77 via the spec's S15.
  const SMBIOS_DDR2: u32 = 0x13;
  const SMBIOS_UNKNOWN: u32 = 0x02;

  fn group(family: u32, model: u32) -> &'static Family10h14hGroup {
    select_family_10h_14h_group(family, model).expect("a documented group")
  }

  fn not_queried() -> Option<u32> {
    panic!("this CPUID value must not be queried for this group")
  }

  /// Admission of a CPUID Fn0000_0001_EAX signature with a `PkgType`.
  fn admit_signature(
    signature: u32,
    package_type: Option<u32>,
  ) -> Result<CpuAdmission, Family10h14hRejection> {
    let (family, model) = effective_family_model(signature);
    group(family, model).admit_cpu(
      model,
      || Some(stepping_from_signature(signature)),
      || package_type,
    )
  }

  fn memory(capacity: Option<u64>, memory_type: Option<u32>) -> Win32PhysicalMemoryType {
    Win32PhysicalMemoryType {
      capacity,
      smbios_memory_type: memory_type,
    }
  }

  // Spec rev 1, Detection: group table.
  #[test]
  fn every_group_row_selects_its_identity_and_select_field_at_both_bounds() {
    use SelectFieldCheck::{NotExamined, RequireHardwareTctl};

    let expected = [
      (0x10, 0x00, 0xFF, 0x1203_1022, RequireHardwareTctl),
      (0x11, 0x00, 0xFF, 0x1303_1022, NotExamined),
      (0x12, 0x00, 0xFF, 0x1703_1022, NotExamined),
      (0x14, 0x00, 0x0F, 0x1703_1022, NotExamined),
    ];

    for (family, first, last, identity, select_field) in expected {
      for model in [first, last] {
        let group = group(family, model);
        assert_eq!(group.identity, identity, "{family:#x}/{model:#x}");
        assert_eq!(group.select_field, select_field, "{family:#x}/{model:#x}");
      }
    }
  }

  // Spec rev 1, Scoped enablement: Family 13h and Family 14h models 10h-FFh
  // (Unsupported) and Family 15h/16h (out of scope).
  #[test]
  fn family_13h_family_14h_models_10h_up_and_other_families_have_no_group() {
    for (family, model) in [
      (0x13, 0x00),
      (0x13, 0xFF),
      (0x14, 0x10),
      (0x14, 0xFF),
      (0x15, 0x00),
      (0x16, 0x00),
      (0x0F, 0x00),
      (0x17, 0x01),
    ] {
      assert_eq!(
        select_family_10h_14h_group(family, model),
        None,
        "{family:#x}/{model:#x}"
      );
    }
    for family in 0x10..=0x14 {
      assert!(is_family_10h_14h(family), "{family:#x}");
    }
    for family in [0x0F, 0x15, 0x16, 0x17] {
      assert!(!is_family_10h_14h(family), "{family:#x}");
    }
  }

  // Spec rev 1, Scoped enablement: the Verified Family 10h row and its
  // evidence identities (S5 Tables 9-11, 6). Evidence only, not gates.
  #[test]
  fn family_10h_s1g3_s1g4_asb2_and_c32_evidence_is_verified() {
    for (signature, package_type) in [
      (0x0010_0F62, PKG_TYPE_S1G3_S1G4), // DA-C2, S1g3
      (0x0010_0F53, PKG_TYPE_S1G3_S1G4), // BL-C3, S1g4
      (0x0010_0F63, PKG_TYPE_S1G3_S1G4), // DA-C3, S1g4
      (0x0010_0F63, PKG_TYPE_ASB2),      // DA-C3, ASB2
      (0x0010_0F81, PKG_TYPE_C32),       // HY-D1, C32r1
    ] {
      assert_eq!(
        admit_signature(signature, Some(package_type)),
        Ok(VERIFIED),
        "{signature:#010x}/{package_type:04b}"
      );
      let (family, model) = effective_family_model(signature);
      assert_eq!(group(family, model).identity, 0x1203_1022);
    }
  }

  // Spec rev 1, Scoped enablement: the Experimental Family 10h AM2r2/AM3 row
  // and its evidence identities (S5 Table 8).
  #[test]
  fn family_10h_am2r2_am3_evidence_is_experimental_behind_the_ddr3_gate() {
    for signature in [
      0x0010_0F52, // BL-C2
      0x0010_0F62, // DA-C2
      0x0010_0F43, // RB-C3
      0x0010_0F53, // BL-C3
      0x0010_0F63, // DA-C3
      0x0010_0FA0, // PH-E0
    ] {
      assert_eq!(
        admit_signature(signature, Some(PKG_TYPE_AM2R2_AM3)),
        Ok(EXPERIMENTAL_AFTER_DDR3),
        "{signature:#010x}"
      );
    }
  }

  // Spec rev 1, Scoped enablement: the Family 11h, 12h and 14h Verified rows
  // and their evidence identities (S6 Table 1, S7 Tables 2-3, S8 Table 2).
  #[test]
  fn family_11h_12h_and_14h_evidence_is_verified_without_family_10h_gates() {
    for (signature, identity) in [
      (0x0020_0F31, 0x1303_1022), // LG-B1, Family 11h
      (0x0030_0F10, 0x1703_1022), // LN-B0, Family 12h
      (0x0050_0F10, 0x1703_1022), // ON-B0, Family 14h
      (0x0050_0F20, 0x1703_1022), // ON-C0, Family 14h
    ] {
      let (family, model) = effective_family_model(signature);
      let group = group(family, model);

      assert_eq!(group.identity, identity, "{signature:#010x}");
      assert_eq!(
        group.admit_cpu(model, not_queried, not_queried),
        Ok(VERIFIED),
        "{signature:#010x}"
      );
    }
  }

  // Spec rev 1, Detection: the erratum 319 revision gate, and Scoped
  // enablement: the documented members of the Unsupported revision set.
  #[test]
  fn family_10h_erratum_319_revisions_are_rejected_before_the_package_type() {
    for signature in [
      0x0010_0F2A, // DR-BA
      0x0010_0F22, // DR-B2
      0x0010_0F23, // DR-B3
      0x0010_0F42, // RB-C2
      0x0010_0F80, // HY-D0
    ] {
      let (_, model) = effective_family_model(signature);
      let stepping = stepping_from_signature(signature);
      assert_eq!(
        group(0x10, model).admit_cpu(model, || Some(stepping), not_queried),
        Err(Family10h14hRejection::Erratum319Revision { model, stepping }),
        "{signature:#010x}"
      );
    }
  }

  // Spec rev 1, Detection: the erratum 319 revision gate at its boundaries,
  // including model 4 steppings 0-1, which have no documented revision
  // (Quirks).
  #[test]
  fn family_10h_erratum_319_gate_covers_exactly_the_specified_pairs() {
    let family_10h = group(0x10, 0x00);
    // C32 is a Verified package type, so a pair outside the revision set must
    // be admitted outright, not merely rejected for another reason.
    let admit = |model: u32, stepping: u32| {
      family_10h.admit_cpu(model, || Some(stepping), || Some(PKG_TYPE_C32))
    };
    let assert_rejected = |model: u32, stepping: u32| {
      assert_eq!(
        admit(model, stepping),
        Err(Family10h14hRejection::Erratum319Revision { model, stepping }),
        "{model:#x}/{stepping:#x}"
      );
    };
    let assert_admitted = |model: u32, stepping: u32| {
      assert_eq!(
        admit(model, stepping),
        Ok(VERIFIED),
        "{model:#x}/{stepping:#x}"
      );
    };

    for model in 0x00..=0x03 {
      for stepping in 0x0..=0xF {
        assert_rejected(model, stepping);
      }
    }
    for stepping in 0x0..=0x2 {
      assert_rejected(0x04, stepping);
    }
    assert_rejected(0x08, 0x0);

    for stepping in 0x3..=0xF {
      assert_admitted(0x04, stepping);
    }
    for stepping in 0x1..=0xF {
      assert_admitted(0x08, stepping);
    }
    for model in [0x05, 0x06, 0x07, 0x09, 0x0A, 0x0F, 0xFF] {
      assert_admitted(model, 0x0);
    }
  }

  #[test]
  fn family_10h_with_an_unreadable_stepping_is_not_admitted() {
    assert_eq!(
      group(0x10, 0x06).admit_cpu(0x06, || None, not_queried),
      Err(Family10h14hRejection::SteppingUnreadable)
    );
  }

  // Spec rev 1, Detection: the Family 10h PkgType gates, and Scoped
  // enablement: socket F (0000b), G34 (0011b, HY-D1 00100F91h) and the
  // reserved encodings.
  #[test]
  fn family_10h_socket_f_g34_reserved_and_unreadable_package_types_are_rejected() {
    assert_eq!(
      admit_signature(0x0010_0F62, Some(PKG_TYPE_SOCKET_F)),
      Err(Family10h14hRejection::SocketF)
    );
    assert_eq!(
      admit_signature(0x0010_0F91, Some(PKG_TYPE_G34)), // HY-D1, Opteron 6100
      Err(Family10h14hRejection::G34)
    );
    for reserved in 0b0110..=0b1111 {
      assert_eq!(
        admit_signature(0x0010_0F62, Some(reserved)),
        Err(Family10h14hRejection::ReservedPackageType(reserved))
      );
    }
    assert_eq!(
      admit_signature(0x0010_0F62, None),
      Err(Family10h14hRejection::PackageTypeUnreadable)
    );
  }

  // Spec rev 1, Decode examples (synthetic): the three Family 10h rows, and
  // the Detection post-read select-field check.
  #[test]
  fn family_10h_decode_examples_check_the_select_field() {
    let family_10h = group(0x10, 0x06);
    let sample =
      |raw: u32| sample_family_10h_14h_tctl(&FakeModule::reading(raw), family_10h);

    assert_eq!(sample(0x2D00_0FEF), Ok(45.0));
    assert_eq!(
      sample(0x2D03_0FEF),
      Err(TctlError::Decode(TctlDecodeError::NotHardwareTctl(0b11)))
    );
    assert_eq!(
      sample(0x2D01_0FEF),
      Err(TctlError::Decode(TctlDecodeError::NotHardwareTctl(0b01)))
    );
  }

  // Spec rev 1, Decode examples (synthetic): the "11h, 12h or 14h" row.
  #[test]
  fn family_11h_12h_and_14h_decode_examples_leave_bits_17_16_unexamined() {
    for (family, model) in [(0x11, 0x03), (0x12, 0x01), (0x14, 0x02)] {
      let module = FakeModule::reading(0x2D03_0FEF);
      assert_eq!(
        sample_family_10h_14h_tctl(&module, group(family, model)),
        Ok(45.0),
        "{family:#x}"
      );
    }
  }

  // Spec rev 1, Decode examples (synthetic): the two "any" rows, and the
  // plausibility gate of Read procedure step 7.
  #[test]
  fn every_group_drops_implausible_and_failed_transaction_values() {
    for (family, model) in [(0x10, 0x06), (0x11, 0x03), (0x12, 0x01), (0x14, 0x02)] {
      let group = group(family, model);
      let sample =
        |raw: u32| sample_family_10h_14h_tctl(&FakeModule::reading(raw), group);

      assert_eq!(
        sample(0x7900_0FEF),
        Err(TctlError::Decode(TctlDecodeError::Implausible {
          cur_tmp: 0x3C8
        })),
        "{family:#x}"
      );
      for raw in [0x0000_0000, 0xFFFF_FFFF] {
        assert_eq!(
          sample(raw),
          Err(TctlError::Decode(TctlDecodeError::FailedTransaction(raw))),
          "{family:#x}/{raw:#010x}"
        );
      }
    }
  }

  // Spec rev 1, Read procedure step 5: D18F3xA4 through ioctl_read_miscctl,
  // no mutex, and only the low 32 bits of out[0].
  #[test]
  fn a_sample_reads_only_d18f3xa4_without_the_pci_lock() {
    let module = FakeModule::new(Ok(0), Ok(0xFFFF_FFFF_2D00_0FEF));

    assert_eq!(
      sample_family_10h_14h_tctl(&module, group(0x12, 0x01)),
      Ok(45.0)
    );
    assert_eq!(module.calls(), vec![Call::D18F3xA4]);
    assert!(!module.held.get());
  }

  // Spec rev 1, Read procedure step 5: an error status skips the sample.
  #[test]
  fn a_failed_d18f3xa4_read_skips_the_sample() {
    let module = FakeModule::new(Ok(0), Err(execute_error("ioctl_read_miscctl")));

    assert_eq!(
      sample_family_10h_14h_tctl(&module, group(0x10, 0x06)),
      Err(TctlError::Read(execute_error("ioctl_read_miscctl")))
    );
  }

  // Spec rev 1, Read procedure step 4 and the Detection identity row: 12h and
  // 14h share 1703_1022h, and another group's value is a mismatch.
  #[test]
  fn identity_gate_admits_each_group_only_on_its_own_d18f3x00_value() {
    for (family, model, identity) in [
      (0x10, 0x06, 0x1203_1022_u32),
      (0x11, 0x03, 0x1303_1022),
      (0x12, 0x01, 0x1703_1022),
      (0x14, 0x02, 0x1703_1022),
    ] {
      let module = FakeModule::new(Ok(u64::from(identity)), Ok(0));
      assert_eq!(
        admit_identity(&module, group(family, model).identity),
        Ok(())
      );
      assert_eq!(module.calls(), vec![Call::Identity]);
    }

    let family_11h_on_10h = FakeModule::new(Ok(0x1303_1022), Ok(0));
    assert_eq!(
      admit_identity(&family_11h_on_10h, group(0x10, 0x06).identity),
      Err(TctlError::IdentityMismatch {
        expected: 0x1203_1022,
        actual: 0x1303_1022,
      })
    );
  }

  // Spec rev 1, Detection: the DDR3 gate passes only when every installed
  // instance reports SMBIOSMemoryType 24 (18h).
  #[test]
  fn ddr3_gate_passes_when_every_installed_module_reports_ddr3() {
    assert_eq!(
      evaluate_ddr3_gate(Ok(vec![memory(
        Some(2 * GIB),
        Some(SMBIOS_MEMORY_TYPE_DDR3)
      )])),
      Ddr3Gate::Confirmed
    );
    assert_eq!(
      evaluate_ddr3_gate(Ok(vec![
        memory(Some(4 * GIB), Some(SMBIOS_MEMORY_TYPE_DDR3)),
        memory(Some(4 * GIB), Some(SMBIOS_MEMORY_TYPE_DDR3)),
      ])),
      Ddr3Gate::Confirmed
    );
    // A Capacity 0 instance (no memory device installed, DSP0134 Table 75)
    // is not required to report DDR3.
    assert_eq!(
      evaluate_ddr3_gate(Ok(vec![
        memory(Some(2 * GIB), Some(SMBIOS_MEMORY_TYPE_DDR3)),
        memory(Some(0), Some(SMBIOS_UNKNOWN)),
      ])),
      Ddr3Gate::Confirmed
    );
  }

  // Spec rev 1, Detection: the DDR3 gate, and Scoped enablement: the "DDR3
  // gate failed" row (a DDR2 system, or memory type unknown).
  #[test]
  fn ddr3_gate_fails_on_ddr2_unknown_types_missing_values_and_empty_results() {
    for records in [
      vec![memory(Some(2 * GIB), Some(SMBIOS_DDR2))],
      vec![
        memory(Some(2 * GIB), Some(SMBIOS_MEMORY_TYPE_DDR3)),
        memory(Some(2 * GIB), Some(SMBIOS_DDR2)),
      ],
      vec![memory(Some(2 * GIB), Some(SMBIOS_UNKNOWN))],
      vec![memory(Some(2 * GIB), None)],
      vec![memory(None, Some(SMBIOS_MEMORY_TYPE_DDR3))],
      vec![
        memory(Some(2 * GIB), Some(SMBIOS_MEMORY_TYPE_DDR3)),
        memory(Some(0), None),
      ],
      vec![memory(Some(0), Some(SMBIOS_MEMORY_TYPE_DDR3))],
      vec![],
    ] {
      assert_eq!(evaluate_ddr3_gate(Ok(records)), Ddr3Gate::NotConfirmed);
    }
  }

  // Spec rev 1, Detection: "a failed or empty query" fails the DDR3 gate. A
  // failed query stays closed and is reported apart from a completed one.
  #[test]
  fn a_failed_ddr3_query_is_kept_apart_from_a_completed_one() {
    assert_eq!(
      evaluate_ddr3_gate(Err("WMI unavailable".to_string())),
      Ddr3Gate::QueryFailed
    );
    assert!(Family10h14hRejection::Ddr3QueryFailed.is_retryable());
    assert!(!Family10h14hRejection::Ddr3Unconfirmed.is_retryable());
    assert!(!Family10h14hRejection::SocketF.is_retryable());
  }

  #[test]
  fn rejection_text_does_not_read_as_a_missing_or_misconfigured_component() {
    use crate::models::ExternalComponentReasonKind;
    use crate::models::external_component_guidance::classify_external_component_reason;

    for rejection in [
      Family10h14hRejection::Erratum319Revision {
        model: 0x04,
        stepping: 0x2,
      },
      Family10h14hRejection::SteppingUnreadable,
      Family10h14hRejection::SocketF,
      Family10h14hRejection::G34,
      Family10h14hRejection::ReservedPackageType(0b0110),
      Family10h14hRejection::PackageTypeUnreadable,
      Family10h14hRejection::SinglePackageUnconfirmed,
      Family10h14hRejection::Ddr3Unconfirmed,
      Family10h14hRejection::Ddr3QueryFailed,
    ] {
      assert_eq!(
        classify_external_component_reason(&rejection.to_string()),
        ExternalComponentReasonKind::Failed,
        "{rejection}"
      );
    }
  }
}
