//! Primary ITE (IT8696E) motherboard temperatures on the Gigabyte B850M AORUS
//! ELITE WIFI6E ICE, read through the already-bound native `gigabyte-wmi`
//! hwmon endpoint.
//!
//! Implemented from `docs/specs/sensors/superio-ite-b850m-wifi6e-ice-linux-wmi.md`
//! revision 2. The app only opens existing sysfs attributes for reading. It
//! never calls a WMI or ACPI method, binds a driver, or touches the chip.

use super::sysfs::{file_identity, is_hwmon_name, resolves_to};
use crate::models::{
  MotherboardSensorCollection, MotherboardSensorSample, MotherboardTemperature,
  SensorAvailability, SensorSupport,
};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// DMI `sys_vendor` and `board_name` of the one board whose firmware maps the
/// WMI temperature selectors to the primary monitor ("Detection").
const SYS_VENDOR: &str = "Gigabyte Technology Co., Ltd.";
const BOARD_NAME: &str = "B850M AORUS ELITE WIFI6E ICE";
/// The WMI GUID that the native endpoint advertises ("Detection").
const WMI_GUID: &str = "DEADBEEF-2001-0000-00A0-C90629100000";
const WMI_DRIVER: &str = "gigabyte-wmi";
const HWMON_NAME: &str = "gigabyte_wmi";
/// The raw IT87 platform binding that overlaps the primary monitor at base
/// `0x0A40`. Its name identifies the conflict; the app never uses the base
/// as an address ("Detection", conflict gate).
const CONFLICTING_DEVICE: &str = "it87.2624";
const CONFLICTING_DRIVER: &str = "it87";
/// `temp1_input` through `temp5_input` are the qualified channels. The sixth
/// possible hardware channel stays unqualified ("Firmware and collection
/// facts").
const TEMPERATURE_CHANNELS: std::ops::RangeInclusive<u8> = 1..=5;
/// The existing Linux motherboard plausibility range, inclusive, in
/// millidegrees Celsius.
const TEMPERATURE_RANGE_MILLI_CELSIUS: std::ops::RangeInclusive<i32> = -55_000..=125_000;
const MILLI_PER_DEGREE: i32 = 1_000;
const SOURCE: &str = "IT8696E (gigabyte-wmi)";

const UNSUPPORTED_BOARD: &str =
  "No Gigabyte WMI temperature profile for this Linux motherboard";
const ENDPOINT_UNAVAILABLE: &str =
  "The bound gigabyte-wmi hwmon endpoint is unavailable or ambiguous";
const CONFLICT_INCOMPLETE: &str =
  "Could not confirm that no it87 driver overlaps the Gigabyte WMI temperatures";
const CONFLICT_FOUND: &str = "The it87 driver is bound to the primary monitor that the Gigabyte WMI temperatures read";
const BINDING_CHANGED: &str = "The gigabyte-wmi hwmon binding changed during collection";
const NO_USABLE_READINGS: &str =
  "The gigabyte-wmi hwmon endpoint returned no usable temperatures";

pub(crate) fn sample() -> MotherboardSensorCollection {
  sample_at(Path::new("/sys"))
}

fn sample_at(sysfs: &Path) -> MotherboardSensorCollection {
  if !is_qualified_board(&sysfs.join("class/dmi/id")) {
    return MotherboardSensorCollection::unsupported(UNSUPPORTED_BOARD);
  }
  let endpoint = match qualify_endpoint(sysfs) {
    Ok(endpoint) => endpoint,
    Err(reason) => return unavailable(reason),
  };
  let temperatures = TEMPERATURE_CHANNELS
    .filter_map(|channel| read_temperature(&endpoint.hwmon, channel))
    .collect();
  finish_sample(sysfs, &endpoint, temperatures)
}

/// Unsupported boards, including an unreadable identity, leave the decision to
/// the other Linux motherboard providers. The BIOS version is evidence only
/// and is never read.
fn is_qualified_board(dmi: &Path) -> bool {
  read_attribute(dmi, "sys_vendor").as_deref() == Some(SYS_VENDOR)
    && read_attribute(dmi, "board_name").as_deref() == Some(BOARD_NAME)
}

fn finish_sample(
  sysfs: &Path,
  endpoint: &Endpoint,
  temperatures: Vec<MotherboardTemperature>,
) -> MotherboardSensorCollection {
  // A removed or replaced binding, or a newly bound conflict, invalidates the
  // readings already taken in this sample.
  if qualify_endpoint(sysfs).as_ref() != Ok(endpoint) {
    return unavailable(BINDING_CHANGED);
  }
  let availability = if temperatures.is_empty() {
    SensorAvailability::unavailable(NO_USABLE_READINGS)
  } else {
    SensorAvailability::Available
  };
  MotherboardSensorCollection {
    sample: MotherboardSensorSample {
      temperatures,
      fan_speeds: Vec::new(),
    },
    availability,
    fan_support: SensorSupport::Unsupported,
    ..MotherboardSensorCollection::default()
  }
}

/// Fan RPM is outside this endpoint's scope on the matching board, so the
/// fan support stays Unsupported even when the temperatures are unavailable.
fn unavailable(reason: &str) -> MotherboardSensorCollection {
  MotherboardSensorCollection {
    availability: SensorAvailability::unavailable(reason),
    fan_support: SensorSupport::Unsupported,
    ..MotherboardSensorCollection::default()
  }
}

#[derive(Debug, PartialEq, Eq)]
struct Endpoint {
  hwmon: PathBuf,
  /// The hwmon directory, the WMI device and its driver link.
  identities: [(u64, u64); 3],
}

/// Check ownership and the known conflict before any input is read.
fn qualify_endpoint(sysfs: &Path) -> Result<Endpoint, &'static str> {
  let device = find_wmi_device(sysfs).ok_or(ENDPOINT_UNAVAILABLE)?;
  let hwmon = find_owned_hwmon(sysfs, &device).ok_or(ENDPOINT_UNAVAILABLE)?;
  check_conflict(sysfs)?;
  let identity = |metadata: std::io::Result<fs::Metadata>| {
    metadata
      .map(|metadata| file_identity(&metadata))
      .map_err(|_| ENDPOINT_UNAVAILABLE)
  };
  let identities = [
    identity(fs::metadata(&hwmon))?,
    identity(fs::metadata(&device))?,
    identity(fs::symlink_metadata(device.join("driver")))?,
  ];
  Ok(Endpoint { hwmon, identities })
}

/// Find the one WMI bus device for the GUID, counting suffixed duplicates,
/// and require that it is bound to the native driver and describes one
/// instance. Returns the canonical device directory.
fn find_wmi_device(sysfs: &Path) -> Option<PathBuf> {
  let bus = sysfs.join("bus/wmi");
  let mut candidates = Vec::new();
  for entry in fs::read_dir(bus.join("devices")).ok()? {
    let entry = entry.ok()?;
    let name = entry.file_name().into_string().ok()?;
    // Every enumerated device describes its GUID; one that cannot be read
    // might be a duplicate.
    let guid = read_attribute(&entry.path(), "guid")?;
    let advertises = is_wmi_guid(&guid);
    let suffixed = name.get(..WMI_GUID.len()).is_some_and(is_wmi_guid);
    if advertises || suffixed {
      candidates.push((name, advertises));
    }
  }
  let [(name, true)] = candidates.as_slice() else {
    return None;
  };

  let device = fs::canonicalize(bus.join("devices").join(name)).ok()?;
  let qualified = device.starts_with(sysfs.join("devices"))
    && device.file_name() == Some(std::ffi::OsStr::new(name))
    && resolves_to(&device.join("subsystem"), &bus)
    && resolves_to(
      &device.join("driver"),
      &bus.join("drivers").join(WMI_DRIVER),
    )
    && read_attribute(&device, "guid").is_some_and(|guid| is_wmi_guid(&guid))
    && read_attribute(&device, "instance_count").as_deref() == Some("1");
  qualified.then_some(device)
}

/// GUIDs compare without regard to hexadecimal letter case.
fn is_wmi_guid(text: &str) -> bool {
  text.eq_ignore_ascii_case(WMI_GUID)
}

/// Require exactly one hwmon child registered under the WMI device, named
/// `gigabyte_wmi`, whose class link and `device` link agree with it. A
/// matching name elsewhere in sysfs never establishes ownership.
fn find_owned_hwmon(sysfs: &Path, device: &Path) -> Option<PathBuf> {
  let mut children = fs::read_dir(device.join("hwmon")).ok()?;
  let child = children.next()?.ok()?;
  if children.next().is_some() {
    return None;
  }
  let class_name = child.file_name();
  let hwmon = device.join("hwmon").join(&class_name);
  let qualified = class_name.to_str().is_some_and(is_hwmon_name)
    && resolves_to(&hwmon, &hwmon)
    && resolves_to(&sysfs.join("class/hwmon").join(&class_name), &hwmon)
    && resolves_to(&hwmon.join("device"), device)
    && read_attribute(&hwmon, "name").as_deref() == Some(HWMON_NAME);
  qualified.then_some(hwmon)
}

/// A bound `it87.2624` owned by the platform-bus `it87` driver is a conflict.
/// A loaded driver without that bound device is not. Metadata that cannot be
/// read leaves the check incomplete, which also blocks sampling.
fn check_conflict(sysfs: &Path) -> Result<(), &'static str> {
  let platform = sysfs.join("bus/platform");
  let devices = platform.join("devices");
  let driver = platform.join("drivers").join(CONFLICTING_DRIVER);
  fs::read_dir(&devices).map_err(|_| CONFLICT_INCOMPLETE)?;

  if entry_exists(&driver.join(CONFLICTING_DEVICE))? {
    return Err(CONFLICT_FOUND);
  }
  let device_link = devices.join(CONFLICTING_DEVICE);
  if !entry_exists(&device_link)? {
    return Ok(());
  }
  let device = fs::canonicalize(device_link).map_err(|_| CONFLICT_INCOMPLETE)?;
  let device_driver = device.join("driver");
  if !entry_exists(&device_driver)? {
    return Ok(());
  }
  let bound = fs::canonicalize(device_driver).map_err(|_| CONFLICT_INCOMPLETE)?;
  if bound == driver {
    Err(CONFLICT_FOUND)
  } else {
    Ok(())
  }
}

/// Stat a sysfs entry without following or opening it.
fn entry_exists(path: &Path) -> Result<bool, &'static str> {
  match fs::symlink_metadata(path) {
    Ok(_) => Ok(true),
    Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
    Err(_) => Err(CONFLICT_INCOMPLETE),
  }
}

fn read_attribute(directory: &Path, attribute: &str) -> Option<String> {
  let path = directory.join(attribute);
  #[cfg(test)]
  tests::record_read(&path);
  let mut text = fs::read_to_string(path).ok()?;
  text.truncate(text.trim_end_matches(['\r', '\n']).len());
  Some(text)
}

fn read_temperature(hwmon: &Path, channel: u8) -> Option<MotherboardTemperature> {
  let raw = read_attribute(hwmon, &format!("temp{channel}_input"))?;
  decode_temperature(&raw).map(|temperature| MotherboardTemperature {
    name: format!("Temperature {channel}"),
    temperature,
    source: SOURCE.to_owned(),
  })
}

/// Decode one `tempN_input` value in millidegrees Celsius. The firmware
/// reports whole degrees and uses zero as an error, so zero and fractional
/// degrees are rejected before the plausibility range applies.
fn decode_temperature(raw: &str) -> Option<f32> {
  let milli_celsius = raw.parse::<i32>().ok()?;
  (milli_celsius != 0
    && milli_celsius % MILLI_PER_DEGREE == 0
    && TEMPERATURE_RANGE_MILLI_CELSIUS.contains(&milli_celsius))
  .then_some((milli_celsius / MILLI_PER_DEGREE) as f32)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::cell::RefCell;
  use std::os::unix::fs::symlink;
  use tempfile::TempDir;

  thread_local! {
    /// Every attribute path that `read_attribute` opened on this test thread.
    static READS: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
  }

  pub(super) fn record_read(path: &Path) {
    READS.with_borrow_mut(|reads| reads.push(path.to_path_buf()));
  }

  /// The F4 owner sample's BIOS version (spec S3). An evidence identity only.
  const OBSERVED_BIOS: &str = "F4";
  /// The S3 live WMI temperatures, 21/37/34/24/27 degrees Celsius, as
  /// synthetic millidegree inputs for channels 1-5.
  const S3_MILLI_CELSIUS: [i32; 5] = [21_000, 37_000, 34_000, 24_000, 27_000];
  const HWMON: &str = "hwmon3";
  /// The only attributes the app may open under the qualified hwmon child.
  const READ_SET: [&str; 6] = [
    "name",
    "temp1_input",
    "temp2_input",
    "temp3_input",
    "temp4_input",
    "temp5_input",
  ];

  struct Fixture {
    _directory: TempDir,
    sysfs: PathBuf,
    device: PathBuf,
    node: PathBuf,
  }

  impl Fixture {
    /// Synthetic sysfs tree for "Fixture acceptance" row 1: exact DMI, one
    /// bound native WMI device with a consistent class child, no overlapping
    /// binding, and the S3 inputs. Unrelated WMI devices, a sixth input, a
    /// fan input, an alarm attribute and an `it87` driver with no bound
    /// device are present so the test proves they are ignored.
    fn new() -> Self {
      Self::with_mapper("PNP0C14:00")
    }

    /// The same tree with the WMI devices under another mapper instance.
    fn with_mapper(mapper: &str) -> Self {
      let directory = tempfile::tempdir().unwrap();
      let sysfs = directory.path().canonicalize().unwrap();
      let dmi = sysfs.join("class/dmi/id");
      fs::create_dir_all(&dmi).unwrap();
      for (attribute, value) in [
        ("sys_vendor", SYS_VENDOR),
        ("board_vendor", SYS_VENDOR),
        ("board_name", BOARD_NAME),
        ("bios_version", OBSERVED_BIOS),
      ] {
        fs::write(dmi.join(attribute), format!("{value}\n")).unwrap();
      }
      for directory in [
        "bus/wmi/devices",
        "bus/wmi/drivers/gigabyte-wmi",
        "bus/platform/devices",
        "bus/platform/drivers/it87",
        "class/hwmon",
      ] {
        fs::create_dir_all(sysfs.join(directory)).unwrap();
      }
      let device =
        add_wmi_device_under(&sysfs, mapper, WMI_GUID, WMI_GUID, Some(WMI_DRIVER));
      add_wmi_device_under(
        &sysfs,
        mapper,
        "05901221-D566-11D1-B2F0-00A0C9062910",
        "05901221-D566-11D1-B2F0-00A0C9062910",
        None,
      );
      let node = add_hwmon(&sysfs, &device, HWMON, HWMON_NAME);
      let fixture = Self {
        _directory: directory,
        sysfs,
        device,
        node,
      };
      for (channel, milli) in (1..).zip(S3_MILLI_CELSIUS) {
        fixture.write(&format!("temp{channel}_input"), &format!("{milli}\n"));
      }
      fixture.write("temp6_input", "30000\n");
      fixture.write("fan1_input", "1200\n");
      fixture.write("temp1_alarm", "0\n");
      fixture
    }

    fn write(&self, attribute: &str, value: &str) {
      fs::write(self.node.join(attribute), value).unwrap();
    }

    fn sample(&self) -> MotherboardSensorCollection {
      sample_at(&self.sysfs)
    }

    fn endpoint(&self) -> Endpoint {
      qualify_endpoint(&self.sysfs).unwrap()
    }

    /// Drain the read log and return the attributes opened under the hwmon
    /// child since the previous call.
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

    /// Replace `link` with one to `target`, returning the original target.
    fn relink(&self, link: &Path, target: &Path) -> PathBuf {
      let original = fs::read_link(link).unwrap();
      fs::remove_file(link).unwrap();
      symlink(target, link).unwrap();
      original
    }

    fn bind_platform_device(&self, name: &str, driver: Option<&str>) {
      let device = self.sysfs.join("devices/platform").join(name);
      fs::create_dir_all(&device).unwrap();
      symlink(&device, self.sysfs.join("bus/platform/devices").join(name)).unwrap();
      if let Some(driver) = driver {
        let driver = self.sysfs.join("bus/platform/drivers").join(driver);
        fs::create_dir_all(&driver).unwrap();
        symlink(&driver, device.join("driver")).unwrap();
        symlink(&device, driver.join(name)).unwrap();
      }
    }
  }

  /// A WMI bus device under a synthetic mapper parent for a `PNP0C14` ACPI
  /// device, with the passive `guid` and `instance_count` attributes.
  fn add_wmi_device(
    sysfs: &Path,
    name: &str,
    guid: &str,
    driver: Option<&str>,
  ) -> PathBuf {
    add_wmi_device_under(sysfs, "PNP0C14:00", name, guid, driver)
  }

  fn add_wmi_device_under(
    sysfs: &Path,
    mapper: &str,
    name: &str,
    guid: &str,
    driver: Option<&str>,
  ) -> PathBuf {
    let device = sysfs
      .join("devices/platform")
      .join(mapper)
      .join("mapper")
      .join(name);
    fs::create_dir_all(&device).unwrap();
    fs::write(device.join("guid"), format!("{guid}\n")).unwrap();
    fs::write(device.join("instance_count"), "1\n").unwrap();
    symlink(sysfs.join("bus/wmi"), device.join("subsystem")).unwrap();
    if let Some(driver) = driver {
      let driver = sysfs.join("bus/wmi/drivers").join(driver);
      fs::create_dir_all(&driver).unwrap();
      symlink(driver, device.join("driver")).unwrap();
    }
    symlink(&device, sysfs.join("bus/wmi/devices").join(name)).unwrap();
    device
  }

  fn add_hwmon(sysfs: &Path, device: &Path, hwmon: &str, name: &str) -> PathBuf {
    let node = device.join("hwmon").join(hwmon);
    fs::create_dir_all(&node).unwrap();
    symlink(device, node.join("device")).unwrap();
    symlink(&node, sysfs.join("class/hwmon").join(hwmon)).unwrap();
    fs::write(node.join("name"), format!("{name}\n")).unwrap();
    node
  }

  fn reading(channel: u8, temperature: f32) -> MotherboardTemperature {
    MotherboardTemperature {
      name: format!("Temperature {channel}"),
      temperature,
      source: SOURCE.to_owned(),
    }
  }

  fn s3_readings() -> Vec<MotherboardTemperature> {
    (1..)
      .zip([21.0, 37.0, 34.0, 24.0, 27.0])
      .map(|(channel, temperature)| reading(channel, temperature))
      .collect()
  }

  fn assert_s3_sample(collection: &MotherboardSensorCollection) {
    assert_eq!(collection.availability, SensorAvailability::Available);
    assert_eq!(collection.fan_support, SensorSupport::Unsupported);
    assert_eq!(collection.sample.temperatures, s3_readings());
    assert!(collection.sample.fan_speeds.is_empty());
  }

  fn assert_unavailable(collection: &MotherboardSensorCollection) {
    assert!(
      matches!(
        collection.availability,
        SensorAvailability::Unavailable { .. }
      ),
      "{:?}",
      collection.availability
    );
    assert_eq!(collection.sample, MotherboardSensorSample::default());
  }

  fn assert_reads_within(reads: &[String], allowed: &[&str]) {
    for attribute in reads {
      assert!(
        allowed.contains(&attribute.as_str()),
        "unexpected read of {attribute}"
      );
    }
  }

  /// Assert that the sample is unavailable and no input was opened.
  fn assert_unavailable_without_input_reads(fixture: &Fixture) {
    fixture.node_reads();
    assert_unavailable(&fixture.sample());
    assert_reads_within(&fixture.node_reads(), &["name"]);
  }

  // Fixture acceptance row 1.
  #[test]
  fn reports_the_five_s3_temperatures_and_reads_nothing_else() {
    let fixture = Fixture::new();
    fixture.node_reads();
    assert_s3_sample(&fixture.sample());
    let reads = fixture.node_reads();
    assert_reads_within(&reads, &READ_SET);
    for attribute in &READ_SET {
      assert!(reads.iter().any(|read| read == attribute), "{attribute}");
    }
  }

  // Fixture acceptance row 2.
  #[test]
  fn admission_ignores_the_bios_version() {
    let fixture = Fixture::new();
    let bios = fixture.sysfs.join("class/dmi/id/bios_version");
    for version in ["F11", "F12e", ""] {
      fs::write(&bios, format!("{version}\n")).unwrap();
      assert_s3_sample(&fixture.sample());
    }
    fs::remove_file(&bios).unwrap();
    assert_s3_sample(&fixture.sample());
    let reads = READS.take();
    assert!(!reads.iter().any(|path| path.ends_with("bios_version")));
  }

  // Fixture acceptance row 2: the ACPI instance suffix and the hwmon number
  // are not admission conditions either.
  #[test]
  fn admission_ignores_the_hwmon_number_and_the_mapper_instance() {
    let fixture = Fixture::new();
    let renamed = fixture.node.with_file_name("hwmon12");
    fs::rename(&fixture.node, &renamed).unwrap();
    fs::remove_file(fixture.sysfs.join("class/hwmon").join(HWMON)).unwrap();
    symlink(&renamed, fixture.sysfs.join("class/hwmon/hwmon12")).unwrap();
    assert_s3_sample(&fixture.sample());
    assert_s3_sample(&Fixture::with_mapper("PNP0C14:03").sample());
  }

  // Fixture acceptance row 3.
  #[test]
  fn the_same_endpoint_on_another_board_is_unsupported_without_input_reads() {
    let fixture = Fixture::new();
    let dmi = fixture.sysfs.join("class/dmi/id");
    for (vendor, board) in [
      (SYS_VENDOR, "B850M AORUS ELITE WIFI6E"),
      (SYS_VENDOR, "B850M AORUS ELITE WIFI6E ICE "),
      (SYS_VENDOR, "X870E AORUS MASTER"),
      ("Gigabyte Technology Co., Ltd", BOARD_NAME),
      ("GIGABYTE", BOARD_NAME),
      ("Micro-Star International Co., Ltd.", BOARD_NAME),
    ] {
      fs::write(dmi.join("sys_vendor"), vendor).unwrap();
      fs::write(dmi.join("board_name"), board).unwrap();
      fixture.node_reads();
      let collection = fixture.sample();
      assert!(
        matches!(
          collection.availability,
          SensorAvailability::Unsupported { .. }
        ),
        "{vendor} / {board}"
      );
      assert_eq!(collection.sample, MotherboardSensorSample::default());
      assert!(fixture.node_reads().is_empty());
    }

    fs::write(dmi.join("sys_vendor"), SYS_VENDOR).unwrap();
    fs::write(dmi.join("board_name"), BOARD_NAME).unwrap();
    for attribute in ["sys_vendor", "board_name"] {
      let path = dmi.join(attribute);
      let original = fs::read(&path).unwrap();
      fs::remove_file(&path).unwrap();
      fixture.node_reads();
      assert!(matches!(
        fixture.sample().availability,
        SensorAvailability::Unsupported { .. }
      ));
      assert!(fixture.node_reads().is_empty());
      fs::write(path, original).unwrap();
    }
    // `board_vendor` is not the qualifying vendor attribute.
    fs::write(dmi.join("board_vendor"), "Other\n").unwrap();
    fs::write(dmi.join("sys_vendor"), "Gigabyte Technology Co., Ltd.\r\n").unwrap();
    assert_s3_sample(&fixture.sample());
  }

  // Fixture acceptance row 4: missing native owner and wrong driver.
  #[test]
  fn requires_the_native_wmi_driver_binding() {
    let fixture = Fixture::new();
    let driver_link = fixture.device.join("driver");
    let native = fs::read_link(&driver_link).unwrap();
    fs::remove_file(&driver_link).unwrap();
    assert_unavailable_without_input_reads(&fixture);

    let other = fixture.sysfs.join("bus/wmi/drivers/wmi-bmof");
    fs::create_dir_all(&other).unwrap();
    symlink(&other, &driver_link).unwrap();
    assert_unavailable_without_input_reads(&fixture);
    // A driver of the same name on another bus is not the native owner.
    let platform_driver = fixture.sysfs.join("bus/platform/drivers/gigabyte-wmi");
    fs::create_dir_all(&platform_driver).unwrap();
    fixture.relink(&driver_link, &platform_driver);
    assert_unavailable_without_input_reads(&fixture);

    fixture.relink(&driver_link, &native);
    assert_s3_sample(&fixture.sample());
  }

  // Fixture acceptance row 4: spoofed class path and different device link.
  #[test]
  fn requires_consistent_class_and_device_links() {
    let fixture = Fixture::new();
    let class_link = fixture.sysfs.join("class/hwmon").join(HWMON);
    let elsewhere = fixture.sysfs.join("devices/virtual/hwmon").join(HWMON);
    fs::create_dir_all(&elsewhere).unwrap();
    fs::write(elsewhere.join("name"), "gigabyte_wmi\n").unwrap();
    let original = fixture.relink(&class_link, &elsewhere);
    assert_unavailable_without_input_reads(&fixture);
    fs::remove_file(&class_link).unwrap();
    assert_unavailable_without_input_reads(&fixture);
    // A class link with another index is not the child's own class link.
    symlink(&original, fixture.sysfs.join("class/hwmon/hwmon9")).unwrap();
    assert_unavailable_without_input_reads(&fixture);
    fs::remove_file(fixture.sysfs.join("class/hwmon/hwmon9")).unwrap();
    symlink(&original, &class_link).unwrap();
    assert_s3_sample(&fixture.sample());

    let device_link = fixture.node.join("device");
    let other_device = add_wmi_device(
      &fixture.sysfs,
      "ABBC0F6A-8EA1-11D1-00A0-C90629100000",
      "ABBC0F6A-8EA1-11D1-00A0-C90629100000",
      None,
    );
    let original = fixture.relink(&device_link, &other_device);
    assert_unavailable_without_input_reads(&fixture);
    fixture.relink(&device_link, &fixture.sysfs.join("devices/platform"));
    assert_unavailable_without_input_reads(&fixture);
    fixture.relink(&device_link, &original);

    // The hwmon `name` alone never establishes ownership.
    fixture.write("name", "gigabyte_wmi \n");
    assert_unavailable_without_input_reads(&fixture);
    fixture.write("name", "it8696\n");
    assert_unavailable_without_input_reads(&fixture);
    fixture.write("name", "gigabyte_wmi\n");
    assert_s3_sample(&fixture.sample());
  }

  // Fixture acceptance row 4: duplicate GUID, including a suffixed name.
  #[test]
  fn refuses_duplicate_wmi_devices_for_the_guid() {
    for (name, guid) in [
      (format!("{WMI_GUID}-1"), WMI_GUID.to_owned()),
      (
        format!("{WMI_GUID}-1"),
        "00000000-0000-0000-0000-000000000000".to_owned(),
      ),
      (
        "DEADBEEF-2001-0000-00A0-C90629100001".to_owned(),
        WMI_GUID.to_ascii_lowercase(),
      ),
    ] {
      let fixture = Fixture::new();
      add_wmi_device(&fixture.sysfs, &name, &guid, None);
      assert_unavailable_without_input_reads(&fixture);
    }

    let fixture = Fixture::new();
    let bus = fixture.sysfs.join("bus/wmi/devices");
    // An unreadable GUID could hide a duplicate.
    let unknown = add_wmi_device(
      &fixture.sysfs,
      "8F1F6436-9F42-42C8-BADC-0E9424F20C9A",
      "8F1F6436-9F42-42C8-BADC-0E9424F20C9A",
      None,
    );
    fs::remove_file(unknown.join("guid")).unwrap();
    assert_unavailable_without_input_reads(&fixture);
    fs::write(
      unknown.join("guid"),
      "8F1F6436-9F42-42C8-BADC-0E9424F20C9A\n",
    )
    .unwrap();
    assert_s3_sample(&fixture.sample());

    fs::remove_file(bus.join(WMI_GUID)).unwrap();
    assert_unavailable_without_input_reads(&fixture);
    fs::remove_dir_all(fixture.sysfs.join("bus/wmi")).unwrap();
    assert_unavailable_without_input_reads(&fixture);
  }

  // Fixture acceptance row 4: the passive WMI metadata must describe one
  // instance of the GUID, and the device must be on the WMI bus.
  #[test]
  fn requires_the_passive_wmi_metadata_and_bus_links() {
    let fixture = Fixture::new();
    for (attribute, value) in [
      ("instance_count", Some("2\n")),
      ("instance_count", Some("0\n")),
      ("instance_count", None),
      ("guid", Some("DEADBEEF-2001-0000-00A0-C90629100001\n")),
      ("guid", None),
    ] {
      let path = fixture.device.join(attribute);
      let original = fs::read(&path).unwrap();
      match value {
        Some(value) => fs::write(&path, value).unwrap(),
        None => fs::remove_file(&path).unwrap(),
      }
      assert_unavailable_without_input_reads(&fixture);
      fs::write(path, original).unwrap();
    }
    fs::write(fixture.device.join("guid"), WMI_GUID.to_ascii_lowercase()).unwrap();
    assert_s3_sample(&fixture.sample());

    let subsystem = fixture.device.join("subsystem");
    let original = fixture.relink(&subsystem, &fixture.sysfs.join("bus/platform"));
    assert_unavailable_without_input_reads(&fixture);
    fixture.relink(&subsystem, &original);

    let bus_link = fixture.sysfs.join("bus/wmi/devices").join(WMI_GUID);
    let virtual_device = fixture.sysfs.join("elsewhere").join(WMI_GUID);
    fs::create_dir_all(&virtual_device).unwrap();
    let original = fixture.relink(&bus_link, &virtual_device);
    assert_unavailable_without_input_reads(&fixture);
    fixture.relink(&bus_link, &original);
    assert_s3_sample(&fixture.sample());
  }

  // Fixture acceptance row 4: duplicate hwmon child.
  #[test]
  fn refuses_more_than_one_hwmon_child_under_the_wmi_device() {
    for second_name in [HWMON_NAME, "other"] {
      let fixture = Fixture::new();
      add_hwmon(&fixture.sysfs, &fixture.device, "hwmon4", second_name);
      fs::write(fixture.device.join("hwmon/hwmon4/temp1_input"), "50000").unwrap();
      assert_unavailable_without_input_reads(&fixture);
    }
    let fixture = Fixture::new();
    fs::remove_dir_all(fixture.device.join("hwmon")).unwrap();
    assert_unavailable(&fixture.sample());
  }

  // Fixture acceptance row 5: bound `it87.2624`.
  #[test]
  fn a_bound_overlapping_it87_device_blocks_sampling() {
    let fixture = Fixture::new();
    fixture.bind_platform_device(CONFLICTING_DEVICE, Some(CONFLICTING_DRIVER));
    assert_unavailable_without_input_reads(&fixture);

    // Either side of the binding metadata establishes the conflict.
    fs::remove_file(
      fixture
        .sysfs
        .join("devices/platform")
        .join(CONFLICTING_DEVICE)
        .join("driver"),
    )
    .unwrap();
    assert_unavailable_without_input_reads(&fixture);
    let driver_side = fixture
      .sysfs
      .join("bus/platform/drivers/it87")
      .join(CONFLICTING_DEVICE);
    fs::remove_file(&driver_side).unwrap();
    // An unbound device with that name is not a conflict.
    assert_s3_sample(&fixture.sample());
    let device = fixture
      .sysfs
      .join("devices/platform")
      .join(CONFLICTING_DEVICE);
    symlink(
      fixture.sysfs.join("bus/platform/drivers/it87"),
      device.join("driver"),
    )
    .unwrap();
    assert_unavailable_without_input_reads(&fixture);
  }

  // Fixture acceptance row 5: a loaded module without the overlapping
  // binding, or the device bound to another driver, is not the conflict.
  #[test]
  fn only_the_documented_binding_is_a_conflict() {
    let fixture = Fixture::new();
    fixture.bind_platform_device("it87.2560", Some(CONFLICTING_DRIVER));
    assert_s3_sample(&fixture.sample());

    let fixture = Fixture::new();
    fixture.bind_platform_device(CONFLICTING_DEVICE, Some("other-driver"));
    assert_s3_sample(&fixture.sample());

    let fixture = Fixture::new();
    fs::remove_dir_all(fixture.sysfs.join("bus/platform/drivers/it87")).unwrap();
    assert_s3_sample(&fixture.sample());
  }

  // Fixture acceptance row 5: unreadable conflict metadata.
  #[test]
  fn incomplete_conflict_metadata_blocks_sampling() {
    let fixture = Fixture::new();
    fs::remove_dir_all(fixture.sysfs.join("bus/platform/devices")).unwrap();
    assert_unavailable_without_input_reads(&fixture);

    let fixture = Fixture::new();
    symlink(
      fixture.sysfs.join("devices/platform/missing"),
      fixture
        .sysfs
        .join("bus/platform/devices")
        .join(CONFLICTING_DEVICE),
    )
    .unwrap();
    assert_unavailable_without_input_reads(&fixture);

    let fixture = Fixture::new();
    fixture.bind_platform_device(CONFLICTING_DEVICE, None);
    let device = fixture
      .sysfs
      .join("devices/platform")
      .join(CONFLICTING_DEVICE);
    symlink(
      fixture.sysfs.join("bus/platform/drivers/gone"),
      device.join("driver"),
    )
    .unwrap();
    assert_unavailable_without_input_reads(&fixture);
  }

  // Fixture acceptance row 5: ownership lost or changed during a sample.
  #[test]
  fn discards_samples_whose_binding_changed() {
    let fixture = Fixture::new();
    let endpoint = fixture.endpoint();
    let driver = fixture.device.join("driver");
    let target = fs::read_link(&driver).unwrap();
    fs::rename(&driver, fixture.device.join("old-driver")).unwrap();
    symlink(&target, &driver).unwrap();
    assert_unavailable(&finish_sample(&fixture.sysfs, &endpoint, s3_readings()));

    let endpoint = fixture.endpoint();
    fixture.bind_platform_device(CONFLICTING_DEVICE, Some(CONFLICTING_DRIVER));
    assert_unavailable(&finish_sample(&fixture.sysfs, &endpoint, s3_readings()));

    let fixture = Fixture::new();
    let endpoint = fixture.endpoint();
    fs::rename(&fixture.node, fixture.sysfs.join("replaced-hwmon")).unwrap();
    fs::create_dir(&fixture.node).unwrap();
    symlink(&fixture.device, fixture.node.join("device")).unwrap();
    fixture.write("name", "gigabyte_wmi\n");
    let replacement = fixture.endpoint();
    assert_eq!(replacement.hwmon, endpoint.hwmon);
    assert_ne!(replacement, endpoint);
    assert_unavailable(&finish_sample(&fixture.sysfs, &endpoint, s3_readings()));

    let fixture = Fixture::new();
    let endpoint = fixture.endpoint();
    fs::remove_file(fixture.device.join("driver")).unwrap();
    assert_unavailable(&finish_sample(&fixture.sysfs, &endpoint, s3_readings()));
  }

  // Fixture acceptance row 6.
  #[test]
  fn skips_unusable_channels_and_keeps_siblings_with_their_numbers() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.node.join("temp1_input")).unwrap();
    fs::create_dir(fixture.node.join("temp1_input")).unwrap();
    fixture.write("temp2_input", "0\n");
    fixture.write("temp3_input", "34 C\n");
    fixture.write("temp4_input", "24500\n");
    fs::remove_file(fixture.node.join("temp5_input")).unwrap();
    assert_unavailable(&fixture.sample());

    fixture.write("temp3_input", "34000\n");
    fixture.write("temp5_input", "130000\n");
    let collection = fixture.sample();
    assert_eq!(collection.availability, SensorAvailability::Available);
    assert_eq!(collection.sample.temperatures, [reading(3, 34.0)]);

    fixture.write("temp5_input", "27000\n");
    let collection = fixture.sample();
    assert_eq!(
      collection.sample.temperatures,
      [reading(3, 34.0), reading(5, 27.0)]
    );
  }

  // Fixture acceptance row 7.
  #[test]
  fn admits_the_range_boundaries_and_reads_no_sixth_channel() {
    let fixture = Fixture::new();
    for (channel, value) in [(1, "-55000"), (2, "125000"), (3, "-56000"), (4, "126000")] {
      fixture.write(&format!("temp{channel}_input"), value);
    }
    fixture.write("temp7_input", "40000");
    fixture.node_reads();
    let collection = fixture.sample();
    assert_eq!(
      collection.sample.temperatures,
      [reading(1, -55.0), reading(2, 125.0), reading(5, 27.0)]
    );
    assert_reads_within(&fixture.node_reads(), &READ_SET);
  }

  // Fixture acceptance row 8.
  #[test]
  fn no_usable_input_leaves_the_temperatures_unavailable() {
    let fixture = Fixture::new();
    for channel in TEMPERATURE_CHANNELS {
      fixture.write(&format!("temp{channel}_input"), "0\n");
    }
    let collection = fixture.sample();
    assert_unavailable(&collection);
    assert_eq!(collection.fan_support, SensorSupport::Unsupported);
  }

  #[test]
  fn decodes_signed_whole_degree_millidegrees() {
    for (raw, expected) in [
      ("21000", Some(21.0)),
      ("-55000", Some(-55.0)),
      ("125000", Some(125.0)),
      ("-1000", Some(-1.0)),
      ("0", None),
      ("-0", None),
      ("-56000", None),
      ("126000", None),
      ("21500", None),
      ("21001", None),
      ("-55999", None),
      ("", None),
      (" 21000", None),
      ("21000 ", None),
      ("21.0", None),
      ("0x5208", None),
      ("2147483648", None),
    ] {
      assert_eq!(decode_temperature(raw), expected, "{raw:?}");
    }
  }
}
