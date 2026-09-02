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

use std::collections::{HashMap, HashSet};
use std::sync::{PoisonError, RwLock};

use chrono::{DateTime, Duration, Utc};

use crate::infrastructure::providers::environmental::{
  AMBIENT_READING_MAX_AGE_SECONDS, EnvironmentalReading, EnvironmentalSensorProvider,
};

use super::advertisement::SwitchBotMeterFrame;

/// Sensor Source Label used while no device has been chosen.
///
/// Once a device is chosen the label carries its short handle - see
/// [`source_label`]. This bare form is what an unbound provider's status
/// line shows; no reading is ever archived under it.
pub const SWITCHBOT_METER_SOURCE_LABEL: &str = "SwitchBot Meter";

/// How many trailing characters of a device handle appear in the label.
const DEVICE_HANDLE_LEN: usize = 4;

/// The Sensor Source Label for readings from `device_id`.
///
/// The device handle is part of the label on purpose. The archive is
/// row-per-source, so if this machine ever ends up bound to a different
/// meter, that meter's readings must land under a different label -
/// otherwise two rooms' temperatures share one series and every Thermal
/// Delta computed from it is a silent blend of both. Putting the
/// identity in the key makes that mixing structurally impossible instead
/// of merely unlikely: a re-bind starts a visibly new source rather than
/// continuing an old one under false pretenses.
///
/// Only a short tail of the handle is used. It is enough to tell one
/// meter from another and to show the user which device they are bound
/// to, without writing a full Bluetooth address into an archive that
/// outlives the reason for having it.
pub fn source_label(device_id: Option<&str>) -> String {
  match device_id {
    None => SWITCHBOT_METER_SOURCE_LABEL.to_string(),
    Some(device_id) => {
      format!(
        "{SWITCHBOT_METER_SOURCE_LABEL} ({})",
        device_handle(device_id)
      )
    }
  }
}

/// A short, display-safe handle for one device: the trailing
/// alphanumeric characters of its id, which for a Bluetooth address are
/// its last two bytes.
fn device_handle(device_id: &str) -> String {
  let compact: String = device_id
    .chars()
    .filter(|c| c.is_ascii_alphanumeric())
    .collect();

  let start = compact.len().saturating_sub(DEVICE_HANDLE_LEN);
  compact[start..].to_ascii_lowercase()
}

/// What the provider did with an observed frame.
///
/// Returned rather than logged here so the caller can report a
/// state change once instead of on every advertisement - a meter
/// broadcasts every few seconds, and a log line per broadcast would
/// bury everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationOutcome {
  /// A reading from the chosen device.
  Recorded,
  /// A different meter was in range. Its reading was discarded, and
  /// this is the first time that particular device has been discarded,
  /// so it is worth saying once.
  IgnoredNewDevice,
  /// A different meter that has already been reported as ignored.
  IgnoredKnownDevice,
}

/// A single SwitchBot device, cached from whatever the radio last heard.
///
/// # One device, chosen by the user
///
/// The provider answers for exactly one device. Letting any meter in
/// range write under one label would interleave two rooms' temperatures
/// into one series, and every Thermal Delta computed from it would be
/// quietly wrong with no way to tell from the data.
///
/// Which device that is comes from the user, never from luck. The stored
/// choice is passed to [`Self::bound`] at startup and changed through
/// [`Self::rebind`] when the user picks another; until a choice exists
/// ([`Self::unbound`]) every frame is recorded as a candidate and none is
/// read. Several devices in one room were seen reading degrees apart, so
/// adopting whichever advertised first would have picked the number
/// every Thermal Delta is measured against by chance, differently on
/// each launch.
///
/// A chosen device that is out of range yields no readings, and the
/// registry reports the source unavailable. It deliberately does *not*
/// fall back to another meter: silently substituting a different room's
/// sensor is the failure this design exists to prevent, and "no reading"
/// is the honest answer.
pub struct SwitchBotMeterProvider {
  observed: RwLock<ObservedState>,
}

#[derive(Default)]
struct ObservedState {
  /// Address of the device this provider answers for, or `None` until
  /// the user has chosen one. Set from the stored choice at construction
  /// and replaced by [`SwitchBotMeterProvider::rebind`] when the choice
  /// changes; never set by a frame.
  bound_device: Option<String>,
  /// Label readings are written under, and the label reported before
  /// any reading exists. Follows `bound_device`, so a provider that
  /// knows its device says so even while the device is silent.
  active_source: String,
  latest: Option<EnvironmentalReading>,
  /// Devices already reported as skipped. Bounded by the number of
  /// distinct SwitchBot meters within radio range, so it cannot grow
  /// with time or traffic the way a per-advertisement record would.
  reported_other_devices: HashSet<String>,
  /// Every device heard this session, whether chosen or not.
  ///
  /// Kept so the user can be shown what is actually in the room and pick
  /// from it. A capture in one room found four SwitchBot devices reading
  /// between 25.2 °C and 27.3 °C - a spread wider than the rise Cooling
  /// Insight treats as a sustained observation - so which one is used
  /// changes the analysis, and an arbitrary pick is not good enough.
  ///
  /// Bounded by the number of SwitchBot devices in radio range.
  discovered: HashMap<String, DiscoveredSensor>,
}

/// One SwitchBot device heard during this session.
///
/// Carries the reading rather than a model name: model identity cannot
/// be trusted from these broadcasts (see `advertisement::reading_offset`),
/// and the temperature is the more useful thing to choose by anyway -
/// it is what tells the user which device sits near the intake.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredSensor {
  /// Full address form, the value persisted as the chosen device.
  pub device_id: String,
  pub temperature_celsius: f32,
  pub humidity_percent: Option<f32>,
  pub last_seen: DateTime<Utc>,
}

impl SwitchBotMeterProvider {
  /// A provider bound to a remembered device, ignoring every other one.
  pub fn bound(device_id: impl Into<String>) -> Self {
    let device_id = device_id.into();
    let source = source_label(Some(&device_id));

    Self {
      observed: RwLock::new(ObservedState {
        bound_device: Some(device_id),
        active_source: source,
        ..ObservedState::default()
      }),
    }
  }

  /// A provider with no chosen device. It records every device it hears
  /// as a candidate for [`Self::discovered_sensors`] and reads none of
  /// them until [`Self::rebind`] names one.
  pub fn unbound() -> Self {
    Self {
      observed: RwLock::new(ObservedState {
        active_source: SWITCHBOT_METER_SOURCE_LABEL.to_string(),
        ..ObservedState::default()
      }),
    }
  }

  /// Build from an optional remembered binding.
  pub fn new(bound_device: Option<String>) -> Self {
    match bound_device {
      Some(device_id) => Self::bound(device_id),
      None => Self::unbound(),
    }
  }

  /// Every SwitchBot device heard within the freshness window ending at
  /// `now`, in device-id order, so the user can choose one.
  ///
  /// This is a live view of the room, not a stored list. Each entry
  /// shows its reading as current, so a device that has gone quiet -
  /// carried off, battery flat - drops out rather than sitting beside
  /// the live ones with a temperature from an hour ago. The window is
  /// the archive's own ([`AMBIENT_READING_MAX_AGE_SECONDS`]): "current"
  /// means the same thing on the settings screen as it does for a row.
  pub fn discovered_sensors(&self, now: DateTime<Utc>) -> Vec<DiscoveredSensor> {
    let observed = self.observed.read().unwrap_or_else(PoisonError::into_inner);

    let mut sensors: Vec<DiscoveredSensor> = observed
      .discovered
      .values()
      .filter(|sensor| {
        now - sensor.last_seen <= Duration::seconds(AMBIENT_READING_MAX_AGE_SECONDS)
      })
      .cloned()
      .collect();
    // Ordered by identity, not by when each was last heard. Every device
    // in range broadcasts every few seconds, so ordering by recency
    // reshuffles the list continuously - under the cursor of someone
    // trying to click one of them.
    sensors.sort_by(|a, b| a.device_id.cmp(&b.device_id));
    sensors
  }

  /// The device this provider answers for, if one has been chosen.
  pub fn bound_device(&self) -> Option<String> {
    self
      .observed
      .read()
      .unwrap_or_else(PoisonError::into_inner)
      .bound_device
      .clone()
  }

  /// Answer for `device_id` from now on, or for nothing when `None`.
  ///
  /// The choice is made on the settings screen while the scan is already
  /// running, so it takes effect here rather than on the next launch.
  /// The cached reading is dropped with the old device: left in place,
  /// the next archive tick would write the previous device's reading
  /// under the new device's label, and the label exists precisely so
  /// that two sensors can never share one series.
  pub fn rebind(&self, device_id: Option<String>) {
    let mut observed = self
      .observed
      .write()
      .unwrap_or_else(PoisonError::into_inner);

    observed.active_source = source_label(device_id.as_deref());
    observed.bound_device = device_id;
    observed.latest = None;
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
    self.observe_reading(
      device_id,
      frame.temperature_celsius,
      frame.humidity_percent,
      observed_at,
    )
  }

  /// Record a reading that arrived without a model attached.
  ///
  /// The Hub family broadcasts in manufacturer data, which carries no
  /// device type worth trusting (see `advertisement::reading_offset`),
  /// so those readings reach the provider as bare values. Binding and
  /// source-label rules are identical either way - only the decode
  /// differs - so both paths share this one implementation rather than
  /// growing a second copy that could drift.
  pub fn observe_reading(
    &self,
    device_id: &str,
    temperature_celsius: f32,
    humidity_percent: Option<f32>,
    observed_at: DateTime<Utc>,
  ) -> ObservationOutcome {
    // A panicking writer must not take the ambient source down with it:
    // the archive tick reads this lock every minute, and a poisoned
    // lock would turn one bad frame into a permanently dead sensor.
    let mut observed = self
      .observed
      .write()
      .unwrap_or_else(PoisonError::into_inner);

    // Everything heard is offered to the user, chosen or not.
    observed.discovered.insert(
      device_id.to_string(),
      DiscoveredSensor {
        device_id: device_id.to_string(),
        temperature_celsius,
        humidity_percent,
        last_seen: observed_at,
      },
    );

    let outcome = match observed.bound_device.as_deref() {
      // Nothing is used until the user picks a device. Latching to
      // whichever sensor happened to advertise first was arbitrary in
      // exactly the case that matters - several sensors in one room,
      // reading degrees apart - and produced a different answer on every
      // launch. Reporting no ambient source until the choice is made is
      // the honest state.
      None => {
        return if observed
          .reported_other_devices
          .insert(device_id.to_string())
        {
          ObservationOutcome::IgnoredNewDevice
        } else {
          ObservationOutcome::IgnoredKnownDevice
        };
      }
      Some(bound) if bound == device_id => ObservationOutcome::Recorded,
      Some(_) => {
        return if observed
          .reported_other_devices
          .insert(device_id.to_string())
        {
          ObservationOutcome::IgnoredNewDevice
        } else {
          ObservationOutcome::IgnoredKnownDevice
        };
      }
    };

    observed.latest = Some(EnvironmentalReading {
      temperature_celsius,
      humidity_percent,
      timestamp: observed_at,
      source: observed.active_source.clone(),
    });

    outcome
  }
}

impl Default for SwitchBotMeterProvider {
  fn default() -> Self {
    Self::unbound()
  }
}

impl EnvironmentalSensorProvider for SwitchBotMeterProvider {
  fn source(&self) -> String {
    self
      .observed
      .read()
      .unwrap_or_else(PoisonError::into_inner)
      .active_source
      .clone()
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
  use super::super::advertisement::SwitchBotMeterModel;
  use super::*;
  use crate::infrastructure::providers::environmental::{
    AMBIENT_READING_MAX_AGE_SECONDS, AmbientSensorAvailability,
    EnvironmentalSensorRegistry,
  };
  use chrono::Duration;
  use std::sync::Arc;

  /// Two meters, identified the way the scan hands them in: the Bluetooth
  /// address as twelve lowercase hex digits.
  const METER_A: &str = "aabbccdda1b2";
  const METER_B: &str = "aabbccddc3d4";
  const LABEL_A: &str = "SwitchBot Meter (a1b2)";
  const LABEL_B: &str = "SwitchBot Meter (c3d4)";

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

  fn registry_with(provider: Arc<SwitchBotMeterProvider>) -> EnvironmentalSensorRegistry {
    let mut registry = EnvironmentalSensorRegistry::new();
    registry.register(provider as Arc<_>);
    registry
  }

  // -- source labels --

  #[test]
  fn an_unidentified_source_uses_the_bare_label() {
    assert_eq!(source_label(None), "SwitchBot Meter");
  }

  /// The label carries the device, so two meters can never share an
  /// archive key.
  #[test]
  fn a_known_device_is_named_in_its_source_label() {
    assert_eq!(source_label(Some(METER_A)), LABEL_A);
    assert_eq!(source_label(Some(METER_B)), LABEL_B);
    assert_ne!(source_label(Some(METER_A)), source_label(Some(METER_B)));
  }

  #[test]
  fn a_device_label_is_stable_for_the_same_device() {
    assert_eq!(source_label(Some(METER_A)), source_label(Some(METER_A)));
  }

  #[test]
  fn a_short_device_identifier_still_produces_a_label() {
    assert_eq!(source_label(Some("ab")), "SwitchBot Meter (ab)");
  }

  // -- before anything has been heard --

  /// The state the app spends its first seconds in, and stays in
  /// forever when no meter is present. #2043 turns this into
  /// `Unavailable`, so "no adapter", "no meter", and "meter out of
  /// range" all end up as the same honest answer without this provider
  /// having to describe any of them.
  #[test]
  fn a_provider_that_has_heard_nothing_reports_no_reading() {
    assert_eq!(SwitchBotMeterProvider::unbound().latest_reading(), None);
  }

  #[test]
  fn an_unbound_provider_declares_the_bare_source_label() {
    assert_eq!(
      SwitchBotMeterProvider::unbound().source(),
      SWITCHBOT_METER_SOURCE_LABEL
    );
  }

  /// A remembered binding is visible before the meter has said anything,
  /// so the status line names the device the app is waiting for rather
  /// than looking unconfigured.
  #[test]
  fn a_bound_provider_declares_its_remembered_device_before_any_reading() {
    let provider = SwitchBotMeterProvider::bound(METER_A);
    assert_eq!(provider.source(), LABEL_A);
    assert_eq!(provider.latest_reading(), None);
  }

  // -- nothing is read until a device is chosen --

  /// The rule that replaced latching. A capture in one room found four
  /// SwitchBot devices reading degrees apart, so adopting whichever
  /// advertised first picked the number every Thermal Delta is measured
  /// against by luck, differently on each launch.
  #[test]
  fn an_unchosen_provider_reads_nothing_however_many_meters_are_in_range() {
    let provider = SwitchBotMeterProvider::unbound();

    assert_eq!(
      provider.observe(METER_A, frame(24.5), at(0)),
      ObservationOutcome::IgnoredNewDevice
    );
    assert_eq!(
      provider.observe(METER_B, frame(31.0), at(1)),
      ObservationOutcome::IgnoredNewDevice
    );

    assert_eq!(provider.latest_reading(), None);
  }

  /// What the settings screen offers: everything heard, chosen or not.
  #[test]
  fn every_meter_heard_is_offered_for_selection() {
    let provider = SwitchBotMeterProvider::unbound();
    provider.observe(METER_A, frame(24.5), at(0));
    provider.observe(METER_B, frame(31.0), at(1));

    let discovered = provider.discovered_sensors(at(5));

    assert_eq!(discovered.len(), 2);
    assert_eq!(discovered[0].device_id, METER_A);
    assert_eq!(discovered[1].device_id, METER_B);
    assert_eq!(discovered[1].temperature_celsius, 31.0);
  }

  /// Carrying the machine to another room must cost the reading, not
  /// swap it for a different room's sensor.
  ///
  /// The chosen device simply stops being heard; the others keep
  /// broadcasting and keep being refused. #2043's freshness window then
  /// ages the cached reading out, so those minutes get no ambient row
  /// and no Thermal Delta - which is the honest answer, where a
  /// substituted sensor would be a confident wrong one.
  #[test]
  fn a_silent_chosen_meter_is_never_replaced_by_another_in_range() {
    let provider = SwitchBotMeterProvider::bound(METER_A);
    provider.observe(METER_A, frame(24.5), at(0));

    // The chosen device goes quiet; another one is still shouting.
    for tick in 1..5 {
      provider.observe(METER_B, frame(31.0), at(tick));
    }

    let latest = provider.latest_reading().unwrap();
    assert_eq!(
      latest.temperature_celsius, 24.5,
      "readings must keep coming from the chosen device only"
    );
    assert_eq!(latest.source, LABEL_A);
    // Stamped when the chosen device was last heard, so the freshness
    // window can age it out rather than it looking current forever.
    assert_eq!(latest.timestamp, at(0));
  }

  /// The list is polled while the settings screen is open, and every
  /// device in range re-broadcasts between polls. Ordering must not
  /// depend on that, or the row a user is reaching for moves.
  #[test]
  fn the_offered_order_does_not_change_when_a_device_is_heard_again() {
    let provider = SwitchBotMeterProvider::unbound();
    provider.observe(METER_A, frame(24.5), at(0));
    provider.observe(METER_B, frame(31.0), at(1));

    let before: Vec<String> = provider
      .discovered_sensors(at(5))
      .into_iter()
      .map(|sensor| sensor.device_id)
      .collect();

    // METER_A speaks again, which under a recency order would move it.
    provider.observe(METER_A, frame(24.6), at(2));

    let after: Vec<String> = provider
      .discovered_sensors(at(5))
      .into_iter()
      .map(|sensor| sensor.device_id)
      .collect();

    assert_eq!(before, after);
  }

  /// The list shows each device's reading as current, so a device that
  /// has gone quiet - carried off, battery flat - must leave the list
  /// rather than sit there with a temperature from an hour ago beside
  /// the live ones. The same freshness window the archive applies to a
  /// reading decides when "current" stops being true.
  #[test]
  fn a_device_not_heard_within_the_freshness_window_is_no_longer_offered() {
    let provider = SwitchBotMeterProvider::unbound();
    provider.observe(METER_A, frame(24.5), at(0));
    provider.observe(METER_B, frame(31.0), at(AMBIENT_READING_MAX_AGE_SECONDS));

    let offered: Vec<String> = provider
      .discovered_sensors(at(AMBIENT_READING_MAX_AGE_SECONDS + 1))
      .into_iter()
      .map(|sensor| sensor.device_id)
      .collect();

    assert_eq!(offered, vec![METER_B.to_string()]);
  }

  /// The boundary is the archive's: a reading exactly as old as the
  /// window is still fresh there, so the device is still offered here.
  #[test]
  fn a_device_heard_exactly_a_window_ago_is_still_offered() {
    let provider = SwitchBotMeterProvider::unbound();
    provider.observe(METER_A, frame(24.5), at(0));

    let offered = provider.discovered_sensors(at(AMBIENT_READING_MAX_AGE_SECONDS));

    assert_eq!(offered.len(), 1);
  }

  /// A quiet device comes back the moment it is heard again.
  #[test]
  fn a_device_heard_again_is_offered_again() {
    let provider = SwitchBotMeterProvider::unbound();
    provider.observe(METER_A, frame(24.5), at(0));
    assert!(
      provider
        .discovered_sensors(at(AMBIENT_READING_MAX_AGE_SECONDS * 2))
        .is_empty()
    );

    provider.observe(
      METER_A,
      frame(24.7),
      at(AMBIENT_READING_MAX_AGE_SECONDS * 2),
    );

    let offered = provider.discovered_sensors(at(AMBIENT_READING_MAX_AGE_SECONDS * 2));
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0].temperature_celsius, 24.7);
  }

  /// A device that was never chosen still appears in the list, which is
  /// what lets a user switch to it.
  #[test]
  fn a_chosen_provider_still_offers_the_meters_it_is_not_reading() {
    let provider = SwitchBotMeterProvider::bound(METER_A);
    provider.observe(METER_A, frame(24.5), at(0));
    provider.observe(METER_B, frame(31.0), at(1));

    let offered: Vec<String> = provider
      .discovered_sensors(at(5))
      .into_iter()
      .map(|sensor| sensor.device_id)
      .collect();

    assert!(
      offered.iter().any(|id| id == METER_A) && offered.iter().any(|id| id == METER_B)
    );
    assert_eq!(provider.bound_device().as_deref(), Some(METER_A));
  }

  #[test]
  fn a_frame_from_the_chosen_meter_becomes_the_reading() {
    let provider = SwitchBotMeterProvider::bound(METER_A);

    assert_eq!(
      provider.observe(METER_A, frame(24.5), at(0)),
      ObservationOutcome::Recorded
    );

    assert_eq!(
      provider.latest_reading(),
      Some(EnvironmentalReading {
        temperature_celsius: 24.5,
        humidity_percent: Some(48.0),
        timestamp: at(0),
        source: LABEL_A.to_string(),
      })
    );
  }

  /// The reading must be labelled with the device it came from: that
  /// label is the archive key, so a different choice starts a different
  /// series rather than continuing one.
  #[test]
  fn a_reading_is_labelled_with_the_device_it_came_from() {
    let provider = SwitchBotMeterProvider::bound(METER_B);
    provider.observe(METER_B, frame(24.5), at(0));

    assert_eq!(provider.latest_reading().unwrap().source, LABEL_B);
  }

  #[test]
  fn a_later_frame_from_the_bound_meter_replaces_the_cached_reading() {
    let provider = SwitchBotMeterProvider::bound(METER_A);
    provider.observe(METER_A, frame(24.5), at(0));

    assert_eq!(
      provider.observe(METER_A, frame(25.1), at(30)),
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
    let provider = SwitchBotMeterProvider::bound(METER_A);
    provider.observe(METER_A, frame(24.5), at(-120));

    assert_eq!(provider.latest_reading().unwrap().timestamp, at(-120));
  }

  #[test]
  fn a_temperature_only_frame_caches_without_humidity() {
    let provider = SwitchBotMeterProvider::bound(METER_A);
    let mut dry = frame(24.5);
    dry.humidity_percent = None;
    provider.observe(METER_A, dry, at(0));

    assert_eq!(provider.latest_reading().unwrap().humidity_percent, None);
  }

  /// The provider never expires its own cache. Deciding that a reading
  /// is too old to represent a minute is the registry's rule (#2043),
  /// and duplicating it here would create a second freshness boundary
  /// that could drift from the one the archive actually applies.
  #[test]
  fn an_old_reading_is_still_returned_and_left_for_the_registry_to_judge() {
    let provider = SwitchBotMeterProvider::bound(METER_A);
    provider.observe(METER_A, frame(24.5), at(-86_400));

    let latest = provider
      .latest_reading()
      .expect("the provider reports what it heard; freshness is not its call");
    assert_eq!(latest.timestamp, at(-86_400));
  }

  // -- a remembered binding survives the restart --

  /// The regression this whole binding design exists for. A provider
  /// restarted with a remembered device must not answer for a different
  /// meter that happens to advertise first.
  #[test]
  fn a_remembered_binding_ignores_a_different_meter_that_advertises_first() {
    let provider = SwitchBotMeterProvider::bound(METER_A);

    assert_eq!(
      provider.observe(METER_B, frame(31.0), at(0)),
      ObservationOutcome::IgnoredNewDevice
    );
    assert_eq!(
      provider.latest_reading(),
      None,
      "the remembered meter has not been heard, so there is nothing to report"
    );
  }

  #[test]
  fn a_remembered_binding_accepts_its_own_meter_without_re_binding() {
    let provider = SwitchBotMeterProvider::bound(METER_A);

    assert_eq!(
      provider.observe(METER_A, frame(24.5), at(0)),
      ObservationOutcome::Recorded,
      "the device was already chosen, so there is nothing new to persist"
    );
    assert_eq!(provider.latest_reading().unwrap().source, LABEL_A);
  }

  /// Out of range means unavailable, never a quiet substitution: the
  /// alternative is another room's temperature landing in this room's
  /// history.
  #[test]
  fn a_remembered_meter_out_of_range_is_unavailable_rather_than_replaced() {
    let provider = Arc::new(SwitchBotMeterProvider::bound(METER_A));
    provider.observe(METER_B, frame(31.0), at(-5));

    let registry = registry_with(provider);

    assert!(registry.fresh_readings(at(0)).is_empty());
    let status = &registry.provider_statuses(at(0))[0];
    assert_eq!(status.availability, AmbientSensorAvailability::Unavailable);
    assert_eq!(status.last_reading_at, None);
    assert_eq!(status.source, LABEL_A);
  }

  /// A meter refused under one choice is read under the next: being
  /// ignored is a consequence of the current choice, not a property of
  /// the device.
  #[test]
  fn a_meter_ignored_under_one_choice_is_read_under_the_next() {
    let bound = SwitchBotMeterProvider::bound(METER_A);
    assert_eq!(
      bound.observe(METER_B, frame(31.0), at(0)),
      ObservationOutcome::IgnoredNewDevice
    );

    // What the next launch looks like once the other meter was chosen.
    let rebound = SwitchBotMeterProvider::new(Some(METER_B.to_string()));
    assert_eq!(
      rebound.observe(METER_B, frame(31.0), at(0)),
      ObservationOutcome::Recorded
    );
    assert_eq!(rebound.latest_reading().unwrap().source, LABEL_B);
  }

  /// A re-bind must start a new archive series rather than continuing
  /// the old one, so the two meters' histories can never be read as one.
  #[test]
  fn re_binding_to_another_meter_writes_under_a_different_source() {
    let first = SwitchBotMeterProvider::bound(METER_A);
    first.observe(METER_A, frame(24.5), at(0));

    let second = SwitchBotMeterProvider::bound(METER_B);
    second.observe(METER_B, frame(31.0), at(0));

    assert_ne!(
      first.latest_reading().unwrap().source,
      second.latest_reading().unwrap().source
    );
  }

  // -- choosing a different device while the scan is running --

  /// The choice is made on the settings screen while the scan is
  /// already running, so it must take effect there and then rather than
  /// on the next launch. The old device's cached reading goes with it:
  /// left in place, the next archive tick would write it under the new
  /// device's label, which is the two-rooms-in-one-series mistake the
  /// label exists to prevent.
  #[test]
  fn rebinding_to_another_meter_forgets_the_old_ones_reading() {
    let provider = SwitchBotMeterProvider::bound(METER_A);
    provider.observe(METER_A, frame(24.5), at(0));

    provider.rebind(Some(METER_B.to_string()));

    assert_eq!(provider.latest_reading(), None);
    assert_eq!(provider.bound_device().as_deref(), Some(METER_B));
    assert_eq!(provider.source(), LABEL_B);
  }

  #[test]
  fn after_rebinding_the_old_meter_is_refused_until_the_new_one_is_heard() {
    let provider = SwitchBotMeterProvider::bound(METER_A);
    provider.observe(METER_A, frame(24.5), at(0));
    provider.rebind(Some(METER_B.to_string()));

    assert_eq!(
      provider.observe(METER_A, frame(24.6), at(1)),
      ObservationOutcome::IgnoredNewDevice
    );
    assert_eq!(provider.latest_reading(), None);

    assert_eq!(
      provider.observe(METER_B, frame(31.0), at(2)),
      ObservationOutcome::Recorded
    );
    let latest = provider.latest_reading().unwrap();
    assert_eq!(latest.temperature_celsius, 31.0);
    assert_eq!(latest.source, LABEL_B);
  }

  /// Clearing the choice returns the provider to reading nothing, which
  /// is what turning the source off and on again relies on.
  #[test]
  fn rebinding_to_nothing_reads_nothing_again() {
    let provider = SwitchBotMeterProvider::bound(METER_A);
    provider.observe(METER_A, frame(24.5), at(0));

    provider.rebind(None);

    assert_eq!(provider.latest_reading(), None);
    assert_eq!(provider.source(), SWITCHBOT_METER_SOURCE_LABEL);
    assert_eq!(
      provider.observe(METER_A, frame(24.6), at(1)),
      ObservationOutcome::IgnoredNewDevice
    );
    assert_eq!(provider.latest_reading(), None);
  }

  #[test]
  fn new_with_a_remembered_device_matches_the_bound_constructor() {
    let provider = SwitchBotMeterProvider::new(Some(METER_A.to_string()));
    assert_eq!(provider.source(), LABEL_A);
    assert_eq!(
      provider.observe(METER_B, frame(31.0), at(0)),
      ObservationOutcome::IgnoredNewDevice
    );
  }

  // -- more than one meter in range --

  #[test]
  fn a_second_meter_does_not_overwrite_the_bound_meters_reading() {
    let provider = SwitchBotMeterProvider::bound(METER_A);
    provider.observe(METER_A, frame(24.5), at(0));

    assert_eq!(
      provider.observe(METER_B, frame(31.0), at(10)),
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
    let provider = SwitchBotMeterProvider::bound(METER_A);
    provider.observe(METER_A, frame(24.5), at(0));
    provider.observe(METER_B, frame(31.0), at(10));

    provider.observe(METER_A, frame(24.9), at(20));

    let latest = provider.latest_reading().unwrap();
    assert_eq!(latest.temperature_celsius, 24.9);
    assert_eq!(latest.timestamp, at(20));
  }

  /// A skipped meter broadcasts as often as the bound one, so it is
  /// worth naming once and then staying quiet.
  #[test]
  fn a_skipped_meter_is_reported_once_however_often_it_broadcasts() {
    let provider = SwitchBotMeterProvider::unbound();
    provider.observe(METER_A, frame(24.5), at(0));

    assert_eq!(
      provider.observe(METER_B, frame(31.0), at(10)),
      ObservationOutcome::IgnoredNewDevice
    );
    for second in 1..5 {
      assert_eq!(
        provider.observe(METER_B, frame(31.0), at(10 + second)),
        ObservationOutcome::IgnoredKnownDevice
      );
    }
  }

  #[test]
  fn each_distinct_skipped_meter_is_reported_once() {
    let provider = SwitchBotMeterProvider::unbound();
    provider.observe(METER_A, frame(24.5), at(0));

    assert_eq!(
      provider.observe(METER_B, frame(31.0), at(10)),
      ObservationOutcome::IgnoredNewDevice
    );
    assert_eq!(
      provider.observe("aabbccdde5f6", frame(19.0), at(11)),
      ObservationOutcome::IgnoredNewDevice
    );
    assert_eq!(
      provider.observe(METER_B, frame(31.0), at(12)),
      ObservationOutcome::IgnoredKnownDevice
    );
  }

  // -- the provider inside the registry it was built for --

  /// The contract this provider exists to satisfy, asserted through
  /// #2043's registry rather than against this type's own accessors.
  #[test]
  fn a_bound_meter_becomes_an_available_ambient_source_in_the_registry() {
    let provider = Arc::new(SwitchBotMeterProvider::bound(METER_A));
    provider.observe(METER_A, frame(24.5), at(-30));

    let registry = registry_with(provider);

    let readings = registry.fresh_readings(at(0));
    assert_eq!(readings.len(), 1);
    assert_eq!(readings[0].source, LABEL_A);
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
    let provider = Arc::new(SwitchBotMeterProvider::bound(METER_A));
    provider.observe(
      METER_A,
      frame(24.5),
      at(-AMBIENT_READING_MAX_AGE_SECONDS - 1),
    );

    let registry = registry_with(provider);

    assert!(registry.fresh_readings(at(0)).is_empty());
    assert_eq!(
      registry.provider_statuses(at(0))[0].availability,
      AmbientSensorAvailability::Stale
    );
  }

  /// No adapter, no meter, and a meter out of range are indistinguishable
  /// from outside - exactly as #2043 requires. This is also what a failed
  /// scan looks like: the transport reason stays inside the provider.
  #[test]
  fn a_provider_that_never_heard_a_meter_is_unavailable_in_the_registry() {
    let registry = registry_with(Arc::new(SwitchBotMeterProvider::unbound()));

    assert!(registry.fresh_readings(at(0)).is_empty());
    let status = &registry.provider_statuses(at(0))[0];
    assert_eq!(status.availability, AmbientSensorAvailability::Unavailable);
    assert_eq!(status.last_reading_at, None);
    assert_eq!(status.source, SWITCHBOT_METER_SOURCE_LABEL);
  }
}
