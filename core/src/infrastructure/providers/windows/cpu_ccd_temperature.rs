use std::collections::HashSet;

use windows::Win32::{
  Foundation::ERROR_INSUFFICIENT_BUFFER,
  System::SystemInformation::{
    GROUP_AFFINITY, GetLogicalProcessorInformationEx, PROCESSOR_RELATIONSHIP,
    RelationProcessorPackage, SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
  },
};
use windows::core::HRESULT;

use super::{
  cpu_identity::{cpu_brand_bytes, cpuid_leaf, vendor_id_from_leaf0},
  pawn_io::PawnIoClient,
};
use crate::models::CpuCcdTemperature;

const VERMEER_B0_SIGNATURE: u32 = 0x00a2_0f10;
const AM4_PACKAGE_FIELD: u32 = 0x2;
const RYZEN_5900X_NAME: &[u8] = b"AMD Ryzen 9 5900X 12-Core Processor";
const CCD_ADDRESSES: [u64; 2] = [0x0005_9954, 0x0005_9958];
const MAX_PACKAGE_RECORD_BYTES: usize = 64 * 1024;
const HEADER_SIZE: usize =
  std::mem::offset_of!(SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX, Anonymous);

/// Experimental target authorized by the exact processor and package gates.
pub(crate) struct VermeerCcdTarget(());

impl VermeerCcdTarget {
  pub(crate) fn detect() -> Option<Self> {
    authorize_target(is_supported_processor(), query_single_package)
  }

  pub(crate) fn read(&self, client: &PawnIoClient) -> Vec<CpuCcdTemperature> {
    self.read_with(|address| u32::try_from(client.read_smu_register(address).ok()?).ok())
  }

  pub(crate) fn unavailable_readings(&self) -> Vec<CpuCcdTemperature> {
    self.read_with(|_| None)
  }

  fn read_with(
    &self,
    mut read: impl FnMut(u64) -> Option<u32>,
  ) -> Vec<CpuCcdTemperature> {
    CCD_ADDRESSES
      .into_iter()
      .enumerate()
      .map(|(index, address)| CpuCcdTemperature {
        package_index: 0,
        ccd_slot: index as u8 + 1,
        temperature_celsius: read(address).and_then(decode_ccd_temperature),
      })
      .collect()
  }
}

pub(crate) fn is_supported_processor() -> bool {
  fn detect_identity() -> Option<bool> {
    let vendor = vendor_id_from_leaf0(cpuid_leaf(0)?);
    let signature = cpuid_leaf(1)?.eax;
    let package = cpuid_leaf(0x8000_0001)?.ebx;
    let name = cpu_brand_bytes()?;
    Some(matches_identity(&vendor, signature, package, &name))
  }

  cfg!(target_arch = "x86_64") && detect_identity().unwrap_or(false)
}

fn matches_identity(vendor: &str, signature: u32, package: u32, name: &[u8]) -> bool {
  vendor == "AuthenticAMD"
    && signature == VERMEER_B0_SIGNATURE
    && package >> 28 == AM4_PACKAGE_FIELD
    && name
      .strip_prefix(RYZEN_5900X_NAME)
      .is_some_and(|suffix| suffix.iter().all(|&byte| matches!(byte, b' ' | 0)))
}

fn authorize_target(
  exact_processor: bool,
  single_package: impl FnOnce() -> Option<()>,
) -> Option<VermeerCcdTarget> {
  if !exact_processor {
    return None;
  }

  single_package()?;
  Some(VermeerCcdTarget(()))
}

fn query_single_package() -> Option<()> {
  let mut length = 0;
  let error = unsafe {
    GetLogicalProcessorInformationEx(RelationProcessorPackage, None, &mut length)
  }
  .err()?;
  if error.code() != HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0)
    || !(HEADER_SIZE..=MAX_PACKAGE_RECORD_BYTES).contains(&(length as usize))
  {
    return None;
  }

  let capacity = length;
  // The Windows API requires an aligned buffer for variable-size records.
  let mut words = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
  unsafe {
    GetLogicalProcessorInformationEx(
      RelationProcessorPackage,
      Some(words.as_mut_ptr().cast()),
      &mut length,
    )
  }
  .ok()?;
  if length > capacity {
    return None;
  }

  // The allocation is initialized and covers the returned byte count.
  let bytes =
    unsafe { std::slice::from_raw_parts(words.as_ptr().cast(), length as usize) };
  validate_single_package_record(bytes)
}

fn validate_single_package_record(bytes: &[u8]) -> Option<()> {
  let relationship = i32::from_ne_bytes(bytes.get(..4)?.try_into().ok()?);
  let size = u32::from_ne_bytes(bytes.get(4..8)?.try_into().ok()?) as usize;
  if relationship != RelationProcessorPackage.0 || size != bytes.len() {
    return None;
  }

  // The package record ends in GroupCount group-affinity records. A single
  // physical package can span multiple processor groups.
  let data = bytes.get(HEADER_SIZE..)?;
  let group_count_offset = std::mem::offset_of!(PROCESSOR_RELATIONSHIP, GroupCount);
  let group_count = u16::from_ne_bytes(
    data
      .get(group_count_offset..group_count_offset + 2)?
      .try_into()
      .ok()?,
  ) as usize;
  let groups = data.get(std::mem::offset_of!(PROCESSOR_RELATIONSHIP, GroupMask)..)?;
  if group_count == 0 || groups.len() != group_count * size_of::<GROUP_AFFINITY>() {
    return None;
  }

  let mut seen = HashSet::with_capacity(group_count);
  for group in groups.as_chunks::<{ size_of::<GROUP_AFFINITY>() }>().0 {
    // GROUP_AFFINITY contains only integer fields and the chunk covers the
    // entire record. The source buffer may be unaligned at this offset.
    let affinity = unsafe { group.as_ptr().cast::<GROUP_AFFINITY>().read_unaligned() };
    if affinity.Mask == 0 || !seen.insert(affinity.Group) {
      return None;
    }
  }
  Some(())
}

fn decode_ccd_temperature(value: u32) -> Option<f32> {
  if value == u32::MAX || value & (1 << 11) == 0 {
    return None;
  }

  let temperature = (value & 0x7ff) as f32 * 0.125 - 49.0;
  (-40.0..=120.0)
    .contains(&temperature)
    .then_some(temperature)
}

#[cfg(test)]
pub(crate) fn synthetic_target() -> VermeerCcdTarget {
  authorize_target(true, || Some(())).unwrap()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn only_the_exact_amd_5900x_signature_package_and_padded_brand_are_admitted() {
    let mut name = [0; 48];
    name[..RYZEN_5900X_NAME.len()].copy_from_slice(RYZEN_5900X_NAME);
    name[RYZEN_5900X_NAME.len()] = b' ';
    let vendor = "AuthenticAMD";
    let signature = 0x00a2_0f10;
    let am4 = 0x2ff0_0000;
    assert!(matches_identity(vendor, signature, am4, &name));
    assert!(!matches_identity("GenuineIntel", signature, am4, &name));
    assert!(!matches_identity(vendor, signature | 2, am4, &name));
    assert!(!matches_identity(vendor, signature, 0x3000_0000, &name));
    assert!(!matches_identity(
      vendor,
      signature,
      am4,
      b"AMD Ryzen 9 5950X 16-Core Processor"
    ));
    assert!(!matches_identity(
      vendor,
      signature,
      am4,
      b" AMD Ryzen 9 5900X 12-Core Processor"
    ));
    name[47] = b'X';
    assert!(!matches_identity(vendor, signature, am4, &name));
  }

  // Synthetic byte records use the Windows bindings' ABI field offsets.
  fn package_record(group_masks: &[(u16, usize)]) -> Vec<u8> {
    let count_offset =
      HEADER_SIZE + std::mem::offset_of!(PROCESSOR_RELATIONSHIP, GroupCount);
    let masks_offset =
      HEADER_SIZE + std::mem::offset_of!(PROCESSOR_RELATIONSHIP, GroupMask);
    let length = masks_offset + group_masks.len() * size_of::<GROUP_AFFINITY>();
    let mut bytes = vec![0; length];
    bytes[..4].copy_from_slice(&RelationProcessorPackage.0.to_ne_bytes());
    bytes[4..8].copy_from_slice(&(length as u32).to_ne_bytes());
    bytes[count_offset..count_offset + 2]
      .copy_from_slice(&(group_masks.len() as u16).to_ne_bytes());

    for (&(group, mask), record) in group_masks.iter().zip(
      bytes[masks_offset..]
        .as_chunks_mut::<{ size_of::<GROUP_AFFINITY>() }>()
        .0,
    ) {
      let mask_offset = std::mem::offset_of!(GROUP_AFFINITY, Mask);
      record[mask_offset..mask_offset + size_of::<usize>()]
        .copy_from_slice(&mask.to_ne_bytes());
      let group_offset = std::mem::offset_of!(GROUP_AFFINITY, Group);
      record[group_offset..group_offset + 2].copy_from_slice(&group.to_ne_bytes());
    }
    bytes
  }

  #[test]
  fn topology_admits_one_valid_package_and_refuses_ambiguous_or_missing_records() {
    let first = package_record(&[(0, 0b1111)]);
    let second = package_record(&[(0, 0b1111_0000)]);
    assert_eq!(validate_single_package_record(&first), Some(()));
    assert_eq!(
      validate_single_package_record(&package_record(&[(0, 1), (1, 1)])),
      Some(())
    );

    for invalid in [
      [first.as_slice(), second.as_slice()].concat(),
      first[..first.len() - 1].to_vec(),
      package_record(&[(0, 1), (0, 2)]),
      package_record(&[(1, 0)]),
      package_record(&[]),
      Vec::new(),
    ] {
      assert!(
        authorize_target(true, || validate_single_package_record(&invalid)).is_none()
      );
    }
    assert!(authorize_target(true, || None).is_none());
    assert!(
      authorize_target(false, || panic!(
        "An unsupported processor cannot query topology"
      ))
      .is_none()
    );
  }

  #[test]
  fn temperatures_use_only_valid_words_within_the_plausibility_limits() {
    for (word, expected) in [
      (0x0ac8, Some(40.0)),
      (0x0b38, Some(54.0)),
      (0x0988, Some(0.0)),
      (0x0848, Some(-40.0)),
      (0x0d48, Some(120.0)),
      (0x0847, None),
      (0x0d49, None),
      (0x02c8, None),
      (0, None),
      (u32::MAX, None),
    ] {
      assert_eq!(decode_ccd_temperature(word), expected);
    }
  }

  #[test]
  fn read_failures_do_not_shift_slots_or_scan_extra_addresses() {
    let target = synthetic_target();
    for (words, expected) in [
      ([Some(0x0ac8), Some(0x0b38)], [Some(40.0), Some(54.0)]),
      ([None, Some(0x0b38)], [None, Some(54.0)]),
      ([Some(0x0ac8), None], [Some(40.0), None]),
      ([Some(0x02c8), Some(0x0b38)], [None, Some(54.0)]),
      ([Some(0), Some(0x0b38)], [None, Some(54.0)]),
      ([Some(u32::MAX), Some(0x0b38)], [None, Some(54.0)]),
      ([None, None], [None, None]),
    ] {
      let mut addresses = Vec::new();
      let mut words = words.into_iter();
      let result = target.read_with(|address| {
        addresses.push(address);
        words.next().expect("each CCD is requested at most once")
      });
      assert_eq!(addresses, [0x0005_9954, 0x0005_9958]);
      assert_eq!(
        result,
        [
          CpuCcdTemperature {
            package_index: 0,
            ccd_slot: 1,
            temperature_celsius: expected[0]
          },
          CpuCcdTemperature {
            package_index: 0,
            ccd_slot: 2,
            temperature_celsius: expected[1]
          },
        ]
      );
    }
  }
}
