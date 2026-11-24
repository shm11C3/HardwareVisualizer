use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::super::client::{
  PawnIoClient, PawnIoDiscovery, PawnIoError, PawnIoInitError, PawnIoModule,
  open_shared_intel_msr, read_shared_msr,
};
pub use super::identity::CpuIdentity;
use super::identity::{CpuVendor, detect_amd_rapl_support, detect_cpu_identity};
use super::power_decode::{PowerDecoder, PowerUnitDecodeError};
use crate::models::SensorEnablement;

const MSR_RAPL_POWER_UNIT: u64 = 0x606;
const MSR_PKG_ENERGY_STATUS: u64 = 0x611;
const AMD_RAPL_POWER_UNIT: u64 = 0xc001_0299;
const AMD_PKG_ENERGY_STATUS: u64 = 0xc001_029b;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuPowerSource {
  IntelRaplPackageMsr,
  AmdZenRaplPackageMsr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CpuPowerFallbackReason {
  UnsupportedCpuVendor(String),
  IntelSilvermontModel(u32),
  AmdFamilyUnsupported(u32),
  AmdRaplUnavailable,
}

impl std::fmt::Display for CpuPowerFallbackReason {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::UnsupportedCpuVendor(vendor) => {
        write!(f, "unsupported CPU vendor {vendor}")
      }
      Self::IntelSilvermontModel(model) => {
        write!(
          f,
          "Intel model 0x{model:x} is excluded from the standard RAPL path"
        )
      }
      Self::AmdFamilyUnsupported(family) => {
        write!(
          f,
          "AMD family 0x{family:x} is unsupported by the AMDFamily17 path"
        )
      }
      Self::AmdRaplUnavailable => f.write_str("AMD RAPL capability is unavailable"),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CpuPowerCandidate {
  pub(crate) source: CpuPowerSource,
  pub(crate) module: PawnIoModule,
  pub(crate) enablement: SensorEnablement,
}

pub(crate) fn amd_power_enablement(family: u32, model: u32) -> SensorEnablement {
  match family {
    0x17 if matches!(model, 0x01 | 0x08) => SensorEnablement::Verified,
    0x17 => SensorEnablement::Experimental,
    0x19 if matches!(model, 0x21 | 0x61) => SensorEnablement::Verified,
    0x19 => SensorEnablement::Experimental,
    0x1a if model == 0x02 => SensorEnablement::Verified,
    0x1a => SensorEnablement::Experimental,
    _ => SensorEnablement::Unsupported,
  }
}

pub(crate) fn select_cpu_power_candidate(
  cpu: &CpuIdentity,
  amd_rapl_supported: Option<bool>,
) -> Result<CpuPowerCandidate, CpuPowerFallbackReason> {
  match (&cpu.vendor, cpu.vendor_id.as_str()) {
    (CpuVendor::Intel, "GenuineIntel") => {
      if cpu.family == 6 && matches!(cpu.model, 0x37 | 0x4a | 0x5a | 0x5d) {
        return Err(CpuPowerFallbackReason::IntelSilvermontModel(cpu.model));
      }

      Ok(CpuPowerCandidate {
        source: CpuPowerSource::IntelRaplPackageMsr,
        module: PawnIoModule::IntelMsr,
        enablement: SensorEnablement::Verified,
      })
    }
    (CpuVendor::Amd, "AuthenticAMD") => {
      if !(0x17..=0x1a).contains(&cpu.family) {
        return Err(CpuPowerFallbackReason::AmdFamilyUnsupported(cpu.family));
      }
      if amd_rapl_supported != Some(true) {
        return Err(CpuPowerFallbackReason::AmdRaplUnavailable);
      }

      let enablement = amd_power_enablement(cpu.family, cpu.model);
      if enablement == SensorEnablement::Unsupported {
        return Err(CpuPowerFallbackReason::AmdFamilyUnsupported(cpu.family));
      }

      Ok(CpuPowerCandidate {
        source: CpuPowerSource::AmdZenRaplPackageMsr,
        module: PawnIoModule::AmdFamily17,
        enablement,
      })
    }
    _ => Err(CpuPowerFallbackReason::UnsupportedCpuVendor(
      cpu.vendor_id.clone(),
    )),
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuPowerDiagnostics {
  pub cpu: CpuIdentity,
  pub amd_rapl_supported: Option<bool>,
  pub pawnio: PawnIoDiscovery,
  pub selected_source: Option<CpuPowerSource>,
  pub selected_enablement: Option<SensorEnablement>,
  pub fallback_reason: Option<String>,
}

struct RuntimePowerSampler {
  decoder: PowerDecoder,
  clock_origin: Instant,
  skip_first_public_sample: bool,
}

impl RuntimePowerSampler {
  fn from_probe(
    client: &PawnIoClient,
    unit_msr: u64,
    energy_msr: u64,
  ) -> Result<Self, String> {
    let clock_origin = Instant::now();
    let unit_result = client.read_msr(unit_msr);
    let energy_result = client.read_msr(energy_msr);
    // This timestamp is captured immediately after the probe energy read so
    // the successful probe itself is the decoder baseline.
    let probe_timestamp = clock_origin.elapsed().as_secs_f64();
    let unit_register = unit_result.map_err(|error| error.to_string())?;
    let energy_register = energy_result.map_err(|error| error.to_string())?;
    let decoder = PowerDecoder::from_unit_register_with_baseline(
      unit_register,
      energy_register,
      probe_timestamp,
    )
    .map_err(format_power_unit_error)?;

    Ok(Self {
      decoder,
      clock_origin,
      skip_first_public_sample: true,
    })
  }

  fn sample_shared(
    &mut self,
    client: &Arc<Mutex<PawnIoClient>>,
    energy_msr: u64,
  ) -> Result<Option<f64>, String> {
    let reading = read_shared_msr(client, energy_msr);
    let timestamp = self.clock_origin.elapsed().as_secs_f64();
    self.consume_public_reading(reading, timestamp)
  }

  fn sample_owned(
    &mut self,
    client: &PawnIoClient,
    energy_msr: u64,
  ) -> Result<Option<f64>, String> {
    let reading = client.read_msr(energy_msr);
    let timestamp = self.clock_origin.elapsed().as_secs_f64();
    self.consume_public_reading(reading, timestamp)
  }

  fn consume_public_reading(
    &mut self,
    reading: Result<u64, PawnIoError>,
    timestamp_seconds: f64,
  ) -> Result<Option<f64>, String> {
    let suppress_public_sample = self.skip_first_public_sample;
    self.skip_first_public_sample = false;

    let result = self.consume_reading(reading, timestamp_seconds);
    match result {
      Err(reason) => Err(reason),
      Ok(_) if suppress_public_sample => Ok(None),
      Ok(power) => Ok(power),
    }
  }

  fn consume_reading(
    &mut self,
    reading: Result<u64, PawnIoError>,
    timestamp_seconds: f64,
  ) -> Result<Option<f64>, String> {
    match reading {
      Ok(value) => Ok(self.decoder.sample(Some(value), timestamp_seconds)),
      Err(error) => {
        self.decoder.sample(None, timestamp_seconds);
        Err(error.to_string())
      }
    }
  }
}

enum ActivePowerSource {
  Intel {
    client: Arc<Mutex<PawnIoClient>>,
    sampler: RuntimePowerSampler,
  },
  Amd {
    client: PawnIoClient,
    sampler: RuntimePowerSampler,
  },
}

pub(crate) struct CpuPowerSampler {
  pub(crate) diagnostics: CpuPowerDiagnostics,
  active: Option<ActivePowerSource>,
  inactive_reason: Option<String>,
}

impl CpuPowerSampler {
  pub(crate) fn new() -> Self {
    let cpu = detect_cpu_identity();
    let amd_rapl_supported = (cpu.vendor == CpuVendor::Amd)
      .then(detect_amd_rapl_support)
      .flatten();
    let candidate = match select_cpu_power_candidate(&cpu, amd_rapl_supported) {
      Ok(candidate) => candidate,
      Err(reason) => {
        let fallback_reason = reason.to_string();
        let mut pawnio = PawnIoDiscovery::probe_install();
        if pawnio.fallback_reason.is_none() {
          pawnio.fallback_reason =
            Some("PawnIO module not selected for unsupported CPU".to_string());
        }
        return Self {
          diagnostics: CpuPowerDiagnostics {
            cpu,
            amd_rapl_supported,
            pawnio,
            selected_source: None,
            selected_enablement: None,
            fallback_reason: Some(fallback_reason.clone()),
          },
          active: None,
          inactive_reason: Some(fallback_reason),
        };
      }
    };

    match open_power_source(&candidate) {
      Ok((active, pawnio)) => Self {
        diagnostics: CpuPowerDiagnostics {
          cpu,
          amd_rapl_supported,
          pawnio,
          selected_source: Some(candidate.source),
          selected_enablement: Some(candidate.enablement),
          fallback_reason: None,
        },
        active: Some(active),
        inactive_reason: None,
      },
      Err(PawnIoInitError { discovery, reason }) => Self {
        diagnostics: CpuPowerDiagnostics {
          cpu,
          amd_rapl_supported,
          pawnio: *discovery,
          selected_source: None,
          selected_enablement: Some(candidate.enablement),
          fallback_reason: Some(reason.clone()),
        },
        active: None,
        inactive_reason: Some(reason),
      },
    }
  }

  pub(crate) fn sample(&mut self) -> Result<Option<f64>, String> {
    let Some(active) = self.active.as_mut() else {
      return Err(
        self
          .inactive_reason
          .clone()
          .unwrap_or_else(|| "CPU package power unavailable".to_string()),
      );
    };

    match active {
      ActivePowerSource::Intel { client, sampler } => {
        sampler.sample_shared(client, MSR_PKG_ENERGY_STATUS)
      }
      ActivePowerSource::Amd { client, sampler } => {
        sampler.sample_owned(client, AMD_PKG_ENERGY_STATUS)
      }
    }
  }
}

fn open_power_source(
  candidate: &CpuPowerCandidate,
) -> Result<(ActivePowerSource, PawnIoDiscovery), PawnIoInitError> {
  match candidate.source {
    CpuPowerSource::IntelRaplPackageMsr => {
      let (client, mut discovery) = open_shared_intel_msr()?;
      let sampler = match client.lock() {
        Ok(client) => RuntimePowerSampler::from_probe(
          &client,
          MSR_RAPL_POWER_UNIT,
          MSR_PKG_ENERGY_STATUS,
        ),
        Err(_) => Err(PawnIoError::SharedClientPoisoned.to_string()),
      };
      match sampler {
        Ok(sampler) => Ok((ActivePowerSource::Intel { client, sampler }, discovery)),
        Err(reason) => {
          discovery.fallback_reason = Some(reason.clone());
          Err(PawnIoInitError {
            discovery: Box::new(discovery),
            reason,
          })
        }
      }
    }
    CpuPowerSource::AmdZenRaplPackageMsr => {
      let (client, mut discovery) = PawnIoClient::open(candidate.module.clone())?;
      let sampler = match RuntimePowerSampler::from_probe(
        &client,
        AMD_RAPL_POWER_UNIT,
        AMD_PKG_ENERGY_STATUS,
      ) {
        Ok(sampler) => sampler,
        Err(reason) => {
          discovery.fallback_reason = Some(reason.clone());
          return Err(PawnIoInitError {
            discovery: Box::new(discovery),
            reason,
          });
        }
      };

      Ok((ActivePowerSource::Amd { client, sampler }, discovery))
    }
  }
}

fn format_power_unit_error(error: PowerUnitDecodeError) -> String {
  format!("CPU power unit decode failed: {error:?}")
}

#[cfg(test)]
mod tests {
  use super::*;

  fn cpu(vendor: CpuVendor, vendor_id: &str, family: u32, model: u32) -> CpuIdentity {
    CpuIdentity {
      vendor,
      vendor_id: vendor_id.to_string(),
      brand: String::new(),
      family,
      model,
    }
  }

  #[test]
  fn intel_requires_the_exact_vendor_string() {
    let candidate = cpu(CpuVendor::Intel, "GenuineIntel", 6, 0xa5);
    assert!(select_cpu_power_candidate(&candidate, None).is_ok());
    assert_eq!(
      select_cpu_power_candidate(&cpu(CpuVendor::Intel, "Intel", 6, 0xa5), None),
      Err(CpuPowerFallbackReason::UnsupportedCpuVendor(
        "Intel".to_string()
      ))
    );
  }

  #[test]
  fn intel_excludes_only_the_four_silvermont_models() {
    for model in [0x37, 0x4a, 0x5a, 0x5d] {
      assert_eq!(
        select_cpu_power_candidate(
          &cpu(CpuVendor::Intel, "GenuineIntel", 6, model),
          None
        ),
        Err(CpuPowerFallbackReason::IntelSilvermontModel(model))
      );
    }

    assert!(
      select_cpu_power_candidate(&cpu(CpuVendor::Intel, "GenuineIntel", 6, 0x36), None)
        .is_ok()
    );
    assert!(
      select_cpu_power_candidate(&cpu(CpuVendor::Intel, "GenuineIntel", 7, 0x37), None)
        .is_ok()
    );
  }

  #[test]
  fn amd_requires_family_and_cpuid_rapl_gates() {
    assert_eq!(
      select_cpu_power_candidate(
        &cpu(CpuVendor::Amd, "AuthenticAMD", 0x16, 0),
        Some(true)
      ),
      Err(CpuPowerFallbackReason::AmdFamilyUnsupported(0x16))
    );
    assert_eq!(
      select_cpu_power_candidate(
        &cpu(CpuVendor::Amd, "AuthenticAMD", 0x17, 0x01),
        Some(false)
      ),
      Err(CpuPowerFallbackReason::AmdRaplUnavailable)
    );
    assert_eq!(
      select_cpu_power_candidate(&cpu(CpuVendor::Amd, "AuthenticAMD", 0x17, 0x01), None),
      Err(CpuPowerFallbackReason::AmdRaplUnavailable)
    );
    assert_eq!(
      select_cpu_power_candidate(&cpu(CpuVendor::Amd, "AMD", 0x17, 0x01), Some(true)),
      Err(CpuPowerFallbackReason::UnsupportedCpuVendor(
        "AMD".to_string()
      ))
    );
  }

  #[test]
  fn amd_enablement_matches_the_exact_verified_and_experimental_matrix() {
    assert_eq!(amd_power_enablement(0x17, 0x01), SensorEnablement::Verified);
    assert_eq!(amd_power_enablement(0x17, 0x08), SensorEnablement::Verified);
    assert_eq!(
      amd_power_enablement(0x17, 0x02),
      SensorEnablement::Experimental
    );
    assert_eq!(amd_power_enablement(0x19, 0x21), SensorEnablement::Verified);
    assert_eq!(amd_power_enablement(0x19, 0x61), SensorEnablement::Verified);
    assert_eq!(
      amd_power_enablement(0x19, 0x01),
      SensorEnablement::Experimental
    );
    assert_eq!(amd_power_enablement(0x1a, 0x02), SensorEnablement::Verified);
    assert_eq!(
      amd_power_enablement(0x1a, 0x44),
      SensorEnablement::Experimental
    );
    assert_eq!(
      amd_power_enablement(0x1a, 0x01),
      SensorEnablement::Experimental
    );
    assert_eq!(
      amd_power_enablement(0x16, 0x01),
      SensorEnablement::Unsupported
    );
  }

  #[test]
  fn amd_candidate_uses_amdfamily17_and_exposes_enablement() {
    assert_eq!(
      select_cpu_power_candidate(
        &cpu(CpuVendor::Amd, "AuthenticAMD", 0x1a, 0x02),
        Some(true),
      ),
      Ok(CpuPowerCandidate {
        source: CpuPowerSource::AmdZenRaplPackageMsr,
        module: PawnIoModule::AmdFamily17,
        enablement: SensorEnablement::Verified,
      })
    );
  }

  #[test]
  fn runtime_sampler_reads_and_updates_baseline_before_suppressing_first_public_sample() {
    let decoder =
      PowerDecoder::from_unit_register_with_baseline(16 << 8, 0, 0.0).unwrap();
    let mut sampler = RuntimePowerSampler {
      decoder,
      clock_origin: Instant::now(),
      skip_first_public_sample: true,
    };

    assert_eq!(sampler.consume_public_reading(Ok(65_536), 1.0), Ok(None));
    assert_eq!(
      sampler.consume_public_reading(Ok(65_536), 2.0),
      Ok(Some(0.0))
    );
  }

  #[test]
  fn runtime_sampler_rebaselines_after_a_read_failure() {
    let decoder =
      PowerDecoder::from_unit_register_with_baseline(16 << 8, 0, 0.0).unwrap();
    let mut sampler = RuntimePowerSampler {
      decoder,
      clock_origin: Instant::now(),
      skip_first_public_sample: false,
    };

    assert_eq!(
      sampler.consume_reading(
        Err(PawnIoError::NoOutput {
          function: "ioctl_read_msr"
        }),
        1.0
      ),
      Err("pawnio_execute ioctl_read_msr returned no output cells".to_string())
    );
    assert_eq!(sampler.consume_reading(Ok(65_536), 2.0), Ok(None));
    assert_eq!(sampler.consume_reading(Ok(131_072), 3.0), Ok(Some(1.0)));
  }

  #[test]
  fn first_public_read_failure_returns_error_and_clears_the_baseline() {
    let decoder =
      PowerDecoder::from_unit_register_with_baseline(16 << 8, 0, 0.0).unwrap();
    let mut sampler = RuntimePowerSampler {
      decoder,
      clock_origin: Instant::now(),
      skip_first_public_sample: true,
    };

    assert_eq!(
      sampler.consume_public_reading(
        Err(PawnIoError::NoOutput {
          function: "ioctl_read_msr"
        }),
        1.0
      ),
      Err("pawnio_execute ioctl_read_msr returned no output cells".to_string())
    );
    assert_eq!(sampler.consume_public_reading(Ok(65_536), 2.0), Ok(None));
    assert_eq!(
      sampler.consume_public_reading(Ok(131_072), 3.0),
      Ok(Some(1.0))
    );
  }
}
