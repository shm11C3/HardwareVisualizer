//! Experimental ASUS ROG CROSSHAIR VIII EC readings through the already
//! bound in-tree Linux `asus-ec-sensors` hwmon endpoint.
//!
//! The application reads public sysfs attributes only. It opens no raw
//! device, writes nothing, and never loads, binds or parameterizes the
//! module. The kernel driver owns every EC transaction under the firmware
//! mutex and the kernel EC lock.
//!
//! A refresh behind an input read can wait for the firmware mutex, so passes
//! run on a detached thread at most once per [`MIN_PASS_INTERVAL`], and the
//! collector reports the latest completed pass.

use super::sysfs::{file_identity, is_hwmon_name, resolves_to};
use crate::models::{
  FanSpeedStatus, MotherboardFanSpeed, MotherboardSensorCollection,
  MotherboardSensorSample, MotherboardTemperature, SensorAvailability, SensorSupport,
};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

const ASUS_VENDOR: &str = "ASUSTeK COMPUTER INC.";
/// The platform device, which the driver registers without an instance
/// number, and the platform driver that must own it.
const PLATFORM_DEVICE: &str = "asus-ec-sensors";
const PLATFORM_DRIVER: &str = "asus-ec-sensors";
const HWMON_NAME: &str = "asusec";
const MODULE: &str = "asus_ec_sensors";
const SOURCE: &str = "ASUS EC (hwmon)";

/// Passes start at least this far apart.
const MIN_PASS_INTERVAL: Duration = Duration::from_secs(1);
/// A completed pass older than this is not reported, so a stalled read does
/// not keep publishing its last values.
const MAX_RESULT_AGE: Duration = Duration::from_secs(5);

/// Inclusive temperature bounds in millidegrees Celsius.
const MIN_MILLI_CELSIUS: i32 = -55_000;
const MAX_MILLI_CELSIUS: i32 = 125_000;
/// Values that an environment header without a probe reports on kernels that
/// predate the "no data" read error.
const BLANK_HEADER_MILLI_CELSIUS: [i32; 3] = [-40_000, -60_000, -62_000];
/// Inclusive fan bounds in RPM.
const MIN_RPM: u32 = 1;
const MAX_RPM: u32 = 65_534;

/// A scoped `tempN` channel located by its `tempN_label`.
struct TemperatureChannel {
  label: &'static str,
  name: &'static str,
  /// Environment header that reads a blank value without a probe.
  blank_header: bool,
}

/// A scoped `fanN` channel located by its `fanN_label`.
struct FanChannel {
  label: &'static str,
  name: &'static str,
}

/// The scoped channels of one EC register map and firmware mutex that the
/// kernel driver binds for the boards that select it.
struct EcMap {
  temperatures: &'static [TemperatureChannel],
  fans: &'static [FanChannel],
}

/// A board whose DMI identity selects an audited EC map. The BIOS version is
/// an evidence identity recorded in the specification and does not gate a
/// read.
struct BoardProfile {
  vendor: &'static str,
  board: &'static str,
  map: &'static EcMap,
}

/// The AMD 500-series map with firmware mutex `\AMW0.ASMX`, restricted to
/// the readings that no other Linux provider reports. The EC CPU,
/// Motherboard, CPU current and CPU core voltage readings and the W_FLOW
/// flow meter are excluded, and their inputs are never read.
const AMD_500_SERIES_ASMX: EcMap = EcMap {
  temperatures: &[
    TemperatureChannel {
      label: "Chipset",
      name: "Chipset",
      blank_header: false,
    },
    TemperatureChannel {
      label: "VRM",
      name: "VRM",
      blank_header: false,
    },
    TemperatureChannel {
      label: "T_Sensor",
      name: "T_SENSOR",
      blank_header: true,
    },
    TemperatureChannel {
      label: "Water_In",
      name: "W_IN",
      blank_header: true,
    },
    TemperatureChannel {
      label: "Water_Out",
      name: "W_OUT",
      blank_header: true,
    },
  ],
  fans: &[
    FanChannel {
      label: "CPU_Opt",
      name: "CPU_OPT",
    },
    FanChannel {
      label: "Chipset",
      name: "PCH_FAN",
    },
  ],
};

static PROFILES: [BoardProfile; 3] = [
  BoardProfile {
    vendor: ASUS_VENDOR,
    board: "ROG CROSSHAIR VIII HERO",
    map: &AMD_500_SERIES_ASMX,
  },
  BoardProfile {
    vendor: ASUS_VENDOR,
    board: "ROG CROSSHAIR VIII HERO (WI-FI)",
    map: &AMD_500_SERIES_ASMX,
  },
  BoardProfile {
    vendor: ASUS_VENDOR,
    board: "ROG CROSSHAIR VIII FORMULA",
    map: &AMD_500_SERIES_ASMX,
  },
];

static SAMPLER: Sampler = Sampler::new();

pub(crate) fn sample() -> MotherboardSensorCollection {
  SAMPLER.sample(PathBuf::from("/sys"), |pass| {
    std::thread::Builder::new()
      .name("asus-ec-hwmon".to_owned())
      .spawn(pass)
      .map(drop)
  })
}

type Pass = Box<dyn FnOnce() + Send>;

/// Schedules sampling passes off the caller's thread and keeps the latest
/// completed one.
struct Sampler {
  state: Mutex<SamplerState>,
}

impl Sampler {
  const fn new() -> Self {
    Self {
      state: Mutex::new(SamplerState {
        last_start: None,
        in_flight: false,
        latest: None,
      }),
    }
  }

  fn lock(&self) -> MutexGuard<'_, SamplerState> {
    self.state.lock().unwrap_or_else(PoisonError::into_inner)
  }

  /// Returns the DMI result directly on a board outside the profiles.
  /// Otherwise starts a pass when one is due and reports the latest
  /// completed pass.
  fn sample(
    &'static self,
    sysfs: PathBuf,
    spawn: impl FnOnce(Pass) -> std::io::Result<()>,
  ) -> MotherboardSensorCollection {
    if let Err(collection) = identify_profile(&sysfs.join("class/dmi/id")) {
      return collection;
    }
    // The guard is released before spawning so the pass can take it.
    let due = self.lock().begin(Instant::now());
    if due {
      let pass: Pass = Box::new(move || {
        let collection = sample_at(&sysfs);
        self.lock().finish(Instant::now(), collection);
      });
      if spawn(pass).is_err() {
        self.lock().abandon();
      }
    }
    self.lock().current(Instant::now()).unwrap_or_else(|| {
      MotherboardSensorCollection::unavailable(
        "Waiting for the asus-ec-sensors hwmon reading",
      )
    })
  }
}

struct SamplerState {
  last_start: Option<Instant>,
  in_flight: bool,
  latest: Option<(Instant, MotherboardSensorCollection)>,
}

impl SamplerState {
  /// Claims a pass when none is running and the previous one started at
  /// least [`MIN_PASS_INTERVAL`] ago.
  fn begin(&mut self, now: Instant) -> bool {
    let due = !self.in_flight
      && self
        .last_start
        .is_none_or(|start| now.saturating_duration_since(start) >= MIN_PASS_INTERVAL);
    if due {
      self.in_flight = true;
      self.last_start = Some(now);
    }
    due
  }

  fn finish(&mut self, now: Instant, collection: MotherboardSensorCollection) {
    self.in_flight = false;
    self.latest = Some((now, collection));
  }

  fn abandon(&mut self) {
    self.in_flight = false;
  }

  fn current(&self, now: Instant) -> Option<MotherboardSensorCollection> {
    self
      .latest
      .as_ref()
      .filter(|(completed, _)| {
        now.saturating_duration_since(*completed) <= MAX_RESULT_AGE
      })
      .map(|(_, collection)| collection.clone())
  }
}

/// Admit a board from the DMI baseboard vendor and product only.
fn identify_profile(
  dmi: &Path,
) -> Result<&'static BoardProfile, MotherboardSensorCollection> {
  let missing = || {
    MotherboardSensorCollection::unavailable("Could not identify the Linux motherboard")
  };
  let unsupported = || {
    MotherboardSensorCollection::unsupported(
      "No supported ASUS EC sensor profile for this board",
    )
  };
  let vendor = read_attribute(dmi, "board_vendor").ok_or_else(missing)?;
  if !PROFILES.iter().any(|profile| profile.vendor == vendor) {
    return Err(unsupported());
  }
  let board = read_attribute(dmi, "board_name").ok_or_else(missing)?;
  PROFILES
    .iter()
    .find(|profile| profile.vendor == vendor && profile.board == board)
    .ok_or_else(unsupported)
}

/// One full pass: qualify the endpoint, read every label, then read the
/// scoped inputs, then re-qualify the endpoint.
fn sample_at(sysfs: &Path) -> MotherboardSensorCollection {
  let profile = match identify_profile(&sysfs.join("class/dmi/id")) {
    Ok(profile) => profile,
    Err(collection) => return collection,
  };
  let Some(endpoint) = find_endpoint(sysfs) else {
    return MotherboardSensorCollection::unavailable(
      "The bound in-tree asus-ec-sensors hwmon endpoint is unavailable or ambiguous",
    );
  };

  let temperature_labels = channel_labels(&endpoint.path, "temp");
  let fan_labels = channel_labels(&endpoint.path, "fan");
  let temperatures = profile
    .map
    .temperatures
    .iter()
    .filter_map(|channel| {
      let index = unique_channel(&temperature_labels, channel.label)?;
      read_temperature(&endpoint.path, index, channel)
    })
    .collect();
  let fan_speeds = profile
    .map
    .fans
    .iter()
    .filter_map(|channel| {
      let index = unique_channel(&fan_labels, channel.label)?;
      read_fan(&endpoint.path, index, channel)
    })
    .collect();
  finish_sample(
    sysfs,
    &endpoint,
    MotherboardSensorSample {
      temperatures,
      fan_speeds,
    },
  )
}

fn finish_sample(
  sysfs: &Path,
  endpoint: &Endpoint,
  sample: MotherboardSensorSample,
) -> MotherboardSensorCollection {
  // Linux can reuse the same paths after a binding or hwmon node is replaced.
  if find_endpoint(sysfs).as_ref() != Some(endpoint) {
    return MotherboardSensorCollection::unavailable(
      "The asus-ec-sensors hwmon binding changed during collection",
    );
  }
  let availability = if sample.temperatures.is_empty() && sample.fan_speeds.is_empty() {
    SensorAvailability::unavailable(
      "The asus-ec-sensors hwmon endpoint returned no usable readings",
    )
  } else {
    SensorAvailability::Available
  };
  MotherboardSensorCollection {
    sample,
    availability,
    fan_support: SensorSupport::Supported,
    ..MotherboardSensorCollection::default()
  }
}

#[derive(Debug, PartialEq, Eq)]
struct Endpoint {
  path: PathBuf,
  identities: [(u64, u64); 4],
}

/// Qualify the one canonical endpoint: the bound platform device, its only
/// hwmon child carrying the chip name, and an in-tree module.
fn find_endpoint(sysfs: &Path) -> Option<Endpoint> {
  let device = sysfs.join("devices/platform").join(PLATFORM_DEVICE);
  let driver_link = device.join("driver");
  let driver = sysfs.join("bus/platform/drivers").join(PLATFORM_DRIVER);
  if !resolves_to(&device, &device)
    || !resolves_to(&device.join("subsystem"), &sysfs.join("bus/platform"))
    || !resolves_to(&driver_link, &driver)
    || !resolves_to(
      &sysfs.join("bus/platform/devices").join(PLATFORM_DEVICE),
      &device,
    )
  {
    return None;
  }
  let path = only_hwmon_child(&device)?;
  let index = path.file_name()?;
  if !resolves_to(&path.join("device"), &device)
    || !resolves_to(&sysfs.join("class/hwmon").join(index), &path)
    || read_attribute(&path, "name").as_deref() != Some(HWMON_NAME)
  {
    return None;
  }
  let module = in_tree_module(sysfs, &driver)?;

  let identities = [
    file_identity(&fs::metadata(&path).ok()?),
    file_identity(&fs::metadata(&device).ok()?),
    file_identity(&fs::symlink_metadata(&driver_link).ok()?),
    file_identity(&fs::metadata(module).ok()?),
  ];
  Some(Endpoint { path, identities })
}

/// The device's only `hwmon/hwmonN` child. More than one child is ambiguous.
fn only_hwmon_child(device: &Path) -> Option<PathBuf> {
  let mut child = None;
  for entry in fs::read_dir(device.join("hwmon")).ok()? {
    let entry = entry.ok()?;
    if !entry.file_name().to_str().is_some_and(is_hwmon_name) {
      continue;
    }
    if child.replace(entry.path()).is_some() {
      return None;
    }
  }
  child
}

/// Admit the driver's module directory when it is a loadable module without
/// the out-of-tree taint flag, or a built-in driver, which has neither
/// `initstate` nor `taint`. Any other combination is unavailable.
fn in_tree_module(sysfs: &Path, driver: &Path) -> Option<PathBuf> {
  let module = sysfs.join("module").join(MODULE);
  if !resolves_to(&driver.join("module"), &module) || !module.is_dir() {
    return None;
  }
  match (
    exists(&module.join("initstate"))?,
    exists(&module.join("taint"))?,
  ) {
    (true, true) => {
      let taint = read_attribute(&module, "taint")?;
      (!taint.contains('O')).then_some(module)
    }
    (false, false) => Some(module),
    _ => None,
  }
}

/// `Some(false)` only for a confirmed absence; an inaccessible entry is
/// unknown.
fn exists(path: &Path) -> Option<bool> {
  match fs::symlink_metadata(path) {
    Ok(_) => Some(true),
    Err(error) if error.kind() == ErrorKind::NotFound => Some(false),
    Err(_) => None,
  }
}

/// Every `<kind>N_label` of the endpoint with its label, or `None` for an
/// unreadable label. Reading labels performs no EC access.
fn channel_labels(path: &Path, kind: &str) -> Vec<(u16, Option<String>)> {
  let Ok(entries) = fs::read_dir(path) else {
    return Vec::new();
  };
  entries
    .filter_map(|entry| {
      let file_name = entry.ok()?.file_name();
      let index = channel_index(file_name.to_str()?, kind)?;
      Some((index, read_attribute(path, &format!("{kind}{index}_label"))))
    })
    .collect()
}

/// The canonical decimal `N` of `<kind>N_label`.
fn channel_index(file_name: &str, kind: &str) -> Option<u16> {
  let digits = file_name.strip_prefix(kind)?.strip_suffix("_label")?;
  let index = digits.parse::<u16>().ok()?;
  (digits == index.to_string()).then_some(index)
}

/// The one channel that carries `label`. A missing or duplicated label
/// excludes the reading.
fn unique_channel(labels: &[(u16, Option<String>)], label: &str) -> Option<u16> {
  let mut matching = labels
    .iter()
    .filter(|(_, value)| value.as_deref() == Some(label))
    .map(|(index, _)| *index);
  let index = matching.next()?;
  matching.next().is_none().then_some(index)
}

/// Read a sysfs attribute and trim one trailing line ending.
fn read_attribute(directory: &Path, attribute: &str) -> Option<String> {
  let path = directory.join(attribute);
  #[cfg(test)]
  tests::record_read(&path);
  let text = fs::read_to_string(path).ok()?;
  let text = text
    .strip_suffix("\r\n")
    .or_else(|| text.strip_suffix('\n'))
    .unwrap_or(&text);
  Some(text.to_owned())
}

fn read_temperature(
  path: &Path,
  index: u16,
  channel: &TemperatureChannel,
) -> Option<MotherboardTemperature> {
  let raw = read_attribute(path, &format!("temp{index}_input"))?;
  decode_temperature(&raw, channel.blank_header).map(|temperature| {
    MotherboardTemperature {
      name: channel.name.to_owned(),
      temperature,
      source: SOURCE.to_owned(),
    }
  })
}

fn read_fan(
  path: &Path,
  index: u16,
  channel: &FanChannel,
) -> Option<MotherboardFanSpeed> {
  let raw = read_attribute(path, &format!("fan{index}_input"))?;
  decode_fan(&raw).map(|rpm| MotherboardFanSpeed {
    name: channel.name.to_owned(),
    rpm: Some(rpm),
    status: FanSpeedStatus::Active,
    source: SOURCE.to_owned(),
  })
}

/// Decode a complete signed decimal millidegree value to degrees Celsius.
/// Zero is rejected as never read, and the blank values of an environment
/// header without a probe are rejected on header channels.
fn decode_temperature(raw: &str, blank_header: bool) -> Option<f32> {
  let digits = raw.strip_prefix('-').unwrap_or(raw);
  if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
    return None;
  }
  let milli_celsius = raw.parse::<i32>().ok()?;
  let plausible = (MIN_MILLI_CELSIUS..=MAX_MILLI_CELSIUS).contains(&milli_celsius)
    && milli_celsius != 0
    && !(blank_header && BLANK_HEADER_MILLI_CELSIUS.contains(&milli_celsius));
  plausible.then_some(milli_celsius as f32 / 1000.0)
}

/// Decode a complete unsigned decimal RPM value. Zero does not distinguish a
/// stopped fan from an empty header, so it is unavailable.
fn decode_fan(raw: &str) -> Option<u32> {
  if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
    return None;
  }
  raw
    .parse::<u32>()
    .ok()
    .filter(|rpm| (MIN_RPM..=MAX_RPM).contains(rpm))
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::cell::RefCell;
  use std::os::unix::fs::{PermissionsExt, symlink};
  use tempfile::TempDir;

  thread_local! {
    /// Every attribute path that `read_attribute` opened on this test thread.
    static READS: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
  }

  pub(super) fn record_read(path: &Path) {
    READS.with_borrow_mut(|reads| reads.push(path.to_path_buf()));
  }

  const HERO: &str = "ROG CROSSHAIR VIII HERO";
  const HERO_WIFI: &str = "ROG CROSSHAIR VIII HERO (WI-FI)";
  const FORMULA: &str = "ROG CROSSHAIR VIII FORMULA";
  /// The S3 observation's BIOS version, an evidence identity only.
  const S3_BIOS: &str = "4201";

  /// Synthetic channel numbering of the whole board sensor set, in an order
  /// chosen for the fixture. The provider locates channels by label, so a
  /// test below also permutes it.
  const TEMPERATURE_LABELS: [&str; 7] = [
    "Chipset",
    "CPU",
    "Motherboard",
    "T_Sensor",
    "VRM",
    "Water_In",
    "Water_Out",
  ];
  const FAN_LABELS: [&str; 3] = ["CPU_Opt", "Chipset", "Water_Flow"];

  /// Synthetic sysfs values for the spec's S3 `sensors` report
  /// ("Synthetic fixture acceptance", first case): Chipset 58 °C, CPU 40 °C,
  /// Motherboard 34 °C, T_Sensor/Water_In/Water_Out −40 °C, VRM 28 °C;
  /// CPU_Opt 0, Chipset 1829 and Water_Flow 0 RPM; CPU Core 1.06 V; CPU 0 A.
  const S3_TEMPERATURES: [&str; 7] = [
    "58000", "40000", "34000", "-40000", "28000", "-40000", "-40000",
  ];
  const S3_FANS: [&str; 3] = ["0", "1829", "0"];

  /// The inputs of the excluded EC CPU, Motherboard, W_FLOW, CPU core
  /// voltage and CPU current channels in the fixture numbering.
  const EXCLUDED_INPUTS: [&str; 5] = [
    "temp2_input",
    "temp3_input",
    "fan3_input",
    "in1_input",
    "curr1_input",
  ];

  struct Fixture {
    _directory: TempDir,
    sysfs: PathBuf,
    node: PathBuf,
  }

  impl Fixture {
    /// The canonical HERO endpoint with a loadable in-tree module and the S3
    /// values.
    fn hero() -> Self {
      Self::board(HERO)
    }

    fn board(board: &str) -> Self {
      let fixture = Self::empty(ASUS_VENDOR, board, PLATFORM_DEVICE, "hwmon3");
      fixture.write_channels("temp", &TEMPERATURE_LABELS, &S3_TEMPERATURES);
      fixture.write_channels("fan", &FAN_LABELS, &S3_FANS);
      fixture.write("in1_label", "CPU Core\n");
      fixture.write("in1_input", "1060\n");
      fixture.write("curr1_label", "CPU\n");
      fixture.write("curr1_input", "0\n");
      fixture
    }

    fn empty(vendor: &str, board: &str, device_name: &str, hwmon: &str) -> Self {
      let directory = tempfile::tempdir().unwrap();
      let sysfs = directory.path().canonicalize().unwrap();
      let dmi = sysfs.join("class/dmi/id");
      fs::create_dir_all(&dmi).unwrap();
      for (attribute, value) in [
        ("board_vendor", vendor),
        ("board_name", board),
        ("bios_version", S3_BIOS),
      ] {
        fs::write(dmi.join(attribute), format!("{value}\n")).unwrap();
      }

      let device = sysfs.join("devices/platform").join(device_name);
      let driver = sysfs.join("bus/platform/drivers").join(PLATFORM_DRIVER);
      let bus_devices = sysfs.join("bus/platform/devices");
      let module = sysfs.join("module").join(MODULE);
      let class = sysfs.join("class/hwmon");
      let node = device.join("hwmon").join(hwmon);
      for directory in [&driver, &bus_devices, &module, &node, &class] {
        fs::create_dir_all(directory).unwrap();
      }
      fs::write(module.join("initstate"), "live\n").unwrap();
      fs::write(module.join("taint"), "\n").unwrap();
      symlink(&module, driver.join("module")).unwrap();
      symlink(sysfs.join("bus/platform"), device.join("subsystem")).unwrap();
      symlink(&driver, device.join("driver")).unwrap();
      symlink(&device, bus_devices.join(device_name)).unwrap();
      symlink(&device, node.join("device")).unwrap();
      symlink(&node, class.join(hwmon)).unwrap();
      fs::write(node.join("name"), format!("{HWMON_NAME}\n")).unwrap();
      Self {
        _directory: directory,
        sysfs,
        node,
      }
    }

    fn write_channels(&self, kind: &str, labels: &[&str], inputs: &[&str]) {
      for (index, (label, input)) in (1..).zip(labels.iter().zip(inputs)) {
        self.write(&format!("{kind}{index}_label"), &format!("{label}\n"));
        self.write(&format!("{kind}{index}_input"), &format!("{input}\n"));
      }
    }

    fn device(&self) -> PathBuf {
      self.sysfs.join("devices/platform").join(PLATFORM_DEVICE)
    }

    fn module(&self) -> PathBuf {
      self.sysfs.join("module").join(MODULE)
    }

    fn dmi(&self) -> PathBuf {
      self.sysfs.join("class/dmi/id")
    }

    fn write(&self, attribute: &str, value: &str) {
      fs::write(self.node.join(attribute), value).unwrap();
    }

    /// Replace an attribute file with a directory, which fails to read.
    fn make_unreadable(&self, attribute: &str) {
      let path = self.node.join(attribute);
      fs::remove_file(&path).unwrap();
      fs::create_dir(path).unwrap();
    }

    fn sample(&self) -> MotherboardSensorCollection {
      sample_at(&self.sysfs)
    }

    fn endpoint(&self) -> Option<Endpoint> {
      find_endpoint(&self.sysfs)
    }

    /// Drain the read log and return the attributes opened under this
    /// fixture's hwmon node since the previous call.
    fn node_reads(&self) -> Vec<String> {
      READS
        .take()
        .into_iter()
        .filter_map(|path| {
          let attribute = path.strip_prefix(&self.node).ok()?;
          Some(attribute.to_str()?.to_owned())
        })
        .collect()
    }

    fn input_reads(&self) -> Vec<String> {
      self
        .node_reads()
        .into_iter()
        .filter(|attribute| attribute.ends_with("_input"))
        .collect()
    }
  }

  fn temperature(name: &str, temperature: f32) -> MotherboardTemperature {
    MotherboardTemperature {
      name: name.to_owned(),
      temperature,
      source: SOURCE.to_owned(),
    }
  }

  fn fan(name: &str, rpm: u32) -> MotherboardFanSpeed {
    MotherboardFanSpeed {
      name: name.to_owned(),
      rpm: Some(rpm),
      status: FanSpeedStatus::Active,
      source: SOURCE.to_owned(),
    }
  }

  fn s3_sample() -> MotherboardSensorSample {
    MotherboardSensorSample {
      temperatures: vec![temperature("Chipset", 58.0), temperature("VRM", 28.0)],
      fan_speeds: vec![fan("PCH_FAN", 1829)],
    }
  }

  fn connected_sample() -> MotherboardSensorSample {
    MotherboardSensorSample {
      temperatures: vec![
        temperature("Chipset", 58.0),
        temperature("VRM", 28.0),
        temperature("T_SENSOR", 31.0),
        temperature("W_IN", 27.0),
        temperature("W_OUT", 29.0),
      ],
      fan_speeds: vec![fan("CPU_OPT", 1200), fan("PCH_FAN", 1829)],
    }
  }

  fn connect_headers(fixture: &Fixture) {
    fixture.write("temp4_input", "31000\n");
    fixture.write("temp6_input", "27000\n");
    fixture.write("temp7_input", "29000\n");
    fixture.write("fan1_input", "1200\n");
  }

  fn assert_available(
    collection: &MotherboardSensorCollection,
    expected: &MotherboardSensorSample,
  ) {
    assert_eq!(collection.availability, SensorAvailability::Available);
    assert_eq!(collection.fan_support, SensorSupport::Supported);
    assert_eq!(&collection.sample, expected);
  }

  fn assert_unavailable(collection: &MotherboardSensorCollection) {
    assert!(
      matches!(
        collection.availability,
        SensorAvailability::Unavailable { .. }
      ),
      "{collection:?}"
    );
    assert_eq!(collection.sample, MotherboardSensorSample::default());
  }

  fn assert_unsupported(collection: &MotherboardSensorCollection) {
    assert!(
      matches!(
        collection.availability,
        SensorAvailability::Unsupported { .. }
      ),
      "{collection:?}"
    );
    assert_eq!(collection.fan_support, SensorSupport::Unsupported);
    assert_eq!(collection.sample, MotherboardSensorSample::default());
  }

  /// Unavailable before any input of the endpoint is opened.
  fn assert_rejected_without_input_reads(fixture: &Fixture) {
    fixture.node_reads();
    assert_unavailable(&fixture.sample());
    assert!(fixture.input_reads().is_empty());
  }

  // Spec "Synthetic fixture acceptance": HERO DMI, canonical endpoint,
  // loadable module with `initstate` and empty `taint`, S3 values.
  #[test]
  fn hero_reports_chipset_vrm_and_pch_fan_without_reading_excluded_inputs() {
    let fixture = Fixture::hero();
    fixture.node_reads();
    assert_available(&fixture.sample(), &s3_sample());

    let reads = fixture.node_reads();
    for excluded in EXCLUDED_INPUTS {
      assert!(!reads.iter().any(|read| read == excluded), "{excluded}");
    }
    let mut inputs: Vec<_> = reads
      .iter()
      .filter(|attribute| attribute.ends_with("_input"))
      .map(String::as_str)
      .collect();
    inputs.sort_unstable();
    assert_eq!(
      inputs,
      [
        "fan1_input",
        "fan2_input",
        "temp1_input",
        "temp4_input",
        "temp5_input",
        "temp6_input",
        "temp7_input",
      ]
    );
    assert!(
      !reads
        .iter()
        .any(|read| read.starts_with("in") || read.starts_with("curr"))
    );
  }

  // Spec "Synthetic fixture acceptance": T_Sensor 31000, Water_In 27000,
  // Water_Out 29000, CPU_Opt 1200.
  #[test]
  fn connected_headers_report_all_seven_readings() {
    let fixture = Fixture::hero();
    connect_headers(&fixture);
    assert_available(&fixture.sample(), &connected_sample());
  }

  // Spec "Synthetic fixture acceptance": blank headers return a read error
  // (v7.3 behavior); a lock timeout I/O error is the same read failure.
  #[test]
  fn input_read_errors_make_only_those_readings_unavailable() {
    let fixture = Fixture::hero();
    for attribute in ["temp4_input", "temp6_input", "temp7_input"] {
      fixture.make_unreadable(attribute);
    }
    assert_available(&fixture.sample(), &s3_sample());

    let fixture = Fixture::hero();
    connect_headers(&fixture);
    fixture.make_unreadable("temp1_input");
    fixture.make_unreadable("fan1_input");
    let mut expected = connected_sample();
    expected.temperatures.remove(0);
    expected.fan_speeds.remove(0);
    assert_available(&fixture.sample(), &expected);

    for attribute in [
      "temp4_input",
      "temp5_input",
      "temp6_input",
      "temp7_input",
      "fan2_input",
    ] {
      fixture.make_unreadable(attribute);
    }
    assert_unavailable(&fixture.sample());
  }

  // Spec "Synthetic fixture acceptance": HERO (WI-FI) or FORMULA DMI.
  #[test]
  fn hero_wifi_and_formula_are_admitted_identically() {
    for board in [HERO_WIFI, FORMULA] {
      let fixture = Fixture::board(board);
      assert_available(&fixture.sample(), &s3_sample());
      connect_headers(&fixture);
      assert_available(&fixture.sample(), &connected_sample());
    }
  }

  // Spec "Synthetic fixture acceptance": BIOS version missing, `4201` or any
  // other string; kernel release any string.
  #[test]
  fn admission_ignores_the_bios_version_and_reads_no_kernel_release() {
    let fixture = Fixture::hero();
    let bios = fixture.dmi().join("bios_version");
    for value in [Some("4201"), Some("4602"), Some("custom build"), None] {
      match value {
        Some(value) => fs::write(&bios, format!("{value}\n")).unwrap(),
        None => fs::remove_file(&bios).unwrap(),
      }
      READS.take();
      assert_available(&fixture.sample(), &s3_sample());
      let reads = READS.take();
      let module = fixture.module();
      for path in reads {
        assert!(
          path.starts_with(&fixture.node)
            || path == fixture.dmi().join("board_vendor")
            || path == fixture.dmi().join("board_name")
            || path == module.join("taint"),
          "unexpected read of {}",
          path.display()
        );
      }
    }
  }

  // Spec "Synthetic fixture acceptance": other ASUS board name with an
  // `asusec` endpoint, or a non-ASUS vendor string.
  #[test]
  fn other_boards_and_vendors_are_unsupported_without_reading_the_endpoint() {
    let fixture = Fixture::hero();
    let dmi = fixture.dmi();
    for (attribute, value) in [
      ("board_name", "ROG CROSSHAIR VIII DARK HERO"),
      ("board_name", "ROG CROSSHAIR VIII HERO "),
      ("board_name", "rog crosshair viii hero"),
      ("board_name", "PRIME X570-PRO"),
      ("board_vendor", "ASUSTeK Computer INC."),
      ("board_vendor", "Micro-Star International Co., Ltd."),
    ] {
      let path = dmi.join(attribute);
      let original = fs::read(&path).unwrap();
      fs::write(&path, value).unwrap();
      fixture.node_reads();
      assert_unsupported(&fixture.sample());
      assert!(fixture.node_reads().is_empty());
      fs::write(path, original).unwrap();
    }
    assert_available(&fixture.sample(), &s3_sample());
  }

  #[test]
  fn missing_identity_is_unavailable_and_one_line_ending_is_trimmed() {
    let fixture = Fixture::hero();
    let dmi = fixture.dmi();
    for attribute in ["board_vendor", "board_name"] {
      let path = dmi.join(attribute);
      let original = fs::read(&path).unwrap();
      fs::remove_file(&path).unwrap();
      assert_rejected_without_input_reads(&fixture);
      fs::create_dir(&path).unwrap();
      assert_rejected_without_input_reads(&fixture);
      fs::remove_dir(&path).unwrap();
      fs::write(path, original).unwrap();
    }

    fs::write(dmi.join("board_vendor"), format!("{ASUS_VENDOR}\r\n")).unwrap();
    fs::write(dmi.join("board_name"), HERO).unwrap();
    assert_available(&fixture.sample(), &s3_sample());
    fs::write(dmi.join("board_name"), format!("{HERO}\n\n")).unwrap();
    assert_unsupported(&fixture.sample());
  }

  // Spec "Synthetic fixture acceptance": hwmon name other than `asusec`,
  // wrong driver or subsystem link, class link to another device.
  #[test]
  fn rejects_unqualified_endpoints_before_any_input_read() {
    let fixture = Fixture::hero();
    for name in ["asus_ec\n", "asusec \n", "nct6775\n"] {
      fixture.write("name", name);
      assert_rejected_without_input_reads(&fixture);
    }
    fixture.write("name", "asusec\n");

    let device = fixture.device();
    let other_device = fixture.sysfs.join("devices/platform/other");
    fs::create_dir(&other_device).unwrap();
    let other_driver = fixture.sysfs.join("bus/platform/drivers/asus_wmi_sensors");
    fs::create_dir(&other_driver).unwrap();
    for (link, wrong_target) in [
      (device.join("driver"), other_driver.clone()),
      (device.join("subsystem"), fixture.sysfs.join("class")),
      (
        fixture
          .sysfs
          .join("bus/platform/devices")
          .join(PLATFORM_DEVICE),
        other_device.clone(),
      ),
      (fixture.node.join("device"), other_device.clone()),
      (
        fixture.sysfs.join("class/hwmon/hwmon3"),
        other_device.clone(),
      ),
    ] {
      let original = fs::read_link(&link).unwrap();
      fs::remove_file(&link).unwrap();
      assert_rejected_without_input_reads(&fixture);
      symlink(wrong_target, &link).unwrap();
      assert_rejected_without_input_reads(&fixture);
      fs::remove_file(&link).unwrap();
      symlink(original, link).unwrap();
    }

    // A class entry with another index is not the canonical child.
    let class = fixture.sysfs.join("class/hwmon");
    fs::remove_file(class.join("hwmon3")).unwrap();
    symlink(&fixture.node, class.join("hwmon9")).unwrap();
    assert_rejected_without_input_reads(&fixture);
    fs::remove_file(class.join("hwmon9")).unwrap();
    symlink(&fixture.node, class.join("hwmon3")).unwrap();
    assert_available(&fixture.sample(), &s3_sample());
  }

  // Spec "Synthetic fixture acceptance": two children.
  #[test]
  fn rejects_a_second_hwmon_child() {
    let fixture = Fixture::hero();
    let device = fixture.device();
    let second = device.join("hwmon/hwmon4");
    fs::create_dir(&second).unwrap();
    assert_rejected_without_input_reads(&fixture);
    symlink(&device, second.join("device")).unwrap();
    fs::write(second.join("name"), "asusec\n").unwrap();
    symlink(&second, fixture.sysfs.join("class/hwmon/hwmon4")).unwrap();
    assert_rejected_without_input_reads(&fixture);
  }

  // Spec "Synthetic fixture acceptance": device with an instance suffix.
  #[test]
  fn rejects_a_device_with_an_instance_suffix() {
    for device in ["asus-ec-sensors.0", "asus-ec-sensors.1"] {
      let fixture = Fixture::empty(ASUS_VENDOR, HERO, device, "hwmon3");
      fixture.write_channels("temp", &TEMPERATURE_LABELS, &S3_TEMPERATURES);
      fixture.write_channels("fan", &FAN_LABELS, &S3_FANS);
      assert_rejected_without_input_reads(&fixture);
    }
  }

  #[test]
  fn rejects_an_unbound_device() {
    let fixture = Fixture::hero();
    fs::remove_file(fixture.device().join("driver")).unwrap();
    assert_rejected_without_input_reads(&fixture);
  }

  // Spec "Synthetic fixture acceptance": module directory with `initstate`
  // and `taint` `O` or `OE`, or with `initstate` and a missing or
  // unreadable `taint`.
  #[test]
  fn rejects_out_of_tree_and_incomplete_loadable_modules() {
    let fixture = Fixture::hero();
    let taint = fixture.module().join("taint");
    for value in ["O\n", "OE\n", "PO\n", "O"] {
      fs::write(&taint, value).unwrap();
      assert_rejected_without_input_reads(&fixture);
    }
    for value in ["E\n", "P\n", ""] {
      fs::write(&taint, value).unwrap();
      assert_available(&fixture.sample(), &s3_sample());
    }

    fs::remove_file(&taint).unwrap();
    assert_rejected_without_input_reads(&fixture);
    fs::create_dir(&taint).unwrap();
    assert_rejected_without_input_reads(&fixture);
  }

  // Spec "Synthetic fixture acceptance": module directory with neither
  // `initstate` nor `taint`, driver `module` link resolving to it.
  #[test]
  fn admits_a_built_in_driver() {
    let fixture = Fixture::hero();
    for attribute in ["initstate", "taint"] {
      fs::remove_file(fixture.module().join(attribute)).unwrap();
    }
    assert_available(&fixture.sample(), &s3_sample());
  }

  // Spec "Synthetic fixture acceptance": built-in driver without a module
  // directory or driver `module` link (the known fail-closed limitation);
  // `taint` but no `initstate`; module directory absent; `module` link
  // missing or resolving elsewhere.
  #[test]
  fn rejects_unverifiable_module_forms() {
    let driver_module = |fixture: &Fixture| {
      fixture
        .sysfs
        .join("bus/platform/drivers")
        .join(PLATFORM_DRIVER)
        .join("module")
    };

    let fixture = Fixture::hero();
    fs::remove_file(fixture.module().join("initstate")).unwrap();
    assert_rejected_without_input_reads(&fixture);

    let fixture = Fixture::hero();
    fs::remove_file(driver_module(&fixture)).unwrap();
    fs::remove_dir_all(fixture.module()).unwrap();
    assert_rejected_without_input_reads(&fixture);

    let fixture = Fixture::hero();
    fs::remove_dir_all(fixture.module()).unwrap();
    assert_rejected_without_input_reads(&fixture);

    for built_in in [false, true] {
      let fixture = Fixture::hero();
      if built_in {
        for attribute in ["initstate", "taint"] {
          fs::remove_file(fixture.module().join(attribute)).unwrap();
        }
      }
      let link = driver_module(&fixture);
      fs::remove_file(&link).unwrap();
      assert_rejected_without_input_reads(&fixture);
      let other = fixture.sysfs.join("module/asus_ec_sensors_dkms");
      fs::create_dir(&other).unwrap();
      symlink(&other, &link).unwrap();
      assert_rejected_without_input_reads(&fixture);
    }
  }

  #[test]
  fn an_inaccessible_module_directory_is_not_taken_for_a_built_in_driver() {
    let fixture = Fixture::hero();
    for attribute in ["initstate", "taint"] {
      fs::remove_file(fixture.module().join(attribute)).unwrap();
    }
    let module = fixture.module();
    fs::set_permissions(&module, fs::Permissions::from_mode(0o000)).unwrap();
    // Root can stat entries regardless of the mode and then sees a
    // confirmed absence, so the assertion applies to other users only.
    let inaccessible = fs::symlink_metadata(module.join("initstate"))
      .is_err_and(|error| error.kind() != ErrorKind::NotFound);
    let collection = fixture.sample();
    fs::set_permissions(&module, fs::Permissions::from_mode(0o755)).unwrap();
    if inaccessible {
      assert_unavailable(&collection);
    }
  }

  // Spec "Synthetic fixture acceptance": label missing, unreadable,
  // different, or the same label on two channels of one type.
  #[test]
  fn label_problems_exclude_only_that_reading() {
    let connected = connected_sample();
    let without = |temperature: Option<&str>, fan: Option<&str>| {
      let mut sample = connected.clone();
      sample
        .temperatures
        .retain(|reading| Some(reading.name.as_str()) != temperature);
      sample
        .fan_speeds
        .retain(|reading| Some(reading.name.as_str()) != fan);
      sample
    };

    let fixture = Fixture::hero();
    connect_headers(&fixture);
    fs::remove_file(fixture.node.join("temp5_label")).unwrap();
    fixture.node_reads();
    assert_available(&fixture.sample(), &without(Some("VRM"), None));
    assert!(!fixture.input_reads().contains(&"temp5_input".to_owned()));

    let fixture = Fixture::hero();
    connect_headers(&fixture);
    fixture.make_unreadable("fan2_label");
    assert_available(&fixture.sample(), &without(None, Some("PCH_FAN")));

    let fixture = Fixture::hero();
    connect_headers(&fixture);
    fixture.write("temp6_label", "Water In\n");
    fixture.write("fan1_label", "CPU_OPT\n");
    let mut expected = without(Some("W_IN"), Some("CPU_OPT"));
    assert_available(&fixture.sample(), &expected);

    // A duplicated temperature label excludes that temperature; the fan with
    // the same label is a different type.
    let fixture = Fixture::hero();
    connect_headers(&fixture);
    fixture.write("temp3_label", "Chipset\n");
    expected = without(Some("Chipset"), None);
    fixture.node_reads();
    assert_available(&fixture.sample(), &expected);
    let inputs = fixture.input_reads();
    assert!(!inputs.contains(&"temp1_input".to_owned()));
    assert!(!inputs.contains(&"temp3_input".to_owned()));

    let fixture = Fixture::hero();
    connect_headers(&fixture);
    fixture.write("fan3_label", "CPU_Opt\n");
    assert_available(&fixture.sample(), &without(None, Some("CPU_OPT")));

    // A label with a trailing space or a second line ending is different.
    let fixture = Fixture::hero();
    connect_headers(&fixture);
    fixture.write("temp1_label", "Chipset \n");
    fixture.write("fan2_label", "Chipset\n\n");
    assert_available(
      &fixture.sample(),
      &without(Some("Chipset"), Some("PCH_FAN")),
    );
  }

  #[test]
  fn locates_channels_by_label_not_by_number() {
    let fixture = Fixture::empty(ASUS_VENDOR, HERO, PLATFORM_DEVICE, "hwmon12");
    let mut labels = TEMPERATURE_LABELS;
    let mut inputs = [
      "58000", "40000", "34000", "31000", "28000", "27000", "29000",
    ];
    labels.reverse();
    inputs.reverse();
    fixture.write_channels("temp", &labels, &inputs);
    fixture.write_channels(
      "fan",
      &["Water_Flow", "Chipset", "CPU_Opt"],
      &["0", "1829", "1200"],
    );
    assert_available(&fixture.sample(), &connected_sample());

    for ignored in ["temp01_label", "temp_label", "tempx_label", "temp8_labels"] {
      fixture.write(ignored, "Chipset\n");
    }
    assert_available(&fixture.sample(), &connected_sample());
  }

  // Spec "Synthetic fixture acceptance": any scoped temperature at 0,
  // −55001 or 125001; boundaries −55000 and 125000.
  #[test]
  fn checks_temperature_bounds_and_zero_on_every_scoped_channel() {
    for blank_header in [false, true] {
      for (raw, expected) in [
        ("-55000", Some(-55.0)),
        ("125000", Some(125.0)),
        ("1", Some(0.001)),
        ("-1", Some(-0.001)),
        ("0", None),
        ("-0", None),
        ("-55001", None),
        ("125001", None),
        ("+58000", None),
        ("58000 ", None),
        ("58 000", None),
        ("", None),
        ("-", None),
        ("2147483648", None),
        ("-2147483649", None),
      ] {
        assert_eq!(decode_temperature(raw, blank_header), expected, "{raw}");
      }
    }

    let fixture = Fixture::hero();
    connect_headers(&fixture);
    for index in [1, 4, 5, 6, 7] {
      let attribute = format!("temp{index}_input");
      let original = fs::read(fixture.node.join(&attribute)).unwrap();
      for raw in ["0\n", "-55001\n", "125001\n"] {
        fixture.write(&attribute, raw);
        let sample = fixture.sample().sample;
        assert_eq!(sample.temperatures.len(), 4, "{attribute} {raw}");
        assert_eq!(sample.fan_speeds.len(), 2);
      }
      for (raw, value) in [("-55000\n", -55.0), ("125000\n", 125.0)] {
        fixture.write(&attribute, raw);
        let sample = fixture.sample().sample;
        assert_eq!(sample.temperatures.len(), 5);
        assert!(
          sample
            .temperatures
            .iter()
            .any(|reading| reading.temperature == value)
        );
      }
      fs::write(fixture.node.join(&attribute), original).unwrap();
    }
  }

  // Spec "Synthetic fixture acceptance": `T_Sensor`, `Water_In` or
  // `Water_Out` at −40000, −60000 or −62000; `Chipset` or `VRM` at −40000.
  #[test]
  fn blank_values_apply_only_to_the_three_header_channels() {
    for raw in ["-40000", "-60000", "-62000"] {
      assert_eq!(decode_temperature(raw, true), None);
    }
    assert_eq!(decode_temperature("-40000", false), Some(-40.0));
    assert_eq!(decode_temperature("-60000", false), None);
    assert_eq!(decode_temperature("-41000", true), Some(-41.0));

    let fixture = Fixture::hero();
    connect_headers(&fixture);
    for blank in ["-40000\n", "-60000\n", "-62000\n"] {
      for index in [4, 6, 7] {
        fixture.write(&format!("temp{index}_input"), blank);
      }
      assert_available(&fixture.sample(), &{
        let mut expected = connected_sample();
        expected.temperatures.truncate(2);
        expected
      });
    }

    fixture.write("temp1_input", "-40000\n");
    fixture.write("temp5_input", "-40000\n");
    let sample = fixture.sample().sample;
    assert_eq!(
      sample.temperatures,
      [temperature("Chipset", -40.0), temperature("VRM", -40.0)]
    );
  }

  // Spec "Synthetic fixture acceptance": fan −1, 0, 65535, malformed or
  // overflowing; fan 1 and 65534.
  #[test]
  fn checks_fan_bounds_and_never_reports_zero_rpm() {
    for (raw, expected) in [
      ("1", Some(1)),
      ("65534", Some(65_534)),
      ("1829", Some(1829)),
      ("-1", None),
      ("0", None),
      ("65535", None),
      ("4294967296", None),
      ("+1829", None),
      ("1829 RPM", None),
      ("", None),
    ] {
      assert_eq!(decode_fan(raw), expected, "{raw}");
    }

    let fixture = Fixture::hero();
    connect_headers(&fixture);
    for raw in ["-1\n", "0\n", "65535\n", "garbage\n", "99999999999\n"] {
      fixture.write("fan2_input", raw);
      let collection = fixture.sample();
      assert_eq!(collection.availability, SensorAvailability::Available);
      assert_eq!(collection.sample.fan_speeds, [fan("CPU_OPT", 1200)]);
      assert!(
        collection
          .sample
          .fan_speeds
          .iter()
          .all(|reading| reading.rpm != Some(0))
      );
    }
    for (raw, rpm) in [("1\n", 1), ("65534\n", 65_534)] {
      fixture.write("fan2_input", raw);
      assert_eq!(fixture.sample().sample.fan_speeds[1], fan("PCH_FAN", rpm));
    }
  }

  // Spec "Synthetic fixture acceptance": endpoint replaced or `hwmonN`
  // renumbered during collection.
  #[test]
  fn discards_readings_when_the_endpoint_changes_during_collection() {
    let fixture = Fixture::hero();
    let original = fixture.endpoint().unwrap();

    // Replaced hwmon child at the same path.
    let old = fixture.node.with_file_name("old");
    fs::rename(&fixture.node, &old).unwrap();
    fs::create_dir(&fixture.node).unwrap();
    symlink(fixture.device(), fixture.node.join("device")).unwrap();
    fixture.write("name", "asusec\n");
    fs::remove_dir_all(&old).unwrap();
    let replacement = fixture.endpoint().unwrap();
    assert_eq!(replacement.path, original.path);
    assert_ne!(replacement, original);
    assert_unavailable(&finish_sample(&fixture.sysfs, &original, s3_sample()));

    // Rebound driver.
    let fixture = Fixture::hero();
    let original = fixture.endpoint().unwrap();
    let driver_link = fixture.device().join("driver");
    let target = fs::read_link(&driver_link).unwrap();
    fs::rename(&driver_link, fixture.device().join("old-driver")).unwrap();
    symlink(target, &driver_link).unwrap();
    assert_unavailable(&finish_sample(&fixture.sysfs, &original, s3_sample()));

    // Reloaded module.
    let fixture = Fixture::hero();
    let original = fixture.endpoint().unwrap();
    let module = fixture.module();
    fs::rename(&module, module.with_file_name("old-module")).unwrap();
    fs::create_dir(&module).unwrap();
    fs::write(module.join("initstate"), "live\n").unwrap();
    fs::write(module.join("taint"), "\n").unwrap();
    assert_unavailable(&finish_sample(&fixture.sysfs, &original, s3_sample()));
    assert_available(&fixture.sample(), &s3_sample());
  }

  #[test]
  fn a_renumbered_child_qualifies_again_on_the_next_pass() {
    let fixture = Fixture::hero();
    let original = fixture.endpoint().unwrap();
    let renamed = fixture.node.with_file_name("hwmon7");
    fs::rename(&fixture.node, &renamed).unwrap();
    let class = fixture.sysfs.join("class/hwmon");
    fs::remove_file(class.join("hwmon3")).unwrap();
    symlink(&renamed, class.join("hwmon7")).unwrap();

    assert_unavailable(&finish_sample(&fixture.sysfs, &original, s3_sample()));
    assert_available(&sample_at(&fixture.sysfs), &s3_sample());
  }

  #[test]
  fn sampler_starts_passes_at_least_one_second_apart() {
    let start = Instant::now();
    let mut state = Sampler::new().state.into_inner().unwrap();
    assert!(state.begin(start));
    assert!(
      !state.begin(start + Duration::from_secs(2)),
      "pass in flight"
    );
    state.finish(start + Duration::from_millis(900), s3_collection());
    assert!(!state.begin(start + Duration::from_millis(999)));
    assert!(state.begin(start + MIN_PASS_INTERVAL));
    state.abandon();
    assert!(!state.begin(start + Duration::from_millis(1999)));
    assert!(state.begin(start + Duration::from_secs(2)));
  }

  #[test]
  fn sampler_reports_only_a_recent_completed_pass() {
    let start = Instant::now();
    let mut state = Sampler::new().state.into_inner().unwrap();
    assert_eq!(state.current(start), None);
    state.finish(start, s3_collection());
    assert_eq!(state.current(start + MAX_RESULT_AGE), Some(s3_collection()));
    assert_eq!(
      state.current(start + MAX_RESULT_AGE + Duration::from_millis(1)),
      None
    );
  }

  #[test]
  fn sampler_answers_other_boards_from_dmi_without_a_pass() {
    let sampler: &'static Sampler = Box::leak(Box::new(Sampler::new()));
    let fixture = Fixture::hero();
    fs::write(fixture.dmi().join("board_name"), "PRIME X570-PRO\n").unwrap();
    let collection =
      sampler.sample(fixture.sysfs.clone(), |_| panic!("no pass expected"));
    assert_unsupported(&collection);
    fs::remove_file(fixture.dmi().join("board_vendor")).unwrap();
    assert_unavailable(
      &sampler.sample(fixture.sysfs.clone(), |_| panic!("no pass expected")),
    );
  }

  #[test]
  fn sampler_reports_the_pass_result_and_survives_a_failed_spawn() {
    let sampler: &'static Sampler = Box::leak(Box::new(Sampler::new()));
    let fixture = Fixture::hero();
    let failed = sampler.sample(fixture.sysfs.clone(), |_| Err(ErrorKind::Other.into()));
    assert_unavailable(&failed);
    assert!(!sampler.lock().in_flight);

    sampler.lock().last_start = None;
    let collection = sampler.sample(fixture.sysfs.clone(), |pass| {
      pass();
      Ok(())
    });
    assert_eq!(collection, s3_collection());
  }

  fn s3_collection() -> MotherboardSensorCollection {
    MotherboardSensorCollection {
      sample: s3_sample(),
      availability: SensorAvailability::Available,
      fan_support: SensorSupport::Supported,
      ..MotherboardSensorCollection::default()
    }
  }
}
