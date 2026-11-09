//! Experimental CCD monitoring from the approved Vermeer rev 5 and Raphael
//! rev 1 specifications under `docs/specs/sensors/`.

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
  super::client::PawnIoClient,
  identity::{CpuidLeaf, cpuid_leaf, effective_family_model, vendor_id_from_leaf0},
};
use crate::models::CpuCcdTemperature;

const AMD_VENDOR_ID: &str = "AuthenticAMD";
/// Family `0x19` model `0x21` is the Vermeer die. Every stepping shares the
/// CCD register map, so the stepping field takes no part in admission.
const VERMEER_FAMILY_MODEL: (u32, u32) = (0x19, 0x21);
const AM4_PACKAGE_FIELD: u32 = 0x2;
const RAPHAEL_SIGNATURE: u32 = 0x00A6_0F12;
const AM5_PACKAGE_FIELD: u32 = 0x0;
const MAX_PACKAGE_RECORD_BYTES: usize = 64 * 1024;
const HEADER_SIZE: usize =
  std::mem::offset_of!(SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX, Anonymous);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CcdRegisterMap {
  Vermeer,
  Raphael,
}

impl CcdRegisterMap {
  /// Authorized SMN words in fixed slot order for this mechanism.
  const fn slots(self) -> [u64; 2] {
    match self {
      Self::Vermeer => [0x0005_9954, 0x0005_9958],
      Self::Raphael => [0x0005_9B08, 0x0005_9B0C],
    }
  }
}

/// Experimental target admitted by the CPU mechanism and a single physical
/// package. The processor name is not read for admission.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct AmdCcdTarget(CcdRegisterMap);

impl AmdCcdTarget {
  pub(crate) fn detect() -> Option<Self> {
    authorize_target(processor_register_map(), query_single_package)
  }

  pub(crate) fn read(&self, client: &PawnIoClient) -> Vec<CpuCcdTemperature> {
    self.read_with(|address| u32::try_from(client.read_smu_register(address).ok()?).ok())
  }

  pub(crate) fn unavailable_readings(&self) -> Vec<CpuCcdTemperature> {
    self.read_with(|_| None)
  }

  /// Reads each authorized word once, in slot order, with no retries. A slot
  /// that fails stays unavailable in its own position; slot 2 is never
  /// remapped onto slot 1.
  fn read_with(
    &self,
    mut read: impl FnMut(u64) -> Option<u32>,
  ) -> Vec<CpuCcdTemperature> {
    self
      .0
      .slots()
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

/// The processor-side mechanism gates. The service also validates the
/// physical package topology before authorizing the selected map.
pub(crate) fn is_supported_processor() -> bool {
  processor_register_map().is_some()
}

fn processor_register_map() -> Option<CcdRegisterMap> {
  if !cfg!(target_arch = "x86_64") {
    return None;
  }
  select_register_map(cpuid_leaf)
}

fn select_register_map(
  read_leaf: impl Fn(u32) -> Option<CpuidLeaf>,
) -> Option<CcdRegisterMap> {
  let leaf0 = read_leaf(0)?;
  if vendor_id_from_leaf0(leaf0) != AMD_VENDOR_ID {
    return None;
  }

  let signature = read_leaf(1)?.eax;
  let (map, package_field) = if signature == RAPHAEL_SIGNATURE {
    (CcdRegisterMap::Raphael, AM5_PACKAGE_FIELD)
  } else if effective_family_model(signature) == VERMEER_FAMILY_MODEL {
    (CcdRegisterMap::Vermeer, AM4_PACKAGE_FIELD)
  } else {
    return None;
  };
  (read_leaf(0x8000_0001)?.ebx >> 28 == package_field).then_some(map)
}

fn authorize_target(
  map: Option<CcdRegisterMap>,
  single_package: impl FnOnce() -> Option<()>,
) -> Option<AmdCcdTarget> {
  let map = map?;
  single_package()?;
  Some(AmdCcdTarget(map))
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
pub(crate) fn synthetic_target() -> AmdCcdTarget {
  authorize_target(Some(CcdRegisterMap::Vermeer), || Some(())).unwrap()
}

#[cfg(test)]
pub(crate) fn synthetic_raphael_target() -> AmdCcdTarget {
  authorize_target(Some(CcdRegisterMap::Raphael), || Some(())).unwrap()
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;

  use super::*;

  const AMD: &[u8; 12] = b"AuthenticAMD";
  const INTEL: &[u8; 12] = b"GenuineIntel";
  const VERMEER_B0: u32 = 0x00a2_0f10;
  const VERMEER_B2: u32 = 0x00a2_0f12;
  const CEZANNE: u32 = 0x00a5_0f00;
  const RAPHAEL_B2: u32 = 0x00a6_0f12;
  // Family 0x17 and 0x1A signatures built from the extended-field encoding.
  const FAMILY_17H: u32 = (0x8 << 20) | (0x7 << 16) | 0x0f10;
  const FAMILY_1AH: u32 = (0xb << 20) | (0x4 << 16) | 0x0f40;
  const AM4: u32 = 0x2ff0_0000;
  const NOT_AM4: u32 = 0x3000_0000;
  const BRAND_LEAVES: std::ops::RangeInclusive<u32> = 0x8000_0002..=0x8000_0004;

  const OBSERVED_NAMES: [&[u8]; 3] = [
    b"AMD Ryzen 7 5800X 8-Core Processor",
    b"AMD Ryzen 9 5900X 12-Core Processor",
    b"AMD Ryzen 9 5950X 16-Core Processor",
  ];
  const SYNTHETIC_NAMES: [&[u8]; 3] = [
    b"AMD Ryzen 5 5600X 6-Core Processor",
    b"AMD Ryzen 7 5800X3D 8-Core Processor",
    b"AMD Ryzen 7 5800XT 8-Core Processor",
  ];

  /// Synthetic CPUID identity. The brand is optional so the missing-name
  /// fixture row can be exercised.
  struct Identity {
    vendor: &'static [u8; 12],
    signature: u32,
    package: u32,
    brand: Option<&'static [u8]>,
  }

  impl Identity {
    fn leaf(&self, leaf: u32) -> Option<CpuidLeaf> {
      let word = |bytes: &[u8]| u32::from_le_bytes(bytes.try_into().unwrap());
      match leaf {
        0 => Some(CpuidLeaf {
          eax: 0x10,
          ebx: word(&self.vendor[..4]),
          ecx: word(&self.vendor[8..]),
          edx: word(&self.vendor[4..8]),
        }),
        1 => Some(CpuidLeaf {
          eax: self.signature,
          ebx: 0,
          ecx: 0,
          edx: 0,
        }),
        0x8000_0000 => Some(CpuidLeaf {
          eax: if self.brand.is_some() {
            0x8000_0004
          } else {
            0x8000_0001
          },
          ebx: 0,
          ecx: 0,
          edx: 0,
        }),
        0x8000_0001 => Some(CpuidLeaf {
          eax: self.signature,
          ebx: self.package,
          ecx: 0,
          edx: 0,
        }),
        leaf if BRAND_LEAVES.contains(&leaf) => {
          let name = self.brand?;
          let mut padded = [0; 48];
          padded[..name.len()].copy_from_slice(name);
          let chunk = &padded[(leaf - 0x8000_0002) as usize * 16..][..16];
          Some(CpuidLeaf {
            eax: word(&chunk[..4]),
            ebx: word(&chunk[4..8]),
            ecx: word(&chunk[8..12]),
            edx: word(&chunk[12..]),
          })
        }
        _ => None,
      }
    }
  }

  fn names() -> impl Iterator<Item = Option<&'static [u8]>> {
    OBSERVED_NAMES
      .into_iter()
      .chain(SYNTHETIC_NAMES)
      .map(Some)
      .chain(std::iter::once(None))
  }

  /// Runs the identity gate and records which CPUID leaves it requested.
  fn identify(identity: &Identity) -> (Option<CcdRegisterMap>, Vec<u32>) {
    let requested = RefCell::new(Vec::new());
    let map = select_register_map(|leaf| {
      requested.borrow_mut().push(leaf);
      identity.leaf(leaf)
    });
    (map, requested.into_inner())
  }

  #[test]
  fn vermeer_die_is_admitted_at_any_stepping_without_reading_the_processor_name() {
    for signature in [VERMEER_B0, VERMEER_B2] {
      for brand in names() {
        let (map, requested) = identify(&Identity {
          vendor: AMD,
          signature,
          package: AM4,
          brand,
        });
        assert_eq!(
          map,
          Some(CcdRegisterMap::Vermeer),
          "signature {signature:#010x}, name {brand:?}"
        );
        assert_eq!(requested, [0, 1, 0x8000_0001]);
      }
    }
  }

  #[test]
  fn exact_raphael_am5_signature_selects_its_map_without_reading_the_name() {
    for brand in [
      Some(b"AMD Ryzen 9 7950X 16-Core Processor".as_slice()),
      Some(b"AMD Ryzen 5 7600 6-Core Processor".as_slice()),
      None,
    ] {
      let (map, requested) = identify(&Identity {
        vendor: AMD,
        signature: RAPHAEL_B2,
        package: 0,
        brand,
      });
      assert_eq!(map, Some(CcdRegisterMap::Raphael));
      assert_eq!(requested, [0, 1, 0x8000_0001]);
    }
  }

  #[test]
  fn other_dies_vendors_and_packages_are_rejected_regardless_of_name() {
    assert_eq!(effective_family_model(CEZANNE), (0x19, 0x50));
    assert_eq!(effective_family_model(RAPHAEL_B2), (0x19, 0x61));
    assert_eq!(effective_family_model(FAMILY_17H).0, 0x17);
    assert_eq!(effective_family_model(FAMILY_1AH).0, 0x1a);

    for (vendor, signature, package) in [
      (AMD, CEZANNE, AM4),
      (AMD, RAPHAEL_B2, AM4),
      (AMD, RAPHAEL_B2, 0x1000_0000), // Dragon Range FL1
      (AMD, RAPHAEL_B2, NOT_AM4),
      (AMD, 0x00a6_0f11, 0), // Different stepping of model 0x61
      (AMD, 0x00a6_0f22, 0), // Adjacent model 0x62
      (INTEL, RAPHAEL_B2, 0),
      (AMD, FAMILY_17H, AM4),
      (AMD, FAMILY_1AH, AM4),
      (INTEL, VERMEER_B0, AM4),
      (AMD, VERMEER_B0, NOT_AM4),
      (AMD, VERMEER_B0, 0),
    ] {
      for brand in names() {
        let (map, requested) = identify(&Identity {
          vendor,
          signature,
          package,
          brand,
        });
        assert!(
          map.is_none(),
          "vendor {:?}, signature {signature:#010x}, package {package:#010x}, name {brand:?}",
          std::str::from_utf8(vendor).unwrap()
        );
        assert!(requested.iter().all(|leaf| !BRAND_LEAVES.contains(leaf)));
      }
    }
  }

  #[test]
  fn missing_cpuid_leaves_refuse_admission() {
    for (signature, package) in [(VERMEER_B0, AM4), (RAPHAEL_B2, 0)] {
      let identity = Identity {
        vendor: AMD,
        signature,
        package,
        brand: None,
      };
      for missing in [0, 1, 0x8000_0001] {
        assert!(
          select_register_map(|leaf| (leaf != missing).then(|| identity.leaf(leaf))?)
            .is_none()
        );
      }
    }
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

    let invalid_records = [
      [first.as_slice(), second.as_slice()].concat(),
      first[..first.len() - 1].to_vec(),
      package_record(&[(0, 1), (0, 2)]),
      package_record(&[(1, 0)]),
      package_record(&[]),
      Vec::new(),
    ];
    for map in [CcdRegisterMap::Vermeer, CcdRegisterMap::Raphael] {
      assert!(
        authorize_target(Some(map), || validate_single_package_record(&first)).is_some()
      );
      for invalid in &invalid_records {
        assert!(
          authorize_target(Some(map), || validate_single_package_record(invalid))
            .is_none()
        );
      }
      assert!(authorize_target(Some(map), || None).is_none());
    }
    assert!(
      authorize_target(None, || panic!(
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
  fn both_slots_are_read_once_in_fixed_order_and_fail_independently() {
    let package = package_record(&[(0, 0b1111)]);
    let samples = [
      ([Some(0x0ac8), Some(0x0b38)], [Some(40.0), Some(54.0)]),
      // A later sample loses slot 1 validity. Slot 2 keeps its identity.
      ([Some(0x02c8), Some(0x0b38)], [None, Some(54.0)]),
      ([None, Some(0x0b38)], [None, Some(54.0)]),
      ([Some(0), Some(0x0b38)], [None, Some(54.0)]),
      ([Some(u32::MAX), Some(0x0b38)], [None, Some(54.0)]),
      // The inverse, including the single-CCD pattern with bit 11 clear.
      ([Some(0x0ac8), None], [Some(40.0), None]),
      ([Some(0x0ac8), Some(0x02c8)], [Some(40.0), None]),
      ([Some(0x0ac8), Some(0)], [Some(40.0), None]),
      ([Some(0x0ac8), Some(u32::MAX)], [Some(40.0), None]),
      ([None, None], [None, None]),
    ];

    for (map, expected_addresses) in [
      (CcdRegisterMap::Vermeer, [0x0005_9954, 0x0005_9958]),
      (CcdRegisterMap::Raphael, [0x0005_9B08, 0x0005_9B0C]),
    ] {
      let target =
        authorize_target(Some(map), || validate_single_package_record(&package)).unwrap();
      for (words, expected) in samples {
        let mut addresses = Vec::new();
        let mut words = words.into_iter();
        let result = target.read_with(|address| {
          addresses.push(address);
          words.next().expect("each CCD is requested at most once")
        });
        assert_eq!(addresses, expected_addresses);
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
      assert_eq!(
        target.unavailable_readings(),
        target.read_with(|_| None),
        "a missing module or mutex reports both slots unavailable without reads"
      );
    }
  }
}
