//! Experimental temperatures through an already-bound Linux gigabyte-wmi
//! hwmon endpoint. See `superio-ite-b850m-wifi6e-ice-linux-wmi.md` revision 2.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Read};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use crate::models::{
  MotherboardSensorCollection, MotherboardSensorSample, MotherboardTemperature,
  SensorAvailability, SensorSupport,
};

const VENDOR: &str = "Gigabyte Technology Co., Ltd.";
const BOARD: &str = "B850M AORUS ELITE WIFI6E ICE";
const GUID: &str = "DEADBEEF-2001-0000-00A0-C90629100000";
const CONFLICT: &str = "it87.2624";
const SOURCE: &str = "IT8696E (gigabyte-wmi)";
const MAX_ATTRIBUTE_BYTES: u64 = 128;

pub(crate) fn sample() -> MotherboardSensorCollection {
  sample_at(Path::new("/sys"), read_attribute)
}

fn sample_at(
  sysfs: &Path,
  mut read_input: impl FnMut(&Path) -> Option<String>,
) -> MotherboardSensorCollection {
  let dmi = sysfs.join("class/dmi/id");
  let board = read_attribute(&dmi.join("board_name"));
  let vendor = read_attribute(&dmi.join("sys_vendor"));
  match (board.as_deref(), vendor.as_deref()) {
    (Some(BOARD), Some(VENDOR)) => {}
    (Some(BOARD), None) => {
      return MotherboardSensorCollection::unavailable(
        "Could not identify the Gigabyte motherboard vendor",
      );
    }
    _ => {
      return MotherboardSensorCollection::unsupported(
        "No supported gigabyte-wmi motherboard sensor profile for this board",
      );
    }
  }

  let Some(endpoint) = safe_endpoint(sysfs) else {
    return MotherboardSensorCollection::unavailable(
      "Experimental gigabyte-wmi endpoint ownership or conflict check failed",
    );
  };

  // The firmware selector supplies the channel identity. The hwmon and
  // mapper instance numbers are transient and never enter the reading name.
  let temperatures: Vec<_> = (1..=5)
    .filter_map(|channel| {
      let input = endpoint.hwmon.join(format!("temp{channel}_input"));
      let temperature = decode_temperature(&read_input(&input)?)?;
      Some(MotherboardTemperature {
        name: format!("Temperature {channel}"),
        temperature,
        source: SOURCE.to_owned(),
      })
    })
    .collect();

  if safe_endpoint(sysfs).as_ref() != Some(&endpoint) {
    return MotherboardSensorCollection::unavailable(
      "Experimental gigabyte-wmi ownership or conflict state changed during collection",
    );
  }

  let availability = if temperatures.is_empty() {
    SensorAvailability::unavailable(
      "Experimental gigabyte-wmi endpoint returned no usable temperatures",
    )
  } else {
    SensorAvailability::Available
  };
  MotherboardSensorCollection {
    sample: MotherboardSensorSample {
      temperatures,
      ..MotherboardSensorSample::default()
    },
    availability,
    fan_support: SensorSupport::Unsupported,
    ..MotherboardSensorCollection::default()
  }
}

#[derive(Debug, PartialEq, Eq)]
struct Endpoint {
  hwmon: PathBuf,
  identities: [(u64, u64); 5],
}

fn safe_endpoint(sysfs: &Path) -> Option<Endpoint> {
  if has_raw_conflict(sysfs)? {
    return None;
  }
  let wmi = sysfs.join("bus/wmi");
  // Include all kernel suffix variants in the cardinality check, even if a
  // duplicate is unbound or its symlink can no longer be resolved.
  let bus_link = only_entry(&wmi.join("devices"), |name| name.starts_with(GUID))?;
  let device = symlink_target(&bus_link)?;
  // The bound WMI bus device is a child of the mapper. Confirm its ACPI
  // ancestry without depending on a particular mapper or ACPI instance number.
  let mapper = device.parent()?;
  let devices = sysfs.join("devices");
  let driver = device.join("driver");
  let native_driver = wmi.join("drivers/gigabyte-wmi");
  if device.file_name() != bus_link.file_name()
    || !mapper.starts_with(&devices)
    || !mapper
      .ancestors()
      .skip(1)
      .take_while(|path| path.starts_with(&devices))
      .any(|path| {
        path
          .file_name()
          .and_then(|name| name.to_str())
          .is_some_and(|name| name.split_once(':').is_some_and(|(id, _)| id == "PNP0C14"))
      })
    || !links_to(&device.join("subsystem"), &wmi)
    || !links_to(&driver, &native_driver)
    || !fs::symlink_metadata(&native_driver).ok()?.is_dir()
    || read_attribute(&device.join("guid")).as_deref() != Some(GUID)
    || read_attribute(&device.join("instance_count")).as_deref() != Some("1")
  {
    return None;
  }

  let hwmon = only_entry(&device.join("hwmon"), |name| {
    name.strip_prefix("hwmon").is_some_and(|index| {
      !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
    })
  })?;
  let class_link = sysfs.join("class/hwmon").join(hwmon.file_name()?);
  if !fs::symlink_metadata(&hwmon).ok()?.is_dir()
    || !links_to(&class_link, &hwmon)
    || !links_to(&hwmon.join("device"), &device)
    || read_attribute(&hwmon.join("name")).as_deref() != Some("gigabyte_wmi")
  {
    return None;
  }

  let identities = [
    file_identity(&hwmon)?,
    file_identity(&device)?,
    file_identity(mapper)?,
    file_identity(&driver)?,
    file_identity(&class_link)?,
  ];
  Some(Endpoint { hwmon, identities })
}

fn file_identity(path: &Path) -> Option<(u64, u64)> {
  let metadata = fs::symlink_metadata(path).ok()?;
  Some((metadata.dev(), metadata.ino()))
}

/// Prove the platform bus can be inspected before treating absent links as
/// evidence that the competing raw driver is unbound.
fn has_raw_conflict(sysfs: &Path) -> Option<bool> {
  let platform = sysfs.join("bus/platform");
  let devices = platform.join("devices");
  let drivers = platform.join("drivers");
  fs::read_dir(&devices).ok()?;
  fs::read_dir(&drivers).ok()?;
  let it87 = drivers.join("it87");
  let reverse = match fs::symlink_metadata(&it87) {
    Ok(meta) if meta.is_dir() => {
      fs::read_dir(&it87).ok()?;
      optional_link(&it87.join(CONFLICT))?
    }
    Err(error) if error.kind() == ErrorKind::NotFound => None,
    _ => return None,
  };
  let Some(device) = optional_link(&devices.join(CONFLICT))? else {
    return reverse.is_none().then_some(false);
  };
  if device != sysfs.join("devices/platform").join(CONFLICT)
    || !links_to(&device.join("subsystem"), &platform)
  {
    return None;
  }
  let binding = optional_link(&device.join("driver"))?;
  if binding.as_ref() == Some(&it87) {
    return Some(true);
  }
  if reverse.is_some()
    || binding.is_some_and(|owner| owner.parent() != Some(&drivers) || !owner.is_dir())
  {
    return None;
  }
  Some(false)
}

fn only_entry(directory: &Path, matches: impl Fn(&str) -> bool) -> Option<PathBuf> {
  let mut found = None;
  for entry in fs::read_dir(directory).ok()? {
    let entry = entry.ok()?;
    if entry.file_name().to_str().is_some_and(&matches)
      && found.replace(entry.path()).is_some()
    {
      return None;
    }
  }
  found
}

fn optional_link(path: &Path) -> Option<Option<PathBuf>> {
  match fs::symlink_metadata(path) {
    Ok(meta) if meta.is_symlink() => Some(Some(fs::canonicalize(path).ok()?)),
    Err(error) if error.kind() == ErrorKind::NotFound => Some(None),
    _ => None,
  }
}

fn symlink_target(path: &Path) -> Option<PathBuf> {
  optional_link(path)?
}

fn links_to(link: &Path, target: &Path) -> bool {
  symlink_target(link).as_deref() == Some(target)
}

fn read_attribute(path: &Path) -> Option<String> {
  if !fs::symlink_metadata(path).ok()?.is_file() {
    return None;
  }
  let file = OpenOptions::new()
    .read(true)
    .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
    .open(path)
    .ok()?;
  let mut text = String::new();
  file
    .take(MAX_ATTRIBUTE_BYTES + 1)
    .read_to_string(&mut text)
    .ok()?;
  if text.len() as u64 > MAX_ATTRIBUTE_BYTES {
    return None;
  }
  text.truncate(text.trim_end_matches(['\r', '\n']).len());
  Some(text)
}

fn decode_temperature(value: &str) -> Option<f32> {
  let value: i32 = value.parse().ok()?;
  (value != 0 && value % 1_000 == 0 && (-55_000..=125_000).contains(&value))
    .then_some(value as f32 / 1_000.0)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::os::unix::fs::symlink;
  use tempfile::TempDir;

  struct Fixture {
    sysfs: TempDir,
    device: PathBuf,
    hwmon: PathBuf,
  }

  impl Fixture {
    fn new() -> Self {
      Self::under_acpi_device("PNP0C14:09")
    }

    fn under_acpi_device(acpi: &str) -> Self {
      let sysfs = TempDir::new().unwrap();
      let root = sysfs.path();
      let device = root
        .join("devices/platform")
        .join(acpi)
        .join("wmi_bus/mapper")
        .join(GUID);
      let hwmon = device.join("hwmon/hwmon47");
      for directory in [
        root.join("class/dmi/id"),
        root.join("class/hwmon"),
        root.join("bus/wmi/devices"),
        root.join("bus/wmi/drivers/gigabyte-wmi"),
        root.join("bus/platform/devices"),
        root.join("bus/platform/drivers"),
        hwmon.clone(),
      ] {
        fs::create_dir_all(directory).unwrap();
      }
      let fixture = Self {
        sysfs,
        device,
        hwmon,
      };
      fixture.write("class/dmi/id/sys_vendor", VENDOR);
      fixture.write("class/dmi/id/board_name", BOARD);
      symlink(
        &fixture.device,
        fixture.root().join("bus/wmi/devices").join(GUID),
      )
      .unwrap();
      symlink(
        fixture.root().join("bus/wmi"),
        fixture.device.join("subsystem"),
      )
      .unwrap();
      symlink(
        fixture.root().join("bus/wmi/drivers/gigabyte-wmi"),
        fixture.device.join("driver"),
      )
      .unwrap();
      write_value(&fixture.device.join("guid"), GUID);
      write_value(&fixture.device.join("instance_count"), "1");
      symlink(&fixture.hwmon, fixture.class_link()).unwrap();
      symlink(&fixture.device, fixture.hwmon.join("device")).unwrap();
      write_value(&fixture.hwmon.join("name"), "gigabyte_wmi");
      for (channel, value) in ["21000", "37000", "34000", "24000", "27000"]
        .into_iter()
        .enumerate()
      {
        fixture.input(channel + 1, value);
      }
      fixture
    }

    fn root(&self) -> &Path {
      self.sysfs.path()
    }

    fn class_link(&self) -> PathBuf {
      self.root().join("class/hwmon/hwmon47")
    }

    fn write(&self, relative: &str, value: &str) {
      write_value(&self.root().join(relative), value);
    }

    fn input(&self, channel: usize, value: &str) {
      write_value(&self.hwmon.join(format!("temp{channel}_input")), value);
    }

    fn sample(&self) -> MotherboardSensorCollection {
      sample_at(self.root(), read_attribute)
    }

    fn without_inputs(&self) -> MotherboardSensorCollection {
      sample_at(self.root(), |input| {
        panic!("unexpected read of {}", input.display())
      })
    }

    fn it87_device(&self, name: &str, bound: bool) {
      let device = self.root().join("devices/platform").join(name);
      let platform = self.root().join("bus/platform");
      let driver = platform.join("drivers/it87");
      fs::create_dir_all(&device).unwrap();
      fs::create_dir_all(&driver).unwrap();
      symlink(&device, platform.join("devices").join(name)).unwrap();
      symlink(&platform, device.join("subsystem")).unwrap();
      if bound {
        symlink(&driver, device.join("driver")).unwrap();
        symlink(&device, driver.join(name)).unwrap();
      }
    }
  }

  fn write_value(path: &Path, value: &str) {
    fs::write(path, format!("{value}\n")).unwrap();
  }

  fn temperatures(collection: &MotherboardSensorCollection) -> Vec<(&str, f32)> {
    collection
      .sample
      .temperatures
      .iter()
      .map(|reading| (reading.name.as_str(), reading.temperature))
      .collect()
  }

  fn assert_unavailable(collection: MotherboardSensorCollection) {
    assert!(matches!(
      collection.availability,
      SensorAvailability::Unavailable { .. }
    ));
    assert!(collection.sample.temperatures.is_empty());
    assert!(collection.sample.fan_speeds.is_empty());
    assert!(collection.guidance_candidates.is_empty());
  }

  #[test]
  fn collects_only_five_numbered_temperature_inputs_with_qualified_ownership() {
    let fixture = Fixture::new();
    for attr in ["temp6_input", "temp1_alarm", "fan1_input"] {
      write_value(&fixture.hwmon.join(attr), "28000");
    }
    for bios in [None, Some("F4"), Some("F11"), Some("F12e"), Some("unknown")] {
      if let Some(bios) = bios {
        fixture.write("class/dmi/id/bios_version", bios);
      }
      let mut reads = Vec::new();
      let sample = sample_at(fixture.root(), |input| {
        reads.push(input.file_name().unwrap().to_string_lossy().into_owned());
        read_attribute(input)
      });
      assert_eq!(sample.availability, SensorAvailability::Available);
      assert_eq!(
        temperatures(&sample),
        [
          ("Temperature 1", 21.0),
          ("Temperature 2", 37.0),
          ("Temperature 3", 34.0),
          ("Temperature 4", 24.0),
          ("Temperature 5", 27.0),
        ]
      );
      assert!(
        sample
          .sample
          .temperatures
          .iter()
          .all(|reading| reading.source == SOURCE)
      );
      assert!(sample.sample.fan_speeds.is_empty());
      assert_eq!(sample.fan_support, SensorSupport::Unsupported);
      assert_eq!(
        reads,
        [
          "temp1_input",
          "temp2_input",
          "temp3_input",
          "temp4_input",
          "temp5_input"
        ]
      );
    }
  }

  #[test]
  fn ignores_identical_wmi_endpoint_on_another_board() {
    let fixture = Fixture::new();
    fixture.write("class/dmi/id/board_name", "B850 AORUS ELITE WIFI6E ICE");
    assert!(matches!(
      fixture.without_inputs().availability,
      SensorAvailability::Unsupported { .. }
    ));
    fixture.write("class/dmi/id/board_name", BOARD);
    fixture.write("class/dmi/id/sys_vendor", "Another vendor");
    assert!(matches!(
      fixture.without_inputs().availability,
      SensorAvailability::Unsupported { .. }
    ));
    fs::remove_file(fixture.root().join("class/dmi/id/sys_vendor")).unwrap();
    assert_unavailable(fixture.without_inputs());
  }

  #[test]
  fn refuses_missing_wrong_or_ambiguous_wmi_ownership_before_reading() {
    assert_unavailable(Fixture::under_acpi_device("PNP0C15:09").without_inputs());
    let cases: &[fn(&Fixture)] = &[
      |f| fs::remove_file(f.device.join("driver")).unwrap(),
      |f| {
        fs::remove_file(f.device.join("driver")).unwrap();
        symlink(f.root().join("bus/wmi"), f.device.join("driver")).unwrap();
      },
      |f| {
        fs::remove_file(f.class_link()).unwrap();
        symlink(&f.device, f.class_link()).unwrap();
      },
      |f| {
        fs::remove_file(f.hwmon.join("device")).unwrap();
        symlink(f.root().join("devices/platform"), f.hwmon.join("device")).unwrap();
      },
      |f| {
        symlink(
          f.root().join("devices/nonexistent"),
          f.root().join("bus/wmi/devices").join(format!("{GUID}-1")),
        )
        .unwrap();
      },
      |f| fs::create_dir(f.device.join("hwmon/hwmon48")).unwrap(),
      |f| write_value(&f.device.join("instance_count"), "2"),
      |f| {
        write_value(
          &f.device.join("guid"),
          "00000000-0000-0000-0000-000000000000",
        )
      },
    ];
    for invalidate in cases {
      let fixture = Fixture::new();
      invalidate(&fixture);
      assert_unavailable(fixture.without_inputs());
    }
  }

  #[test]
  fn excludes_a_bound_primary_raw_driver_and_incomplete_conflict_metadata() {
    let unbound = Fixture::new();
    unbound.it87_device(CONFLICT, false);
    assert_eq!(unbound.sample().availability, SensorAvailability::Available);

    let unrelated = Fixture::new();
    unrelated.it87_device("it87.2632", true);
    assert_eq!(
      unrelated.sample().availability,
      SensorAvailability::Available
    );

    let conflicting = Fixture::new();
    conflicting.it87_device(CONFLICT, true);
    assert_unavailable(conflicting.without_inputs());

    let no_platform = Fixture::new();
    fs::remove_dir(no_platform.root().join("bus/platform/devices")).unwrap();
    assert_unavailable(no_platform.without_inputs());

    let dangling = Fixture::new();
    dangling.it87_device(CONFLICT, false);
    symlink(
      dangling.root().join("bus/platform/drivers/unresolved"),
      dangling
        .root()
        .join("devices/platform")
        .join(CONFLICT)
        .join("driver"),
    )
    .unwrap();
    assert_unavailable(dangling.without_inputs());
  }

  #[test]
  fn invalid_missing_or_redirected_input_preserves_other_channels() {
    let fixture = Fixture::new();
    fixture.input(1, "-55000");
    fixture.input(2, "bad");
    fixture.input(3, "0");
    fixture.input(4, "125000");
    fs::remove_file(fixture.hwmon.join("temp5_input")).unwrap();
    assert_eq!(
      temperatures(&fixture.sample()),
      [("Temperature 1", -55.0), ("Temperature 4", 125.0)]
    );

    fixture.input(1, "-56000");
    fixture.input(2, "22500");
    fixture.input(4, "126000");
    let alarm = fixture.hwmon.join("temp5_alarm");
    write_value(&alarm, "28000");
    symlink(alarm, fixture.hwmon.join("temp5_input")).unwrap();
    assert_unavailable(fixture.sample());
  }

  #[test]
  fn discards_collected_values_if_binding_changes_or_conflict_appears() {
    let fixture = Fixture::new();
    let sample = sample_at(fixture.root(), |path| {
      if path.ends_with("temp1_input") {
        fixture.it87_device(CONFLICT, true);
      }
      read_attribute(path)
    });
    assert_unavailable(sample);

    let fixture = Fixture::new();
    let sample = sample_at(fixture.root(), |path| {
      if path.ends_with("temp1_input") {
        fs::rename(
          fixture.device.join("driver"),
          fixture.device.join("old-driver"),
        )
        .unwrap();
        symlink(
          fixture.root().join("bus/wmi/drivers/gigabyte-wmi"),
          fixture.device.join("driver"),
        )
        .unwrap();
      }
      read_attribute(path)
    });
    assert_unavailable(sample);
  }
}
