//! Pure helpers and the scoped chip-profile table for the early Nuvoton
//! hardware monitors that report temperatures through six source-selected
//! slots (SMIOVT1-SMIOVT6) instead of the fixed bank 4 bytes of
//! [`super::nuvoton`].
//!
//! Enabled rows: NCT6776F/NCT6776D raw `0xC330`-`0xC337` (temperatures and
//! five direct RPM fans, Experimental) and NCT6775F/NCT6771F raw
//! `0xB470`-`0xB477` (temperatures only, Experimental, lead-only). Every
//! NCT610x ID, NCT6775-family fans, non-diode slot sources and voltages stay
//! unsupported.
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-nuvoton-nct6775-nct6776-nct610x.md`
//!   revision 1
//!
//! No other external sensor source was used.

use super::is_plausible_super_io_temperature;
use super::nuvoton::NUVOTON_GENERIC_SOURCE_LABEL;

/// Hardware-monitor banks the profiles read ("Read procedure and decode",
/// Sampling step 2). No other bank is ever selected.
pub const SMIOVT_BANK_0: u8 = 0x00;
pub const SMIOVT_BANK_1: u8 = 0x01;
pub const SMIOVT_BANK_2: u8 = 0x02;
pub const SMIOVT_BANK_6: u8 = 0x06;

/// Every value the bank-select register `0x4E` is ever written with ("Safety
/// notes"): `0x80` / `0x00` for the vendor-ID bytes, then the bank numbers.
pub const SMIOVT_ALLOWED_BANK_SELECT_VALUES: [u8; 5] = [0x80, 0x00, 0x01, 0x02, 0x06];

/// Bank-select value for bank 0 with HBACS (bit 7) set: `0x4F` returns the
/// vendor ID high byte.
pub const SMIOVT_VENDOR_ID_HIGH_BYTE_SELECT: u8 = 0x80;
/// Bank-select value for bank 0 with HBACS clear: `0x4F` returns the low byte.
pub const SMIOVT_VENDOR_ID_LOW_BYTE_SELECT: u8 = 0x00;
/// Bank 0 index `0x4F`, the Nuvoton vendor ID byte selected by HBACS.
pub const SMIOVT_VENDOR_ID_REGISTER: u8 = 0x4F;
/// Nuvoton hardware-monitor vendor ID (S1 p. 99).
pub const NUVOTON_HM_VENDOR_ID: u16 = 0x5CA3;

/// Bank 0 index `0x40`, the hardware-monitor configuration register.
pub const SMIOVT_MONITORING_REGISTER: u8 = 0x40;
const MONITORING_START_BIT: u8 = 1 << 0;
const MONITORING_INT_CLEAR_BIT: u8 = 1 << 3;

/// Bank 0 index `0x4D`, NCT6776 FAN IN/OUT control (FANINC1 bit 0, FANINC2
/// bit 2).
pub const NCT6776_FAN_INPUT_CONTROL_REGISTER: u8 = 0x4D;
const FANINC1_BIT: u8 = 1 << 0;
const FANINC2_BIT: u8 = 1 << 2;

/// Global configuration registers read on NCT6776 only.
pub const NCT6776_CR1C_REGISTER: u8 = 0x1C;
pub const NCT6776_CR24_REGISTER: u8 = 0x24;
pub const NCT6776_CR26_REGISTER: u8 = 0x26;
pub const NCT6776_CR27_REGISTER: u8 = 0x27;
const CR26_LOCKREG_BIT: u8 = 1 << 5;
const CR1C_AUXFANIN1_PIN_BIT: u8 = 1 << 0;
const CR1C_AUXFANIN2_PIN_BIT: u8 = 1 << 1;
const CR24_FLOPPY_FUNCTION_BIT: u8 = 1 << 6;
const CR27_GPIO7_PINS_BIT: u8 = 1 << 7;
const CR30_AUXFANIN0_GPIO_BIT: u8 = 1 << 7;
const CR30_AUXFANIN1_GPIO_BIT: u8 = 1 << 6;
const CR30_AUXFANIN2_GPIO_BIT: u8 = 1 << 5;

/// LDN B `CR30` bit 0: Hardware Monitor & SB-TSI active.
const CR30_HARDWARE_MONITOR_ACTIVE_BIT: u8 = 1 << 0;

/// The aligned HM base window of the base gate.
const SMIOVT_MIN_HM_BASE: u16 = 0x0100;
const SMIOVT_MAX_HM_BASE: u16 = 0x0FF8;
const SMIOVT_HM_BASE_ALIGNMENT_MASK: u16 = 0x0007;

/// Source-select field (bits 4:0); bits 7:5 are reserved and ignored.
const SOURCE_SELECT_MASK: u8 = 0x1F;
/// Slot configuration register bit 0: `1` = slot not monitored (STOP).
const SLOT_STOP_BIT: u8 = 1 << 0;

/// A direct RPM value at or below this is reported as 0 RPM: `0x00A4` is the
/// power-on default and the RPM of the saturated 13-bit count.
pub const NCT6776_STOPPED_FAN_RPM_FLOOR: u32 = 0x00A4;

/// Read-to-clear status registers, as (bank, index). The profiles never read
/// them: NCT6776 bank 0 `0x41`, `0x42`, `0x45` and bank 4 `0x41`, `0x50`.
pub const SMIOVT_READ_CLEAR_REGISTERS: [(u8, u8); 5] = [
  (0x00, 0x41),
  (0x00, 0x42),
  (0x00, 0x45),
  (0x04, 0x41),
  (0x04, 0x50),
];

/// Which register layout a profile uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmiovtChipKind {
  /// NCT6776F/NCT6776D: primary-source layout with 9-bit bank 6 slots, a
  /// bank 0 STOP bit, `CR26` LOCKREG, INT_Clear and five direct RPM fans.
  Nct6776,
  /// NCT6775F/NCT6771F (lead-only): the same slot addresses with 8-bit bank 6
  /// slots, no bank 0 STOP bit, no LOCKREG or INT_Clear gate and no fans.
  Nct6775,
}

/// One scoped source-selected-slot profile, selected by raw chip ID.
#[derive(Debug, PartialEq, Eq)]
pub struct NuvotonSmiovtProfile {
  /// Chip label surfaced in diagnostics and failure text.
  pub chip_label: &'static str,
  pub kind: SmiovtChipKind,
}

/// The diode/thermistor sources a slot may select and the profiles publish.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmiovtSource {
  Systin,
  Cputin,
  Auxtin,
}

/// The published sources in output order.
pub const SMIOVT_SOURCES: [SmiovtSource; 3] = [
  SmiovtSource::Systin,
  SmiovtSource::Cputin,
  SmiovtSource::Auxtin,
];

impl SmiovtSource {
  /// Decode a source-select register. Only codes 1-3 are enabled; every other
  /// code (undefined, SMBus master, PECI, PCH, BYTE_TEMP) leaves the slot out.
  pub const fn from_select(raw: u8) -> Option<Self> {
    match raw & SOURCE_SELECT_MASK {
      0x01 => Some(Self::Systin),
      0x02 => Some(Self::Cputin),
      0x03 => Some(Self::Auxtin),
      _ => None,
    }
  }

  /// The label bound to the physical input, keyed by source code.
  pub const fn label(self) -> &'static str {
    match self {
      Self::Systin => "SYSTIN",
      Self::Cputin => "CPUTIN",
      Self::Auxtin => "AUXTIN",
    }
  }
}

/// A hardware-monitor register address: (bank, index).
pub type HmRegister = (u8, u8);

/// One SMIOVT slot of a profile.
#[derive(Debug, PartialEq, Eq)]
pub struct SmiovtSlot {
  /// Datasheet slot name, used only in diagnostics.
  pub name: &'static str,
  /// Bank 6 source-select register.
  pub source_select: HmRegister,
  /// Reading byte: temp<7:0> of an 8-bit slot or temp<8:1> of a 9-bit slot.
  pub reading: HmRegister,
  /// temp<0> of a 9-bit slot: register and bit mask. `None` = 8-bit slot.
  pub half_degree: Option<(HmRegister, u8)>,
  /// Slot configuration register whose bit 0 is STOP. `None` = no STOP gate.
  pub stop: Option<HmRegister>,
}

const fn slot(
  name: &'static str,
  source_select: u8,
  reading: HmRegister,
  half_degree: Option<(HmRegister, u8)>,
  stop: Option<HmRegister>,
) -> SmiovtSlot {
  SmiovtSlot {
    name,
    source_select: (SMIOVT_BANK_6, source_select),
    reading,
    half_degree,
    stop,
  }
}

/// "NCT6776 temperature slots" table (S1 pp. 62-63, 92-93, 116-117,
/// 128-129, 158-165).
const NCT6776_SLOTS: [SmiovtSlot; 6] = [
  slot(
    "SMIOVT1",
    0x21,
    (SMIOVT_BANK_0, 0x27),
    None,
    Some((SMIOVT_BANK_0, 0x18)),
  ),
  slot(
    "SMIOVT2",
    0x22,
    (SMIOVT_BANK_1, 0x50),
    Some(((SMIOVT_BANK_1, 0x51), 0x80)),
    Some((SMIOVT_BANK_1, 0x52)),
  ),
  slot(
    "SMIOVT3",
    0x23,
    (SMIOVT_BANK_2, 0x50),
    Some(((SMIOVT_BANK_2, 0x51), 0x80)),
    Some((SMIOVT_BANK_2, 0x52)),
  ),
  slot(
    "SMIOVT4",
    0x24,
    (SMIOVT_BANK_6, 0x2B),
    Some(((SMIOVT_BANK_6, 0x2E), 0x01)),
    Some((SMIOVT_BANK_6, 0x28)),
  ),
  slot(
    "SMIOVT5",
    0x25,
    (SMIOVT_BANK_6, 0x2C),
    Some(((SMIOVT_BANK_6, 0x2E), 0x02)),
    Some((SMIOVT_BANK_6, 0x29)),
  ),
  slot(
    "SMIOVT6",
    0x26,
    (SMIOVT_BANK_6, 0x2D),
    Some(((SMIOVT_BANK_6, 0x2E), 0x04)),
    Some((SMIOVT_BANK_6, 0x2A)),
  ),
];

/// "NCT6775F/NCT6771F temperature profile (lead-only)": the NCT6776 slot
/// addresses, bank 6 slots 8-bit (`0x2E` not used), no bank 0 STOP gate.
const NCT6775_SLOTS: [SmiovtSlot; 6] = [
  slot("SMIOVT1", 0x21, (SMIOVT_BANK_0, 0x27), None, None),
  slot(
    "SMIOVT2",
    0x22,
    (SMIOVT_BANK_1, 0x50),
    Some(((SMIOVT_BANK_1, 0x51), 0x80)),
    Some((SMIOVT_BANK_1, 0x52)),
  ),
  slot(
    "SMIOVT3",
    0x23,
    (SMIOVT_BANK_2, 0x50),
    Some(((SMIOVT_BANK_2, 0x51), 0x80)),
    Some((SMIOVT_BANK_2, 0x52)),
  ),
  slot(
    "SMIOVT4",
    0x24,
    (SMIOVT_BANK_6, 0x2B),
    None,
    Some((SMIOVT_BANK_6, 0x28)),
  ),
  slot(
    "SMIOVT5",
    0x25,
    (SMIOVT_BANK_6, 0x2C),
    None,
    Some((SMIOVT_BANK_6, 0x29)),
  ),
  slot(
    "SMIOVT6",
    0x26,
    (SMIOVT_BANK_6, 0x2D),
    None,
    Some((SMIOVT_BANK_6, 0x2A)),
  ),
];

/// Candidate order per source ("Temperature decode"), as indexes into the
/// slot table: NCT6776 SMIOVT2-6 then SMIOVT1 (9-bit slots first); NCT6775
/// SMIOVT2, SMIOVT3, then SMIOVT1, SMIOVT4-6.
const NCT6776_SLOT_PREFERENCE: [usize; 6] = [1, 2, 3, 4, 5, 0];
const NCT6775_SLOT_PREFERENCE: [usize; 6] = [1, 2, 0, 3, 4, 5];

/// Sampling step 2, after the bank 0 `0x40` gate and before the RPM pairs:
/// the registers each bank group reads, in order.
const NCT6776_SAMPLE_READS: [(u8, &[u8]); 4] = [
  (
    SMIOVT_BANK_0,
    &[0x27, 0x18, NCT6776_FAN_INPUT_CONTROL_REGISTER],
  ),
  (SMIOVT_BANK_1, &[0x50, 0x51, 0x52]),
  (SMIOVT_BANK_2, &[0x50, 0x51, 0x52]),
  (
    SMIOVT_BANK_6,
    &[
      0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E,
    ],
  ),
];
const NCT6775_SAMPLE_READS: [(u8, &[u8]); 4] = [
  (SMIOVT_BANK_0, &[0x27]),
  (SMIOVT_BANK_1, &[0x50, 0x51, 0x52]),
  (SMIOVT_BANK_2, &[0x50, 0x51, 0x52]),
  (
    SMIOVT_BANK_6,
    &[
      0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D,
    ],
  ),
];

/// How a fan input's pin is proven to be routed to its tachometer input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmiovtFanRoute {
  /// Bank 0 `0x4D` bit = 1 (read every sample).
  FanInputControl(u8),
  /// AUXFANIN0-2: LDN B `CR30` source-select bit chooses the dedicated pin or
  /// the GPIO pin, each with its own pin-function condition (cached at
  /// detection).
  AuxFanIn(u8),
}

/// One NCT6776 fan input ("NCT6776 fan inputs" table).
#[derive(Debug, PartialEq, Eq)]
pub struct SmiovtFanInput {
  pub label: &'static str,
  /// Bank 6 direct RPM high / low registers, read high first.
  pub rpm_high: u8,
  pub rpm_low: u8,
  pub route: SmiovtFanRoute,
}

const NCT6776_FAN_INPUTS: [SmiovtFanInput; 5] = [
  SmiovtFanInput {
    label: "Fan 1",
    rpm_high: 0x56,
    rpm_low: 0x57,
    route: SmiovtFanRoute::FanInputControl(FANINC1_BIT),
  },
  SmiovtFanInput {
    label: "Fan 2",
    rpm_high: 0x58,
    rpm_low: 0x59,
    route: SmiovtFanRoute::FanInputControl(FANINC2_BIT),
  },
  SmiovtFanInput {
    label: "Fan 3",
    rpm_high: 0x5A,
    rpm_low: 0x5B,
    route: SmiovtFanRoute::AuxFanIn(0),
  },
  SmiovtFanInput {
    label: "Fan 4",
    rpm_high: 0x5C,
    rpm_low: 0x5D,
    route: SmiovtFanRoute::AuxFanIn(1),
  },
  SmiovtFanInput {
    label: "Fan 5",
    rpm_high: 0x5E,
    rpm_low: 0x5F,
    route: SmiovtFanRoute::AuxFanIn(2),
  },
];

impl NuvotonSmiovtProfile {
  /// Every row of this document is Experimental.
  pub const fn is_experimental(&self) -> bool {
    true
  }

  /// The existing generic Nuvoton Super I/O source label ("Input identity and
  /// ordering").
  pub const fn source_label(&self) -> &'static str {
    NUVOTON_GENERIC_SOURCE_LABEL
  }

  /// Whether global `CR26` LOCKREG and the fan pin-function registers are
  /// read during detection (NCT6776 only).
  pub const fn reads_nct6776_configuration(&self) -> bool {
    matches!(self.kind, SmiovtChipKind::Nct6776)
  }

  pub fn slots(&self) -> &'static [SmiovtSlot; 6] {
    match self.kind {
      SmiovtChipKind::Nct6776 => &NCT6776_SLOTS,
      SmiovtChipKind::Nct6775 => &NCT6775_SLOTS,
    }
  }

  /// Slot indexes in the order a source's candidate is chosen.
  pub fn slot_preference(&self) -> &'static [usize; 6] {
    match self.kind {
      SmiovtChipKind::Nct6776 => &NCT6776_SLOT_PREFERENCE,
      SmiovtChipKind::Nct6775 => &NCT6775_SLOT_PREFERENCE,
    }
  }

  /// Bank groups of sampling step 2, in read order. The last group is bank 6,
  /// so the RPM pairs follow without another bank-select write.
  pub fn sample_reads(&self) -> &'static [(u8, &'static [u8]); 4] {
    match self.kind {
      SmiovtChipKind::Nct6776 => &NCT6776_SAMPLE_READS,
      SmiovtChipKind::Nct6775 => &NCT6775_SAMPLE_READS,
    }
  }

  /// Fan inputs read on this row (none on NCT6775: fans are Unsupported).
  pub fn fan_inputs(&self) -> &'static [SmiovtFanInput] {
    match self.kind {
      SmiovtChipKind::Nct6776 => &NCT6776_FAN_INPUTS,
      SmiovtChipKind::Nct6775 => &[],
    }
  }

  /// Bank 0 `0x40` monitoring gate: Start (bit 0) set and, on NCT6776,
  /// INT_Clear (bit 3) clear.
  pub const fn is_monitoring(&self, configuration: u8) -> bool {
    let started = configuration & MONITORING_START_BIT != 0;
    match self.kind {
      SmiovtChipKind::Nct6776 => started && configuration & MONITORING_INT_CLEAR_BIT == 0,
      SmiovtChipKind::Nct6775 => started,
    }
  }
}

const fn profile(chip_label: &'static str, kind: SmiovtChipKind) -> NuvotonSmiovtProfile {
  NuvotonSmiovtProfile { chip_label, kind }
}

const NCT6776_PROFILE: NuvotonSmiovtProfile =
  profile("NCT6776F / NCT6776D", SmiovtChipKind::Nct6776);
const NCT6776_UNRECOGNIZED_PROFILE: NuvotonSmiovtProfile = profile(
  "NCT6776 family (unrecognized version)",
  SmiovtChipKind::Nct6776,
);
const NCT6775_PROFILE: NuvotonSmiovtProfile =
  profile("NCT6775F / NCT6771F", SmiovtChipKind::Nct6775);
const NCT6775_UNRECOGNIZED_PROFILE: NuvotonSmiovtProfile = profile(
  "NCT6775F family (unrecognized version)",
  SmiovtChipKind::Nct6775,
);

/// Select the scoped profile for a raw chip ID ("ID-matching policy"), or
/// `None` when the ID is outside the enabled rows of this document.
pub const fn nuvoton_smiovt_profile(
  raw_chip_id: u16,
) -> Option<&'static NuvotonSmiovtProfile> {
  match raw_chip_id {
    0xC331..=0xC333 => Some(&NCT6776_PROFILE),
    0xC330 | 0xC334..=0xC337 => Some(&NCT6776_UNRECOGNIZED_PROFILE),
    0xB472 | 0xB473 => Some(&NCT6775_PROFILE),
    0xB470 | 0xB471 | 0xB474..=0xB477 => Some(&NCT6775_UNRECOGNIZED_PROFILE),
    _ => None,
  }
}

/// `CR26` bit 5 (LOCKREG): configuration register access is disabled.
pub const fn is_nct6776_lockreg_set(cr26: u8) -> bool {
  cr26 & CR26_LOCKREG_BIT != 0
}

/// LDN B `CR30` bit 0: the hardware monitor is active.
pub const fn is_smiovt_hardware_monitor_active(cr30: u8) -> bool {
  cr30 & CR30_HARDWARE_MONITOR_ACTIVE_BIT != 0
}

/// The HM base gate: inside `0x0100`-`0x0FF8` with the low three bits zero,
/// so base + 5 and base + 6 stay inside one 8-byte-aligned PawnIO window.
pub const fn is_usable_smiovt_hm_base(base: u16) -> bool {
  base >= SMIOVT_MIN_HM_BASE
    && base <= SMIOVT_MAX_HM_BASE
    && base & SMIOVT_HM_BASE_ALIGNMENT_MASK == 0
}

/// Combine the two vendor-ID bytes read with HBACS = 1 and HBACS = 0.
pub const fn smiovt_vendor_id(high: u8, low: u8) -> u16 {
  ((high as u16) << 8) | low as u16
}

/// Slot configuration bit 0 (STOP): `1` = the slot is not monitored.
pub const fn is_smiovt_slot_stopped(configuration: u8) -> bool {
  configuration & SLOT_STOP_BIT != 0
}

/// Decode an 8-bit two's complement slot reading in degrees C.
pub const fn decode_smiovt_8bit_temperature(raw: u8) -> f32 {
  (raw as i8) as f32
}

/// Decode a 9-bit two's complement slot reading in 0.5 degree C steps:
/// v = high * 2 + bit, v - 512 when v >= 256, temperature = v * 0.5.
pub const fn decode_smiovt_9bit_temperature(high: u8, half_degree: bool) -> f32 {
  let mut value = (high as i16) * 2 + half_degree as i16;
  if value >= 256 {
    value -= 512;
  }
  value as f32 * 0.5
}

/// Decode an NCT6776 direct RPM pair (high byte first). A value at or below
/// [`NCT6776_STOPPED_FAN_RPM_FLOOR`] is reported as 0 RPM.
pub const fn decode_nct6776_direct_rpm(high: u8, low: u8) -> u32 {
  let rpm = ((high as u32) << 8) | low as u32;
  if rpm <= NCT6776_STOPPED_FAN_RPM_FLOOR {
    0
  } else {
    rpm
  }
}

/// The NCT6776 configuration registers that route AUXFANIN0-2, read once
/// during detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nct6776FanPinConfiguration {
  pub cr1c: u8,
  pub cr24: u8,
  pub cr27: u8,
  /// LDN B `CR30`; bits 7-5 select dedicated pin or GPIO pin.
  pub cr30: u8,
}

impl Nct6776FanPinConfiguration {
  /// Whether AUXFANIN`aux` (0-2) reaches its tachometer input:
  /// - AUXFANIN0: `CR30` bit 7 = 0 and `CR24` bit 6 = 0, or bit 7 = 1 and
  ///   `CR27` bit 7 = 1;
  /// - AUXFANIN1: `CR30` bit 6 = 0 and `CR1C` bit 0 = 1, or bit 6 = 1 and
  ///   `CR27` bit 7 = 1;
  /// - AUXFANIN2: `CR30` bit 5 = 0 and `CR1C` bit 1 = 1, or bit 5 = 1 and
  ///   `CR27` bit 7 = 1.
  pub const fn routes_aux_fan_input(&self, aux: u8) -> bool {
    let (gpio_select_bit, dedicated_pin_routed) = match aux {
      0 => (
        CR30_AUXFANIN0_GPIO_BIT,
        self.cr24 & CR24_FLOPPY_FUNCTION_BIT == 0,
      ),
      1 => (
        CR30_AUXFANIN1_GPIO_BIT,
        self.cr1c & CR1C_AUXFANIN1_PIN_BIT != 0,
      ),
      2 => (
        CR30_AUXFANIN2_GPIO_BIT,
        self.cr1c & CR1C_AUXFANIN2_PIN_BIT != 0,
      ),
      _ => return false,
    };
    if self.cr30 & gpio_select_bit == 0 {
      dedicated_pin_routed
    } else {
      self.cr27 & CR27_GPIO7_PINS_BIT != 0
    }
  }
}

/// Whether one fan input passes its pin-routing gate in this sample. A
/// profile without pin configuration routes no fan.
pub const fn is_smiovt_fan_routed(
  route: SmiovtFanRoute,
  pins: Option<&Nct6776FanPinConfiguration>,
  fan_input_control: u8,
) -> bool {
  let Some(pins) = pins else {
    return false;
  };
  match route {
    SmiovtFanRoute::FanInputControl(bit) => fan_input_control & bit != 0,
    SmiovtFanRoute::AuxFanIn(aux) => pins.routes_aux_fan_input(aux),
  }
}

/// Whether a hardware-monitor read is permitted at all: only banks 0, 1, 2
/// and 6, and never a read-to-clear status register.
pub fn is_allowed_smiovt_hm_read(bank: u8, register: u8) -> bool {
  matches!(
    bank,
    SMIOVT_BANK_0 | SMIOVT_BANK_1 | SMIOVT_BANK_2 | SMIOVT_BANK_6
  ) && !SMIOVT_READ_CLEAR_REGISTERS.contains(&(bank, register))
}

/// The registers one sample read, keyed by (bank, index).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmiovtRegisterSnapshot {
  values: Vec<(HmRegister, u8)>,
}

impl SmiovtRegisterSnapshot {
  pub fn insert(&mut self, register: HmRegister, value: u8) {
    match self.values.iter_mut().find(|(key, _)| *key == register) {
      Some((_, existing)) => *existing = value,
      None => self.values.push((register, value)),
    }
  }

  pub fn get(&self, register: HmRegister) -> Result<u8, String> {
    self
      .values
      .iter()
      .find(|(key, _)| *key == register)
      .map(|(_, value)| *value)
      .ok_or_else(|| {
        format!(
          "bank {} register 0x{:02X} was not read",
          register.0, register.1
        )
      })
  }

  #[cfg(test)]
  pub fn from_dump(dump: &[(u8, u8, u8)]) -> Self {
    let mut snapshot = Self::default();
    for &(bank, register, value) in dump {
      snapshot.insert((bank, register), value);
    }
    snapshot
  }
}

/// The outcome for one published source in one sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SmiovtTemperatureReading {
  /// The preferred candidate slot decoded inside -55 to +125 degrees C.
  Available {
    slot: &'static str,
    temperature: f32,
  },
  /// The preferred candidate slot decoded outside the plausibility window;
  /// the source is unavailable and no other slot is substituted.
  Implausible {
    slot: &'static str,
    temperature: f32,
  },
  /// No slot both selects this source and is monitored.
  NoCandidate,
}

/// Decode one slot's reading as degrees C with the slot's width.
fn decode_slot(
  slot: &SmiovtSlot,
  snapshot: &SmiovtRegisterSnapshot,
) -> Result<f32, String> {
  let reading = snapshot.get(slot.reading)?;
  Ok(match slot.half_degree {
    Some((register, mask)) => {
      decode_smiovt_9bit_temperature(reading, snapshot.get(register)? & mask != 0)
    }
    None => decode_smiovt_8bit_temperature(reading),
  })
}

/// Whether a slot selects `source` and is monitored (STOP clear where the row
/// has a STOP bit).
fn is_candidate(
  slot: &SmiovtSlot,
  source: SmiovtSource,
  snapshot: &SmiovtRegisterSnapshot,
) -> Result<bool, String> {
  if SmiovtSource::from_select(snapshot.get(slot.source_select)?) != Some(source) {
    return Ok(false);
  }
  match slot.stop {
    Some(register) => Ok(!is_smiovt_slot_stopped(snapshot.get(register)?)),
    None => Ok(true),
  }
}

/// Choose and decode SYSTIN, CPUTIN and AUXTIN from one sample's registers
/// ("Temperature decode"). Each source is published once, from its first
/// candidate slot in the profile's preference order. An error means a
/// register the decode needs was not read.
pub fn select_smiovt_temperatures(
  profile: &NuvotonSmiovtProfile,
  snapshot: &SmiovtRegisterSnapshot,
) -> Result<[(SmiovtSource, SmiovtTemperatureReading); 3], String> {
  let slots = profile.slots();
  let mut readings =
    SMIOVT_SOURCES.map(|source| (source, SmiovtTemperatureReading::NoCandidate));
  for (source, reading) in &mut readings {
    for &index in profile.slot_preference() {
      let slot = &slots[index];
      if !is_candidate(slot, *source, snapshot)? {
        continue;
      }
      let temperature = decode_slot(slot, snapshot)?;
      *reading = if is_plausible_super_io_temperature(temperature) {
        SmiovtTemperatureReading::Available {
          slot: slot.name,
          temperature,
        }
      } else {
        SmiovtTemperatureReading::Implausible {
          slot: slot.name,
          temperature,
        }
      };
      break;
    }
  }
  Ok(readings)
}

#[cfg(test)]
mod tests {
  use super::*;

  const B0: u8 = SMIOVT_BANK_0;
  const B1: u8 = SMIOVT_BANK_1;
  const B2: u8 = SMIOVT_BANK_2;
  const B6: u8 = SMIOVT_BANK_6;

  fn nct6776() -> &'static NuvotonSmiovtProfile {
    nuvoton_smiovt_profile(0xC333).unwrap()
  }

  fn nct6775() -> &'static NuvotonSmiovtProfile {
    nuvoton_smiovt_profile(0xB473).unwrap()
  }

  /// Synthetic NCT6776 dump with the datasheet's power-on source selects
  /// (SMIOVT1 SYSTIN, SMIOVT2 CPUTIN, SMIOVT3 AUXTIN, SMIOVT4-6 SYSTIN),
  /// every slot monitored, and distinct readings per slot:
  /// SMIOVT1 0x1E (30), SMIOVT2 0x28 + half (40.5), SMIOVT3 0x23 (35),
  /// SMIOVT4 0x20 + half (32.5), SMIOVT5 0x21 (33), SMIOVT6 0x22 (34).
  const NCT6776_DEFAULT_DUMP: [(u8, u8, u8); 16] = [
    (B0, 0x27, 0x1E),
    (B0, 0x18, 0x00),
    (B0, 0x4D, 0x05),
    (B1, 0x50, 0x28),
    (B1, 0x51, 0x80),
    (B1, 0x52, 0x00),
    (B2, 0x50, 0x23),
    (B2, 0x51, 0x00),
    (B2, 0x52, 0x00),
    (B6, 0x21, 0x01),
    (B6, 0x22, 0x02),
    (B6, 0x23, 0x03),
    (B6, 0x24, 0x01),
    (B6, 0x25, 0x01),
    (B6, 0x26, 0x01),
    (B6, 0x2E, 0x01),
  ];
  const NCT6776_SLOT_4_6_DUMP: [(u8, u8, u8); 6] = [
    (B6, 0x28, 0x00),
    (B6, 0x29, 0x00),
    (B6, 0x2A, 0x00),
    (B6, 0x2B, 0x20),
    (B6, 0x2C, 0x21),
    (B6, 0x2D, 0x22),
  ];

  fn default_dump() -> SmiovtRegisterSnapshot {
    let mut snapshot = SmiovtRegisterSnapshot::from_dump(&NCT6776_DEFAULT_DUMP);
    for &(bank, register, value) in &NCT6776_SLOT_4_6_DUMP {
      snapshot.insert((bank, register), value);
    }
    snapshot
  }

  fn with(
    mut snapshot: SmiovtRegisterSnapshot,
    overrides: &[(u8, u8, u8)],
  ) -> SmiovtRegisterSnapshot {
    for &(bank, register, value) in overrides {
      snapshot.insert((bank, register), value);
    }
    snapshot
  }

  fn readings(
    profile: &NuvotonSmiovtProfile,
    snapshot: &SmiovtRegisterSnapshot,
  ) -> [SmiovtTemperatureReading; 3] {
    select_smiovt_temperatures(profile, snapshot)
      .unwrap()
      .map(|(_, reading)| reading)
  }

  fn available(slot: &'static str, temperature: f32) -> SmiovtTemperatureReading {
    SmiovtTemperatureReading::Available { slot, temperature }
  }

  // --- Spec "ID-matching policy" ---

  #[test]
  fn id_matching_policy_maps_every_enabled_id_to_its_row_label() {
    for raw_id in [0xC331, 0xC332, 0xC333] {
      let profile = nuvoton_smiovt_profile(raw_id).unwrap();
      assert_eq!(profile.chip_label, "NCT6776F / NCT6776D", "0x{raw_id:04X}");
      assert_eq!(profile.kind, SmiovtChipKind::Nct6776);
    }
    for raw_id in [0xC330, 0xC334, 0xC335, 0xC336, 0xC337] {
      let profile = nuvoton_smiovt_profile(raw_id).unwrap();
      assert_eq!(
        profile.chip_label, "NCT6776 family (unrecognized version)",
        "0x{raw_id:04X}"
      );
      assert_eq!(profile.kind, SmiovtChipKind::Nct6776);
    }
    for raw_id in [0xB472, 0xB473] {
      let profile = nuvoton_smiovt_profile(raw_id).unwrap();
      assert_eq!(profile.chip_label, "NCT6775F / NCT6771F", "0x{raw_id:04X}");
      assert_eq!(profile.kind, SmiovtChipKind::Nct6775);
    }
    for raw_id in [0xB470, 0xB471, 0xB474, 0xB475, 0xB476, 0xB477] {
      let profile = nuvoton_smiovt_profile(raw_id).unwrap();
      assert_eq!(
        profile.chip_label, "NCT6775F family (unrecognized version)",
        "0x{raw_id:04X}"
      );
      assert_eq!(profile.kind, SmiovtChipKind::Nct6775);
    }
  }

  #[test]
  fn id_matching_policy_keeps_unsupported_ids_unsupported() {
    let unsupported = (0xC338..=0xC33F)
      .chain(0xB478..=0xB47F)
      .chain(0xC450..=0xC45F)
      .chain([0x1061, 0xC32F, 0xB46F, 0x0000, 0xFFFF]);
    for raw_id in unsupported {
      assert_eq!(nuvoton_smiovt_profile(raw_id), None, "0x{raw_id:04X}");
    }
  }

  #[test]
  fn id_matching_policy_does_not_overlap_the_bank4_profiles() {
    // "Its raw-ID ranges do not overlap any ID in this document" (S6).
    for raw_id in (0xC330..=0xC337).chain(0xB470..=0xB477) {
      assert!(
        super::super::nuvoton::nuvoton_hm_profile(raw_id).is_none(),
        "0x{raw_id:04X}"
      );
    }
  }

  #[test]
  fn every_row_is_experimental_with_the_generic_nuvoton_source_label() {
    for raw_id in [0xC330, 0xC333, 0xB470, 0xB473] {
      let profile = nuvoton_smiovt_profile(raw_id).unwrap();
      assert!(profile.is_experimental());
      assert_eq!(profile.source_label(), "Nuvoton / Super I/O");
    }
  }

  // --- Spec "Runtime gates" ---

  #[test]
  fn runtime_gate_cr26_lockreg_reads_only_bit_five() {
    assert!(is_nct6776_lockreg_set(0x20));
    assert!(is_nct6776_lockreg_set(0xFF));
    assert!(!is_nct6776_lockreg_set(0x00));
    assert!(!is_nct6776_lockreg_set(0xDF));
    // Bit 6 (HEFRAS) reports the strap and is not a lock.
    assert!(!is_nct6776_lockreg_set(0x40));
  }

  #[test]
  fn runtime_gate_ldn_b_cr30_bit_zero_activates_the_hardware_monitor() {
    assert!(is_smiovt_hardware_monitor_active(0x01));
    assert!(is_smiovt_hardware_monitor_active(0xE1));
    assert!(!is_smiovt_hardware_monitor_active(0x00));
    assert!(!is_smiovt_hardware_monitor_active(0xE0));
  }

  #[test]
  fn runtime_gate_hm_base_requires_the_aligned_window() {
    for base in [0x0100, 0x0290, 0x0A00, 0x0FF8] {
      assert!(is_usable_smiovt_hm_base(base), "0x{base:04X}");
    }
    // Two-byte-aligned per S1 but not 8-byte-aligned for PawnIO windows.
    for base in [0x0292, 0x0294, 0x0296, 0x0FFE, 0x0101] {
      assert!(!is_usable_smiovt_hm_base(base), "0x{base:04X}");
    }
    for base in [0x0000, 0x00F8, 0x1000, 0x1290, 0xFFF8, 0xFFFF] {
      assert!(!is_usable_smiovt_hm_base(base), "0x{base:04X}");
    }
  }

  #[test]
  fn runtime_gate_vendor_id_combines_hbacs_high_and_low_bytes() {
    assert_eq!(smiovt_vendor_id(0x5C, 0xA3), NUVOTON_HM_VENDOR_ID);
    assert_ne!(smiovt_vendor_id(0xA3, 0x5C), NUVOTON_HM_VENDOR_ID);
    assert_ne!(smiovt_vendor_id(0xFF, 0xFF), NUVOTON_HM_VENDOR_ID);
  }

  #[test]
  fn runtime_gate_monitoring_requires_start_and_on_nct6776_no_int_clear() {
    assert!(nct6776().is_monitoring(0x01));
    assert!(nct6776().is_monitoring(0x03));
    assert!(!nct6776().is_monitoring(0x00));
    // Bit 3 (INT_Clear) stops monitoring on NCT6776.
    assert!(!nct6776().is_monitoring(0x09));

    assert!(nct6775().is_monitoring(0x01));
    assert!(!nct6775().is_monitoring(0x00));
    // The bit 3 gate is NCT6776 only.
    assert!(nct6775().is_monitoring(0x09));
  }

  #[test]
  fn runtime_gate_source_select_accepts_only_codes_one_to_three() {
    assert_eq!(SmiovtSource::from_select(0x01), Some(SmiovtSource::Systin));
    assert_eq!(SmiovtSource::from_select(0x02), Some(SmiovtSource::Cputin));
    assert_eq!(SmiovtSource::from_select(0x03), Some(SmiovtSource::Auxtin));
    // Bits 7:5 are reserved and ignored.
    assert_eq!(SmiovtSource::from_select(0xE2), Some(SmiovtSource::Cputin));
    // Undefined, SMBus master 0-7, PECI 0/1, PCH sources, BYTE_TEMP, undefined.
    for raw in [0x00].into_iter().chain(0x04..=0x1F) {
      assert_eq!(SmiovtSource::from_select(raw), None, "0x{raw:02X}");
    }
  }

  #[test]
  fn runtime_gate_slot_stop_bit_is_bit_zero() {
    assert!(is_smiovt_slot_stopped(0x01));
    assert!(is_smiovt_slot_stopped(0xFF));
    assert!(!is_smiovt_slot_stopped(0x00));
    assert!(!is_smiovt_slot_stopped(0xFE));
  }

  // --- Spec "NCT6776 fan inputs" pin-routing gates ---

  fn pins(cr1c: u8, cr24: u8, cr27: u8, cr30: u8) -> Nct6776FanPinConfiguration {
    Nct6776FanPinConfiguration {
      cr1c,
      cr24,
      cr27,
      cr30,
    }
  }

  #[test]
  fn pin_routing_fan_1_and_2_follow_faninc_bits_of_bank0_0x4d() {
    let pins = pins(0, 0, 0, 0x01);
    let fans = nct6776().fan_inputs();
    let routed =
      |fan: usize, control| is_smiovt_fan_routed(fans[fan].route, Some(&pins), control);

    assert!(routed(0, 0x01));
    assert!(!routed(0, 0x04));
    assert!(routed(1, 0x04));
    assert!(!routed(1, 0x01));
    assert!(routed(0, 0x05) && routed(1, 0x05));
    assert!(!routed(0, 0xFA) && !routed(1, 0xFA));
  }

  #[test]
  fn pin_routing_auxfanin0_uses_the_dedicated_pin_or_gp70() {
    // CR30 bit 7 = 0: dedicated pin 3 needs CR24 bit 6 = 0.
    assert!(pins(0, 0x00, 0, 0x01).routes_aux_fan_input(0));
    assert!(!pins(0, 0x40, 0, 0x01).routes_aux_fan_input(0));
    // CR27 is irrelevant on the dedicated pin.
    assert!(!pins(0, 0x40, 0x80, 0x01).routes_aux_fan_input(0));
    // CR30 bit 7 = 1: GPIO pin needs CR27 bit 7 = 1; CR24 is irrelevant.
    assert!(pins(0, 0x40, 0x80, 0x81).routes_aux_fan_input(0));
    assert!(!pins(0, 0x00, 0x00, 0x81).routes_aux_fan_input(0));
  }

  #[test]
  fn pin_routing_auxfanin1_uses_cr1c_bit0_or_gp72() {
    assert!(pins(0x01, 0x40, 0, 0x01).routes_aux_fan_input(1));
    assert!(!pins(0x00, 0x00, 0, 0x01).routes_aux_fan_input(1));
    assert!(!pins(0x02, 0x00, 0x80, 0x01).routes_aux_fan_input(1));
    assert!(pins(0x00, 0x00, 0x80, 0x41).routes_aux_fan_input(1));
    assert!(!pins(0x01, 0x00, 0x00, 0x41).routes_aux_fan_input(1));
  }

  #[test]
  fn pin_routing_auxfanin2_uses_cr1c_bit1_or_gp73() {
    assert!(pins(0x02, 0x40, 0, 0x01).routes_aux_fan_input(2));
    assert!(!pins(0x01, 0x00, 0, 0x01).routes_aux_fan_input(2));
    assert!(pins(0x00, 0x00, 0x80, 0x21).routes_aux_fan_input(2));
    assert!(!pins(0x02, 0x00, 0x00, 0x21).routes_aux_fan_input(2));
  }

  #[test]
  fn pin_routing_power_on_floppy_default_routes_no_aux_fan_input() {
    // CR24 bit 6 = 1 (floppy, default), CR1C bits 0-1 = 0, CR30 bits 7-5 = 0.
    let defaults = pins(0x00, 0x40, 0x00, 0x01);
    for aux in 0..3 {
      assert!(!defaults.routes_aux_fan_input(aux), "AUXFANIN{aux}");
    }
    assert!(!defaults.routes_aux_fan_input(3));
  }

  #[test]
  fn pin_routing_without_pin_configuration_routes_no_fan() {
    for fan in nct6776().fan_inputs() {
      assert!(
        !is_smiovt_fan_routed(fan.route, None, 0xFF),
        "{}",
        fan.label
      );
    }
  }

  #[test]
  fn fan_inputs_bind_labels_to_the_bank6_direct_rpm_pairs() {
    let pairs: Vec<(&str, u8, u8)> = nct6776()
      .fan_inputs()
      .iter()
      .map(|fan| (fan.label, fan.rpm_high, fan.rpm_low))
      .collect();
    assert_eq!(
      pairs,
      [
        ("Fan 1", 0x56, 0x57),
        ("Fan 2", 0x58, 0x59),
        ("Fan 3", 0x5A, 0x5B),
        ("Fan 4", 0x5C, 0x5D),
        ("Fan 5", 0x5E, 0x5F),
      ]
    );
    // NCT6775-family fans are Unsupported.
    assert!(nct6775().fan_inputs().is_empty());
  }

  // --- Spec "Fan decode (NCT6776)" ---

  #[test]
  fn fan_decode_combines_high_then_low_bytes() {
    // S1 p. 65 example: 0x09C4 = 2500 RPM.
    assert_eq!(decode_nct6776_direct_rpm(0x09, 0xC4), 2500);
    assert_eq!(decode_nct6776_direct_rpm(0x00, 0xA5), 165);
    assert_eq!(decode_nct6776_direct_rpm(0xFF, 0xFF), 0xFFFF);
  }

  #[test]
  fn fan_decode_reports_the_power_on_floor_and_below_as_zero() {
    // 0x00A4 is the power-on default (saturated count): stopped.
    assert_eq!(decode_nct6776_direct_rpm(0x00, 0xA4), 0);
    assert_eq!(decode_nct6776_direct_rpm(0x00, 0x00), 0);
    assert_eq!(decode_nct6776_direct_rpm(0x00, 0x01), 0);
    // A torn 0x00FF -> 0x0100 read (0x0000) also decodes as 0.
    assert_eq!(decode_nct6776_direct_rpm(0x00, 0x00), 0);
  }

  // --- Spec "Temperature decode" and Table 8-1 examples ---

  #[test]
  fn table_8_1_eight_bit_examples_decode_signed_celsius() {
    for (raw, expected) in [
      (0x7D, 125.0),
      (0x19, 25.0),
      (0x01, 1.0),
      (0x00, 0.0),
      (0xFF, -1.0),
      (0xE7, -25.0),
      (0xC9, -55.0),
    ] {
      assert_eq!(decode_smiovt_8bit_temperature(raw), expected, "0x{raw:02X}");
    }
  }

  #[test]
  fn table_8_1_nine_bit_examples_decode_half_degrees() {
    // (9-bit value, expected): high byte = value >> 1, bit = value & 1.
    for (value, expected) in [
      (0x0FA_u16, 125.0),
      (0x032, 25.0),
      (0x002, 1.0),
      (0x001, 0.5),
      (0x000, 0.0),
      (0x1FF, -0.5),
      (0x1FE, -1.0),
      (0x1CE, -25.0),
      (0x192, -55.0),
    ] {
      let high = (value >> 1) as u8;
      let bit = value & 1 != 0;
      assert_eq!(
        decode_smiovt_9bit_temperature(high, bit),
        expected,
        "0x{value:03X}"
      );
      // Equivalent form: signed high byte plus 0.5 for the low bit.
      assert_eq!(
        decode_smiovt_9bit_temperature(high, bit),
        (high as i8) as f32 + if bit { 0.5 } else { 0.0 }
      );
    }
  }

  #[test]
  fn temperature_decode_prefers_nine_bit_slots_and_publishes_each_source_once() {
    // Defaults: SYSTIN is selected by SMIOVT1, 4, 5 and 6; SMIOVT4 (9-bit)
    // wins over SMIOVT1 on NCT6776.
    assert_eq!(
      readings(nct6776(), &default_dump()),
      [
        available("SMIOVT4", 32.5),
        available("SMIOVT2", 40.5),
        available("SMIOVT3", 35.0),
      ]
    );
    let sources: Vec<&str> = select_smiovt_temperatures(nct6776(), &default_dump())
      .unwrap()
      .iter()
      .map(|(source, _)| source.label())
      .collect();
    assert_eq!(sources, ["SYSTIN", "CPUTIN", "AUXTIN"]);
  }

  #[test]
  fn temperature_decode_on_nct6775_prefers_smiovt1_over_the_bank6_slots() {
    // NCT6775 order: SMIOVT2, SMIOVT3, SMIOVT1, SMIOVT4-6; bank 6 is 8-bit
    // and the 0x2E half bit is ignored.
    assert_eq!(
      readings(nct6775(), &default_dump()),
      [
        available("SMIOVT1", 30.0),
        available("SMIOVT2", 40.5),
        available("SMIOVT3", 35.0),
      ]
    );
    let moved = with(default_dump(), &[(B6, 0x21, 0x0C)]);
    assert_eq!(readings(nct6775(), &moved)[0], available("SMIOVT4", 32.0));
  }

  #[test]
  fn temperature_decode_falls_back_to_smiovt1_only_when_no_nine_bit_slot_qualifies() {
    // SMIOVT4-6 point at PECI 0 / SMBus master / BYTE_TEMP.
    let snapshot = with(
      default_dump(),
      &[(B6, 0x24, 0x0C), (B6, 0x25, 0x04), (B6, 0x26, 0x16)],
    );
    assert_eq!(
      readings(nct6776(), &snapshot)[0],
      available("SMIOVT1", 30.0)
    );
  }

  #[test]
  fn temperature_decode_skips_stopped_slots() {
    // SMIOVT4 stopped -> SMIOVT5 (8-bit reading 0x21, half bit clear).
    let snapshot = with(default_dump(), &[(B6, 0x28, 0x01)]);
    assert_eq!(
      readings(nct6776(), &snapshot)[0],
      available("SMIOVT5", 33.0)
    );

    // CPUTIN only in SMIOVT2, which is stopped -> no candidate.
    let snapshot = with(default_dump(), &[(B1, 0x52, 0x01)]);
    assert_eq!(
      readings(nct6776(), &snapshot)[1],
      SmiovtTemperatureReading::NoCandidate
    );

    // NCT6776 has a bank 0 STOP bit for SMIOVT1.
    let only_smiovt1 = with(
      default_dump(),
      &[
        (B6, 0x24, 0x00),
        (B6, 0x25, 0x00),
        (B6, 0x26, 0x00),
        (B0, 0x18, 0x01),
      ],
    );
    assert_eq!(
      readings(nct6776(), &only_smiovt1)[0],
      SmiovtTemperatureReading::NoCandidate
    );
    // NCT6775 has no configuration register for SMIOVT1, so 0x18 is ignored.
    assert_eq!(
      readings(nct6775(), &only_smiovt1)[0],
      available("SMIOVT1", 30.0)
    );
  }

  #[test]
  fn temperature_decode_keys_labels_by_source_not_by_slot() {
    // Firmware swaps the sources of SMIOVT2 and SMIOVT3.
    let snapshot = with(default_dump(), &[(B6, 0x22, 0x03), (B6, 0x23, 0x02)]);
    assert_eq!(
      readings(nct6776(), &snapshot),
      [
        available("SMIOVT4", 32.5),
        available("SMIOVT3", 35.0),
        available("SMIOVT2", 40.5),
      ]
    );
  }

  #[test]
  fn temperature_decode_leaves_non_diode_sources_unavailable() {
    // Every slot selects PECI agent 0, SMBus master, PCH or an undefined code.
    let snapshot = with(
      default_dump(),
      &[
        (B6, 0x21, 0x0C),
        (B6, 0x22, 0x0D),
        (B6, 0x23, 0x04),
        (B6, 0x24, 0x0E),
        (B6, 0x25, 0x16),
        (B6, 0x26, 0x00),
      ],
    );
    assert_eq!(
      readings(nct6776(), &snapshot),
      [SmiovtTemperatureReading::NoCandidate; 3]
    );
  }

  #[test]
  fn plausibility_window_is_inclusive_on_both_widths() {
    // 9-bit +125 (0x7D, 0) and -55 (0xC9, 0) are published.
    let snapshot = with(default_dump(), &[(B1, 0x50, 0x7D), (B1, 0x51, 0x00)]);
    assert_eq!(
      readings(nct6776(), &snapshot)[1],
      available("SMIOVT2", 125.0)
    );
    let snapshot = with(default_dump(), &[(B1, 0x50, 0xC9), (B1, 0x51, 0x00)]);
    assert_eq!(
      readings(nct6776(), &snapshot)[1],
      available("SMIOVT2", -55.0)
    );
    // 8-bit bounds on SMIOVT1.
    let only_smiovt1 = |raw| {
      with(
        default_dump(),
        &[(B6, 0x24, 0), (B6, 0x25, 0), (B6, 0x26, 0), (B0, 0x27, raw)],
      )
    };
    assert_eq!(
      readings(nct6776(), &only_smiovt1(0x7D))[0],
      available("SMIOVT1", 125.0)
    );
    assert_eq!(
      readings(nct6776(), &only_smiovt1(0xC9))[0],
      available("SMIOVT1", -55.0)
    );
  }

  #[test]
  fn plausibility_rejects_just_outside_the_window_without_substitution() {
    // 9-bit +125.5 (0x7D, 1) and -55.5 (0xC8, 1).
    for (high, bit, decoded) in [(0x7D, 0x80, 125.5), (0xC8, 0x80, -55.5)] {
      let snapshot = with(default_dump(), &[(B1, 0x50, high), (B1, 0x51, bit)]);
      assert_eq!(
        readings(nct6776(), &snapshot)[1],
        SmiovtTemperatureReading::Implausible {
          slot: "SMIOVT2",
          temperature: decoded,
        }
      );
    }
    // SMIOVT4 (preferred SYSTIN) reads -128; SMIOVT5/6/1 are not substituted.
    let snapshot = with(default_dump(), &[(B6, 0x2B, 0x80), (B6, 0x2E, 0x00)]);
    assert_eq!(
      readings(nct6776(), &snapshot)[0],
      SmiovtTemperatureReading::Implausible {
        slot: "SMIOVT4",
        temperature: -128.0,
      }
    );
    // 8-bit +126 on NCT6775 SMIOVT1.
    let snapshot = with(default_dump(), &[(B0, 0x27, 0x7E)]);
    assert_eq!(
      readings(nct6775(), &snapshot)[0],
      SmiovtTemperatureReading::Implausible {
        slot: "SMIOVT1",
        temperature: 126.0,
      }
    );
  }

  #[test]
  fn temperature_decode_reports_a_register_missing_from_the_sample() {
    let snapshot = SmiovtRegisterSnapshot::from_dump(&[(B6, 0x21, 0x01)]);
    let error = select_smiovt_temperatures(nct6776(), &snapshot).unwrap_err();
    assert!(error.contains("was not read"), "{error}");
  }

  // --- Spec "Read procedure and decode" sampling plan, "Status registers
  // that clear on read" and "Safety notes" ---

  fn planned_reads(profile: &NuvotonSmiovtProfile) -> Vec<HmRegister> {
    profile
      .sample_reads()
      .iter()
      .flat_map(|(bank, registers)| {
        registers.iter().map(move |register| (*bank, *register))
      })
      .collect()
  }

  #[test]
  fn sampling_plan_reads_banks_0_1_2_6_in_order_ending_in_bank_6() {
    for profile in [nct6776(), nct6775()] {
      let banks: Vec<u8> = profile
        .sample_reads()
        .iter()
        .map(|(bank, _)| *bank)
        .collect();
      assert_eq!(banks, [B0, B1, B2, B6], "{}", profile.chip_label);
    }
    assert_eq!(nct6776().sample_reads()[0].1, &[0x27, 0x18, 0x4D]);
    assert_eq!(nct6775().sample_reads()[0].1, &[0x27]);
    assert!(nct6776().sample_reads()[3].1.contains(&0x2E));
    assert!(!nct6775().sample_reads()[3].1.contains(&0x2E));
  }

  #[test]
  fn sampling_plan_covers_every_register_the_decode_needs() {
    for profile in [nct6776(), nct6775()] {
      let mut snapshot = SmiovtRegisterSnapshot::default();
      for register in planned_reads(profile) {
        snapshot.insert(register, 0x01);
      }
      assert!(
        select_smiovt_temperatures(profile, &snapshot).is_ok(),
        "{}",
        profile.chip_label
      );
    }
  }

  #[test]
  fn sampling_plan_never_touches_read_clear_registers_or_other_banks() {
    for profile in [nct6776(), nct6775()] {
      for (bank, register) in planned_reads(profile) {
        assert!(
          is_allowed_smiovt_hm_read(bank, register),
          "{} bank {bank} 0x{register:02X}",
          profile.chip_label
        );
      }
      for fan in profile.fan_inputs() {
        assert!(is_allowed_smiovt_hm_read(B6, fan.rpm_high));
        assert!(is_allowed_smiovt_hm_read(B6, fan.rpm_low));
      }
    }
  }

  #[test]
  fn read_guard_refuses_read_clear_status_registers_and_banks_3_4_5_7() {
    for (bank, register) in SMIOVT_READ_CLEAR_REGISTERS {
      assert!(!is_allowed_smiovt_hm_read(bank, register));
    }
    for bank in [0x03, 0x04, 0x05, 0x07] {
      for register in [0x00, 0x27, 0x50, 0x90] {
        assert!(!is_allowed_smiovt_hm_read(bank, register), "bank {bank}");
      }
    }
    assert!(is_allowed_smiovt_hm_read(B0, SMIOVT_MONITORING_REGISTER));
    assert!(is_allowed_smiovt_hm_read(B0, SMIOVT_VENDOR_ID_REGISTER));
  }

  #[test]
  fn bank_select_allow_list_matches_the_safety_notes() {
    assert_eq!(
      SMIOVT_ALLOWED_BANK_SELECT_VALUES,
      [
        SMIOVT_VENDOR_ID_HIGH_BYTE_SELECT,
        SMIOVT_VENDOR_ID_LOW_BYTE_SELECT,
        B1,
        B2,
        B6
      ]
    );
    assert!(SMIOVT_ALLOWED_BANK_SELECT_VALUES.contains(&B0));
  }
}
