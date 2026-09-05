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

/// How far ahead of the archive tick a reading may be stamped and still
/// be believed.
///
/// Sixty seconds. Both stamps come from the same host - the transport
/// callback records the observation, the archive tick reads it moments
/// later - so the only legitimate future skew is clock jitter or a small
/// adjustment in between, which is milliseconds in normal operation. A
/// full archive interval of slack absorbs that with room to spare while
/// still bounding the failure the window would otherwise have: without an
/// upper bound, one reading stamped far into the future by a clock
/// rewind, a bad NTP step, or a bogus device timestamp would count as
/// fresh for as long as it stayed ahead of `now` - potentially days of
/// identical rows. Past that bound the reading is refused instead.
pub const AMBIENT_READING_MAX_FUTURE_SKEW_SECONDS: i64 = 60;

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

/// Whether an ambient source's readings are arriving.
///
/// Deliberately *not* a connection state. The first concrete provider
/// listens to passive BLE advertisements and never establishes a
/// connection, so a link concept has no shared meaning here. Availability
/// is defined by observed readings alone: transport-specific causes
/// (radio unavailable, scan not running, device out of range) stay inside
/// the concrete provider and surface only as readings that stop arriving.
///
/// `Available` means exactly one thing: this source is eligible to have
/// an ambient row written for the current archive minute. The panel and
/// the archive read the same eligibility rule, so the panel can never
/// call a source available while nothing is even being attempted for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbientSensorAvailability {
  /// A usable reading arrived inside the freshness window, so this source
  /// is eligible to represent the current archive minute.
  Available,
  /// Usable readings arrived before, but the newest one is outside the
  /// freshness window. The archive writes no row for this source; the
  /// panel still shows how long ago it last succeeded.
  Stale,
  /// Nothing usable is arriving: the source has never reported, or what
  /// it reports cannot be archived (no Sensor Source Label, a non-finite
  /// temperature, or a label another provider already claimed this
  /// minute).
  Unavailable,
}

/// Per-provider status for the Phase 4 data-state panel: is the source
/// arriving, and when did it last succeed.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentalProviderStatus {
  /// Sensor Source Label the panel shows. Once a usable reading exists
  /// this is that reading's normalized label - the same key the ambient
  /// row is written under - so a status line and its archive rows can
  /// never disagree about which source they describe. Before any usable
  /// reading it falls back to the provider's own declared label.
  pub source: String,
  pub availability: AmbientSensorAvailability,
  /// Timestamp of the newest *usable* reading, or `None` when none has
  /// arrived. An unusable reading does not advance it: reporting a fresh
  /// success the archive rejected would misdescribe the source. Not
  /// filtered by freshness - the panel wants to show how stale the last
  /// success is, not hide it.
  pub last_reading_at: Option<DateTime<Utc>>,
}

/// A source of ambient environment readings.
///
/// Implementations cache whatever their transport last delivered and
/// answer from that cache; nothing here may block on I/O, because the
/// hardware-archive tick calls it inline. The contract is deliberately
/// two observations - who the source is and what it last reported -
/// because every status the app shows is derived from those. A provider
/// that internally knows *why* nothing is arriving keeps that reason to
/// itself rather than widening this trait with a transport concept.
pub trait EnvironmentalSensorProvider: Send + Sync {
  /// Sensor Source Label identifying this provider. Readings it returns
  /// are expected to carry the same label. Owned rather than borrowed
  /// because a provider whose device is chosen at runtime keeps its
  /// label behind a lock.
  fn source(&self) -> String;

  /// The newest reading held, or `None` when the provider has never
  /// observed one. Freshness is judged by the caller, not here.
  fn latest_reading(&self) -> Option<EnvironmentalReading>;
}

/// The set of environmental providers this process collects from.
///
/// Built once at startup and then read-only, so it is shared as an
/// `Arc`. With no provider registered every method is trivially empty and
/// the archive tick writes no ambient rows - ambient data stays optional.
#[derive(Default)]
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
    self
      .observe(now)
      .into_iter()
      .filter_map(|observation| observation.eligible)
      .collect()
  }

  /// Availability and last-success timestamp for every registered
  /// provider as of `now`, in registration order.
  ///
  /// Derived from the same evaluation [`Self::fresh_readings`] writes
  /// from, so `Available` and "a row is attempted" cannot drift apart.
  pub fn provider_statuses(
    &self,
    now: DateTime<Utc>,
  ) -> Vec<EnvironmentalProviderStatus> {
    self
      .observe(now)
      .into_iter()
      .map(|observation| observation.status)
      .collect()
  }

  /// Evaluate every provider once for the minute ending at `now`.
  ///
  /// This is the single place the eligibility rule lives. Both public
  /// readers project out of it, which is what makes "the panel says
  /// Available" and "the archive attempts a row" the same condition by
  /// construction rather than by two rules kept in sync by hand.
  fn observe(&self, now: DateTime<Utc>) -> Vec<ProviderObservation> {
    let mut claimed: HashSet<String> = HashSet::new();

    self
      .providers
      .iter()
      .map(|provider| {
        // An unusable reading is not a success, so it neither yields a row
        // nor advances the last-success timestamp the panel shows.
        let Some(reading) = provider.latest_reading().and_then(normalize_reading) else {
          return ProviderObservation::unusable(&provider.source());
        };

        if !is_fresh(reading.timestamp, now) {
          return ProviderObservation::stale(reading);
        }

        // Row-per-source only stays joinable if one source contributes one
        // row per minute. Two providers claiming the same label would
        // otherwise double-count in every downstream rollup, so the later
        // one is reported as contributing nothing rather than silently
        // looking healthy.
        if !claimed.insert(reading.source.clone()) {
          log_warn!(
            &format!(
              "duplicate ambient Sensor Source Label `{}`; keeping the first provider's reading",
              reading.source
            ),
            "providers::environmental::observe",
            None::<&str>
          );
          return ProviderObservation::unclaimable(reading);
        }

        ProviderObservation::available(reading)
      })
      .collect()
  }
}

/// One provider's evaluated contribution to a single archive minute.
struct ProviderObservation {
  status: EnvironmentalProviderStatus,
  /// The row to write, present exactly when the status is
  /// [`AmbientSensorAvailability::Available`].
  eligible: Option<EnvironmentalReading>,
}

impl ProviderObservation {
  /// Nothing usable has ever arrived, so the provider's own declared
  /// label is all there is to identify it by.
  fn unusable(source: &str) -> Self {
    Self {
      status: EnvironmentalProviderStatus {
        source: source.trim().to_string(),
        availability: AmbientSensorAvailability::Unavailable,
        last_reading_at: None,
      },
      eligible: None,
    }
  }

  fn stale(reading: EnvironmentalReading) -> Self {
    Self {
      status: EnvironmentalProviderStatus {
        source: reading.source,
        availability: AmbientSensorAvailability::Stale,
        last_reading_at: Some(reading.timestamp),
      },
      eligible: None,
    }
  }

  /// A usable, fresh reading whose label another provider already claimed
  /// this minute. It did read successfully, so the last-success timestamp
  /// stands, but no row can be attributed to it.
  fn unclaimable(reading: EnvironmentalReading) -> Self {
    Self {
      status: EnvironmentalProviderStatus {
        source: reading.source,
        availability: AmbientSensorAvailability::Unavailable,
        last_reading_at: Some(reading.timestamp),
      },
      eligible: None,
    }
  }

  fn available(reading: EnvironmentalReading) -> Self {
    Self {
      status: EnvironmentalProviderStatus {
        // The normalized reading's label is the key the row is written
        // under, so the status must report that same key.
        source: reading.source.clone(),
        availability: AmbientSensorAvailability::Available,
        last_reading_at: Some(reading.timestamp),
      },
      eligible: Some(reading),
    }
  }
}

/// Whether a reading observed at `observed_at` still stands for the
/// minute ending at `now`.
///
/// Bounded on both sides. A reading stamped slightly ahead of `now` is
/// ordinary host clock jitter between the transport callback and the
/// archive tick, so a little skew is accepted; past
/// [`AMBIENT_READING_MAX_FUTURE_SKEW_SECONDS`] it is refused, because a
/// reading stamped far into the future would otherwise stay "fresh" for
/// as long as it led the clock.
fn is_fresh(observed_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
  let age = now.signed_duration_since(observed_at);
  age <= Duration::seconds(AMBIENT_READING_MAX_AGE_SECONDS)
    && age >= -Duration::seconds(AMBIENT_READING_MAX_FUTURE_SKEW_SECONDS)
}

/// Normalize one reading's shape, or `None` when it could never be
/// archived whatever its age.
///
/// Deliberately time-independent: freshness is a separate question, so
/// that a well-formed but old reading can be reported as stale while a
/// malformed one is reported as no reading at all.
fn normalize_reading(reading: EnvironmentalReading) -> Option<EnvironmentalReading> {
  let source = reading.source.trim();
  // An ambient row is only meaningful attributed to a source: the table
  // is row-per-source and every consumer selects by label.
  if source.is_empty() || !reading.temperature_celsius.is_finite() {
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
  }

  impl MockProvider {
    fn reporting(source: &str, reading: EnvironmentalReading) -> Arc<Self> {
      Arc::new(Self {
        source: source.to_string(),
        reading: Some(reading),
      })
    }

    fn silent(source: &str) -> Arc<Self> {
      Arc::new(Self {
        source: source.to_string(),
        reading: None,
      })
    }
  }

  impl EnvironmentalSensorProvider for MockProvider {
    fn source(&self) -> String {
      self.source.clone()
    }

    fn latest_reading(&self) -> Option<EnvironmentalReading> {
      self.reading.clone()
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
    assert!(registry.provider_statuses(now()).is_empty());
  }

  // -- staleness --

  #[test]
  fn a_reading_inside_the_freshness_window_represents_the_minute() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::reporting("Room", reading("Room", 90)));

    let fresh = registry.fresh_readings(now());
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].temperature_celsius, 24.5);
    assert_eq!(fresh[0].humidity_percent, Some(48.0));
  }

  #[test]
  fn a_reading_exactly_at_the_freshness_limit_is_still_accepted() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::reporting(
      "Room",
      reading("Room", AMBIENT_READING_MAX_AGE_SECONDS),
    ));

    assert_eq!(registry.fresh_readings(now()).len(), 1);
  }

  #[test]
  fn a_reading_past_the_freshness_limit_writes_no_row_rather_than_repeating() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::reporting(
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
    registry.register(MockProvider::reporting("Room", reading("Room", -2)));

    assert_eq!(registry.fresh_readings(now()).len(), 1);
  }

  #[test]
  fn a_reading_at_the_future_skew_limit_is_still_accepted() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::reporting(
      "Room",
      reading("Room", -AMBIENT_READING_MAX_FUTURE_SKEW_SECONDS),
    ));

    assert_eq!(registry.fresh_readings(now()).len(), 1);
  }

  #[test]
  fn a_reading_stamped_far_in_the_future_is_refused() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::reporting(
      "Room",
      reading("Room", -(AMBIENT_READING_MAX_FUTURE_SKEW_SECONDS + 1)),
    ));

    assert!(
      registry.fresh_readings(now()).is_empty(),
      "a clock rewind must not leave one reading fresh for as long as it leads the clock"
    );
    assert_eq!(
      registry.provider_statuses(now())[0].availability,
      AmbientSensorAvailability::Stale
    );
  }

  // -- normalization --

  #[test]
  fn a_non_finite_temperature_writes_no_row() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut broken = reading("Room", 0);
    broken.temperature_celsius = f32::NAN;
    registry.register(MockProvider::reporting("Room", broken));

    assert!(registry.fresh_readings(now()).is_empty());
  }

  #[test]
  fn a_non_finite_humidity_drops_only_the_humidity() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut partial = reading("Room", 0);
    partial.humidity_percent = Some(f32::INFINITY);
    registry.register(MockProvider::reporting("Room", partial));

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
    registry.register(MockProvider::reporting("Room", dry));

    let fresh = registry.fresh_readings(now());
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].humidity_percent, None);
  }

  #[test]
  fn an_unlabeled_reading_writes_no_row() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut anonymous = reading("Room", 0);
    anonymous.source = "   ".to_string();
    registry.register(MockProvider::reporting("Room", anonymous));

    assert!(registry.fresh_readings(now()).is_empty());
  }

  #[test]
  fn the_stored_source_label_is_trimmed() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut padded = reading("Room", 0);
    padded.source = "  Living Room  ".to_string();
    registry.register(MockProvider::reporting("Living Room", padded));

    assert_eq!(registry.fresh_readings(now())[0].source, "Living Room");
  }

  // -- one source key for the row and its status --

  /// A provider whose declared label disagrees with the label on the
  /// reading it hands over must not split into two keys: the archive row
  /// and the panel line have to describe the same source.
  #[test]
  fn the_archive_row_and_the_status_share_the_normalized_reading_label() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut relabeled = reading("Room", 30);
    relabeled.source = "  Living Room  ".to_string();
    registry.register(MockProvider::reporting("stale-provider-label", relabeled));

    let row_label = registry.fresh_readings(now())[0].source.clone();
    let status_label = registry.provider_statuses(now())[0].source.clone();

    assert_eq!(row_label, "Living Room");
    assert_eq!(status_label, row_label);
  }

  // -- multiple sources --

  #[test]
  fn each_source_contributes_its_own_row() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut desk = reading("Desk", 10);
    desk.temperature_celsius = 26.0;
    registry.register(MockProvider::reporting("Room", reading("Room", 10)));
    registry.register(MockProvider::reporting("Desk", desk));

    let fresh = registry.fresh_readings(now());
    assert_eq!(fresh.len(), 2);
    assert_eq!(fresh[0].source, "Room");
    assert_eq!(fresh[1].source, "Desk");
    assert_eq!(fresh[1].temperature_celsius, 26.0);
  }

  #[test]
  fn one_stale_source_does_not_suppress_a_fresh_one() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::reporting(
      "Room",
      reading("Room", AMBIENT_READING_MAX_AGE_SECONDS + 60),
    ));
    registry.register(MockProvider::reporting("Desk", reading("Desk", 5)));

    let fresh = registry.fresh_readings(now());
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].source, "Desk");
  }

  #[test]
  fn a_duplicated_source_label_contributes_only_one_row_per_minute() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::reporting("Room", reading("Room", 5)));
    let mut second = reading("Room", 5);
    second.temperature_celsius = 31.0;
    registry.register(MockProvider::reporting("Room", second));

    let fresh = registry.fresh_readings(now());
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].temperature_celsius, 24.5);
  }

  #[test]
  fn a_provider_whose_label_was_already_claimed_is_not_reported_available() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::reporting("Room", reading("Room", 5)));
    registry.register(MockProvider::reporting("Room", reading("Room", 5)));

    let statuses = registry.provider_statuses(now());
    assert_eq!(
      statuses[0].availability,
      AmbientSensorAvailability::Available
    );
    assert_eq!(
      statuses[1].availability,
      AmbientSensorAvailability::Unavailable,
      "no row is written for the second provider, so it must not look healthy"
    );
    // It did read successfully, so the panel can still say when.
    assert_eq!(
      statuses[1].last_reading_at,
      Some(now() - Duration::seconds(5))
    );
  }

  // -- provider status --

  #[test]
  fn a_source_with_readings_arriving_is_available_with_its_last_success() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::reporting("Room", reading("Room", 30)));

    assert_eq!(
      registry.provider_statuses(now()),
      vec![EnvironmentalProviderStatus {
        source: "Room".to_string(),
        availability: AmbientSensorAvailability::Available,
        last_reading_at: Some(now() - Duration::seconds(30)),
      }]
    );
  }

  #[test]
  fn a_source_that_has_never_reported_is_unavailable() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::silent("Desk"));

    assert_eq!(
      registry.provider_statuses(now()),
      vec![EnvironmentalProviderStatus {
        source: "Desk".to_string(),
        availability: AmbientSensorAvailability::Unavailable,
        last_reading_at: None,
      }]
    );
  }

  #[test]
  fn a_source_that_went_quiet_is_stale_and_still_reports_its_last_success() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::reporting(
      "Room",
      reading("Room", AMBIENT_READING_MAX_AGE_SECONDS * 10),
    ));

    // The archive stops writing rows, but the panel must still be able to
    // say how long ago the sensor last reported.
    assert!(registry.fresh_readings(now()).is_empty());
    assert_eq!(
      registry.provider_statuses(now()),
      vec![EnvironmentalProviderStatus {
        source: "Room".to_string(),
        availability: AmbientSensorAvailability::Stale,
        last_reading_at: Some(
          now() - Duration::seconds(AMBIENT_READING_MAX_AGE_SECONDS * 10)
        ),
      }]
    );
  }

  #[test]
  fn availability_uses_the_same_freshness_boundary_as_the_archive() {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(MockProvider::reporting(
      "Room",
      reading("Room", AMBIENT_READING_MAX_AGE_SECONDS),
    ));

    // The panel must never call a source available while the archive is
    // writing no rows for it, so both read the same boundary.
    assert_eq!(registry.fresh_readings(now()).len(), 1);
    assert_eq!(
      registry.provider_statuses(now())[0].availability,
      AmbientSensorAvailability::Available
    );
  }

  /// Regression: availability used to be derived from the raw reading's
  /// timestamp alone, so a reading the archive rejects on shape still
  /// reported as `Available` - "available but no row" by construction.
  #[test]
  fn an_unlabeled_but_fresh_reading_is_never_reported_available() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut anonymous = reading("Room", 5);
    anonymous.source = "   ".to_string();
    registry.register(MockProvider::reporting("Room", anonymous));

    assert!(registry.fresh_readings(now()).is_empty());
    assert_eq!(
      registry.provider_statuses(now()),
      vec![EnvironmentalProviderStatus {
        source: "Room".to_string(),
        availability: AmbientSensorAvailability::Unavailable,
        last_reading_at: None,
      }]
    );
  }

  #[test]
  fn a_fresh_reading_with_a_non_finite_temperature_is_never_reported_available() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut broken = reading("Room", 5);
    broken.temperature_celsius = f32::NAN;
    registry.register(MockProvider::reporting("Room", broken));

    assert!(registry.fresh_readings(now()).is_empty());
    let status = &registry.provider_statuses(now())[0];
    assert_eq!(status.availability, AmbientSensorAvailability::Unavailable);
    assert_eq!(
      status.last_reading_at, None,
      "a reading the archive rejected is not a success and must not be timestamped as one"
    );
  }

  /// The invariant the whole status contract exists to protect, asserted
  /// directly over a registry holding every rejection shape at once.
  #[test]
  fn available_is_reported_exactly_for_the_sources_that_get_a_row() {
    let mut registry = EnvironmentalSensorRegistry::new();
    let mut anonymous = reading("Anonymous", 5);
    anonymous.source = "  ".to_string();
    let mut broken = reading("Broken", 5);
    broken.temperature_celsius = f32::NEG_INFINITY;

    registry.register(MockProvider::reporting(
      "Living Room",
      reading("Living Room", 5),
    ));
    registry.register(MockProvider::silent("Never"));
    registry.register(MockProvider::reporting("Anonymous", anonymous));
    registry.register(MockProvider::reporting("Broken", broken));
    registry.register(MockProvider::reporting(
      "Quiet",
      reading("Quiet", AMBIENT_READING_MAX_AGE_SECONDS + 1),
    ));
    registry.register(MockProvider::reporting("Desk", reading("Desk", 0)));

    let archived: Vec<String> = registry
      .fresh_readings(now())
      .into_iter()
      .map(|reading| reading.source)
      .collect();
    let available: Vec<String> = registry
      .provider_statuses(now())
      .into_iter()
      .filter(|status| status.availability == AmbientSensorAvailability::Available)
      .map(|status| status.source)
      .collect();

    assert_eq!(
      archived,
      vec!["Living Room".to_string(), "Desk".to_string()]
    );
    assert_eq!(available, archived);
  }
}
