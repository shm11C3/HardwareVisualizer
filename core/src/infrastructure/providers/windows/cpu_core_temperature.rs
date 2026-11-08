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
  cpu_identity::{CpuVendor, cpuid_leaf, detect_cpu_identity},
  cpu_temperature_decode::{
    decode_intel_package_temperature, decode_intel_temperature_target,
  },
  pawn_io::{PawnIoClient, PawnIoModule},
};
use crate::models::CpuCoreTemperature;

const MSR_TEMPERATURE_TARGET: u64 = 0x1a2;
const IA32_THERM_STATUS: u64 = 0x19c;
const SILVERMONT_ATOM_C2000: u32 = 0x4d;
const MAX_TOPOLOGY_BYTES: usize = 64 * 1024;
const HEADER_SIZE: usize =
  std::mem::offset_of!(SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX, Anonymous);

pub(crate) fn supported_model() -> Option<u32> {
  if !cfg!(target_arch = "x86_64") {
    return None;
  }
  let cpu = detect_cpu_identity();
  select_model(cpu.vendor, cpu.family, cpu.model, cpuid_leaf(6)?.eax)
}

fn select_model(
  vendor: CpuVendor,
  family: u32,
  model: u32,
  thermal_features: u32,
) -> Option<u32> {
  (vendor == CpuVendor::Intel
    && family == 6
    && matches!(
      model,
      0x1a | 0x1e | 0x1f | 0x2e | 0x2a | 0x2d | SILVERMONT_ATOM_C2000
    )
    && thermal_features & 1 != 0)
    .then_some(model)
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
  model: u32,
  topology: &CoreTopology,
  deadline: Instant,
) -> Option<Vec<CpuCoreTemperature>> {
  if Instant::now() >= deadline {
    return None;
  }
  // A synchronous request can outlive its wait budget. Give this disposable
  // thread its own client so an in-flight call holds no package sampler lock.
  let (client, _) = PawnIoClient::open(PawnIoModule::IntelMsr).ok()?;
  collect(model, topology, deadline, &client)
}

trait CoreAccess {
  type PreviousAffinity;

  fn pin(&self, processor: Processor) -> Option<Self::PreviousAffinity>;
  fn current_processor(&self) -> Processor;
  fn model(&self) -> Option<u32>;
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

  fn model(&self) -> Option<u32> {
    supported_model()
  }

  fn read_msr(&self, msr: u64) -> Option<u64> {
    PawnIoClient::read_msr(self, msr).ok()
  }

  fn restore(&self, previous: Self::PreviousAffinity) -> bool {
    unsafe { SetThreadGroupAffinity(GetCurrentThread(), &previous, None) }.as_bool()
  }
}

fn collect(
  model: u32,
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
      let reading = sample_core(model, processor, deadline, access);
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
  model: u32,
  processor: Processor,
  deadline: Instant,
  access: &impl CoreAccess,
) -> Option<f32> {
  if access.current_processor() != processor
    || access.model() != Some(model)
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

  decode_core_temperature(model, target, status)
}

fn decode_core_temperature(model: u32, target: u64, status: u64) -> Option<f32> {
  if status & (1 << 31) == 0
    || (model == SILVERMONT_ATOM_C2000 && (target >> 24) & 0x3f != 0)
  {
    return None;
  }

  // The core and package digital readout fields use the same seven bits.
  let target = decode_intel_temperature_target(target).ok()?;
  decode_intel_package_temperature(target, status).ok()
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::cell::{Cell, RefCell};
  use std::time::Duration;
  use windows::Win32::System::SystemInformation::PROCESSOR_GROUP_INFO;

  #[test]
  fn core_model_gate_is_independent_of_package_capability() {
    // Synthetic leaf-6 values reflect the two DTS-only observations in the spec.
    for (model, leaf6) in [(0x1a, 0x03), (0x4d, 0x05), (0x2a, 0x41)] {
      assert_eq!(select_model(CpuVendor::Intel, 6, model, leaf6), Some(model));
    }
    for leaf6 in [0, 0x40] {
      assert_eq!(select_model(CpuVendor::Intel, 6, 0x2a, leaf6), None);
    }
    assert_eq!(select_model(CpuVendor::Intel, 6, 0x4c, 0x41), None);
    assert_eq!(select_model(CpuVendor::Intel, 5, 0x4d, 0x41), None);
    assert_eq!(select_model(CpuVendor::Amd, 6, 0x4d, 0x41), None);
  }

  #[test]
  fn core_decode_requires_validity_a_plausible_target_and_positive_temperature() {
    let valid = 1 << 31;
    assert_eq!(
      decode_core_temperature(0x1a, 100 << 16, valid | (42 << 16)),
      Some(58.0)
    );
    assert_eq!(decode_core_temperature(0x1a, 50 << 16, valid), Some(50.0));
    assert_eq!(decode_core_temperature(0x1a, 120 << 16, valid), Some(120.0));
    assert_eq!(
      decode_core_temperature(0x1a, 50 << 16, valid | (49 << 16)),
      Some(1.0)
    );

    for target in [0, 49, 121] {
      assert_eq!(decode_core_temperature(0x1a, target << 16, valid), None);
    }
    assert_eq!(decode_core_temperature(0x1a, 50 << 16, 49 << 16), None);
    assert_eq!(
      decode_core_temperature(0x1a, 50 << 16, valid | (50 << 16)),
      None
    );
    assert_eq!(
      decode_core_temperature(0x1a, 50 << 16, valid | (127 << 16)),
      None
    );
    assert_eq!(decode_core_temperature(0x4d, 100 << 16, valid), Some(100.0));
    for offset in [1, 0x20, 0x3f] {
      assert_eq!(
        decode_core_temperature(0x4d, (100 << 16) | (offset << 24), valid),
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
    Model,
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

    fn model(&self) -> Option<u32> {
      (self.failure != (self.current.get().number, Failure::Model)).then_some(0x1a)
    }

    fn read_msr(&self, msr: u64) -> Option<u64> {
      let processor = self.current.get();
      self
        .calls
        .borrow_mut()
        .push(Call::Read(processor.number, msr));
      match msr {
        MSR_TEMPERATURE_TARGET if self.failure != (processor.number, Failure::Target) => {
          Some(100 << 16)
        }
        IA32_THERM_STATUS if self.failure != (processor.number, Failure::Status) => {
          if self.failure == (processor.number, Failure::MovedAfterStatus) {
            self.current.set(Processor {
              group: 1,
              ..processor
            });
          }
          Some((1 << 31) | (42 << 16))
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
  fn isolated_failures_preserve_the_slot_and_other_physical_cores() {
    let topology = synthetic_topology(&[0b1010, 0b0101]);
    for failure in [
      Failure::Pin,
      Failure::Target,
      Failure::Status,
      Failure::Model,
      Failure::MovedAfterStatus,
    ] {
      let fixture = Fixture::new((0, failure));
      let result = collect(
        0x1a,
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
      if matches!(failure, Failure::Pin | Failure::Model) {
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
        0x1a,
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
    assert!(collect(0x1a, &synthetic_topology(&[1]), Instant::now(), &fixture).is_none());
    assert!(fixture.calls.into_inner().is_empty());
  }
}
