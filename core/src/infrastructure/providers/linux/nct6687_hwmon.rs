use crate::models::{
  FanSpeedStatus, MotherboardFanSpeed, MotherboardSensorCollection,
  MotherboardSensorSample, MotherboardTemperature, SensorAvailability, SensorSupport,
};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const SOURCE: &str = "NCT6687D (hwmon)";
const PLATFORM_NODE: &str = "devices/platform/nct6683.2592";
const BOARD_IDENTITY: [(&str, &str); 3] = [
  ("board_vendor", "Micro-Star International Co., Ltd."),
  ("board_name", "PRO B850-P WIFI (MS-7E56)"),
  ("bios_version", "2.A93"),
];
const TEMPERATURE_CHANNELS: [(u8, &str, &str); 3] = [
  (2, "Diode 0 (curr)", "3"),
  (3, "Thermistor 15", "4"),
  (4, "Thermistor 1", "4"),
];

pub(crate) fn sample() -> MotherboardSensorCollection {
  sample_at(Path::new("/sys"))
}

fn sample_at(sysfs: &Path) -> MotherboardSensorCollection {
  let dmi = sysfs.join("class/dmi/id");
  for (attribute, expected) in BOARD_IDENTITY {
    match read_attribute(&dmi, attribute).as_deref() {
      Some(value) if value == expected => {}
      Some(_) => {
        return MotherboardSensorCollection::unsupported(
          "No supported Linux motherboard sensor profile for this board and BIOS",
        );
      }
      None => {
        return MotherboardSensorCollection::unavailable(
          "Could not identify the Linux motherboard and BIOS",
        );
      }
    }
  }

  let Some(endpoint) = find_endpoint(sysfs) else {
    return MotherboardSensorCollection::unavailable(
      "The bound NCT6687D hwmon endpoint is unavailable or ambiguous",
    );
  };

  let temperatures = TEMPERATURE_CHANNELS
    .into_iter()
    .filter_map(|(channel, label, kind)| {
      read_temperature(&endpoint.path, channel, label, kind)
    })
    .collect::<Vec<_>>();
  let fan_speeds = (1..=2)
    .filter_map(|channel| read_fan(&endpoint.path, channel))
    .collect::<Vec<_>>();

  // A hwmon index can be reused by a new device during collection.
  if find_endpoint(sysfs).as_ref() != Some(&endpoint) {
    return MotherboardSensorCollection::unavailable(
      "The NCT6687D hwmon binding changed during collection",
    );
  }

  let availability = if temperatures.is_empty() && fan_speeds.is_empty() {
    SensorAvailability::unavailable(
      "The NCT6687D hwmon endpoint returned no usable readings",
    )
  } else {
    SensorAvailability::Available
  };
  MotherboardSensorCollection {
    sample: MotherboardSensorSample {
      temperatures,
      fan_speeds,
    },
    availability,
    fan_support: SensorSupport::Supported,
    ..MotherboardSensorCollection::default()
  }
}

#[derive(Debug, PartialEq, Eq)]
struct Endpoint {
  path: PathBuf,
  device: u64,
  inode: u64,
}

fn find_endpoint(sysfs: &Path) -> Option<Endpoint> {
  let parent = sysfs.join(PLATFORM_NODE);
  if !resolves_to(&parent.join("subsystem"), &sysfs.join("bus/platform"))
    || !resolves_to(
      &parent.join("driver"),
      &sysfs.join("bus/platform/drivers/nct6683"),
    )
  {
    return None;
  }
  let expected_hwmon = parent.join("hwmon");
  let mut endpoint = None;
  for entry in fs::read_dir(sysfs.join("class/hwmon")).ok()? {
    let entry = entry.ok()?;
    if !entry.file_name().to_str().is_some_and(is_hwmon_name) {
      continue;
    }
    let Ok(path) = fs::canonicalize(entry.path()) else {
      continue;
    };
    if path.parent() != Some(expected_hwmon.as_path())
      || !path.file_name()?.to_str().is_some_and(is_hwmon_name)
      || !resolves_to(&path.join("device"), &parent)
      || read_attribute(&path, "name").as_deref() != Some("nct6687")
    {
      continue;
    }
    if endpoint
      .as_ref()
      .is_some_and(|candidate: &Endpoint| candidate.path != path)
    {
      return None;
    }
    let metadata = fs::metadata(&path).ok()?;
    endpoint = Some(Endpoint {
      path,
      device: metadata.dev(),
      inode: metadata.ino(),
    });
  }
  endpoint
}

fn is_hwmon_name(name: &str) -> bool {
  name.strip_prefix("hwmon").is_some_and(|index| {
    !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
  })
}

fn resolves_to(link: &Path, expected: &Path) -> bool {
  fs::canonicalize(link).is_ok_and(|target| target == expected)
}

fn read_attribute(directory: &Path, attribute: &str) -> Option<String> {
  let mut text = fs::read_to_string(directory.join(attribute)).ok()?;
  text.truncate(text.trim_end_matches(['\r', '\n']).len());
  Some(text)
}

fn read_temperature(
  path: &Path,
  channel: u8,
  label: &str,
  kind: &str,
) -> Option<MotherboardTemperature> {
  if read_attribute(path, &format!("temp{channel}_label")).as_deref() != Some(label)
    || read_attribute(path, &format!("temp{channel}_type")).as_deref() != Some(kind)
  {
    return None;
  }
  let milli_celsius = read_attribute(path, &format!("temp{channel}_input"))?
    .parse::<i32>()
    .ok()?;
  (-55_000..=125_000)
    .contains(&milli_celsius)
    .then(|| MotherboardTemperature {
      name: format!("Temperature {channel}"),
      temperature: milli_celsius as f32 / 1000.0,
      source: SOURCE.to_owned(),
    })
}

fn read_fan(path: &Path, channel: u8) -> Option<MotherboardFanSpeed> {
  let rpm = read_attribute(path, &format!("fan{channel}_input"))?
    .parse::<u32>()
    .ok()?;
  (1..=65_534).contains(&rpm).then(|| MotherboardFanSpeed {
    name: format!("Fan {channel}"),
    rpm: Some(rpm),
    status: FanSpeedStatus::Active,
    source: SOURCE.to_owned(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::os::unix::fs::symlink;
  use tempfile::TempDir;

  struct Fixture {
    _directory: TempDir,
    sysfs: PathBuf,
    node: PathBuf,
  }

  impl Fixture {
    fn new() -> Self {
      let directory = tempfile::tempdir().unwrap();
      let sysfs = directory.path().canonicalize().unwrap();
      let dmi = sysfs.join("class/dmi/id");
      let parent = sysfs.join(PLATFORM_NODE);
      let driver = sysfs.join("bus/platform/drivers/nct6683");
      let node = parent.join("hwmon/hwmon5");
      for path in [&dmi, &driver, &node, &sysfs.join("class/hwmon")] {
        fs::create_dir_all(path).unwrap();
      }
      for (attribute, value) in BOARD_IDENTITY {
        fs::write(dmi.join(attribute), format!("{value}\n")).unwrap();
      }
      symlink(sysfs.join("bus/platform"), parent.join("subsystem")).unwrap();
      symlink(driver, parent.join("driver")).unwrap();
      symlink(parent, node.join("device")).unwrap();
      symlink(&node, sysfs.join("class/hwmon/hwmon5")).unwrap();

      let fixture = Self {
        _directory: directory,
        sysfs,
        node,
      };
      fixture.write("name", "nct6687\n");
      for ((channel, label, kind), reading) in TEMPERATURE_CHANNELS
        .into_iter()
        .zip([26_000, 22_000, 26_000])
      {
        fixture.write(&format!("temp{channel}_label"), label);
        fixture.write(&format!("temp{channel}_type"), kind);
        fixture.write(&format!("temp{channel}_input"), &format!("{reading}\n"));
      }
      fixture.write("fan1_input", "518\n");
      fixture.write("fan2_input", "0\n");
      fixture
    }

    fn write(&self, attribute: &str, value: &str) {
      fs::write(self.node.join(attribute), value).unwrap();
    }

    fn sample(&self) -> MotherboardSensorCollection {
      sample_at(&self.sysfs)
    }
  }

  #[test]
  fn samples_only_the_qualified_channels() {
    let fixture = Fixture::new();
    fixture.write("temp1_input", "45000");
    fixture.write("fan3_input", "1200");
    let collection = fixture.sample();
    assert_eq!(collection.availability, SensorAvailability::Available);
    assert_eq!(collection.fan_support, SensorSupport::Supported);
    assert_eq!(
      collection
        .sample
        .temperatures
        .iter()
        .map(|reading| {
          (
            reading.name.as_str(),
            reading.temperature,
            reading.source.as_str(),
          )
        })
        .collect::<Vec<_>>(),
      [
        ("Temperature 2", 26.0, SOURCE),
        ("Temperature 3", 22.0, SOURCE),
        ("Temperature 4", 26.0, SOURCE)
      ],
    );
    assert_eq!(
      collection.sample.fan_speeds,
      [MotherboardFanSpeed {
        name: "Fan 1".to_owned(),
        rpm: Some(518),
        status: FanSpeedStatus::Active,
        source: SOURCE.to_owned(),
      }],
    );
  }

  #[test]
  fn distinguishes_wrong_identity_from_missing_metadata() {
    let fixture = Fixture::new();
    let dmi = fixture.sysfs.join("class/dmi/id");
    for (attribute, bad_value) in [
      ("board_vendor", "Micro-Star International Co., Ltd. "),
      ("board_name", "B850 GAMING PLUS WIFI (MS-7E56)"),
      ("bios_version", "2.A94"),
    ] {
      let path = dmi.join(attribute);
      let original = fs::read(&path).unwrap();
      fs::write(&path, bad_value).unwrap();
      let result = fixture.sample();
      assert!(matches!(
        result.availability,
        SensorAvailability::Unsupported { .. }
      ));
      assert_eq!(result.sample, MotherboardSensorSample::default());
      fs::write(path, original).unwrap();
    }
    fs::remove_file(dmi.join("bios_version")).unwrap();
    assert!(matches!(
      fixture.sample().availability,
      SensorAvailability::Unavailable { .. }
    ));
  }

  #[test]
  fn requires_platform_binding_and_a_unique_canonical_child() {
    let fixture = Fixture::new();
    let parent = fixture.sysfs.join(PLATFORM_NODE);
    let class_node = fixture.sysfs.join("class/hwmon/hwmon5");
    for (link, wrong_target) in [
      (class_node, parent.clone()),
      (fixture.node.join("device"), fixture.sysfs.clone()),
      (parent.join("subsystem"), fixture.sysfs.join("class")),
      (parent.join("driver"), fixture.sysfs.join("bus/platform")),
    ] {
      let original = fs::read_link(&link).unwrap();
      fs::remove_file(&link).unwrap();
      symlink(wrong_target, &link).unwrap();
      let result = fixture.sample();
      assert!(matches!(
        result.availability,
        SensorAvailability::Unavailable { .. }
      ));
      assert_eq!(result.sample, MotherboardSensorSample::default());
      fs::remove_file(&link).unwrap();
      symlink(original, link).unwrap();
    }
    let duplicate = parent.join("hwmon/hwmon6");
    fs::create_dir(&duplicate).unwrap();
    symlink(parent, duplicate.join("device")).unwrap();
    fs::write(duplicate.join("name"), "nct6687\n").unwrap();
    symlink(duplicate, fixture.sysfs.join("class/hwmon/hwmon6")).unwrap();
    assert!(find_endpoint(&fixture.sysfs).is_none());
  }

  #[test]
  fn rejects_wrong_kind_and_missing_binding() {
    let fixture = Fixture::new();
    fixture.write("name", "nct6686\n");
    assert!(find_endpoint(&fixture.sysfs).is_none());
    fixture.write("name", "nct6687\n");
    fs::remove_file(fixture.sysfs.join(PLATFORM_NODE).join("driver")).unwrap();
    assert!(find_endpoint(&fixture.sysfs).is_none());
  }

  #[test]
  fn preserves_sibling_values_on_partial_failure() {
    let fixture = Fixture::new();
    fixture.write("temp2_label", "Wrong source\n");
    fs::remove_file(fixture.node.join("temp3_type")).unwrap();
    fixture.write("fan1_input", "garbage");
    fixture.write("fan2_input", "612\n");
    let result = fixture.sample();
    assert_eq!(result.availability, SensorAvailability::Available);
    assert_eq!(result.sample.temperatures.len(), 1);
    assert_eq!(result.sample.temperatures[0].name, "Temperature 4");
    assert_eq!(result.sample.fan_speeds.len(), 1);
    assert_eq!(result.sample.fan_speeds[0].name, "Fan 2");

    fs::remove_file(fixture.node.join("temp4_input")).unwrap();
    fixture.write("fan2_input", "65535");
    let result = fixture.sample();
    assert!(matches!(
      result.availability,
      SensorAvailability::Unavailable { .. }
    ));
    assert_eq!(result.sample, MotherboardSensorSample::default());
  }

  #[test]
  fn checks_temperature_and_rpm_bounds() {
    let fixture = Fixture::new();
    for (raw, expected) in [
      ("-55000", Some(-55.0)),
      ("125000", Some(125.0)),
      ("0", Some(0.0)),
      ("-55001", None),
      ("125001", None),
      ("1\n2", None),
      ("2147483648", None),
    ] {
      fixture.write("temp2_input", raw);
      assert_eq!(
        read_temperature(&fixture.node, 2, "Diode 0 (curr)", "3")
          .map(|reading| reading.temperature),
        expected,
      );
    }
    for (raw, expected) in [
      ("-1", None),
      ("0", None),
      ("1", Some(1)),
      ("65534", Some(65_534)),
      ("65535", None),
      ("4294967296", None),
    ] {
      fixture.write("fan1_input", raw);
      assert_eq!(
        read_fan(&fixture.node, 1).and_then(|reading| reading.rpm),
        expected
      );
    }
  }

  #[test]
  fn detects_replaced_endpoint_even_when_the_class_index_is_reused() {
    let fixture = Fixture::new();
    let original = find_endpoint(&fixture.sysfs).unwrap();
    fs::rename(&fixture.node, fixture.node.with_file_name("old")).unwrap();
    fs::create_dir(&fixture.node).unwrap();
    symlink(
      fixture.sysfs.join(PLATFORM_NODE),
      fixture.node.join("device"),
    )
    .unwrap();
    fixture.write("name", "nct6687\n");
    let replacement = find_endpoint(&fixture.sysfs).unwrap();
    assert_eq!(replacement.path, original.path);
    assert_ne!(replacement, original);
  }
}
