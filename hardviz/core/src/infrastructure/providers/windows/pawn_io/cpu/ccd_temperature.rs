//! Experimental CCD monitoring from the approved AMD CCD specifications
//! under `docs/specs/sensors/`.

use std::collections::HashMap;

use windows::Win32::{
  Foundation::ERROR_INSUFFICIENT_BUFFER,
  System::SystemInformation::{
    GROUP_AFFINITY, GROUP_RELATIONSHIP, GetLogicalProcessorInformationEx,
    LOGICAL_PROCESSOR_RELATIONSHIP, PROCESSOR_GROUP_INFO, PROCESSOR_RELATIONSHIP,
    RelationAll, RelationGroup, RelationProcessorPackage,
    SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
  },
};
use windows::core::HRESULT;

use super::{
  super::client::{PawnIoClient, PawnIoError},
  identity::{CpuidLeaf, cpuid_leaf, effective_family_model, vendor_id_from_leaf0},
};
use crate::models::CpuCcdTemperature;

const AMD_VENDOR_ID: &str = "AuthenticAMD";
const AM4_PACKAGE_FIELD: u32 = 0x2;
const AM5_PACKAGE_FIELD: u32 = 0x0;
const FL1_PACKAGE_FIELD: u32 = 0x1;
const SP5_PACKAGE_FIELD: u32 = 0x4;
const STRX4_PACKAGE_FIELD: u32 = 0x7;
const MAX_PACKAGE_RECORD_BYTES: usize = 64 * 1024;
const HEADER_SIZE: usize =
  std::mem::offset_of!(SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX, Anonymous);

/// How a row recognizes its processor from the CPUID leaf 1 signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessorMatch {
  /// The exact signature, stepping included.
  Signature(u32),
  /// The effective family and model. Every stepping shares the register map,
  /// so the stepping field takes no part in admission.
  FamilyModel(u32, u32),
}

impl ProcessorMatch {
  fn matches(self, signature: u32) -> bool {
    match self {
      Self::Signature(expected) => signature == expected,
      Self::FamilyModel(family, model) => {
        effective_family_model(signature) == (family, model)
      }
    }
  }
}

/// One CCD mechanism, as its own specification admits it.
#[derive(Debug, PartialEq, Eq)]
struct CcdRegisterMap {
  processor: ProcessorMatch,
  /// The required CPUID `Fn8000_0001_EBX[31:28]` package type.
  package_field: u32,
  /// Authorized SMN words in fixed slot order.
  slots: &'static [u64],
}

const VERMEER: CcdRegisterMap = CcdRegisterMap {
  processor: ProcessorMatch::FamilyModel(0x19, 0x21),
  package_field: AM4_PACKAGE_FIELD,
  slots: &[0x0005_9954, 0x0005_9958],
};

const MATISSE: CcdRegisterMap = CcdRegisterMap {
  processor: ProcessorMatch::Signature(0x0087_0F10),
  package_field: AM4_PACKAGE_FIELD,
  slots: &[0x0005_9954, 0x0005_9958],
};

const CASTLE_PEAK: CcdRegisterMap = CcdRegisterMap {
  processor: ProcessorMatch::Signature(0x0083_0F10),
  package_field: STRX4_PACKAGE_FIELD,
  slots: &[
    0x0005_9954,
    0x0005_9958,
    0x0005_995C,
    0x0005_9960,
    0x0005_9964,
    0x0005_9968,
    0x0005_996C,
    0x0005_9970,
  ],
};

// Each specification independently pins these SMN words.
const RAPHAEL: CcdRegisterMap = CcdRegisterMap {
  processor: ProcessorMatch::Signature(0x00A6_0F12),
  package_field: AM5_PACKAGE_FIELD,
  slots: &[0x0005_9B08, 0x0005_9B0C],
};

const DRAGON_RANGE: CcdRegisterMap = CcdRegisterMap {
  processor: ProcessorMatch::Signature(0x00A6_0F12),
  package_field: FL1_PACKAGE_FIELD,
  slots: &[0x0005_9B08, 0x0005_9B0C],
};

const GRANITE_RIDGE: CcdRegisterMap = CcdRegisterMap {
  processor: ProcessorMatch::Signature(0x00B4_0F40),
  package_field: AM5_PACKAGE_FIELD,
  slots: &[0x0005_9B08, 0x0005_9B0C],
};

const FIRE_RANGE: CcdRegisterMap = CcdRegisterMap {
  processor: ProcessorMatch::Signature(0x00B4_0F40),
  package_field: FL1_PACKAGE_FIELD,
  slots: &[0x0005_9B08, 0x0005_9B0C],
};

const GENOA_B1: CcdRegisterMap = CcdRegisterMap {
  processor: ProcessorMatch::Signature(0x00A1_0F11),
  package_field: SP5_PACKAGE_FIELD,
  slots: &[
    0x0005_9B00,
    0x0005_9B04,
    0x0005_9B08,
    0x0005_9B0C,
    0x0005_9B10,
    0x0005_9B14,
    0x0005_9B18,
    0x0005_9B1C,
    0x0005_9B20,
    0x0005_9B24,
    0x0005_9B28,
    0x0005_9B2C,
  ],
};

const CCD_REGISTER_MAPS: [&CcdRegisterMap; 8] = [
  &VERMEER,
  &RAPHAEL,
  &DRAGON_RANGE,
  &GRANITE_RIDGE,
  &FIRE_RANGE,
  &MATISSE,
  &CASTLE_PEAK,
  &GENOA_B1,
];

/// Experimental target admitted by the CPU mechanism and a single physical
/// package. The processor name is not read for admission.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct AmdCcdTarget(&'static CcdRegisterMap);

impl AmdCcdTarget {
  pub(crate) fn detect() -> Option<Self> {
    let map = processor_register_map()?;
    authorize_target(Some(map), || {
      if *map == GENOA_B1 {
        // The fixed boot-package route is only authorized if that package
        // covers every active processor group in the same OS snapshot.
        query_relationships(RelationAll, validate_active_single_package)
      } else {
        is_single_physical_package().then_some(())
      }
    })
  }

  pub(crate) fn read(&self, client: &PawnIoClient) -> Vec<CpuCcdTemperature> {
    self.read_with(|address| smu_word(client.read_smu_register(address)))
  }

  pub(crate) fn unavailable_readings(&self) -> Vec<CpuCcdTemperature> {
    self.read_with(|_| None)
  }

  pub(crate) fn readings_without_module(&self) -> Option<Vec<CpuCcdTemperature>> {
    // Dragon Range, Fire Range and Genoa require a loaded module before
    // exposing slots.
    (![&DRAGON_RANGE, &FIRE_RANGE, &GENOA_B1].contains(&self.0))
      .then(|| self.unavailable_readings())
  }

  /// Reads each authorized word once, in slot order, with no retries. A slot
  /// that fails stays unavailable in its own position.
  fn read_with(
    &self,
    mut read: impl FnMut(u64) -> Option<u32>,
  ) -> Vec<CpuCcdTemperature> {
    self
      .0
      .slots
      .iter()
      .copied()
      .enumerate()
      .map(|(index, address)| CpuCcdTemperature {
        package_index: 0,
        ccd_slot: index as u8 + 1,
        temperature_celsius: read(address).and_then(decode_ccd_temperature),
      })
      .collect()
  }
}

/// Whether `GetLogicalProcessorInformationEx(RelationProcessorPackage)`
/// succeeds and returns exactly one physical package record. A failed query
/// is not evidence of a single package.
pub(super) fn is_single_physical_package() -> bool {
  query_relationships(RelationProcessorPackage, validate_single_package_record).is_some()
}

/// The processor-side mechanism gates. The service also validates the
/// physical package topology before authorizing the selected map.
pub(crate) fn is_supported_processor() -> bool {
  processor_register_map().is_some()
}

fn processor_register_map() -> Option<&'static CcdRegisterMap> {
  if !cfg!(target_arch = "x86_64") {
    return None;
  }
  select_register_map(cpuid_leaf)
}

fn select_register_map(
  read_leaf: impl Fn(u32) -> Option<CpuidLeaf>,
) -> Option<&'static CcdRegisterMap> {
  let leaf0 = read_leaf(0)?;
  if vendor_id_from_leaf0(leaf0) != AMD_VENDOR_ID {
    return None;
  }

  let signature = read_leaf(1)?.eax;
  let package_field = read_leaf(0x8000_0001)?.ebx >> 28;
  CCD_REGISTER_MAPS
    .into_iter()
    .find(|map| map.processor.matches(signature) && map.package_field == package_field)
}

fn authorize_target(
  map: Option<&'static CcdRegisterMap>,
  single_package: impl FnOnce() -> Option<()>,
) -> Option<AmdCcdTarget> {
  let map = map?;
  single_package()?;
  Some(AmdCcdTarget(map))
}

fn query_relationships(
  relationship: LOGICAL_PROCESSOR_RELATIONSHIP,
  validate: impl FnOnce(&[u8]) -> Option<()>,
) -> Option<()> {
  let mut length = 0;
  let error =
    unsafe { GetLogicalProcessorInformationEx(relationship, None, &mut length) }.err()?;
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
      relationship,
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
  validate(bytes)
}

fn validate_single_package_record(bytes: &[u8]) -> Option<()> {
  parse_single_package_masks(bytes).map(|_| ())
}

fn parse_single_package_masks(bytes: &[u8]) -> Option<HashMap<u16, usize>> {
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

  let mut masks = HashMap::with_capacity(group_count);
  for group in groups.as_chunks::<{ size_of::<GROUP_AFFINITY>() }>().0 {
    // GROUP_AFFINITY contains only integer fields and the chunk covers the
    // entire record. The source buffer may be unaligned at this offset.
    let affinity = unsafe { group.as_ptr().cast::<GROUP_AFFINITY>().read_unaligned() };
    if affinity.Mask == 0 || masks.insert(affinity.Group, affinity.Mask).is_some() {
      return None;
    }
  }
  Some(masks)
}

fn validate_active_single_package(mut bytes: &[u8]) -> Option<()> {
  let mut package = None;
  let mut active = None;
  while !bytes.is_empty() {
    let relationship = i32::from_ne_bytes(bytes.get(..4)?.try_into().ok()?);
    let size = u32::from_ne_bytes(bytes.get(4..8)?.try_into().ok()?) as usize;
    let record = bytes.get(..size)?;
    let data = record.get(HEADER_SIZE..)?;
    bytes = &bytes[size..];
    if relationship == RelationProcessorPackage.0 {
      if package
        .replace(parse_single_package_masks(record)?)
        .is_some()
      {
        return None;
      }
    } else if relationship == RelationGroup.0
      && active.replace(parse_active_group_masks(data)?).is_some()
    {
      return None;
    }
  }
  (package? == active?).then_some(())
}

fn parse_active_group_masks(data: &[u8]) -> Option<HashMap<u16, usize>> {
  let count_offset = std::mem::offset_of!(GROUP_RELATIONSHIP, ActiveGroupCount);
  let active =
    u16::from_ne_bytes(data.get(count_offset..count_offset + 2)?.try_into().ok()?);
  let maximum = u16::from_ne_bytes(data.get(..2)?.try_into().ok()?);
  let groups = data.get(std::mem::offset_of!(GROUP_RELATIONSHIP, GroupInfo)..)?;
  if active == 0
    || active > maximum
    || groups.len() != usize::from(active) * size_of::<PROCESSOR_GROUP_INFO>()
  {
    return None;
  }

  let mut masks = HashMap::with_capacity(usize::from(active));
  for (index, group) in groups
    .as_chunks::<{ size_of::<PROCESSOR_GROUP_INFO>() }>()
    .0
    .iter()
    .enumerate()
  {
    // GroupInfo is indexed by group number. Each chunk contains a complete
    // integer-only record, but it may be unaligned in this byte slice.
    let info = unsafe {
      group
        .as_ptr()
        .cast::<PROCESSOR_GROUP_INFO>()
        .read_unaligned()
    };
    let count = u32::from(info.ActiveProcessorCount);
    if count == 0
      || count != info.ActiveProcessorMask.count_ones()
      || info.ActiveProcessorCount > info.MaximumProcessorCount
      || u32::from(info.MaximumProcessorCount) > usize::BITS
    {
      return None;
    }
    masks.insert(index as u16, info.ActiveProcessorMask);
  }
  Some(masks)
}

/// A failed signed read, including a mutex that was denied or timed out
/// before the request, leaves its slot without a word.
fn smu_word(result: Result<u64, PawnIoError>) -> Option<u32> {
  u32::try_from(result.ok()?).ok()
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
  authorize_target(Some(&VERMEER), || Some(())).unwrap()
}

#[cfg(test)]
pub(crate) fn synthetic_raphael_target() -> AmdCcdTarget {
  authorize_target(Some(&RAPHAEL), || Some(())).unwrap()
}

#[cfg(test)]
pub(crate) fn synthetic_dragon_range_target() -> AmdCcdTarget {
  authorize_target(Some(&DRAGON_RANGE), || Some(())).unwrap()
}

#[cfg(test)]
pub(crate) fn synthetic_fire_range_target() -> AmdCcdTarget {
  authorize_target(Some(&FIRE_RANGE), || Some(())).unwrap()
}

#[cfg(test)]
pub(crate) fn synthetic_genoa_target() -> AmdCcdTarget {
  authorize_target(Some(&GENOA_B1), || Some(())).unwrap()
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;

  use windows::Win32::Foundation::{E_ACCESSDENIED, WAIT_FAILED};

  use super::*;
  use crate::infrastructure::providers::windows::pawn_io::client::{
    ACCESS_PCI_MUTEX, MutexError,
  };

  #[test]
  fn no_processor_and_package_select_more_than_one_register_map() {
    for (index, map) in CCD_REGISTER_MAPS.iter().enumerate() {
      for other in &CCD_REGISTER_MAPS[index + 1..] {
        let overlaps = match (map.processor, other.processor) {
          (ProcessorMatch::Signature(signature), processor)
          | (processor, ProcessorMatch::Signature(signature)) => {
            processor.matches(signature)
          }
          (ProcessorMatch::FamilyModel(..), ProcessorMatch::FamilyModel(..)) => {
            map.processor == other.processor
          }
        };
        assert!(
          !overlaps || map.package_field != other.package_field,
          "{map:?} overlaps {other:?}"
        );
      }
    }
  }

  const AMD: &[u8; 12] = b"AuthenticAMD";
  const INTEL: &[u8; 12] = b"GenuineIntel";
  const VERMEER_B0: u32 = 0x00a2_0f10;
  const VERMEER_B2: u32 = 0x00a2_0f12;
  const CEZANNE: u32 = 0x00a5_0f00;
  const RAPHAEL_B2: u32 = 0x00a6_0f12;
  const FL1: u32 = 0x1000_0000;
  const GRANITE_RIDGE_B0: u32 = 0x00b4_0f40;
  const MATISSE_B0: u32 = 0x0087_0f10;
  const CASTLE_PEAK_B0: u32 = 0x0083_0f10;
  const STRX4: u32 = 0x7000_0000;
  const GENOA_B1_SIGNATURE: u32 = 0x00a1_0f11;
  const SP5: u32 = 0x4000_0000;
  const OTHER_FAMILY_1AH: u32 = 0x00b4_0f50;
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
  fn identify(identity: &Identity) -> (Option<&'static CcdRegisterMap>, Vec<u32>) {
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
          Some(&VERMEER),
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
      assert_eq!(map, Some(&RAPHAEL));
      assert_eq!(requested, [0, 1, 0x8000_0001]);
    }
  }

  #[test]
  fn shared_signature_selects_dragon_range_only_with_fl1() {
    for brand in [Some(b"AMD Ryzen 9 7945HX".as_slice()), None] {
      let (map, requested) = identify(&Identity {
        vendor: AMD,
        signature: RAPHAEL_B2,
        package: FL1,
        brand,
      });
      assert_eq!(map, Some(&DRAGON_RANGE));
      assert_eq!(requested, [0, 1, 0x8000_0001]);
    }
  }

  #[test]
  fn exact_granite_ridge_am5_signature_selects_its_map_without_reading_the_name() {
    for brand in [
      Some(b"AMD Ryzen 9 9950X 16-Core Processor".as_slice()),
      Some(b"AMD Ryzen 9 9950X3D 16-Core Processor".as_slice()),
      Some(b"AMD EPYC 4565P 16-Core Processor".as_slice()),
      None,
    ] {
      let (map, requested) = identify(&Identity {
        vendor: AMD,
        signature: GRANITE_RIDGE_B0,
        package: 0,
        brand,
      });
      assert_eq!(map, Some(&GRANITE_RIDGE));
      assert_eq!(requested, [0, 1, 0x8000_0001]);
    }
  }

  const RYZEN_9_9955HX: &[u8] = b"AMD Ryzen 9 9955HX 16-Core Processor";
  const FIRE_RANGE_WORDS: [u64; 2] = [0x0005_9B08, 0x0005_9B0C];

  fn fire_range_identity(package: u32, brand: Option<&'static [u8]>) -> Identity {
    Identity {
      vendor: AMD,
      signature: GRANITE_RIDGE_B0,
      package,
      brand,
    }
  }

  fn slot_temperatures(readings: &[CpuCcdTemperature]) -> Vec<(u8, u8, Option<f32>)> {
    readings
      .iter()
      .map(|reading| {
        (
          reading.package_index,
          reading.ccd_slot,
          reading.temperature_celsius,
        )
      })
      .collect()
  }

  /// Fire Range rev 1, Synthetic fixture acceptance row 1 (admission). The
  /// gate never requests a brand leaf, so a changed, absent or desktop
  /// name cannot change it. No laptop or firmware identity is an input.
  #[test]
  fn fire_range_fl1_selects_its_map_without_reading_the_name() {
    for brand in [
      Some(RYZEN_9_9955HX),
      Some(b"AMD Ryzen 9 9955HX3D 16-Core Processor".as_slice()),
      Some(b"AMD Ryzen 9 9850HX 12-Core Processor".as_slice()),
      Some(b"AMD Ryzen 9 9950X 16-Core Processor".as_slice()),
      None,
    ] {
      let (map, requested) = identify(&fire_range_identity(FL1, brand));
      assert_eq!(map, Some(&FIRE_RANGE), "name {brand:?}");
      assert_eq!(requested, [0, 1, 0x8000_0001]);
    }
  }

  /// Fire Range rev 1, fixture rows 2 and 3. Field 0 stays with Granite
  /// Ridge, field 1 is Fire Range, and every other field is outside both.
  /// Only EBX bits 31:28 select the package.
  #[test]
  fn fire_range_signature_routes_each_package_field_to_at_most_one_scope() {
    for field in 0..=0xf_u32 {
      let expected = match field {
        0 => Some(&GRANITE_RIDGE),
        1 => Some(&FIRE_RANGE),
        _ => None,
      };
      for low_bits in [0, 0x0fff_ffff] {
        let package = (field << 28) | low_bits;
        let (map, requested) =
          identify(&fire_range_identity(package, Some(RYZEN_9_9955HX)));
        assert_eq!(map, expected, "EBX {package:#010x}");
        assert_eq!(requested, [0, 1, 0x8000_0001]);
      }
    }
  }

  /// Fire Range rev 1, fixture rows 1 and 4 (topology and requests). One
  /// package record admits the target. The only CCD requests are the two
  /// fixed words, once each, decoded to 40 °C and 54 °C.
  #[test]
  fn fire_range_reads_two_fixed_words_on_exactly_one_package() {
    let (map, _) = identify(&fire_range_identity(FL1, None));
    let first = package_record(&[(0, 0xffff)]);
    let second = package_record(&[(0, 0xffff_0000)]);
    for refused in [
      [first.as_slice(), second.as_slice()].concat(),
      package_record(&[(0, 1), (0, 2)]),
      package_record(&[]),
      Vec::new(),
    ] {
      assert!(
        authorize_target(map, || validate_single_package_record(&refused)).is_none()
      );
    }

    let target =
      authorize_target(map, || validate_single_package_record(&first)).unwrap();
    let mut addresses = Vec::new();
    let readings = target.read_with(|address| {
      addresses.push(address);
      match address {
        0x0005_9B08 => Some(0x0AC8),
        0x0005_9B0C => Some(0x0B38),
        _ => panic!("unexpected Fire Range address: {address:#x}"),
      }
    });
    assert_eq!(addresses, FIRE_RANGE_WORDS);
    assert_eq!(
      slot_temperatures(&readings),
      [(0, 1, Some(40.0)), (0, 2, Some(54.0))]
    );
  }

  /// Fire Range rev 1, fixture row 6 in both slot positions. The boundary
  /// values survive and one increment beyond either limit is unavailable.
  #[test]
  fn fire_range_slots_keep_the_plausibility_limits_in_either_position() {
    let target = synthetic_fire_range_target();
    for (word, expected) in [
      (0x0988, Some(0.0)),
      (0x0848, Some(-40.0)),
      (0x0D48, Some(120.0)),
      (0x0847, None),
      (0x0D49, None),
    ] {
      let mut words = [Some(word), Some(0x0AC8)].into_iter();
      assert_eq!(
        slot_temperatures(&target.read_with(|_| words.next().unwrap())),
        [(0, 1, expected), (0, 2, Some(40.0))]
      );
      let mut words = [Some(0x0AC8), Some(word)].into_iter();
      assert_eq!(
        slot_temperatures(&target.read_with(|_| words.next().unwrap())),
        [(0, 1, Some(40.0)), (0, 2, expected)]
      );
    }
  }

  /// Fire Range rev 1, fixture row 7 (mutex). A denied or timed-out
  /// `Global\Access_PCI` acquisition fails that slot's signed request. The
  /// slot keeps its identity without a temperature, the sibling survives,
  /// and nothing is retried, in both slot orders.
  #[test]
  fn fire_range_mutex_failure_leaves_only_its_slot_unavailable() {
    let target = synthetic_fire_range_target();
    let denied = windows::core::Error::from_hresult(E_ACCESSDENIED);
    for failure in [
      MutexError::Timeout {
        name: ACCESS_PCI_MUTEX,
      },
      MutexError::OpenOrCreate {
        name: ACCESS_PCI_MUTEX,
        open: denied.clone(),
        create: denied,
      },
      MutexError::Wait {
        name: ACCESS_PCI_MUTEX,
        wait: WAIT_FAILED,
      },
    ] {
      for (locked, expected) in [
        (FIRE_RANGE_WORDS[0], [(0, 1, None), (0, 2, Some(54.0))]),
        (FIRE_RANGE_WORDS[1], [(0, 1, Some(54.0)), (0, 2, None)]),
      ] {
        let mut attempts = Vec::new();
        let readings = target.read_with(|address| {
          attempts.push(address);
          smu_word(if address == locked {
            Err(PawnIoError::Mutex(failure.clone()))
          } else {
            Ok(0x0B38)
          })
        });
        assert_eq!(attempts, FIRE_RANGE_WORDS, "{failure}");
        assert_eq!(slot_temperatures(&readings), expected, "{failure}");
      }
    }
  }

  /// Fire Range rev 1, fixture row 7 (module). Without the signed module
  /// the profile returns no target, unlike Granite Ridge on the same
  /// signature, which keeps its slots unavailable.
  #[test]
  fn fire_range_returns_no_target_without_the_module() {
    assert!(
      synthetic_fire_range_target()
        .readings_without_module()
        .is_none()
    );
    let granite_ridge = authorize_target(Some(&GRANITE_RIDGE), || Some(())).unwrap();
    assert_eq!(
      granite_ridge.readings_without_module(),
      Some(granite_ridge.unavailable_readings())
    );
  }

  #[test]
  fn other_dies_vendors_and_packages_are_rejected_regardless_of_name() {
    assert_eq!(effective_family_model(CEZANNE), (0x19, 0x50));
    assert_eq!(effective_family_model(RAPHAEL_B2), (0x19, 0x61));
    assert_eq!(effective_family_model(MATISSE_B0), (0x17, 0x71));
    assert_eq!(effective_family_model(GRANITE_RIDGE_B0), (0x1a, 0x44));
    assert_eq!(effective_family_model(OTHER_FAMILY_1AH), (0x1a, 0x45));

    for (vendor, signature, package) in [
      (AMD, CEZANNE, AM4),
      (AMD, RAPHAEL_B2, AM4),
      (AMD, RAPHAEL_B2, NOT_AM4),
      (AMD, 0x00a6_0f11, 0), // Different stepping of model 0x61
      (AMD, 0x00a6_0f11, FL1),
      (AMD, 0x00a6_0f22, 0), // Adjacent model 0x62
      (AMD, GENOA_B1_SIGNATURE, STRX4),
      (AMD, 0x00a1_0f12, SP5), // Genoa B2
      (INTEL, GENOA_B1_SIGNATURE, SP5),
      (INTEL, RAPHAEL_B2, 0),
      (INTEL, RAPHAEL_B2, FL1),
      (AMD, GRANITE_RIDGE_B0, NOT_AM4),
      (AMD, 0x00b4_0f41, 0),   // Granite Ridge B1
      (AMD, 0x00b4_0f41, FL1), // Stepping 1 is outside Fire Range rev 1
      (AMD, OTHER_FAMILY_1AH, FL1),
      (INTEL, GRANITE_RIDGE_B0, 0),
      (INTEL, GRANITE_RIDGE_B0, FL1),
      (AMD, 0x0087_0f11, AM4), // Matisse with a different stepping
      (AMD, MATISSE_B0, STRX4),
      (INTEL, MATISSE_B0, AM4),
      (AMD, 0x0083_0f11, STRX4), // Castle Peak with a different stepping
      (AMD, CASTLE_PEAK_B0, 0x4000_0000), // sWRX8 / SP3 encoding
      (AMD, CASTLE_PEAK_B0, 0),
      (INTEL, CASTLE_PEAK_B0, STRX4),
      (AMD, OTHER_FAMILY_1AH, 0),
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
    for (signature, package) in [
      (VERMEER_B0, AM4),
      (RAPHAEL_B2, 0),
      (RAPHAEL_B2, FL1),
      (GRANITE_RIDGE_B0, 0),
      (GRANITE_RIDGE_B0, FL1),
      (MATISSE_B0, AM4),
      (CASTLE_PEAK_B0, STRX4),
      (GENOA_B1_SIGNATURE, SP5),
    ] {
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
    for map in CCD_REGISTER_MAPS {
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

  fn active_group_record(masks: &[usize]) -> Vec<u8> {
    let group_offset = HEADER_SIZE + std::mem::offset_of!(GROUP_RELATIONSHIP, GroupInfo);
    let length = group_offset + masks.len() * size_of::<PROCESSOR_GROUP_INFO>();
    let mut bytes = vec![0; length];
    bytes[..4].copy_from_slice(&RelationGroup.0.to_ne_bytes());
    bytes[4..8].copy_from_slice(&(length as u32).to_ne_bytes());
    for offset in [
      std::mem::offset_of!(GROUP_RELATIONSHIP, MaximumGroupCount),
      std::mem::offset_of!(GROUP_RELATIONSHIP, ActiveGroupCount),
    ] {
      let start = HEADER_SIZE + offset;
      bytes[start..start + 2].copy_from_slice(&(masks.len() as u16).to_ne_bytes());
    }
    for (&mask, group) in masks.iter().zip(
      bytes[group_offset..]
        .as_chunks_mut::<{ size_of::<PROCESSOR_GROUP_INFO>() }>()
        .0,
    ) {
      group[std::mem::offset_of!(PROCESSOR_GROUP_INFO, MaximumProcessorCount)] =
        usize::BITS as u8;
      group[std::mem::offset_of!(PROCESSOR_GROUP_INFO, ActiveProcessorCount)] =
        mask.count_ones() as u8;
      let start = std::mem::offset_of!(PROCESSOR_GROUP_INFO, ActiveProcessorMask);
      group[start..start + size_of::<usize>()].copy_from_slice(&mask.to_ne_bytes());
    }
    bytes
  }

  #[test]
  fn genoa_package_must_cover_every_active_group_mask() {
    let groups = active_group_record(&[usize::MAX, 0xff]);
    let package = package_record(&[(1, 0xff), (0, usize::MAX)]);
    let extra_package = package_record(&[(1, 1)]);
    let missing_bits = package_record(&[(1, 0x0f), (0, usize::MAX)]);
    let wrong_bits = package_record(&[(1, 0xff00), (0, usize::MAX)]);
    let wrong_group = package_record(&[(2, 0xff), (0, usize::MAX)]);
    let malformed_groups = &groups[..groups.len() - 1];

    for records in [
      [groups.as_slice(), package.as_slice()].concat(),
      [package.as_slice(), groups.as_slice()].concat(),
    ] {
      assert_eq!(validate_active_single_package(&records), Some(()));
    }
    for records in [
      groups.clone(),
      package.clone(),
      [&groups[..], &package[..], &extra_package[..]].concat(),
      [&groups[..], &groups[..], &package[..]].concat(),
      [&groups[..], &missing_bits[..]].concat(),
      [&groups[..], &wrong_bits[..]].concat(),
      [&groups[..], &wrong_group[..]].concat(),
      [&package[..], malformed_groups].concat(),
    ] {
      assert!(validate_active_single_package(&records).is_none());
    }
  }

  #[test]
  fn genoa_b1_sp5_keeps_twelve_fixed_slots_and_rechecks_validity() {
    let (map, requested) = identify(&Identity {
      vendor: AMD,
      signature: GENOA_B1_SIGNATURE,
      package: SP5,
      brand: None,
    });
    assert_eq!(map, Some(&GENOA_B1));
    assert_eq!(requested, [0, 1, 0x8000_0001]);
    let package = package_record(&[(0, usize::MAX), (1, 0xff)]);
    let topology = [package, active_group_record(&[usize::MAX, 0xff])].concat();
    let target =
      authorize_target(map, || validate_active_single_package(&topology)).unwrap();
    let expected = [
      0x0005_9B00,
      0x0005_9B04,
      0x0005_9B08,
      0x0005_9B0C,
      0x0005_9B10,
      0x0005_9B14,
      0x0005_9B18,
      0x0005_9B1C,
      0x0005_9B20,
      0x0005_9B24,
      0x0005_9B28,
      0x0005_9B2C,
    ];
    for (slot_3_word, slot_3_temperature) in [(0x0ac4, Some(39.5)), (0x02c4, None)] {
      let mut requested = Vec::new();
      let readings = target.read_with(|address| {
        requested.push(address);
        match address {
          0x0005_9B00 => Some(0x0848),
          0x0005_9B04 => None,
          0x0005_9B08 => Some(slot_3_word),
          0x0005_9B0C => Some(u32::MAX),
          0x0005_9B10 => Some(0x0847),
          0x0005_9B14 => Some(0x0d49),
          0x0005_9B20 => Some(0x0d48),
          0x0005_9B2C => Some(0x0b00),
          _ => Some(0),
        }
      });
      assert_eq!(requested, expected);
      assert_eq!(readings.len(), expected.len());
      for (index, reading) in readings.iter().enumerate() {
        assert_eq!(reading.package_index, 0);
        assert_eq!(usize::from(reading.ccd_slot), index + 1);
        assert_eq!(
          reading.temperature_celsius,
          match reading.ccd_slot {
            1 => Some(-40.0),
            3 => slot_3_temperature,
            9 => Some(120.0),
            12 => Some(47.0),
            _ => None,
          }
        );
      }
    }
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
  fn matisse_am4_reads_two_fixed_ccd_words() {
    let (map, requested) = identify(&Identity {
      vendor: AMD,
      signature: MATISSE_B0,
      package: AM4,
      brand: Some(b"AMD Ryzen 9 3900X 12-Core Processor"),
    });
    assert_eq!(map, Some(&MATISSE));
    assert_eq!(requested, [0, 1, 0x8000_0001]);
    let package = package_record(&[(0, 0b1111)]);
    let target =
      authorize_target(map, || validate_single_package_record(&package)).unwrap();
    let mut addresses = Vec::new();
    let values = target.read_with(|address| {
      addresses.push(address);
      match address {
        0x0005_9954 => Some(0x0B76),
        0x0005_9958 => Some(0x0BEC),
        _ => panic!("unexpected Matisse address: {address:#x}"),
      }
    });
    assert_eq!(addresses, [0x0005_9954, 0x0005_9958]);
    assert_eq!(
      values
        .iter()
        .map(|value| (
          value.package_index,
          value.ccd_slot,
          value.temperature_celsius
        ))
        .collect::<Vec<_>>(),
      [(0, 1, Some(61.75)), (0, 2, Some(76.5))]
    );
  }

  #[test]
  fn castle_peak_strx4_keeps_eight_slots_on_one_package_across_groups() {
    let (map, requested) = identify(&Identity {
      vendor: AMD,
      signature: CASTLE_PEAK_B0,
      package: STRX4,
      brand: None,
    });
    assert_eq!(map, Some(&CASTLE_PEAK));
    assert_eq!(requested, [0, 1, 0x8000_0001]);
    let package = package_record(&[(0, usize::MAX), (1, usize::MAX)]);
    let target =
      authorize_target(map, || validate_single_package_record(&package)).unwrap();
    let expected_addresses = [
      0x0005_9954,
      0x0005_9958,
      0x0005_995C,
      0x0005_9960,
      0x0005_9964,
      0x0005_9968,
      0x0005_996C,
      0x0005_9970,
    ];
    let mut addresses = Vec::new();
    let mut words = [
      Some(0x0ABC),
      Some(0),
      Some(0x0AD8),
      None,
      Some(0x0ADE),
      Some(u32::MAX),
      Some(0x0AE4),
      Some(0x02C8),
    ]
    .into_iter();
    let values = target.read_with(|address| {
      addresses.push(address);
      words.next().expect("only the eight scoped words")
    });
    assert_eq!(addresses, expected_addresses);
    assert_eq!(
      values
        .iter()
        .map(|value| (
          value.package_index,
          value.ccd_slot,
          value.temperature_celsius
        ))
        .collect::<Vec<_>>(),
      [
        (0, 1, Some(38.5)),
        (0, 2, None),
        (0, 3, Some(42.0)),
        (0, 4, None),
        (0, 5, Some(42.75)),
        (0, 6, None),
        (0, 7, Some(43.5)),
        (0, 8, None),
      ]
    );
    let mut all_valid = (0x0AC8..0x0AD0).map(Some);
    let values = target.read_with(|_| all_valid.next().expect("one word per slot"));
    assert_eq!(
      values
        .iter()
        .map(|value| value.temperature_celsius)
        .collect::<Vec<_>>(),
      [
        Some(40.0),
        Some(40.125),
        Some(40.25),
        Some(40.375),
        Some(40.5),
        Some(40.625),
        Some(40.75),
        Some(40.875),
      ]
    );
    let unavailable = target.unavailable_readings();
    assert_eq!(unavailable.len(), 8);
    assert_eq!(unavailable[7].ccd_slot, 8);
    assert!(
      unavailable
        .iter()
        .all(|value| value.temperature_celsius.is_none())
    );
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
      ([Some(0x0847), Some(0x0d48)], [None, Some(120.0)]),
      ([Some(0x0848), Some(0x0d49)], [Some(-40.0), None]),
      ([None, None], [None, None]),
    ];

    for (map, expected_addresses) in [
      (&VERMEER, [0x0005_9954, 0x0005_9958]),
      (&MATISSE, [0x0005_9954, 0x0005_9958]),
      (&RAPHAEL, [0x0005_9B08, 0x0005_9B0C]),
      (&DRAGON_RANGE, [0x0005_9B08, 0x0005_9B0C]),
      (&GRANITE_RIDGE, [0x0005_9B08, 0x0005_9B0C]),
      (&FIRE_RANGE, [0x0005_9B08, 0x0005_9B0C]),
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
        "failed requests preserve both fixed positions"
      );
    }
  }
}
