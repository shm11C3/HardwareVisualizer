//! The [`EnvironmentalSensorProvider`] a SwitchBot meter is read through
//! (#2044).
//!
//! Deliberately transport-free: the radio layer decodes an
//! advertisement and pushes the result in, and the archive tick pulls
//! the newest one back out. Nothing here knows what BLE is, so the whole
//! caching and device-selection behavior is exercised on every platform
//! without an adapter.
//!
//! Why a cache at all: the archive tick calls
//! [`EnvironmentalSensorProvider::latest_reading`] inline and must not
//! block on I/O, and a meter broadcasts when it chooses rather than when
//! it is asked. Push in, poll out is the shape #2043 was built for.

use std::collections::HashSet;
use std::sync::{PoisonError, RwLock};

use chrono::{DateTime, Utc};

use crate::infrastructure::providers::environmental::{
  EnvironmentalReading, EnvironmentalSensorProvider,
};

use super::advertisement::SwitchBotMeterFrame;

/// Sensor Source Label every reading from this provider is archived
/// under.
///
/// A fixed label rather than a per-device one: the archive is
/// row-per-source and Thermal Delta history is only comparable across
/// days if the label stays put, so it must not change when a meter's
/// Bluetooth address does (a battery swap, a re-pair, a firmware
/// update).
pub const SWITCHBOT_METER_SOURCE_LABEL: &str = "SwitchBot Meter";

/// What the provider did with an observed frame.
///
/// Returned rather than logged here so the caller can report a
/// state change once instead of on every advertisement - a meter
/// broadcasts every few seconds, and a log line per broadcast would
/// bury everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationOutcome {
  /// The first frame from the device this provider is now bound to.
  Bound,
  /// A further reading from the bound device.
  Recorded,
  /// A different meter was in range. Its reading was discarded, and
  /// this is the first time that particular device has been discarded,
  /// so it is worth saying once.
  IgnoredNewDevice,
  /// A different meter that has already been reported as ignored.
  IgnoredKnownDevice,
}

/// A single SwitchBot meter, cached from whatever the radio last heard.
///
/// # One meter, not many
///
/// The provider binds to the first meter it hears and ignores every
/// other one. The alternative - letting any meter in range write under
/// this label - would interleave two rooms' temperatures into one
/// series, and every Thermal Delta computed from it would be quietly
/// wrong with no way to tell from the data. Refusing the second meter
/// keeps the label meaning one physical sensor.
///
/// Choosing *which* meter when several are in range is a real product
/// question (naming devices, per-device labels), and #2044 does not
/// answer it: it ships one ambient source. Until it is answered, the
/// honest behavior is one sensor plus a log line naming the ones that
/// were skipped.
pub struct SwitchBotMeterProvider {
  source: String,
  observed: RwLock<ObservedState>,
}

#[derive(Default)]
struct ObservedState {
  /// Transport identity of the meter this provider answers for, set by
  /// the first frame and never reassigned for the process's lifetime.
  bound_device: Option<String>,
  latest: Option<EnvironmentalReading>,
  /// Devices already reported as skipped. Bounded by the number of
  /// distinct SwitchBot meters within radio range, so it cannot grow
  /// with time or traffic the way a per-advertisement record would.
  reported_other_devices: HashSet<String>,
}

impl SwitchBotMeterProvider {
  pub fn new() -> Self {
    Self {
      source: SWITCHBOT_METER_SOURCE_LABEL.to_string(),
      observed: RwLock::new(ObservedState::default()),
    }
  }

  /// Take one decoded advertisement observed from `device_id`.
  ///
  /// `observed_at` is the host's time of reception, not a device
  /// timestamp: the meter does not stamp its broadcasts, and #2043's
  /// freshness window compares against the archive tick on this same
  /// host, so the moment of reception is the only stamp that means
  /// anything here.
  pub fn observe(
    &self,
    device_id: &str,
    frame: SwitchBotMeterFrame,
    observed_at: DateTime<Utc>,
  ) -> ObservationOutcome {
    // A panicking writer must not take the ambient source down with it:
    // the archive tick reads this lock every minute, and a poisoned
    // lock would turn one bad frame into a permanently dead sensor.
    let mut observed = self
      .observed
      .write()
      .unwrap_or_else(PoisonError::into_inner);

    let outcome = match observed.bound_device.as_deref() {
      None => {
        observed.bound_device = Some(device_id.to_string());
        ObservationOutcome::Bound
      }
      Some(bound) if bound == device_id => ObservationOutcome::Recorded,
      Some(_) => {
        return if observed.reported_other_devices.insert(device_id.to_string()) {
          ObservationOutcome::IgnoredNewDevice
        } else {
          ObservationOutcome::IgnoredKnownDevice
        };
      }
    };

    observed.latest = Some(EnvironmentalReading {
      temperature_celsius: frame.temperature_celsius,
      humidity_percent: frame.humidity_percent,
      timestamp: observed_at,
      source: self.source.clone(),
    });

    outcome
  }
}

impl Default for SwitchBotMeterProvider {
  fn default() -> Self {
    Self::new()
  }
}

impl EnvironmentalSensorProvider for SwitchBotMeterProvider {
  fn source(&self) -> &str {
    &self.source
  }

  fn latest_reading(&self) -> Option<EnvironmentalReading> {
    self
      .observed
      .read()
      .unwrap_or_else(PoisonError::into_inner)
      .latest
      .clone()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use super::super::advertisement::SwitchBotMeterModel;
  use chrono::Duration;

  fn at(offset_seconds: i64) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-30T12:00:00Z")
      .unwrap()
      .with_timezone(&Utc)
      + Duration::seconds(offset_seconds)
  }

  fn frame(temperature_celsius: f32) -> SwitchBotMeterFrame {
    SwitchBotMeterFrame {
      model: SwitchBotMeterModel::Meter,
      temperature_celsius,
      humidity_percent: Some(48.0),
    }
  }

  // -- before anything has been heard --

  /// The state the app spends its first seconds in, and stays in
  /// forever when no meter is present. #2043 turns this into
  /// `Unavailable`, so "no adapter", "no meter", and "meter out of
  /// range" all end up as the same honest answer without this provider
  /// having to describe any of them.
  #[test]
  fn a_provider_that_has_heard_nothing_reports_no_reading() {
    let provider = SwitchBotMeterProvider::new();
    assert_eq!(provider.latest_reading(), None);
  }

  #[test]
  fn the_provider_declares_the_switchbot_source_label() {
    assert_eq!(
      SwitchBotMeterProvider::new().source(),
      SWITCHBOT_METER_SOURCE_LABEL
    );
  }

  // -- caching one meter --

  #[test]
  fn the_first_frame_binds_the_provider_and_becomes_the_reading() {
    let provider = SwitchBotMeterProvider::new();

    assert_eq!(
      provider.observe("meter-a", frame(24.5), at(0)),
      ObservationOutcome::Bound
    );

    assert_eq!(
      provider.latest_reading(),
      Some(EnvironmentalReading {
        temperature_celsius: 24.5,
        humidity_percent: Some(48.0),
        timestamp: at(0),
        source: SWITCHBOT_METER_SOURCE_LABEL.to_string(),
      })
    );
  }

  #[test]
  fn a_later_frame_from_the_bound_meter_replaces_the_cached_reading() {
    let provider = SwitchBotMeterProvider::new();
    provider.observe("meter-a", frame(24.5), at(0));

    assert_eq!(
      provider.observe("meter-a", frame(25.1), at(30)),
      ObservationOutcome::Recorded
    );

    let latest = provider.latest_reading().unwrap();
    assert_eq!(latest.temperature_celsius, 25.1);
    assert_eq!(latest.timestamp, at(30));
  }

  /// The reading carries the host's reception time, which is what
  /// #2043's freshness window is measured against.
  #[test]
  fn the_reading_is_stamped_with_the_time_it_was_received() {
    let provider = SwitchBotMeterProvider::new();
    provider.observe("meter-a", frame(24.5), at(-120));

    assert_eq!(provider.latest_reading().unwrap().timestamp, at(-120));
  }

  #[test]
  fn a_temperature_only_frame_caches_without_humidity() {
    let provider = SwitchBotMeterProvider::new();
    let mut dry = frame(24.5);
    dry.humidity_percent = None;
    provider.observe("meter-a", dry, at(0));

    assert_eq!(provider.latest_reading().unwrap().humidity_percent, None);
  }

  /// The provider never expires its own cache. Deciding that a reading
  /// is too old to represent a minute is the registry's rule (#2043),
  /// and duplicating it here would create a second freshness boundary
  /// that could drift from the one the archive actually applies.
  #[test]
  fn an_old_reading_is_still_returned_and_left_for_the_registry_to_judge() {
    let provider = SwitchBotMeterProvider::new();
    provider.observe("meter-a", frame(24.5), at(-86_400));

    let latest = provider
      .latest_reading()
      .expect("the provider reports what it heard; freshness is not its call");
    assert_eq!(latest.timestamp, at(-86_400));
  }

  // -- more than one meter in range --

  #[test]
  fn a_second_meter_does_not_overwrite_the_bound_meters_reading() {
    let provider = SwitchBotMeterProvider::new();
    provider.observe("meter-a", frame(24.5), at(0));

    assert_eq!(
      provider.observe("meter-b", frame(31.0), at(10)),
      ObservationOutcome::IgnoredNewDevice
    );

    let latest = provider.latest_reading().unwrap();
    assert_eq!(
      latest.temperature_celsius, 24.5,
      "one label must mean one physical sensor, or every Thermal Delta from it is a blend of two rooms"
    );
    assert_eq!(latest.timestamp, at(0));
  }

  #[test]
  fn the_bound_meter_keeps_updating_while_another_is_in_range() {
    let provider = SwitchBotMeterProvider::new();
    provider.observe("meter-a", frame(24.5), at(0));
    provider.observe("meter-b", frame(31.0), at(10));

    provider.observe("meter-a", frame(24.9), at(20));

    let latest = provider.latest_reading().unwrap();
    assert_eq!(latest.temperature_celsius, 24.9);
    assert_eq!(latest.timestamp, at(20));
  }

  /// A skipped meter broadcasts as often as the bound one, so it is
  /// worth naming once and then staying quiet.
  #[test]
  fn a_skipped_meter_is_reported_once_however_often_it_broadcasts() {
    let provider = SwitchBotMeterProvider::new();
    provider.observe("meter-a", frame(24.5), at(0));

    assert_eq!(
      provider.observe("meter-b", frame(31.0), at(10)),
      ObservationOutcome::IgnoredNewDevice
    );
    for second in 1..5 {
      assert_eq!(
        provider.observe("meter-b", frame(31.0), at(10 + second)),
        ObservationOutcome::IgnoredKnownDevice
      );
    }
  }

  #[test]
  fn each_distinct_skipped_meter_is_reported_once() {
    let provider = SwitchBotMeterProvider::new();
    provider.observe("meter-a", frame(24.5), at(0));

    assert_eq!(
      provider.observe("meter-b", frame(31.0), at(10)),
      ObservationOutcome::IgnoredNewDevice
    );
    assert_eq!(
      provider.observe("meter-c", frame(19.0), at(11)),
      ObservationOutcome::IgnoredNewDevice
    );
    assert_eq!(
      provider.observe("meter-b", frame(31.0), at(12)),
      ObservationOutcome::IgnoredKnownDevice
    );
  }

  // -- the provider inside the registry it was built for --

  /// The contract this provider exists to satisfy, asserted through
  /// #2043's registry rather than against this type's own accessors.
  #[test]
  fn a_bound_meter_becomes_an_available_ambient_source_in_the_registry() {
    use crate::infrastructure::providers::environmental::{
      AmbientSensorAvailability, EnvironmentalSensorRegistry,
    };
    use std::sync::Arc;

    let provider = Arc::new(SwitchBotMeterProvider::new());
    provider.observe("meter-a", frame(24.5), at(-30));

    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(Arc::clone(&provider) as Arc<_>);

    let readings = registry.fresh_readings(at(0));
    assert_eq!(readings.len(), 1);
    assert_eq!(readings[0].source, SWITCHBOT_METER_SOURCE_LABEL);
    assert_eq!(readings[0].temperature_celsius, 24.5);

    assert_eq!(
      registry.provider_statuses(at(0))[0].availability,
      AmbientSensorAvailability::Available
    );
  }

  /// A meter that stops broadcasting - out of range, flat battery,
  /// adapter switched off - stops producing rows without this provider
  /// ever describing a radio state, which is the #2043 availability
  /// contract.
  #[test]
  fn a_meter_that_goes_quiet_becomes_stale_without_a_transport_concept() {
    use crate::infrastructure::providers::environmental::{
      AMBIENT_READING_MAX_AGE_SECONDS, AmbientSensorAvailability,
      EnvironmentalSensorRegistry,
    };
    use std::sync::Arc;

    let provider = Arc::new(SwitchBotMeterProvider::new());
    provider.observe("meter-a", frame(24.5), at(-AMBIENT_READING_MAX_AGE_SECONDS - 1));

    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(provider as Arc<_>);

    assert!(registry.fresh_readings(at(0)).is_empty());
    assert_eq!(
      registry.provider_statuses(at(0))[0].availability,
      AmbientSensorAvailability::Stale
    );
  }

  /// No adapter, no meter, and a meter out of range are indistinguishable
  /// from outside - exactly as #2043 requires.
  #[test]
  fn a_provider_that_never_heard_a_meter_is_unavailable_in_the_registry() {
    use crate::infrastructure::providers::environmental::{
      AmbientSensorAvailability, EnvironmentalSensorRegistry,
    };
    use std::sync::Arc;

    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(Arc::new(SwitchBotMeterProvider::new()) as Arc<_>);

    assert!(registry.fresh_readings(at(0)).is_empty());
    let status = &registry.provider_statuses(at(0))[0];
    assert_eq!(status.availability, AmbientSensorAvailability::Unavailable);
    assert_eq!(status.last_reading_at, None);
    assert_eq!(status.source, SWITCHBOT_METER_SOURCE_LABEL);
  }
}
