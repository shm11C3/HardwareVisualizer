//! Environmental (ambient) sensor abstraction (#2043).
//!
//! Ambient room temperature is an explanatory input for Cooling Insight,
//! never a required one: a +6 °C summer drift and a genuine cooling
//! degradation look identical without it. Core owns the shape of an
//! ambient reading and the freshness rule that decides whether a reading
//! may represent a given archive minute; concrete transports (BLE, USB,
//! or anything else) live behind [`EnvironmentalSensorProvider`] and are
//! registered from outside this module. No vendor type appears here.
//!
//! Providers are polled, not awaited. Every transport in scope pushes
//! readings on its own cadence (a BLE advertisement arrives when the
//! device chooses to send one), so a provider caches its latest sample
//! and the archive tick reads that cache without blocking on I/O.

use std::collections::HashSet;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::log_warn;

/// How old an ambient reading may be and still represent the minute
/// currently being archived.
///
/// Five minutes. Indoor air temperature is a slow signal: under normal
/// occupancy and HVAC behavior a room moves well under 1 °C in five
/// minutes, which is inside the accuracy an ambient sensor reports in
/// the first place, so a five-minute-old sample still describes the
/// current minute honestly. It is also many multiples of a BLE
/// advertisement interval (seconds to tens of seconds), so ordinary
/// packet loss or a missed scan window does not punch holes in the
/// archive. A sensor that genuinely stops reporting therefore stops
/// producing rows within five minutes instead of freezing its last value
/// across hours of history.
pub const AMBIENT_READING_MAX_AGE_SECONDS: i64 = 5 * 60;

/// One ambient environment sample.
///
/// `temperature_celsius` is the raw hardware fact in degrees Celsius;
/// presentation units belong at the App boundary. `humidity_percent` is
/// optional because a temperature-only sensor is a legitimate ambient
/// source. `source` is the Sensor Source Label recorded with the reading.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentalReading {
  pub temperature_celsius: f32,
  pub humidity_percent: Option<f32>,
  pub timestamp: DateTime<Utc>,
  pub source: String,
}

/// Whether a provider's transport is currently delivering readings.
///
/// Deliberately two states: "configured but never seen a reading" is
/// `Disconnected` with no last reading timestamp, so the data-state panel
/// can tell that apart from a link that dropped after working.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentalConnectionState {
  /// The transport is up and further readings are expected.
  Connected,
  /// The provider is registered but its transport is not delivering
  /// (adapter off, device out of range, permission refused, never paired).
  Disconnected,
}

/// Per-provider state for the Phase 4 data-state panel.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentalProviderState {
  /// Sensor Source Label identifying the provider.
  pub source: String,
  pub connection: EnvironmentalConnectionState,
  /// Timestamp of the newest reading the provider holds, or `None` when
  /// it has never produced one. Not filtered by freshness - the panel
  /// wants to show how stale the last success is, not hide it.
  pub last_reading_at: Option<DateTime<Utc>>,
}

/// A source of ambient environment readings.
///
/// Implementations cache whatever their transport last delivered and
/// answer from that cache; nothing here may block on I/O, because the
/// hardware-archive tick calls it inline.
pub trait EnvironmentalSensorProvider: Send + Sync {
  /// Sensor Source Label identifying this provider. Readings it returns
  /// are expected to carry the same label.
  fn source(&self) -> &str;

  /// The newest reading held, or `None` when the provider has never
  /// observed one. Freshness is judged by the caller, not here.
  fn latest_reading(&self) -> Option<EnvironmentalReading>;

  /// Whether the transport is currently delivering readings.
  fn connection_state(&self) -> EnvironmentalConnectionState;
}

/// The set of environmental providers this process collects from.
///
/// Built once at startup and then read-only, so it is shared as an
/// `Arc`. With no provider registered every method is trivially empty and
/// the archive tick writes no ambient rows - ambient data stays optional.
#[derive(Default, Clone)]
pub struct EnvironmentalSensorRegistry {
  providers: Vec<Arc<dyn EnvironmentalSensorProvider>>,
}

impl EnvironmentalSensorRegistry {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn register(&mut self, provider: Arc<dyn EnvironmentalSensorProvider>) {
    self.providers.push(provider);
  }

  pub fn is_empty(&self) -> bool {
    self.providers.is_empty()
  }

  /// Readings that may represent the minute ending at `now`, one per
  /// source.
  ///
  /// A reading is dropped rather than repaired: stale, unlabeled, or
  /// non-finite readings produce no row at all, because a minute without
  /// a usable ambient sample must stay absent instead of being zeroed or
  /// interpolated (DP-02).
  pub fn fresh_readings(&self, now: DateTime<Utc>) -> Vec<EnvironmentalReading> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut readings = Vec::new();

    for provider in &self.providers {
      let Some(reading) = provider
        .latest_reading()
        .and_then(|reading| normalize_reading(reading, now))
      else {
        continue;
      };

      // Row-per-source only stays joinable if one source contributes one
      // row per minute. Two providers claiming the same label would
      // otherwise double-count in every downstream rollup.
      if !seen.insert(reading.source.clone()) {
        log_warn!(
          &format!(
            "duplicate ambient Sensor Source Label `{}`; keeping the first provider's reading",
            reading.source
          ),
          "providers::environmental::fresh_readings",
          None::<&str>
        );
        continue;
      }

      readings.push(reading);
    }

    readings
  }

  /// Connection state and last-success timestamp for every registered
  /// provider, in registration order.
  pub fn provider_states(&self) -> Vec<EnvironmentalProviderState> {
    self
      .providers
      .iter()
      .map(|provider| EnvironmentalProviderState {
        source: provider.source().to_string(),
        connection: provider.connection_state(),
        last_reading_at: provider.latest_reading().map(|reading| reading.timestamp),
      })
      .collect()
  }
}

/// Normalize one reading for archiving, or `None` when it cannot honestly
/// stand for the minute ending at `now`.
fn normalize_reading(
  reading: EnvironmentalReading,
  now: DateTime<Utc>,
) -> Option<EnvironmentalReading> {
  let source = reading.source.trim();
  // An ambient row is only meaningful attributed to a source: the table
  // is row-per-source and every consumer selects by label.
  if source.is_empty() || !reading.temperature_celsius.is_finite() {
    return None;
  }

  // A reading stamped slightly ahead of `now` (host clock jitter between
  // the transport callback and the archive tick) is still current, so the
  // window is one-sided.
  let age = now.signed_duration_since(reading.timestamp);
  if age > Duration::seconds(AMBIENT_READING_MAX_AGE_SECONDS) {
    return None;
  }

  Some(EnvironmentalReading {
    temperature_celsius: reading.temperature_celsius,
    // Humidity is an optional extra on an otherwise usable reading, so a
    // garbage humidity value drops the field, never the row.
    humidity_percent: reading.humidity_percent.filter(|value| value.is_finite()),
    timestamp: reading.timestamp,
    source: source.to_string(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Minimal provider used to drive the registry without any transport.
  struct MockProvider {
    source: String,
    reading: Option<EnvironmentalReading>,
    connection: EnvironmentalConnectionState,
  }

  impl MockProvider {
    fn connected(source: &str, reading: EnvironmentalReading) -> Arc<Self> {
      Arc::new(Self {
        source: source.to_string(),
        reading: Some(reading),
        connection: EnvironmentalConnectionState::Connected,
      })
    }

    fn silent(source: &str) -> Arc<Self> {
      Arc::new(Self {
        source: source.to_string(),
        reading: None,
        connection: EnvironmentalConnectionState::Disconnected,
      })
    }
  }

  impl EnvironmentalSensorProvider for MockProvider {
    fn source(&self) -> &str {
      &self.source
    }

    fn latest_reading(&self) -> Option<EnvironmentalReading> {
      self.reading.clone()
    }

    fn connection_state(&self) -> EnvironmentalConnectionState {
      self.connection
    }
  }

  fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-30T12:00:00Z")
      .unwrap()
      .with_timezone(&Utc)
  }

  fn reading(source: &str, seconds_old: i64) -> EnvironmentalReading {
    EnvironmentalReading {
      temperature_celsius: 24.5,
      humidity_percent: Some(48.0),
      timestamp: now() - Duration::seconds(seconds_old),
      source: source.to_string(),
    }
  }

  // -- registry with no provider --

  #[test]
  fn an_empty_registry_produces_no_ambient_rows() {
    let registry = EnvironmentalSensorRegistry::new();
    assert!(registry.is_empty());
    assert!(registry.fresh_readings(now()).is_empty());
    assert!(registry.provider_states().is_empty());
  }

  // -- staleness --

  #[test]
  fn a_reading_inside_the_freshness_window_represents_the_minute() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::connected("Room", reading("Room", 90)));

    let fresh = registry.fresh_readings(now());
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].temperature_celsius, 24.5);
    assert_eq!(fresh[0].humidity_percent, Some(48.0));
  }

  #[test]
  fn a_reading_exactly_at_the_freshness_limit_is_still_accepted() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::connected(
      "Room",
      reading("Room", AMBIENT_READING_MAX_AGE_SECONDS),
    ));

    assert_eq!(registry.fresh_readings(now()).len(), 1);
  }

  #[test]
  fn a_reading_past_the_freshness_limit_writes_no_row_rather_than_repeating() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::connected(
      "Room",
      reading("Room", AMBIENT_READING_MAX_AGE_SECONDS + 1),
    ));

    assert!(
      registry.fresh_readings(now()).is_empty(),
      "a stale sample must leave the minute absent, not carry a frozen value forward"
    );
  }

  #[test]
  fn a_reading_stamped_slightly_ahead_of_the_tick_is_accepted() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::connected("Room", reading("Room", -2)));

    assert_eq!(registry.fresh_readings(now()).len(), 1);
  }

  // -- normalization --

  #[test]
  fn a_non_finite_temperature_writes_no_row() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut broken = reading("Room", 0);
    broken.temperature_celsius = f32::NAN;
    registry.register(MockProvider::connected("Room", broken));

    assert!(registry.fresh_readings(now()).is_empty());
  }

  #[test]
  fn a_non_finite_humidity_drops_only_the_humidity() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut partial = reading("Room", 0);
    partial.humidity_percent = Some(f32::INFINITY);
    registry.register(MockProvider::connected("Room", partial));

    let fresh = registry.fresh_readings(now());
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].temperature_celsius, 24.5);
    assert_eq!(fresh[0].humidity_percent, None);
  }

  #[test]
  fn a_temperature_only_reading_is_archived_without_humidity() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut dry = reading("Room", 0);
    dry.humidity_percent = None;
    registry.register(MockProvider::connected("Room", dry));

    let fresh = registry.fresh_readings(now());
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].humidity_percent, None);
  }

  #[test]
  fn an_unlabeled_reading_writes_no_row() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut anonymous = reading("Room", 0);
    anonymous.source = "   ".to_string();
    registry.register(MockProvider::connected("Room", anonymous));

    assert!(registry.fresh_readings(now()).is_empty());
  }

  #[test]
  fn the_stored_source_label_is_trimmed() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut padded = reading("Room", 0);
    padded.source = "  Living Room  ".to_string();
    registry.register(MockProvider::connected("Room", padded));

    assert_eq!(registry.fresh_readings(now())[0].source, "Living Room");
  }

  // -- multiple sources --

  #[test]
  fn each_source_contributes_its_own_row() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut desk = reading("Desk", 10);
    desk.temperature_celsius = 26.0;
    registry.register(MockProvider::connected("Room", reading("Room", 10)));
    registry.register(MockProvider::connected("Desk", desk));

    let fresh = registry.fresh_readings(now());
    assert_eq!(fresh.len(), 2);
    assert_eq!(fresh[0].source, "Room");
    assert_eq!(fresh[1].source, "Desk");
    assert_eq!(fresh[1].temperature_celsius, 26.0);
  }

  #[test]
  fn one_stale_source_does_not_suppress_a_fresh_one() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::connected(
      "Room",
      reading("Room", AMBIENT_READING_MAX_AGE_SECONDS + 60),
    ));
    registry.register(MockProvider::connected("Desk", reading("Desk", 5)));

    let fresh = registry.fresh_readings(now());
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].source, "Desk");
  }

  #[test]
  fn a_duplicated_source_label_contributes_only_one_row_per_minute() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::connected("Room", reading("Room", 5)));
    let mut second = reading("Room", 5);
    second.temperature_celsius = 31.0;
    registry.register(MockProvider::connected("Room", second));

    let fresh = registry.fresh_readings(now());
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].temperature_celsius, 24.5);
  }

  // -- connection state --

  #[test]
  fn provider_states_report_connection_and_last_success() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::connected("Room", reading("Room", 30)));
    registry.register(MockProvider::silent("Desk"));

    assert_eq!(
      registry.provider_states(),
      vec![
        EnvironmentalProviderState {
          source: "Room".to_string(),
          connection: EnvironmentalConnectionState::Connected,
          last_reading_at: Some(now() - Duration::seconds(30)),
        },
        EnvironmentalProviderState {
          source: "Desk".to_string(),
          connection: EnvironmentalConnectionState::Disconnected,
          last_reading_at: None,
        },
      ]
    );
  }

  #[test]
  fn provider_states_keep_reporting_a_last_success_that_is_now_stale() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::connected(
      "Room",
      reading("Room", AMBIENT_READING_MAX_AGE_SECONDS * 10),
    ));

    // The archive stops writing rows, but the panel must still be able to
    // say how long ago the sensor last reported.
    assert!(registry.fresh_readings(now()).is_empty());
    assert_eq!(
      registry.provider_states()[0].last_reading_at,
      Some(now() - Duration::seconds(AMBIENT_READING_MAX_AGE_SECONDS * 10))
    );
  }
}
