//! Scoped ITE Environment Controller profiles and pure decoding helpers.
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-ite-it86xx-it87xx.md` revision 2
//! - `docs/specs/sensors/superio-ite-primary-lpc-expansion.md` revision 4
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IteEcProfile {
  It8620e,
  It8628e,
  It8716fD,
  It8728f,
  It8728fExGaB75mD3h,
}

impl IteEcProfile {
  /// Match the raw chip ID exactly. IT8716F still requires a CR22 D-version
  /// check in configuration mode before any logical-device selection.
  pub(super) const fn from_slot_and_chip_id(slot: u8, id: u16) -> Option<Self> {
    match (slot, id) {
      (0, 0x8620) => Some(Self::It8620e),
      (0, 0x8628) => Some(Self::It8628e),
      (0, 0x8716) => Some(Self::It8716fD),
      (0 | 1, 0x8728) => Some(Self::It8728f),
      _ => None,
    }
  }

  pub(super) const fn chip_label(self) -> &'static str {
    match self {
      Self::It8620e => "IT8620E",
      Self::It8628e => "IT8628E",
      Self::It8716fD => "IT8716F D",
      Self::It8728f | Self::It8728fExGaB75mD3h => "IT8728F/EX",
    }
  }

  pub(super) const fn is_modern(self) -> bool {
    matches!(self, Self::It8620e | Self::It8628e)
  }

  pub(super) const fn has_temperatures(self) -> bool {
    !matches!(self, Self::It8716fD)
  }

  pub(super) const fn has_fans(self) -> bool {
    !matches!(self, Self::It8728f)
  }
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
