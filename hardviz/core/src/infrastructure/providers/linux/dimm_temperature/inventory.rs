//! Inventory completion contract of the fixed
//! `pkexec dmidecode --type memory --type 127` text output, shared by the
//! Linux DIMM routes (`memory-ddr5-odroid-h4-spd5118.md` rev 3, "Inventory
//! completion").

use std::collections::{HashMap, HashSet};

/// Raw Type 17 fields are used only during this request and never cached.
#[derive(Default)]
pub(super) struct Dimm<'a> {
  pub(super) array_handle: Option<&'a str>,
  pub(super) size: Option<&'a str>,
  pub(super) form_factor: Option<&'a str>,
  pub(super) memory_type: Option<&'a str>,
  pub(super) locator: Option<&'a str>,
  pub(super) bank_locator: Option<&'a str>,
  pub(super) serial: Option<&'a str>,
  pub(super) part: Option<&'a str>,
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
      "Bank Locator" => &mut self.bank_locator,
      "Serial Number" => &mut self.serial,
      "Part Number" => &mut self.part,
      _ => return Some(()),
    };
    if field.replace(value.strip_prefix(' ')?).is_some() {
      return None;
    }
    Some(())
  }

  pub(super) fn locator(&self) -> Option<&'a str> {
    self.locator.filter(|locator| is_label_text(locator))
  }
}

/// A firmware string fit for a label: no surrounding whitespace, control
/// characters or decoder replacement characters.
pub(super) fn is_label_text(value: &str) -> bool {
  value.trim() == value && !value.chars().any(|c| c.is_control() || c == '\u{fffd}')
}

pub(super) fn is_reported(value: &str) -> bool {
  let value = value.trim();
  !value.is_empty()
    && !matches!(
      value,
      "Unknown" | "Not Specified" | "Not Provided" | "None" | "Bad Index"
    )
}

enum Record<'a> {
  Array { handle: u16, slots: Option<u16> },
  Device(Dimm<'a>),
}

impl<'a> Record<'a> {
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

/// Type 16 and Type 17 records of one complete, diagnostic-free decoder
/// output that ends at its terminal Type 127 record.
#[derive(Default)]
pub(super) struct Inventory<'a> {
  /// Declared device count of every Type 16 array, keyed by its handle.
  array_slots: HashMap<u16, usize>,
  pub(super) devices: Vec<Dimm<'a>>,
}

impl<'a> Inventory<'a> {
  fn finish(&mut self, record: Option<Record<'a>>) -> Option<()> {
    match record {
      Some(Record::Array { handle, slots }) => {
        self.array_slots.insert(handle, usize::from(slots?));
      }
      Some(Record::Device(device)) => self.devices.push(device),
      None => {}
    }
    Some(())
  }

  /// Every Type 17 names a reported Type 16, and every array declares exactly
  /// as many devices as reference it. Empty sockets also occupy a Type 16
  /// slot, so every Type 17 record counts against the array it references.
  pub(super) fn array_counts_match(&self) -> bool {
    let mut device_counts = HashMap::new();
    for device in &self.devices {
      let Some(array) = device.array_handle.and_then(parse_handle) else {
        return false;
      };
      *device_counts.entry(array).or_default() += 1;
    }
    device_counts == self.array_slots
  }

  pub(super) fn single_array_with_one_slot(&self) -> bool {
    self.array_slots.len() == 1 && self.array_slots.values().next() == Some(&1)
  }
}

fn parse_handle(value: &str) -> Option<u16> {
  let digits = value.strip_prefix("0x")?;
  if digits.len() != 4 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
    return None;
  }
  u16::from_str_radix(digits, 16).ok()
}

/// Parses the decoder text output. Returns `None` unless the output passes the
/// completion contract: ordinary notices only before the first record, unique
/// handles, well-formed Type 5/6/16/17 records without diagnostics, and exactly
/// one terminal decoded Type 127 followed only by whitespace.
pub(super) fn parse(raw: &str) -> Option<Inventory<'_>> {
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
        {
          return None;
        }
        return Some(inventory);
      }

      let length = length.strip_suffix(" bytes")?.parse::<u8>().ok()?;
      current = match (kind, heading) {
        ("5", "Memory Controller Information") | ("6", "Memory Module Information")
          if length >= 4 =>
        {
          None
        }
        ("16", "Physical Memory Array") if length >= 0x0f => Some(Record::Array {
          handle,
          slots: None,
        }),
        ("17", "Memory Device") if length >= 0x1b => {
          Some(Record::Device(Dimm::default()))
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
