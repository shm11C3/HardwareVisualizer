//! Windows BLE advertisement scanning that feeds
//! [`SwitchBotMeterProvider`] (#2044).
//!
//! Observation only. The scan never connects, pairs, bonds, or writes to
//! a meter: it reads the temperature the device already broadcasts to
//! anyone listening, which is why no pairing flow appears anywhere in
//! this feature. Everything the app learns here comes off the air and
//! stays on this machine - there is no SwitchBot account, no cloud API,
//! and no outbound request of any kind.
//!
//! One honest caveat about the words: the SwitchBot meter publishes its
//! service data in the *scan response*, which a radio only receives if
//! it answers advertisements with a scan request. btleplug's Windows
//! backend accordingly runs the WinRT advertisement watcher in
//! `BluetoothLEScanningMode::Active`. So this is passive in the sense
//! the product cares about - no connection, no pairing, no device state
//! changed - while the radio itself does transmit scan requests.
//!
//! This module is Windows-only by compilation. The decoding and caching
//! it drives are not: they live in
//! [`super::advertisement`] and [`super::provider`] and are tested on
//! every platform from fixed byte strings.

use std::collections::HashSet;
use std::sync::Arc;

use btleplug::api::{Central, CentralEvent, Manager as _, ScanFilter};
use btleplug::platform::Manager;
use chrono::Utc;
use futures::StreamExt;
use tokio::runtime::Handle;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::{log_info, log_warn};

use super::advertisement::{MeterAdvertisement, decode_service_data};
use super::provider::{ObservationOutcome, SwitchBotMeterProvider};

/// Called once with the device identifier when the provider latches, so
/// the binding can be remembered for the next launch.
pub type OnBound = Arc<dyn Fn(&str) + Send + Sync>;

/// Log target for everything this module reports.
const LOG_TARGET: &str = "providers::switchbot_meter::scan";

/// A running SwitchBot advertisement scan.
///
/// Mirrors the other Core workers: spawned on the app's runtime, stopped
/// through a `watch` channel, and joined on shutdown so the radio is
/// released before the process exits.
pub struct SwitchBotScanController {
  handle: JoinHandle<()>,
  stop_tx: watch::Sender<bool>,
}

impl SwitchBotScanController {
  /// Start scanning on `runtime`, pushing every decoded meter frame into
  /// `provider`.
  ///
  /// Never fails: a machine with no Bluetooth radio, a disabled adapter,
  /// or a refused scan is a normal configuration for an optional ambient
  /// source, not an error the user must dismiss. Those cases log once
  /// and leave the provider reporting nothing, which #2043 already
  /// renders as an unavailable source.
  pub fn setup(
    runtime: Handle,
    provider: Arc<SwitchBotMeterProvider>,
    on_bound: OnBound,
  ) -> Self {
    let (stop_tx, mut stop_rx) = watch::channel(false);

    let handle = runtime.spawn(async move {
      // Encrypted meters already reported, so the notice is made once
      // per device rather than on every broadcast.
      let mut reported_encrypted: HashSet<String> = HashSet::new();

      let Some(central) = open_adapter().await else {
        return;
      };

      // The event stream is subscribed to before the scan starts so no
      // advertisement can arrive in the gap between the two calls.
      let mut events = match central.events().await {
        Ok(events) => events,
        Err(e) => {
          log_warn!(
            &format!(
              "cannot observe Bluetooth events, ambient source stays unavailable: {e}"
            ),
            LOG_TARGET,
            None::<&str>
          );
          return;
        }
      };

      // No service filter. btleplug applies `ScanFilter` in software on
      // Windows and holds back non-matching devices' scan responses,
      // which is exactly the packet the meter's reading lives in.
      // Filtering here instead costs one map lookup per advertisement
      // and cannot hide the payload we came for.
      if let Err(e) = central.start_scan(ScanFilter::default()).await {
        log_warn!(
          &format!("cannot start Bluetooth scan, ambient source stays unavailable: {e}"),
          LOG_TARGET,
          None::<&str>
        );
        return;
      }

      log_info!(
        "scanning for SwitchBot meter advertisements",
        LOG_TARGET,
        None::<&str>
      );

      loop {
        tokio::select! {
          biased;
          changed = stop_rx.changed() => {
            if changed.is_err() || *stop_rx.borrow() {
              break;
            }
          }
          event = events.next() => match event {
            Some(event) => handle_event(
              event,
              &provider,
              on_bound.as_ref(),
              &mut reported_encrypted,
            ),
            // The adapter went away mid-session (dongle unplugged,
            // radio disabled). Recovering would mean re-probing on a
            // timer forever; the ambient source simply stops reporting
            // and #2043 reports it stale, which is the same outcome as
            // the meter itself going quiet.
            None => {
              log_warn!(
                "Bluetooth event stream ended, ambient source stays unavailable",
                LOG_TARGET,
                None::<&str>
              );
              break;
            }
          },
        }
      }

      // Release the radio rather than leaving a scan running for a
      // provider nobody will read again.
      if let Err(e) = central.stop_scan().await {
        log_warn!(
          &format!("failed to stop Bluetooth scan: {e}"),
          LOG_TARGET,
          None::<&str>
        );
      }
    });

    Self { handle, stop_tx }
  }

  pub async fn terminate(self) {
    let _ = self.stop_tx.send(true);
    let _ = self.handle.await;
  }
}

/// The first Bluetooth adapter, or `None` when the machine has none.
///
/// "No adapter" is reported at info level, not warn: a desktop without
/// Bluetooth is an ordinary machine, and the user opted into an ambient
/// source rather than promising one exists.
async fn open_adapter() -> Option<btleplug::platform::Adapter> {
  let manager = match Manager::new().await {
    Ok(manager) => manager,
    Err(e) => {
      log_warn!(
        &format!("Bluetooth is unavailable, ambient source stays unavailable: {e}"),
        LOG_TARGET,
        None::<&str>
      );
      return None;
    }
  };

  let adapters = match manager.adapters().await {
    Ok(adapters) => adapters,
    Err(e) => {
      log_warn!(
        &format!("cannot list Bluetooth adapters, ambient source stays unavailable: {e}"),
        LOG_TARGET,
        None::<&str>
      );
      return None;
    }
  };

  let adapter = adapters.into_iter().next();
  if adapter.is_none() {
    log_info!(
      "no Bluetooth adapter found, SwitchBot ambient source stays unavailable",
      LOG_TARGET,
      None::<&str>
    );
  }
  adapter
}

/// Route one central event into the provider.
///
/// Only service-data advertisements carry a meter reading; every other
/// event from every other nearby device is dropped without a word,
/// because a scan with no filter sees a great many of them.
///
/// `on_bound` is called with the device identifier the very first time
/// this provider latches, so the caller can remember it and bind to the
/// same meter next launch. `reported_encrypted` keeps the encrypted-meter
/// notice to once per device, since such a meter re-broadcasts as often
/// as a readable one.
fn handle_event(
  event: CentralEvent,
  provider: &SwitchBotMeterProvider,
  on_bound: &(dyn Fn(&str) + Send + Sync),
  reported_encrypted: &mut HashSet<String>,
) {
  let CentralEvent::ServiceDataAdvertisement { id, service_data } = event else {
    return;
  };

  // `PeripheralId` is the transport's stable handle for one device. Only
  // a short tail of it reaches a label or the archive; the full form is
  // kept here and in settings so two meters can be told apart reliably.
  let device_id = format!("{id:?}");
  let observed_at = Utc::now();

  for (uuid, data) in &service_data {
    let frame = match decode_service_data(uuid.as_u128(), data) {
      MeterAdvertisement::Reading(frame) => frame,

      // Worth saying once: from outside, an encrypted meter is
      // indistinguishable from no meter at all, and the user would
      // otherwise have no way to learn why the sensor they own reports
      // nothing.
      MeterAdvertisement::Encrypted(model) => {
        if reported_encrypted.insert(device_id.clone()) {
          log_warn!(
            &format!(
              "a SwitchBot {model:?} is broadcasting an encrypted payload; its readings cannot be decoded and are ignored"
            ),
            LOG_TARGET,
            None::<&str>
          );
        }
        continue;
      }

      MeterAdvertisement::Ignored => continue,
    };

    match provider.observe(&device_id, frame, observed_at) {
      ObservationOutcome::Bound => {
        log_info!(
          &format!("bound ambient source to a SwitchBot {:?}", frame.model),
          LOG_TARGET,
          None::<&str>
        );
        // Remember the choice, or the next launch would make it again -
        // possibly landing on a different meter.
        on_bound(&device_id);
      }
      ObservationOutcome::IgnoredNewDevice => {
        log_warn!(
          "another SwitchBot meter is in range; readings from it are ignored so one ambient source stays one sensor",
          LOG_TARGET,
          None::<&str>
        );
      }
      ObservationOutcome::Recorded | ObservationOutcome::IgnoredKnownDevice => {}
    }
  }
}
