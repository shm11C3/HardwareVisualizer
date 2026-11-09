use nvml_wrapper::{Nvml, enum_wrappers::device::TemperatureSensor};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;
use tokio::sync::{Semaphore, mpsc};
use tokio::time::{Instant, timeout_at};

use super::drm_sys;

// Leave the rest of the one-second collector cycle time for other providers.
const NVML_WAIT_BUDGET: Duration = Duration::from_millis(500);
static NVML_PASS: Semaphore = Semaphore::const_new(1);

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

  sample_with_deadline(&NVML_PASS, NVML_WAIT_BUDGET, move |sender| {
    let Some(nvml) = initialized_nvml() else {
      return;
    };
    let skip =
      |address: &str| sender.is_closed() || drm_sys::pci_runtime_suspended(address);
    let read = |address: &str| {
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
    let _ =
      read_adapters(pci_addresses, skip, read).try_for_each(|sample| sender.send(sample));
  })
  .await
}

fn initialized_nvml() -> Option<&'static Nvml> {
  static NVML: OnceLock<Nvml> = OnceLock::new();
  if let Some(nvml) = NVML.get() {
    return Some(nvml);
  }
  // The worker holds the only pass permit. Keep a successful initialization
  // even if it finishes after the deadline, and let a failed init retry.
  let nvml = Nvml::init().ok()?;
  Some(NVML.get_or_init(|| nvml))
}

async fn sample_with_deadline(
  gate: &'static Semaphore,
  budget: Duration,
  sample: impl FnOnce(&mpsc::UnboundedSender<(String, NvidiaReadings)>) + Send + 'static,
) -> HashMap<String, NvidiaReadings> {
  let Ok(permit) = gate.try_acquire() else {
    return HashMap::new();
  };
  let deadline = Instant::now() + budget;
  let (sender, mut receiver) = mpsc::unbounded_channel();
  // The driver can outlive this request. Keep the permit in the worker so a
  // slow or wedged call cannot pile up retries or hold up Tokio shutdown.
  let _ = thread::Builder::new()
    .name("nvml-sample".into())
    .spawn(move || {
      let _permit = permit;
      if !sender.is_closed() {
        sample(&sender);
      }
    });

  let mut samples = HashMap::new();
  while let Ok(Some((address, readings))) = timeout_at(deadline, receiver.recv()).await {
    samples.insert(address, readings);
  }
  samples
}

fn read_adapters(
  pci_addresses: Vec<String>,
  mut is_suspended: impl FnMut(&str) -> bool,
  mut read: impl FnMut(&str) -> Option<NvidiaReadings>,
) -> impl Iterator<Item = (String, NvidiaReadings)> {
  pci_addresses.into_iter().filter_map(move |address| {
    // Initialization and earlier lookups can outlast the DRM snapshot.
    if is_suspended(&address) {
      return None;
    }
    read(&address).map(|readings| (address, readings))
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn keeps_completed_devices_and_refuses_a_retry_while_the_driver_is_busy() {
    static GATE: Semaphore = Semaphore::const_new(1);
    let (release, blocked) = std::sync::mpsc::channel();
    let (late_result, late_result_received) = tokio::sync::oneshot::channel();
    let first = "0000:01:00.0";
    let late = "0000:02:00.0";
    let budget = Duration::from_millis(250);

    let samples = sample_with_deadline(&GATE, budget, move |sender| {
      let _ = sender.send((
        first.into(),
        NvidiaReadings::from_nvml(Some(17), None, None).unwrap(),
      ));
      let _ = blocked.recv();
      let discarded = sender
        .send((late.into(), NvidiaReadings::default()))
        .is_err();
      let _ = late_result.send(discarded);
    })
    .await;

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[first].usage, Some(17.0));
    assert_eq!(GATE.available_permits(), 0);
    assert!(
      sample_with_deadline(&GATE, budget, |sender| {
        let _ = sender.send(("unexpected retry".into(), NvidiaReadings::default()));
      })
      .await
      .is_empty()
    );

    release.send(()).unwrap();
    assert!(
      tokio::time::timeout(Duration::from_secs(2), late_result_received)
        .await
        .unwrap()
        .unwrap()
    );
    let permit = tokio::time::timeout(Duration::from_secs(2), GATE.acquire())
      .await
      .unwrap()
      .unwrap();
    drop(permit);
    let samples = sample_with_deadline(&GATE, budget, move |sender| {
      let _ = sender.send((
        first.into(),
        NvidiaReadings::from_nvml(Some(18), None, None).unwrap(),
      ));
    })
    .await;
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[first].usage, Some(18.0));
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
    let samples: HashMap<_, _> = read_adapters(
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
    )
    .collect();

    assert_eq!(samples.len(), 2);
    assert_eq!(samples["0000:01:00.0"].usage, Some(12.0));
    assert_eq!(samples["0000:01:00.1"].temperature, Some(62.0));
    assert!(!samples.contains_key("0001:01:00.0"));
  }

  #[test]
  fn skips_a_card_that_suspended_after_selection() {
    let addresses = vec!["0000:01:00.0".into(), "0000:02:00.0".into()];
    let mut queried = Vec::new();
    let samples: HashMap<_, _> = read_adapters(
      addresses,
      |address| address == "0000:01:00.0",
      |address| {
        queried.push(address.to_owned());
        NvidiaReadings::from_nvml(Some(12), None, None)
      },
    )
    .collect();

    assert_eq!(queried, ["0000:02:00.0"]);
    assert_eq!(samples.len(), 1);
    assert_eq!(samples["0000:02:00.0"].usage, Some(12.0));
  }
}
