//! Sole populated DDR5 module through the sole `spd5118` I²C client, attributed
//! by SPD identity confirmation (`memory-ddr5-sole-module-spd5118.md` rev 1).
//!
//! The route never opens a bus device, scans, binds, selects a page or reads
//! alarm attributes. It reads the client's own read-only `eeprom` slice and the
//! hwmon `temp1_enable` / `temp1_input` attributes only.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::ErrorKind;
use std::ops::Range;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

use super::decode_temperature;
use super::inventory::{self, Dimm, Inventory, is_label_text, is_reported};
use super::sysfs::{attribute, file_identity, links_to, read_hwmon_attribute};
use crate::models::hardware::DimmTemperature;

const DRIVER: &str = "spd5118";
/// SPD bytes 512–550: manufacturer ID, location, date, serial and module part.
const IDENTITY_OFFSET: u64 = 512;
const IDENTITY_LEN: usize = 39;
/// SPD bytes 512–520 inside the identity slice.
const LONG_SERIAL_BYTES: Range<usize> = 0..9;
/// SPD bytes 517–520 inside the identity slice.
const SERIAL_BYTES: Range<usize> = 5..9;
/// SPD bytes 521–550 inside the identity slice.
const PART_BYTES: Range<usize> = 9..IDENTITY_LEN;
/// Hub device type ID in 7-bit address bits 6:3.
const HUB_DEVICE_TYPE: u16 = 0b1010;

/// Runs this route. Every failed gate leaves the temperature unavailable;
/// configurations outside the specification's scope report unsupported.
pub(super) fn collect_from(
  sys: &Path,
  raw_inventory: &str,
  stderr_empty: bool,
) -> DimmTemperature {
  match sample(sys, raw_inventory, stderr_empty) {
    Ok(temperature) | Err(temperature) => temperature,
  }
}

fn sample(
  sys: &Path,
  raw_inventory: &str,
  stderr_empty: bool,
) -> Result<DimmTemperature, DimmTemperature> {
  if !stderr_empty {
    return Err(DimmTemperature::Unavailable);
  }
  let inventory = inventory::parse(raw_inventory).ok_or(DimmTemperature::Unavailable)?;
  let dimm = sole_ddr5_module(&inventory)?;
  let locator = label(dimm).ok_or(DimmTemperature::Unavailable)?;
  let expected = ExpectedIdentity::from_type17(dimm);

  let endpoint = find_endpoint(sys)?;
  let expected = expected.ok_or(DimmTemperature::Unavailable)?;
  let temperature_celsius =
    read_temperature(sys, &endpoint, &expected).ok_or(DimmTemperature::Unavailable)?;
  Ok(DimmTemperature::Available {
    locator,
    temperature_celsius,
  })
}

enum Socket {
  Empty,
  Populated,
}

/// Size 0 is rendered as no installed module. Any other value must be a known
/// nonzero size; an unknown size cannot be placed on either side.
fn socket(dimm: &Dimm<'_>) -> Option<Socket> {
  let size = dimm.size?;
  if size == "No Module Installed" {
    return Some(Socket::Empty);
  }
  let (amount, unit) = size.split_once(' ')?;
  if amount.is_empty()
    || !amount.bytes().all(|byte| byte.is_ascii_digit())
    || amount.parse::<u64>().ok()? == 0
    || !matches!(unit, "kB" | "MB" | "GB" | "TB")
  {
    return None;
  }
  Some(Socket::Populated)
}

/// Across every array exactly one Type 17 is populated and the rest are empty.
fn sole_ddr5_module<'i, 'a>(
  inventory: &'i Inventory<'a>,
) -> Result<&'i Dimm<'a>, DimmTemperature> {
  if !inventory.array_counts_match() {
    return Err(DimmTemperature::Unavailable);
  }
  let mut populated = Vec::new();
  for dimm in &inventory.devices {
    match socket(dimm).ok_or(DimmTemperature::Unavailable)? {
      Socket::Empty => {}
      Socket::Populated => populated.push(dimm),
    }
  }
  let [dimm] = populated[..] else {
    return Err(DimmTemperature::Unsupported);
  };
  match dimm.memory_type {
    Some("DDR5") => Ok(dimm),
    Some(memory_type) if is_reported(memory_type) => Err(DimmTemperature::Unsupported),
    _ => Err(DimmTemperature::Unavailable),
  }
}

/// The Device Locator, followed by the Bank Locator when it is present.
fn label(dimm: &Dimm<'_>) -> Option<String> {
  let locator = dimm.locator().filter(|locator| is_reported(locator))?;
  match dimm
    .bank_locator
    .filter(|bank| is_reported(bank) && is_label_text(bank))
  {
    Some(bank) => Some(format!("{locator} / {bank}")),
    None => Some(locator.to_owned()),
  }
}

/// The Type 17 serial and part to confirm against the hub's SPD bytes. Kept
/// local to the request and never displayed, persisted or sent.
#[derive(Debug, PartialEq, Eq)]
struct ExpectedIdentity<'a> {
  serial: Serial,
  part: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
enum Serial {
  /// Eight digits: SPD bytes 517–520 in increasing address order.
  Short([u8; 4]),
  /// Eighteen digits: SPD bytes 512–520 in increasing address order.
  Long([u8; 9]),
}

impl<'a> ExpectedIdentity<'a> {
  fn from_type17(dimm: &Dimm<'a>) -> Option<Self> {
    Some(Self {
      serial: decode_serial(dimm.serial?)?,
      part: usable_part(dimm.part)?,
    })
  }

  fn confirms(&self, identity: &[u8; IDENTITY_LEN]) -> bool {
    let serial = &identity[SERIAL_BYTES];
    if serial.iter().all(|&byte| byte == 0) || serial.iter().all(|&byte| byte == u8::MAX)
    {
      return false;
    }
    let serial_matches = match &self.serial {
      Serial::Short(expected) => identity[SERIAL_BYTES] == expected[..],
      Serial::Long(expected) => identity[LONG_SERIAL_BYTES] == expected[..],
    };
    serial_matches
      && self
        .part
        .is_none_or(|part| spd_part(&identity[PART_BYTES]) == part.as_bytes())
  }
}

/// Exactly 8 or 18 hexadecimal digits in either letter case, read forward two
/// digits per byte. No other length, grouping, prefix or byte order.
fn decode_serial(value: &str) -> Option<Serial> {
  if !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
    return None;
  }
  let byte =
    |index: usize| u8::from_str_radix(value.get(index * 2..index * 2 + 2)?, 16).ok();
  match value.len() {
    8 => {
      let mut serial = [0; 4];
      for (index, slot) in serial.iter_mut().enumerate() {
        *slot = byte(index)?;
      }
      Some(Serial::Short(serial))
    }
    18 => {
      let mut serial = [0; 9];
      for (index, slot) in serial.iter_mut().enumerate() {
        *slot = byte(index)?;
      }
      Some(Serial::Long(serial))
    }
    _ => None,
  }
}

/// `Some(None)` when the Type 17 part is absent, so the serial alone confirms.
/// `None` when a reported part is not printable ASCII and cannot be compared.
fn usable_part(part: Option<&str>) -> Option<Option<&str>> {
  let Some(part) = part else {
    return Some(None);
  };
  if part.bytes().all(|byte| byte == b' ') || matches!(part, "Not Specified" | "Unknown")
  {
    return Some(None);
  }
  part
    .bytes()
    .all(|byte| byte == b' ' || byte.is_ascii_graphic())
    .then_some(Some(part))
}

/// SPD module part with trailing ASCII spaces removed.
fn spd_part(bytes: &[u8]) -> &[u8] {
  let end = bytes
    .iter()
    .rposition(|&byte| byte != b' ')
    .map_or(0, |last| last + 1);
  &bytes[..end]
}

#[derive(Debug, PartialEq, Eq)]
struct Endpoint {
  client: PathBuf,
  hwmon: PathBuf,
  eeprom: PathBuf,
  /// Client, hwmon child, driver link and EEPROM file identities.
  identity: [(u64, u64); 4],
}

/// Qualifies the sole `spd5118` client in the system and its ownership links.
fn find_endpoint(sys: &Path) -> Result<Endpoint, DimmTemperature> {
  let clients = spd5118_clients(sys).ok_or(DimmTemperature::Unavailable)?;
  let [client] = &clients[..] else {
    return Err(DimmTemperature::Unsupported);
  };
  // An unbound hub is outside this route's scope.
  if fs::symlink_metadata(client.join("driver"))
    .is_err_and(|error| error.kind() == ErrorKind::NotFound)
  {
    return Err(DimmTemperature::Unsupported);
  }
  qualify_endpoint(sys, client).ok_or(DimmTemperature::Unavailable)
}

/// Canonical I²C devices named `spd5118` or bound to the `spd5118` driver,
/// bound or not. Aliases of one device count once.
fn spd5118_clients(sys: &Path) -> Option<Vec<PathBuf>> {
  let i2c = sys.join("bus/i2c");
  let driver = i2c.join("drivers").join(DRIVER);
  let entries = match fs::read_dir(i2c.join("devices")) {
    Ok(entries) => entries,
    Err(error) if error.kind() == ErrorKind::NotFound => return Some(Vec::new()),
    Err(_) => return None,
  };
  let mut devices = BTreeSet::new();
  for entry in entries {
    devices.insert(fs::canonicalize(entry.ok()?.path()).ok()?);
  }
  let mut clients = Vec::new();
  for device in devices {
    if attribute(&device, "name")? == DRIVER || links_to(&device.join("driver"), &driver)
    {
      clients.push(device);
    }
  }
  Some(clients)
}

/// 7-bit client address from the canonical `<bus>-<address>` client name, when
/// it belongs to the hub device type `1010b`. The HID bits are never a slot.
fn hub_address(client_name: &str) -> Option<(&str, u16)> {
  let (bus, address) = client_name.split_once('-')?;
  if bus.is_empty()
    || !bus.bytes().all(|byte| byte.is_ascii_digit())
    || address.len() != 4
    || !address.bytes().all(|byte| byte.is_ascii_hexdigit())
  {
    return None;
  }
  let address = u16::from_str_radix(address, 16).ok()?;
  (address <= 0x7f && address >> 3 == HUB_DEVICE_TYPE).then_some((bus, address))
}

fn qualify_endpoint(sys: &Path, client: &Path) -> Option<Endpoint> {
  let i2c = sys.join("bus/i2c");
  let (bus, _) = hub_address(client.file_name()?.to_str()?)?;
  let adapter = client.parent()?;
  if adapter.file_name()?.to_str()? != format!("i2c-{bus}")
    || !links_to(&client.join("driver"), &i2c.join("drivers").join(DRIVER))
    || !links_to(&client.join("subsystem"), &i2c)
    || !links_to(&adapter.join("subsystem"), &i2c)
  {
    return None;
  }

  let eeprom = fs::canonicalize(client.join("eeprom")).ok()?;
  let eeprom_metadata = fs::metadata(&eeprom).ok()?;
  if eeprom.parent()? != client || !eeprom_metadata.is_file() {
    return None;
  }

  let hwmon = sole_hwmon_child(client)?;
  Some(Endpoint {
    client: client.to_owned(),
    identity: [
      file_identity(&fs::metadata(client).ok()?),
      file_identity(&fs::metadata(&hwmon).ok()?),
      file_identity(&fs::symlink_metadata(client.join("driver")).ok()?),
      file_identity(&eeprom_metadata),
    ],
    hwmon,
    eeprom,
  })
}

/// Exactly one hwmon child named `spd5118` under the client, which is a real
/// child directory whose `device` link resolves to that same client.
fn sole_hwmon_child(client: &Path) -> Option<PathBuf> {
  let directory = client.join("hwmon");
  let mut found = None;
  for entry in fs::read_dir(&directory).ok()? {
    let path = entry.ok()?.path();
    if attribute(&path, "name").as_deref() == Some(DRIVER)
      && found.replace(path).is_some()
    {
      return None;
    }
  }
  let hwmon = found?;
  (fs::canonicalize(&hwmon).ok()? == hwmon && links_to(&hwmon.join("device"), client))
    .then_some(hwmon)
}

fn read_temperature(
  sys: &Path,
  endpoint: &Endpoint,
  expected: &ExpectedIdentity<'_>,
) -> Option<f32> {
  let unchanged = || find_endpoint(sys).ok().as_ref() == Some(endpoint);

  let eeprom = File::open(&endpoint.eeprom).ok()?;
  let mut identity = [0; IDENTITY_LEN];
  if file_identity(&eeprom.metadata().ok()?) != endpoint.identity[3]
    || !unchanged()
    || eeprom.read_at(&mut identity, IDENTITY_OFFSET).ok()? != IDENTITY_LEN
    || !expected.confirms(&identity)
    || !unchanged()
  {
    return None;
  }

  if read_hwmon_attribute(&endpoint.hwmon, "temp1_enable")? != "1" {
    return None;
  }
  let temperature =
    decode_temperature(&read_hwmon_attribute(&endpoint.hwmon, "temp1_input")?)?;
  // Discard the confirmation if the client or hwmon child changed meanwhile.
  unchanged().then_some(temperature)
}

#[cfg(test)]
mod tests {
  use super::super::test_fixture::*;
  use super::*;
  use crate::infrastructure::providers::linux::dmidecode::parse_dmidecode_memory_info;
  use std::os::unix::fs::symlink;

  const SPD_SERIAL: usize = 517;
  const SPD_PART: usize = 521;

  fn with_serial(serial: &str) -> String {
    INVENTORY.replace(
      "Serial Number: 01234567",
      &format!("Serial Number: {serial}"),
    )
  }

  fn with_part(part: &str) -> String {
    INVENTORY.replace(
      "\tPart Number: M425R1GB4BB0-CWMOD\n",
      &format!("\tPart Number: {part}\n"),
    )
  }

  /// A system outside both board profiles with one bound hub at `0x52`.
  fn sole_client() -> (Fixture, PathBuf) {
    let fixture = Fixture::generic();
    let client = fixture.add_client(7, "0052", 3);
    (fixture, client)
  }

  fn swap_link(link: &Path, target: &Path) {
    fs::remove_file(link).unwrap();
    symlink(target, link).unwrap();
  }

  // Spec "Synthetic fixture acceptance", first row: one populated DDR5
  // record, sole client at 0x52, SPD serial 01 23 45 67, Type 17 serial
  // 01234567, equal part, enable 1, input 40250.
  #[test]
  fn confirms_the_sole_module_and_attributes_it_to_its_type17_record() {
    let (fixture, _) = sole_client();
    assert_eq!(fixture.collect_sole(INVENTORY), available(40.25));
  }

  // Spec "Read procedure and decode" step 6: Device Locator, adding Bank
  // Locator when present.
  #[test]
  fn labels_the_reading_with_the_device_and_present_bank_locators() {
    let (fixture, _) = sole_client();
    let with_bank = |bank: &str| {
      INVENTORY.replace(
        "\tType: DDR5\n",
        &format!("\tBank Locator: {bank}\n\tType: DDR5\n"),
      )
    };
    assert_eq!(
      fixture.collect_sole(&with_bank("BANK 0")),
      DimmTemperature::Available {
        locator: "DIMM 0 / BANK 0".into(),
        temperature_celsius: 40.25,
      }
    );
    for absent in ["Not Specified", "Unknown", " BANK 0"] {
      assert_eq!(fixture.collect_sole(&with_bank(absent)), available(40.25));
    }
    for unusable in ["Locator: Not Specified", "Locator: ", "Locator:  DIMM 0"] {
      let records = INVENTORY.replace("Locator: DIMM 0", unusable);
      assert_eq!(fixture.collect_sole(&records), DimmTemperature::Unavailable);
    }
  }

  // Spec "Synthetic fixture acceptance": SPD serial 0A 1B 2C 3D with Type 17
  // serial 0a1b2c3d or 0A1B2C3D is confirmed.
  #[test]
  fn accepts_either_hexadecimal_letter_case() {
    let (fixture, client) = sole_client();
    write_spd(&client, SPD_SERIAL, &[0x0a, 0x1b, 0x2c, 0x3d]);
    for serial in ["0a1b2c3d", "0A1B2C3D"] {
      assert_eq!(fixture.collect_sole(&with_serial(serial)), available(40.25));
    }
  }

  // Spec "Synthetic fixture acceptance": SPD bytes 512–520
  // 80 CE 01 23 45 01 23 45 67 with Type 17 serial 80CE01234501234567.
  #[test]
  fn accepts_the_eighteen_digit_forward_form() {
    let (fixture, client) = sole_client();
    write_spd(
      &client,
      512,
      &[0x80, 0xce, 0x01, 0x23, 0x45, 0x01, 0x23, 0x45, 0x67],
    );
    for serial in ["80CE01234501234567", "80ce01234501234567"] {
      assert_eq!(fixture.collect_sole(&with_serial(serial)), available(40.25));
    }
    // Each of the nine bytes takes part in the comparison.
    for serial in [
      "81CE01234501234567",
      "80CE01234401234567",
      "0001234501234567",
    ] {
      assert_eq!(
        fixture.collect_sole(&with_serial(serial)),
        DimmTemperature::Unavailable
      );
    }
  }

  // Spec "Synthetic fixture acceptance": Type 17 serial 67452301 against SPD
  // 01 23 45 67 is unavailable, with no reversed match; and a sole client of
  // an unreported module with a different serial is unavailable.
  #[test]
  fn rejects_reversed_or_foreign_serials() {
    let (fixture, client) = sole_client();
    assert_eq!(
      fixture.collect_sole(&with_serial("67452301")),
      DimmTemperature::Unavailable
    );
    write_spd(&client, SPD_SERIAL, &[0x89, 0xab, 0xcd, 0xef]);
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unavailable
    );
  }

  // Spec "Synthetic fixture acceptance": SPD serial all 00 or all FF, or a
  // Type 17 serial that is blank, 7 or 10 digits, or non-hexadecimal.
  #[test]
  fn rejects_placeholder_and_malformed_serials() {
    let (fixture, client) = sole_client();
    for serial in [
      "",
      "0123456",
      "0123456789",
      "0123456G",
      "0x012345",
      " 01234567",
      "0123-4567",
      "Not Specified",
      "Unknown",
    ] {
      assert_eq!(
        fixture.collect_sole(&with_serial(serial)),
        DimmTemperature::Unavailable,
        "{serial:?}"
      );
    }
    assert_eq!(
      fixture.collect_sole(&INVENTORY.replace("\tSerial Number: 01234567\n", "")),
      DimmTemperature::Unavailable
    );
    for (bytes, serial) in [([0x00; 4], "00000000"), ([0xff; 4], "FFFFFFFF")] {
      write_spd(&client, SPD_SERIAL, &bytes);
      assert_eq!(
        fixture.collect_sole(&with_serial(serial)),
        DimmTemperature::Unavailable
      );
      assert_eq!(
        fixture.collect_sole(&with_serial(&format!("0000000000{serial}"))),
        DimmTemperature::Unavailable
      );
    }
  }

  // Spec "Synthetic fixture acceptance": the serial matches but the Type 17
  // part differs; and a Type 17 part of `Not Specified` lets the serial alone
  // confirm.
  #[test]
  fn compares_a_usable_part_exactly_and_skips_an_absent_one() {
    let (fixture, client) = sole_client();
    for part in [
      "M425R1GB4BB0-CWMO",
      "m425r1gb4bb0-cwmod",
      "M425R1GB4BB0-CWMOD ",
      "M425R1GB4BB0-CWMODX",
      "None",
    ] {
      assert_eq!(
        fixture.collect_sole(&with_part(part)),
        DimmTemperature::Unavailable,
        "{part:?}"
      );
    }
    for part in ["Not Specified", "Unknown", "", "    "] {
      assert_eq!(
        fixture.collect_sole(&with_part(part)),
        available(40.25),
        "{part:?}"
      );
    }
    assert_eq!(
      fixture.collect_sole(&INVENTORY.replace("\tPart Number: M425R1GB4BB0-CWMOD\n", "")),
      available(40.25)
    );

    // Only trailing ASCII spaces are removed from the SPD part.
    write_spd(&client, SPD_PART + 18, b"\0");
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unavailable
    );
    assert_eq!(
      fixture.collect_sole(&with_part("Not Specified")),
      available(40.25)
    );
  }

  // Spec "Synthetic fixture acceptance": two populated Type 17 records, or a
  // populated record with Size FFFFh; LPDDR5 or DDR4; duplicate handle,
  // unresolved array handle or declared count mismatch.
  #[test]
  fn requires_exactly_one_populated_ddr5_record_in_a_consistent_inventory() {
    let (fixture, _) = sole_client();
    let (array, device) = INVENTORY.split_once("\n\n").unwrap();
    let two_slots = array.replace("Number Of Devices: 1", "Number Of Devices: 2");
    let second = device.replace("Handle 0x0010", "Handle 0x0011");
    let empty = "Handle 0x0011, DMI type 17, 92 bytes\nMemory Device\n\tArray Handle: 0x000F\n\tSize: No Module Installed\n";

    assert_eq!(
      fixture.collect_sole(&format!("{two_slots}\n\n{device}\n{empty}")),
      available(40.25)
    );
    assert_eq!(
      fixture.collect_sole(&format!("{two_slots}\n\n{device}\n{second}")),
      DimmTemperature::Unsupported
    );
    for memory_type in ["LPDDR5", "DDR4"] {
      assert_eq!(
        fixture.collect_sole(
          &INVENTORY.replace("Type: DDR5", &format!("Type: {memory_type}"))
        ),
        DimmTemperature::Unsupported
      );
    }
    assert_eq!(
      fixture.collect_sole(&INVENTORY.replace("Size: 8 GB", "Size: No Module Installed")),
      DimmTemperature::Unsupported
    );
    for unavailable in [
      INVENTORY.replace("Size: 8 GB", "Size: Unknown"),
      format!(
        "{two_slots}\n\n{device}\n{}",
        empty.replace("No Module Installed", "Unknown")
      ),
      INVENTORY.replace("Type: DDR5", "Type: Unknown"),
      INVENTORY.replace("Handle 0x0010", "Handle 0x000F"),
      INVENTORY.replace("Array Handle: 0x000F", "Array Handle: 0x002F"),
      INVENTORY.replace("Number Of Devices: 1", "Number Of Devices: 2"),
      format!("{two_slots}\n\n{device}"),
      format!("{INVENTORY}\n{empty}"),
    ] {
      assert_eq!(
        fixture.collect_sole(&unavailable),
        DimmTemperature::Unavailable,
        "{unavailable}"
      );
    }
  }

  // Spec "Synthetic fixture acceptance": missing Type 127, header-only
  // marker, diagnostics or stderr leave the temperature unavailable and the
  // ordinary inventory preserved.
  #[test]
  fn requires_the_inventory_completion_contract() {
    let (fixture, _) = sole_client();
    let complete = format!("{INVENTORY}\n{END_OF_TABLE}");
    assert_eq!(
      collect_from(&fixture.sys, &complete, true),
      available(40.25)
    );
    let header_only = format!("{INVENTORY}\n{}\n", END_OF_TABLE.lines().next().unwrap());
    let diagnostic = format!("{INVENTORY}\t<TRUNCATED>\n\n{END_OF_TABLE}");
    for (raw, stderr_empty) in [
      (INVENTORY.to_owned(), true),
      (header_only, true),
      (diagnostic, true),
      (complete.clone(), false),
    ] {
      assert_eq!(
        collect_from(&fixture.sys, &raw, stderr_empty),
        DimmTemperature::Unavailable,
        "{raw:?}, stderr_empty={stderr_empty}"
      );
      assert_eq!(parse_dmidecode_memory_info(&raw).memory_count, 1);
    }
  }

  // Spec "Synthetic fixture acceptance": two `spd5118` clients, one of them
  // unbound. Canonical aliases of one client count once.
  #[test]
  fn requires_exactly_one_spd5118_client_bound_or_not() {
    let (fixture, client) = sole_client();
    let alias = fixture.sys.join("bus/i2c/devices/alias");
    symlink(&client, &alias).unwrap();
    assert_eq!(fixture.collect_sole(INVENTORY), available(40.25));

    let unbound = fixture.add_unbound_client(8, "0053");
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unsupported
    );

    // A device bound to the driver counts even under another name.
    write_attr(&unbound, "name", "other");
    assert_eq!(fixture.collect_sole(INVENTORY), available(40.25));
    symlink(
      fixture.sys.join("bus/i2c/drivers/spd5118"),
      unbound.join("driver"),
    )
    .unwrap();
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unsupported
    );
  }

  #[test]
  fn an_unbound_sole_client_or_no_client_is_out_of_scope() {
    let fixture = Fixture::generic();
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unsupported
    );
    fixture.add_unbound_client(7, "0050");
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unsupported
    );
  }

  // Spec "Synthetic fixture acceptance": client address 0x48 with name
  // `spd5118`.
  #[test]
  fn requires_the_hub_device_type_in_the_client_address() {
    let fixture = Fixture::generic();
    fixture.add_client(7, "0048", 3);
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unavailable
    );

    for (name, expected) in [
      ("7-0050", Some(("7", 0x50))),
      ("7-0057", Some(("7", 0x57))),
      ("12-0053", Some(("12", 0x53))),
      ("7-0048", None),
      ("7-0058", None),
      ("7-00d0", None),
      ("7-a050", None),
      ("7-050", None),
      ("-0050", None),
      ("x-0050", None),
      ("i2c-7", None),
    ] {
      assert_eq!(hub_address(name), expected, "{name}");
    }
  }

  // Spec "Synthetic fixture acceptance": adapter or client `subsystem` link
  // resolving to a class, foreign `eeprom` parent, or hwmon child of another
  // client; and the `driver` link.
  #[test]
  fn requires_kernel_ownership_links() {
    let (fixture, client) = sole_client();
    let adapter = client.parent().unwrap().to_owned();
    let i2c = fixture.sys.join("bus/i2c");
    for class in ["class/i2c-adapter", "class/i2c-dev"] {
      let class = fixture.sys.join(class);
      fs::create_dir_all(&class).unwrap();
      for node in [&adapter, &client] {
        swap_link(&node.join("subsystem"), &class);
        assert_eq!(
          fixture.collect_sole(INVENTORY),
          DimmTemperature::Unavailable
        );
        swap_link(&node.join("subsystem"), &i2c);
      }
    }
    assert_eq!(fixture.collect_sole(INVENTORY), available(40.25));

    let other_driver = i2c.join("drivers/other");
    fs::create_dir_all(&other_driver).unwrap();
    swap_link(&client.join("driver"), &other_driver);
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unavailable
    );
    swap_link(&client.join("driver"), &i2c.join("drivers/spd5118"));

    let eeprom = client.join("eeprom");
    let foreign = fixture.sys.join("foreign-eeprom");
    fs::rename(&eeprom, &foreign).unwrap();
    symlink(&foreign, &eeprom).unwrap();
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unavailable
    );
    fs::remove_file(&eeprom).unwrap();
    fs::rename(&foreign, &eeprom).unwrap();
    assert_eq!(fixture.collect_sole(INVENTORY), available(40.25));

    // The hwmon child must belong to this client, and be its only one.
    let hwmon = client.join("hwmon/hwmon3");
    let other_client = fixture.controller.join("other-client");
    fs::create_dir_all(&other_client).unwrap();
    swap_link(&hwmon.join("device"), &other_client);
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unavailable
    );
    swap_link(&hwmon.join("device"), &client);
    fixture.add_hwmon(&client, 4);
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unavailable
    );
    write_attr(&client.join("hwmon/hwmon4"), "name", "other");
    assert_eq!(fixture.collect_sole(INVENTORY), available(40.25));
  }

  // Spec "Synthetic fixture acceptance": `temp1_enable` 0, the `temp1_input`
  // boundaries, and a short EEPROM read.
  #[test]
  fn requires_an_enabled_sensor_a_valid_input_and_a_full_identity_read() {
    let (fixture, client) = sole_client();
    let hwmon = client.join("hwmon/hwmon3");
    write_attr(&hwmon, "temp1_enable", "0");
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unavailable
    );
    write_attr(&hwmon, "temp1_enable", "1");

    for (value, expected) in [
      ("0", DimmTemperature::Unavailable),
      ("250", available(0.25)),
      ("125000", available(125.0)),
      ("125250", DimmTemperature::Unavailable),
      ("40251", DimmTemperature::Unavailable),
    ] {
      write_attr(&hwmon, "temp1_input", value);
      assert_eq!(fixture.collect_sole(INVENTORY), expected, "{value}");
    }
    write_attr(&hwmon, "temp1_input", "40250");

    let eeprom = client.join("eeprom");
    let spd = fs::read(&eeprom).unwrap();
    fs::write(&eeprom, &spd[..550]).unwrap();
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Unavailable
    );
    fs::write(&eeprom, &spd[..551]).unwrap();
    assert_eq!(fixture.collect_sole(INVENTORY), available(40.25));
  }

  // Spec "Safety notes": requalify a recreated or renumbered node.
  #[test]
  fn a_recreated_hwmon_node_requires_a_fresh_qualification() {
    let (fixture, client) = sole_client();
    let first = find_endpoint(&fixture.sys).unwrap();
    fs::rename(&first.hwmon, fixture.sys.join("detached-hwmon")).unwrap();
    fs::remove_file(fixture.sys.join("class/hwmon/hwmon3")).unwrap();
    let replacement = fixture.add_hwmon(&client, 5);
    write_attr(&replacement, "temp1_input", "41000");

    let expected = ExpectedIdentity {
      serial: Serial::Short(SERIAL),
      part: None,
    };
    assert_eq!(read_temperature(&fixture.sys, &first, &expected), None);
    assert_eq!(
      fixture.collect_sole(INVENTORY),
      DimmTemperature::Available {
        locator: "DIMM 0".into(),
        temperature_celsius: 41.0,
      }
    );
  }

  // Spec "SPD identity confirmation", as a pure decoder over the 39 bytes.
  #[test]
  fn confirmation_compares_forward_serial_bytes_and_the_trimmed_part() {
    let mut identity = [b' '; IDENTITY_LEN];
    identity[..9]
      .copy_from_slice(&[0x80, 0xce, 0x01, 0x23, 0x45, 0x01, 0x23, 0x45, 0x67]);
    identity[PART_BYTES][..6].copy_from_slice(b"PART-1");
    let expect = |serial: &str, part: Option<&'static str>| ExpectedIdentity {
      serial: decode_serial(serial).unwrap(),
      part,
    };
    assert!(expect("01234567", Some("PART-1")).confirms(&identity));
    assert!(expect("01234567", None).confirms(&identity));
    assert!(expect("80ce01234501234567", Some("PART-1")).confirms(&identity));
    assert!(!expect("67452301", None).confirms(&identity));
    assert!(!expect("01234567", Some("PART-")).confirms(&identity));
    assert!(!expect("01234567", Some("PART-1 ")).confirms(&identity));

    assert_eq!(usable_part(Some("PART\u{7f}")), None);
    assert_eq!(usable_part(Some("Ünknown")), None);
    assert_eq!(usable_part(Some(" PART")), Some(Some(" PART")));
    assert_eq!(
      decode_serial("0123456789abcdef01"),
      Some(Serial::Long([
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01
      ]))
    );
  }
}
