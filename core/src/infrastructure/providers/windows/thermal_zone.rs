//! ACPI thermal-zone temperature provider (Windows).
//!
//! Reads thermal-zone temperatures via WMI from two sources, preferring
//! whichever answered last:
//!
//! 1. `MSAcpi_ThermalZoneTemperature` (`root\WMI`) — the canonical ACPI
//!    sensor interface. On many consumer systems it requires
//!    administrator rights or is not implemented by the firmware.
//! 2. `Win32_PerfFormattedData_Counters_ThermalZoneInformation`
//!    (`root\CIMV2`) — thermal-zone performance counters, usually
//!    readable without elevation.
//!
//! Both report ACPI thermal zones (CPU package, chipset, skin, …), not
//! per-core CPU sensors — reading those would need a kernel driver.
//!
//! `WMIConnection` is not `Send`, so all WMI work is confined to one
//! dedicated sampler thread (mirroring the macOS GPU-usage sampler). The
//! collector reads the latest result from a process-wide cache.

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::Deserialize;
use wmi::WMIConnection;

use crate::models::SensorTemperature;
use crate::utils::thermal;
use crate::{log_debug, log_error};

/// Cadence of the sampler thread. Thermal zones change slowly compared to
/// usage counters, so 2 s keeps WMI load negligible while staying close
/// to the collector's 1 s tick.
const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);

/// After this many consecutive failed or empty reads the sampler assumes
/// the host exposes no readable thermal zones and falls back to
/// [`IDLE_INTERVAL`]. It keeps retrying so transient WMI outages recover.
const FAILURES_BEFORE_IDLE: u32 = 5;
const IDLE_INTERVAL: Duration = Duration::from_secs(60);

/// Latest successfully read zones (empty while unavailable).
static LATEST_ZONES: OnceLock<Mutex<Vec<SensorTemperature>>> = OnceLock::new();

/// Indicates whether the sampler thread has been started.
static SAMPLER_STARTED: OnceLock<()> = OnceLock::new();

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MsAcpiThermalZoneTemperature {
  instance_name: String,
  current_temperature: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ThermalZonePerfCounter {
  name: String,
  temperature: Option<u32>,
  high_precision_temperature: Option<u32>,
}

/// Start the background sampler thread. No-op if already started.
pub fn init_thermal_zone_sampler() {
  if SAMPLER_STARTED.get().is_some() {
    return;
  }

  let _ = LATEST_ZONES.get_or_init(|| Mutex::new(Vec::new()));

  // Even under races, `OnceLock` ensures only one thread is spawned.
  if SAMPLER_STARTED.set(()).is_err() {
    return;
  }

  if let Err(e) = std::thread::Builder::new()
    .name("thermal-zone-sampler".to_string())
    .spawn(sampler_loop)
  {
    log_error!(
      "failed to spawn thermal-zone-sampler thread",
      "thermal_zone::init_thermal_zone_sampler",
      Some(e.to_string())
    );
  }
}

/// Latest thermal-zone readings in raw °C. Empty until the sampler has
/// produced its first successful read (or when the host has no readable
/// zones).
pub fn read_thermal_zones_cached() -> Vec<SensorTemperature> {
  LATEST_ZONES
    .get()
    .and_then(|m| m.lock().ok())
    .map(|zones| zones.clone())
    .unwrap_or_default()
}

fn store_zones(zones: Vec<SensorTemperature>) {
  if let Some(latest) = LATEST_ZONES.get()
    && let Ok(mut guard) = latest.lock()
  {
    *guard = zones;
  }
}

/// Which WMI source produced zones on the previous round. The preferred
/// source is tried first on the next round so a permanently unavailable
/// source (e.g. `MSAcpi` without elevation) is not re-queried every tick.
#[derive(Clone, Copy, PartialEq)]
enum Source {
  MsAcpi,
  PerfCounter,
}

/// Dedicated thread: owns the WMI connections and refreshes
/// [`LATEST_ZONES`] forever.
fn sampler_loop() {
  let mut acpi_conn: Option<WMIConnection> = None;
  let mut perf_conn: Option<WMIConnection> = None;
  let mut preferred = Source::MsAcpi;
  let mut consecutive_failures: u32 = 0;

  loop {
    let order = match preferred {
      Source::MsAcpi => [Source::MsAcpi, Source::PerfCounter],
      Source::PerfCounter => [Source::PerfCounter, Source::MsAcpi],
    };

    let mut zones: Vec<SensorTemperature> = Vec::new();
    for source in order {
      let result = match source {
        Source::MsAcpi => query_source(&mut acpi_conn, r"root\WMI", query_msacpi_zones),
        Source::PerfCounter => {
          query_source(&mut perf_conn, r"root\CIMV2", query_perf_counter_zones)
        }
      };
      if let Some(read) = result
        && !read.is_empty()
      {
        zones = read;
        preferred = source;
        break;
      }
    }

    if zones.is_empty() {
      consecutive_failures = consecutive_failures.saturating_add(1);
      if consecutive_failures == FAILURES_BEFORE_IDLE {
        log_debug!(
          "no readable thermal zones (MSAcpi + perf counters); switching to idle polling",
          "thermal_zone::sampler_loop",
          None::<&str>
        );
      }
    } else {
      consecutive_failures = 0;
    }
    store_zones(zones);

    let interval = if consecutive_failures >= FAILURES_BEFORE_IDLE {
      IDLE_INTERVAL
    } else {
      SAMPLE_INTERVAL
    };
    std::thread::sleep(interval);
  }
}

/// Run one query against a lazily (re)created namespace connection.
/// On failure the connection is dropped so the next round reconnects
/// (handles WMI service restarts).
fn query_source(
  conn: &mut Option<WMIConnection>,
  namespace: &str,
  query: fn(&WMIConnection) -> Result<Vec<SensorTemperature>, String>,
) -> Option<Vec<SensorTemperature>> {
  if conn.is_none() {
    *conn = WMIConnection::with_namespace_path(namespace).ok();
  }

  match conn.as_ref().map(query)? {
    Ok(zones) => Some(zones),
    Err(_) => {
      // Expected on hosts where the class is unsupported or requires
      // elevation — stay quiet and let the caller try the other source.
      *conn = None;
      None
    }
  }
}

fn query_msacpi_zones(conn: &WMIConnection) -> Result<Vec<SensorTemperature>, String> {
  let rows: Vec<MsAcpiThermalZoneTemperature> = conn
    .raw_query(
      "SELECT InstanceName, CurrentTemperature FROM MSAcpi_ThermalZoneTemperature",
    )
    .map_err(|e| format!("MSAcpi_ThermalZoneTemperature query failed: {e:?}"))?;

  let zones = rows
    .into_iter()
    .filter_map(|row| {
      let celsius = thermal::decikelvin_to_celsius(row.current_temperature);
      thermal::is_plausible_celsius(celsius).then(|| SensorTemperature {
        name: thermal::clean_zone_name(&row.instance_name),
        temperature: celsius,
      })
    })
    .collect();

  Ok(thermal::dedupe_zones(zones))
}

fn query_perf_counter_zones(
  conn: &WMIConnection,
) -> Result<Vec<SensorTemperature>, String> {
  let rows: Vec<ThermalZonePerfCounter> = conn
    .raw_query(
      "SELECT Name, Temperature, HighPrecisionTemperature \
       FROM Win32_PerfFormattedData_Counters_ThermalZoneInformation",
    )
    .map_err(|e| format!("ThermalZoneInformation query failed: {e:?}"))?;

  let zones = rows
    .into_iter()
    .filter_map(|row| {
      // `HighPrecisionTemperature` (tenths of K) is only populated on
      // newer Windows builds; fall back to whole-Kelvin `Temperature`.
      // Take the first candidate that passes the plausibility filter so
      // a zone with one bogus counter still reports through the other.
      let celsius = [
        row
          .high_precision_temperature
          .map(thermal::decikelvin_to_celsius),
        row.temperature.map(thermal::kelvin_to_celsius),
      ]
      .into_iter()
      .flatten()
      .find(|c| thermal::is_plausible_celsius(*c))?;

      Some(SensorTemperature {
        name: thermal::clean_zone_name(&row.name),
        temperature: celsius,
      })
    })
    .collect();

  Ok(thermal::dedupe_zones(zones))
}
