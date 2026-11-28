use std::fmt;
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

use super::super::client::{
  PawnIoClient, PawnIoDiscovery, PawnIoInitError, PawnIoModule, open_shared_intel_msr,
  read_shared_msr,
};
use super::amd_family_15h_16h_tctl::{
  TctlGroup, admit_identity, cpuid_package_type, is_family_15h_16h, sample_tctl,
  select_tctl_group,
};
use super::ccd_temperature::{AmdCcdTarget, is_single_physical_package};
pub use super::identity::{CpuIdentity, CpuVendor};
use super::identity::{cpuid_leaf, detect_cpu_identity};
use super::temperature_decode::{
  CpuTemperatureDecodeError, decode_amd_zen_package_temperature,
  decode_intel_package_temperature, decode_intel_package_thermal_status,
  decode_intel_temperature_target,
};
use crate::models::{CpuCcdTemperature, CpuPackageThermalStatus, SensorEnablement};
use crate::{log_debug, log_warn};

const MSR_TEMPERATURE_TARGET: u64 = 0x1a2;
const IA32_PACKAGE_THERM_STATUS: u64 = 0x1b1;
const AMD_THM_TCON_CUR_TMP: u64 = 0x0005_9800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntelThermalCapabilities {
  pub digital_temperature_sensor: bool,
  pub package_thermal_management: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuTemperatureSource {
  IntelDtsPackageMsr,
  AmdZenSmnTctl,
  /// Family 15h/16h control temperature through `AMDFamily10`, on its own
  /// Tctl scale. It is not a die temperature.
  AmdFamily15h16hTctl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuTemperatureFallbackReason {
  UnsupportedCpuVendor(String),
  IntelDtsUnavailable,
  IntelPackageThermalUnavailable,
  AmdFamilyUnsupported(u32),
  AmdModelUnsupported {
    family: u32,
    model: u32,
  },
  /// Family 15h models 00h-0Fh outside the single-node AM3r2 and C32r1
  /// packages; `None` when CPUID leaf 8000_0001h is unreadable.
  AmdPackageTypeUnsupported {
    package_type: Option<u32>,
  },
  AmdSinglePackageUnconfirmed,
}

impl fmt::Display for CpuTemperatureFallbackReason {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::UnsupportedCpuVendor(vendor) => {
        write!(f, "unsupported CPU vendor {vendor}")
      }
      Self::IntelDtsUnavailable => write!(f, "Intel digital thermal sensor unavailable"),
      Self::IntelPackageThermalUnavailable => {
        write!(f, "Intel package thermal management unavailable")
      }
      Self::AmdFamilyUnsupported(family) => {
        write!(
          f,
          "AMD family 0x{family:x} is unsupported by the PawnIO RyzenSMU path"
        )
      }
      Self::AmdModelUnsupported { family, model } => write!(
        f,
        "AMD family 0x{family:x} model 0x{model:x} has no documented Tctl register map"
      ),
      Self::AmdPackageTypeUnsupported {
        package_type: Some(package_type),
      } => write!(
        f,
        "AMD Family 15h models 00h-0Fh package type 0x{package_type:x} is not a \
         single-node AM3r2 or C32r1 package"
      ),
      Self::AmdPackageTypeUnsupported { package_type: None } => write!(
        f,
        "AMD Family 15h models 00h-0Fh package type could not be read"
      ),
      Self::AmdSinglePackageUnconfirmed => write!(
        f,
        "the AMD Family 15h/16h Tctl path requires exactly one physical package"
      ),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuPackageTemperatureError {
  Unsupported(CpuTemperatureFallbackReason),
  Unavailable {
    reason: String,
    enablement: SensorEnablement,
    thermal_status: Option<CpuPackageThermalStatus>,
  },
  Internal(String),
}

impl fmt::Display for CpuPackageTemperatureError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Unsupported(reason) => fmt::Display::fmt(reason, f),
      Self::Unavailable {
        reason,
        enablement: SensorEnablement::Experimental,
        ..
      } => write!(
        f,
        "experimental CPU package temperature attempt failed: {reason}"
      ),
      Self::Unavailable { reason, .. } | Self::Internal(reason) => f.write_str(reason),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CpuTemperatureCandidate {
  pub(crate) source: CpuTemperatureSource,
  pub(crate) module: PawnIoModule,
  pub(crate) enablement: SensorEnablement,
  /// The Family 15h/16h model group, for the `AMDFamily10` path only.
  pub(crate) tctl_group: Option<&'static TctlGroup>,
}

pub fn amd_family_enablement(family: u32) -> SensorEnablement {
  match family {
    0x17 | 0x19 => SensorEnablement::Verified,
    0x1a => SensorEnablement::Experimental,
    _ => SensorEnablement::Unsupported,
  }
}

/// Pick the CPU temperature path. `package_type` (CPUID Fn8000_0001_EBX
/// `PkgType`) and `single_package` are the Family 15h/16h CPU and topology
/// gates, in the specification's order. Each is queried only when it
/// applies, and both run before any module is loaded.
pub(crate) fn select_cpu_temperature_candidate(
  cpu: &CpuIdentity,
  intel: Option<IntelThermalCapabilities>,
  package_type: impl FnOnce() -> Option<u32>,
  single_package: impl FnOnce() -> bool,
) -> Result<CpuTemperatureCandidate, CpuTemperatureFallbackReason> {
  match cpu.vendor {
    CpuVendor::Intel => {
      let capabilities = intel.unwrap_or(IntelThermalCapabilities {
        digital_temperature_sensor: false,
        package_thermal_management: false,
      });
      if !capabilities.digital_temperature_sensor {
        return Err(CpuTemperatureFallbackReason::IntelDtsUnavailable);
      }
      if !capabilities.package_thermal_management {
        return Err(CpuTemperatureFallbackReason::IntelPackageThermalUnavailable);
      }
      Ok(CpuTemperatureCandidate {
        source: CpuTemperatureSource::IntelDtsPackageMsr,
        module: PawnIoModule::IntelMsr,
        enablement: SensorEnablement::Verified,
        tctl_group: None,
      })
    }
    CpuVendor::Amd if is_family_15h_16h(cpu.family) => {
      let group = select_tctl_group(cpu.family, cpu.model).ok_or(
        CpuTemperatureFallbackReason::AmdModelUnsupported {
          family: cpu.family,
          model: cpu.model,
        },
      )?;
      group
        .admits_package_type(package_type)
        .map_err(|package_type| {
          CpuTemperatureFallbackReason::AmdPackageTypeUnsupported { package_type }
        })?;
      if !single_package() {
        return Err(CpuTemperatureFallbackReason::AmdSinglePackageUnconfirmed);
      }
      Ok(CpuTemperatureCandidate {
        source: CpuTemperatureSource::AmdFamily15h16hTctl,
        module: PawnIoModule::AmdFamily10,
        enablement: SensorEnablement::Verified,
        tctl_group: Some(group),
      })
    }
    CpuVendor::Amd => {
      let enablement = amd_family_enablement(cpu.family);
      match enablement {
        SensorEnablement::Verified | SensorEnablement::Experimental => {
          Ok(CpuTemperatureCandidate {
            source: CpuTemperatureSource::AmdZenSmnTctl,
            module: PawnIoModule::RyzenSmu,
            enablement,
            tctl_group: None,
          })
        }
        SensorEnablement::Unsupported => Err(
          CpuTemperatureFallbackReason::AmdFamilyUnsupported(cpu.family),
        ),
      }
    }
    CpuVendor::Other => Err(CpuTemperatureFallbackReason::UnsupportedCpuVendor(
      cpu.vendor_id.clone(),
    )),
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuTemperatureDiagnostics {
  pub cpu: CpuIdentity,
  pub intel_capabilities: Option<IntelThermalCapabilities>,
  pub amd_enablement: Option<SensorEnablement>,
  pub pawnio: PawnIoDiscovery,
  pub selected_source: Option<CpuTemperatureSource>,
  pub selected_enablement: Option<SensorEnablement>,
  pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CpuPackageTemperature {
  pub temperature_celsius: f32,
  pub source: CpuTemperatureSource,
  pub thermal_status: Option<CpuPackageThermalStatus>,
}

enum ActiveCpuTemperatureSource {
  Intel {
    client: Arc<Mutex<PawnIoClient>>,
    target_celsius: u32,
    enablement: SensorEnablement,
  },
  Amd {
    client: PawnIoClient,
    tctl_offset_celsius: f32,
    enablement: SensorEnablement,
  },
  /// Admitted only after the identity gate passed on this handle.
  AmdFamily15h16h {
    client: PawnIoClient,
    group: &'static TctlGroup,
    enablement: SensorEnablement,
  },
}

pub(crate) struct CpuTemperatureSampler {
  diagnostics: CpuTemperatureDiagnostics,
  active: Option<ActiveCpuTemperatureSource>,
  inactive_error: Option<CpuPackageTemperatureError>,
  sample_failure_logged: bool,
}

static CPU_TEMPERATURE_SAMPLER: OnceLock<Mutex<CpuTemperatureSampler>> = OnceLock::new();

pub fn cpu_temperature_diagnostics() -> CpuTemperatureDiagnostics {
  let sampler = CPU_TEMPERATURE_SAMPLER.get_or_init(|| {
    let sampler = CpuTemperatureSampler::new();
    log_debug!(
      "cpu_temperature_diagnostics",
      "windows::cpu_temperature::cpu_temperature_diagnostics",
      Some(format!("{:?}", sampler.diagnostics))
    );
    Mutex::new(sampler)
  });

  sampler
    .lock()
    .map(|sampler| sampler.diagnostics.clone())
    .unwrap_or_else(|_| CpuTemperatureDiagnostics::unavailable("sampler lock poisoned"))
}

impl CpuTemperatureDiagnostics {
  fn unavailable(reason: impl Into<String>) -> Self {
    Self {
      cpu: CpuIdentity::unknown(),
      intel_capabilities: None,
      amd_enablement: None,
      pawnio: PawnIoDiscovery::unavailable("sampler unavailable"),
      selected_source: None,
      selected_enablement: None,
      fallback_reason: Some(reason.into()),
    }
  }
}

impl CpuTemperatureSampler {
  pub(crate) fn new() -> Self {
    let cpu = detect_cpu_identity();
    let intel_capabilities = (cpu.vendor == CpuVendor::Intel)
      .then(detect_intel_thermal_capabilities)
      .flatten();
    let selection = select_cpu_temperature_candidate(
      &cpu,
      intel_capabilities,
      cpuid_package_type,
      is_single_physical_package,
    );
    // Family 15h/16h enablement depends on model and package gates, so take
    // it from the selection rather than from the family alone.
    let amd_enablement = (cpu.vendor == CpuVendor::Amd).then(|| {
      selection
        .as_ref()
        .map_or(SensorEnablement::Unsupported, |candidate| {
          candidate.enablement
        })
    });

    let candidate = match selection {
      Ok(candidate) => candidate,
      Err(reason) => {
        let fallback_reason = reason.to_string();
        let mut pawnio = PawnIoDiscovery::probe_install();
        if pawnio.fallback_reason.is_none() {
          pawnio.fallback_reason =
            Some("PawnIO module not selected for unsupported CPU".to_string());
        }
        return Self {
          diagnostics: CpuTemperatureDiagnostics {
            cpu,
            intel_capabilities,
            amd_enablement,
            pawnio,
            selected_source: None,
            selected_enablement: None,
            fallback_reason: Some(fallback_reason),
          },
          active: None,
          inactive_error: Some(CpuPackageTemperatureError::Unsupported(reason)),
          sample_failure_logged: false,
        };
      }
    };

    match ActiveCpuTemperatureSource::open(&cpu, &candidate) {
      Ok((active, pawnio)) => Self {
        diagnostics: CpuTemperatureDiagnostics {
          cpu,
          intel_capabilities,
          amd_enablement,
          pawnio,
          selected_source: Some(candidate.source),
          selected_enablement: Some(candidate.enablement),
          fallback_reason: None,
        },
        active: Some(active),
        inactive_error: None,
        sample_failure_logged: false,
      },
      Err(error) => {
        let PawnIoInitError { discovery, reason } = error;
        Self {
          diagnostics: CpuTemperatureDiagnostics {
            cpu,
            intel_capabilities,
            amd_enablement,
            pawnio: *discovery,
            selected_source: None,
            selected_enablement: Some(candidate.enablement),
            fallback_reason: Some(reason.clone()),
          },
          active: None,
          inactive_error: Some(CpuPackageTemperatureError::Unavailable {
            reason,
            enablement: candidate.enablement,
            thermal_status: None,
          }),
          sample_failure_logged: false,
        }
      }
    }
  }

  pub(crate) fn sample(
    &mut self,
  ) -> Result<CpuPackageTemperature, CpuPackageTemperatureError> {
    let enablement = self
      .active
      .as_ref()
      .map(ActiveCpuTemperatureSource::enablement)
      .unwrap_or(SensorEnablement::Verified);
    let result = match self.active.as_mut() {
      Some(ActiveCpuTemperatureSource::Intel {
        client,
        target_celsius,
        enablement: _,
      }) => read_shared_msr(client, IA32_PACKAGE_THERM_STATUS)
        .map_err(|error| CpuPackageTemperatureError::Unavailable {
          reason: error.to_string(),
          enablement,
          thermal_status: None,
        })
        .and_then(|status| {
          decode_intel_package_temperature_sample(*target_celsius, status, enablement)
        }),
      Some(ActiveCpuTemperatureSource::Amd {
        client,
        tctl_offset_celsius,
        enablement: _,
      }) => client
        .read_smu_register(AMD_THM_TCON_CUR_TMP)
        .map_err(|error| error.to_string())
        .and_then(|value| {
          decode_amd_zen_package_temperature(value as u32, *tctl_offset_celsius)
            .map_err(format_decode_error)
        })
        .map(|temperature| CpuPackageTemperature {
          temperature_celsius: temperature,
          source: CpuTemperatureSource::AmdZenSmnTctl,
          thermal_status: None,
        })
        .map_err(|reason| CpuPackageTemperatureError::Unavailable {
          reason,
          enablement,
          thermal_status: None,
        }),
      Some(ActiveCpuTemperatureSource::AmdFamily15h16h { client, group, .. }) => {
        sample_tctl(client, group.map)
          .map(|tctl| CpuPackageTemperature {
            temperature_celsius: tctl,
            source: CpuTemperatureSource::AmdFamily15h16hTctl,
            thermal_status: None,
          })
          .map_err(|error| CpuPackageTemperatureError::Unavailable {
            reason: error.to_string(),
            enablement,
            thermal_status: None,
          })
      }
      None => Err(self.inactive_error.clone().unwrap_or_else(|| {
        CpuPackageTemperatureError::Internal(
          "CPU package temperature unavailable".to_string(),
        )
      })),
    };

    if let Err(reason) = &result
      && !self.sample_failure_logged
    {
      self.sample_failure_logged = true;
      log_warn!(
        "cpu_temperature_sample_failed",
        "windows::cpu_temperature::CpuTemperatureSampler::sample",
        Some(reason.to_string())
      );
    }

    result
  }

  pub(crate) fn sample_ccd(
    &self,
    target: AmdCcdTarget,
  ) -> (Option<Vec<CpuCcdTemperature>>, bool) {
    let Some(ActiveCpuTemperatureSource::Amd { client, .. }) = &self.active else {
      return (target.readings_without_module(), true);
    };

    (Some(target.read(client)), false)
  }

  #[cfg(test)]
  pub(crate) fn synthetic_unavailable() -> Self {
    Self {
      diagnostics: CpuTemperatureDiagnostics::unavailable("synthetic missing module"),
      active: None,
      inactive_error: Some(CpuPackageTemperatureError::Unavailable {
        reason: "synthetic RyzenSMU load failure".into(),
        enablement: SensorEnablement::Verified,
        thermal_status: None,
      }),
      sample_failure_logged: true,
    }
  }
}

impl ActiveCpuTemperatureSource {
  const fn enablement(&self) -> SensorEnablement {
    match self {
      Self::Intel { enablement, .. }
      | Self::Amd { enablement, .. }
      | Self::AmdFamily15h16h { enablement, .. } => *enablement,
    }
  }

  fn open(
    cpu: &CpuIdentity,
    candidate: &CpuTemperatureCandidate,
  ) -> Result<(Self, PawnIoDiscovery), PawnIoInitError> {
    match candidate.source {
      CpuTemperatureSource::IntelDtsPackageMsr => {
        let (client, mut discovery) = open_shared_intel_msr()?;
        let target_msr = match read_shared_msr(&client, MSR_TEMPERATURE_TARGET) {
          Ok(value) => value,
          Err(error) => {
            let reason = error.to_string();
            discovery.fallback_reason = Some(reason.clone());
            return Err(PawnIoInitError {
              discovery: Box::new(discovery),
              reason,
            });
          }
        };
        let target_celsius = match decode_intel_temperature_target(target_msr) {
          Ok(value) => value,
          Err(error) => {
            let reason = format_decode_error(error);
            discovery.fallback_reason = Some(reason.clone());
            return Err(PawnIoInitError {
              discovery: Box::new(discovery),
              reason,
            });
          }
        };
        Ok((
          Self::Intel {
            client,
            target_celsius,
            enablement: candidate.enablement,
          },
          discovery,
        ))
      }
      CpuTemperatureSource::AmdZenSmnTctl => {
        let (client, discovery) = PawnIoClient::open(candidate.module.clone())?;
        Ok((
          Self::Amd {
            client,
            tctl_offset_celsius: amd_tctl_offset_celsius(&cpu.brand),
            enablement: candidate.enablement,
          },
          discovery,
        ))
      }
      CpuTemperatureSource::AmdFamily15h16hTctl => {
        let Some(group) = candidate.tctl_group else {
          return Err(PawnIoInitError {
            discovery: Box::new(PawnIoDiscovery::probe_install()),
            reason: "AMD Family 15h/16h candidate has no model group".to_string(),
          });
        };
        let (client, mut discovery) = PawnIoClient::open(candidate.module.clone())?;
        // Dropping the client on a failed gate closes the handle unread.
        if let Err(error) = admit_identity(&client, group) {
          let reason = error.to_string();
          discovery.fallback_reason = Some(reason.clone());
          return Err(PawnIoInitError {
            discovery: Box::new(discovery),
            reason,
          });
        }
        Ok((
          Self::AmdFamily15h16h {
            client,
            group,
            enablement: candidate.enablement,
          },
          discovery,
        ))
      }
    }
  }
}

fn decode_intel_package_temperature_sample(
  target_celsius: u32,
  package_therm_status: u64,
  enablement: SensorEnablement,
) -> Result<CpuPackageTemperature, CpuPackageTemperatureError> {
  let thermal_status = Some(decode_intel_package_thermal_status(package_therm_status));
  decode_intel_package_temperature(target_celsius, package_therm_status)
    .map(|temperature| CpuPackageTemperature {
      temperature_celsius: temperature,
      source: CpuTemperatureSource::IntelDtsPackageMsr,
      thermal_status,
    })
    .map_err(|error| CpuPackageTemperatureError::Unavailable {
      reason: format_decode_error(error),
      enablement,
      thermal_status,
    })
}

fn format_decode_error(error: CpuTemperatureDecodeError) -> String {
  format!("CPU temperature decode failed: {error:?}")
}

fn detect_intel_thermal_capabilities() -> Option<IntelThermalCapabilities> {
  cpuid_leaf(6).map(|leaf| IntelThermalCapabilities {
    digital_temperature_sensor: (leaf.eax & 0x1) != 0,
    package_thermal_management: (leaf.eax & (1 << 6)) != 0,
  })
}

fn amd_tctl_offset_celsius(brand: &str) -> f32 {
  let brand = brand.to_ascii_uppercase();
  if brand.contains("RYZEN 7 1700X") || brand.contains("RYZEN 7 1800X") {
    20.0
  } else if brand.contains("RYZEN 7 2700X") {
    10.0
  } else {
    0.0
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn cpu(vendor: CpuVendor, vendor_id: &str, family: u32) -> CpuIdentity {
    CpuIdentity {
      vendor,
      vendor_id: vendor_id.to_string(),
      brand: String::new(),
      family,
      model: 0,
    }
  }

  #[test]
  fn amd_family_17h_and_19h_are_verified() {
    assert_eq!(amd_family_enablement(0x17), SensorEnablement::Verified);
    assert_eq!(amd_family_enablement(0x19), SensorEnablement::Verified);
  }

  #[test]
  fn amd_family_1ah_is_experimental() {
    assert_eq!(amd_family_enablement(0x1a), SensorEnablement::Experimental);
  }

  fn amd(family: u32, model: u32) -> CpuIdentity {
    CpuIdentity {
      model,
      ..cpu(CpuVendor::Amd, "AuthenticAMD", family)
    }
  }

  fn topology_not_queried() -> bool {
    panic!("the package topology must not be queried for this CPU")
  }

  fn package_type_not_queried() -> Option<u32> {
    panic!("the package type must not be queried for this CPU")
  }

  /// CPUID Fn8000_0001_EBX[31:28] `PkgType` `0001b` (AM3r2), from the spec's
  /// Detection table (rev 2).
  fn am3r2() -> Option<u32> {
    Some(0b0001)
  }

  fn select_without_15h_16h_probes(
    cpu: &CpuIdentity,
    intel: Option<IntelThermalCapabilities>,
  ) -> Result<CpuTemperatureCandidate, CpuTemperatureFallbackReason> {
    select_cpu_temperature_candidate(
      cpu,
      intel,
      package_type_not_queried,
      topology_not_queried,
    )
  }

  #[test]
  fn intel_candidate_requires_dts_and_package_thermal_management() {
    let cpu = cpu(CpuVendor::Intel, "GenuineIntel", 6);
    assert_eq!(
      select_without_15h_16h_probes(
        &cpu,
        Some(IntelThermalCapabilities {
          digital_temperature_sensor: true,
          package_thermal_management: true,
        }),
      ),
      Ok(CpuTemperatureCandidate {
        source: CpuTemperatureSource::IntelDtsPackageMsr,
        module: PawnIoModule::IntelMsr,
        enablement: SensorEnablement::Verified,
        tctl_group: None,
      })
    );

    assert_eq!(
      select_without_15h_16h_probes(
        &cpu,
        Some(IntelThermalCapabilities {
          digital_temperature_sensor: true,
          package_thermal_management: false,
        }),
      ),
      Err(CpuTemperatureFallbackReason::IntelPackageThermalUnavailable)
    );
  }

  #[test]
  fn amd_candidate_classifies_verified_experimental_and_unsupported_families() {
    assert_eq!(
      select_without_15h_16h_probes(&amd(0x19, 0x21), None),
      Ok(CpuTemperatureCandidate {
        source: CpuTemperatureSource::AmdZenSmnTctl,
        module: PawnIoModule::RyzenSmu,
        enablement: SensorEnablement::Verified,
        tctl_group: None,
      })
    );

    assert_eq!(
      select_without_15h_16h_probes(&amd(0x1a, 0x44), None),
      Ok(CpuTemperatureCandidate {
        source: CpuTemperatureSource::AmdZenSmnTctl,
        module: PawnIoModule::RyzenSmu,
        enablement: SensorEnablement::Experimental,
        tctl_group: None,
      })
    );
    assert_eq!(
      select_without_15h_16h_probes(&amd(0x14, 0x02), None),
      Err(CpuTemperatureFallbackReason::AmdFamilyUnsupported(0x14))
    );
  }

  #[test]
  fn amd_family_15h_16h_groups_select_the_amd_family10_path_on_one_package() {
    // One model inside each group of the spec's Detection table (rev 2).
    for (family, model) in [
      (0x15, 0x02),
      (0x15, 0x13),
      (0x15, 0x30),
      (0x15, 0x65),
      (0x15, 0x70),
      (0x16, 0x00),
      (0x16, 0x30),
    ] {
      let candidate =
        select_cpu_temperature_candidate(&amd(family, model), None, am3r2, || true)
          .unwrap_or_else(|reason| panic!("{family:#x}/{model:#x}: {reason}"));

      assert_eq!(candidate.source, CpuTemperatureSource::AmdFamily15h16hTctl);
      assert_eq!(candidate.module, PawnIoModule::AmdFamily10);
      assert_eq!(candidate.enablement, SensorEnablement::Verified);
      assert_eq!(candidate.tctl_group, select_tctl_group(family, model));
      assert!(candidate.tctl_group.is_some());
    }
  }

  #[test]
  fn family_15h_models_00h_0fh_admit_am3r2_and_c32r1_packages() {
    for package_type in [0b0001, 0b0101] {
      let candidate = select_cpu_temperature_candidate(
        &amd(0x15, 0x02),
        None,
        || Some(package_type),
        || true,
      );
      assert_eq!(
        candidate.map(|candidate| candidate.source),
        Ok(CpuTemperatureSource::AmdFamily15h16hTctl),
        "{package_type:04b}"
      );
    }
  }

  #[test]
  fn family_15h_models_00h_0fh_reject_g34r1_and_reserved_packages_before_topology() {
    // 0011b is G34r1 (dual-node); 0000b and 1111b are Reserved.
    for package_type in [Some(0b0011), Some(0b0000), Some(0b1111), None] {
      assert_eq!(
        select_cpu_temperature_candidate(
          &amd(0x15, 0x01),
          None,
          || package_type,
          topology_not_queried,
        ),
        Err(CpuTemperatureFallbackReason::AmdPackageTypeUnsupported { package_type })
      );
    }
  }

  #[test]
  fn other_family_15h_16h_groups_do_not_read_the_package_type() {
    for (family, model) in [(0x15, 0x13), (0x15, 0x65), (0x16, 0x30)] {
      assert!(
        select_cpu_temperature_candidate(
          &amd(family, model),
          None,
          package_type_not_queried,
          || true,
        )
        .is_ok()
      );
    }
  }

  #[test]
  fn amd_family_15h_16h_rejects_more_than_one_physical_package() {
    for (family, model) in [(0x15, 0x02), (0x15, 0x70), (0x16, 0x00)] {
      assert_eq!(
        select_cpu_temperature_candidate(&amd(family, model), None, am3r2, || false),
        Err(CpuTemperatureFallbackReason::AmdSinglePackageUnconfirmed)
      );
    }
  }

  #[test]
  fn undocumented_family_15h_16h_models_are_unsupported_before_topology_or_module() {
    for (family, model) in [
      (0x15, 0x20),
      (0x15, 0x40),
      (0x15, 0x80),
      (0x16, 0x10),
      (0x16, 0x40),
    ] {
      assert_eq!(
        select_without_15h_16h_probes(&amd(family, model), None),
        Err(CpuTemperatureFallbackReason::AmdModelUnsupported { family, model })
      );
    }
  }

  #[test]
  fn a_non_amd_vendor_never_reaches_the_family_15h_16h_path() {
    let other = CpuIdentity {
      model: 0x02,
      ..cpu(CpuVendor::Other, "CentaurHauls", 0x15)
    };

    assert_eq!(
      select_without_15h_16h_probes(&other, None),
      Err(CpuTemperatureFallbackReason::UnsupportedCpuVendor(
        "CentaurHauls".to_string()
      ))
    );
  }

  #[test]
  fn family_15h_16h_fallback_reasons_suppress_component_guidance() {
    for reason in [
      CpuTemperatureFallbackReason::AmdModelUnsupported {
        family: 0x15,
        model: 0x20,
      },
      CpuTemperatureFallbackReason::AmdPackageTypeUnsupported {
        package_type: Some(0b0011),
      },
      CpuTemperatureFallbackReason::AmdSinglePackageUnconfirmed,
    ] {
      let sample = crate::platform::windows::sensors::build_temperature_sample(
        Err(CpuPackageTemperatureError::Unsupported(reason)),
        Vec::new(),
      );
      assert!(matches!(
        sample.availability,
        crate::models::SensorAvailability::Unsupported { .. }
      ));
      assert!(sample.guidance_candidates.is_empty());
    }
  }

  #[test]
  fn experimental_failure_is_identified_only_in_the_error() {
    let error = CpuPackageTemperatureError::Unavailable {
      reason: "CPU temperature decode failed".to_string(),
      enablement: SensorEnablement::Experimental,
      thermal_status: None,
    };

    assert_eq!(
      error.to_string(),
      "experimental CPU package temperature attempt failed: CPU temperature decode failed"
    );

    let verified_error = CpuPackageTemperatureError::Unavailable {
      reason: "CPU temperature decode failed".to_string(),
      enablement: SensorEnablement::Verified,
      thermal_status: None,
    };
    assert_eq!(verified_error.to_string(), "CPU temperature decode failed");
  }

  #[test]
  fn intel_temperature_decode_failure_preserves_live_thermal_status() {
    let package_therm_status = (127 << 16) | (1 << 0) | (1 << 2) | (1 << 10);
    let error = decode_intel_package_temperature_sample(
      100,
      package_therm_status,
      SensorEnablement::Verified,
    )
    .expect_err("the temperature readout should be invalid");

    assert_eq!(
      error,
      CpuPackageTemperatureError::Unavailable {
        reason: "CPU temperature decode failed: ImplausibleIntelPackageTemperature { temperature: -27, target: 100 }".to_string(),
        enablement: SensorEnablement::Verified,
        thermal_status: Some(CpuPackageThermalStatus {
          thermal_status: true,
          prochot_or_forcepr_asserted: true,
          power_limitation_status: true,
        }),
      }
    );
  }

  #[test]
  fn amd_tctl_offset_is_limited_to_ready_phase1_models() {
    assert_eq!(
      amd_tctl_offset_celsius("AMD Ryzen 7 1700X Eight-Core"),
      20.0
    );
    assert_eq!(
      amd_tctl_offset_celsius("AMD Ryzen 7 1800X Eight-Core"),
      20.0
    );
    assert_eq!(
      amd_tctl_offset_celsius("AMD Ryzen 7 2700X Eight-Core"),
      10.0
    );
    assert_eq!(amd_tctl_offset_celsius("AMD Ryzen 9 3900X"), 0.0);
  }
}
