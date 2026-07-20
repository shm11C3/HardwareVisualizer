//! Pure helpers for thermal-sensor (ACPI thermal zone) readings.
//!
//! The Windows thermal-zone provider and the collector use these to turn
//! raw WMI values into displayable °C readings. They are platform-neutral
//! pure functions so the logic stays unit-testable on every OS.

use crate::models::SensorTemperature;

/// Plausible range for a hardware temperature sensor in °C. Readings
/// outside this range are treated as firmware glitches (e.g. zones that
/// report 0 K or a constant placeholder) and dropped.
const MIN_PLAUSIBLE_CELSIUS: f32 = -40.0;
const MAX_PLAUSIBLE_CELSIUS: f32 = 150.0;

/// Convert tenths of Kelvin to °C.
///
/// `MSAcpi_ThermalZoneTemperature.CurrentTemperature` and the
/// `HighPrecisionTemperature` performance counter both use this unit.
pub fn decikelvin_to_celsius(raw: u32) -> f32 {
  raw as f32 / 10.0 - 273.15
}

/// Convert whole Kelvin to °C.
///
/// The `Temperature` field of
/// `Win32_PerfFormattedData_Counters_ThermalZoneInformation` uses this unit.
pub fn kelvin_to_celsius(raw: u32) -> f32 {
  raw as f32 - 273.15
}

/// Whether a reading looks like a real sensor value rather than a glitch.
pub fn is_plausible_celsius(celsius: f32) -> bool {
  (MIN_PLAUSIBLE_CELSIUS..=MAX_PLAUSIBLE_CELSIUS).contains(&celsius)
}

/// Extract a short display name from a WMI thermal-zone instance name.
///
/// Examples:
/// - `ACPI\ThermalZone\TZ00_0` → `TZ00` (MSAcpi instance name)
/// - `\_TZ.CPUZ` → `CPUZ` (performance-counter instance name)
/// - `THM0` → `THM0` (already short)
pub fn clean_zone_name(raw: &str) -> String {
  let tail = raw.rsplit(['\\', '.']).next().unwrap_or(raw);
  // MSAcpi instance names append a numeric `_<n>` suffix; strip it.
  let cleaned = match tail.rsplit_once('_') {
    Some((head, suffix))
      if !head.is_empty()
        && !suffix.is_empty()
        && suffix.bytes().all(|b| b.is_ascii_digit()) =>
    {
      head
    }
    _ => tail,
  };
  let cleaned = cleaned.trim();
  if cleaned.is_empty() {
    raw.trim().to_string()
  } else {
    cleaned.to_string()
  }
}

/// Collapse zones that share a cleaned name, keeping the hottest reading
/// per name and the first-seen order. Duplicate names can appear when two
/// WMI instances map to the same zone label.
pub fn dedupe_zones(zones: Vec<SensorTemperature>) -> Vec<SensorTemperature> {
  let mut result: Vec<SensorTemperature> = Vec::with_capacity(zones.len());
  for zone in zones {
    match result.iter_mut().find(|z| z.name == zone.name) {
      Some(existing) => existing.temperature = existing.temperature.max(zone.temperature),
      None => result.push(zone),
    }
  }
  result
}

/// Pick the headline "CPU temperature" from a set of thermal zones.
///
/// Preference order:
/// 1. A zone whose name mentions the CPU (`CPU` / `PROC`) — an exact match.
/// 2. Otherwise the hottest zone: on consumer ACPI tables the CPU package
///    usually dominates, so this is the best-effort estimate.
pub fn select_cpu_temperature(zones: &[SensorTemperature]) -> Option<f32> {
  let named = zones.iter().find(|z| {
    let upper = z.name.to_uppercase();
    upper.contains("CPU") || upper.contains("PROC")
  });
  if let Some(zone) = named {
    return Some(zone.temperature);
  }
  zones.iter().map(|z| z.temperature).reduce(f32::max)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn zone(name: &str, temperature: f32) -> SensorTemperature {
    SensorTemperature {
      name: name.to_string(),
      temperature,
    }
  }

  // ── decikelvin_to_celsius / kelvin_to_celsius ──

  #[test]
  fn decikelvin_to_celsius_typical_reading() {
    // 3231 deci-K = 323.1 K = 49.95 °C
    let celsius = decikelvin_to_celsius(3231);
    assert!((celsius - 49.95).abs() < 0.01);
  }

  #[test]
  fn decikelvin_to_celsius_zero_kelvin() {
    assert!((decikelvin_to_celsius(0) - -273.15).abs() < 0.01);
  }

  #[test]
  fn kelvin_to_celsius_typical_reading() {
    assert!((kelvin_to_celsius(318) - 44.85).abs() < 0.01);
  }

  // ── is_plausible_celsius ──

  #[test]
  fn plausible_range_accepts_normal_temperatures() {
    assert!(is_plausible_celsius(0.0));
    assert!(is_plausible_celsius(49.9));
    assert!(is_plausible_celsius(105.0));
    assert!(is_plausible_celsius(-40.0));
    assert!(is_plausible_celsius(150.0));
  }

  #[test]
  fn plausible_range_rejects_glitches() {
    // 0 K reported by a dead sensor
    assert!(!is_plausible_celsius(-273.15));
    assert!(!is_plausible_celsius(-40.1));
    assert!(!is_plausible_celsius(150.1));
    assert!(!is_plausible_celsius(f32::NAN));
  }

  // ── clean_zone_name ──

  #[test]
  fn clean_zone_name_msacpi_instance() {
    assert_eq!(clean_zone_name(r"ACPI\ThermalZone\TZ00_0"), "TZ00");
  }

  #[test]
  fn clean_zone_name_perf_counter_instance() {
    assert_eq!(clean_zone_name(r"\_TZ.CPUZ"), "CPUZ");
    assert_eq!(clean_zone_name(r"\_TZ.TZ01"), "TZ01");
  }

  #[test]
  fn clean_zone_name_already_short() {
    assert_eq!(clean_zone_name("THM0"), "THM0");
  }

  #[test]
  fn clean_zone_name_keeps_non_numeric_suffix() {
    assert_eq!(clean_zone_name(r"ACPI\ThermalZone\CPU_Z"), "CPU_Z");
  }

  #[test]
  fn clean_zone_name_empty_tail_falls_back_to_raw() {
    assert_eq!(clean_zone_name("TZ00."), "TZ00.");
  }

  // ── dedupe_zones ──

  #[test]
  fn dedupe_zones_keeps_unique_names_in_order() {
    let zones = vec![zone("TZ00", 40.0), zone("TZ01", 50.0)];
    let result = dedupe_zones(zones);
    assert_eq!(result, vec![zone("TZ00", 40.0), zone("TZ01", 50.0)]);
  }

  #[test]
  fn dedupe_zones_keeps_hottest_reading_per_name() {
    let zones = vec![zone("TZ00", 40.0), zone("TZ00", 55.0), zone("TZ00", 45.0)];
    let result = dedupe_zones(zones);
    assert_eq!(result, vec![zone("TZ00", 55.0)]);
  }

  // ── select_cpu_temperature ──

  #[test]
  fn select_cpu_temperature_empty_returns_none() {
    assert_eq!(select_cpu_temperature(&[]), None);
  }

  #[test]
  fn select_cpu_temperature_prefers_cpu_named_zone() {
    let zones = vec![zone("TZ00", 70.0), zone("CPUZ", 55.0)];
    assert_eq!(select_cpu_temperature(&zones), Some(55.0));
  }

  #[test]
  fn select_cpu_temperature_matches_proc_named_zone_case_insensitive() {
    let zones = vec![zone("TZ00", 70.0), zone("Proc1", 52.0)];
    assert_eq!(select_cpu_temperature(&zones), Some(52.0));
  }

  #[test]
  fn select_cpu_temperature_falls_back_to_hottest_zone() {
    let zones = vec![zone("TZ00", 41.0), zone("TZ01", 63.0), zone("TZ02", 47.0)];
    assert_eq!(select_cpu_temperature(&zones), Some(63.0));
  }
}
