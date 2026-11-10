use nvml_wrapper::{
  Device, Nvml,
  enum_wrappers::device::TemperatureSensor,
  error::{NvmlError, nvml_try},
};
use std::collections::HashMap;
use tokio::{sync::OnceCell, task::spawn_blocking};

use super::drm_sys;

#[derive(Debug, Default, PartialEq)]
pub(crate) struct NvidiaReadings {
  pub(crate) usage: Option<f32>,
  pub(crate) temperature: Option<f32>,
  pub(crate) dedicated_memory_kb: Option<f32>,
}

impl NvidiaReadings {
  fn from_nvml(
    usage_percent: Option<u32>,
    temperature_c: Option<u32>,
    memory_bytes: Option<(u64, u64)>,
  ) -> Option<Self> {
    let readings = Self {
      usage: usage_percent
        .filter(|&value| value <= 100)
        .map(|value| value as f32),
      temperature: temperature_c
        .filter(|&value| value != u32::MAX)
        .map(|value| value as f32),
      dedicated_memory_kb: memory_bytes
        .filter(|&(used, total)| total > 0 && used <= total)
        .map(|(used, _)| used as f32 / 1024.0),
    };
    (readings != Self::default()).then_some(readings)
  }
}

/// Look up only the NVIDIA DRM adapters requested by the live collector.
pub(crate) async fn sample(
  pci_addresses: Vec<String>,
) -> HashMap<String, NvidiaReadings> {
  if pci_addresses.is_empty() {
    return HashMap::new();
  }

  static NVML: OnceCell<Nvml> = OnceCell::const_new();
  // In the opt-in MPS view, multiple locality domains can share a physical
  // PCI address. NVML returns the first domain's handle and memory readings.
  let mps_enumeration = std::env::var("NVML_MPS_DEVICE_ENUMERATION").ok();
  if !physical_nvml_view(mps_enumeration.as_deref()) {
    return HashMap::new();
  }
  // Keep successful initialization. Retry failures so a driver that becomes
  // available later can provide readings without restarting the app.
  let Ok(nvml) = NVML
    .get_or_try_init(|| async {
      spawn_blocking(Nvml::init)
        .await
        .ok()
        .and_then(Result::ok)
        .ok_or(())
    })
    .await
  else {
    return HashMap::new();
  };

  spawn_blocking(move || {
    read_adapters(pci_addresses, drm_sys::pci_runtime_suspended, |address| {
      let device = nvml.device_by_pci_bus_id(address).ok()?;
      NvidiaReadings::from_nvml(
        device.utilization_rates().ok().map(|value| value.gpu),
        device.temperature(TemperatureSensor::Gpu).ok(),
        read_memory(&device),
      )
    })
  })
  .await
  .unwrap_or_default()
}

fn physical_nvml_view(mps_enumeration: Option<&str>) -> bool {
  mps_enumeration != Some("1")
}

fn read_adapters(
  pci_addresses: Vec<String>,
  mut is_suspended: impl FnMut(&str) -> bool,
  mut read: impl FnMut(&str) -> Option<NvidiaReadings>,
) -> HashMap<String, NvidiaReadings> {
  pci_addresses
    .into_iter()
    .filter_map(|address| {
      // Initialization and earlier lookups can outlast the DRM snapshot.
      if is_suspended(&address) {
        return None;
      }
      read(&address).map(|readings| (address, readings))
    })
    .collect()
}

fn read_memory(device: &Device<'_>) -> Option<(u64, u64)> {
  memory_with_legacy_fallback(
    device
      .memory_info()
      .map(|memory| (memory.used, memory.total)),
    || {
      let read = device.nvml().lib().nvmlDeviceGetMemoryInfo.as_ref().ok()?;
      let mut memory = std::mem::MaybeUninit::zeroed();
      // SAFETY: The handle belongs to this NVML instance and the generated
      // binding accepts the zeroed v1 integer-only memory record.
      let memory = unsafe {
        nvml_try(read(device.handle(), memory.as_mut_ptr())).ok()?;
        memory.assume_init()
      };
      Some((memory.used, memory.total))
    },
  )
}

fn memory_with_legacy_fallback(
  modern: Result<(u64, u64), NvmlError>,
  legacy: impl FnOnce() -> Option<(u64, u64)>,
) -> Option<(u64, u64)> {
  match modern {
    Ok(memory) => Some(memory),
    // Drivers before R510 only export the v1 entry point. An error from
    // an available v2 query still describes that device's availability.
    Err(NvmlError::FailedToLoadSymbol(_)) => legacy(),
    Err(_) => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn refuses_pci_attribution_in_the_opt_in_mps_view() {
    assert!(!physical_nvml_view(Some("1")));
    for physical_view in [None, Some(""), Some("0")] {
      assert!(physical_nvml_view(physical_view));
    }
  }

  #[test]
  fn keeps_partial_readings_and_distinguishes_idle_from_unavailable() {
    assert_eq!(
      NvidiaReadings::from_nvml(Some(0), None, Some((4096, 8192))),
      Some(NvidiaReadings {
        usage: Some(0.0),
        temperature: None,
        dedicated_memory_kb: Some(4.0),
      })
    );
    assert_eq!(
      NvidiaReadings::from_nvml(None, Some(68), Some((0, 8192))),
      Some(NvidiaReadings {
        usage: None,
        temperature: Some(68.0),
        dedicated_memory_kb: Some(0.0),
      })
    );
    assert_eq!(NvidiaReadings::from_nvml(None, None, None), None);
  }

  #[test]
  fn rejects_out_of_range_and_unavailable_values_independently() {
    assert_eq!(
      NvidiaReadings::from_nvml(Some(101), Some(u32::MAX), Some((9, 8))),
      None
    );
    assert_eq!(
      NvidiaReadings::from_nvml(Some(100), Some(0), Some((0, 0))),
      Some(NvidiaReadings {
        usage: Some(100.0),
        temperature: Some(0.0),
        dedicated_memory_kb: None,
      })
    );
  }

  #[test]
  fn one_failed_device_preserves_sibling_results_by_full_pci_address() {
    // Synthetic devices with the same slot on separate PCI domains and a
    // nonzero PCI function. NVML lookup takes the complete DRM address.
    let samples = read_adapters(
      ["0001:01:00.0", "0000:01:00.0", "0000:01:00.1"]
        .map(str::to_string)
        .to_vec(),
      |_| false,
      |address| match address {
        "0000:01:00.0" => NvidiaReadings::from_nvml(Some(12), Some(51), None),
        "0000:01:00.1" => NvidiaReadings::from_nvml(None, Some(62), None),
        "0001:01:00.0" => None,
        _ => unreachable!(),
      },
    );

    assert_eq!(samples.len(), 2);
    assert_eq!(samples["0000:01:00.0"].usage, Some(12.0));
    assert_eq!(samples["0000:01:00.1"].temperature, Some(62.0));
    assert!(!samples.contains_key("0001:01:00.0"));
  }

  #[test]
  fn skips_a_card_that_suspended_after_selection() {
    let addresses = vec!["0000:01:00.0".into(), "0000:02:00.0".into()];
    let mut queried = Vec::new();
    let samples = read_adapters(
      addresses,
      |address| address == "0000:01:00.0",
      |address| {
        queried.push(address.to_owned());
        NvidiaReadings::from_nvml(Some(12), None, None)
      },
    );

    assert_eq!(queried, ["0000:02:00.0"]);
    assert_eq!(samples.len(), 1);
    assert_eq!(samples["0000:02:00.0"].usage, Some(12.0));
  }

  #[test]
  fn uses_legacy_memory_only_if_the_modern_symbol_is_missing() {
    let legacy = || Some((4096, 8192));
    let missing = || Err(NvmlError::FailedToLoadSymbol("synthetic missing v2".into()));
    let unexpected_fallback = || panic!("v2 was available");

    assert_eq!(
      memory_with_legacy_fallback(Ok((0, 8192)), unexpected_fallback),
      Some((0, 8192))
    );
    assert_eq!(memory_with_legacy_fallback(missing(), legacy), legacy());
    assert_eq!(memory_with_legacy_fallback(missing(), || None), None);
    for error in [
      NvmlError::NotSupported,
      NvmlError::NoPermission,
      NvmlError::GpuLost,
    ] {
      assert_eq!(
        memory_with_legacy_fallback(Err(error), unexpected_fallback),
        None
      );
    }
  }
}
