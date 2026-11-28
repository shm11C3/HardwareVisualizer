use crate::models::{
  FanSpeedStatus, MotherboardFanSpeed, MotherboardSensorCollection,
  MotherboardSensorSample, MotherboardTemperature, SensorAvailability, SensorSupport,
};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const MSI_VENDOR: &str = "Micro-Star International Co., Ltd.";
const PRO_BOARD: &str = "PRO B850-P WIFI (MS-7E56)";
const MORTAR_BOARD: &str = "MAG B850M MORTAR WIFI (MS-7E61)";
const CARBON_BOARD: &str = "MPG X870E CARBON WIFI (MS-7E49)";
const TOMAHAWK_MAX_BOARD: &str = "MAG B850 TOMAHAWK MAX WIFI (MS-7E62)";

/// The chip kind that the bound `nct6683` platform driver publishes as the
/// hwmon `name` of its endpoint.
struct Chip {
  /// Chip designation used in availability reasons.
  label: &'static str,
  /// The hwmon `name` attribute that the qualified endpoint must carry.
  hwmon_name: &'static str,
  /// The source recorded on every reading from this endpoint.
  source: &'static str,
}

const NCT6687D: Chip = Chip {
  label: "NCT6687D",
  hwmon_name: "nct6687",
  source: "NCT6687D (hwmon)",
};

/// A temperature channel assigned by the OEM EC firmware: the hwmon suffix,
/// the `tempN_label` and `tempN_type` metadata that must match on the same
/// node, and the reported name.
struct TemperatureChannel {
  channel: u8,
  label: &'static str,
  kind: &'static str,
  name: &'static str,
}

/// A fan input on its fixed hwmon suffix and the reported name.
struct FanChannel {
  channel: u8,
  name: &'static str,
}

/// A board whose OEM EC firmware defines the scoped hwmon channel
/// assignment. The BIOS version and the EC base address are evidence
/// identities recorded in the specification; they do not gate a read.
struct BoardProfile {
  vendor: &'static str,
  board: &'static str,
  chip: &'static Chip,
  temperatures: &'static [TemperatureChannel],
  fans: &'static [FanChannel],
}

const DIODE_CHANNEL: TemperatureChannel = TemperatureChannel {
  channel: 2,
  label: "Diode 0 (curr)",
  kind: "3",
  name: "Temperature 2",
};
const PRO_TEMPERATURE_CHANNELS: [TemperatureChannel; 3] = [
  DIODE_CHANNEL,
  TemperatureChannel {
    channel: 3,
    label: "Thermistor 15",
    kind: "4",
    name: "Temperature 3",
  },
  TemperatureChannel {
    channel: 4,
    label: "Thermistor 1",
    kind: "4",
    name: "Temperature 4",
  },
];
const FAN_INPUT_1: FanChannel = FanChannel {
  channel: 1,
  name: "Fan 1",
};
const FAN_INPUTS_1_AND_2: [FanChannel; 2] = [
  FAN_INPUT_1,
  FanChannel {
    channel: 2,
    name: "Fan 2",
  },
];

static PROFILES: [BoardProfile; 4] = [
  BoardProfile {
    vendor: MSI_VENDOR,
    board: PRO_BOARD,
    chip: &NCT6687D,
    temperatures: &PRO_TEMPERATURE_CHANNELS,
    fans: &FAN_INPUTS_1_AND_2,
  },
  BoardProfile {
    vendor: MSI_VENDOR,
    board: MORTAR_BOARD,
    chip: &NCT6687D,
    temperatures: &[],
    fans: &[FanChannel {
      channel: 16,
      name: "SYS_FAN1",
    }],
  },
  BoardProfile {
    vendor: MSI_VENDOR,
    board: CARBON_BOARD,
    chip: &NCT6687D,
    temperatures: &[DIODE_CHANNEL],
    fans: &FAN_INPUTS_1_AND_2,
  },
  BoardProfile {
    vendor: MSI_VENDOR,
    board: TOMAHAWK_MAX_BOARD,
    chip: &NCT6687D,
    temperatures: &[DIODE_CHANNEL],
    fans: &[FAN_INPUT_1],
  },
];

pub(crate) fn sample() -> MotherboardSensorCollection {
  sample_at(Path::new("/sys"))
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
      "No supported Linux motherboard sensor profile for this board",
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

fn sample_at(sysfs: &Path) -> MotherboardSensorCollection {
  let profile = match identify_profile(&sysfs.join("class/dmi/id")) {
    Ok(profile) => profile,
    Err(collection) => return collection,
  };
  let chip = profile.chip;
  let Some(endpoint) = find_endpoint(sysfs, profile) else {
    return MotherboardSensorCollection::unavailable(format!(
      "The bound {} hwmon endpoint is unavailable or ambiguous",
      chip.label
    ));
  };

  let temperatures = profile
    .temperatures
    .iter()
    .filter_map(|channel| read_temperature(&endpoint.path, channel, chip.source))
    .collect();
  let fan_speeds = profile
    .fans
    .iter()
    .filter_map(|channel| read_fan(&endpoint.path, channel, chip.source))
    .collect();
  finish_sample(
    sysfs,
    profile,
    &endpoint,
    MotherboardSensorSample {
      temperatures,
      fan_speeds,
    },
  )
}

fn finish_sample(
  sysfs: &Path,
  profile: &BoardProfile,
  endpoint: &Endpoint,
  sample: MotherboardSensorSample,
) -> MotherboardSensorCollection {
  let chip = profile.chip;
  // Linux can reuse the same paths after a binding or hwmon node is replaced.
  if find_endpoint(sysfs, profile).as_ref() != Some(endpoint) {
    return MotherboardSensorCollection::unavailable(format!(
      "The {} hwmon binding changed during collection",
      chip.label
    ));
  }

  let availability = if sample.temperatures.is_empty() && sample.fan_speeds.is_empty() {
    SensorAvailability::unavailable(format!(
      "The {} hwmon endpoint returned no usable readings",
      chip.label
    ))
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

/// Find the one distinct canonical hwmon child of a bound `nct6683.<id>`
/// platform device that carries the profile's chip kind. The same endpoint
/// checks apply to every board profile.
fn find_endpoint(sysfs: &Path, profile: &BoardProfile) -> Option<Endpoint> {
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
    let Some(candidate) = qualify_endpoint(sysfs, path, profile.chip.hwmon_name) else {
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

fn qualify_endpoint(sysfs: &Path, path: PathBuf, hwmon_name: &str) -> Option<Endpoint> {
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
    || read_attribute(&path, "name").as_deref() != Some(hwmon_name)
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
  channel: &TemperatureChannel,
  source: &str,
) -> Option<MotherboardTemperature> {
  let index = channel.channel;
  if read_attribute(path, &format!("temp{index}_label")).as_deref() != Some(channel.label)
    || read_attribute(path, &format!("temp{index}_type")).as_deref() != Some(channel.kind)
  {
    return None;
  }
  let milli_celsius = read_attribute(path, &format!("temp{index}_input"))?
    .parse::<i32>()
    .ok()?;
  (-55_000..=125_000)
    .contains(&milli_celsius)
    .then(|| MotherboardTemperature {
      name: channel.name.to_owned(),
      temperature: milli_celsius as f32 / 1000.0,
      source: source.to_owned(),
    })
}

fn read_fan(
  path: &Path,
  channel: &FanChannel,
  source: &str,
) -> Option<MotherboardFanSpeed> {
  let rpm = read_attribute(path, &format!("fan{}_input", channel.channel))?
    .parse::<u32>()
    .ok()?;
  (1..=65_534).contains(&rpm).then(|| MotherboardFanSpeed {
    name: channel.name.to_owned(),
    rpm: Some(rpm),
    status: FanSpeedStatus::Active,
    source: source.to_owned(),
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

  const SOURCE: &str = NCT6687D.source;

  struct Fixture {
    _directory: TempDir,
    sysfs: PathBuf,
    node: PathBuf,
    profile: &'static BoardProfile,
  }

  impl Fixture {
    fn new() -> Self {
      Self::pro(Some(PRO_OBSERVED_BIOS), PRO_OBSERVED_INSTANCE)
    }

    /// Synthetic PRO sysfs values; the specification's capture shows 26/22/26
    /// degrees Celsius and 518/0 RPM.
    fn pro(bios: Option<&str>, instance: &str) -> Self {
      let fixture = Self::with_binding(PRO_BOARD, bios, instance);
      for (channel, reading) in PRO_TEMPERATURE_CHANNELS
        .iter()
        .zip([26_000, 22_000, 26_000])
      {
        let index = channel.channel;
        fixture.write(&format!("temp{index}_label"), channel.label);
        fixture.write(&format!("temp{index}_type"), channel.kind);
        fixture.write(&format!("temp{index}_input"), &format!("{reading}\n"));
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

    fn carbon(instance: &str) -> Self {
      let fixture = Self::with_binding(CARBON_BOARD, None, instance);
      for (attribute, value) in [
        ("temp2_label", "Diode 0 (curr)"),
        ("temp2_type", "3"),
        ("temp2_input", "56000"),
        ("fan1_input", "747"),
        ("fan2_input", "745"),
      ] {
        fixture.write(attribute, value);
      }
      fixture
    }

    fn with_binding(board: &str, bios: Option<&str>, instance: &str) -> Self {
      let profile = PROFILES
        .iter()
        .find(|profile| profile.board == board)
        .unwrap();
      let directory = tempfile::tempdir().unwrap();
      let sysfs = directory.path().canonicalize().unwrap();
      let dmi = sysfs.join("class/dmi/id");
      fs::create_dir_all(&dmi).unwrap();
      for (attribute, value) in [("board_vendor", profile.vendor), ("board_name", board)]
      {
        fs::write(dmi.join(attribute), format!("{value}\n")).unwrap();
      }
      if let Some(bios) = bios {
        fs::write(dmi.join("bios_version"), format!("{bios}\n")).unwrap();
      }
      let node = add_binding(&sysfs, instance, "hwmon5", profile.chip.hwmon_name);
      Self {
        _directory: directory,
        sysfs,
        node,
        profile,
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

    fn endpoint(&self) -> Option<Endpoint> {
      find_endpoint(&self.sysfs, self.profile)
    }

    fn add_binding(&self, instance: &str, hwmon: &str) -> PathBuf {
      add_binding(&self.sysfs, instance, hwmon, self.profile.chip.hwmon_name)
    }
  }

  fn add_binding(sysfs: &Path, instance: &str, hwmon: &str, hwmon_name: &str) -> PathBuf {
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
    fs::write(node.join("name"), format!("{hwmon_name}\n")).unwrap();
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
    assert!(fixture.endpoint().is_none());
  }

  #[test]
  fn rejects_wrong_kind_and_missing_binding() {
    let fixture = Fixture::new();
    fixture.write("name", "nct6686\n");
    assert!(fixture.endpoint().is_none());
    fixture.write("name", "nct6687\n");
    fs::remove_file(fixture.parent().to_path_buf().join("driver")).unwrap();
    assert!(fixture.endpoint().is_none());
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
        read_temperature(&fixture.node, &DIODE_CHANNEL, SOURCE)
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
        read_fan(&fixture.node, &FAN_INPUT_1, SOURCE).and_then(|reading| reading.rpm),
        expected
      );
    }
  }

  #[test]
  fn detects_replaced_endpoint_even_when_the_class_index_is_reused() {
    let fixture = Fixture::new();
    let original = fixture.endpoint().unwrap();
    fs::rename(&fixture.node, fixture.node.with_file_name("old")).unwrap();
    fs::create_dir(&fixture.node).unwrap();
    symlink(fixture.parent(), fixture.node.join("device")).unwrap();
    fixture.write("name", "nct6687\n");
    let replacement = fixture.endpoint().unwrap();
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
    fixture.add_binding("nct6683.2744", "hwmon6");
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
    let old = fixture.endpoint().unwrap();
    let sample = MotherboardSensorSample {
      temperatures: Vec::new(),
      fan_speeds: vec![read_fan(&old.path, &fixture.profile.fans[0], SOURCE).unwrap()],
    };
    let driver = fixture.parent().join("driver");
    let target = fs::read_link(&driver).unwrap();
    fs::rename(&driver, fixture.parent().join("old-driver")).unwrap();
    symlink(target, driver).unwrap();

    assert_unavailable(&finish_sample(
      &fixture.sysfs,
      fixture.profile,
      &old,
      sample,
    ));
    fixture.write("fan16_input", "1000");
    assert_mortar_fan(&fixture.sample(), 1000);
  }

  #[test]
  fn carbon_reports_only_the_diode_and_two_fan_inputs() {
    for instance in ["nct6683.2592", "nct6683.2600"] {
      let fixture = Fixture::carbon(instance);
      fs::create_dir(fixture.sysfs.join("class/dmi/id/bios_version")).unwrap();
      for (attribute, value) in [
        ("temp1_label", "AMD TSI Addr 98h"),
        ("temp1_type", "6"),
        ("temp1_input", "42000"),
        ("temp3_label", "Thermistor 15"),
        ("temp3_type", "4"),
        ("temp3_input", "25000"),
        ("temp4_label", "Thermistor 1"),
        ("temp4_type", "4"),
        ("temp4_input", "23000"),
        ("fan3_input", "1200"),
      ] {
        fixture.write(attribute, value);
      }

      let collection = fixture.sample();
      assert_eq!(collection.availability, SensorAvailability::Available);
      assert_eq!(collection.fan_support, SensorSupport::Supported);
      assert_eq!(
        collection.sample.temperatures,
        [MotherboardTemperature {
          name: "Temperature 2".to_owned(),
          temperature: 56.0,
          source: SOURCE.to_owned(),
        }]
      );
      assert_eq!(
        collection.sample.fan_speeds,
        [
          MotherboardFanSpeed {
            name: "Fan 1".to_owned(),
            rpm: Some(747),
            status: FanSpeedStatus::Active,
            source: SOURCE.to_owned(),
          },
          MotherboardFanSpeed {
            name: "Fan 2".to_owned(),
            rpm: Some(745),
            status: FanSpeedStatus::Active,
            source: SOURCE.to_owned(),
          },
        ]
      );
    }
  }

  #[test]
  fn carbon_preserves_valid_siblings_when_a_channel_is_unavailable() {
    let fixture = Fixture::carbon("nct6683.2592");
    fixture.write("temp3_label", "Diode 0 (curr)");
    fixture.write("temp3_type", "3");
    fixture.write("temp3_input", "57000");
    fixture.write("temp2_label", "Thermistor 15");
    let collection = fixture.sample();
    assert_eq!(collection.availability, SensorAvailability::Available);
    assert!(collection.sample.temperatures.is_empty());
    assert_eq!(collection.sample.fan_speeds.len(), 2);

    fixture.write("temp2_label", "Diode 0 (curr)");
    fs::remove_file(fixture.node.join("temp2_type")).unwrap();
    assert!(fixture.sample().sample.temperatures.is_empty());
    fixture.write("temp2_type", "3");
    for unavailable in ["0", "65535", "747 RPM"] {
      fixture.write("fan1_input", unavailable);
      let collection = fixture.sample();
      assert_eq!(collection.availability, SensorAvailability::Available);
      assert_eq!(collection.sample.temperatures[0].temperature, 56.0);
      assert_eq!(collection.sample.fan_speeds.len(), 1);
      assert_eq!(collection.sample.fan_speeds[0].name, "Fan 2");
      assert_eq!(collection.sample.fan_speeds[0].rpm, Some(745));
    }

    fixture.write("fan1_input", "65534");
    fs::remove_file(fixture.node.join("fan2_input")).unwrap();
    let collection = fixture.sample();
    assert_eq!(collection.sample.temperatures[0].temperature, 56.0);
    assert_eq!(collection.sample.fan_speeds.len(), 1);
    assert_eq!(collection.sample.fan_speeds[0].rpm, Some(65_534));
  }

  #[test]
  fn carbon_requires_exact_dmi_and_a_unique_native_binding() {
    let fixture = Fixture::carbon("nct6683.2592");
    let board = fixture.sysfs.join("class/dmi/id/board_name");
    fs::write(&board, "MPG X870E CARBON MAX WIFI (MS-7E49)").unwrap();
    assert!(matches!(
      fixture.sample().availability,
      SensorAvailability::Unsupported { .. }
    ));
    fs::remove_file(&board).unwrap();
    assert_unavailable(&fixture.sample());
    fs::write(board, CARBON_BOARD).unwrap();
    let driver_link = fixture.parent().join("driver");
    let driver = fs::read_link(&driver_link).unwrap();
    fs::remove_file(&driver_link).unwrap();
    let dkms_driver = fixture.sysfs.join("bus/platform/drivers/nct6687");
    fs::create_dir(&dkms_driver).unwrap();
    symlink(dkms_driver, &driver_link).unwrap();
    assert_unavailable(&fixture.sample());
    fs::remove_file(&driver_link).unwrap();
    symlink(driver, driver_link).unwrap();
    fixture.add_binding("nct6683.2600", "hwmon6");
    assert_unavailable(&fixture.sample());
  }

  fn tomahawk_max_fixture(bios: Option<&str>, instance: &str) -> Fixture {
    let fixture = Fixture::with_binding(TOMAHAWK_MAX_BOARD, bios, instance);
    for (attribute, value) in [
      ("temp2_label", "Diode 0 (curr)"),
      ("temp2_type", "3"),
      ("temp2_input", "36000"),
      ("fan1_input", "605"),
      ("fan2_input", "745"),
      ("temp3_label", "Thermistor 15"),
      ("temp3_type", "4"),
      ("temp3_input", "25000"),
    ] {
      fixture.write(attribute, value);
    }
    fixture
  }

  #[test]
  fn tomahawk_max_reports_only_temperature_2_and_fan_1_on_the_native_binding() {
    for (bios, instance) in [(None, "nct6683.2592"), (Some("2.A60"), "nct6683.2600")] {
      let fixture = tomahawk_max_fixture(bios, instance);
      let collection = fixture.sample();
      assert_eq!(collection.availability, SensorAvailability::Available);
      assert_eq!(collection.fan_support, SensorSupport::Supported);
      assert_eq!(
        collection.sample.temperatures,
        [MotherboardTemperature {
          name: "Temperature 2".to_owned(),
          temperature: 36.0,
          source: SOURCE.to_owned(),
        }]
      );
      assert_eq!(
        collection.sample.fan_speeds,
        [MotherboardFanSpeed {
          name: "Fan 1".to_owned(),
          rpm: Some(605),
          status: FanSpeedStatus::Active,
          source: SOURCE.to_owned(),
        }]
      );
    }
  }

  #[test]
  fn tomahawk_max_preserves_either_valid_reading_when_the_other_is_unavailable() {
    for (attribute, value) in [
      ("temp2_label", "Thermistor 15"),
      ("temp2_type", "4"),
      ("temp2_input", "36000 mC"),
      ("temp2_input", "-55001"),
      ("temp2_input", "125001"),
    ] {
      let fixture = tomahawk_max_fixture(None, "nct6683.2592");
      fixture.write("temp3_label", "Diode 0 (curr)");
      fixture.write("temp3_type", "3");
      fixture.write(attribute, value);
      let sample = fixture.sample().sample;
      assert!(sample.temperatures.is_empty());
      assert_eq!(sample.fan_speeds[0].rpm, Some(605));
    }

    for (attribute, missing_temperature) in [
      ("temp2_type", true),
      ("temp2_input", true),
      ("fan1_input", false),
    ] {
      let fixture = tomahawk_max_fixture(None, "nct6683.2592");
      fs::remove_file(fixture.node.join(attribute)).unwrap();
      for inaccessible in [false, true] {
        if inaccessible {
          fs::create_dir(fixture.node.join(attribute)).unwrap();
        }
        let collection = fixture.sample();
        assert_eq!(collection.availability, SensorAvailability::Available);
        assert_eq!(
          collection.sample.temperatures.is_empty(),
          missing_temperature
        );
        assert_eq!(
          collection.sample.fan_speeds.is_empty(),
          !missing_temperature
        );
      }
    }

    let fixture = tomahawk_max_fixture(None, "nct6683.2592");
    for raw in ["-1", "0", "65535", "605 RPM"] {
      fixture.write("fan1_input", raw);
      let sample = fixture.sample().sample;
      assert_eq!(sample.temperatures[0].temperature, 36.0);
      assert!(sample.fan_speeds.is_empty());
    }
    fixture.write("temp2_input", "invalid");
    assert_unavailable(&fixture.sample());
  }

  #[test]
  fn tomahawk_max_requires_exact_dmi_and_a_unique_native_binding() {
    let fixture = tomahawk_max_fixture(None, "nct6683.2592");
    let board = fixture.sysfs.join("class/dmi/id/board_name");
    fs::write(&board, "MAG B850 TOMAHAWK WIFI (MS-7E62)").unwrap();
    assert!(matches!(
      fixture.sample().availability,
      SensorAvailability::Unsupported { .. }
    ));
    fs::remove_file(&board).unwrap();
    assert_unavailable(&fixture.sample());
    fs::write(board, TOMAHAWK_MAX_BOARD).unwrap();

    let endpoint = fixture.endpoint().unwrap();
    let sample = fixture.sample().sample;
    let driver_link = fixture.parent().join("driver");
    let native_driver = fs::read_link(&driver_link).unwrap();
    fs::rename(&driver_link, fixture.parent().join("previous-driver")).unwrap();
    let external = fixture.sysfs.join("bus/platform/drivers/nct6687");
    fs::create_dir(&external).unwrap();
    symlink(external, &driver_link).unwrap();
    assert_unavailable(&fixture.sample());
    assert_unavailable(&finish_sample(
      &fixture.sysfs,
      fixture.profile,
      &endpoint,
      sample.clone(),
    ));

    fs::remove_file(&driver_link).unwrap();
    symlink(native_driver, &driver_link).unwrap();
    assert_unavailable(&finish_sample(
      &fixture.sysfs,
      fixture.profile,
      &endpoint,
      sample,
    ));
    fixture.add_binding("nct6683.2600", "hwmon6");
    assert_unavailable(&fixture.sample());
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
