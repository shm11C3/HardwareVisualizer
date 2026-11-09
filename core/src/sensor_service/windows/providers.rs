use crate::infrastructure::providers::windows::pawn_io::{
  client::release_shared_intel_msr,
  cpu::{
    ccd_temperature::AmdCcdTarget, power::CpuPowerSampler,
    temperature::CpuTemperatureSampler,
  },
  super_io::{
    diagnostics::read_super_io_chip_id_diagnostics, motherboard::MotherboardSensorSampler,
  },
};
use crate::models::{
  CpuCcdTemperature, MotherboardSensorCollection, SensorAvailability, SensorSupport,
  TemperatureSample, hardware::SuperIoChipIdDiagnostics,
};
use crate::platform::windows::sensors::{
  build_temperature_sample, motherboard_collection_from_readout,
};
use crate::sensor_service::protocol::*;

use super::uptime_ms;

pub(super) const CPU_INTERVAL_MS: u64 = 1_000;
// Leave headroom over the ITE sampler's 1.5 s floor for the coarse uptime clock.
const MOTHERBOARD_INTERVAL_MS: u64 = 1_600;
const REOPEN_FAILED_PROVIDER_MS: u64 = 30_000;
const IDLE_RELEASE_MS: u64 = 15_000;
const DIAGNOSTIC_INTERVAL_MS: u64 = 60_000;

struct Slot<S, T> {
  sampler: Option<(u64, S)>,
  reading: Option<Reading<T>>,
}

impl<S, T> Default for Slot<S, T> {
  fn default() -> Self {
    Self {
      sampler: None,
      reading: None,
    }
  }
}

impl<S, T: Clone> Slot<S, T> {
  fn sample(
    &mut self,
    now: u64,
    interval: u64,
    open: impl FnOnce() -> S,
    read: impl FnOnce(&mut S) -> (T, bool),
  ) -> Reading<T> {
    if let Some(cached) = &self.reading
      && now.saturating_sub(cached.acquired_at_ms) < interval
    {
      return cached.clone();
    }

    let (opened, sampler) = self.sampler.get_or_insert_with(|| (now, open()));
    let (value, reopen) = read(sampler);

    if reopen && now.saturating_sub(*opened) >= REOPEN_FAILED_PROVIDER_MS {
      self.sampler = None;
    }

    let reading = Reading {
      acquired_at_ms: uptime_ms(),
      value,
    };
    self.reading = Some(reading.clone());
    reading
  }
}

pub(super) struct Providers {
  temperature: Slot<CpuTemperatureSampler, TemperatureSample>,
  ccd_reading: Option<(AmdCcdTarget, Reading<Vec<CpuCcdTemperature>>)>,
  last_ccd_request_ms: Option<u64>,
  power: Slot<CpuPowerSampler, CpuPower>,
  motherboard: Slot<MotherboardSensorSampler, MotherboardSensorCollection>,
  diagnostics: Option<(u64, SuperIoChipIdDiagnostics)>,
  last_sample_ms: Option<u64>,
}

impl Providers {
  pub(super) fn new() -> Self {
    Self {
      temperature: Slot::default(),
      ccd_reading: None,
      last_ccd_request_ms: None,
      power: Slot::default(),
      motherboard: Slot::default(),
      diagnostics: None,
      last_sample_ms: None,
    }
  }

  pub(super) fn reset(&mut self) {
    self.ccd_reading = None;
    self.last_ccd_request_ms = None;
    self.release_monitor_samplers();
    self.diagnostics = None;
  }

  fn release_monitor_samplers(&mut self) {
    self.temperature.reading = None;
    if self.last_ccd_request_ms.is_none() {
      self.temperature.sampler = None;
    }
    self.power = Slot::default();
    self.motherboard = Slot::default();
    self.last_sample_ms = None;
    release_shared_intel_msr();
  }

  pub(super) fn expire_idle(&mut self) {
    let now = uptime_ms();
    if self
      .last_ccd_request_ms
      .is_some_and(|at| now.saturating_sub(at) > IDLE_RELEASE_MS)
    {
      self.ccd_reading = None;
      self.last_ccd_request_ms = None;
      if self.last_sample_ms.is_none() {
        self.temperature.sampler = None;
      }
    }
    if self
      .last_sample_ms
      .is_some_and(|at| now.saturating_sub(at) > IDLE_RELEASE_MS)
    {
      self.release_monitor_samplers();
    }
  }

  pub(super) fn sample(&mut self) -> Snapshot {
    let now = uptime_ms();

    // Reset the sampling baseline after a gap, including suspend before SCM's
    // power event reaches the service. Diagnostics cannot extend this lifetime.
    if self
      .last_sample_ms
      .is_some_and(|at| now.saturating_sub(at) > FRESHNESS_MS)
    {
      self.release_monitor_samplers();
    }

    self.last_sample_ms = Some(now);

    Snapshot {
      temperature: self.temperature.sample(
        now,
        CPU_INTERVAL_MS,
        CpuTemperatureSampler::new,
        |sampler| {
          let value = build_temperature_sample(sampler.sample(), Vec::new());
          let reopen =
            matches!(value.availability, SensorAvailability::Unavailable { .. });
          (value, reopen)
        },
      ),
      power: self
        .power
        .sample(now, CPU_INTERVAL_MS, CpuPowerSampler::new, |sampler| {
          let watts = sampler.sample().ok().flatten().map(|v| v as f32);
          let support = if sampler.diagnostics.selected_enablement.is_some() {
            SensorSupport::Supported
          } else {
            SensorSupport::Unsupported
          };

          (
            CpuPower { watts, support },
            !sampler.diagnostics.pawnio.module_loadable,
          )
        }),
      motherboard: self.motherboard.sample(
        now,
        MOTHERBOARD_INTERVAL_MS,
        MotherboardSensorSampler::new,
        |sampler| {
          let value = motherboard_collection_from_readout(sampler.readout());
          let reopen =
            matches!(value.availability, SensorAvailability::Unavailable { .. });
          (value, reopen)
        },
      ),
    }
  }

  pub(super) fn cpu_ccd_temperatures(
    &mut self,
  ) -> Option<Reading<Vec<CpuCcdTemperature>>> {
    self.sample_ccd_temperatures(AmdCcdTarget::detect(), uptime_ms())
  }

  fn sample_ccd_temperatures(
    &mut self,
    target: Option<AmdCcdTarget>,
    now: u64,
  ) -> Option<Reading<Vec<CpuCcdTemperature>>> {
    let Some(target) = target else {
      self.ccd_reading = None;
      return None;
    };

    self.last_ccd_request_ms = Some(now);
    if let Some((cached_target, cached)) = &self.ccd_reading
      && *cached_target == target
      && cached.is_fresh(now)
      && now - cached.acquired_at_ms < CPU_INTERVAL_MS
    {
      return Some(cached.clone());
    }

    // Reuse the same RyzenSMU handle as package temperature. This request
    // does not touch the package reading or its cadence.
    let (opened, sampler) = self
      .temperature
      .sampler
      .get_or_insert_with(|| (now, CpuTemperatureSampler::new()));
    let (value, reopen) = sampler.sample_ccd(target);
    if reopen && now.saturating_sub(*opened) >= REOPEN_FAILED_PROVIDER_MS {
      self.temperature.sampler = None;
    }

    let reading = Reading {
      acquired_at_ms: uptime_ms(),
      value,
    };
    self.ccd_reading = Some((target, reading.clone()));
    Some(reading)
  }

  pub(super) fn diagnostics(&mut self) -> SuperIoChipIdDiagnostics {
    let now = uptime_ms();

    if self
      .diagnostics
      .as_ref()
      .is_none_or(|(at, _)| now.saturating_sub(*at) >= DIAGNOSTIC_INTERVAL_MS)
    {
      self.diagnostics = Some((now, read_super_io_chip_id_diagnostics()));
    }

    self.diagnostics.as_ref().unwrap().1.clone()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::infrastructure::providers::windows::pawn_io::cpu::ccd_temperature::{
    synthetic_raphael_target, synthetic_target,
  };

  #[test]
  fn missing_ccd_module_returns_both_slots_and_keeps_the_package_cache() {
    let now = uptime_ms();
    let mut providers = Providers::new();
    providers.temperature.sampler =
      Some((now, CpuTemperatureSampler::synthetic_unavailable()));
    providers.temperature.reading = Some(Reading {
      acquired_at_ms: now,
      value: TemperatureSample {
        cpu_temperature: Some(63.0),
        ..Default::default()
      },
    });

    let expected = [
      CpuCcdTemperature {
        package_index: 0,
        ccd_slot: 1,
        temperature_celsius: None,
      },
      CpuCcdTemperature {
        package_index: 0,
        ccd_slot: 2,
        temperature_celsius: None,
      },
    ];
    let first = providers
      .sample_ccd_temperatures(Some(synthetic_target()), now)
      .unwrap();
    assert_eq!(first.value, expected);
    assert!(providers.temperature.sampler.is_some());

    let cached = providers
      .sample_ccd_temperatures(
        Some(synthetic_target()),
        first.acquired_at_ms + CPU_INTERVAL_MS - 1,
      )
      .unwrap();
    assert_eq!(cached.acquired_at_ms, first.acquired_at_ms);
    assert_eq!(cached.value, expected);
    assert!(providers.temperature.sampler.is_some());

    let retry_due = providers
      .sample_ccd_temperatures(
        Some(synthetic_target()),
        first.acquired_at_ms + REOPEN_FAILED_PROVIDER_MS,
      )
      .unwrap();
    assert_eq!(retry_due.value, expected);
    assert!(providers.temperature.sampler.is_none());

    let package = providers.temperature.reading.as_ref().unwrap();
    assert_eq!(package.acquired_at_ms, now);
    assert_eq!(package.value.cpu_temperature, Some(63.0));
    assert!(providers.sample_ccd_temperatures(None, now).is_none());
    assert!(providers.ccd_reading.is_none());
  }

  #[test]
  fn ccd_cache_is_scoped_to_the_selected_register_map() {
    let now = uptime_ms();
    let vermeer = synthetic_target();
    let raphael = synthetic_raphael_target();
    let mut providers = Providers::new();
    providers.temperature.sampler =
      Some((now, CpuTemperatureSampler::synthetic_unavailable()));

    let mut previous = vermeer.unavailable_readings();
    previous[0].temperature_celsius = Some(40.0);
    previous[1].temperature_celsius = Some(54.0);
    providers.ccd_reading = Some((
      vermeer,
      Reading {
        acquired_at_ms: now,
        value: previous,
      },
    ));

    let reading = providers
      .sample_ccd_temperatures(Some(raphael), now)
      .unwrap();
    assert_eq!(reading.value, raphael.unavailable_readings());
    assert!(
      providers
        .ccd_reading
        .is_some_and(|(cached, _)| cached == raphael)
    );
  }

  #[test]
  fn shared_cache_keeps_the_original_acquisition_until_the_provider_is_due() {
    let mut slot = Slot::<(), u32>::default();
    let interval = MOTHERBOARD_INTERVAL_MS;
    let first = slot.sample(uptime_ms(), interval, || (), |_| (40, false));

    let cached = slot.sample(
      first.acquired_at_ms + interval - 1,
      interval,
      || panic!("cached reads must not open a sampler"),
      |_| panic!("cached reads must not touch hardware"),
    );

    assert_eq!(cached.acquired_at_ms, first.acquired_at_ms);
    assert_eq!(cached.value, 40);

    let next = slot.sample(
      first.acquired_at_ms + interval,
      interval,
      || panic!("reuse the open sampler"),
      |_| (42, false),
    );

    assert_eq!(next.value, 42);
  }
}
