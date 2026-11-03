use crate::infrastructure::providers::windows::{
  cpu_power::CpuPowerSampler,
  cpu_temperature::CpuTemperatureSampler,
  pawn_io::release_shared_intel_msr,
  super_io::{
    diagnostics::read_super_io_chip_id_diagnostics, motherboard::MotherboardSensorSampler,
  },
};
use crate::models::{SensorSupport, hardware::SuperIoChipIdDiagnostics};
use crate::platform::windows::sensors::{
  build_temperature_sample, motherboard_collection_from_readout,
};
use crate::sensor_service::protocol::*;

use super::uptime_ms;

pub(super) struct Providers {
  temperature: Option<CpuTemperatureSampler>,
  power: Option<CpuPowerSampler>,
  motherboard: Option<MotherboardSensorSampler>,
  pub(super) snapshot: Snapshot,
  diagnostics: Option<(u64, SuperIoChipIdDiagnostics)>,
  sequence: u64,
  last_request_ms: u64,
  temperature_opened_ms: u64,
  power_opened_ms: u64,
  motherboard_opened_ms: u64,
}

impl Providers {
  pub(super) fn new() -> Self {
    Self {
      temperature: None,
      power: None,
      motherboard: None,
      snapshot: Snapshot::default(),
      diagnostics: None,
      sequence: 0,
      last_request_ms: uptime_ms(),
      temperature_opened_ms: 0,
      power_opened_ms: 0,
      motherboard_opened_ms: 0,
    }
  }

  pub(super) fn reset(&mut self) {
    self.temperature = None;
    self.power = None;
    self.motherboard = None;
    self.snapshot = Snapshot::default();
    self.diagnostics = None;
    release_shared_intel_msr();
  }

  pub(super) fn expire_idle(&mut self) {
    if uptime_ms().saturating_sub(self.last_request_ms) > 15_000 {
      self.reset();
    }
  }

  pub(super) fn sample(&mut self, families: u8) -> Snapshot {
    let now = uptime_ms();

    // A gap also covers suspend/resume before SCM's power event is handled.
    if now.saturating_sub(self.last_request_ms) > FRESHNESS_MS {
      self.reset();
    }

    self.last_request_ms = now;

    if families & TEMPERATURE != 0 && due(&self.snapshot.temperature, now, 1_000) {
      if self.temperature.is_none() {
        self.temperature_opened_ms = now;
      }

      let sampler = self
        .temperature
        .get_or_insert_with(CpuTemperatureSampler::new);

      let value = build_temperature_sample(sampler.sample(), Vec::new());

      if matches!(
        value.availability,
        crate::models::SensorAvailability::Unavailable { .. }
      ) && now.saturating_sub(self.temperature_opened_ms) >= 30_000
      {
        self.temperature = None;
      }

      self.sequence += 1;
      self.snapshot.temperature = Some(Reading {
        acquired_at_ms: uptime_ms(),
        sequence: self.sequence,
        value,
      });
    }

    if families & POWER != 0 && due(&self.snapshot.power, now, 1_000) {
      if self.power.is_none() {
        self.power_opened_ms = now;
      }

      let sampler = self.power.get_or_insert_with(CpuPowerSampler::new);
      let watts = sampler.sample().ok().flatten().map(|v| v as f32);
      let support = if sampler.diagnostics.selected_enablement.is_some() {
        SensorSupport::Supported
      } else {
        SensorSupport::Unsupported
      };

      if !sampler.diagnostics.pawnio.module_loadable
        && now.saturating_sub(self.power_opened_ms) >= 30_000
      {
        self.power = None;
      }

      self.sequence += 1;
      self.snapshot.power = Some(Reading {
        acquired_at_ms: uptime_ms(),
        sequence: self.sequence,
        value: CpuPower { watts, support },
      });
    }

    if families & MOTHERBOARD != 0 && due(&self.snapshot.motherboard, now, 1_600) {
      if self.motherboard.is_none() {
        self.motherboard_opened_ms = now;
      }

      let sampler = self
        .motherboard
        .get_or_insert_with(MotherboardSensorSampler::new);

      let value = motherboard_collection_from_readout(sampler.readout());

      if matches!(
        value.availability,
        crate::models::SensorAvailability::Unavailable { .. }
      ) && now.saturating_sub(self.motherboard_opened_ms) >= 30_000
      {
        self.motherboard = None;
      }

      self.sequence += 1;
      self.snapshot.motherboard = Some(Reading {
        acquired_at_ms: uptime_ms(),
        sequence: self.sequence,
        value,
      });
    }

    Snapshot {
      temperature: (families & TEMPERATURE != 0)
        .then(|| self.snapshot.temperature.clone())
        .flatten(),
      power: (families & POWER != 0)
        .then(|| self.snapshot.power.clone())
        .flatten(),
      motherboard: (families & MOTHERBOARD != 0)
        .then(|| self.snapshot.motherboard.clone())
        .flatten(),
    }
  }

  pub(super) fn diagnostics(&mut self) -> SuperIoChipIdDiagnostics {
    let now = uptime_ms();
    self.last_request_ms = now;

    if self
      .diagnostics
      .as_ref()
      .is_none_or(|(at, _)| now.saturating_sub(*at) >= 60_000)
    {
      self.diagnostics = Some((now, read_super_io_chip_id_diagnostics()));
    }

    self.diagnostics.as_ref().unwrap().1.clone()
  }
}

fn due<T>(reading: &Option<Reading<T>>, now: u64, interval: u64) -> bool {
  reading
    .as_ref()
    .is_none_or(|v| now.saturating_sub(v.acquired_at_ms) >= interval)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn multiple_clients_do_not_multiply_acquisitions_or_change_age() {
    let cached = Some(Reading {
      acquired_at_ms: 100,
      sequence: 2,
      value: (),
    });
    assert!(!due(&cached, 200, 1_500));
    assert!(!due(&cached, 1_599, 1_500));
    assert!(due(&cached, 1_600, 1_500));
    assert!(due::<()>(&None, 100, 1_500));
  }
}
