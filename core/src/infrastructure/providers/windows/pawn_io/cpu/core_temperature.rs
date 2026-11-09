use std::time::Instant;

use windows::Win32::{
  Foundation::ERROR_INSUFFICIENT_BUFFER,
  System::{
    SystemInformation::{
      GROUP_AFFINITY, GROUP_RELATIONSHIP, GetLogicalProcessorInformationEx,
      PROCESSOR_RELATIONSHIP, RelationAll, RelationGroup, RelationProcessorCore,
      RelationProcessorPackage, SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
    },
    Threading::{GetCurrentProcessorNumberEx, GetCurrentThread, SetThreadGroupAffinity},
  },
};
use windows::core::HRESULT;

use super::{
  super::client::{PawnIoClient, PawnIoModule},
  identity::{CpuidLeaf, cpuid_leaf, effective_family_model, vendor_id_from_leaf0},
  temperature_decode::{
    decode_intel_package_temperature, decode_intel_temperature_target,
  },
};
use crate::models::CpuCoreTemperature;

const MSR_TEMPERATURE_TARGET: u64 = 0x1a2;
const IA32_THERM_STATUS: u64 = 0x19c;
const ARROW_LAKE_SIGNATURE: u32 = 0x000c_0662;
const HYBRID_CORE_TYPE_LEAF: u32 = 0x1a;
const SILVERMONT_ATOM_C2000: u32 = 0x4d;
const ALDER_LAKE_S_HX: u32 = 0x0009_0672;
const ALDER_LAKE_H_P_PS: u32 = 0x0009_06a3;
const RAPTOR_LAKE_S_HX: u32 = 0x000b_0671;
const METEOR_LAKE_C0: u32 = 0x000a_06a4;
const HYBRID_CAPABILITY: u32 = 1 << 15;
const MAX_TOPOLOGY_BYTES: usize = 64 * 1024;
const HEADER_SIZE: usize =
  std::mem::offset_of!(SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX, Anonymous);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CoreDtsProfile {
  LegacyModel(u32),
  HybridSignature(u32),
}

pub(crate) fn supported_profile() -> Option<CoreDtsProfile> {
  if !cfg!(target_arch = "x86_64") {
    return None;
  }

  profile_from_cpuid(cpuid_leaf)
}

fn profile_from_cpuid(
  mut read_leaf: impl FnMut(u32) -> Option<CpuidLeaf>,
) -> Option<CoreDtsProfile> {
  let leaf0 = read_leaf(0)?;
  let vendor = vendor_id_from_leaf0(leaf0);
  let signature = read_leaf(1)?.eax;
  if signature == ARROW_LAKE_SIGNATURE && leaf0.eax < HYBRID_CORE_TYPE_LEAF {
    return None;
  }
  let thermal_features = read_leaf(6)?.eax;
  // cpuid_leaf selects subleaf 0.
  let extended_features = read_leaf(7).map_or(0, |leaf| leaf.edx);
  select_profile(&vendor, signature, thermal_features, extended_features)
}

fn select_profile(
  vendor: &str,
  signature: u32,
  thermal_features: u32,
  extended_features: u32,
) -> Option<CoreDtsProfile> {
  if vendor != "GenuineIntel" || thermal_features & 1 == 0 {
    return None;
  }

  if signature == ARROW_LAKE_SIGNATURE && extended_features & HYBRID_CAPABILITY != 0 {
    return Some(CoreDtsProfile::HybridSignature(signature));
  }

  if extended_features & HYBRID_CAPABILITY != 0 {
    return matches!(
      signature,
      ALDER_LAKE_S_HX | ALDER_LAKE_H_P_PS | RAPTOR_LAKE_S_HX | METEOR_LAKE_C0
    )
    .then_some(CoreDtsProfile::HybridSignature(signature));
  }

  let (family, model) = effective_family_model(signature);
  (family == 6
    && matches!(
      model,
      0x1a | 0x1e | 0x1f | 0x2e | 0x2a | 0x2d | SILVERMONT_ATOM_C2000
    ))
  .then_some(CoreDtsProfile::LegacyModel(model))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Processor {
  group: u16,
  number: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CoreTopology {
  group: u16,
  active_mask: usize,
  package_mask: usize,
  // Sorted by representative processor, so enumeration order has no effect.
  core_masks: Vec<usize>,
}

#[cfg(test)]
pub(crate) fn synthetic_topology(core_masks: &[usize]) -> CoreTopology {
  let active = core_masks.iter().copied().fold(0, |mask, core| mask | core);
  CoreTopology::validate(
    0,
    active,
    (0, active),
    core_masks.iter().map(|mask| (0, *mask)).collect(),
  )
  .expect("fixture topology is valid")
}

impl CoreTopology {
  fn validate(
    group: u16,
    active_mask: usize,
    package: (u16, usize),
    mut cores: Vec<(u16, usize)>,
  ) -> Option<Self> {
    if active_mask == 0 || package != (group, active_mask) {
      return None;
    }

    let mut seen = 0;
    for &(core_group, mask) in &cores {
      if core_group != group || mask == 0 || mask & !active_mask != 0 || mask & seen != 0
      {
        return None;
      }
      seen |= mask;
    }

    if seen != active_mask {
      return None;
    }

    cores.sort_unstable_by_key(|(_, mask)| mask.trailing_zeros());
    Some(Self {
      group,
      active_mask,
      package_mask: package.1,
      core_masks: cores.into_iter().map(|(_, mask)| mask).collect(),
    })
  }
}

pub(crate) fn read_topology() -> Option<CoreTopology> {
  let mut length = 0;
  let error =
    unsafe { GetLogicalProcessorInformationEx(RelationAll, None, &mut length) }.err()?;
  if error.code() != HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0)
    || !(HEADER_SIZE..=MAX_TOPOLOGY_BYTES).contains(&(length as usize))
  {
    return None;
  }

  // The API requires an aligned output buffer and returns variable-size records.
  let mut words = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
  let capacity = length;
  unsafe {
    GetLogicalProcessorInformationEx(
      RelationAll,
      Some(words.as_mut_ptr().cast()),
      &mut length,
    )
  }
  .ok()?;
  if length > capacity {
    return None;
  }

  // The words are initialized, aligned, and cover the returned byte count.
  let bytes =
    unsafe { std::slice::from_raw_parts(words.as_ptr().cast(), length as usize) };
  parse_topology(bytes)
}

fn parse_topology(mut bytes: &[u8]) -> Option<CoreTopology> {
  let mut group_mask = None;
  let mut package = None;
  let mut cores = Vec::new();

  while !bytes.is_empty() {
    let relationship = i32::from_ne_bytes(bytes.get(..4)?.try_into().ok()?);
    let size = u32::from_ne_bytes(bytes.get(4..8)?.try_into().ok()?) as usize;
    let data = bytes.get(HEADER_SIZE..size)?;
    bytes = bytes.get(size..)?;

    if relationship == RelationGroup.0 {
      // Only one active group is supported. Its GroupInfo array index is 0.
      let groups = unsafe { read_record::<GROUP_RELATIONSHIP>(data) }?;
      let [group] = groups.GroupInfo;
      if group_mask.is_some()
        || groups.ActiveGroupCount != 1
        || groups.MaximumGroupCount == 0
        || u32::from(group.ActiveProcessorCount) != group.ActiveProcessorMask.count_ones()
        || group.ActiveProcessorCount > group.MaximumProcessorCount
        || u32::from(group.MaximumProcessorCount) > usize::BITS
      {
        return None;
      }
      group_mask = Some(group.ActiveProcessorMask);
    } else if relationship == RelationProcessorCore.0
      || relationship == RelationProcessorPackage.0
    {
      let processors = unsafe { read_record::<PROCESSOR_RELATIONSHIP>(data) }?;
      if processors.GroupCount != 1 {
        return None;
      }
      let [affinity] = processors.GroupMask;
      let mask = (affinity.Group, affinity.Mask);
      if relationship == RelationProcessorPackage.0 {
        if package.replace(mask).is_some() {
          return None;
        }
      } else {
        cores.push(mask);
      }
    }
  }

  CoreTopology::validate(0, group_mask?, package?, cores)
}

/// The caller may use only integer-only FFI records valid for every bit pattern.
unsafe fn read_record<T: Copy>(bytes: &[u8]) -> Option<T> {
  if bytes.len() < size_of::<T>() {
    return None;
  }
  Some(unsafe { bytes.as_ptr().cast::<T>().read_unaligned() })
}

pub(crate) fn acquire(
  profile: CoreDtsProfile,
  topology: &CoreTopology,
  deadline: Instant,
) -> Option<Vec<CpuCoreTemperature>> {
  if Instant::now() >= deadline {
    return None;
  }
  // A synchronous request can outlive its wait budget. Give this disposable
  // thread its own client so an in-flight call holds no package sampler lock.
  let (client, _) = PawnIoClient::open(PawnIoModule::IntelMsr).ok()?;
  collect(profile, topology, deadline, &client)
}

trait CoreAccess {
  type PreviousAffinity;

  fn pin(&self, processor: Processor) -> Option<Self::PreviousAffinity>;
  fn current_processor(&self) -> Processor;
  fn profile(&self) -> Option<CoreDtsProfile>;
  fn hybrid_core_type(&self) -> Option<u8>;
  fn read_msr(&self, msr: u64) -> Option<u64>;
  fn restore(&self, previous: Self::PreviousAffinity) -> bool;
}

impl CoreAccess for PawnIoClient {
  type PreviousAffinity = GROUP_AFFINITY;

  fn pin(&self, processor: Processor) -> Option<Self::PreviousAffinity> {
    let mut previous = GROUP_AFFINITY::default();
    let affinity = GROUP_AFFINITY {
      Mask: 1usize << processor.number,
      Group: processor.group,
      Reserved: [0; 3],
    };
    unsafe { SetThreadGroupAffinity(GetCurrentThread(), &affinity, Some(&mut previous)) }
      .as_bool()
      .then_some(previous)
  }

  fn current_processor(&self) -> Processor {
    let current = unsafe { GetCurrentProcessorNumberEx() };
    Processor {
      group: current.Group,
      number: current.Number,
    }
  }

  fn profile(&self) -> Option<CoreDtsProfile> {
    supported_profile()
  }

  fn hybrid_core_type(&self) -> Option<u8> {
    Some((cpuid_leaf(HYBRID_CORE_TYPE_LEAF)?.eax >> 24) as u8)
  }

  fn read_msr(&self, msr: u64) -> Option<u64> {
    PawnIoClient::read_msr(self, msr).ok()
  }

  fn restore(&self, previous: Self::PreviousAffinity) -> bool {
    unsafe { SetThreadGroupAffinity(GetCurrentThread(), &previous, None) }.as_bool()
  }
}

fn collect(
  profile: CoreDtsProfile,
  topology: &CoreTopology,
  deadline: Instant,
  access: &impl CoreAccess,
) -> Option<Vec<CpuCoreTemperature>> {
  let mut readings = Vec::with_capacity(topology.core_masks.len());
  for mask in &topology.core_masks {
    if Instant::now() >= deadline {
      return None;
    }
    let processor = Processor {
      group: topology.group,
      number: mask.trailing_zeros() as u8,
    };
    let temperature_celsius = if let Some(previous) = access.pin(processor) {
      let reading = sample_core(profile, processor, deadline, access);
      if !access.restore(previous) {
        return None;
      }
      reading
    } else {
      None
    };
    readings.push(CpuCoreTemperature {
      processor_group: processor.group,
      logical_processor: processor.number,
      temperature_celsius,
    });
  }
  (Instant::now() < deadline).then_some(readings)
}

fn sample_core(
  profile: CoreDtsProfile,
  processor: Processor,
  deadline: Instant,
  access: &impl CoreAccess,
) -> Option<f32> {
  if access.current_processor() != processor
    || access.profile() != Some(profile)
    || (matches!(profile, CoreDtsProfile::HybridSignature(_))
      && !matches!(access.hybrid_core_type(), Some(0x40 | 0x20)))
    || Instant::now() >= deadline
  {
    return None;
  }

  let target = access.read_msr(MSR_TEMPERATURE_TARGET)?;
  if Instant::now() >= deadline {
    return None;
  }
  let status = access.read_msr(IA32_THERM_STATUS)?;
  if access.current_processor() != processor {
    return None;
  }

  decode_core_temperature(profile, target, status)
}

fn decode_core_temperature(
  profile: CoreDtsProfile,
  target: u64,
  status: u64,
) -> Option<f32> {
  if status & (1 << 31) == 0
    || (matches!(profile, CoreDtsProfile::LegacyModel(SILVERMONT_ATOM_C2000))
      && (target >> 24) & 0x3f != 0)
  {
    return None;
  }

  // On the approved hybrid targets the DTS reference is factory Tjmax.
  // The TCC offset affects the control threshold and does not enter this decode.
  let target = decode_intel_temperature_target(target).ok()?;
  decode_intel_package_temperature(target, status).ok()
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::cell::{Cell, RefCell};
  use std::time::Duration;
  use windows::Win32::System::SystemInformation::PROCESSOR_GROUP_INFO;

  const BLOOMFIELD_SIGNATURE: u32 = 0x0001_06a4;
  const HYBRID_SIGNATURES: [u32; 4] = [
    ALDER_LAKE_S_HX,
    ALDER_LAKE_H_P_PS,
    RAPTOR_LAKE_S_HX,
    METEOR_LAKE_C0,
  ];
  const LEGACY_PROFILE: CoreDtsProfile = CoreDtsProfile::LegacyModel(0x1a);
  const SILVERMONT_PROFILE: CoreDtsProfile =
    CoreDtsProfile::LegacyModel(SILVERMONT_ATOM_C2000);

  #[test]
  fn core_model_gate_is_independent_of_package_capability() {
    // Synthetic leaf-6 values reflect the two DTS-only observations in the spec.
    for (signature, model, leaf6) in [
      (BLOOMFIELD_SIGNATURE, 0x1a, 0x03),
      (0x0004_06d8, 0x4d, 0x05),
      (0x0002_06a7, 0x2a, 0x41),
    ] {
      assert_eq!(
        select_profile("GenuineIntel", signature, leaf6, 0),
        Some(CoreDtsProfile::LegacyModel(model))
      );
    }
    for leaf6 in [0, 0x40] {
      assert_eq!(
        select_profile("GenuineIntel", BLOOMFIELD_SIGNATURE, leaf6, 0),
        None
      );
    }
    assert_eq!(select_profile("GenuineIntel", 0x0004_06c8, 0x41, 0), None);
    assert_eq!(select_profile("GenuineIntel", 0x0000_05d8, 0x41, 0), None);
    assert_eq!(
      select_profile("AuthenticAMD", BLOOMFIELD_SIGNATURE, 0x41, 0),
      None
    );
  }

  #[test]
  fn hybrid_gate_requires_the_exact_full_signature_and_flag() {
    for signature in HYBRID_SIGNATURES {
      for thermal_features in [1, 0x41] {
        assert_eq!(
          select_profile(
            "GenuineIntel",
            signature,
            thermal_features,
            HYBRID_CAPABILITY
          ),
          Some(CoreDtsProfile::HybridSignature(signature))
        );
      }
      assert_eq!(select_profile("GenuineIntel", signature, 0x41, 0), None);
      assert_eq!(
        select_profile("GenuineIntel", signature, 0x40, HYBRID_CAPABILITY),
        None
      );
      assert_eq!(
        select_profile("AuthenticAMD", signature, 0x41, HYBRID_CAPABILITY),
        None
      );
    }
    for other_signature in [
      0x0009_0675,
      0x0009_06a2,
      0x0009_06a4,
      0x000a_06a3,
      0x000a_06a5,
      0x000b_06f2,
      0x000b_0672,
      BLOOMFIELD_SIGNATURE,
    ] {
      assert_eq!(
        select_profile("GenuineIntel", other_signature, 0x41, HYBRID_CAPABILITY),
        None
      );
    }
  }

  #[test]
  fn hybrid_decode_uses_tjmax_at_zero_and_nonzero_tcc_offsets() {
    for signature in HYBRID_SIGNATURES {
      let profile = CoreDtsProfile::HybridSignature(signature);
      for (offset, other_bits) in [(0, 0), (10, 0), (10, 0x55aa), (63, 0xffff)] {
        for (target, digital_readout, expected) in [
          (49, 0, None),
          (50, 0, Some(50.0)),
          (50, 50, None),
          (100, 30, Some(70.0)),
          (100, 0, Some(100.0)),
          (110, 59, Some(51.0)),
          (110, 0, Some(110.0)),
          (120, 0, Some(120.0)),
          (121, 0, None),
        ] {
          let target_word = (target << 16) | (offset << 24) | other_bits;
          let status = (1 << 31) | (digital_readout << 16);
          assert_eq!(
            decode_core_temperature(profile, target_word, status),
            expected
          );
        }
      }
    }
  }

  #[test]
  fn core_decode_requires_validity_a_plausible_target_and_positive_temperature() {
    let valid = 1 << 31;
    assert_eq!(
      decode_core_temperature(LEGACY_PROFILE, 100 << 16, valid | (42 << 16)),
      Some(58.0)
    );
    assert_eq!(
      decode_core_temperature(LEGACY_PROFILE, 50 << 16, valid),
      Some(50.0)
    );
    assert_eq!(
      decode_core_temperature(LEGACY_PROFILE, 120 << 16, valid),
      Some(120.0)
    );
    assert_eq!(
      decode_core_temperature(LEGACY_PROFILE, 50 << 16, valid | (49 << 16)),
      Some(1.0)
    );

    for target in [0, 49, 121] {
      assert_eq!(
        decode_core_temperature(LEGACY_PROFILE, target << 16, valid),
        None
      );
    }
    assert_eq!(
      decode_core_temperature(LEGACY_PROFILE, 50 << 16, 49 << 16),
      None
    );
    assert_eq!(
      decode_core_temperature(LEGACY_PROFILE, 50 << 16, valid | (50 << 16)),
      None
    );
    assert_eq!(
      decode_core_temperature(LEGACY_PROFILE, 50 << 16, valid | (127 << 16)),
      None
    );
    assert_eq!(
      decode_core_temperature(SILVERMONT_PROFILE, 100 << 16, valid),
      Some(100.0)
    );
    for offset in [1, 0x20, 0x3f] {
      assert_eq!(
        decode_core_temperature(SILVERMONT_PROFILE, (100 << 16) | (offset << 24), valid),
        None
      );
    }
  }

  // Synthetic bytes are laid out with the Windows bindings' ABI offsets.
  fn processor_record(relationship: i32, group: u16, mask: usize) -> Vec<u8> {
    let mut data = vec![0; size_of::<PROCESSOR_RELATIONSHIP>()];
    let count = std::mem::offset_of!(PROCESSOR_RELATIONSHIP, GroupCount);
    data[count..count + 2].copy_from_slice(&1u16.to_ne_bytes());
    let affinity = std::mem::offset_of!(PROCESSOR_RELATIONSHIP, GroupMask);
    let mask_offset = affinity + std::mem::offset_of!(GROUP_AFFINITY, Mask);
    data[mask_offset..mask_offset + size_of::<usize>()]
      .copy_from_slice(&mask.to_ne_bytes());
    let group_offset = affinity + std::mem::offset_of!(GROUP_AFFINITY, Group);
    data[group_offset..group_offset + 2].copy_from_slice(&group.to_ne_bytes());
    record(relationship, &data)
  }

  fn group_record(group_count: u16, mask: usize) -> Vec<u8> {
    let mut data = vec![0; size_of::<GROUP_RELATIONSHIP>()];
    let maximum = std::mem::offset_of!(GROUP_RELATIONSHIP, MaximumGroupCount);
    let active = std::mem::offset_of!(GROUP_RELATIONSHIP, ActiveGroupCount);
    data[maximum..maximum + 2].copy_from_slice(&group_count.to_ne_bytes());
    data[active..active + 2].copy_from_slice(&group_count.to_ne_bytes());
    let group_info = std::mem::offset_of!(GROUP_RELATIONSHIP, GroupInfo);
    data
      [group_info + std::mem::offset_of!(PROCESSOR_GROUP_INFO, MaximumProcessorCount)] =
      usize::BITS as u8;
    data[group_info + std::mem::offset_of!(PROCESSOR_GROUP_INFO, ActiveProcessorCount)] =
      mask.count_ones() as u8;
    let active_mask =
      group_info + std::mem::offset_of!(PROCESSOR_GROUP_INFO, ActiveProcessorMask);
    data[active_mask..active_mask + size_of::<usize>()]
      .copy_from_slice(&mask.to_ne_bytes());
    record(RelationGroup.0, &data)
  }

  fn record(relationship: i32, data: &[u8]) -> Vec<u8> {
    let length = HEADER_SIZE + data.len();
    let mut bytes = vec![0; HEADER_SIZE];
    bytes[..4].copy_from_slice(&relationship.to_ne_bytes());
    bytes[4..8].copy_from_slice(&(length as u32).to_ne_bytes());
    bytes.extend_from_slice(data);
    bytes
  }

  #[test]
  fn windows_records_require_one_complete_disjoint_package_in_one_active_group() {
    let group = group_record(1, 0b1111);
    let package = processor_record(RelationProcessorPackage.0, 0, 0b1111);
    let first = processor_record(RelationProcessorCore.0, 0, 0b0101);
    let second = processor_record(RelationProcessorCore.0, 0, 0b1010);
    let ordered = [
      group.clone(),
      package.clone(),
      first.clone(),
      second.clone(),
    ]
    .concat();
    let reordered = [
      second.clone(),
      group.clone(),
      first.clone(),
      package.clone(),
    ]
    .concat();
    let expected = synthetic_topology(&[0b0101, 0b1010]);
    assert_eq!(parse_topology(&ordered), Some(expected.clone()));
    assert_eq!(parse_topology(&reordered), Some(expected));
    assert_eq!(parse_topology(&ordered[..ordered.len() - 1]), None);

    for invalid in [
      [
        group_record(2, 0b1111),
        package.clone(),
        first.clone(),
        second.clone(),
      ]
      .concat(),
      [
        group.clone(),
        package.clone(),
        first.clone(),
        second.clone(),
        package.clone(),
      ]
      .concat(),
      [group.clone(), package.clone(), first.clone()].concat(),
      [group.clone(), package.clone(), first.clone(), first.clone()].concat(),
      [
        group.clone(),
        package.clone(),
        first.clone(),
        processor_record(RelationProcessorCore.0, 1, 0b1010),
      ]
      .concat(),
      [
        group.clone(),
        package.clone(),
        first.clone(),
        processor_record(RelationProcessorCore.0, 0, 0b11010),
      ]
      .concat(),
      [
        group,
        processor_record(RelationProcessorPackage.0, 0, 0b0111),
        first,
        second,
      ]
      .concat(),
      [0; HEADER_SIZE].to_vec(),
    ] {
      assert_eq!(parse_topology(&invalid), None);
    }
  }

  #[derive(Clone, Copy, PartialEq, Eq)]
  enum Failure {
    Pin,
    Target,
    Status,
    InvalidStatus,
    NoDts,
    DifferentSignature,
    NoHybridFlag,
    MissingCoreType,
    UnknownCoreType,
    MovedAfterStatus,
    Restore,
  }

  #[derive(Debug, PartialEq, Eq)]
  enum Call {
    Pin(u8),
    Read(u8, u64),
    Restore(u8),
  }

  struct Fixture {
    current: Cell<Processor>,
    failure: (u8, Failure),
    calls: RefCell<Vec<Call>>,
    signature: u32,
    thermal_features: u32,
    extended_features: u32,
    target_by_core_type: [u64; 2],
  }

  impl Fixture {
    fn new(failure: (u8, Failure)) -> Self {
      Self {
        current: Cell::new(Processor {
          group: 0,
          number: 7,
        }),
        failure,
        calls: RefCell::default(),
        signature: BLOOMFIELD_SIGNATURE,
        thermal_features: 0x41,
        extended_features: 0,
        target_by_core_type: [100 << 16; 2],
      }
    }

    fn hybrid(signature: u32, failure: (u8, Failure)) -> Self {
      Self {
        signature,
        extended_features: HYBRID_CAPABILITY,
        ..Self::new(failure)
      }
    }
  }

  impl CoreAccess for Fixture {
    type PreviousAffinity = (u8, Processor);

    fn pin(&self, processor: Processor) -> Option<Self::PreviousAffinity> {
      self.calls.borrow_mut().push(Call::Pin(processor.number));
      if self.failure == (processor.number, Failure::Pin) {
        return None;
      }
      Some((processor.number, self.current.replace(processor)))
    }

    fn current_processor(&self) -> Processor {
      self.current.get()
    }

    fn profile(&self) -> Option<CoreDtsProfile> {
      let target = self.current.get().number;
      let signature = if self.failure == (target, Failure::DifferentSignature) {
        if self.signature == ALDER_LAKE_S_HX {
          RAPTOR_LAKE_S_HX
        } else {
          ALDER_LAKE_S_HX
        }
      } else {
        self.signature
      };
      let thermal_features = if self.failure == (target, Failure::NoDts) {
        self.thermal_features & !1
      } else {
        self.thermal_features
      };
      let extended_features = if self.failure == (target, Failure::NoHybridFlag) {
        0
      } else {
        self.extended_features
      };
      select_profile(
        "GenuineIntel",
        signature,
        thermal_features,
        extended_features,
      )
    }

    fn hybrid_core_type(&self) -> Option<u8> {
      assert_ne!(
        self.extended_features, 0,
        "legacy targets do not need leaf 1AH"
      );
      let target = self.current.get().number;
      if self.failure == (target, Failure::MissingCoreType) {
        None
      } else if self.failure == (target, Failure::UnknownCoreType) {
        Some(0x30)
      } else {
        Some(if target == 0 { 0x40 } else { 0x20 })
      }
    }

    fn read_msr(&self, msr: u64) -> Option<u64> {
      let processor = self.current.get();
      self
        .calls
        .borrow_mut()
        .push(Call::Read(processor.number, msr));
      match msr {
        MSR_TEMPERATURE_TARGET if self.failure != (processor.number, Failure::Target) => {
          Some(self.target_by_core_type[usize::from(processor.number != 0)])
        }
        IA32_THERM_STATUS if self.failure != (processor.number, Failure::Status) => {
          if self.failure == (processor.number, Failure::MovedAfterStatus) {
            self.current.set(Processor {
              group: 1,
              ..processor
            });
          }
          let valid = self.failure != (processor.number, Failure::InvalidStatus);
          Some((u64::from(valid) << 31) | (42 << 16))
        }
        MSR_TEMPERATURE_TARGET | IA32_THERM_STATUS => None,
        _ => panic!("Core DTS must never read the package status MSR"),
      }
    }

    fn restore(&self, (number, previous): Self::PreviousAffinity) -> bool {
      self.calls.borrow_mut().push(Call::Restore(number));
      if self.failure == (number, Failure::Restore) {
        return false;
      }
      self.current.set(previous);
      true
    }
  }

  #[test]
  fn hybrid_affinity_samples_one_representative_per_physical_core() {
    // Processor 1 is an SMT sibling of P-core 0. E-cores 2 and 3
    // remain separate physical core records.
    let topology = synthetic_topology(&[0b1000, 0b0011, 0b0100]);
    for signature in HYBRID_SIGNATURES {
      let fixture = Fixture::hybrid(signature, (u8::MAX, Failure::Pin));
      let readings = collect(
        CoreDtsProfile::HybridSignature(signature),
        &topology,
        Instant::now() + Duration::from_secs(1),
        &fixture,
      )
      .unwrap();
      assert_eq!(
        readings
          .iter()
          .map(|reading| (reading.logical_processor, reading.temperature_celsius))
          .collect::<Vec<_>>(),
        [(0, Some(58.0)), (2, Some(58.0)), (3, Some(58.0))]
      );
      let calls = fixture.calls.into_inner();
      for (index, representative) in [0, 2, 3].into_iter().enumerate() {
        assert_eq!(
          calls[index * 4..(index + 1) * 4],
          [
            Call::Pin(representative),
            Call::Read(representative, MSR_TEMPERATURE_TARGET),
            Call::Read(representative, IA32_THERM_STATUS),
            Call::Restore(representative),
          ]
        );
      }
      assert_eq!(calls.len(), 12);
    }
  }

  #[test]
  fn alder_lake_mobile_uses_each_cores_tjmax_and_preserves_partial_results() {
    let topology = synthetic_topology(&[0b0011, 0b0100]);
    for (offset, failure, expected_e_core) in [
      (0, (u8::MAX, Failure::Pin), Some(63.0)),
      (10, (u8::MAX, Failure::Pin), Some(63.0)),
      (10, (2, Failure::InvalidStatus), None),
    ] {
      let mut fixture = Fixture::hybrid(0x0009_06a3, failure);
      fixture.target_by_core_type =
        [(100 << 16) | (offset << 24), (105 << 16) | (offset << 24)];
      let readings = collect(
        CoreDtsProfile::HybridSignature(0x0009_06a3),
        &topology,
        Instant::now() + Duration::from_secs(1),
        &fixture,
      )
      .unwrap();
      assert_eq!(
        readings
          .iter()
          .map(|reading| (reading.logical_processor, reading.temperature_celsius))
          .collect::<Vec<_>>(),
        [(0, Some(58.0)), (2, expected_e_core)]
      );
    }
  }

  #[test]
  fn meteor_lake_without_package_dts_keeps_separate_e_core_slots() {
    // Core 2 represents a compute-tile E-core and core 3 a low-power E-core.
    // Both advertise type 0x20, but each has a separate Windows core record.
    let topology = synthetic_topology(&[0b0011, 0b0100, 0b1000]);
    for (failure, expected_low_power_core) in [
      ((u8::MAX, Failure::Pin), Some(68.0)),
      ((3, Failure::MissingCoreType), None),
      ((3, Failure::InvalidStatus), None),
    ] {
      let mut fixture = Fixture::hybrid(0x000a_06a4, failure);
      fixture.thermal_features = 1;
      fixture.target_by_core_type = [110 << 16, (110 << 16) | (5 << 24)];
      let readings = collect(
        CoreDtsProfile::HybridSignature(0x000a_06a4),
        &topology,
        Instant::now() + Duration::from_secs(1),
        &fixture,
      )
      .unwrap();
      assert_eq!(
        readings
          .iter()
          .map(|reading| (reading.logical_processor, reading.temperature_celsius))
          .collect::<Vec<_>>(),
        [
          (0, Some(68.0)),
          (2, Some(68.0)),
          (3, expected_low_power_core)
        ]
      );
      if failure.1 == Failure::MissingCoreType {
        let calls = fixture.calls.into_inner();
        assert!(calls.contains(&Call::Restore(3)));
        assert!(!calls.iter().any(|call| matches!(call, Call::Read(3, _))));
      }
    }
  }

  #[test]
  fn hybrid_target_rechecks_signature_dts_flag_and_core_type_before_msr_reads() {
    let topology = synthetic_topology(&[0b0011, 0b0100, 0b1000]);
    for signature in HYBRID_SIGNATURES {
      for failure in [
        Failure::DifferentSignature,
        Failure::NoDts,
        Failure::NoHybridFlag,
        Failure::MissingCoreType,
        Failure::UnknownCoreType,
      ] {
        let fixture = Fixture::hybrid(signature, (2, failure));
        let readings = collect(
          CoreDtsProfile::HybridSignature(signature),
          &topology,
          Instant::now() + Duration::from_secs(1),
          &fixture,
        )
        .unwrap();
        assert_eq!(
          readings
            .iter()
            .map(|reading| (reading.logical_processor, reading.temperature_celsius))
            .collect::<Vec<_>>(),
          [(0, Some(58.0)), (2, None), (3, Some(58.0))]
        );
        let calls = fixture.calls.into_inner();
        assert!(calls.contains(&Call::Restore(2)));
        assert!(!calls.iter().any(|call| matches!(call, Call::Read(2, _))));
      }
    }
  }

  #[test]
  fn isolated_failures_preserve_the_slot_and_other_physical_cores() {
    let topology = synthetic_topology(&[0b1010, 0b0101]);
    for failure in [
      Failure::Pin,
      Failure::Target,
      Failure::Status,
      Failure::NoDts,
      Failure::MovedAfterStatus,
    ] {
      let fixture = Fixture::new((0, failure));
      let result = collect(
        LEGACY_PROFILE,
        &topology,
        Instant::now() + Duration::from_secs(1),
        &fixture,
      )
      .unwrap();
      assert_eq!(
        result,
        [
          CpuCoreTemperature {
            processor_group: 0,
            logical_processor: 0,
            temperature_celsius: None
          },
          CpuCoreTemperature {
            processor_group: 0,
            logical_processor: 1,
            temperature_celsius: Some(58.0)
          },
        ]
      );
      assert_eq!(
        fixture.current.get(),
        Processor {
          group: 0,
          number: 7
        }
      );
      let calls = fixture.calls.borrow();
      assert_eq!(calls.contains(&Call::Restore(0)), failure != Failure::Pin);
      assert!(calls.contains(&Call::Restore(1)));
      if matches!(failure, Failure::Pin | Failure::NoDts) {
        assert!(!calls.iter().any(|call| matches!(call, Call::Read(0, _))));
      }
    }
  }

  #[test]
  fn restore_failure_discards_previous_slots_and_stops_the_pass() {
    let topology = synthetic_topology(&[0b001001, 0b010010, 0b100100]);
    let fixture = Fixture::new((1, Failure::Restore));
    assert!(
      collect(
        LEGACY_PROFILE,
        &topology,
        Instant::now() + Duration::from_secs(1),
        &fixture
      )
      .is_none()
    );
    assert_eq!(
      fixture.calls.into_inner(),
      [
        Call::Pin(0),
        Call::Read(0, MSR_TEMPERATURE_TARGET),
        Call::Read(0, IA32_THERM_STATUS),
        Call::Restore(0),
        Call::Pin(1),
        Call::Read(1, MSR_TEMPERATURE_TARGET),
        Call::Read(1, IA32_THERM_STATUS),
        Call::Restore(1),
      ]
    );
  }

  #[test]
  fn expired_pass_does_not_pin_or_read_any_processor() {
    let fixture = Fixture::new((0, Failure::Restore));
    assert!(
      collect(
        LEGACY_PROFILE,
        &synthetic_topology(&[1]),
        Instant::now(),
        &fixture
      )
      .is_none()
    );
    assert!(fixture.calls.into_inner().is_empty());
  }

  #[test]
  fn arrow_lake_uses_the_exact_signature_and_basic_cpuid_leaves() {
    let profile = CoreDtsProfile::HybridSignature(0x000c_0662);
    for (maximum_leaf, thermal_features, missing_leaf, expected) in [
      (0x1a, 1, None, Some(profile)),
      (0x1f, 0x41, None, Some(profile)),
      (0x19, 0x41, None, None),
      (0x1a, 0x41, Some(1), None),
      (0x1a, 0x41, Some(6), None),
      (0x1a, 0x41, Some(7), None),
    ] {
      let actual = profile_from_cpuid(|leaf| {
        if missing_leaf == Some(leaf) {
          return None;
        }
        let (eax, ebx, ecx, edx) = match leaf {
          0 => (
            maximum_leaf,
            u32::from_le_bytes(*b"Genu"),
            u32::from_le_bytes(*b"ntel"),
            u32::from_le_bytes(*b"ineI"),
          ),
          1 => (0x000c_0662, 0, 0, 0),
          6 => (thermal_features, 0, 0, 0),
          7 => (0, 0, 0, HYBRID_CAPABILITY),
          _ => panic!("Arrow Lake discovery only requests the required basic leaves"),
        };
        Some(CpuidLeaf { eax, ebx, ecx, edx })
      });
      assert_eq!(actual, expected);
    }

    for (vendor, signature, thermal_features, extended_features) in [
      ("AuthenticAMD", 0x000c_0662, 0x41, HYBRID_CAPABILITY),
      ("GenuineIntel", 0x000c_0661, 0x41, HYBRID_CAPABILITY),
      ("GenuineIntel", 0x000c_0664, 0x41, HYBRID_CAPABILITY),
      ("GenuineIntel", 0x000c_0652, 0x41, HYBRID_CAPABILITY),
      ("GenuineIntel", 0x000c_0662, 0x40, HYBRID_CAPABILITY),
      ("GenuineIntel", 0x000c_0662, 0x41, 0),
    ] {
      assert_eq!(
        select_profile(vendor, signature, thermal_features, extended_features),
        None
      );
    }
  }

  #[test]
  fn arrow_lake_uses_calibrated_tjmax_and_valid_delta() {
    let profile = CoreDtsProfile::HybridSignature(0x000c_0662);
    let valid = 1 << 31;
    for target in [0x0069_1400, 0x8269_0000] {
      assert_eq!(
        decode_core_temperature(profile, target, 0x8835_2800),
        Some(52.0)
      );
      assert_eq!(decode_core_temperature(profile, target, valid), Some(105.0));
    }
    for (target, expected) in [
      (49, None),
      (50, Some(50.0)),
      (120, Some(120.0)),
      (121, None),
    ] {
      assert_eq!(
        decode_core_temperature(profile, target << 16, valid),
        expected
      );
    }
    assert_eq!(decode_core_temperature(profile, 105 << 16, 53 << 16), None);
    assert_eq!(
      decode_core_temperature(profile, 105 << 16, valid | (105 << 16)),
      None
    );
  }

  #[test]
  fn arrow_lake_retains_the_eligible_core_when_its_sibling_is_unavailable() {
    let profile = CoreDtsProfile::HybridSignature(0x000c_0662);
    let topology = synthetic_topology(&[0b0001, 0b0100]);
    for (failure, expected_e_core, should_read) in [
      ((u8::MAX, Failure::Pin), Some(63.0), true),
      ((2, Failure::DifferentSignature), None, false),
      ((2, Failure::NoDts), None, false),
      ((2, Failure::NoHybridFlag), None, false),
      ((2, Failure::MissingCoreType), None, false),
      ((2, Failure::UnknownCoreType), None, false),
      ((2, Failure::Status), None, true),
      ((2, Failure::InvalidStatus), None, true),
      ((2, Failure::MovedAfterStatus), None, true),
    ] {
      let mut fixture = Fixture::hybrid(0x000c_0662, failure);
      fixture.target_by_core_type = [0x0069_1400, 0x8269_0000];
      let readings = collect(
        profile,
        &topology,
        Instant::now() + Duration::from_secs(1),
        &fixture,
      )
      .unwrap();
      assert_eq!(
        readings
          .iter()
          .map(|reading| (reading.logical_processor, reading.temperature_celsius))
          .collect::<Vec<_>>(),
        [(0, Some(63.0)), (2, expected_e_core)]
      );
      let calls = fixture.calls.into_inner();
      assert!(calls.contains(&Call::Restore(2)));
      assert_eq!(
        calls.iter().any(|call| matches!(call, Call::Read(2, _))),
        should_read
      );
    }
  }
}
