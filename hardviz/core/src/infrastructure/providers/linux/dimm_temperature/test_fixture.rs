//! Synthetic kernel-owned-style sysfs topology shared by the DIMM route tests.
//! These are not dumps from a real system.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::board_profile::{
  BIOS_VERSION, BOARD_NAME, BOARD_VENDOR, CONTROLLER_ADDRESS, DIMM_PART,
  SPD_SERIAL_OFFSET,
};
use crate::models::hardware::DimmTemperature;

pub(super) const SERIAL: [u8; 4] = [0x01, 0x23, 0x45, 0x67];
pub(super) const END_OF_TABLE: &str =
  "Handle 0x007F, DMI type 127, 4 bytes\nEnd Of Table\n";

pub(super) const INVENTORY: &str = r#"Handle 0x000F, DMI type 16, 23 bytes
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

pub(super) struct Fixture {
  _directory: TempDir,
  pub(super) sys: PathBuf,
  pub(super) controller: PathBuf,
}

impl Fixture {
  pub(super) fn new() -> Self {
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

  pub(super) fn add_client(&self, bus: u32, address: &str, hwmon_index: u32) -> PathBuf {
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

  pub(super) fn add_hwmon(&self, client: &Path, index: u32) -> PathBuf {
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

  /// Runs the board-profile route alone on `records` plus the terminal record.
  pub(super) fn collect(&self, records: &str) -> DimmTemperature {
    super::board_profile::collect_from(
      &self.sys,
      &format!("{records}\n{END_OF_TABLE}"),
      true,
    )
  }

  pub(super) fn protectli() -> Self {
    let fixture = Self::new();
    let dmi = fixture.sys.join("class/dmi/id");
    write_attr(&dmi, "board_vendor", "Protectli");
    write_attr(&dmi, "board_name", "VP2430");
    write_attr(&dmi, "bios_version", "Dasharo (coreboot+UEFI) v0.9.0");
    fixture
  }
}

pub(super) fn write_attr(directory: &Path, name: &str, value: &str) {
  fs::write(directory.join(name), format!("{value}\n")).unwrap();
}

pub(super) fn available(temperature_celsius: f32) -> DimmTemperature {
  DimmTemperature::Available {
    locator: "DIMM 0".into(),
    temperature_celsius,
  }
}
