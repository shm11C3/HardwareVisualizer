use std::fmt;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use super::cpu_temperature_decode::{
  CpuTemperatureDecodeError, decode_amd_zen_package_temperature,
  decode_intel_package_temperature, decode_intel_temperature_target,
};
use super::pawn_io::{
  ACCESS_PCI_MUTEX, NamedMutex, PawnIoClient, PawnIoDiscovery, PawnIoInitError,
  PawnIoModule,
};
use crate::{log_debug, log_warn};

const PAWNIO_MUTEX_TIMEOUT: Duration = Duration::from_millis(50);
const MSR_TEMPERATURE_TARGET: u64 = 0x1a2;
const IA32_PACKAGE_THERM_STATUS: u64 = 0x1b1;
const AMD_THM_TCON_CUR_TMP: u64 = 0x0005_9800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVendor {
  Intel,
  Amd,
  Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuIdentity {
  pub vendor: CpuVendor,
  pub vendor_id: String,
  pub brand: String,
  pub family: u32,
  pub model: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntelThermalCapabilities {
  pub digital_temperature_sensor: bool,
  pub package_thermal_management: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmdFamilyEnablement {
  pub recognized_by_pawnio_module: bool,
  pub enabled_by_spec: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuTemperatureSource {
  IntelDtsPackageMsr,
  AmdZenSmnTctl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuTemperatureFallbackReason {
  UnsupportedCpuVendor(String),
  IntelDtsUnavailable,
  IntelPackageThermalUnavailable,
  AmdFamilyDisabled(u32),
  AmdFamilyUnsupported(u32),
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
      Self::AmdFamilyDisabled(family) => {
        write!(f, "AMD family 0x{family:x} is disabled by the ready spec")
      }
      Self::AmdFamilyUnsupported(family) => {
        write!(
          f,
          "AMD family 0x{family:x} is unsupported by the ready spec"
        )
      }
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CpuTemperatureCandidate {
  pub(crate) source: CpuTemperatureSource,
  pub(crate) module: PawnIoModule,
}

pub fn amd_family_enablement(family: u32) -> AmdFamilyEnablement {
  match family {
    0x17 | 0x19 => AmdFamilyEnablement {
      recognized_by_pawnio_module: true,
      enabled_by_spec: true,
    },
    0x1a => AmdFamilyEnablement {
      recognized_by_pawnio_module: true,
      enabled_by_spec: false,
    },
    _ => AmdFamilyEnablement {
      recognized_by_pawnio_module: false,
      enabled_by_spec: false,
    },
  }
}

pub(crate) fn select_cpu_temperature_candidate(
  cpu: &CpuIdentity,
  intel: Option<IntelThermalCapabilities>,
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
      })
    }
    CpuVendor::Amd => {
      let enablement = amd_family_enablement(cpu.family);
      if enablement.enabled_by_spec {
        Ok(CpuTemperatureCandidate {
          source: CpuTemperatureSource::AmdZenSmnTctl,
          module: PawnIoModule::RyzenSmu,
        })
      } else if enablement.recognized_by_pawnio_module {
        Err(CpuTemperatureFallbackReason::AmdFamilyDisabled(cpu.family))
      } else {
        Err(CpuTemperatureFallbackReason::AmdFamilyUnsupported(
          cpu.family,
        ))
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
  pub amd_enablement: Option<AmdFamilyEnablement>,
  pub pawnio: PawnIoDiscovery,
  pub selected_source: Option<CpuTemperatureSource>,
  pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CpuPackageTemperature {
  pub temperature_celsius: f32,
  pub source: CpuTemperatureSource,
}

enum ActiveCpuTemperatureSource {
  Intel {
    client: PawnIoClient,
    target_celsius: u32,
  },
  Amd {
    client: PawnIoClient,
    tctl_offset_celsius: f32,
  },
}

struct CpuTemperatureSampler {
  diagnostics: CpuTemperatureDiagnostics,
  active: Option<ActiveCpuTemperatureSource>,
  sample_failure_logged: bool,
}

static CPU_TEMPERATURE_SAMPLER: OnceLock<Mutex<CpuTemperatureSampler>> = OnceLock::new();

pub fn sample_cpu_package_temperature() -> Result<CpuPackageTemperature, String> {
  let sampler = CPU_TEMPERATURE_SAMPLER.get_or_init(|| {
    let sampler = CpuTemperatureSampler::new();
    log_debug!(
      "cpu_temperature_diagnostics",
      "windows::cpu_temperature::sample_cpu_package_temperature",
      Some(format!("{:?}", sampler.diagnostics))
    );
    Mutex::new(sampler)
  });

  sampler
    .lock()
    .map_err(|_| "CPU temperature sampler lock poisoned".to_string())?
    .sample()
}

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
      fallback_reason: Some(reason.into()),
    }
  }
}

impl CpuTemperatureSampler {
  fn new() -> Self {
    let cpu = detect_cpu_identity();
    let intel_capabilities = (cpu.vendor == CpuVendor::Intel)
      .then(detect_intel_thermal_capabilities)
      .flatten();
    let amd_enablement =
      (cpu.vendor == CpuVendor::Amd).then(|| amd_family_enablement(cpu.family));

    let candidate = match select_cpu_temperature_candidate(&cpu, intel_capabilities) {
      Ok(candidate) => candidate,
      Err(reason) => {
        let reason = reason.to_string();
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
            fallback_reason: Some(reason),
          },
          active: None,
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
          fallback_reason: None,
        },
        active: Some(active),
        sample_failure_logged: false,
      },
      Err(error) => Self {
        diagnostics: CpuTemperatureDiagnostics {
          cpu,
          intel_capabilities,
          amd_enablement,
          pawnio: *error.discovery,
          selected_source: None,
          fallback_reason: Some(error.reason),
        },
        active: None,
        sample_failure_logged: false,
      },
    }
  }

  fn sample(&mut self) -> Result<CpuPackageTemperature, String> {
    let result = match self.active.as_mut() {
      Some(ActiveCpuTemperatureSource::Intel {
        client,
        target_celsius,
      }) => client
        .read_msr(IA32_PACKAGE_THERM_STATUS)
        .and_then(|status| {
          decode_intel_package_temperature(*target_celsius, status)
            .map_err(format_decode_error)
        })
        .map(|temperature| CpuPackageTemperature {
          temperature_celsius: temperature,
          source: CpuTemperatureSource::IntelDtsPackageMsr,
        }),
      Some(ActiveCpuTemperatureSource::Amd {
        client,
        tctl_offset_celsius,
      }) => match NamedMutex::acquire(ACCESS_PCI_MUTEX, PAWNIO_MUTEX_TIMEOUT) {
        Ok(_mutex) => client
          .read_smu_register(AMD_THM_TCON_CUR_TMP)
          .and_then(|value| {
            decode_amd_zen_package_temperature(value as u32, *tctl_offset_celsius)
              .map_err(format_decode_error)
          })
          .map(|temperature| CpuPackageTemperature {
            temperature_celsius: temperature,
            source: CpuTemperatureSource::AmdZenSmnTctl,
          }),
        Err(reason) => Err(reason),
      },
      None => Err(
        self
          .diagnostics
          .fallback_reason
          .clone()
          .unwrap_or_else(|| "CPU package temperature unavailable".to_string()),
      ),
    };

    if let Err(reason) = &result
      && !self.sample_failure_logged
    {
      self.sample_failure_logged = true;
      log_warn!(
        "cpu_temperature_sample_failed",
        "windows::cpu_temperature::CpuTemperatureSampler::sample",
        Some(reason.clone())
      );
    }

    result
  }
}

impl ActiveCpuTemperatureSource {
  fn open(
    cpu: &CpuIdentity,
    candidate: &CpuTemperatureCandidate,
  ) -> Result<(Self, PawnIoDiscovery), PawnIoInitError> {
    let (client, mut discovery) = PawnIoClient::open(candidate.module.clone())?;
    let active = match candidate.source {
      CpuTemperatureSource::IntelDtsPackageMsr => {
        let target_msr = match client.read_msr(MSR_TEMPERATURE_TARGET) {
          Ok(value) => value,
          Err(reason) => {
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
        Self::Intel {
          client,
          target_celsius,
        }
      }
      CpuTemperatureSource::AmdZenSmnTctl => Self::Amd {
        client,
        tctl_offset_celsius: amd_tctl_offset_celsius(&cpu.brand),
      },
    };

    Ok((active, discovery))
  }
}

fn format_decode_error(error: CpuTemperatureDecodeError) -> String {
  format!("CPU temperature decode failed: {error:?}")
}

fn detect_cpu_identity() -> CpuIdentity {
  detect_cpu_identity_x86().unwrap_or_else(CpuIdentity::unknown)
}

impl CpuIdentity {
  fn unknown() -> Self {
    Self {
      vendor: CpuVendor::Other,
      vendor_id: "unknown".to_string(),
      brand: String::new(),
      family: 0,
      model: 0,
    }
  }
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

#[derive(Debug, Clone, Copy)]
struct CpuidLeaf {
  eax: u32,
  ebx: u32,
  ecx: u32,
  edx: u32,
}

#[cfg(target_arch = "x86_64")]
fn cpuid_leaf(leaf: u32) -> Option<CpuidLeaf> {
  use core::arch::x86_64::__cpuid;
  let max_leaf = if leaf >= 0x8000_0000 {
    __cpuid(0x8000_0000).eax
  } else {
    __cpuid(0).eax
  };
  (leaf <= max_leaf).then(|| {
    let result = __cpuid(leaf);
    CpuidLeaf {
      eax: result.eax,
      ebx: result.ebx,
      ecx: result.ecx,
      edx: result.edx,
    }
  })
}

#[cfg(target_arch = "x86")]
fn cpuid_leaf(leaf: u32) -> Option<CpuidLeaf> {
  use core::arch::x86::__cpuid;
  let max_leaf = if leaf >= 0x8000_0000 {
    __cpuid(0x8000_0000).eax
  } else {
    __cpuid(0).eax
  };
  (leaf <= max_leaf).then(|| {
    let result = __cpuid(leaf);
    CpuidLeaf {
      eax: result.eax,
      ebx: result.ebx,
      ecx: result.ecx,
      edx: result.edx,
    }
  })
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn cpuid_leaf(_leaf: u32) -> Option<CpuidLeaf> {
  None
}

#[cfg(target_arch = "x86_64")]
fn cpuid_leaf_unchecked(leaf: u32) -> CpuidLeaf {
  use core::arch::x86_64::__cpuid;
  let result = __cpuid(leaf);
  CpuidLeaf {
    eax: result.eax,
    ebx: result.ebx,
    ecx: result.ecx,
    edx: result.edx,
  }
}

#[cfg(target_arch = "x86")]
fn cpuid_leaf_unchecked(leaf: u32) -> CpuidLeaf {
  use core::arch::x86::__cpuid;
  let result = __cpuid(leaf);
  CpuidLeaf {
    eax: result.eax,
    ebx: result.ebx,
    ecx: result.ecx,
    edx: result.edx,
  }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn cpuid_leaf_unchecked(_leaf: u32) -> CpuidLeaf {
  CpuidLeaf {
    eax: 0,
    ebx: 0,
    ecx: 0,
    edx: 0,
  }
}

fn detect_cpu_identity_x86() -> Option<CpuIdentity> {
  let leaf0 = cpuid_leaf(0)?;
  let vendor_id = vendor_id_from_leaf0(leaf0);
  let leaf1 = cpuid_leaf(1)?;
  let (family, model) = effective_family_model(leaf1.eax);
  let brand = cpu_brand_string();
  let vendor = match vendor_id.as_str() {
    "GenuineIntel" => CpuVendor::Intel,
    "AuthenticAMD" => CpuVendor::Amd,
    _ => CpuVendor::Other,
  };

  Some(CpuIdentity {
    vendor,
    vendor_id,
    brand,
    family,
    model,
  })
}

fn vendor_id_from_leaf0(leaf: CpuidLeaf) -> String {
  let mut bytes = Vec::with_capacity(12);
  bytes.extend_from_slice(&leaf.ebx.to_le_bytes());
  bytes.extend_from_slice(&leaf.edx.to_le_bytes());
  bytes.extend_from_slice(&leaf.ecx.to_le_bytes());
  String::from_utf8_lossy(&bytes).trim().to_string()
}

fn cpu_brand_string() -> String {
  let Some(extended) = cpuid_leaf(0x8000_0000) else {
    return String::new();
  };
  if extended.eax < 0x8000_0004 {
    return String::new();
  }

  let mut bytes = Vec::with_capacity(48);
  for leaf in 0x8000_0002..=0x8000_0004 {
    let result = cpuid_leaf_unchecked(leaf);
    bytes.extend_from_slice(&result.eax.to_le_bytes());
    bytes.extend_from_slice(&result.ebx.to_le_bytes());
    bytes.extend_from_slice(&result.ecx.to_le_bytes());
    bytes.extend_from_slice(&result.edx.to_le_bytes());
  }

  String::from_utf8_lossy(&bytes)
    .trim_matches(char::from(0))
    .trim()
    .to_string()
}

fn effective_family_model(eax: u32) -> (u32, u32) {
  let base_family = (eax >> 8) & 0x0f;
  let base_model = (eax >> 4) & 0x0f;
  let extended_family = (eax >> 20) & 0xff;
  let extended_model = (eax >> 16) & 0x0f;

  let family = if base_family == 0x0f {
    base_family + extended_family
  } else {
    base_family
  };
  let model = if base_family == 0x06 || base_family == 0x0f {
    base_model + (extended_model << 4)
  } else {
    base_model
  };

  (family, model)
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
  fn amd_family_17h_and_19h_are_enabled() {
    assert_eq!(
      amd_family_enablement(0x17),
      AmdFamilyEnablement {
        recognized_by_pawnio_module: true,
        enabled_by_spec: true
      }
    );
    assert_eq!(
      amd_family_enablement(0x19),
      AmdFamilyEnablement {
        recognized_by_pawnio_module: true,
        enabled_by_spec: true
      }
    );
  }

  #[test]
  fn amd_family_1ah_is_recognized_but_disabled() {
    assert_eq!(
      amd_family_enablement(0x1a),
      AmdFamilyEnablement {
        recognized_by_pawnio_module: true,
        enabled_by_spec: false
      }
    );
  }

  #[test]
  fn intel_candidate_requires_dts_and_package_thermal_management() {
    let cpu = cpu(CpuVendor::Intel, "GenuineIntel", 6);
    assert_eq!(
      select_cpu_temperature_candidate(
        &cpu,
        Some(IntelThermalCapabilities {
          digital_temperature_sensor: true,
          package_thermal_management: true,
        }),
      ),
      Ok(CpuTemperatureCandidate {
        source: CpuTemperatureSource::IntelDtsPackageMsr,
        module: PawnIoModule::IntelMsr,
      })
    );

    assert_eq!(
      select_cpu_temperature_candidate(
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
  fn amd_candidate_accepts_only_enabled_families() {
    assert_eq!(
      select_cpu_temperature_candidate(&cpu(CpuVendor::Amd, "AuthenticAMD", 0x19), None),
      Ok(CpuTemperatureCandidate {
        source: CpuTemperatureSource::AmdZenSmnTctl,
        module: PawnIoModule::RyzenSmu,
      })
    );

    assert_eq!(
      select_cpu_temperature_candidate(&cpu(CpuVendor::Amd, "AuthenticAMD", 0x1a), None),
      Err(CpuTemperatureFallbackReason::AmdFamilyDisabled(0x1a))
    );
    assert_eq!(
      select_cpu_temperature_candidate(&cpu(CpuVendor::Amd, "AuthenticAMD", 0x16), None),
      Err(CpuTemperatureFallbackReason::AmdFamilyUnsupported(0x16))
    );
  }

  #[test]
  fn effective_family_model_adds_extended_family_for_amd_zen() {
    let eax = (0x8 << 20) | (0x1 << 16) | (0xf << 8) | (0x1 << 4);

    assert_eq!(effective_family_model(eax), (0x17, 0x11));
  }

  #[test]
  fn effective_family_model_adds_extended_model_for_intel_family_6() {
    let eax = (0xa << 16) | (0x6 << 8) | (0x5 << 4);

    assert_eq!(effective_family_model(eax), (0x6, 0xa5));
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
