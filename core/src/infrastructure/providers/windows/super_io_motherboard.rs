use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use super::pawn_io::{ACCESS_ISABUS_MUTEX, NamedMutex, PawnIoClient, PawnIoModule};
use crate::models::{
  MotherboardFanSpeed, MotherboardSensorSample, MotherboardTemperature,
  SensorVerification,
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
pub(crate) const UNSUPPORTED_NUVOTON_HM_PATH_REASON: &str =
  "No supported Nuvoton NCT6799D Super I/O hardware-monitor path found";

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
  active: Option<ActiveNuvotonMotherboardSensors<PawnIoClient>>,
  unavailable_reason: Option<String>,
  retry_unavailable: bool,
  sample_failure_logged: bool,
}

impl MotherboardSensorSampler {
  fn new() -> Self {
    let mut sampler = Self {
      active: None,
      unavailable_reason: None,
      retry_unavailable: false,
      sample_failure_logged: false,
    };
    let _ = sampler.try_open();
    sampler
  }

  fn sample(&mut self) -> Result<MotherboardSensorSample, String> {
    if self.active.is_none()
      && (self.unavailable_reason.is_none() || self.retry_unavailable)
    {
      let _ = self.try_open();
    }

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

  fn try_open(&mut self) -> Result<(), String> {
    match ActiveNuvotonMotherboardSensors::open() {
      Ok(active) => {
        self.active = Some(active);
        self.unavailable_reason = None;
        self.retry_unavailable = false;
        self.sample_failure_logged = false;
        Ok(())
      }
      Err(reason) => {
        self.active = None;
        self.retry_unavailable = is_retryable_init_error(&reason);
        self.unavailable_reason = Some(reason.clone());
        Err(reason)
      }
    }
  }

  fn diagnostic_summary(&self) -> String {
    match &self.active {
      Some(active) => format!(
        "active chip={} slot={} hm_base=0x{:04X}",
        active.chip_label, active.slot, active.hm_base
      ),
      None => format!(
        "unavailable retryable={} reason={}",
        self.retry_unavailable,
        self
          .unavailable_reason
          .as_deref()
          .unwrap_or("not attempted")
      ),
    }
  }
}

trait LpcIoOps {
  fn select_lpc_slot(&self, slot: u64) -> Result<(), String>;
  fn find_lpc_bars(&self) -> Result<(), String>;
  fn pio_inb(&self, port: u16) -> Result<u8, String>;
  fn pio_outb(&self, port: u16, value: u8) -> Result<(), String>;
  fn superio_inb(&self, register: u8) -> Result<u8, String>;
  fn superio_outb(&self, register: u8, value: u8) -> Result<(), String>;
}

impl LpcIoOps for PawnIoClient {
  fn select_lpc_slot(&self, slot: u64) -> Result<(), String> {
    PawnIoClient::select_lpc_slot(self, slot)
  }

  fn find_lpc_bars(&self) -> Result<(), String> {
    PawnIoClient::find_lpc_bars(self)
  }

  fn pio_inb(&self, port: u16) -> Result<u8, String> {
    PawnIoClient::pio_inb(self, port)
  }

  fn pio_outb(&self, port: u16, value: u8) -> Result<(), String> {
    PawnIoClient::pio_outb(self, port, value)
  }

  fn superio_inb(&self, register: u8) -> Result<u8, String> {
    PawnIoClient::superio_inb(self, register)
  }

  fn superio_outb(&self, register: u8, value: u8) -> Result<(), String> {
    PawnIoClient::superio_outb(self, register, value)
  }
}

struct ActiveNuvotonMotherboardSensors<C: LpcIoOps> {
  client: C,
  slot: u8,
  hm_base: u16,
  chip_label: &'static str,
  source_label: &'static str,
  verification: SensorVerification,
}

impl ActiveNuvotonMotherboardSensors<PawnIoClient> {
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
            verification: discovered.verification,
          });
        }
        Ok(None) => {}
        Err(reason) => {
          last_error = Some(reason);
        }
      }
    }

    Err(last_error.unwrap_or_else(|| UNSUPPORTED_NUVOTON_HM_PATH_REASON.to_string()))
  }

  fn discover_slot(
    client: &impl LpcIoOps,
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
}

impl<C: LpcIoOps> ActiveNuvotonMotherboardSensors<C> {
  fn sample(&self) -> Result<MotherboardSensorSample, String> {
    let _mutex = NamedMutex::acquire(ACCESS_ISABUS_MUTEX, ISABUS_MUTEX_TIMEOUT)?;
    self.sample_unlocked()
  }

  fn sample_unlocked(&self) -> Result<MotherboardSensorSample, String> {
    // `ioctl_find_bars` authorizes normal-HM ports for this loaded LpcIO
    // handle during discovery. Re-running `ioctl_select_slot` here can clear
    // that BAR authorization, which makes the subsequent HM index/data port
    // I/O fail with access denied on the validated NZXT N7 B650E path.
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
            verification: self.verification,
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
          verification: self.verification,
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
  verification: SensorVerification,
}

fn discover_nuvoton_hm_base(
  client: &impl LpcIoOps,
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

  Ok(Some(DetectedSlot {
    slot,
    hm_base,
    verification: SensorVerification::Verified,
  }))
}

fn enter_nuvoton(client: &impl LpcIoOps, index_port: u16) -> Result<(), String> {
  client.pio_outb(index_port, 0x87)?;
  client.pio_outb(index_port, 0x87)
}

fn exit_nuvoton(client: &impl LpcIoOps, index_port: u16) -> Result<(), String> {
  client.pio_outb(index_port, 0xAA)
}

fn is_valid_hm_base(base: u16) -> bool {
  base != 0x0000 && base != 0xFFFF && base <= u16::MAX - HM_DATA_OFFSET
}

fn is_retryable_init_error(reason: &str) -> bool {
  reason.contains("timed out waiting for mutex")
    || reason.contains("failed waiting for mutex")
}

#[cfg(test)]
mod tests {
  use std::cell::{Cell, RefCell};

  use super::*;

  struct FakeLpcIo {
    selected_slot_calls: Cell<u32>,
    find_bars_calls: Cell<u32>,
    hm_bars_authorized: Cell<bool>,
    selected_hm_register: Cell<Option<u8>>,
    selected_hm_bank: Cell<u8>,
    selected_ldn: Cell<Option<u8>>,
    hm_base_high_read: Cell<bool>,
    hm_base_low_read: Cell<bool>,
    read_registers: RefCell<Vec<u8>>,
  }

  impl FakeLpcIo {
    fn new() -> Self {
      Self {
        selected_slot_calls: Cell::new(0),
        find_bars_calls: Cell::new(0),
        hm_bars_authorized: Cell::new(false),
        selected_hm_register: Cell::new(None),
        selected_hm_bank: Cell::new(0),
        selected_ldn: Cell::new(None),
        hm_base_high_read: Cell::new(false),
        hm_base_low_read: Cell::new(false),
        read_registers: RefCell::new(Vec::new()),
      }
    }

    fn with_authorized_hm_bars() -> Self {
      Self {
        selected_slot_calls: Cell::new(0),
        find_bars_calls: Cell::new(0),
        hm_bars_authorized: Cell::new(true),
        selected_hm_register: Cell::new(None),
        selected_hm_bank: Cell::new(0),
        selected_ldn: Cell::new(Some(NUVOTON_HARDWARE_MONITOR_LDN)),
        hm_base_high_read: Cell::new(true),
        hm_base_low_read: Cell::new(true),
        read_registers: RefCell::new(Vec::new()),
      }
    }

    fn require_hm_bars(&self) -> Result<(), String> {
      if self.hm_bars_authorized.get() {
        Ok(())
      } else {
        Err("hm ports are not authorized".to_string())
      }
    }

    fn read_value_for(register: u8) -> u8 {
      match register {
        0x90..=0x95 => 32,
        0xC0 | 0xC2 | 0xC4 | 0xC6 | 0xC8 | 0xCA => 0x03,
        0xC1 | 0xC3 | 0xC5 | 0xC7 | 0xC9 | 0xCB => 0x20,
        _ => 0,
      }
    }
  }

  impl LpcIoOps for FakeLpcIo {
    fn select_lpc_slot(&self, _slot: u64) -> Result<(), String> {
      self
        .selected_slot_calls
        .set(self.selected_slot_calls.get() + 1);
      self.hm_bars_authorized.set(false);
      Ok(())
    }

    fn find_lpc_bars(&self) -> Result<(), String> {
      self.find_bars_calls.set(self.find_bars_calls.get() + 1);
      if self.selected_ldn.get() != Some(NUVOTON_HARDWARE_MONITOR_LDN) {
        return Err("find_lpc_bars called before LDN B selection".to_string());
      }
      if !self.hm_base_high_read.get() || !self.hm_base_low_read.get() {
        return Err("find_lpc_bars called before HM base discovery".to_string());
      }
      self.hm_bars_authorized.set(true);
      Ok(())
    }

    fn pio_inb(&self, port: u16) -> Result<u8, String> {
      if port != 0x0296 {
        return Err(format!("unexpected HM data port 0x{port:04X}"));
      }

      self.require_hm_bars()?;
      if self.selected_hm_bank.get() != NUVOTON_BANK_4 {
        return Err("bank 4 is not selected".to_string());
      }

      let register = self
        .selected_hm_register
        .get()
        .ok_or_else(|| "HM index register was not selected".to_string())?;
      self.read_registers.borrow_mut().push(register);
      Ok(Self::read_value_for(register))
    }

    fn pio_outb(&self, port: u16, value: u8) -> Result<(), String> {
      match port {
        0x0295 => {
          self.require_hm_bars()?;
          self.selected_hm_register.set(Some(value));
          Ok(())
        }
        0x0296 => {
          self.require_hm_bars()?;
          if self.selected_hm_register.get() == Some(HM_BANK_SELECT_REGISTER) {
            self.selected_hm_bank.set(value);
          }
          Ok(())
        }
        _ => Ok(()),
      }
    }

    fn superio_inb(&self, register: u8) -> Result<u8, String> {
      match register {
        CHIP_ID_HIGH_REGISTER => Ok(0xD8),
        CHIP_ID_LOW_REGISTER => Ok(0x02),
        LDN_ACTIVATION_REGISTER => {
          if self.selected_ldn.get() == Some(NUVOTON_HARDWARE_MONITOR_LDN) {
            Ok(0x09)
          } else {
            Ok(0x00)
          }
        }
        HM_BASE_HIGH_REGISTER => {
          self.hm_base_high_read.set(true);
          Ok(0x02)
        }
        HM_BASE_LOW_REGISTER => {
          self.hm_base_low_read.set(true);
          Ok(0x90)
        }
        _ => Ok(0),
      }
    }

    fn superio_outb(&self, register: u8, value: u8) -> Result<(), String> {
      if register == LOGICAL_DEVICE_SELECT_REGISTER {
        self.selected_ldn.set(Some(value));
      }
      Ok(())
    }
  }

  #[test]
  fn discover_slot_authorizes_hm_bars_after_base_discovery() {
    let client = FakeLpcIo::new();

    let detected = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0)
      .unwrap()
      .unwrap();

    assert_eq!(detected.slot, 0);
    assert_eq!(detected.hm_base, 0x0290);
    assert_eq!(client.selected_slot_calls.get(), 1);
    assert_eq!(client.find_bars_calls.get(), 1);
    assert!(client.hm_bars_authorized.get());
  }

  #[test]
  fn sample_preserves_discovered_hm_bar_authorization() {
    let active = ActiveNuvotonMotherboardSensors {
      client: FakeLpcIo::with_authorized_hm_bars(),
      slot: 0,
      hm_base: 0x0290,
      chip_label: NUVOTON_CHIP_LABEL,
      source_label: NUVOTON_SOURCE_LABEL,
      verification: SensorVerification::Verified,
    };

    let sample = active.sample_unlocked().unwrap();

    assert_eq!(active.client.selected_slot_calls.get(), 0);
    assert_eq!(sample.temperatures.len(), 6);
    assert_eq!(sample.fan_speeds.len(), 6);
    assert!(
      sample
        .temperatures
        .iter()
        .all(|sensor| sensor.verification == SensorVerification::Verified)
    );
    assert_eq!(
      active.client.read_registers.borrow().as_slice(),
      &[
        0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6,
        0xC7, 0xC8, 0xC9, 0xCA, 0xCB
      ]
    );
  }
}
