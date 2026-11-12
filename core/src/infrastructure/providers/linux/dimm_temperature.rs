use std::collections::{HashMap, HashSet};
use std::fs::{self, File, Metadata};
use std::os::unix::fs::{FileExt, MetadataExt};
use std::path::{Path, PathBuf};

use crate::models::hardware::DimmTemperature;

const BOARD_VENDOR: &str = "HARDKERNEL";
const BOARD_NAME: &str = "ODROID-H4";
const BIOS_VERSION: &str = "Dasharo (coreboot+UEFI) v0.9.1";
const DIMM_PART: &str = "M425R1GB4BB0-CWMOD";
const CONTROLLER_ADDRESS: &str = "0000:00:1f.4";
const SPD_SERIAL_OFFSET: u64 = 517;

pub(crate) fn collect(raw_inventory: &str, stderr_empty: bool) -> DimmTemperature {
  collect_from(Path::new("/sys"), raw_inventory, stderr_empty)
}

fn collect_from(sys: &Path, raw_inventory: &str, stderr_empty: bool) -> DimmTemperature {
  let dmi = sys.join("class/dmi/id");
  let vendor = attribute(&dmi, "board_vendor");
  let board = attribute(&dmi, "board_name");
  match (vendor.as_deref(), board.as_deref()) {
    (Some(BOARD_VENDOR), Some(BOARD_NAME)) => {}
    (Some(BOARD_VENDOR), None) | (None, Some(BOARD_NAME)) | (None, None) => {
      return DimmTemperature::Unavailable;
    }
    _ => return DimmTemperature::Unsupported,
  }
  match attribute(&dmi, "bios_version").as_deref() {
    Some(BIOS_VERSION) => {}
    Some(_) => return DimmTemperature::Unsupported,
    None => return DimmTemperature::Unavailable,
  }

  if !stderr_empty {
    return DimmTemperature::Unavailable;
  }
  let Some(dimm) = single_populated_dimm(raw_inventory) else {
    return DimmTemperature::Unavailable;
  };
  if !dimm.supported() {
    return DimmTemperature::Unsupported;
  }
  let Some((locator, serial)) = dimm.identity() else {
    return DimmTemperature::Unavailable;
  };

  let reading =
    find_endpoint(sys).and_then(|endpoint| read_temperature(sys, &endpoint, serial));
  match reading {
    Some(temperature_celsius) => DimmTemperature::Available {
      locator: locator.to_owned(),
      temperature_celsius,
    },
    None => DimmTemperature::Unavailable,
  }
}

/// Raw Type 17 fields are used only during this request and never cached.
#[derive(Default)]
struct Dimm<'a> {
  array_handle: Option<&'a str>,
  size: Option<&'a str>,
  form_factor: Option<&'a str>,
  memory_type: Option<&'a str>,
  locator: Option<&'a str>,
  serial: Option<&'a str>,
  part: Option<&'a str>,
}

impl<'a> Dimm<'a> {
  fn field(&mut self, line: &'a str) -> Option<()> {
    let Some((key, value)) = line.split_once(':') else {
      return Some(());
    };
    let field = match key {
      "Array Handle" => &mut self.array_handle,
      "Size" => &mut self.size,
      "Form Factor" => &mut self.form_factor,
      "Type" => &mut self.memory_type,
      "Locator" => &mut self.locator,
      "Serial Number" => &mut self.serial,
      "Part Number" => &mut self.part,
      _ => return Some(()),
    };
    if field.replace(value.strip_prefix(' ')?).is_some() {
      return None;
    }
    Some(())
  }

  fn complete(&self) -> Option<bool> {
    let size = self.size?;
    if size == "No Module Installed" {
      return Some(false);
    }
    let (amount, unit) = size.split_once(' ')?;
    if amount.parse::<u64>().ok()? == 0 || !matches!(unit, "kB" | "MB" | "GB" | "TB") {
      return None;
    }
    for value in [
      self.form_factor?,
      self.memory_type?,
      self.locator?,
      self.serial?,
      self.part?,
    ] {
      if !is_reported(value) {
        return None;
      }
    }
    Some(true)
  }

  fn supported(&self) -> bool {
    matches!(self.size, Some("8 GB" | "8192 MB"))
      && self.form_factor == Some("SODIMM")
      && self.memory_type == Some("DDR5")
      && self.part == Some(DIMM_PART)
  }

  fn identity(&self) -> Option<(&'a str, [u8; 4])> {
    let locator = self.locator?;
    if locator.trim() != locator
      || locator.chars().any(|c| c.is_control() || c == '\u{fffd}')
    {
      return None;
    }
    Some((locator, decode_serial(self.serial?)?))
  }
}

fn is_reported(value: &str) -> bool {
  let value = value.trim();
  !value.is_empty()
    && !matches!(
      value,
      "Unknown" | "Not Specified" | "Not Provided" | "None" | "Bad Index"
    )
}

enum InventoryRecord<'a> {
  Array { handle: u16, slots: Option<u16> },
  Device(Dimm<'a>),
}

impl<'a> InventoryRecord<'a> {
  fn field(&mut self, line: &'a str) -> Option<()> {
    match self {
      Self::Device(device) => device.field(line),
      Self::Array { slots, .. } => {
        if let Some(raw) = line.strip_prefix("Number Of Devices:") {
          let digits = raw.strip_prefix(' ')?;
          if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
          }
          let count = digits.parse::<u16>().ok()?;
          if count == 0 || slots.replace(count).is_some() {
            return None;
          }
        }
        Some(())
      }
    }
  }
}

#[derive(Default)]
struct Inventory<'a> {
  array_slots: HashMap<u16, usize>,
  device_counts: HashMap<u16, usize>,
  populated: Option<Dimm<'a>>,
}

impl<'a> Inventory<'a> {
  fn finish(&mut self, record: Option<InventoryRecord<'a>>) -> Option<()> {
    match record {
      Some(InventoryRecord::Array { handle, slots }) => {
        self.array_slots.insert(handle, usize::from(slots?));
      }
      Some(InventoryRecord::Device(device)) => {
        let populated = device.complete()?;
        // Empty sockets also occupy a Type 16 slot. Count every Type 17
        // record against the exact physical array it references.
        let array = parse_handle(device.array_handle?)?;
        *self.device_counts.entry(array).or_default() += 1;
        if populated && self.populated.replace(device).is_some() {
          return None;
        }
      }
      None => {}
    }
    Some(())
  }
}

fn parse_handle(value: &str) -> Option<u16> {
  let digits = value.strip_prefix("0x")?;
  if digits.len() != 4 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
    return None;
  }
  u16::from_str_radix(digits, 16).ok()
}

fn single_populated_dimm(raw: &str) -> Option<Dimm<'_>> {
  let mut inventory = Inventory::default();
  let mut current = None;
  let mut handles = HashSet::new();
  let mut lines = raw.lines();
  while let Some(line) = lines.next() {
    if line.trim().is_empty() {
      continue;
    }
    if let Some(header) = line.strip_prefix("Handle ") {
      inventory.finish(current.take())?;
      let (handle, fields) = header.split_once(", DMI type ")?;
      let (kind, length) = fields.split_once(", ")?;
      let handle = parse_handle(handle)?;
      if !handles.insert(handle) {
        return None;
      }

      let heading = lines.next()?;
      if kind == "127" {
        // Type counts alone cannot distinguish a clean SMBIOS 3 prefix
        // from a complete table. Require the decoder's terminal record.
        if length != "4 bytes"
          || heading != "End Of Table"
          || lines.any(|line| !line.trim().is_empty())
          || inventory.array_slots != inventory.device_counts
        {
          return None;
        }
        return inventory.populated;
      }

      let length = length.strip_suffix(" bytes")?.parse::<u8>().ok()?;
      current = match (kind, heading) {
        ("5", "Memory Controller Information") | ("6", "Memory Module Information")
          if length >= 4 =>
        {
          None
        }
        ("16", "Physical Memory Array") if length >= 0x0f => {
          Some(InventoryRecord::Array {
            handle,
            slots: None,
          })
        }
        ("17", "Memory Device") if length >= 0x1b => {
          Some(InventoryRecord::Device(Dimm::default()))
        }
        _ => return None,
      };
    } else if handles.is_empty() {
      if !is_inventory_notice(line) {
        return None;
      }
    } else {
      // Attribute lines start with a tab, and nested values with two.
      // Reject diagnostic lines without inspecting firmware field values.
      let field = line.strip_prefix('\t')?;
      if field.trim_start().starts_with('<')
        || (!field.starts_with('\t') && !field.contains(": "))
      {
        return None;
      }
      if !field.starts_with('\t')
        && let Some(record) = current.as_mut()
      {
        record.field(field)?;
      }
    }
  }
  None
}

fn is_inventory_notice(line: &str) -> bool {
  if matches!(
    line,
    "Getting SMBIOS data from sysfs." | "Scanning /dev/mem for entry point."
  ) {
    return true;
  }
  if let Some(version) = line.strip_prefix("# dmidecode ").or_else(|| {
    line
      .strip_prefix("SMBIOS ")
      .or_else(|| line.strip_prefix("Legacy DMI "))?
      .strip_suffix(" present.")
  }) {
    return version.split('.').all(is_decimal);
  }
  if let Some(address) = line
    .strip_prefix("Table at 0x")
    .and_then(|value| value.strip_suffix('.'))
  {
    return !address.is_empty() && address.bytes().all(|byte| byte.is_ascii_hexdigit());
  }
  line
    .strip_suffix(" bytes.")
    .and_then(|value| value.split_once(" structures occupying "))
    .is_some_and(|(count, size)| is_decimal(count) && is_decimal(size))
}

fn is_decimal(value: &str) -> bool {
  !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
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
  eeprom: PathBuf,
  identity: [(u64, u64); 3],
}

fn find_endpoint(sys: &Path) -> Option<Endpoint> {
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
    if let Some(endpoint) = qualify_endpoint(&entry.path(), &class, &i2c, &controller)
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
) -> Option<Endpoint> {
  let hwmon = fs::canonicalize(class_entry).ok()?;
  let client = fs::canonicalize(hwmon.join("device")).ok()?;
  let adapter = client.parent()?;
  if adapter.parent()? != controller {
    return None;
  }
  let adapter_name = adapter.file_name()?.to_str()?;
  let bus = adapter_name.strip_prefix("i2c-")?.parse::<u32>().ok()?;
  if client.file_name()?.to_str()? != format!("{bus}-0052")
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

  let eeprom = fs::canonicalize(client.join("eeprom")).ok()?;
  if eeprom.parent()? != client {
    return None;
  }
  let eeprom_metadata = fs::metadata(&eeprom).ok()?;
  if !eeprom_metadata.is_file() {
    return None;
  }
  let identity = [
    file_identity(&fs::metadata(&client).ok()?),
    file_identity(&fs::metadata(&hwmon).ok()?),
    file_identity(&eeprom_metadata),
  ];
  Some(Endpoint {
    client,
    hwmon,
    eeprom,
    identity,
  })
}

fn read_temperature(sys: &Path, endpoint: &Endpoint, serial: [u8; 4]) -> Option<f32> {
  let eeprom = File::open(&endpoint.eeprom).ok()?;
  let mut identity = [0; 34];
  if file_identity(&eeprom.metadata().ok()?) != endpoint.identity[2]
    || find_endpoint(sys).as_ref() != Some(endpoint)
    || eeprom.read_at(&mut identity, SPD_SERIAL_OFFSET).ok()? != identity.len()
    || !matches_spd_identity(&identity, serial)
    || find_endpoint(sys).as_ref() != Some(endpoint)
  {
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
  (find_endpoint(sys).as_ref() == Some(endpoint)).then_some(temperature)
}

fn matches_spd_identity(data: &[u8; 34], serial: [u8; 4]) -> bool {
  data[..4] == serial
    && data[4..]
      .strip_prefix(DIMM_PART.as_bytes())
      .is_some_and(|padding| padding.iter().all(|&byte| byte == b' '))
}

fn decode_temperature(value: &str) -> Option<f32> {
  let millidegrees = value.parse::<i32>().ok()?;
  (millidegrees > 0 && millidegrees <= 125_000 && millidegrees % 250 == 0)
    .then_some(millidegrees as f32 / 1000.0)
}

fn file_identity(metadata: &Metadata) -> (u64, u64) {
  (metadata.dev(), metadata.ino())
}

fn links_to(link: &Path, target: &Path) -> bool {
  fs::symlink_metadata(link).is_ok_and(|metadata| metadata.is_symlink())
    && fs::canonicalize(link).is_ok_and(|path| path == target)
}

fn read_hwmon_attribute(directory: &Path, name: &str) -> Option<String> {
  let path = directory.join(name);
  if fs::canonicalize(&path).ok()? != path {
    return None;
  }
  attribute(directory, name)
}

fn attribute(directory: &Path, name: &str) -> Option<String> {
  let mut value = fs::read_to_string(directory.join(name)).ok()?;
  if value.ends_with('\n') {
    value.pop();
  }
  Some(value)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::os::unix::fs::symlink;
  use tempfile::TempDir;

  const SERIAL: [u8; 4] = [0x01, 0x23, 0x45, 0x67];
  const END_OF_TABLE: &str = "Handle 0x007F, DMI type 127, 4 bytes\nEnd Of Table\n";

  const INVENTORY: &str = r#"Handle 0x000F, DMI type 16, 23 bytes
Physical Memory Array
	Number Of Devices: 1

Handle 0x0010, DMI type 17, 92 bytes
Memory Device
	Array Handle: 0x000F
	Size: 8 GB
	Form Factor: SODIMM
	Locator: DIMM 0
	Type: DDR5
	Serial Number: 01234567
	Part Number: M425R1GB4BB0-CWMOD
"#;

  // Synthetic kernel-owned-style topology, not a dump from a real system.
  struct Fixture {
    _directory: TempDir,
    sys: PathBuf,
    controller: PathBuf,
  }

  impl Fixture {
    fn new() -> Self {
      let directory = TempDir::new().unwrap();
      let sys = directory.path().canonicalize().unwrap();
      for relative in [
        "bus/pci/devices",
        "bus/pci/drivers/i801_smbus",
        "bus/i2c/drivers/spd5118",
        "class/dmi/id",
        "class/hwmon",
      ] {
        fs::create_dir_all(sys.join(relative)).unwrap();
      }
      let dmi = sys.join("class/dmi/id");
      write_attr(&dmi, "board_vendor", BOARD_VENDOR);
      write_attr(&dmi, "board_name", BOARD_NAME);
      write_attr(&dmi, "bios_version", BIOS_VERSION);
      let controller = sys.join("devices/pci0000:00").join(CONTROLLER_ADDRESS);
      fs::create_dir_all(&controller).unwrap();
      symlink(
        &controller,
        sys.join("bus/pci/devices").join(CONTROLLER_ADDRESS),
      )
      .unwrap();
      symlink(sys.join("bus/pci"), controller.join("subsystem")).unwrap();
      symlink(
        sys.join("bus/pci/drivers/i801_smbus"),
        controller.join("driver"),
      )
      .unwrap();
      write_attr(&controller, "vendor", "0x8086");
      write_attr(&controller, "device", "0x54a3");
      Self {
        _directory: directory,
        sys,
        controller,
      }
    }

    fn add_client(&self, bus: u32, address: &str, hwmon_index: u32) -> PathBuf {
      let adapter = self.controller.join(format!("i2c-{bus}"));
      let client = adapter.join(format!("{bus}-{address}"));
      fs::create_dir_all(&client).unwrap();
      symlink(self.sys.join("bus/i2c"), adapter.join("subsystem")).unwrap();
      symlink(self.sys.join("bus/i2c"), client.join("subsystem")).unwrap();
      symlink(
        self.sys.join("bus/i2c/drivers/spd5118"),
        client.join("driver"),
      )
      .unwrap();
      let mut spd = [0; 1024];
      let offset = SPD_SERIAL_OFFSET as usize;
      spd[offset..offset + 4].copy_from_slice(&SERIAL);
      spd[offset + 4..offset + 34].fill(b' ');
      spd[offset + 4..offset + 4 + DIMM_PART.len()].copy_from_slice(DIMM_PART.as_bytes());
      fs::write(client.join("eeprom"), spd).unwrap();
      self.add_hwmon(&client, hwmon_index);
      client
    }

    fn add_hwmon(&self, client: &Path, index: u32) -> PathBuf {
      let name = format!("hwmon{index}");
      let class = self.sys.join("class/hwmon");
      let hwmon = client.join("hwmon").join(&name);
      fs::create_dir_all(&hwmon).unwrap();
      symlink(&hwmon, class.join(name)).unwrap();
      symlink(client, hwmon.join("device")).unwrap();
      symlink(class, hwmon.join("subsystem")).unwrap();
      write_attr(&hwmon, "name", "spd5118");
      write_attr(&hwmon, "temp1_enable", "1");
      write_attr(&hwmon, "temp1_input", "40250");
      hwmon
    }

    fn collect(&self, records: &str) -> DimmTemperature {
      collect_from(&self.sys, &format!("{records}\n{END_OF_TABLE}"), true)
    }
  }

  fn write_attr(directory: &Path, name: &str, value: &str) {
    fs::write(directory.join(name), format!("{value}\n")).unwrap();
  }

  fn available(temperature_celsius: f32) -> DimmTemperature {
    DimmTemperature::Available {
      locator: "DIMM 0".into(),
      temperature_celsius,
    }
  }

  #[test]
  fn reads_a_unique_bound_endpoint_and_attributes_the_type17_locator() {
    let fixture = Fixture::new();
    fixture.add_client(7, "0052", 3);
    assert_eq!(fixture.collect(INVENTORY), available(40.25));
  }

  #[test]
  fn requires_a_terminal_record_and_clean_decoder_output_for_attribution() {
    use super::super::dmidecode::parse_dmidecode_memory_info;

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
    let first = find_endpoint(&fixture.sys).unwrap();
    fs::rename(&first.hwmon, first.hwmon.with_extension("detached")).unwrap();
    fs::remove_file(fixture.sys.join("class/hwmon/hwmon3")).unwrap();
    let replacement = fixture.add_hwmon(&client, 3);
    write_attr(&replacement, "temp1_input", "41000");

    assert_eq!(read_temperature(&fixture.sys, &first, SERIAL), None);
    assert_eq!(fixture.collect(INVENTORY), available(41.0));
  }
}
