use nvml_wrapper::{Nvml, enum_wrappers::device::TemperatureSensor};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio::time::{Instant, timeout_at};

use super::drm_sys;

// Leave the rest of the one-second collector cycle time for other providers.
const NVML_WAIT_BUDGET: Duration = Duration::from_millis(500);
// One stalled GPU leaves a worker for healthy GPUs without starting a native
// thread for every device if the driver itself stops responding.
const DEVICE_WORKER_LIMIT: usize = 2;
static DEVICE_WORKERS: DeviceWorkers = DeviceWorkers::new();

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

  let deadline = Instant::now() + NVML_WAIT_BUDGET;
  let Some(nvml) = initialized_nvml(deadline).await else {
    return HashMap::new();
  };
  DEVICE_WORKERS
    .sample(pci_addresses, deadline, move |address| {
      read_if_active(address, drm_sys::pci_runtime_suspended, |address| {
        let device = nvml.device_by_pci_bus_id(address).ok()?;
        NvidiaReadings::from_nvml(
          device.utilization_rates().ok().map(|value| value.gpu),
          device.temperature(TemperatureSensor::Gpu).ok(),
          device
            .memory_info()
            .ok()
            .map(|memory| (memory.used, memory.total)),
        )
      })
    })
    .await
}

async fn initialized_nvml(deadline: Instant) -> Option<&'static Nvml> {
  static NVML: OnceLock<Nvml> = OnceLock::new();
  static INITIALIZING: Semaphore = Semaphore::const_new(1);
  if let Some(nvml) = NVML.get() {
    return Some(nvml);
  }
  let permit = INITIALIZING.try_acquire().ok()?;
  if let Some(nvml) = NVML.get() {
    return Some(nvml);
  }
  let (sender, receiver) = oneshot::channel();
  thread::Builder::new()
    .name("nvml-init".into())
    .spawn(move || {
      // Keep a late success for later cycles and let a failed init retry.
      let nvml = Nvml::init().ok().map(|nvml| NVML.get_or_init(|| nvml));
      drop(permit);
      let _ = sender.send(nvml);
    })
    .ok()?;
  timeout_at(deadline, receiver).await.ok()?.ok()?
}

struct DeviceWorkers {
  slots: Semaphore,
  active: Mutex<Vec<String>>,
}

impl DeviceWorkers {
  const fn new() -> Self {
    Self {
      slots: Semaphore::const_new(DEVICE_WORKER_LIMIT),
      active: Mutex::new(Vec::new()),
    }
  }

  fn claim<'a>(&'a self, address: &'a str) -> Option<ActiveDevice<'a>> {
    let mut active = self.active.lock().ok()?;
    if active.iter().any(|candidate| candidate == address) {
      return None;
    }
    active.push(address.to_owned());
    Some(ActiveDevice {
      address,
      devices: &self.active,
    })
  }

  async fn sample(
    &'static self,
    mut addresses: Vec<String>,
    deadline: Instant,
    read: impl Fn(&str) -> Option<NvidiaReadings> + Send + Sync + 'static,
  ) -> HashMap<String, NvidiaReadings> {
    if let Ok(active) = self.active.lock() {
      addresses.retain(|address| !active.contains(address));
    } else {
      return HashMap::new();
    }
    let worker_count = addresses.len().min(DEVICE_WORKER_LIMIT);
    let addresses = Arc::new(Mutex::new(addresses.into_iter()));
    let read = Arc::new(read);
    let (sender, mut receiver) = mpsc::unbounded_channel();
    for _ in 0..worker_count {
      let Ok(permit) = self.slots.try_acquire() else {
        break;
      };
      let addresses = Arc::clone(&addresses);
      let read = Arc::clone(&read);
      let sender = sender.clone();
      // Neither timeout nor caller cancellation releases a slot or a device
      // claim while native code is running. Tokio shutdown can still proceed.
      let _ = thread::Builder::new()
        .name("nvml-sample".into())
        .spawn(move || {
          let _permit = permit;
          while !sender.is_closed() && Instant::now() < deadline {
            let address = addresses.lock().ok().and_then(|mut pending| pending.next());
            let Some(address) = address else { break };
            let readings = {
              let Some(_device) = self.claim(&address) else {
                continue;
              };
              read(&address)
            };
            if let Some(readings) = readings {
              let _ = sender.send((address, readings));
            }
          }
        });
    }
    drop(sender);

    let mut samples = HashMap::new();
    while let Ok(Some((address, readings))) = timeout_at(deadline, receiver.recv()).await
    {
      samples.insert(address, readings);
    }
    samples
  }
}

struct ActiveDevice<'a> {
  address: &'a str,
  devices: &'a Mutex<Vec<String>>,
}

impl Drop for ActiveDevice<'_> {
  fn drop(&mut self) {
    if let Ok(mut devices) = self.devices.lock() {
      devices.retain(|address| address != self.address);
    }
  }
}

fn read_if_active(
  address: &str,
  is_suspended: impl FnOnce(&str) -> bool,
  read: impl FnOnce(&str) -> Option<NvidiaReadings>,
) -> Option<NvidiaReadings> {
  // Initialization and earlier lookups can outlast the DRM snapshot.
  if is_suspended(address) {
    return None;
  }
  read(address)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn a_stalled_first_device_does_not_hide_siblings_or_accumulate_retries() {
    static WORKERS: DeviceWorkers = DeviceWorkers::new();
    let (release, blocked) = std::sync::mpsc::channel();
    let blocked = Mutex::new(blocked);
    let first = "0000:01:00.0";
    let second = "0000:02:00.0";
    let third = "0000:03:00.0";
    let budget = Duration::from_millis(250);
    let addresses = || [first, second, third].map(str::to_owned).to_vec();

    let samples = WORKERS
      .sample(addresses(), Instant::now() + budget, move |address| {
        if address == first {
          let _ = blocked.lock().unwrap().recv();
        }
        NvidiaReadings::from_nvml(Some(17), None, None)
      })
      .await;

    assert_eq!(samples.len(), 2);
    assert_eq!(samples[second].usage, Some(17.0));
    assert_eq!(samples[third].usage, Some(17.0));
    assert!(!samples.contains_key(first));
    assert_eq!(WORKERS.slots.available_permits(), 1);

    let next = WORKERS
      .sample(addresses(), Instant::now() + budget, |_| {
        NvidiaReadings::from_nvml(Some(18), None, None)
      })
      .await;
    assert_eq!(next.len(), 2);
    assert_eq!(next[second].usage, Some(18.0));
    assert!(!next.contains_key(first));

    release.send(()).unwrap();
    let permit = tokio::time::timeout(
      Duration::from_secs(2),
      WORKERS.slots.acquire_many(DEVICE_WORKER_LIMIT as u32),
    )
    .await
    .unwrap()
    .unwrap();
    drop(permit);
    let samples = WORKERS
      .sample(addresses(), Instant::now() + budget, |_| {
        NvidiaReadings::from_nvml(Some(19), None, None)
      })
      .await;
    assert_eq!(samples.len(), 3);
    assert_eq!(samples[first].usage, Some(19.0));
  }

  #[tokio::test]
  async fn a_driver_stall_cannot_start_more_than_two_workers() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static WORKERS: DeviceWorkers = DeviceWorkers::new();
    static STARTED: AtomicUsize = AtomicUsize::new(0);
    let (release, blocked) = std::sync::mpsc::channel::<()>();
    let blocked = Mutex::new(blocked);
    let addresses = || {
      ["0000:01:00.0", "0000:02:00.0", "0000:03:00.0"]
        .map(str::to_owned)
        .to_vec()
    };
    let budget = Duration::from_millis(250);

    let samples = WORKERS
      .sample(addresses(), Instant::now() + budget, move |_| {
        STARTED.fetch_add(1, Ordering::Relaxed);
        let _ = blocked.lock().unwrap().recv();
        None
      })
      .await;
    assert!(samples.is_empty());
    assert_eq!(STARTED.load(Ordering::Relaxed), DEVICE_WORKER_LIMIT);

    let samples = WORKERS
      .sample(addresses(), Instant::now() + budget, |_| {
        STARTED.fetch_add(1, Ordering::Relaxed);
        NvidiaReadings::from_nvml(Some(1), None, None)
      })
      .await;
    assert!(samples.is_empty());
    assert_eq!(STARTED.load(Ordering::Relaxed), DEVICE_WORKER_LIMIT);
    drop(release);
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

  #[tokio::test]
  async fn one_failed_device_preserves_sibling_results_by_full_pci_address() {
    static WORKERS: DeviceWorkers = DeviceWorkers::new();
    // Synthetic devices with the same slot on separate PCI domains and a
    // nonzero PCI function. NVML lookup takes the complete DRM address.
    let samples = WORKERS
      .sample(
        ["0001:01:00.0", "0000:01:00.0", "0000:01:00.1"]
          .map(str::to_string)
          .to_vec(),
        Instant::now() + Duration::from_secs(2),
        |address| match address {
          "0000:01:00.0" => NvidiaReadings::from_nvml(Some(12), Some(51), None),
          "0000:01:00.1" => NvidiaReadings::from_nvml(None, Some(62), None),
          "0001:01:00.0" => None,
          _ => unreachable!(),
        },
      )
      .await;

    assert_eq!(samples.len(), 2);
    assert_eq!(samples["0000:01:00.0"].usage, Some(12.0));
    assert_eq!(samples["0000:01:00.1"].temperature, Some(62.0));
    assert!(!samples.contains_key("0001:01:00.0"));
  }

  #[test]
  fn skips_a_card_that_suspended_after_selection() {
    let suspended = read_if_active(
      "0000:01:00.0",
      |_| true,
      |_| panic!("unexpected lookup after suspend"),
    );
    assert!(suspended.is_none());
    let active = read_if_active(
      "0000:02:00.0",
      |_| false,
      |address| {
        assert_eq!(address, "0000:02:00.0");
        NvidiaReadings::from_nvml(Some(12), None, None)
      },
    );
    assert_eq!(active.unwrap().usage, Some(12.0));
  }
}
