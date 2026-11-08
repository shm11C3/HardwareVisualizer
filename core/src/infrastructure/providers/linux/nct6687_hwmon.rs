use crate::models::{
  FanSpeedStatus, MotherboardFanSpeed, MotherboardSensorCollection,
  MotherboardSensorSample, MotherboardTemperature, SensorAvailability, SensorSupport,
};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const SOURCE: &str = "NCT6687D (hwmon)";
const BOARD_VENDOR: &str = "Micro-Star International Co., Ltd.";
const PRO_BOARD: &str = "PRO B850-P WIFI (MS-7E56)";
const MORTAR_BOARD: &str = "MAG B850M MORTAR WIFI (MS-7E61)";

/// The board whose OEM EC firmware defines the scoped hwmon channel
/// assignment. The BIOS version and the EC base address are evidence
/// identities recorded in the specification; they do not gate a read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BoardProfile {
  Pro,
  Mortar,
}

const TEMPERATURE_CHANNELS: [(u8, &str, &str); 3] = [
  (2, "Diode 0 (curr)", "3"),
  (3, "Thermistor 15", "4"),
  (4, "Thermistor 1", "4"),
];

pub(crate) fn sample() -> MotherboardSensorCollection {
  sample_at(Path::new("/sys"))
}

/// Admit a board from the DMI baseboard vendor and product only.
fn identify_profile(dmi: &Path) -> Result<BoardProfile, MotherboardSensorCollection> {
  let missing = || {
    MotherboardSensorCollection::unavailable("Could not identify the Linux motherboard")
  };
  let unsupported = || {
    MotherboardSensorCollection::unsupported(
      "No supported Linux motherboard sensor profile for this board",
    )
  };
  if read_attribute(dmi, "board_vendor").ok_or_else(missing)? != BOARD_VENDOR {
    return Err(unsupported());
  }
  match read_attribute(dmi, "board_name")
    .ok_or_else(missing)?
    .as_str()
  {
    PRO_BOARD => Ok(BoardProfile::Pro),
    MORTAR_BOARD => Ok(BoardProfile::Mortar),
    _ => Err(unsupported()),
  }
}

fn sample_at(sysfs: &Path) -> MotherboardSensorCollection {
  let profile = match identify_profile(&sysfs.join("class/dmi/id")) {
    Ok(profile) => profile,
    Err(collection) => return collection,
  };
  let Some(endpoint) = find_endpoint(sysfs) else {
    return MotherboardSensorCollection::unavailable(
      "The bound NCT6687D hwmon endpoint is unavailable or ambiguous",
    );
  };

  let (temperatures, fan_speeds) = match profile {
    BoardProfile::Pro => (
      TEMPERATURE_CHANNELS
        .into_iter()
        .filter_map(|(channel, label, kind)| {
          read_temperature(&endpoint.path, channel, label, kind)
        })
        .collect(),
      [(1, "Fan 1"), (2, "Fan 2")]
        .into_iter()
        .filter_map(|(channel, name)| read_fan(&endpoint.path, channel, name))
        .collect(),
    ),
    BoardProfile::Mortar => (
      Vec::new(),
      read_fan(&endpoint.path, 16, "SYS_FAN1")
        .into_iter()
        .collect(),
    ),
  };
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
      "The NCT6687D hwmon binding changed during collection",
    );
  }

  let availability = if sample.temperatures.is_empty() && sample.fan_speeds.is_empty() {
    SensorAvailability::unavailable(
      "The NCT6687D hwmon endpoint returned no usable readings",
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
  identities: [(u64, u64); 3],
}

/// Find the one distinct canonical `nct6687` hwmon child of a bound
/// `nct6683.<id>` platform device. The same endpoint checks apply to every
/// board profile.
fn find_endpoint(sysfs: &Path) -> Option<Endpoint> {
  let mut endpoint = None;
  for entry in fs::read_dir(sysfs.join("class/hwmon")).ok()? {
    let entry = entry.ok()?;
    let class_name = entry.file_name();
    if !class_name.to_str().is_some_and(is_hwmon_name) {
      continue;
    }
    let Ok(path) = fs::canonicalize(entry.path()) else {
      continue;
    };
    if path.file_name() != Some(class_name.as_os_str()) {
      continue;
    }
    let Some(candidate) = qualify_endpoint(sysfs, path) else {
      continue;
    };
    if endpoint
      .as_ref()
      .is_some_and(|previous| previous != &candidate)
    {
      return None;
    }
    endpoint = Some(candidate);
  }
  endpoint
}

fn qualify_endpoint(sysfs: &Path, path: PathBuf) -> Option<Endpoint> {
  let hwmon = path.parent()?;
  let parent = hwmon.parent()?;
  let instance = parent.file_name()?.to_str()?;
  let driver = parent.join("driver");
  if hwmon.file_name()?.to_str()? != "hwmon"
    || parent.parent() != Some(sysfs.join("devices/platform").as_path())
    || !is_platform_instance(instance)
    || !path.file_name()?.to_str().is_some_and(is_hwmon_name)
    || !resolves_to(&path.join("device"), parent)
    || !resolves_to(&parent.join("subsystem"), &sysfs.join("bus/platform"))
    || !resolves_to(&driver, &sysfs.join("bus/platform/drivers/nct6683"))
    || !resolves_to(&sysfs.join("bus/platform/devices").join(instance), parent)
    || read_attribute(&path, "name").as_deref() != Some("nct6687")
  {
    return None;
  }

  let identities = [
    file_identity(&fs::metadata(&path).ok()?),
    file_identity(&fs::metadata(parent).ok()?),
    file_identity(&fs::symlink_metadata(driver).ok()?),
  ];
  Some(Endpoint { path, identities })
}

/// The driver names the platform device after the firmware-discovered EC
/// base, aligned to eight bytes, as a canonical unsigned decimal. Accept
/// `nct6683.<id>` with `id` from 8 through 65528 divisible by 8. The suffix
/// validates and identifies the sysfs instance; it does not select a profile.
fn is_platform_instance(name: &str) -> bool {
  let Some(suffix) = name.strip_prefix("nct6683.") else {
    return false;
  };
  suffix.parse::<u16>().is_ok_and(|id| {
    (8..=65_528).contains(&id) && id.is_multiple_of(8) && suffix == id.to_string()
  })
}

fn file_identity(metadata: &fs::Metadata) -> (u64, u64) {
  (metadata.dev(), metadata.ino())
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

fn read_fan(path: &Path, channel: u8, name: &str) -> Option<MotherboardFanSpeed> {
  let rpm = read_attribute(path, &format!("fan{channel}_input"))?
    .parse::<u32>()
    .ok()?;
  (1..=65_534).contains(&rpm).then(|| MotherboardFanSpeed {
    name: name.to_owned(),
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

  /// Observed BIOS versions from the specification. They are evidence
  /// identities, so fixtures also run with other and missing versions.
  const PRO_OBSERVED_BIOS: &str = "2.A93";
  const MORTAR_OBSERVED_BIOS: &str = "1.A68";
  /// The PRO observation's EC base `0x0A20` as the decimal instance suffix.
  const PRO_OBSERVED_INSTANCE: &str = "nct6683.2592";

  const NONCANONICAL_INSTANCES: [&str; 9] = [
    "nct6683.0",
    "nct6683.7",
    "nct6683.9",
    "nct6683.0008",
    "nct6683.+8",
    "nct6683.-8",
    "nct6683.65535",
    "nct6683.65536",
    "nct6684.8",
  ];

  struct Fixture {
    _directory: TempDir,
    sysfs: PathBuf,
    node: PathBuf,
  }

  impl Fixture {
    fn new() -> Self {
      Self::pro(Some(PRO_OBSERVED_BIOS), PRO_OBSERVED_INSTANCE)
    }

    /// Synthetic PRO sysfs values; the specification's capture shows 26/22/26
    /// degrees Celsius and 518/0 RPM.
    fn pro(bios: Option<&str>, instance: &str) -> Self {
      let fixture = Self::with_binding(PRO_BOARD, bios, instance);
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

    fn mortar(instance: &str) -> Self {
      Self::mortar_with_bios(Some(MORTAR_OBSERVED_BIOS), instance)
    }

    fn mortar_with_bios(bios: Option<&str>, instance: &str) -> Self {
      let fixture = Self::with_binding(MORTAR_BOARD, bios, instance);
      fixture.write("fan16_input", "926\n");
      fixture
    }

    fn with_binding(board: &str, bios: Option<&str>, instance: &str) -> Self {
      let directory = tempfile::tempdir().unwrap();
      let sysfs = directory.path().canonicalize().unwrap();
      let dmi = sysfs.join("class/dmi/id");
      fs::create_dir_all(&dmi).unwrap();
      for (attribute, value) in [("board_vendor", BOARD_VENDOR), ("board_name", board)] {
        fs::write(dmi.join(attribute), format!("{value}\n")).unwrap();
      }
      if let Some(bios) = bios {
        fs::write(dmi.join("bios_version"), format!("{bios}\n")).unwrap();
      }
      let node = add_binding(&sysfs, instance, "hwmon5");
      Self {
        _directory: directory,
        sysfs,
        node,
      }
    }

    fn parent(&self) -> &Path {
      self.node.parent().unwrap().parent().unwrap()
    }

    fn write(&self, attribute: &str, value: &str) {
      fs::write(self.node.join(attribute), value).unwrap();
    }

    fn sample(&self) -> MotherboardSensorCollection {
      sample_at(&self.sysfs)
    }
  }

  fn add_binding(sysfs: &Path, instance: &str, hwmon: &str) -> PathBuf {
    let parent = sysfs.join("devices/platform").join(instance);
    let driver = sysfs.join("bus/platform/drivers/nct6683");
    let bus_devices = sysfs.join("bus/platform/devices");
    let class = sysfs.join("class/hwmon");
    let node = parent.join("hwmon").join(hwmon);
    for directory in [&driver, &bus_devices, &node, &class] {
      fs::create_dir_all(directory).unwrap();
    }
    symlink(sysfs.join("bus/platform"), parent.join("subsystem")).unwrap();
    symlink(driver, parent.join("driver")).unwrap();
    symlink(&parent, bus_devices.join(instance)).unwrap();
    symlink(parent, node.join("device")).unwrap();
    symlink(&node, class.join(hwmon)).unwrap();
    fs::write(node.join("name"), "nct6687\n").unwrap();
    node
  }

  #[test]
  fn samples_only_the_qualified_channels() {
    let fixture = Fixture::new();
    fixture.write("temp1_input", "45000");
    fixture.write("fan3_input", "1200");
    assert_pro_sample(&fixture.sample());
  }

  #[test]
  fn pro_admission_ignores_the_bios_version_and_the_instance_number() {
    for bios in [Some("2.A94"), None, Some("Dasharo (coreboot+UEFI) v0.9.2")] {
      assert_pro_sample(&Fixture::pro(bios, PRO_OBSERVED_INSTANCE).sample());
    }
    for instance in ["nct6683.2600", "nct6683.8", "nct6683.65528"] {
      assert_pro_sample(&Fixture::pro(Some(PRO_OBSERVED_BIOS), instance).sample());
    }
  }

  #[test]
  fn pro_rejects_noncanonical_instances() {
    for instance in NONCANONICAL_INSTANCES {
      assert_unavailable(&Fixture::pro(Some(PRO_OBSERVED_BIOS), instance).sample());
    }
  }

  #[test]
  fn distinguishes_wrong_identity_from_missing_metadata() {
    let fixture = Fixture::new();
    let dmi = fixture.sysfs.join("class/dmi/id");
    for (attribute, bad_value) in [
      ("board_vendor", "Micro-Star International Co., Ltd. "),
      ("board_name", "B850 GAMING PLUS WIFI (MS-7E56)"),
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
    for attribute in ["board_name", "board_vendor"] {
      let path = dmi.join(attribute);
      let original = fs::read(&path).unwrap();
      fs::remove_file(&path).unwrap();
      assert_unavailable(&fixture.sample());
      fs::write(path, original).unwrap();
    }
    assert_pro_sample(&fixture.sample());
  }

  #[test]
  fn requires_platform_binding_and_a_unique_canonical_child() {
    let fixture = Fixture::new();
    let parent = fixture.parent().to_path_buf();
    let class_node = fixture.sysfs.join("class/hwmon/hwmon5");
    for (link, wrong_target) in [
      (class_node.clone(), parent.clone()),
      (fixture.node.join("device"), fixture.sysfs.clone()),
      (parent.join("subsystem"), fixture.sysfs.join("class")),
      (parent.join("driver"), fixture.sysfs.join("bus/platform")),
      (
        fixture
          .sysfs
          .join("bus/platform/devices")
          .join(PRO_OBSERVED_INSTANCE),
        fixture.sysfs.join("devices/platform"),
      ),
    ] {
      let original = fs::read_link(&link).unwrap();
      fs::remove_file(&link).unwrap();
      assert_unavailable(&fixture.sample());
      symlink(wrong_target, &link).unwrap();
      assert_unavailable(&fixture.sample());
      fs::remove_file(&link).unwrap();
      symlink(original, link).unwrap();
    }

    // A class link whose index differs from the child it resolves to is not
    // the canonical child.
    fs::remove_file(&class_node).unwrap();
    symlink(&fixture.node, fixture.sysfs.join("class/hwmon/hwmon9")).unwrap();
    assert_unavailable(&fixture.sample());
    fs::remove_file(fixture.sysfs.join("class/hwmon/hwmon9")).unwrap();
    symlink(&fixture.node, class_node).unwrap();
    assert_pro_sample(&fixture.sample());

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
    fs::remove_file(fixture.parent().to_path_buf().join("driver")).unwrap();
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
        read_fan(&fixture.node, 1, "Fan 1").and_then(|reading| reading.rpm),
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
    symlink(fixture.parent(), fixture.node.join("device")).unwrap();
    fixture.write("name", "nct6687\n");
    let replacement = find_endpoint(&fixture.sysfs).unwrap();
    assert_eq!(replacement.path, original.path);
    assert_ne!(replacement, original);
  }

  #[test]
  fn mortar_reads_only_sys_fan1_for_two_distinct_valid_instances() {
    for instance in ["nct6683.8", "nct6683.65528"] {
      let fixture = Fixture::mortar(instance);
      assert_mortar_fan(&fixture.sample(), 926);

      fixture.write("fan1_input", "1111");
      fixture.write("fan15_input", "2222");
      fixture.write("temp2_label", "Diode 0 (curr)");
      fixture.write("temp2_type", "3");
      fixture.write("temp2_input", "26000");
      assert_mortar_fan(&fixture.sample(), 926);

      let renamed = fixture.node.with_file_name("hwmon7");
      fs::rename(&fixture.node, &renamed).unwrap();
      let class = fixture.sysfs.join("class/hwmon");
      fs::remove_file(class.join("hwmon5")).unwrap();
      symlink(renamed, class.join("hwmon7")).unwrap();
      assert_mortar_fan(&fixture.sample(), 926);
    }
  }

  #[test]
  fn mortar_admission_ignores_the_bios_version() {
    for bios in [Some(MORTAR_OBSERVED_BIOS), Some("1.A69"), None] {
      let fixture = Fixture::mortar_with_bios(bios, "nct6683.2720");
      assert_mortar_fan(&fixture.sample(), 926);
    }
  }

  #[test]
  fn mortar_rejects_unknown_boards_instances_and_ownership() {
    for name in NONCANONICAL_INSTANCES {
      assert_unavailable(&Fixture::mortar(name).sample());
    }

    let fixture = Fixture::mortar("nct6683.2720");
    let dmi = fixture.sysfs.join("class/dmi/id");
    let board_name = dmi.join("board_name");
    let original = fs::read(&board_name).unwrap();
    fs::write(&board_name, "MAG B850M MORTAR (MS-7E61)").unwrap();
    let sample = fixture.sample();
    assert!(matches!(
      sample.availability,
      SensorAvailability::Unsupported { .. }
    ));
    assert_eq!(sample.sample, MotherboardSensorSample::default());
    fs::write(board_name, original).unwrap();

    fixture.write("name", "nct6686");
    assert_unavailable(&fixture.sample());
    fixture.write("name", "nct6687");

    for (link, wrong_target) in [
      (
        fixture.parent().join("driver"),
        fixture.sysfs.join("bus/platform"),
      ),
      (
        fixture.parent().join("subsystem"),
        fixture.sysfs.join("class"),
      ),
      (
        fixture.node.join("device"),
        fixture.sysfs.join("devices/platform"),
      ),
      (
        fixture.sysfs.join("class/hwmon/hwmon5"),
        fixture.parent().to_path_buf(),
      ),
      (
        fixture.sysfs.join("bus/platform/devices/nct6683.2720"),
        fixture.sysfs.join("devices/platform"),
      ),
    ] {
      let original = fs::read_link(&link).unwrap();
      fs::remove_file(&link).unwrap();
      assert_unavailable(&fixture.sample());
      symlink(wrong_target, &link).unwrap();
      assert_unavailable(&fixture.sample());
      fs::remove_file(&link).unwrap();
      symlink(original, link).unwrap();
    }
    assert_mortar_fan(&fixture.sample(), 926);
  }

  #[test]
  fn mortar_refuses_distinct_matching_nodes_even_if_only_one_exposes_fan16() {
    let fixture = Fixture::mortar("nct6683.2720");
    add_binding(&fixture.sysfs, "nct6683.2744", "hwmon6");
    assert_unavailable(&fixture.sample());
  }

  #[test]
  fn mortar_preserves_missing_and_invalid_rpm_and_accepts_inclusive_bounds() {
    let fixture = Fixture::mortar("nct6683.2720");
    fixture.write("fan1_input", "1111");
    for raw in ["-1", "0", "65535", "4294967296", "926 RPM"] {
      fixture.write("fan16_input", raw);
      assert_unavailable(&fixture.sample());
    }
    for (raw, expected) in [("1", 1), ("65534", 65_534)] {
      fixture.write("fan16_input", raw);
      assert_mortar_fan(&fixture.sample(), expected);
    }

    fs::remove_file(fixture.node.join("fan16_input")).unwrap();
    assert_unavailable(&fixture.sample());
    fs::create_dir(fixture.node.join("fan16_input")).unwrap();
    assert_unavailable(&fixture.sample());
  }

  #[test]
  fn mortar_discards_samples_if_the_binding_is_replaced_at_the_same_path() {
    let fixture = Fixture::mortar("nct6683.2720");
    let old = find_endpoint(&fixture.sysfs).unwrap();
    let sample = MotherboardSensorSample {
      temperatures: Vec::new(),
      fan_speeds: vec![read_fan(&old.path, 16, "SYS_FAN1").unwrap()],
    };
    let driver = fixture.parent().join("driver");
    let target = fs::read_link(&driver).unwrap();
    fs::rename(&driver, fixture.parent().join("old-driver")).unwrap();
    symlink(target, driver).unwrap();

    assert_unavailable(&finish_sample(&fixture.sysfs, &old, sample));
    fixture.write("fan16_input", "1000");
    assert_mortar_fan(&fixture.sample(), 1000);
  }

  fn assert_pro_sample(collection: &MotherboardSensorCollection) {
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

  fn assert_mortar_fan(collection: &MotherboardSensorCollection, rpm: u32) {
    assert_eq!(collection.availability, SensorAvailability::Available);
    assert_eq!(collection.fan_support, SensorSupport::Supported);
    assert!(collection.sample.temperatures.is_empty());
    assert_eq!(
      collection.sample.fan_speeds,
      [MotherboardFanSpeed {
        name: "SYS_FAN1".to_owned(),
        rpm: Some(rpm),
        status: FanSpeedStatus::Active,
        source: SOURCE.to_owned(),
      }],
    );
  }

  fn assert_unavailable(collection: &MotherboardSensorCollection) {
    assert!(matches!(
      collection.availability,
      SensorAvailability::Unavailable { .. }
    ));
    assert_eq!(collection.sample, MotherboardSensorSample::default());
  }
}
