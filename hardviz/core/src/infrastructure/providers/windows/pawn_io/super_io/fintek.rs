//! Pure helpers and the scoped chip-profile table for the Fintek F71882 and
//! F71889 hardware monitors (logical device 4).
//!
//! Enabled rows, both Experimental: exact raw chip ID `0x0541` (F71882,
//! TEMP1-3 and FAN1-4) and exact raw chip ID `0x0723` (F71889, TEMP1-3 and
//! FAN1-3). Every other Fintek ID, voltages, digital temperatures, limits,
//! alarms and fan control stay unsupported.
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-fintek-f71882-f71889.md` revision 1
//!
//! No other external sensor source was used.

/// Global `CR23`/`CR24`: "Vendor ID of Fintek devices" (S1 p. 55; S2 p. 65).
pub const FINTEK_VENDOR_ID_HIGH_REGISTER: u8 = 0x23;
pub const FINTEK_VENDOR_ID_LOW_REGISTER: u8 = 0x24;
pub const FINTEK_VENDOR_ID: (u8, u8) = (0x19, 0x34);

/// Global `CR25`; bit 4 (SOFTPD_HM) set stops the HM clock. Never written.
pub const FINTEK_SOFTWARE_POWER_DOWN_REGISTER: u8 = 0x25;
const CR25_SOFTPD_HM_BIT: u8 = 1 << 4;

/// Global routing registers. `CR27` bit 6 is the F71882 SPI_EN strap, `CR28`
/// bit 0 the F71882 GPIO40_SEL, `CR2B` the GPIO10_SEL field of either part.
/// None of them is ever written.
pub const FINTEK_CR27_REGISTER: u8 = 0x27;
pub const FINTEK_CR28_REGISTER: u8 = 0x28;
pub const FINTEK_CR2B_REGISTER: u8 = 0x2B;
const CR27_F71882_SPI_EN_BIT: u8 = 1 << 6;
const CR28_F71882_GPIO40_SEL_BIT: u8 = 1 << 0;
const CR2B_F71882_GPIO10_SEL_BIT: u8 = 1 << 0;
const CR2B_F71889_GPIO10_SEL_MASK: u8 = 0x03;

/// Logical device 4 is the hardware monitor on both parts; `CR07` = `0x04` is
/// the only configuration register write besides the register indexes.
pub const FINTEK_HARDWARE_MONITOR_LDN: u8 = 0x04;
/// LDN 4 `CR30` bit 0 (HM_EN). Never written.
const CR30_HM_EN_BIT: u8 = 1 << 0;

/// HM base window of the base gate: `B & 0xFFF8` in `0x0100`-`0x0FF8`.
const FINTEK_MIN_HM_BASE: u16 = 0x0100;
const FINTEK_MAX_HM_BASE: u16 = 0x0FF8;
const FINTEK_HM_BASE_ALIGNMENT_MASK: u16 = 0x0007;
/// The two base forms `LpcIO` authorizes: low bits `000b` or `101b`.
const FINTEK_ALIGNED_BASE_LOW_BITS: u16 = 0b000;
const FINTEK_ROUNDED_BASE_LOW_BITS: u16 = 0b101;
/// The 8-byte window of the PCI configuration mechanism #1 ports
/// `0x0CF8`-`0x0CFF`, which `LpcIO` never authorizes and never records
/// (`pawnio-interface.md` revision 9, `LpcIO`).
const LPCIO_PROTECTED_WINDOW: u16 = 0x0CF8;
/// HM index/data offsets from the 8-byte-aligned base (lead-only port
/// derivation, S14/S16; S3 corroborates at the default base).
const FINTEK_HM_INDEX_OFFSET: u16 = 0x05;
const FINTEK_HM_DATA_OFFSET: u16 = 0x06;

/// HM `0x01`: POWER_DOWN bit 2, FAN_START bit 1, V_T_START bit 0. Never written.
pub const FINTEK_HM_CONFIGURATION_REGISTER: u8 = 0x01;
const HM01_POWER_DOWN_BIT: u8 = 1 << 2;
const HM01_FAN_START_BIT: u8 = 1 << 1;
const HM01_V_T_START_BIT: u8 = 1 << 0;

/// HM `0x0A`: F71882 MEAS_TYPE bits 1:0; F71889 DIG_T1_EN bit 1 and
/// DIODE_T1_EN bit 0.
pub const FINTEK_HM_TEMP1_TYPE_REGISTER: u8 = 0x0A;
const F71882_MEAS_TYPE_MASK: u8 = 0x03;
const F71889_DIODE_T1_EN_BIT: u8 = 1 << 0;
const F71889_DIG_T1_EN_BIT: u8 = 1 << 1;
/// F71889 HM `0x7F` bits 1:0 (T1_SRC_SEL_REG); `00` = D1+ only.
pub const F71889_HM_T1_SOURCE_SELECT_REGISTER: u8 = 0x7F;
const F71889_T1_SRC_SEL_MASK: u8 = 0x03;

/// F71889 HM vendor ID at `0x5D`/`0x5E` (S2 p. 51).
pub const F71889_HM_VENDOR_ID_HIGH_REGISTER: u8 = 0x5D;
pub const F71889_HM_VENDOR_ID_LOW_REGISTER: u8 = 0x5E;

/// HM `0x6F` (DIODE OPEN Status): bit n reports TEMPn open.
pub const FINTEK_HM_DIODE_OPEN_REGISTER: u8 = 0x6F;

/// HM registers documented to clear (write 1 to clear, or cleared by the next
/// SMBus transfer) or that the profile has no use for. Never read, never
/// written: `0x03`, `0x11`, `0x61`, `0x91`, the real-time status registers
/// `0x62`/`0x92`, and the F71889 digital-temperature and SMBus range
/// `0xE0`-`0xEF` ("Registers that clear"; "Recorded transaction").
pub const FINTEK_NEVER_READ_HM_REGISTERS: [u8; 22] = [
  0x03, 0x11, 0x61, 0x91, 0x62, 0x92, 0xE0, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7,
  0xE8, 0xE9, 0xEA, 0xEB, 0xEC, 0xED, 0xEE, 0xEF,
];

/// A count of `0xFFF` is the power-on default and the fan-fault value: no
/// measurable rotation.
const FINTEK_STOPPED_FAN_COUNT: u16 = 0x0FFF;
/// Counts below this would exceed 23 437 RPM (project plausibility guard).
const FINTEK_MIN_VALID_FAN_COUNT: u16 = 0x0040;
/// RPM = 1.5 x 10^6 / count (S1 p. 39; S2 p. 19).
const FINTEK_FAN_RPM_NUMERATOR: u32 = 1_500_000;

/// F71882 temperature bytes `0x00`-`0x7D` are 0 to +125 degrees C.
const F71882_MAX_TEMPERATURE_BYTE: u8 = 0x7D;
/// F71889 signed bytes from -40 to +125 degrees C.
const F71889_MIN_TEMPERATURE: i8 = -40;
const F71889_MAX_TEMPERATURE: i8 = 125;

/// The register layout and gate set a profile uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FintekChip {
  F71882,
  F71889,
}

/// How a fan input's pin is routed to its tachometer ("Fan pin routing").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FintekFanRoute {
  /// FANIN1 (pin 21) and FANIN2 (pin 23) are dedicated on both parts.
  Dedicated,
  /// F71882 FAN3: `CR28` bit 0 = `0`.
  F71882Fan3,
  /// F71882 FAN4: `CR27` bit 6 = `0` and `CR2B` bit 0 = `1`.
  F71882Fan4,
  /// F71889 FAN3: `CR2B` bits 1:0 = `00`.
  F71889Fan3,
}

/// One temperature input, bound to its physical diode input.
#[derive(Debug, PartialEq, Eq)]
pub struct FintekTemperatureInput {
  pub label: &'static str,
  pub register: u8,
  /// Its Tn_DIODE_OPEN bit in HM `0x6F`.
  pub diode_open_bit: u8,
  /// Whether the HM `0x0A` TEMP1 source gate applies (TEMP1 only).
  pub source_gated: bool,
}

/// One fan input, bound to FANINn.
#[derive(Debug, PartialEq, Eq)]
pub struct FintekFanInput {
  pub label: &'static str,
  /// Read first; reading it latches the LSB.
  pub msb_register: u8,
  pub lsb_register: u8,
  pub route: FintekFanRoute,
}

/// TEMP1 (D1+), TEMP2 (D2+), TEMP3 (D3+) on both parts (S1 pp. 10, 77; S2
/// pp. 12, 32-33).
pub const FINTEK_TEMPERATURE_INPUTS: [FintekTemperatureInput; 3] = [
  FintekTemperatureInput {
    label: "TEMP1",
    register: 0x72,
    diode_open_bit: 1 << 1,
    source_gated: true,
  },
  FintekTemperatureInput {
    label: "TEMP2",
    register: 0x74,
    diode_open_bit: 1 << 2,
    source_gated: false,
  },
  FintekTemperatureInput {
    label: "TEMP3",
    register: 0x76,
    diode_open_bit: 1 << 3,
    source_gated: false,
  },
];

/// The most fan inputs any profile reads (FAN1-FAN4 on F71882).
pub const FINTEK_MAX_FAN_INPUTS: usize = 4;

const fn fan(
  label: &'static str,
  msb_register: u8,
  route: FintekFanRoute,
) -> FintekFanInput {
  FintekFanInput {
    label,
    msb_register,
    lsb_register: msb_register + 1,
    route,
  }
}

const F71882_FANS: [FintekFanInput; 4] = [
  fan("FAN1", 0xA0, FintekFanRoute::Dedicated),
  fan("FAN2", 0xB0, FintekFanRoute::Dedicated),
  fan("FAN3", 0xC0, FintekFanRoute::F71882Fan3),
  fan("FAN4", 0xD0, FintekFanRoute::F71882Fan4),
];

const F71889_FANS: [FintekFanInput; 3] = [
  fan("FAN1", 0xA0, FintekFanRoute::Dedicated),
  fan("FAN2", 0xB0, FintekFanRoute::Dedicated),
  fan("FAN3", 0xC0, FintekFanRoute::F71889Fan3),
];

/// One scoped Fintek profile, selected by exact raw chip ID.
#[derive(Debug, PartialEq, Eq)]
pub struct FintekProfile {
  pub chip: FintekChip,
  pub raw_chip_id: u16,
  /// Chip label surfaced in diagnostics and failure text.
  pub chip_label: &'static str,
  /// Source label of every published reading.
  pub source_label: &'static str,
  pub fans: &'static [FintekFanInput],
}

/// The ID-matching table: exact raw IDs only.
const FINTEK_PROFILES: [FintekProfile; 2] = [
  FintekProfile {
    chip: FintekChip::F71882,
    raw_chip_id: 0x0541,
    chip_label: "Fintek F71882",
    source_label: "Fintek F71882 / Super I/O",
    fans: &F71882_FANS,
  },
  FintekProfile {
    chip: FintekChip::F71889,
    raw_chip_id: 0x0723,
    chip_label: "Fintek F71889",
    source_label: "Fintek F71889 / Super I/O",
    fans: &F71889_FANS,
  },
];

/// The profile for an exact raw chip ID (`CR20`:`CR21`), if the document
/// enables it. Every other ID, including other Fintek parts, is outside it.
pub fn fintek_profile(raw_chip_id: u16) -> Option<&'static FintekProfile> {
  FINTEK_PROFILES
    .iter()
    .find(|profile| profile.raw_chip_id == raw_chip_id)
}

impl FintekProfile {
  /// Every profile in this document is Experimental.
  pub const fn is_experimental(&self) -> bool {
    true
  }

  /// The global routing registers detection reads, in order ("Detection"
  /// step 3): `CR27`, `CR28`, `CR2B` on F71882 and `CR2B` on F71889.
  pub const fn routing_registers(&self) -> &'static [u8] {
    match self.chip {
      FintekChip::F71882 => &[
        FINTEK_CR27_REGISTER,
        FINTEK_CR28_REGISTER,
        FINTEK_CR2B_REGISTER,
      ],
      FintekChip::F71889 => &[FINTEK_CR2B_REGISTER],
    }
  }

  /// Whether detection checks the HM vendor ID through the HM ports.
  pub const fn checks_hm_vendor_id(&self) -> bool {
    matches!(self.chip, FintekChip::F71889)
  }

  /// Whether `register` is one of the HM registers this profile reads. Every
  /// HM read is checked against this before any port I/O; the never-read
  /// list is checked first so no table entry can reach it.
  pub fn is_allowed_hm_read(&self, register: u8) -> bool {
    if FINTEK_NEVER_READ_HM_REGISTERS.contains(&register) {
      return false;
    }
    let shared = matches!(
      register,
      FINTEK_HM_CONFIGURATION_REGISTER
        | FINTEK_HM_TEMP1_TYPE_REGISTER
        | FINTEK_HM_DIODE_OPEN_REGISTER
    ) || FINTEK_TEMPERATURE_INPUTS
      .iter()
      .any(|input| input.register == register)
      || self
        .fans
        .iter()
        .any(|fan| fan.msb_register == register || fan.lsb_register == register);
    shared
      || (self.chip == FintekChip::F71889
        && matches!(
          register,
          F71889_HM_VENDOR_ID_HIGH_REGISTER
            | F71889_HM_VENDOR_ID_LOW_REGISTER
            | F71889_HM_T1_SOURCE_SELECT_REGISTER
        ))
  }
}

/// Whether `CR23`/`CR24` read the Fintek vendor ID `0x19`/`0x34`.
pub const fn is_fintek_vendor_id(high: u8, low: u8) -> bool {
  high == FINTEK_VENDOR_ID.0 && low == FINTEK_VENDOR_ID.1
}

/// Whether `CR25` bit 4 (SOFTPD_HM) has stopped the HM clock.
pub const fn is_fintek_hardware_monitor_powered_down(cr25: u8) -> bool {
  cr25 & CR25_SOFTPD_HM_BIT != 0
}

/// Whether LDN 4 `CR30` bit 0 (HM_EN) enables the hardware monitor.
pub const fn is_fintek_hardware_monitor_enabled(cr30: u8) -> bool {
  cr30 & CR30_HM_EN_BIT != 0
}

/// The HM index/data ports derived from an HM base B = `CR60`:`CR61`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FintekHmPorts {
  pub index: u16,
  pub data: u16,
}

/// Apply the base gate and derive the HM ports: `B & 0xFFF8` must lie in
/// `0x0100`-`0x0FF8` and `B & 0x0007` must be `0` or `5`; the ports are
/// `(B & 0xFFF8) + 5` / `+ 6`. A window that overlaps the PCI configuration
/// ports, which `LpcIO` never authorizes, is refused as well. `None` means the
/// chip is unavailable; no workaround is attempted.
pub const fn fintek_hm_ports(base: u16) -> Option<FintekHmPorts> {
  let window = base & !FINTEK_HM_BASE_ALIGNMENT_MASK;
  let low_bits = base & FINTEK_HM_BASE_ALIGNMENT_MASK;
  let usable_window = window >= FINTEK_MIN_HM_BASE
    && window <= FINTEK_MAX_HM_BASE
    && window != LPCIO_PROTECTED_WINDOW;
  let authorized_form =
    low_bits == FINTEK_ALIGNED_BASE_LOW_BITS || low_bits == FINTEK_ROUNDED_BASE_LOW_BITS;
  if usable_window && authorized_form {
    Some(FintekHmPorts {
      index: window + FINTEK_HM_INDEX_OFFSET,
      data: window + FINTEK_HM_DATA_OFFSET,
    })
  } else {
    None
  }
}

/// Which input groups HM `0x01` lets a sample read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FintekMonitorState {
  /// POWER_DOWN (bit 2) is set, which includes a `0xFF` no-responder read:
  /// every input is unavailable for this sample.
  PoweredDown,
  Running {
    /// V_T_START (bit 0).
    temperatures: bool,
    /// FAN_START (bit 1).
    fans: bool,
  },
}

/// Decode the HM `0x01` start and power-down bits ("Sampling" step 1).
pub const fn fintek_monitor_state(hm01: u8) -> FintekMonitorState {
  if hm01 & HM01_POWER_DOWN_BIT != 0 {
    FintekMonitorState::PoweredDown
  } else {
    FintekMonitorState::Running {
      temperatures: hm01 & HM01_V_T_START_BIT != 0,
      fans: hm01 & HM01_FAN_START_BIT != 0,
    }
  }
}

/// What HM `0x0A` says about the TEMP1 source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FintekTemp1Source {
  /// D1+ external diode: TEMP1 may be read.
  ExternalDiode,
  /// F71889 with DIODE_T1_EN and DIG_T1_EN both set: HM `0x7F` decides.
  SourceSelected,
  /// PECI, AMDSI/TSI, reserved, or no diode: TEMP1 is unavailable.
  Other,
}

/// Decode HM `0x0A` for the TEMP1 source gate of `chip`.
pub const fn fintek_temp1_source(chip: FintekChip, hm0a: u8) -> FintekTemp1Source {
  match chip {
    FintekChip::F71882 => {
      if hm0a & F71882_MEAS_TYPE_MASK == 0 {
        FintekTemp1Source::ExternalDiode
      } else {
        FintekTemp1Source::Other
      }
    }
    FintekChip::F71889 => {
      if hm0a & F71889_DIODE_T1_EN_BIT == 0 {
        FintekTemp1Source::Other
      } else if hm0a & F71889_DIG_T1_EN_BIT == 0 {
        FintekTemp1Source::ExternalDiode
      } else {
        FintekTemp1Source::SourceSelected
      }
    }
  }
}

/// Whether F71889 HM `0x7F` bits 1:0 (T1_SRC_SEL_REG) select D1+ only.
pub const fn is_f71889_temp1_source_diode(hm7f: u8) -> bool {
  hm7f & F71889_T1_SRC_SEL_MASK == 0
}

/// Whether the HM `0x6F` diode-open bit of `input` is set.
pub const fn is_fintek_diode_open(hm6f: u8, input: &FintekTemperatureInput) -> bool {
  hm6f & input.diode_open_bit != 0
}

/// Decode one temperature byte (1 degree C per LSB) within the row's
/// acceptance bounds. `None` is unavailable for this sample: never clamped,
/// never zero.
///
/// F71882 accepts only `0x00`-`0x7D` because S1 does not state the encoding
/// above `0x7F`; F71889 decodes the byte as signed and accepts -40 to +125.
pub const fn decode_fintek_temperature(chip: FintekChip, raw: u8) -> Option<f32> {
  match chip {
    FintekChip::F71882 => {
      if raw <= F71882_MAX_TEMPERATURE_BYTE {
        Some(raw as f32)
      } else {
        None
      }
    }
    FintekChip::F71889 => {
      let value = raw as i8;
      if value >= F71889_MIN_TEMPERATURE && value <= F71889_MAX_TEMPERATURE {
        Some(value as f32)
      } else {
        None
      }
    }
  }
}

/// Combine the MSB-latched count bytes: count = MSB x 256 + LSB.
pub const fn fintek_fan_count(msb: u8, lsb: u8) -> u16 {
  u16::from_be_bytes([msb, lsb])
}

/// Decode a fan count ("Fan decode"). `None` is an invalid reading with no
/// RPM; `Some(0)` is the `0xFFF` no-rotation count.
pub const fn decode_fintek_fan_rpm(count: u16) -> Option<u32> {
  if count == FINTEK_STOPPED_FAN_COUNT {
    Some(0)
  } else if count >= FINTEK_MIN_VALID_FAN_COUNT && count < FINTEK_STOPPED_FAN_COUNT {
    Some(FINTEK_FAN_RPM_NUMERATOR / count as u32)
  } else {
    None
  }
}

/// The routing registers read during detection; `None` is a failed read,
/// which leaves the fans that depend on it unavailable until rediscovery.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FintekRoutingRegisters {
  pub cr27: Option<u8>,
  pub cr28: Option<u8>,
  pub cr2b: Option<u8>,
}

impl FintekRoutingRegisters {
  /// Record one routing register read.
  pub fn record(&mut self, register: u8, value: Option<u8>) {
    match register {
      FINTEK_CR27_REGISTER => self.cr27 = value,
      FINTEK_CR28_REGISTER => self.cr28 = value,
      FINTEK_CR2B_REGISTER => self.cr2b = value,
      _ => {}
    }
  }
}

/// Whether a fan input's pin is routed to its tachometer.
pub fn is_fintek_fan_routed(
  route: FintekFanRoute,
  routing: &FintekRoutingRegisters,
) -> bool {
  match route {
    FintekFanRoute::Dedicated => true,
    FintekFanRoute::F71882Fan3 => routing
      .cr28
      .is_some_and(|cr28| cr28 & CR28_F71882_GPIO40_SEL_BIT == 0),
    FintekFanRoute::F71882Fan4 => {
      routing
        .cr27
        .is_some_and(|cr27| cr27 & CR27_F71882_SPI_EN_BIT == 0)
        && routing
          .cr2b
          .is_some_and(|cr2b| cr2b & CR2B_F71882_GPIO10_SEL_BIT != 0)
    }
    FintekFanRoute::F71889Fan3 => routing
      .cr2b
      .is_some_and(|cr2b| cr2b & CR2B_F71889_GPIO10_SEL_MASK == 0),
  }
}

/// The routed state of each profile fan, by position. Positions past the
/// profile's last fan stay `false`.
pub fn fintek_routed_fans(
  profile: &FintekProfile,
  routing: &FintekRoutingRegisters,
) -> [bool; FINTEK_MAX_FAN_INPUTS] {
  let mut routed = [false; FINTEK_MAX_FAN_INPUTS];
  for (slot, fan) in routed.iter_mut().zip(profile.fans) {
    *slot = is_fintek_fan_routed(fan.route, routing);
  }
  routed
}

#[cfg(test)]
mod tests {
  use super::*;

  fn f71882() -> &'static FintekProfile {
    fintek_profile(0x0541).unwrap()
  }

  fn f71889() -> &'static FintekProfile {
    fintek_profile(0x0723).unwrap()
  }

  // --- "ID-matching policy" ---

  #[test]
  fn id_matching_is_exact_for_the_two_enabled_rows() {
    assert_eq!(f71882().chip, FintekChip::F71882);
    assert_eq!(f71882().chip_label, "Fintek F71882");
    assert_eq!(f71889().chip, FintekChip::F71889);
    assert_eq!(f71889().chip_label, "Fintek F71889");
    // "Synthetic cases": every deferred or other ID stays outside.
    for raw_id in [
      0x0814, 0x1005, 0x0909, 0x1106, 0x1007, 0x0601, 0x0901, 0x0507, 0x1010, 0x1118,
      0xFFFF, 0x0000, 0x0540, 0x0542, 0x0722, 0x0724,
    ] {
      assert!(fintek_profile(raw_id).is_none(), "0x{raw_id:04X}");
    }
  }

  #[test]
  fn profiles_bind_labels_to_physical_inputs() {
    let labels = |profile: &FintekProfile| {
      profile
        .fans
        .iter()
        .map(|fan| (fan.label, fan.msb_register, fan.lsb_register))
        .collect::<Vec<_>>()
    };
    assert_eq!(
      labels(f71882()),
      [
        ("FAN1", 0xA0, 0xA1),
        ("FAN2", 0xB0, 0xB1),
        ("FAN3", 0xC0, 0xC1),
        ("FAN4", 0xD0, 0xD1),
      ]
    );
    assert_eq!(
      labels(f71889()),
      [
        ("FAN1", 0xA0, 0xA1),
        ("FAN2", 0xB0, 0xB1),
        ("FAN3", 0xC0, 0xC1)
      ]
    );
    assert_eq!(
      FINTEK_TEMPERATURE_INPUTS.map(|input| (input.label, input.register)),
      [("TEMP1", 0x72), ("TEMP2", 0x74), ("TEMP3", 0x76)]
    );
    assert_eq!(f71882().source_label, "Fintek F71882 / Super I/O");
    assert_eq!(f71889().source_label, "Fintek F71889 / Super I/O");
  }

  // --- "Runtime gates" ---

  #[test]
  fn vendor_id_gate_requires_0x19_0x34() {
    assert!(is_fintek_vendor_id(0x19, 0x34));
    assert!(!is_fintek_vendor_id(0xFF, 0xFF));
    assert!(!is_fintek_vendor_id(0x34, 0x19));
    assert!(!is_fintek_vendor_id(0x19, 0x00));
  }

  #[test]
  fn cr25_bit_4_powers_the_hardware_monitor_down() {
    assert!(!is_fintek_hardware_monitor_powered_down(0x00));
    assert!(!is_fintek_hardware_monitor_powered_down(0xEF));
    assert!(is_fintek_hardware_monitor_powered_down(0x10));
  }

  #[test]
  fn cr30_bit_0_enables_the_hardware_monitor() {
    assert!(is_fintek_hardware_monitor_enabled(0x01));
    assert!(is_fintek_hardware_monitor_enabled(0xFF));
    assert!(!is_fintek_hardware_monitor_enabled(0x00));
    assert!(!is_fintek_hardware_monitor_enabled(0xFE));
  }

  #[test]
  fn base_gate_derives_ports_from_the_two_authorized_forms() {
    // "Synthetic cases": base 0x0295 (fixture 1), 0x0290, 0x0A40, 0x0A45.
    for (base, ports) in [
      (0x0295, (0x0295, 0x0296)),
      (0x0290, (0x0295, 0x0296)),
      (0x0A40, (0x0A45, 0x0A46)),
      (0x0A45, (0x0A45, 0x0A46)),
      (0x0100, (0x0105, 0x0106)),
      (0x0FFD, (0x0FFD, 0x0FFE)),
    ] {
      assert_eq!(
        fintek_hm_ports(base),
        Some(FintekHmPorts {
          index: ports.0,
          data: ports.1
        }),
        "0x{base:04X}"
      );
    }
  }

  #[test]
  fn base_gate_refuses_other_bases() {
    // "Synthetic cases": 0x0000, 0x0293, 0x00F8, 0x1000; plus the other low
    // bit patterns, the window bounds, and the LpcIO-protected window.
    for base in [
      0x0000, 0x0293, 0x00F8, 0x1000, 0x0291, 0x0292, 0x0294, 0x0296, 0x0297, 0x00FD,
      0x1005, 0xFFFF, 0x0CF8, 0x0CFD,
    ] {
      assert_eq!(fintek_hm_ports(base), None, "0x{base:04X}");
    }
  }

  #[test]
  fn hm01_decodes_power_down_and_start_bits() {
    // "Synthetic cases": 0xFF; 0x02; 0x01; plus the fixture default 0x03.
    assert_eq!(fintek_monitor_state(0xFF), FintekMonitorState::PoweredDown);
    assert_eq!(fintek_monitor_state(0x04), FintekMonitorState::PoweredDown);
    assert_eq!(
      fintek_monitor_state(0x02),
      FintekMonitorState::Running {
        temperatures: false,
        fans: true
      }
    );
    assert_eq!(
      fintek_monitor_state(0x01),
      FintekMonitorState::Running {
        temperatures: true,
        fans: false
      }
    );
    assert_eq!(
      fintek_monitor_state(0x03),
      FintekMonitorState::Running {
        temperatures: true,
        fans: true
      }
    );
    // F71889 bits 7:5 are not gates.
    assert_eq!(
      fintek_monitor_state(0xE3),
      FintekMonitorState::Running {
        temperatures: true,
        fans: true
      }
    );
  }

  #[test]
  fn f71882_temp1_requires_the_external_diode_measurement_type() {
    // "Synthetic cases": 0x0A bits 1:0 = 01 or 10 make TEMP1 unavailable.
    assert_eq!(
      fintek_temp1_source(FintekChip::F71882, 0x00),
      FintekTemp1Source::ExternalDiode
    );
    assert_eq!(
      fintek_temp1_source(FintekChip::F71882, 0xFC),
      FintekTemp1Source::ExternalDiode
    );
    for hm0a in [0x01, 0x02, 0x03] {
      assert_eq!(
        fintek_temp1_source(FintekChip::F71882, hm0a),
        FintekTemp1Source::Other,
        "0x{hm0a:02X}"
      );
    }
  }

  #[test]
  fn f71889_temp1_follows_diode_and_digital_enables() {
    // "Synthetic cases": 0x0A = 0x01 read; 0x03 consults 0x7F; 0x02
    // unavailable.
    assert_eq!(
      fintek_temp1_source(FintekChip::F71889, 0x01),
      FintekTemp1Source::ExternalDiode
    );
    assert_eq!(
      fintek_temp1_source(FintekChip::F71889, 0x03),
      FintekTemp1Source::SourceSelected
    );
    assert_eq!(
      fintek_temp1_source(FintekChip::F71889, 0x02),
      FintekTemp1Source::Other
    );
    assert_eq!(
      fintek_temp1_source(FintekChip::F71889, 0x00),
      FintekTemp1Source::Other
    );
    // 0x7F = 0x00 reads D1+ only; 0x01, 0x02 and 0x03 do not.
    assert!(is_f71889_temp1_source_diode(0x00));
    assert!(is_f71889_temp1_source_diode(0xFC));
    for hm7f in [0x01, 0x02, 0x03] {
      assert!(!is_f71889_temp1_source_diode(hm7f), "0x{hm7f:02X}");
    }
  }

  #[test]
  fn diode_open_bits_map_to_their_inputs() {
    // "Synthetic cases": 0x6F = 0x04 makes only TEMP2 unavailable.
    let open = FINTEK_TEMPERATURE_INPUTS.map(|input| is_fintek_diode_open(0x04, &input));
    assert_eq!(open, [false, true, false]);
    let open = FINTEK_TEMPERATURE_INPUTS.map(|input| is_fintek_diode_open(0x0E, &input));
    assert_eq!(open, [true, true, true]);
    let open = FINTEK_TEMPERATURE_INPUTS.map(|input| is_fintek_diode_open(0xF1, &input));
    assert_eq!(open, [false, false, false]);
  }

  // --- "Temperature decode" ---

  #[test]
  fn f71882_temperature_bytes_decode_within_0_to_125() {
    // "Synthetic cases": 0x7E, 0x80, 0xD8, 0x00 and fixture 1 0x28, 0x1E, 0x7D.
    for (raw, expected) in [
      (0x7E, None),
      (0x80, None),
      (0xD8, None),
      (0x00, Some(0.0)),
      (0x28, Some(40.0)),
      (0x1E, Some(30.0)),
      (0x7D, Some(125.0)),
      (0x7F, None),
      (0xFF, None),
    ] {
      assert_eq!(
        decode_fintek_temperature(FintekChip::F71882, raw),
        expected,
        "0x{raw:02X}"
      );
    }
  }

  #[test]
  fn f71889_temperature_bytes_decode_as_signed_within_minus_40_to_125() {
    // "Synthetic cases": 0xD8, 0xD7, 0x80, 0x00, 0x7D, 0x7E.
    for (raw, expected) in [
      (0xD8, Some(-40.0)),
      (0xD7, None),
      (0x80, None),
      (0x00, Some(0.0)),
      (0x7D, Some(125.0)),
      (0x7E, None),
      (0x7F, None),
      (0xFF, Some(-1.0)),
    ] {
      assert_eq!(
        decode_fintek_temperature(FintekChip::F71889, raw),
        expected,
        "0x{raw:02X}"
      );
    }
  }

  // --- "Fan decode" ---

  #[test]
  fn fan_counts_decode_with_the_documented_limits() {
    // "Synthetic cases": routed fan counts.
    for (count, expected) in [
      (0x0000, None),
      (0x003F, None),
      (0x0040, Some(23_437)),
      (0x0FFE, Some(366)),
      (0x0FFF, Some(0)),
      (0x1000, None),
      (0xFFFF, None),
      (0x0001, None),
    ] {
      assert_eq!(decode_fintek_fan_rpm(count), expected, "0x{count:04X}");
    }
    // Fixture 1: FAN1 0x02/0xEE is 2000 RPM, FAN2 0x0F/0xFF is 0 RPM,
    // FAN3 0x00/0x00 is invalid.
    assert_eq!(
      decode_fintek_fan_rpm(fintek_fan_count(0x02, 0xEE)),
      Some(2000)
    );
    assert_eq!(decode_fintek_fan_rpm(fintek_fan_count(0x0F, 0xFF)), Some(0));
    assert_eq!(decode_fintek_fan_rpm(fintek_fan_count(0x00, 0x00)), None);
  }

  // --- "Fan pin routing" ---

  fn routing(cr27: u8, cr28: u8, cr2b: u8) -> FintekRoutingRegisters {
    FintekRoutingRegisters {
      cr27: Some(cr27),
      cr28: Some(cr28),
      cr2b: Some(cr2b),
    }
  }

  #[test]
  fn f71882_routing_follows_cr27_cr28_and_cr2b() {
    // Fixture 1: CR27=0x50 (SPI enabled), CR28=0x00, CR2B=0x00.
    assert_eq!(
      fintek_routed_fans(f71882(), &routing(0x50, 0x00, 0x00)),
      [true, true, true, false]
    );
    // FAN4 needs SPI disabled and CR2B bit 0 set.
    assert_eq!(
      fintek_routed_fans(f71882(), &routing(0x10, 0x00, 0x01)),
      [true, true, true, true]
    );
    assert_eq!(
      fintek_routed_fans(f71882(), &routing(0x10, 0x00, 0x00)),
      [true, true, true, false]
    );
    // CR28 bit 0 moves pin 25 to GPIO40.
    assert_eq!(
      fintek_routed_fans(f71882(), &routing(0x10, 0x01, 0x01)),
      [true, true, false, true]
    );
  }

  #[test]
  fn f71889_fan3_requires_cr2b_bits_1_0_clear() {
    // "Synthetic cases": CR2B bits 1:0 = 01 or 11 omit FAN3.
    for (cr2b, fan3) in [
      (0x00, true),
      (0xFC, true),
      (0x01, false),
      (0x02, false),
      (0x03, false),
    ] {
      let routing = FintekRoutingRegisters {
        cr2b: Some(cr2b),
        ..FintekRoutingRegisters::default()
      };
      assert_eq!(
        fintek_routed_fans(f71889(), &routing),
        [true, true, fan3, false],
        "0x{cr2b:02X}"
      );
    }
  }

  #[test]
  fn a_failed_routing_read_leaves_only_its_fans_unavailable() {
    let failed = FintekRoutingRegisters::default();
    assert_eq!(
      fintek_routed_fans(f71882(), &failed),
      [true, true, false, false]
    );
    assert_eq!(
      fintek_routed_fans(f71889(), &failed),
      [true, true, false, false]
    );
    let mut cr27_failed = routing(0x10, 0x00, 0x01);
    cr27_failed.record(FINTEK_CR27_REGISTER, None);
    assert_eq!(
      fintek_routed_fans(f71882(), &cr27_failed),
      [true, true, true, false]
    );
  }

  #[test]
  fn routing_registers_follow_the_row() {
    assert_eq!(f71882().routing_registers(), &[0x27, 0x28, 0x2B]);
    assert_eq!(f71889().routing_registers(), &[0x2B]);
    assert!(!f71882().checks_hm_vendor_id());
    assert!(f71889().checks_hm_vendor_id());
  }

  // --- "Registers that clear" and "Safety notes" ---

  #[test]
  fn hm_read_allow_list_excludes_every_never_read_register() {
    for profile in [f71882(), f71889()] {
      for register in FINTEK_NEVER_READ_HM_REGISTERS {
        assert!(
          !profile.is_allowed_hm_read(register),
          "{} 0x{register:02X}",
          profile.chip_label
        );
      }
    }
  }

  #[test]
  fn hm_read_allow_list_is_exactly_the_documented_reads() {
    let allowed = |profile: &FintekProfile| {
      (0..=u8::MAX)
        .filter(|register| profile.is_allowed_hm_read(*register))
        .collect::<Vec<_>>()
    };
    assert_eq!(
      allowed(f71882()),
      [
        0x01, 0x0A, 0x6F, 0x72, 0x74, 0x76, 0xA0, 0xA1, 0xB0, 0xB1, 0xC0, 0xC1, 0xD0,
        0xD1
      ]
    );
    assert_eq!(
      allowed(f71889()),
      [
        0x01, 0x0A, 0x5D, 0x5E, 0x6F, 0x72, 0x74, 0x76, 0x7F, 0xA0, 0xA1, 0xB0, 0xB1,
        0xC0, 0xC1
      ]
    );
  }
}
