use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use super::pawn_io::{ACCESS_ISABUS_MUTEX, NamedMutex, PawnIoClient, PawnIoModule};
use crate::models::{
  MotherboardFanSpeed, MotherboardSensorSample, MotherboardTemperature,
};
use crate::utils::super_io::{
  chip_id, decode_nuvoton_direct_rpm, decode_nuvoton_temperature_byte,
  fan_speed_status_from_rpm, is_absent_id, slot_ports,
};
use crate::{log_debug, log_warn};

const ISABUS_MUTEX_TIMEOUT: Duration = Duration::from_millis(50);
const NUVOTON_NCT6799D_CHIP_ID: u16 = 0xD802;
const NUVOTON_CHIP_LABEL: &str = "NCT6799D";
const NUVOTON_SOURCE_LABEL: &str = "NCT6799D / Super I/O";

const CHIP_ID_HIGH_REGISTER: u8 = 0x20;
const CHIP_ID_LOW_REGISTER: u8 = 0x21;
const LOGICAL_DEVICE_SELECT_REGISTER: u8 = 0x07;
const NUVOTON_HARDWARE_MONITOR_LDN: u8 = 0x0B;
const LDN_ACTIVATION_REGISTER: u8 = 0x30;
const HM_BASE_HIGH_REGISTER: u8 = 0x60;
const HM_BASE_LOW_REGISTER: u8 = 0x61;

const HM_INDEX_OFFSET: u16 = 0x05;
const HM_DATA_OFFSET: u16 = 0x06;
const HM_BANK_SELECT_REGISTER: u8 = 0x4E;
const NUVOTON_BANK_4: u8 = 0x04;

const TEMPERATURE_REGISTERS: [(&str, u8); 6] = [
  ("SYSTIN", 0x90),
  ("CPUTIN", 0x91),
  ("AUXTIN0", 0x92),
  ("AUXTIN1", 0x93),
  ("AUXTIN2", 0x94),
  ("AUXTIN3", 0x95),
];

const FAN_RPM_REGISTERS: [(&str, u8, u8); 6] = [
  ("Fan 1", 0xC0, 0xC1),
  ("Fan 2", 0xC2, 0xC3),
  ("Fan 3", 0xC4, 0xC5),
  ("Fan 4", 0xC6, 0xC7),
  ("Fan 5", 0xC8, 0xC9),
  ("Fan 6", 0xCA, 0xCB),
];

static MOTHERBOARD_SENSOR_SAMPLER: OnceLock<Mutex<MotherboardSensorSampler>> =
  OnceLock::new();

/// Read live motherboard temperature and fan RPM values through the scoped
/// Nuvoton normal-HM path.
///
/// Implemented from:
/// - docs/specs/sensors/pawnio-interface.md revision 4
/// - docs/specs/sensors/superio-access.md revision 3
/// - docs/specs/sensors/superio-nuvoton-nct67xx.md revision 5
///
/// No other external sensor implementation source was used.
pub fn sample_motherboard_sensors() -> Result<MotherboardSensorSample, String> {
  let sampler = MOTHERBOARD_SENSOR_SAMPLER.get_or_init(|| {
    let sampler = MotherboardSensorSampler::new();
    log_debug!(
      "super_io_motherboard_sensor_sampler_initialized",
      "windows::super_io_motherboard::sample_motherboard_sensors",
      Some(sampler.diagnostic_summary())
    );
    Mutex::new(sampler)
  });

  sampler
    .lock()
    .map_err(|_| "Motherboard sensor sampler lock poisoned".to_string())?
    .sample()
}

struct MotherboardSensorSampler {
  active: Option<ActiveNuvotonMotherboardSensors>,
  unavailable_reason: Option<String>,
  sample_failure_logged: bool,
}

impl MotherboardSensorSampler {
  fn new() -> Self {
    match ActiveNuvotonMotherboardSensors::open() {
      Ok(active) => Self {
        active: Some(active),
        unavailable_reason: None,
        sample_failure_logged: false,
      },
      Err(reason) => Self {
        active: None,
        unavailable_reason: Some(reason),
        sample_failure_logged: false,
      },
    }
  }

  fn sample(&mut self) -> Result<MotherboardSensorSample, String> {
    let result = match self.active.as_ref() {
      Some(active) => active.sample(),
      None => Err(
        self
          .unavailable_reason
          .clone()
          .unwrap_or_else(|| "Motherboard sensors unavailable".to_string()),
      ),
    };

    if let Err(reason) = &result
      && !self.sample_failure_logged
    {
      self.sample_failure_logged = true;
      log_warn!(
        "super_io_motherboard_sensor_sample_failed",
        "windows::super_io_motherboard::MotherboardSensorSampler::sample",
        Some(reason.clone())
      );
    }

    result
  }

  fn diagnostic_summary(&self) -> String {
    match &self.active {
      Some(active) => format!(
        "active chip={} slot={} hm_base=0x{:04X}",
        active.chip_label, active.slot, active.hm_base
      ),
      None => format!(
        "unavailable reason={}",
        self.unavailable_reason.as_deref().unwrap_or("unknown")
      ),
    }
  }
}

struct ActiveNuvotonMotherboardSensors {
  client: PawnIoClient,
  slot: u8,
  hm_base: u16,
  chip_label: &'static str,
  source_label: &'static str,
}

impl ActiveNuvotonMotherboardSensors {
  fn open() -> Result<Self, String> {
    let (client, _discovery) =
      PawnIoClient::open(PawnIoModule::LpcIo).map_err(|error| error.reason)?;

    let _mutex = NamedMutex::acquire(ACCESS_ISABUS_MUTEX, ISABUS_MUTEX_TIMEOUT)?;
    let mut last_error: Option<String> = None;

    for slot in [0_u8, 1] {
      match Self::discover_slot(&client, slot) {
        Ok(Some(discovered)) => {
          return Ok(Self {
            client,
            slot: discovered.slot,
            hm_base: discovered.hm_base,
            chip_label: NUVOTON_CHIP_LABEL,
            source_label: NUVOTON_SOURCE_LABEL,
          });
        }
        Ok(None) => {}
        Err(reason) => {
          last_error = Some(reason);
        }
      }
    }

    Err(last_error.unwrap_or_else(|| {
      "No supported Nuvoton NCT6799D Super I/O hardware-monitor path found".to_string()
    }))
  }

  fn discover_slot(
    client: &PawnIoClient,
    slot: u8,
  ) -> Result<Option<DetectedSlot>, String> {
    let Some((index_port, _data_port)) = slot_ports(slot) else {
      return Ok(None);
    };

    client.select_lpc_slot(slot as u64)?;
    enter_nuvoton(client, index_port)?;
    let result = discover_nuvoton_hm_base(client, slot);
    let exit_result = exit_nuvoton(client, index_port);

    match (result, exit_result) {
      (Ok(value), Ok(())) => Ok(value),
      (Ok(_), Err(exit_error)) => {
        Err(format!("Nuvoton config exit failed: {exit_error}"))
      }
      (Err(reason), Ok(())) => Err(reason),
      (Err(reason), Err(exit_error)) => {
        Err(format!("{reason}; exit failed: {exit_error}"))
      }
    }
  }

  fn sample(&self) -> Result<MotherboardSensorSample, String> {
    let _mutex = NamedMutex::acquire(ACCESS_ISABUS_MUTEX, ISABUS_MUTEX_TIMEOUT)?;
    self.client.select_lpc_slot(self.slot as u64)?;
    self.write_hm_byte(HM_BANK_SELECT_REGISTER, NUVOTON_BANK_4)?;

    let temperatures = TEMPERATURE_REGISTERS
      .iter()
      .map(|(name, register)| {
        self
          .read_hm_byte(*register)
          .map(|raw| MotherboardTemperature {
            name: (*name).to_string(),
            temperature: decode_nuvoton_temperature_byte(raw),
            source: self.source_label.to_string(),
          })
      })
      .collect::<Result<Vec<_>, _>>()?;

    let fan_speeds = FAN_RPM_REGISTERS
      .iter()
      .map(|(name, high_register, low_register)| {
        let high = self.read_hm_byte(*high_register)?;
        let low = self.read_hm_byte(*low_register)?;
        let rpm = decode_nuvoton_direct_rpm(high, low);
        Ok(MotherboardFanSpeed {
          name: (*name).to_string(),
          rpm: Some(rpm),
          status: fan_speed_status_from_rpm(rpm),
          source: self.source_label.to_string(),
        })
      })
      .collect::<Result<Vec<_>, String>>()?;

    Ok(MotherboardSensorSample {
      temperatures,
      fan_speeds,
    })
  }

  fn write_hm_byte(&self, register: u8, value: u8) -> Result<(), String> {
    let index_port = self.hm_base + HM_INDEX_OFFSET;
    let data_port = self.hm_base + HM_DATA_OFFSET;
    self.client.pio_outb(index_port, register)?;
    self.client.pio_outb(data_port, value)
  }

  fn read_hm_byte(&self, register: u8) -> Result<u8, String> {
    let index_port = self.hm_base + HM_INDEX_OFFSET;
    let data_port = self.hm_base + HM_DATA_OFFSET;
    self.client.pio_outb(index_port, register)?;
    self.client.pio_inb(data_port)
  }
}

struct DetectedSlot {
  slot: u8,
  hm_base: u16,
}

fn discover_nuvoton_hm_base(
  client: &PawnIoClient,
  slot: u8,
) -> Result<Option<DetectedSlot>, String> {
  let id_high = client.superio_inb(CHIP_ID_HIGH_REGISTER)?;
  let id_low = client.superio_inb(CHIP_ID_LOW_REGISTER)?;
  if is_absent_id(id_high, id_low) {
    return Ok(None);
  }

  let raw_chip_id = chip_id(id_high, id_low);
  if raw_chip_id != NUVOTON_NCT6799D_CHIP_ID {
    return Ok(None);
  }

  client.superio_outb(LOGICAL_DEVICE_SELECT_REGISTER, NUVOTON_HARDWARE_MONITOR_LDN)?;
  let activation = client.superio_inb(LDN_ACTIVATION_REGISTER)?;
  if (activation & 0x01) == 0 {
    return Err("Nuvoton hardware monitor logical device is inactive".to_string());
  }

  let base_high = client.superio_inb(HM_BASE_HIGH_REGISTER)?;
  let base_low = client.superio_inb(HM_BASE_LOW_REGISTER)?;
  let hm_base = ((base_high as u16) << 8) | base_low as u16;
  if !is_valid_hm_base(hm_base) {
    return Err(format!(
      "invalid Nuvoton hardware-monitor base 0x{hm_base:04X}"
    ));
  }

  client.find_lpc_bars()?;

  Ok(Some(DetectedSlot { slot, hm_base }))
}

fn enter_nuvoton(client: &PawnIoClient, index_port: u16) -> Result<(), String> {
  client.pio_outb(index_port, 0x87)?;
  client.pio_outb(index_port, 0x87)
}

fn exit_nuvoton(client: &PawnIoClient, index_port: u16) -> Result<(), String> {
  client.pio_outb(index_port, 0xAA)
}

fn is_valid_hm_base(base: u16) -> bool {
  base != 0x0000 && base != 0xFFFF && base <= u16::MAX - HM_DATA_OFFSET
}
