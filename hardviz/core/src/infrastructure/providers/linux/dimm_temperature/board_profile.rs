//! ODROID-H4 (`memory-ddr5-odroid-h4-spd5118.md` rev 3) and Protectli VP2430
//! (`memory-ddr5-protectli-vp2430-spd5118.md` rev 1) board-profile routes.

use std::fs::{self, File};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

use super::inventory::{self, Dimm, is_reported};
use super::sysfs::{attribute, file_identity, links_to, read_hwmon_attribute};
use super::{BoardRoute, decode_temperature};
use crate::models::hardware::DimmTemperature;

pub(super) const BOARD_VENDOR: &str = "HARDKERNEL";
pub(super) const BOARD_NAME: &str = "ODROID-H4";
pub(super) const BIOS_VERSION: &str = "Dasharo (coreboot+UEFI) v0.9.1";
pub(super) const DIMM_PART: &str = "M425R1GB4BB0-CWMOD";
pub(super) const CONTROLLER_ADDRESS: &str = "0000:00:1f.4";
pub(super) const SPD_SERIAL_OFFSET: u64 = 517;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BoardProfile {
  OdroidH4,
  ProtectliVp2430,
}

impl BoardProfile {
  fn slot_address(self) -> &'static str {
    match self {
      Self::OdroidH4 => "0052",
      Self::ProtectliVp2430 => "0050",
    }
  }
}

/// Runs the board-profile route.
///
/// A Protectli VP2430 that passes the board, inventory and endpoint gates
/// uses this route even when the reading then fails: any client the
/// sole-module route could qualify there is the same client and hwmon node,
/// so a fallback could only repeat the same reads after an EEPROM read. The
/// ODROID-H4 rev 3 route stays a separate input and applies only with a
/// reading.
pub(super) fn route(sys: &Path, raw_inventory: &str, stderr_empty: bool) -> BoardRoute {
  let (profile, locator, serial) = match qualify(sys, raw_inventory, stderr_empty) {
    Ok(qualified) => qualified,
    Err(status) => return BoardRoute::Declined(status),
  };
  let Some(endpoint) = find_endpoint(sys, profile) else {
    return BoardRoute::Declined(DimmTemperature::Unavailable);
  };
  match (read_temperature(sys, profile, &endpoint, serial), profile) {
    (Some(temperature_celsius), _) => BoardRoute::Applies(DimmTemperature::Available {
      locator: locator.to_owned(),
      temperature_celsius,
    }),
    (None, BoardProfile::ProtectliVp2430) => {
      BoardRoute::Applies(DimmTemperature::Unavailable)
    }
    (None, BoardProfile::OdroidH4) => BoardRoute::Declined(DimmTemperature::Unavailable),
  }
}

/// Board profile, Type 17 locator and, for ODROID-H4, the expected SPD serial.
type Qualified<'a> = (BoardProfile, &'a str, Option<[u8; 4]>);

fn qualify<'a>(
  sys: &Path,
  raw_inventory: &'a str,
  stderr_empty: bool,
) -> Result<Qualified<'a>, DimmTemperature> {
  let dmi = sys.join("class/dmi/id");
  let vendor = attribute(&dmi, "board_vendor");
  let board = attribute(&dmi, "board_name");
  let profile = match (vendor.as_deref(), board.as_deref()) {
    (Some(BOARD_VENDOR), Some(BOARD_NAME)) => BoardProfile::OdroidH4,
    (Some("Protectli"), Some("VP2430")) => BoardProfile::ProtectliVp2430,
    (Some(BOARD_VENDOR | "Protectli"), None)
    | (None, Some(BOARD_NAME | "VP2430"))
    | (None, None) => {
      return Err(DimmTemperature::Unavailable);
    }
    _ => return Err(DimmTemperature::Unsupported),
  };
  if profile == BoardProfile::OdroidH4 {
    match attribute(&dmi, "bios_version").as_deref() {
      Some(BIOS_VERSION) => {}
      Some(_) => return Err(DimmTemperature::Unsupported),
      None => return Err(DimmTemperature::Unavailable),
    }
  }

  if !stderr_empty {
    return Err(DimmTemperature::Unavailable);
  }
  let dimm =
    single_populated_dimm(raw_inventory, profile).ok_or(DimmTemperature::Unavailable)?;
  if !supported(&dimm, profile) {
    return Err(DimmTemperature::Unsupported);
  }
  let locator = dimm.locator().ok_or(DimmTemperature::Unavailable)?;

  let serial = match profile {
    BoardProfile::OdroidH4 => Some(
      dimm
        .serial
        .and_then(decode_serial)
        .ok_or(DimmTemperature::Unavailable)?,
    ),
    BoardProfile::ProtectliVp2430 => None,
  };
  Ok((profile, locator, serial))
}

fn complete(dimm: &Dimm<'_>, profile: BoardProfile) -> Option<bool> {
  let size = dimm.size?;
  if size == "No Module Installed" {
    return Some(false);
  }
  let (amount, unit) = size.split_once(' ')?;
  if amount.parse::<u64>().ok()? == 0 || !matches!(unit, "kB" | "MB" | "GB" | "TB") {
    return None;
  }
  for value in [dimm.form_factor?, dimm.memory_type?, dimm.locator?] {
    if !is_reported(value) {
      return None;
    }
  }
  if profile == BoardProfile::OdroidH4
    && (!is_reported(dimm.serial?) || !is_reported(dimm.part?))
  {
    return None;
  }
  Some(true)
}

fn supported(dimm: &Dimm<'_>, profile: BoardProfile) -> bool {
  dimm.form_factor == Some("SODIMM")
    && dimm.memory_type == Some("DDR5")
    && (profile == BoardProfile::ProtectliVp2430
      || (matches!(dimm.size, Some("8 GB" | "8192 MB")) && dimm.part == Some(DIMM_PART)))
}

fn single_populated_dimm(raw: &str, profile: BoardProfile) -> Option<Dimm<'_>> {
  let inventory = inventory::parse(raw)?;
  if !inventory.array_counts_match()
    || (profile == BoardProfile::ProtectliVp2430
      && !inventory.single_array_with_one_slot())
  {
    return None;
  }
  let mut populated = None;
  for device in inventory.devices {
    if complete(&device, profile)? && populated.replace(device).is_some() {
      return None;
    }
  }
  populated
}

fn decode_serial(value: &str) -> Option<[u8; 4]> {
  if value.len() != 8
    || !value
      .bytes()
      .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
  {
    return None;
  }

  let mut serial = [0; 4];
  for (index, byte) in serial.iter_mut().enumerate() {
    *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
  }
  (serial != [0; 4] && serial != [u8::MAX; 4]).then_some(serial)
}

#[derive(Debug, PartialEq, Eq)]
struct Endpoint {
  client: PathBuf,
  hwmon: PathBuf,
  eeprom: Option<(PathBuf, (u64, u64))>,
  identity: [(u64, u64); 3],
}

fn find_endpoint(sys: &Path, profile: BoardProfile) -> Option<Endpoint> {
  let pci = sys.join("bus/pci");
  let controller_link = pci.join("devices").join(CONTROLLER_ADDRESS);
  let controller = fs::canonicalize(&controller_link).ok()?;
  if !controller.starts_with(sys.join("devices"))
    || controller.file_name()?.to_str()? != CONTROLLER_ADDRESS
    || !links_to(&controller_link, &controller)
    || !links_to(&controller.join("subsystem"), &pci)
    || !links_to(&controller.join("driver"), &pci.join("drivers/i801_smbus"))
    || attribute(&controller, "vendor").as_deref() != Some("0x8086")
    || attribute(&controller, "device").as_deref() != Some("0x54a3")
  {
    return None;
  }

  let mut match_found = None;
  let class = sys.join("class/hwmon");
  let i2c = sys.join("bus/i2c");
  for entry in fs::read_dir(&class).ok()? {
    let entry = entry.ok()?;
    if let Some(endpoint) =
      qualify_endpoint(&entry.path(), &class, &i2c, &controller, profile)
      && match_found.replace(endpoint).is_some()
    {
      return None;
    }
  }
  match_found
}

fn qualify_endpoint(
  class_entry: &Path,
  class: &Path,
  i2c: &Path,
  controller: &Path,
  profile: BoardProfile,
) -> Option<Endpoint> {
  let hwmon = fs::canonicalize(class_entry).ok()?;
  let client = fs::canonicalize(hwmon.join("device")).ok()?;
  let adapter = client.parent()?;
  if adapter.parent()? != controller {
    return None;
  }
  let adapter_name = adapter.file_name()?.to_str()?;
  let bus = adapter_name.strip_prefix("i2c-")?.parse::<u32>().ok()?;
  if adapter_name != format!("i2c-{bus}")
    || client.file_name()?.to_str()? != format!("{bus}-{}", profile.slot_address())
    || hwmon.parent()? != client.join("hwmon")
    || !links_to(class_entry, &hwmon)
    || !links_to(&hwmon.join("device"), &client)
    || !links_to(&hwmon.join("subsystem"), class)
    || !links_to(&adapter.join("subsystem"), i2c)
    || !links_to(&client.join("subsystem"), i2c)
    || !links_to(&client.join("driver"), &i2c.join("drivers/spd5118"))
    || attribute(&hwmon, "name").as_deref() != Some("spd5118")
  {
    return None;
  }

  let eeprom = if profile == BoardProfile::OdroidH4 {
    let path = fs::canonicalize(client.join("eeprom")).ok()?;
    let metadata = fs::metadata(&path).ok()?;
    if path.parent()? != client || !metadata.is_file() {
      return None;
    }
    Some((path, file_identity(&metadata)))
  } else {
    None
  };
  let identity = [
    file_identity(&fs::metadata(&client).ok()?),
    file_identity(&fs::metadata(&hwmon).ok()?),
    file_identity(&fs::symlink_metadata(client.join("driver")).ok()?),
  ];
  Some(Endpoint {
    client,
    hwmon,
    eeprom,
    identity,
  })
}

fn read_temperature(
  sys: &Path,
  profile: BoardProfile,
  endpoint: &Endpoint,
  serial: Option<[u8; 4]>,
) -> Option<f32> {
  if let Some(serial) = serial {
    let (path, expected_identity) = endpoint.eeprom.as_ref()?;
    let eeprom = File::open(path).ok()?;
    let mut identity = [0; 34];
    if file_identity(&eeprom.metadata().ok()?) != *expected_identity
      || find_endpoint(sys, profile).as_ref() != Some(endpoint)
      || eeprom.read_at(&mut identity, SPD_SERIAL_OFFSET).ok()? != identity.len()
      || !matches_spd_identity(&identity, serial)
    {
      return None;
    }
  }
  if find_endpoint(sys, profile).as_ref() != Some(endpoint) {
    return None;
  }

  let enabled = read_hwmon_attribute(&endpoint.hwmon, "temp1_enable")?;
  if enabled != "1" {
    return None;
  }
  let temperature =
    decode_temperature(&read_hwmon_attribute(&endpoint.hwmon, "temp1_input")?)?;
  // A request may overlap driver detach or re-enumeration. Never publish
  // readings from a replacement binding under the previous DIMM identity.
  (find_endpoint(sys, profile).as_ref() == Some(endpoint)).then_some(temperature)
}

fn matches_spd_identity(data: &[u8; 34], serial: [u8; 4]) -> bool {
  data[..4] == serial
    && data[4..]
      .strip_prefix(DIMM_PART.as_bytes())
      .is_some_and(|padding| padding.iter().all(|&byte| byte == b' '))
}

#[cfg(test)]
mod tests {
  use super::super::test_fixture::*;
  use super::*;
  use std::os::unix::fs::symlink;

  /// The board-profile route alone, without the sole-module fallback.
  fn collect_from(sys: &Path, raw: &str, stderr_empty: bool) -> DimmTemperature {
    route(sys, raw, stderr_empty).into_temperature()
  }

  #[test]
  fn protectli_uses_the_physical_slot_without_reading_eeprom_or_firmware_identity() {
    let fixture = Fixture::protectli();
    let client = fixture.add_client(7, "0050", 0);
    fs::remove_file(client.join("eeprom")).unwrap();
    let inventory = INVENTORY
      .replace("8 GB", "16 GB")
      .replace("M425R1GB4BB0-CWMOD", "Different DDR5 part");
    for serial in [
      "\tSerial Number: 00000000\n",
      "\tSerial Number: 67452301\n",
      "",
    ] {
      let records = inventory.replace("\tSerial Number: 01234567\n", serial);
      assert_eq!(fixture.collect(&records), available(40.25));
    }
    write_attr(&fixture.sys.join("class/dmi/id"), "bios_version", "Other");
    assert_eq!(fixture.collect(&inventory), available(40.25));
    write_attr(&fixture.sys.join("class/dmi/id"), "board_name", "VP2430e");
    assert_eq!(fixture.collect(&inventory), DimmTemperature::Unsupported);
  }

  #[test]
  fn protectli_requires_a_complete_single_slot_inventory() {
    let fixture = Fixture::protectli();
    fixture.add_client(7, "0050", 0);
    for inventory in [
      INVENTORY.replace("Number Of Devices: 1", "Number Of Devices: 2"),
      INVENTORY.replace("Array Handle: 0x000F", "Array Handle: 0x000E"),
      INVENTORY.replace("Locator: DIMM 0", "Locator: Unknown"),
      INVENTORY.replace("Size: 8 GB", "Size: No Module Installed"),
      INVENTORY.replace("Handle 0x0010", "Handle 0x000F"),
      format!(
        "{INVENTORY}{}",
        INVENTORY
          .replace("0x000F", "0x000E")
          .replace("0x0010", "0x0011")
      ),
    ] {
      assert_eq!(fixture.collect(&inventory), DimmTemperature::Unavailable);
    }
    assert_eq!(
      fixture.collect(&INVENTORY.replace("DDR5", "DDR4")),
      DimmTemperature::Unsupported
    );
    assert_eq!(
      collect_from(&fixture.sys, INVENTORY, true),
      DimmTemperature::Unavailable
    );
    assert_eq!(
      collect_from(&fixture.sys, &format!("{INVENTORY}{END_OF_TABLE}"), false),
      DimmTemperature::Unavailable
    );
  }

  #[test]
  fn protectli_requires_a_unique_qualified_enabled_hub_and_valid_input() {
    let fixture = Fixture::protectli();
    let wrong = fixture.add_client(6, "0052", 1);
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);
    let client = fixture.add_client(7, "0050", 0);
    let hwmon = client.join("hwmon/hwmon0");
    for (reading, expected) in [
      ("0", DimmTemperature::Unavailable),
      ("250", available(0.25)),
      ("125000", available(125.0)),
      ("125250", DimmTemperature::Unavailable),
      ("40251", DimmTemperature::Unavailable),
      ("bad", DimmTemperature::Unavailable),
    ] {
      write_attr(&hwmon, "temp1_input", reading);
      assert_eq!(fixture.collect(INVENTORY), expected);
    }
    write_attr(&hwmon, "temp1_input", "40250");
    write_attr(&hwmon, "temp1_enable", "0");
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);
    write_attr(&hwmon, "temp1_enable", "1");
    fs::remove_file(client.join("driver")).unwrap();
    symlink(wrong, client.join("driver")).unwrap();
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);
    fs::remove_file(client.join("driver")).unwrap();
    symlink(
      fixture.sys.join("bus/i2c/drivers/spd5118"),
      client.join("driver"),
    )
    .unwrap();
    fixture.add_hwmon(&client, 2);
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);
  }

  #[test]
  fn reads_a_unique_bound_endpoint_and_attributes_the_type17_locator() {
    let fixture = Fixture::new();
    fixture.add_client(7, "0052", 3);
    assert_eq!(fixture.collect(INVENTORY), available(40.25));
  }

  #[test]
  fn requires_a_terminal_record_and_clean_decoder_output_for_attribution() {
    use crate::infrastructure::providers::linux::dmidecode::parse_dmidecode_memory_info;

    let fixture = Fixture::new();
    fixture.add_client(7, "0052", 3);
    let prefix = format!(
      "# dmidecode 3.5\nGetting SMBIOS data from sysfs.\nSMBIOS 3.5.0 present.\n\n{INVENTORY}\n"
    );
    let complete = format!("{prefix}{END_OF_TABLE}\n\t\n");
    assert_eq!(
      collect_from(&fixture.sys, &complete, true),
      available(40.25)
    );
    assert_eq!(
      parse_dmidecode_memory_info(&prefix),
      parse_dmidecode_memory_info(&complete)
    );
    assert_eq!(parse_dmidecode_memory_info(&prefix).memory_count, 1);

    for (raw, stderr_empty) in [
      (prefix.clone(), true),
      (
        format!("{prefix}{}", END_OF_TABLE.lines().next().unwrap()),
        true,
      ),
      (complete.replace("127, 4 bytes", "127, 5 bytes"), true),
      (complete.replace("127, 4 bytes", "127, +4 bytes"), true),
      (complete.replace("0x007F", "0x+07F"), true),
      (complete.replace("0x007F", "0x0010"), true),
      (format!("{prefix}{END_OF_TABLE}{END_OF_TABLE}"), true),
      (format!("{prefix}{END_OF_TABLE}{INVENTORY}"), true),
      (
        format!("{prefix}\tBank Locator: {}", END_OF_TABLE.trim()),
        true,
      ),
      (format!("Invalid entry point\n{complete}"), true),
      (format!("{prefix}\t<TRUNCATED>\n{END_OF_TABLE}"), true),
      (complete.clone(), false),
    ] {
      assert_eq!(
        collect_from(&fixture.sys, &raw, stderr_empty),
        DimmTemperature::Unavailable,
        "{raw:?}, stderr_empty={stderr_empty}"
      );
    }

    let firmware_value =
      format!("{prefix}\tBank Locator: Error: <TRUNCATED>\n{END_OF_TABLE}");
    assert_eq!(
      collect_from(&fixture.sys, &firmware_value, true),
      available(40.25)
    );
  }

  #[test]
  fn adapter_and_client_require_the_i2c_bus_subsystem() {
    let fixture = Fixture::new();
    let client = fixture.add_client(7, "0052", 3);
    let adapter = client.parent().unwrap();
    let i2c_bus = fixture.sys.join("bus/i2c");
    assert_eq!(fixture.collect(INVENTORY), available(40.25));

    // The I2C character device is a separate child of the bus adapter.
    for class in ["class/i2c-adapter", "class/i2c-dev"] {
      let class = fixture.sys.join(class);
      fs::create_dir_all(&class).unwrap();
      for node in [adapter, &client] {
        let subsystem = node.join("subsystem");
        fs::remove_file(&subsystem).unwrap();
        symlink(&class, &subsystem).unwrap();
        assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);
        fs::remove_file(&subsystem).unwrap();
        symlink(&i2c_bus, &subsystem).unwrap();
      }
    }
    assert_eq!(fixture.collect(INVENTORY), available(40.25));
  }

  #[test]
  fn physical_array_counts_require_complete_type17_inventory_including_empty_slots() {
    let fixture = Fixture::new();
    fixture.add_client(7, "0052", 3);
    let (array, device) = INVENTORY.split_once("\n\n").unwrap();
    let two_slots = INVENTORY.replace("Number Of Devices: 1", "Number Of Devices: 2");
    let empty_slot = r#"Handle 0x0011, DMI type 17, 92 bytes
Memory Device
	Array Handle: 0x000F
	Size: No Module Installed
"#;
    assert_eq!(fixture.collect(&two_slots), DimmTemperature::Unavailable);
    assert_eq!(
      fixture.collect(&format!("{two_slots}\n{empty_slot}")),
      available(40.25)
    );

    let other_array = array.replace("0x000F", "0x002F");
    let other_array_empty_slot = empty_slot.replace("0x000F", "0x002F");
    assert_eq!(
      fixture.collect(&format!(
        "{INVENTORY}\n{other_array}\n\n{other_array_empty_slot}"
      )),
      available(40.25)
    );
    for incomplete in [
      device.to_owned(),
      INVENTORY.replace("DMI type 16, 23 bytes", "DMI type 16, 14 bytes"),
      INVENTORY.replace("Number Of Devices: 1", ""),
      INVENTORY.replace("Number Of Devices: 1", "Number Of Devices: 0"),
      INVENTORY.replace("Number Of Devices: 1", "Number Of Devices: +1"),
      INVENTORY.replace(
        "Number Of Devices: 1",
        "Number Of Devices: 1\n\tNumber Of Devices: 1",
      ),
      INVENTORY.replace("Array Handle: 0x000F", ""),
      INVENTORY.replace("Array Handle: 0x000F", "Array Handle: 0x002F"),
      INVENTORY.replace("Array Handle: 0x000F", "Array Handle: 0x+00F"),
      INVENTORY.replace(
        "Array Handle: 0x000F",
        "Array Handle: 0x000F\n\tArray Handle: 0x000F",
      ),
      INVENTORY.replace("Handle 0x0010, DMI type 17", "Handle 0x000F, DMI type 17"),
      format!("{INVENTORY}\n{array}\n"),
      format!("{INVENTORY}\n{other_array}\n"),
      format!("{INVENTORY}\n{empty_slot}"),
      format!("{two_slots}\n{other_array_empty_slot}"),
    ] {
      assert_eq!(fixture.collect(&incomplete), DimmTemperature::Unavailable);
    }
  }

  #[test]
  fn wrong_profile_controller_address_or_binding_never_qualifies() {
    let fixture = Fixture::new();
    fixture.add_client(7, "0053", 3);
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);

    let client = fixture.add_client(8, "0052", 4);
    let dmi = fixture.sys.join("class/dmi/id");
    write_attr(&dmi, "bios_version", "Dasharo (coreboot+UEFI) v0.9.2");
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unsupported);
    write_attr(&dmi, "bios_version", BIOS_VERSION);

    write_attr(&fixture.controller, "device", "0x54a2");
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);
    write_attr(&fixture.controller, "device", "0x54a3");

    fs::remove_file(client.join("driver")).unwrap();
    symlink(
      fixture.sys.join("bus/i2c/drivers/other"),
      client.join("driver"),
    )
    .unwrap();
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);

    fs::remove_file(client.join("driver")).unwrap();
    symlink(
      fixture.sys.join("bus/i2c/drivers/spd5118"),
      client.join("driver"),
    )
    .unwrap();
    fs::remove_file(dmi.join("board_vendor")).unwrap();
    fs::remove_file(dmi.join("board_name")).unwrap();
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);
    write_attr(&dmi, "board_vendor", BOARD_VENDOR);
    write_attr(&dmi, "board_name", BOARD_NAME);

    fs::remove_file(client.join("hwmon/hwmon4/device")).unwrap();
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);
  }

  #[test]
  fn duplicate_bound_endpoints_are_ambiguous() {
    let fixture = Fixture::new();
    let first = fixture.add_client(7, "0052", 3);
    fixture.add_hwmon(&first, 4);
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);
  }

  #[test]
  fn a_second_module_or_incomplete_identity_leaves_the_temperature_unavailable() {
    let fixture = Fixture::new();
    fixture.add_client(7, "0052", 3);
    let (_, device) = INVENTORY.split_once("\n\n").unwrap();
    let two_slots = INVENTORY.replace("Number Of Devices: 1", "Number Of Devices: 2");
    let duplicate = format!("{two_slots}\n{}", device.replace("0x0010", "0x0011"));
    for invalid in [
      duplicate,
      INVENTORY.repeat(2),
      INVENTORY.replace("Size: 8 GB", "Size: Unknown"),
      INVENTORY.replace("Type: DDR5", "Type: Unknown"),
      INVENTORY.replace("Form Factor: SODIMM", "Form Factor: Not Specified"),
      INVENTORY.replace("Size: 8 GB", ""),
      INVENTORY.replace("Part Number: M425R1GB4BB0-CWMOD", ""),
      INVENTORY.replace("Locator: DIMM 0", "Locator: "),
      INVENTORY.replace("Locator: DIMM 0", "Locator: Not Specified"),
      INVENTORY.replace("01234567", "67452301"),
      INVENTORY.replace("01234567", "00000000"),
      INVENTORY.replace("01234567", "ffffffff"),
      INVENTORY.replace("01234567", "ABCDEFAA"),
      INVENTORY.replace("01234567", "1234567"),
      INVENTORY.replace("DMI type 17, 92", "DMI type 17, 20"),
      String::new(),
    ] {
      assert_eq!(fixture.collect(&invalid), DimmTemperature::Unavailable);
    }
    let other_part = INVENTORY.replace(DIMM_PART, "OTHER");
    assert_eq!(fixture.collect(&other_part), DimmTemperature::Unsupported);
  }

  #[test]
  fn requires_exact_spd_fields_and_a_full_read() {
    let fixture = Fixture::new();
    let client = fixture.add_client(7, "0052", 3);
    let eeprom = client.join("eeprom");
    let valid = fs::read(&eeprom).unwrap();
    fs::write(&eeprom, &valid[..550]).unwrap();
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);

    let mut wrong = valid;
    wrong[SPD_SERIAL_OFFSET as usize + 4 + DIMM_PART.len()] = b'X';
    fs::write(&eeprom, &wrong).unwrap();
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);

    // An EEPROM redirected outside the bound I2C client is never accepted.
    fs::remove_file(&eeprom).unwrap();
    let other = fixture.sys.join("other-eeprom");
    fs::write(&other, wrong).unwrap();
    symlink(other, eeprom).unwrap();
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);
  }

  #[test]
  fn sensor_enable_and_quarter_degree_boundaries_preserve_unavailability() {
    let fixture = Fixture::new();
    let hwmon = fixture.add_client(7, "0052", 3).join("hwmon/hwmon3");
    write_attr(&hwmon, "temp1_enable", "0");
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);
    write_attr(&hwmon, "temp1_enable", "1");
    for (value, expected) in [
      ("0", DimmTemperature::Unavailable),
      ("250", available(0.25)),
      ("125000", available(125.0)),
      ("125250", DimmTemperature::Unavailable),
      ("40251", DimmTemperature::Unavailable),
      ("-250", DimmTemperature::Unavailable),
      ("40250 trailing", DimmTemperature::Unavailable),
    ] {
      write_attr(&hwmon, "temp1_input", value);
      assert_eq!(fixture.collect(INVENTORY), expected);
    }
    fs::remove_file(hwmon.join("temp1_enable")).unwrap();
    assert_eq!(fixture.collect(INVENTORY), DimmTemperature::Unavailable);
  }

  #[test]
  fn a_recreated_hwmon_node_requires_a_fresh_qualification() {
    let fixture = Fixture::new();
    let client = fixture.add_client(7, "0052", 3);
    let first = find_endpoint(&fixture.sys, BoardProfile::OdroidH4).unwrap();
    fs::rename(&first.hwmon, first.hwmon.with_extension("detached")).unwrap();
    fs::remove_file(fixture.sys.join("class/hwmon/hwmon3")).unwrap();
    let replacement = fixture.add_hwmon(&client, 3);
    write_attr(&replacement, "temp1_input", "41000");

    assert_eq!(
      read_temperature(&fixture.sys, BoardProfile::OdroidH4, &first, Some(SERIAL)),
      None
    );
    assert_eq!(fixture.collect(INVENTORY), available(41.0));
  }
}
