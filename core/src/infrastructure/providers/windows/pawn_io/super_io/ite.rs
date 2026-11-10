//! Scoped ITE Environment Controller profiles and pure decoding helpers.
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-ite-it86xx-it87xx.md` revision 2
//! - `docs/specs/sensors/superio-ite-primary-lpc-expansion.md` revision 5
//! - `docs/specs/sensors/superio-ite-it8728f-ex-ga-b75m-d3h-fans.md` revision 1
//!
//! No other external sensor source was used.

use std::sync::{
  Mutex,
  mpsc::{Receiver, TryRecvError},
};

use super::is_plausible_super_io_temperature;
use crate::infrastructure::providers::windows::wmi_provider::{
  Win32BaseBoard, start_baseboard_identity_query,
};
use crate::{log_debug, log_warn};

type BaseboardIdentityQuery = Receiver<Result<Vec<Win32BaseBoard>, String>>;

// The service can drop and recreate its sampler while WMI is still running.
// Keep the receiver in the host so a later sampler can consume that same result.
static GA_B75M_D3H_LOOKUP: Mutex<GaB75mD3hLookup> =
  Mutex::new(GaB75mD3hLookup::Unstarted);

enum GaB75mD3hLookup {
  Unstarted,
  Pending {
    query: BaseboardIdentityQuery,
    can_retry: bool,
  },
  Resolved(bool),
}

impl GaB75mD3hLookup {
  fn resolved(&self) -> Option<bool> {
    match self {
      Self::Resolved(qualified) => Some(*qualified),
      _ => None,
    }
  }

  fn poll(
    &mut self,
    start_query: impl FnOnce() -> BaseboardIdentityQuery,
  ) -> Option<bool> {
    let (query, can_retry) = match self {
      Self::Unstarted => {
        *self = Self::Pending {
          query: start_query(),
          can_retry: true,
        };
        log_debug!(
          "ite_ga_b75m_d3h_baseboard_query_started",
          "windows::super_io::ite",
          None::<&str>
        );
        return None;
      }
      Self::Pending { query, can_retry } => (query, *can_retry),
      Self::Resolved(qualified) => return Some(*qualified),
    };

    let failure = match query.try_recv() {
      Ok(Ok(records)) => {
        let qualified = is_ga_b75m_d3h_baseboard(&records);
        *self = Self::Resolved(qualified);
        log_debug!(
          "ite_ga_b75m_d3h_baseboard_query_completed",
          "windows::super_io::ite",
          Some(format!("qualified={qualified}"))
        );
        return Some(qualified);
      }
      Err(TryRecvError::Empty) => return None,
      Ok(Err(_)) => "WMI query failed",
      Err(TryRecvError::Disconnected) => "WMI worker disconnected",
    };

    log_warn!(
      "ite_ga_b75m_d3h_baseboard_query_failed",
      "windows::super_io::ite",
      Some(format!("{failure}, retrying={can_retry}"))
    );
    if can_retry {
      *self = Self::Pending {
        query: start_query(),
        can_retry: false,
      };
      None
    } else {
      *self = Self::Resolved(false);
      Some(false)
    }
  }
}

pub(super) fn cached_ga_b75m_d3h_match() -> Option<bool> {
  GA_B75M_D3H_LOOKUP.lock().ok()?.resolved()
}

pub(super) fn poll_ga_b75m_d3h_match() -> Option<bool> {
  GA_B75M_D3H_LOOKUP
    .lock()
    .ok()?
    .poll(start_baseboard_identity_query)
}

/// TMPIN byte registers with their one-based channel for the CR51 eligibility
/// check.
const ITE_TEMPERATURE_REGISTERS: &[(&str, u8, u8)] = &[
  ("TMPIN1", 0x29, 1),
  ("TMPIN2", 0x2A, 2),
  ("TMPIN3", 0x2B, 3),
];

/// Fan counter low / high registers in channel order. The fan name is the key
/// in the live display and archive, so a missing input must not shift the
/// names of its siblings.
const ITE_FAN_REGISTERS: &[(&str, u8, u8)] = &[
  ("FAN1", 0x0D, 0x18),
  ("FAN2", 0x0E, 0x19),
  ("FAN3", 0x0F, 0x1A),
];

/// The GA-B75M-D3H factory board reads the first two counters under its board
/// labels.
const ITE_GA_B75M_D3H_FAN_REGISTERS: &[(&str, u8, u8)] =
  &[("Fan 1", 0x0D, 0x18), ("Fan 2", 0x0E, 0x19)];

const ITE_GA_B75M_D3H_EC_BASE: u16 = 0x0A30;
pub(super) const ITE_EC_FAN_MODE_REGISTER: u8 = 0x0C;

/// How the Environment Controller base is derived from CR60/61 and validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IteEcBaseRule {
  /// The raw CR60/61 value, 8-byte aligned inside `0x0100..=0x0FF0` and not
  /// `0x0CF8`.
  Modern,
  /// CR60/61 masked to `0x0FF8`, with the legacy validity checks.
  Legacy,
}

/// A board whose own mechanism selects a profile with more inputs than the
/// chip row alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct IteBoardVariant {
  pub(super) slot: u8,
  /// The raw CR60/61 value the board firmware programs.
  pub(super) ec_base: u16,
  /// The CR22 low nibble the board's chip reports.
  pub(super) revision: u8,
  pub(super) profile: &'static IteEcProfile,
}

/// One scoped ITE Environment Controller read profile, selected by raw chip ID.
/// Every scoped ITE profile is Experimental.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct IteEcProfile {
  /// Chip label surfaced in diagnostics, failure text and the source label.
  pub(super) chip_label: &'static str,
  /// Slots on which the raw chip ID selects this row.
  pub(super) slots: &'static [u8],
  pub(super) chip_id: u16,
  /// CR22 low nibble that must match before logical-device selection.
  pub(super) required_revision: Option<u8>,
  pub(super) ec_base_rule: IteEcBaseRule,
  /// Whether EC `0x00` bit 7 rejects the controller during discovery.
  pub(super) discovery_rejects_configuration_bit7: bool,
  /// Whether EC `0x00` bit 7 rejects a sample.
  pub(super) sampling_rejects_configuration_bit7: bool,
  /// Whether discovery requires EC `0x58` to hold the ITE vendor byte.
  pub(super) checks_vendor_byte: bool,
  /// Whether the FAN2 / FAN3 routing registers are read with the GPIO logical
  /// device selected, after which the EC logical device is reselected.
  pub(super) routes_in_gpio_ldn: bool,
  /// Temperature inputs; empty when the profile reads none.
  pub(super) temperature_registers: &'static [(&'static str, u8, u8)],
  /// Fan inputs in channel order; empty when the profile reads none.
  pub(super) fan_registers: &'static [(&'static str, u8, u8)],
  /// Whether FAN1 is a tachometer input. FAN2 and FAN3 follow their routing
  /// registers.
  pub(super) fan1_routed: bool,
  /// The EC register gating counters per channel, if the profile has one.
  pub(super) fan_mode_register: Option<u8>,
  pub(super) board_variant: Option<IteBoardVariant>,
}

const MODERN: IteEcProfile = IteEcProfile {
  chip_label: "",
  slots: &[0],
  chip_id: 0,
  required_revision: None,
  ec_base_rule: IteEcBaseRule::Modern,
  discovery_rejects_configuration_bit7: true,
  sampling_rejects_configuration_bit7: true,
  checks_vendor_byte: true,
  routes_in_gpio_ldn: true,
  temperature_registers: ITE_TEMPERATURE_REGISTERS,
  fan_registers: ITE_FAN_REGISTERS,
  fan1_routed: true,
  fan_mode_register: None,
  board_variant: None,
};

const LEGACY: IteEcProfile = IteEcProfile {
  ec_base_rule: IteEcBaseRule::Legacy,
  discovery_rejects_configuration_bit7: false,
  sampling_rejects_configuration_bit7: false,
  checks_vendor_byte: false,
  routes_in_gpio_ldn: false,
  ..MODERN
};

pub(super) const IT8620E: IteEcProfile = IteEcProfile {
  chip_label: "IT8620E",
  chip_id: 0x8620,
  ..MODERN
};

pub(super) const IT8628E: IteEcProfile = IteEcProfile {
  chip_label: "IT8628E",
  chip_id: 0x8628,
  ..MODERN
};

pub(super) const IT8716F_D: IteEcProfile = IteEcProfile {
  chip_label: "IT8716F D",
  chip_id: 0x8716,
  required_revision: Some(0x01),
  temperature_registers: &[],
  fan_mode_register: Some(ITE_EC_FAN_MODE_REGISTER),
  ..LEGACY
};

pub(super) const IT8728F: IteEcProfile = IteEcProfile {
  chip_label: "IT8728F/EX",
  slots: &[0, 1],
  chip_id: 0x8728,
  fan_registers: &[],
  fan1_routed: false,
  board_variant: Some(IteBoardVariant {
    slot: 0,
    ec_base: ITE_GA_B75M_D3H_EC_BASE,
    revision: 0x01,
    profile: &IT8728F_EX_GA_B75M_D3H,
  }),
  ..LEGACY
};

/// Reached only through [`IT8728F`]'s board variant.
pub(super) const IT8728F_EX_GA_B75M_D3H: IteEcProfile = IteEcProfile {
  chip_label: "IT8728F/EX",
  slots: &[0],
  chip_id: 0x8728,
  fan_registers: ITE_GA_B75M_D3H_FAN_REGISTERS,
  ..LEGACY
};

/// FX has only the FAN2 and FAN3 tachometer inputs, regardless of EC 0x13[4].
pub(super) const IT8772E_FX: IteEcProfile = IteEcProfile {
  chip_label: "IT8772E FX",
  chip_id: 0x8772,
  required_revision: Some(0x02),
  sampling_rejects_configuration_bit7: true,
  checks_vendor_byte: true,
  temperature_registers: &[],
  fan1_routed: false,
  ..LEGACY
};

const ITE_EC_PROFILES: [&IteEcProfile; 5] =
  [&IT8620E, &IT8628E, &IT8716F_D, &IT8728F, &IT8772E_FX];

/// Match the raw chip ID exactly. A profile with a required revision still
/// needs the CR22 check before logical-device selection.
pub(super) fn ite_ec_profile(slot: u8, id: u16) -> Option<&'static IteEcProfile> {
  ITE_EC_PROFILES
    .into_iter()
    .find(|profile| profile.chip_id == id && profile.slots.contains(&slot))
}

/// Check cardinality before considering any field. The inventory API's first
/// row alone cannot distinguish this factory board from an ambiguous result.
pub(super) fn is_ga_b75m_d3h_baseboard(records: &[Win32BaseBoard]) -> bool {
  let [board] = records else {
    return false;
  };

  board.manufacturer.as_deref().map(str::trim) == Some("Gigabyte Technology Co., Ltd.")
    && board.product.as_deref().map(str::trim) == Some("B75M-D3H")
    && matches!(
      board.version.as_deref().map(str::trim),
      Some("1.0" | "1.1" | "1.2" | "x.x")
    )
}

/// Decode a scoped ITE Environment Controller temperature byte as signed
/// degrees C.
pub const fn decode_ite_temperature_byte(raw: u8) -> f32 {
  (raw as i8) as f32
}

/// Whether a temperature is inside the scoped ITE plausibility range.
/// Zero is a valid reading for an enabled physical input.
pub const fn is_plausible_ite_temperature(temperature: f32) -> bool {
  is_plausible_super_io_temperature(temperature)
}

/// Whether one TMPIN is an enabled physical temperature input.
///
/// `channel` is one-based (`1..=3`). A channel is eligible only when exactly
/// one of its resistor/diode mode bits is set and the SST/PECI route does not
/// target that TMPIN.
pub const fn is_ite_temperature_channel_eligible(config: u8, channel: u8) -> bool {
  let (resistor_bit, diode_bit) = match channel {
    1 => (3, 0),
    2 => (4, 1),
    3 => (5, 2),
    _ => return false,
  };
  let resistor_enabled = (config & (1 << resistor_bit)) != 0;
  let diode_enabled = (config & (1 << diode_bit)) != 0;
  let routed_external_channel = config >> 6;

  resistor_enabled != diode_enabled && routed_external_channel != channel
}

/// Convert one stable 16-bit count after all profile and channel gates passed.
pub(super) const fn decode_ite_fan_rpm(count: u16) -> Option<u32> {
  match count {
    0..=0x003F => None,
    0xFFFF => Some(0),
    _ => Some(675_000 / count as u32),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn profiles_are_selected_by_slot_and_exact_chip_id() {
    let label = |slot, id| ite_ec_profile(slot, id).map(|profile| profile.chip_label);

    assert_eq!(label(0, 0x8620), Some("IT8620E"));
    assert_eq!(label(0, 0x8628), Some("IT8628E"));
    assert_eq!(label(0, 0x8716), Some("IT8716F D"));
    assert_eq!(label(0, 0x8728), Some("IT8728F/EX"));
    assert_eq!(label(1, 0x8728), Some("IT8728F/EX"));
    assert_eq!(label(0, 0x8772), Some("IT8772E FX"));
    assert_eq!(label(1, 0x8620), None);
    assert_eq!(label(0, 0x8721), None);
  }

  #[test]
  fn the_board_variant_is_reached_only_through_its_chip_row() {
    assert!(
      ITE_EC_PROFILES
        .iter()
        .all(|profile| **profile != IT8728F_EX_GA_B75M_D3H)
    );
    let variant = IT8728F.board_variant.expect("IT8728F has a board variant");
    assert_eq!(variant.profile, &IT8728F_EX_GA_B75M_D3H);
    assert_eq!(variant.profile.chip_label, IT8728F.chip_label);
    assert_eq!(variant.ec_base, ITE_GA_B75M_D3H_EC_BASE);
  }

  #[test]
  fn late_ga_b75m_d3h_identity_remains_pending_until_the_existing_worker_replies() {
    let mut lookup = GaB75mD3hLookup::Unstarted;
    let (send, receive) = std::sync::mpsc::channel();
    assert_eq!(lookup.poll(|| receive), None);
    for _ in 0..3 {
      assert_eq!(lookup.resolved(), None);
      assert_eq!(
        lookup.poll(|| panic!("an in-flight WMI query cannot be replaced")),
        None
      );
    }

    let board = serde_json::from_value::<Win32BaseBoard>(serde_json::json!({
      "Manufacturer": "Gigabyte Technology Co., Ltd.",
      "Product": "B75M-D3H",
      "Version": "x.x",
    }))
    .unwrap();
    send.send(Ok(vec![board])).unwrap();
    assert_eq!(
      lookup.poll(|| panic!("the late result is usable")),
      Some(true)
    );
    assert_eq!(lookup.resolved(), Some(true));
  }

  #[test]
  fn ga_b75m_d3h_identity_retries_a_failed_worker_once() {
    let mut lookup = GaB75mD3hLookup::Unstarted;
    let (first_send, first_receive) = std::sync::mpsc::channel();
    let (retry_send, retry_receive) = std::sync::mpsc::channel();
    assert_eq!(lookup.poll(|| first_receive), None);
    first_send
      .send(Err("synthetic WMI failure".into()))
      .unwrap();
    assert_eq!(lookup.poll(|| retry_receive), None);
    assert_eq!(lookup.poll(|| panic!("the retry is still in flight")), None);

    drop(retry_send);
    assert_eq!(
      lookup.poll(|| panic!("only one retry is permitted")),
      Some(false)
    );
  }

  #[test]
  fn ga_b75m_d3h_identity_requires_one_complete_exact_baseboard() {
    let record = |manufacturer: &str, product: &str, version: Option<&str>| {
      serde_json::from_value::<Win32BaseBoard>(serde_json::json!({
        "Manufacturer": manufacturer,
        "Product": product,
        "Version": version,
      }))
      .unwrap()
    };
    let manufacturer = "Gigabyte Technology Co., Ltd.";
    for version in ["1.0", "1.1", "1.2", "x.x", " x.x\t"] {
      assert!(is_ga_b75m_d3h_baseboard(&[record(
        " Gigabyte Technology Co., Ltd. ",
        " B75M-D3H ",
        Some(version)
      )]));
    }
    assert!(!is_ga_b75m_d3h_baseboard(&[]));
    for other_product in ["B75M-D3H", "B75M-D3V"] {
      assert!(!is_ga_b75m_d3h_baseboard(&[
        record(manufacturer, "B75M-D3H", Some("x.x")),
        record(manufacturer, other_product, Some("x.x")),
      ]));
    }
    for (manufacturer, product, version) in [
      (manufacturer, "B75M-D3V", Some("x.x")),
      (manufacturer, "GA-B75M-D3H", Some("x.x")),
      ("GIGABYTE Technology Co., Ltd.", "B75M-D3H", Some("x.x")),
      (manufacturer, "B75M-D3H", Some("1.3")),
      (manufacturer, "B75M-D3H", Some(" ")),
      (manufacturer, "B75M-D3H", None),
    ] {
      assert!(!is_ga_b75m_d3h_baseboard(&[record(
        manufacturer,
        product,
        version
      )]));
    }
  }

  #[test]
  fn ite_temperature_byte_decodes_signed_celsius() {
    assert_eq!(decode_ite_temperature_byte(0x7D), 125.0);
    assert_eq!(decode_ite_temperature_byte(0x19), 25.0);
    assert_eq!(decode_ite_temperature_byte(0x00), 0.0);
    assert_eq!(decode_ite_temperature_byte(0xFF), -1.0);
    assert_eq!(decode_ite_temperature_byte(0xC9), -55.0);
  }

  #[test]
  fn ite_temperature_plausibility_keeps_zero_and_rejects_outside_bounds() {
    assert!(is_plausible_ite_temperature(-55.0));
    assert!(is_plausible_ite_temperature(0.0));
    assert!(is_plausible_ite_temperature(125.0));
    assert!(!is_plausible_ite_temperature(-56.0));
    assert!(!is_plausible_ite_temperature(126.0));
  }

  #[test]
  fn ite_temperature_channel_requires_one_physical_mode_and_no_external_route() {
    assert!(is_ite_temperature_channel_eligible(0b0000_1000, 1));
    assert!(is_ite_temperature_channel_eligible(0b0000_0010, 2));
    assert!(is_ite_temperature_channel_eligible(0b0010_0000, 3));

    assert!(!is_ite_temperature_channel_eligible(0, 1));
    assert!(!is_ite_temperature_channel_eligible(0b0000_1001, 1));
    assert!(!is_ite_temperature_channel_eligible(0b0100_1000, 1));
    assert!(!is_ite_temperature_channel_eligible(0b1000_0010, 2));
    assert!(!is_ite_temperature_channel_eligible(0b1110_0000, 3));
    assert!(!is_ite_temperature_channel_eligible(0xFF, 4));
  }

  #[test]
  fn ite_16_bit_fan_count_distinguishes_invalid_active_and_inactive() {
    assert_eq!(decode_ite_fan_rpm(0x0000), None);
    assert_eq!(decode_ite_fan_rpm(0x0001), None);
    assert_eq!(decode_ite_fan_rpm(0x003F), None);
    assert_eq!(decode_ite_fan_rpm(0x0040), Some(10_546));
    assert_eq!(decode_ite_fan_rpm(0x01C2), Some(1_500));
    assert_eq!(decode_ite_fan_rpm(0xFFFE), Some(10));
    assert_eq!(decode_ite_fan_rpm(0xFFFF), Some(0));
  }
}
