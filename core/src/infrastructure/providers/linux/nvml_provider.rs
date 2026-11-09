use nvml_wrapper::{Nvml, enum_wrappers::device::TemperatureSensor};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tokio::{sync::OnceCell, task::spawn_blocking};

use super::drm_sys;

const PCI_SYSFS: &str = "/sys/bus/pci";

/// List display functions already bound to the NVIDIA driver, including
/// compute GPUs whose NVIDIA DRM module is absent.
pub(crate) fn pci_display_addresses() -> Vec<String> {
  pci_display_addresses_at(Path::new(PCI_SYSFS))
}

fn pci_display_addresses_at(pci: &Path) -> Vec<String> {
  let Ok(entries) = fs::read_dir(pci.join("drivers/nvidia")) else {
    return Vec::new();
  };
  let mut addresses: Vec<_> = entries
    .flatten()
    .filter_map(|entry| {
      let name = entry.file_name();
      let address = drm_sys::parse_pci_address_from_device_link(name.to_str()?)?;
      let device = pci.join("devices").join(address);
      let vendor = fs::read_to_string(device.join("vendor")).ok()?;
      let class = fs::read_to_string(device.join("class")).ok()?;
      let class = class.trim().strip_prefix("0x")?;
      let is_display = class.len() == 6
        && class.starts_with("03")
        && class.bytes().all(|byte| byte.is_ascii_hexdigit());
      (vendor.trim() == "0x10de" && is_display).then(|| address.to_ascii_lowercase())
    })
    .collect();
  addresses.sort_unstable();
  addresses
}

fn pci_adapter_ready(pci: &Path, address: &str) -> bool {
  let device = pci.join("devices").join(address);
  fs::read_link(device.join("driver")).is_ok_and(|driver| {
    driver.file_name().and_then(|name| name.to_str()) == Some("nvidia")
  }) && fs::read_to_string(device.join("power/runtime_status"))
    .is_ok_and(|status| status.trim() == "active")
}

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

/// Look up only the positively identified NVIDIA functions requested by the collector.
pub(crate) async fn sample(
  pci_addresses: Vec<String>,
  pci_only_addresses: &[String],
) -> HashMap<String, NvidiaReadings> {
  let pci = Path::new(PCI_SYSFS);
  let pci_only_addresses: Vec<_> = pci_only_addresses
    .iter()
    .filter(|address| pci_adapter_ready(pci, address))
    .cloned()
    .collect();
  if pci_addresses.is_empty() && pci_only_addresses.is_empty() {
    return HashMap::new();
  }

  static NVML: OnceCell<Nvml> = OnceCell::const_new();
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
    let mut read = |address: &str| {
      let device = nvml.device_by_pci_bus_id(address).ok()?;
      NvidiaReadings::from_nvml(
        device.utilization_rates().ok().map(|value| value.gpu),
        device.temperature(TemperatureSensor::Gpu).ok(),
        device
          .memory_info()
          .ok()
          .map(|memory| (memory.used, memory.total)),
      )
    };
    let mut readings =
      read_adapters(pci_addresses, drm_sys::pci_runtime_suspended, &mut read);
    readings.extend(read_adapters(
      pci_only_addresses,
      |address| !pci_adapter_ready(pci, address),
      read,
    ));
    readings
  })
  .await
  .unwrap_or_default()
}

fn read_adapters(
  pci_addresses: Vec<String>,
  mut skip: impl FnMut(&str) -> bool,
  mut read: impl FnMut(&str) -> Option<NvidiaReadings>,
) -> HashMap<String, NvidiaReadings> {
  pci_addresses
    .into_iter()
    .filter_map(|address| {
      // Initialization and earlier lookups can outlast the OS snapshot.
      if skip(&address) {
        return None;
      }
      read(&address).map(|readings| (address, readings))
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::os::unix::fs::symlink;

  fn bound_pci_fixture(
    pci: &Path,
    address: &str,
    vendor: &str,
    class: &str,
  ) -> std::path::PathBuf {
    let driver = pci.join("drivers/nvidia");
    let device = pci.join("devices").join(address);
    fs::create_dir_all(&driver).unwrap();
    fs::create_dir_all(device.join("power")).unwrap();
    fs::write(device.join("vendor"), vendor).unwrap();
    fs::write(device.join("class"), class).unwrap();
    symlink(&driver, device.join("driver")).unwrap();
    symlink(&device, driver.join(address)).unwrap();
    device
  }

  #[test]
  fn discovers_only_bound_nvidia_display_functions_with_full_addresses() {
    let directory = tempfile::tempdir().unwrap();
    let pci = directory.path();
    assert!(pci_display_addresses_at(pci).is_empty());

    bound_pci_fixture(pci, "0001:01:00.0", "0x10de\n", "0x030200\n");
    bound_pci_fixture(pci, "0000:01:00.0", "0x10de\n", "0x030000\n");
    bound_pci_fixture(pci, "0000:01:00.1", "0x10de\n", "0x040300\n");
    bound_pci_fixture(pci, "0000:02:00.0", "0x1002\n", "0x030000\n");
    bound_pci_fixture(pci, "0000:03:00.0", "0x10de\n", "0x0300XX\n");
    bound_pci_fixture(pci, "04:00.0", "0x10de\n", "0x030000\n");
    let unbound = bound_pci_fixture(pci, "0000:05:00.0", "0x10de\n", "0x030000\n");
    fs::remove_file(pci.join("drivers/nvidia/0000:05:00.0")).unwrap();
    fs::remove_file(unbound.join("driver")).unwrap();

    assert_eq!(
      pci_display_addresses_at(pci),
      ["0000:01:00.0", "0001:01:00.0"]
    );
  }

  #[test]
  fn pci_only_lookup_rechecks_binding_and_positive_activity() {
    let directory = tempfile::tempdir().unwrap();
    let pci = directory.path();
    let address = "0000:01:00.0";
    let device = bound_pci_fixture(pci, address, "0x10de\n", "0x030200\n");
    let status = device.join("power/runtime_status");
    assert!(!pci_adapter_ready(pci, address));
    for runtime in [
      "suspended",
      "suspending",
      "resuming",
      "unsupported",
      "error",
    ] {
      fs::write(&status, runtime).unwrap();
      assert!(!pci_adapter_ready(pci, address));
    }
    fs::write(&status, "active\n").unwrap();
    assert!(pci_adapter_ready(pci, address));
    fs::remove_file(device.join("driver")).unwrap();
    symlink(pci.join("drivers/nouveau"), device.join("driver")).unwrap();
    assert!(!pci_adapter_ready(pci, address));
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
}
