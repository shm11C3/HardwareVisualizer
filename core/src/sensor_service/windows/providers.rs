use crate::infrastructure::providers::windows::{
  cpu_power::CpuPowerSampler,
  cpu_temperature::CpuTemperatureSampler,
  pawn_io::release_shared_intel_msr,
  super_io::{
    diagnostics::read_super_io_chip_id_diagnostics, motherboard::MotherboardSensorSampler,
  },
};
use crate::models::{
  MotherboardSensorCollection, SensorAvailability, SensorSupport, TemperatureSample,
  hardware::SuperIoChipIdDiagnostics,
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
  power: Slot<CpuPowerSampler, CpuPower>,
  motherboard: Slot<MotherboardSensorSampler, MotherboardSensorCollection>,
  diagnostics: Option<(u64, SuperIoChipIdDiagnostics)>,
  last_sample_ms: Option<u64>,
}

impl Providers {
  pub(super) fn new() -> Self {
    Self {
      temperature: Slot::default(),
      power: Slot::default(),
      motherboard: Slot::default(),
      diagnostics: None,
      last_sample_ms: None,
    }
  }

  pub(super) fn reset(&mut self) {
    self.release_samplers();
    self.diagnostics = None;
  }

  fn release_samplers(&mut self) {
    self.temperature = Slot::default();
    self.power = Slot::default();
    self.motherboard = Slot::default();
    self.last_sample_ms = None;
    release_shared_intel_msr();
  }

  pub(super) fn expire_idle(&mut self) {
    if self
      .last_sample_ms
      .is_some_and(|at| uptime_ms().saturating_sub(at) > IDLE_RELEASE_MS)
    {
      self.release_samplers();
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
      self.release_samplers();
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
