//! `LpcIO` discovery and sampling for the Nuvoton source-selected-slot
//! profiles (NCT6776F/NCT6776D and NCT6775F/NCT6771F).
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-nuvoton-nct6775-nct6776-nct610x.md`
//!   revision 1
//! - `docs/specs/sensors/superio-access.md` revision 4 (Nuvoton enter/exit,
//!   chip-ID reads, absent-ID classification, ISA mutex)
//! - `docs/specs/sensors/pawnio-interface.md` revision 9 (`LpcIO` contract
//!   and mutex conventions; the `LpcIO` section is unchanged since the
//!   revision 8 that the Nuvoton document cites)
//! - `docs/specs/sensors/superio-nuvoton-nct67xx.md` revision 9 (the shared
//!   normal-HM procedure: BAR discovery after base discovery, input identity)
//!
//! No other external sensor source was used.

use super::super::super::client::{ACCESS_ISABUS_MUTEX, NamedMutex};
use super::super::access::{chip_id, is_absent_id, slot_ports};
use super::super::nuvoton::fan_speed_status_from_rpm;
use super::super::nuvoton_smiovt::{
  NCT6776_CR1C_REGISTER, NCT6776_CR24_REGISTER, NCT6776_CR26_REGISTER,
  NCT6776_CR27_REGISTER, NCT6776_FAN_INPUT_CONTROL_REGISTER, NUVOTON_HM_VENDOR_ID,
  Nct6776FanPinConfiguration, NuvotonSmiovtProfile, SMIOVT_ALLOWED_BANK_SELECT_VALUES,
  SMIOVT_BANK_0, SMIOVT_BANK_6, SMIOVT_MONITORING_REGISTER,
  SMIOVT_VENDOR_ID_HIGH_BYTE_SELECT, SMIOVT_VENDOR_ID_LOW_BYTE_SELECT,
  SMIOVT_VENDOR_ID_REGISTER, SmiovtRegisterSnapshot, SmiovtTemperatureReading,
  decode_nct6776_direct_rpm, is_allowed_smiovt_hm_read, is_nct6776_lockreg_set,
  is_smiovt_fan_routed, is_smiovt_hardware_monitor_active, is_usable_smiovt_hm_base,
  nuvoton_smiovt_profile, select_smiovt_temperatures, smiovt_vendor_id,
};
use super::{
  CHIP_ID_HIGH_REGISTER, CHIP_ID_LOW_REGISTER, ChipFailure, HM_BANK_SELECT_REGISTER,
  HM_BASE_HIGH_REGISTER, HM_BASE_LOW_REGISTER, ISABUS_MUTEX_TIMEOUT,
  LDN_ACTIVATION_REGISTER, LOGICAL_DEVICE_SELECT_REGISTER, LpcIoOps,
  MotherboardSensorError, NUVOTON_CONFIGURATION_EXIT_FAILURE_CONTEXT,
  NUVOTON_HARDWARE_MONITOR_LDN, SlotProbe, enter_nuvoton, exit_nuvoton,
  read_nuvoton_hm_byte, write_nuvoton_hm_byte,
};
use crate::log_warn;
use crate::models::{
  MotherboardFanSpeed, MotherboardSensorSample, MotherboardTemperature, SensorSupport,
};

/// A source-selected-slot chip found on one `LpcIO` slot, cached after
/// detection ("Detection" step 9).
#[derive(Debug)]
pub(super) struct DetectedSmiovtSlot {
  slot: u8,
  raw_chip_id: u16,
  hm_base: u16,
  profile: &'static NuvotonSmiovtProfile,
  /// NCT6776 AUXFANIN routing registers; `None` on rows without fans.
  fan_pins: Option<Nct6776FanPinConfiguration>,
}

/// Why detection on a matched chip stopped.
enum DiscoveryFailure {
  /// A PawnIO/transport failure; install or permission guidance may apply.
  Transport(String),
  /// A failed HM port access after BAR discovery. Port authorization can
  /// recover, so the sampler retries detection.
  PortAccess(String),
  /// A gate read successfully through PawnIO failed; no guidance applies.
  HardwareState(String),
}

/// Route a transport failure of one detection step.
fn transport(context: &'static str) -> impl Fn(String) -> DiscoveryFailure {
  move |reason| DiscoveryFailure::Transport(format!("{context}: {reason}"))
}

impl ChipFailure {
  /// Every source-selected-slot row is Experimental.
  fn nuvoton_smiovt(profile: &NuvotonSmiovtProfile, detail: impl Into<String>) -> Self {
    Self {
      vendor: super::SuperIoVendor::Nuvoton,
      chip_label: profile.chip_label,
      experimental: profile.is_experimental(),
      hardware_state: false,
      requires_rediscovery: false,
      detail: detail.into(),
    }
  }
}

/// Configuration-mode steps 2-7 of "Detection" for a slot already selected
/// and in configuration mode.
fn discover_in_configuration_mode(
  client: &impl LpcIoOps,
  slot: u8,
) -> Result<Option<DetectedSmiovtSlot>, MotherboardSensorError> {
  let id_high = client.superio_inb(CHIP_ID_HIGH_REGISTER)?;
  let id_low = client.superio_inb(CHIP_ID_LOW_REGISTER)?;
  if is_absent_id(id_high, id_low) {
    return Ok(None);
  }
  let raw_chip_id = chip_id(id_high, id_low);
  let Some(profile) = nuvoton_smiovt_profile(raw_chip_id) else {
    return Ok(None);
  };

  discover_matched_chip(client, slot, raw_chip_id, profile)
    .map(Some)
    .map_err(|failure| {
      // The raw ID is always recorded in diagnostics.
      let with_id =
        |reason: String| format!("{reason} (raw chip ID 0x{raw_chip_id:04X})");
      match failure {
        DiscoveryFailure::Transport(reason) => {
          ChipFailure::nuvoton_smiovt(profile, with_id(reason)).into()
        }
        DiscoveryFailure::PortAccess(reason) => {
          ChipFailure::nuvoton_smiovt(profile, with_id(reason))
            .requiring_rediscovery()
            .into()
        }
        DiscoveryFailure::HardwareState(reason) => {
          ChipFailure::nuvoton_smiovt(profile, with_id(reason))
            .hardware_state()
            .into()
        }
      }
    })
}

fn discover_matched_chip(
  client: &impl LpcIoOps,
  slot: u8,
  raw_chip_id: u16,
  profile: &'static NuvotonSmiovtProfile,
) -> Result<DetectedSmiovtSlot, DiscoveryFailure> {
  let read_global =
    |register, context| client.superio_inb(register).map_err(transport(context));

  // Step 3 (NCT6776): LOCKREG safety gate, then the fan pin-function
  // registers. CR26 is never written.
  let global_pins = if profile.reads_nct6776_configuration() {
    let cr26 = read_global(NCT6776_CR26_REGISTER, "CR26 read failed")?;
    if is_nct6776_lockreg_set(cr26) {
      return Err(DiscoveryFailure::HardwareState(format!(
        "configuration registers are locked (CR26=0x{cr26:02X}, bit 5 LOCKREG set)"
      )));
    }
    let cr1c = read_global(NCT6776_CR1C_REGISTER, "CR1C read failed")?;
    let cr24 = read_global(NCT6776_CR24_REGISTER, "CR24 read failed")?;
    let cr27 = read_global(NCT6776_CR27_REGISTER, "CR27 read failed")?;
    Some((cr1c, cr24, cr27))
  } else {
    None
  };

  // Step 4: LDN B active. CR30 is never written.
  client
    .superio_outb(LOGICAL_DEVICE_SELECT_REGISTER, NUVOTON_HARDWARE_MONITOR_LDN)
    .map_err(transport("LDN B selection failed"))?;
  let cr30 = read_global(LDN_ACTIVATION_REGISTER, "CR30 read failed")?;
  if !is_smiovt_hardware_monitor_active(cr30) {
    return Err(DiscoveryFailure::HardwareState(format!(
      "hardware monitor logical device is inactive (CR30=0x{cr30:02X})"
    )));
  }
  let fan_pins = global_pins.map(|(cr1c, cr24, cr27)| Nct6776FanPinConfiguration {
    cr1c,
    cr24,
    cr27,
    cr30,
  });

  // Step 5: the aligned HM base gate.
  let base_high = read_global(HM_BASE_HIGH_REGISTER, "CR60 read failed")?;
  let base_low = read_global(HM_BASE_LOW_REGISTER, "CR61 read failed")?;
  let hm_base = u16::from_be_bytes([base_high, base_low]);
  if !is_usable_smiovt_hm_base(hm_base) {
    return Err(DiscoveryFailure::HardwareState(format!(
      "unusable hardware-monitor base 0x{hm_base:04X}"
    )));
  }

  // Step 6: BAR discovery after configuration entry and base discovery.
  client
    .find_lpc_bars()
    .map_err(transport("LpcIO BAR discovery failed"))?;

  // Step 7: vendor ID through the HM ports. A port failure here requests a
  // fresh detection; a vendor ID that reads but differs is a hardware state.
  let read_vendor_byte = |select| {
    write_nuvoton_hm_byte(client, hm_base, HM_BANK_SELECT_REGISTER, select)
      .and_then(|()| read_nuvoton_hm_byte(client, hm_base, SMIOVT_VENDOR_ID_REGISTER))
      .map_err(|reason| {
        DiscoveryFailure::PortAccess(format!("vendor ID read failed: {reason}"))
      })
  };
  let vendor_high = read_vendor_byte(SMIOVT_VENDOR_ID_HIGH_BYTE_SELECT)?;
  let vendor_low = read_vendor_byte(SMIOVT_VENDOR_ID_LOW_BYTE_SELECT)?;
  let vendor_id = smiovt_vendor_id(vendor_high, vendor_low);
  if vendor_id != NUVOTON_HM_VENDOR_ID {
    return Err(DiscoveryFailure::HardwareState(format!(
      "unexpected hardware-monitor vendor ID 0x{vendor_id:04X}"
    )));
  }

  Ok(DetectedSmiovtSlot {
    slot,
    raw_chip_id,
    hm_base,
    profile,
    fan_pins,
  })
}

/// Sequential hardware-monitor reads that select a bank only when the next
/// read is in a different bank. Every bank-select value and every read is
/// checked against the spec's allow-lists before any port I/O.
struct HmBankReader<'a, C: LpcIoOps> {
  client: &'a C,
  hm_base: u16,
  selected_bank: Option<u8>,
}

impl<'a, C: LpcIoOps> HmBankReader<'a, C> {
  fn new(client: &'a C, hm_base: u16) -> Self {
    Self {
      client,
      hm_base,
      selected_bank: None,
    }
  }

  fn read(&mut self, bank: u8, register: u8) -> Result<u8, String> {
    if !is_allowed_smiovt_hm_read(bank, register) {
      return Err(format!(
        "refused read of bank {bank} register 0x{register:02X}"
      ));
    }
    if self.selected_bank != Some(bank) {
      if !SMIOVT_ALLOWED_BANK_SELECT_VALUES.contains(&bank) {
        return Err(format!("refused bank-select write 0x{bank:02X}"));
      }
      write_nuvoton_hm_byte(self.client, self.hm_base, HM_BANK_SELECT_REGISTER, bank)?;
      self.selected_bank = Some(bank);
    }
    read_nuvoton_hm_byte(self.client, self.hm_base, register)
  }
}

pub(super) struct ActiveNuvotonSmiovtSensors<C: LpcIoOps> {
  client: C,
  detected: DetectedSmiovtSlot,
  /// Whether the current run of partial samples was already logged.
  partial_failure_logged: bool,
}

impl<C: LpcIoOps> ActiveNuvotonSmiovtSensors<C> {
  pub(super) fn new(client: C, detected: DetectedSmiovtSlot) -> Self {
    Self {
      client,
      detected,
      partial_failure_logged: false,
    }
  }

  #[cfg(test)]
  pub(super) fn discover_slot(
    client: &C,
    slot: u8,
  ) -> Result<Option<DetectedSmiovtSlot>, MotherboardSensorError> {
    Self::probe_slot(client, slot).result
  }

  /// "Detection" for one slot. The caller holds the ISA mutex. Once the slot
  /// is selected, configuration mode is exited on every path, including a
  /// failed entry key write ("Detection" step 8).
  pub(super) fn probe_slot(client: &C, slot: u8) -> SlotProbe<DetectedSmiovtSlot> {
    let Some((index_port, _data_port)) = slot_ports(slot) else {
      return SlotProbe::exited(Ok(None));
    };

    if let Err(reason) = client.select_lpc_slot(slot as u64) {
      // Nothing was written to the configuration ports.
      return SlotProbe::exited(Err(reason.into()));
    }
    if let Err(entry_error) = enter_nuvoton(client, index_port) {
      // Best-effort exit; the entry failure stays the primary error.
      let error = MotherboardSensorError::from(entry_error);
      return match exit_nuvoton(client, index_port) {
        Ok(()) => SlotProbe::exited(Err(error)),
        Err(exit_error) => SlotProbe::unconfirmed(Err(
          error.with_exit_failure(format!("; exit failed: {exit_error}"), true),
        )),
      };
    }
    let result = discover_in_configuration_mode(client, slot);
    let exit_result = exit_nuvoton(client, index_port);
    let configuration_exited = exit_result.is_ok();

    let result = match (result, exit_result) {
      (Ok(value), Ok(())) => Ok(value),
      (Ok(Some(detected)), Err(exit_error)) => Err(
        ChipFailure::nuvoton_smiovt(
          detected.profile,
          format!("{NUVOTON_CONFIGURATION_EXIT_FAILURE_CONTEXT}: {exit_error}"),
        )
        .requiring_rediscovery()
        .into(),
      ),
      (Ok(None), Err(exit_error)) => {
        Err(format!("{NUVOTON_CONFIGURATION_EXIT_FAILURE_CONTEXT}: {exit_error}").into())
      }
      (Err(reason), Ok(())) => Err(reason),
      // A matched chip whose exit failed needs a fresh detection, as on the
      // successful-detection branch above.
      (Err(reason), Err(exit_error)) => {
        Err(reason.with_exit_failure(format!("; exit failed: {exit_error}"), true))
      }
    };
    SlotProbe {
      result,
      configuration_exited,
    }
  }

  /// Fan capability of the detected row: NCT6776 reads fans, NCT6775 does not.
  pub(super) fn fan_support(&self) -> SensorSupport {
    if self.detected.profile.fan_inputs().is_empty() {
      SensorSupport::Unsupported
    } else {
      SensorSupport::Supported
    }
  }

  pub(super) fn diagnostic_summary(&self) -> String {
    let detected = &self.detected;
    format!(
      "active chip={} raw_id=0x{:04X} slot={} hm_base=0x{:04X}",
      detected.profile.chip_label, detected.raw_chip_id, detected.slot, detected.hm_base
    )
  }

  pub(super) fn sample(
    &mut self,
  ) -> Result<MotherboardSensorSample, MotherboardSensorError> {
    let _mutex = NamedMutex::acquire(ACCESS_ISABUS_MUTEX, ISABUS_MUTEX_TIMEOUT)?;
    self.sample_unlocked_logged()
  }

  fn sample_unlocked_logged(
    &mut self,
  ) -> Result<MotherboardSensorSample, MotherboardSensorError> {
    let (sample, partial_failures) = self.collect_sample_unlocked()?;
    if partial_failures.is_empty() {
      self.partial_failure_logged = false;
    } else if !self.partial_failure_logged {
      self.partial_failure_logged = true;
      log_warn!(
        "nuvoton_smiovt_motherboard_sensor_partial_sample",
        "windows::super_io_motherboard::ActiveNuvotonSmiovtSensors::sample",
        Some(format!(
          "{}: {}",
          self.detected.profile.chip_label,
          partial_failures.join("; ")
        ))
      );
    }
    Ok(sample)
  }

  /// "Sampling": one transaction through the HM ports. A failed index write,
  /// bank-select write or data read aborts the sample; the `Vec<String>`
  /// carries inputs that are unavailable without failing the sample.
  fn collect_sample_unlocked(
    &self,
  ) -> Result<(MotherboardSensorSample, Vec<String>), MotherboardSensorError> {
    let profile = self.detected.profile;
    let failure = |reason: String| -> MotherboardSensorError {
      ChipFailure::nuvoton_smiovt(profile, reason).into()
    };
    let mut reader = HmBankReader::new(&self.client, self.detected.hm_base);

    // Step 1: the monitoring gate. `0x40` is never written.
    let monitoring = reader
      .read(SMIOVT_BANK_0, SMIOVT_MONITORING_REGISTER)
      .map_err(failure)?;
    if !profile.is_monitoring(monitoring) {
      return Err(
        ChipFailure::nuvoton_smiovt(
          profile,
          format!("hardware monitoring is not running (bank 0 0x40=0x{monitoring:02X})"),
        )
        .hardware_state()
        .into(),
      );
    }

    // Step 2: the slot registers, bank by bank.
    let mut snapshot = SmiovtRegisterSnapshot::default();
    for (bank, registers) in profile.sample_reads() {
      for &register in *registers {
        let value = reader.read(*bank, register).map_err(failure)?;
        snapshot.insert((*bank, register), value);
      }
    }

    // Then the RPM pairs of the routed fan inputs, still in bank 6.
    let mut fan_speeds = Vec::new();
    if !profile.fan_inputs().is_empty() {
      let fan_input_control = snapshot
        .get((SMIOVT_BANK_0, NCT6776_FAN_INPUT_CONTROL_REGISTER))
        .map_err(failure)?;
      for fan in profile.fan_inputs() {
        if !is_smiovt_fan_routed(
          fan.route,
          self.detected.fan_pins.as_ref(),
          fan_input_control,
        ) {
          continue;
        }
        let high = reader.read(SMIOVT_BANK_6, fan.rpm_high).map_err(failure)?;
        let low = reader.read(SMIOVT_BANK_6, fan.rpm_low).map_err(failure)?;
        let rpm = decode_nct6776_direct_rpm(high, low);
        fan_speeds.push(MotherboardFanSpeed {
          name: fan.label.to_string(),
          rpm: Some(rpm),
          status: fan_speed_status_from_rpm(rpm),
          source: profile.source_label().to_string(),
        });
      }
    }

    let mut temperatures = Vec::new();
    let mut partial_failures = Vec::new();
    for (source, reading) in
      select_smiovt_temperatures(profile, &snapshot).map_err(failure)?
    {
      match reading {
        SmiovtTemperatureReading::Available { temperature, .. } => {
          temperatures.push(MotherboardTemperature {
            name: source.label().to_string(),
            temperature,
            source: profile.source_label().to_string(),
          });
        }
        SmiovtTemperatureReading::Implausible { slot, temperature } => {
          partial_failures.push(format!(
            "{} ({slot}) returned implausible value {temperature} C and is unavailable",
            source.label()
          ));
        }
        SmiovtTemperatureReading::NoCandidate => {}
      }
    }

    Ok((
      MotherboardSensorSample {
        temperatures,
        fan_speeds,
      },
      partial_failures,
    ))
  }
}

#[cfg(test)]
mod tests {
  use std::cell::{Cell, RefCell};
  use std::collections::HashMap;

  use super::*;
  use crate::models::FanSpeedStatus;

  const HM_BASE: u16 = 0x0290;
  const INDEX_PORT: u16 = HM_BASE + 5;
  const DATA_PORT: u16 = HM_BASE + 6;

  /// A synthetic NCT6776F/NCT6775F register dump behind a fake `LpcIO`. Every
  /// write is checked against the spec's allowed writes and every HM read
  /// against the never-read list; a violation panics.
  struct FakeSmiovtChip {
    chip_id: Cell<u16>,
    /// Global and LDN B configuration registers.
    configuration: RefCell<HashMap<u8, u8>>,
    /// Hardware-monitor registers keyed by (bank, index).
    hm: RefCell<HashMap<(u8, u8), u8>>,
    vendor_id: Cell<(u8, u8)>,
    in_configuration_mode: Cell<bool>,
    selected_ldn: Cell<Option<u8>>,
    bars_authorized: Cell<bool>,
    find_bars_calls: Cell<u32>,
    hm_index: Cell<Option<u8>>,
    bank_select: Cell<u8>,
    fail_config_exit: Cell<bool>,
    /// 1-based configuration exit call that fails (every call counts).
    fail_exit_call: Cell<Option<u32>>,
    exit_calls: Cell<u32>,
    /// Fail the second `0x87` entry key of a session.
    fail_second_entry_key: Cell<bool>,
    entry_keys_in_session: Cell<u32>,
    fail_select_slot: Cell<Option<u64>>,
    failed_hm_read: Cell<Option<(u8, u8)>>,
    config_reads: RefCell<Vec<u8>>,
    bank_select_writes: RefCell<Vec<u8>>,
    hm_reads: RefCell<Vec<(u8, u8)>>,
  }

  impl FakeSmiovtChip {
    /// NCT6776F version C at base `0x0290`, monitoring, default source
    /// selects, all five fan inputs routed (FANINC1/2 set, AUXFANIN0 on its
    /// dedicated pin with `CR24` bit 6 = 0, AUXFANIN1/2 by `CR1C`).
    fn nct6776() -> Self {
      let configuration = HashMap::from([
        (0x1C, 0x03),
        (0x24, 0x00),
        (0x26, 0x00),
        (0x27, 0x00),
        (0x30, 0x01),
        (0x60, 0x02),
        (0x61, 0x90),
      ]);
      let hm = HashMap::from([
        ((0, 0x40), 0x01),
        ((0, 0x27), 0x1E),
        ((0, 0x18), 0x00),
        ((0, 0x4D), 0x05),
        ((1, 0x50), 0x28),
        ((1, 0x51), 0x80),
        ((1, 0x52), 0x00),
        ((2, 0x50), 0x23),
        ((2, 0x51), 0x00),
        ((2, 0x52), 0x00),
        ((6, 0x21), 0x01),
        ((6, 0x22), 0x02),
        ((6, 0x23), 0x03),
        ((6, 0x24), 0x01),
        ((6, 0x25), 0x01),
        ((6, 0x26), 0x01),
        ((6, 0x28), 0x00),
        ((6, 0x29), 0x00),
        ((6, 0x2A), 0x00),
        ((6, 0x2B), 0x20),
        ((6, 0x2C), 0x21),
        ((6, 0x2D), 0x22),
        ((6, 0x2E), 0x01),
        // Fan 1 2500 RPM, Fan 2 1200 RPM, Fan 3 power-on default (stopped),
        // Fan 4 165 RPM (just above the floor), Fan 5 0x0000.
        ((6, 0x56), 0x09),
        ((6, 0x57), 0xC4),
        ((6, 0x58), 0x04),
        ((6, 0x59), 0xB0),
        ((6, 0x5A), 0x00),
        ((6, 0x5B), 0xA4),
        ((6, 0x5C), 0x00),
        ((6, 0x5D), 0xA5),
        ((6, 0x5E), 0x00),
        ((6, 0x5F), 0x00),
      ]);
      Self {
        chip_id: Cell::new(0xC333),
        configuration: RefCell::new(configuration),
        hm: RefCell::new(hm),
        vendor_id: Cell::new((0x5C, 0xA3)),
        in_configuration_mode: Cell::new(false),
        selected_ldn: Cell::new(None),
        bars_authorized: Cell::new(false),
        find_bars_calls: Cell::new(0),
        hm_index: Cell::new(None),
        bank_select: Cell::new(0x80),
        fail_config_exit: Cell::new(false),
        fail_exit_call: Cell::new(None),
        exit_calls: Cell::new(0),
        fail_second_entry_key: Cell::new(false),
        entry_keys_in_session: Cell::new(0),
        fail_select_slot: Cell::new(None),
        failed_hm_read: Cell::new(None),
        config_reads: RefCell::new(Vec::new()),
        bank_select_writes: RefCell::new(Vec::new()),
        hm_reads: RefCell::new(Vec::new()),
      }
    }

    fn with_chip_id(chip_id: u16) -> Self {
      let chip = Self::nct6776();
      chip.chip_id.set(chip_id);
      chip
    }

    fn set_config(&self, register: u8, value: u8) {
      self.configuration.borrow_mut().insert(register, value);
    }

    fn set_hm(&self, bank: u8, register: u8, value: u8) {
      self.hm.borrow_mut().insert((bank, register), value);
    }

    fn hm_reads_after_gate(&self) -> Vec<(u8, u8)> {
      self.hm_reads.borrow()[1..].to_vec()
    }
  }

  impl LpcIoOps for FakeSmiovtChip {
    fn select_lpc_slot(&self, slot: u64) -> Result<(), String> {
      if self.fail_select_slot.get() == Some(slot) {
        return Err(format!("simulated slot {slot} selection failure"));
      }
      self.bars_authorized.set(false);
      Ok(())
    }

    fn find_lpc_bars(&self) -> Result<(), String> {
      self.find_bars_calls.set(self.find_bars_calls.get() + 1);
      assert!(
        self.in_configuration_mode.get(),
        "BARs found outside configuration mode"
      );
      assert_eq!(
        self.selected_ldn.get(),
        Some(0x0B),
        "BARs found before LDN B"
      );
      let reads = self.config_reads.borrow();
      assert!(
        reads.contains(&0x60) && reads.contains(&0x61),
        "BARs found before base discovery"
      );
      self.bars_authorized.set(true);
      Ok(())
    }

    fn pio_inb(&self, port: u16) -> Result<u8, String> {
      assert_eq!(port, DATA_PORT, "unexpected port read");
      if !self.bars_authorized.get() {
        return Err("HM ports are not authorized".to_string());
      }
      let register = self
        .hm_index
        .get()
        .expect("HM index selected before data read");
      let bank_select = self.bank_select.get();
      let bank = bank_select & 0x07;
      assert!(
        is_allowed_smiovt_hm_read(bank, register),
        "read of never-read bank {bank} register 0x{register:02X}"
      );
      self.hm_reads.borrow_mut().push((bank, register));
      if self.failed_hm_read.get() == Some((bank, register)) {
        return Err(format!("simulated bank {bank} 0x{register:02X} failure"));
      }
      if register == 0x4F && bank == 0 {
        let (high, low) = self.vendor_id.get();
        return Ok(if bank_select & 0x80 != 0 { high } else { low });
      }
      Ok(
        self
          .hm
          .borrow()
          .get(&(bank, register))
          .copied()
          .unwrap_or(0),
      )
    }

    fn pio_outb(&self, port: u16, value: u8) -> Result<(), String> {
      match port {
        0x2E | 0x4E => {
          match value {
            0x87 => {
              let keys = self.entry_keys_in_session.get() + 1;
              self.entry_keys_in_session.set(keys);
              self.in_configuration_mode.set(true);
              if keys == 2 && self.fail_second_entry_key.get() {
                return Err("simulated entry key failure".to_string());
              }
            }
            0xAA => {
              self.entry_keys_in_session.set(0);
              let call = self.exit_calls.get() + 1;
              self.exit_calls.set(call);
              if self.fail_config_exit.get() || self.fail_exit_call.get() == Some(call) {
                return Err("simulated config exit failure".to_string());
              }
              self.in_configuration_mode.set(false);
            }
            _ => panic!("unexpected configuration key 0x{value:02X}"),
          }
          Ok(())
        }
        INDEX_PORT => {
          if !self.bars_authorized.get() {
            return Err("HM ports are not authorized".to_string());
          }
          self.hm_index.set(Some(value));
          Ok(())
        }
        DATA_PORT => {
          if !self.bars_authorized.get() {
            return Err("HM ports are not authorized".to_string());
          }
          // "Safety notes": the only HM data write is the bank select.
          assert_eq!(
            self.hm_index.get(),
            Some(HM_BANK_SELECT_REGISTER),
            "HM data write to a register other than 0x4E"
          );
          assert!(
            SMIOVT_ALLOWED_BANK_SELECT_VALUES.contains(&value),
            "bank-select write 0x{value:02X} outside the allow-list"
          );
          self.bank_select.set(value);
          self.bank_select_writes.borrow_mut().push(value);
          Ok(())
        }
        _ => panic!("unexpected port write 0x{port:04X}"),
      }
    }

    fn superio_inb(&self, register: u8) -> Result<u8, String> {
      assert!(
        self.in_configuration_mode.get(),
        "config read outside configuration mode"
      );
      self.config_reads.borrow_mut().push(register);
      Ok(match register {
        0x20 => (self.chip_id.get() >> 8) as u8,
        0x21 => self.chip_id.get() as u8,
        _ => {
          // `0x30` and above address the selected logical device.
          if register >= 0x30 {
            assert_eq!(
              self.selected_ldn.get(),
              Some(0x0B),
              "LDN register before LDN B"
            );
          }
          self
            .configuration
            .borrow()
            .get(&register)
            .copied()
            .unwrap_or(0)
        }
      })
    }

    fn superio_outb(&self, register: u8, value: u8) -> Result<(), String> {
      // "Safety notes": the only configuration write is CR07 = 0x0B.
      assert_eq!(
        (register, value),
        (LOGICAL_DEVICE_SELECT_REGISTER, NUVOTON_HARDWARE_MONITOR_LDN),
        "configuration write outside the allow-list"
      );
      self.selected_ldn.set(Some(value));
      Ok(())
    }
  }

  /// Lets `ActiveMotherboardSensors::discover` borrow the fake so a test can
  /// inspect it afterwards.
  impl LpcIoOps for &FakeSmiovtChip {
    fn select_lpc_slot(&self, slot: u64) -> Result<(), String> {
      (**self).select_lpc_slot(slot)
    }

    fn find_lpc_bars(&self) -> Result<(), String> {
      (**self).find_lpc_bars()
    }

    fn pio_inb(&self, port: u16) -> Result<u8, String> {
      (**self).pio_inb(port)
    }

    fn pio_outb(&self, port: u16, value: u8) -> Result<(), String> {
      (**self).pio_outb(port, value)
    }

    fn superio_inb(&self, register: u8) -> Result<u8, String> {
      (**self).superio_inb(register)
    }

    fn superio_outb(&self, register: u8, value: u8) -> Result<(), String> {
      (**self).superio_outb(register, value)
    }
  }

  fn discover(
    chip: &FakeSmiovtChip,
  ) -> Result<Option<DetectedSmiovtSlot>, MotherboardSensorError> {
    ActiveNuvotonSmiovtSensors::discover_slot(chip, 0)
  }

  fn active(chip: FakeSmiovtChip) -> ActiveNuvotonSmiovtSensors<FakeSmiovtChip> {
    let detected = discover(&chip).unwrap().unwrap();
    chip.hm_reads.borrow_mut().clear();
    chip.bank_select_writes.borrow_mut().clear();
    ActiveNuvotonSmiovtSensors::new(chip, detected)
  }

  fn temperatures(sample: &MotherboardSensorSample) -> Vec<(&str, f32)> {
    sample
      .temperatures
      .iter()
      .map(|temperature| (temperature.name.as_str(), temperature.temperature))
      .collect()
  }

  fn fans(sample: &MotherboardSensorSample) -> Vec<(&str, Option<u32>)> {
    sample
      .fan_speeds
      .iter()
      .map(|fan| (fan.name.as_str(), fan.rpm))
      .collect()
  }

  const NCT6776_FAILURE_PREFIX: &str =
    "Experimental NCT6776F / NCT6776D motherboard sensor path failed";
  const HARDWARE_STATE_PREFIX: &str =
    "Experimental Nuvoton motherboard sensor path failed: hardware state";

  // --- Spec "Read procedure and decode" > Detection ---

  #[test]
  fn detection_follows_the_documented_configuration_sequence() {
    let chip = FakeSmiovtChip::nct6776();

    let detected = discover(&chip).unwrap().unwrap();

    assert_eq!(detected.raw_chip_id, 0xC333);
    assert_eq!(detected.hm_base, HM_BASE);
    assert_eq!(detected.profile.chip_label, "NCT6776F / NCT6776D");
    // Steps 2-5 in order: chip ID, CR26, CR1C/24/27, CR30, CR60/61.
    assert_eq!(
      chip.config_reads.borrow().as_slice(),
      &[0x20, 0x21, 0x26, 0x1C, 0x24, 0x27, 0x30, 0x60, 0x61]
    );
    assert_eq!(chip.find_bars_calls.get(), 1);
    // Step 7: vendor ID with HBACS = 1, then 0.
    assert_eq!(chip.bank_select_writes.borrow().as_slice(), &[0x80, 0x00]);
    assert_eq!(chip.hm_reads.borrow().as_slice(), &[(0, 0x4F), (0, 0x4F)]);
    // Step 8: configuration mode exited.
    assert!(!chip.in_configuration_mode.get());
  }

  #[test]
  fn detection_on_nct6775_skips_lockreg_and_pin_registers() {
    let chip = FakeSmiovtChip::with_chip_id(0xB473);
    // A set LOCKREG bit is not a gate on the NCT6775 row.
    chip.set_config(0x26, 0x20);

    let detected = discover(&chip).unwrap().unwrap();

    assert_eq!(detected.profile.chip_label, "NCT6775F / NCT6771F");
    assert_eq!(detected.fan_pins, None);
    assert_eq!(
      chip.config_reads.borrow().as_slice(),
      &[0x20, 0x21, 0x30, 0x60, 0x61]
    );
  }

  #[test]
  fn detection_ignores_ids_outside_the_document() {
    for raw_id in [0xC338, 0xB478, 0x1061, 0xC452, 0xD802, 0x8728] {
      let chip = FakeSmiovtChip::with_chip_id(raw_id);

      assert!(discover(&chip).unwrap().is_none(), "0x{raw_id:04X}");
      assert_eq!(chip.config_reads.borrow().as_slice(), &[0x20, 0x21]);
      assert_eq!(chip.selected_ldn.get(), None);
      assert_eq!(chip.find_bars_calls.get(), 0);
      assert!(!chip.in_configuration_mode.get());
    }
    for absent in [0x0000, 0xFFFF] {
      assert!(
        discover(&FakeSmiovtChip::with_chip_id(absent))
          .unwrap()
          .is_none()
      );
    }
  }

  #[test]
  fn detection_lockreg_stops_before_ldn_selection_as_hardware_state() {
    let chip = FakeSmiovtChip::nct6776();
    chip.set_config(0x26, 0x20);

    let failure = discover(&chip).unwrap_err();

    assert_eq!(
      failure.to_string(),
      format!(
        "{HARDWARE_STATE_PREFIX} (NCT6776F / NCT6776D): configuration registers are \
         locked (CR26=0x20, bit 5 LOCKREG set) (raw chip ID 0xC333)"
      )
    );
    assert!(!failure.offers_pawnio_guidance());
    assert!(!failure.is_retryable());
    assert_eq!(chip.selected_ldn.get(), None);
    assert_eq!(chip.find_bars_calls.get(), 0);
    assert!(!chip.in_configuration_mode.get());
  }

  #[test]
  fn detection_inactive_ldn_b_is_unavailable_and_never_activated() {
    let chip = FakeSmiovtChip::nct6776();
    chip.set_config(0x30, 0xE0);

    let failure = discover(&chip).unwrap_err();

    let error = failure.to_string();
    assert!(error.starts_with(HARDWARE_STATE_PREFIX), "{error}");
    assert!(
      error.contains("logical device is inactive (CR30=0xE0)"),
      "{error}"
    );
    assert_eq!(chip.find_bars_calls.get(), 0);
    assert!(!chip.in_configuration_mode.get());
  }

  #[test]
  fn detection_rejects_a_base_outside_the_aligned_window() {
    for (high, low) in [
      (0x02, 0x92),
      (0x02, 0x95),
      (0x00, 0x00),
      (0x00, 0xF8),
      (0x10, 0x00),
    ] {
      let chip = FakeSmiovtChip::nct6776();
      chip.set_config(0x60, high);
      chip.set_config(0x61, low);

      let error = discover(&chip).unwrap_err().to_string();

      assert!(error.starts_with(HARDWARE_STATE_PREFIX), "{error}");
      assert!(
        error.contains(&format!(
          "unusable hardware-monitor base 0x{high:02X}{low:02X}"
        )),
        "{error}"
      );
      assert_eq!(chip.find_bars_calls.get(), 0);
    }
  }

  #[test]
  fn detection_requires_the_nuvoton_vendor_id() {
    let chip = FakeSmiovtChip::nct6776();
    chip.vendor_id.set((0xA3, 0x5C));

    let failure = discover(&chip).unwrap_err();

    let error = failure.to_string();
    assert!(error.starts_with(HARDWARE_STATE_PREFIX), "{error}");
    assert!(
      error.contains("unexpected hardware-monitor vendor ID 0xA35C"),
      "{error}"
    );
    // A vendor ID that reads but differs is not retried.
    assert!(!failure.is_retryable());
    assert!(!chip.in_configuration_mode.get());
  }

  #[test]
  fn detection_transport_failure_keeps_the_component_wording() {
    let chip = FakeSmiovtChip::nct6776();
    chip.failed_hm_read.set(Some((0, 0x4F)));

    let failure = discover(&chip).unwrap_err();

    let error = failure.to_string();
    assert!(error.starts_with(NCT6776_FAILURE_PREFIX), "{error}");
    assert!(error.contains("vendor ID read failed"), "{error}");
    assert!(failure.offers_pawnio_guidance());
    // A failed HM port access can recover, so detection is retried.
    assert!(failure.is_retryable());
  }

  #[test]
  fn detection_exit_failure_requires_rediscovery() {
    let chip = FakeSmiovtChip::nct6776();
    chip.fail_config_exit.set(true);

    let probe = ActiveNuvotonSmiovtSensors::probe_slot(&chip, 0);

    assert!(!probe.configuration_exited);
    let failure = probe.result.unwrap_err();
    assert_eq!(
      failure.to_string(),
      format!(
        "{NCT6776_FAILURE_PREFIX}: {NUVOTON_CONFIGURATION_EXIT_FAILURE_CONTEXT}: \
         simulated config exit failure"
      )
    );
    assert!(failure.is_retryable());
  }

  #[test]
  fn detection_failure_with_a_failed_exit_requires_rediscovery() {
    for configure in [
      // A vendor-ID port failure, and a hardware-state gate failure.
      (|chip: &FakeSmiovtChip| chip.failed_hm_read.set(Some((0, 0x4F))))
        as fn(&FakeSmiovtChip),
      |chip: &FakeSmiovtChip| chip.set_config(0x26, 0x20),
    ] {
      let chip = FakeSmiovtChip::nct6776();
      configure(&chip);
      chip.fail_config_exit.set(true);

      let probe = ActiveNuvotonSmiovtSensors::probe_slot(&chip, 0);

      assert!(!probe.configuration_exited);
      let failure = probe.result.unwrap_err();
      let error = failure.to_string();
      assert!(
        error.ends_with("; exit failed: simulated config exit failure"),
        "{error}"
      );
      assert!(failure.is_retryable(), "{error}");
    }
  }

  #[test]
  fn detection_attempts_an_exit_after_a_failed_entry_key() {
    let chip = FakeSmiovtChip::nct6776();
    chip.fail_second_entry_key.set(true);

    let probe = ActiveNuvotonSmiovtSensors::probe_slot(&chip, 0);

    assert!(probe.configuration_exited);
    assert_eq!(
      probe.result.unwrap_err().to_string(),
      "simulated entry key failure"
    );
    assert_eq!(chip.exit_calls.get(), 1);
    assert!(!chip.in_configuration_mode.get());
    // No configuration register was read after the failed entry.
    assert!(chip.config_reads.borrow().is_empty());

    let chip = FakeSmiovtChip::nct6776();
    chip.fail_second_entry_key.set(true);
    chip.fail_config_exit.set(true);

    let probe = ActiveNuvotonSmiovtSensors::probe_slot(&chip, 0);

    assert!(!probe.configuration_exited);
    assert_eq!(
      probe.result.unwrap_err().to_string(),
      "simulated entry key failure; exit failed: simulated config exit failure"
    );
  }

  #[test]
  fn detection_writes_nothing_when_slot_selection_fails() {
    let chip = FakeSmiovtChip::nct6776();
    chip.fail_select_slot.set(Some(0));

    let probe = ActiveNuvotonSmiovtSensors::probe_slot(&chip, 0);

    assert!(probe.configuration_exited);
    assert!(probe.result.is_err());
    assert_eq!(chip.exit_calls.get(), 0);
    assert_eq!(chip.entry_keys_in_session.get(), 0);
  }

  // --- Spec "Safety notes": the slot probe sequence of `open()` ---

  #[test]
  fn failed_smiovt_exit_skips_the_ite_probe_on_that_slot() {
    // Exit 1 is the bank 4 probe (no profile for 0xC333), exit 2 the
    // source-selected probe. The fake panics on any ITE key or CR02 write.
    let chip = FakeSmiovtChip::nct6776();
    chip.fail_exit_call.set(Some(2));
    chip.fail_select_slot.set(Some(1));

    let failure = match super::super::ActiveMotherboardSensors::discover(&chip) {
      Ok(_) => panic!("discovery should fail"),
      Err(failure) => failure,
    };
    assert_eq!(chip.exit_calls.get(), 2);

    // The retryable exit failure of slot 0 survives the later slot 1
    // selection failures.
    assert!(failure.is_retryable());
    assert_eq!(
      failure.to_string(),
      format!(
        "{NCT6776_FAILURE_PREFIX}: {NUVOTON_CONFIGURATION_EXIT_FAILURE_CONTEXT}: \
         simulated config exit failure"
      )
    );
  }

  #[test]
  fn failed_bank4_probe_exit_skips_the_rest_of_that_slot() {
    let chip = FakeSmiovtChip::nct6776();
    chip.fail_exit_call.set(Some(1));
    chip.fail_select_slot.set(Some(1));

    assert!(super::super::ActiveMotherboardSensors::discover(&chip).is_err());

    // Neither the source-selected nor the ITE probe ran on slot 0.
    assert_eq!(chip.exit_calls.get(), 1);
    assert_eq!(chip.config_reads.borrow().as_slice(), &[0x20, 0x21]);
  }

  #[test]
  fn diagnostic_summary_records_the_raw_chip_id() {
    let active = active(FakeSmiovtChip::with_chip_id(0xC335));
    assert_eq!(
      active.diagnostic_summary(),
      "active chip=NCT6776 family (unrecognized version) raw_id=0xC335 slot=0 hm_base=0x0290"
    );
  }

  // --- Spec "Read procedure and decode" > Sampling ---

  #[test]
  fn nct6776_sample_reads_exactly_the_documented_registers_in_order() {
    let active = active(FakeSmiovtChip::nct6776());

    active.collect_sample_unlocked().unwrap();

    let mut expected = vec![(0, 0x40), (0, 0x27), (0, 0x18), (0, 0x4D)];
    expected.extend([0x50, 0x51, 0x52].map(|register| (1, register)));
    expected.extend([0x50, 0x51, 0x52].map(|register| (2, register)));
    expected.extend(
      [
        0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E,
      ]
      .map(|register| (6, register)),
    );
    expected.extend((0x56..=0x5F).map(|register| (6, register)));
    assert_eq!(
      active.client.hm_reads.borrow().as_slice(),
      expected.as_slice()
    );
    // One bank-select write per bank, bit 7 clear.
    assert_eq!(
      active.client.bank_select_writes.borrow().as_slice(),
      &[0x00, 0x01, 0x02, 0x06]
    );
  }

  #[test]
  fn nct6775_sample_reads_temperatures_only() {
    let active = active(FakeSmiovtChip::with_chip_id(0xB472));

    let (sample, partial_failures) = active.collect_sample_unlocked().unwrap();

    let mut expected = vec![(0, 0x40), (0, 0x27)];
    expected.extend([0x50, 0x51, 0x52].map(|register| (1, register)));
    expected.extend([0x50, 0x51, 0x52].map(|register| (2, register)));
    expected.extend(
      [
        0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D,
      ]
      .map(|register| (6, register)),
    );
    assert_eq!(
      active.client.hm_reads.borrow().as_slice(),
      expected.as_slice()
    );
    // SMIOVT1 is preferred over the 8-bit bank 6 slots on this row.
    assert_eq!(
      temperatures(&sample),
      [("SYSTIN", 30.0), ("CPUTIN", 40.5), ("AUXTIN", 35.0)]
    );
    assert!(sample.fan_speeds.is_empty());
    assert!(partial_failures.is_empty());
    assert_eq!(active.fan_support(), SensorSupport::Unsupported);
  }

  #[test]
  fn nct6776_sample_publishes_sources_once_and_fans_with_the_floor() {
    let active = active(FakeSmiovtChip::nct6776());

    let (sample, partial_failures) = active.collect_sample_unlocked().unwrap();

    assert_eq!(
      temperatures(&sample),
      [("SYSTIN", 32.5), ("CPUTIN", 40.5), ("AUXTIN", 35.0)]
    );
    assert_eq!(
      fans(&sample),
      [
        ("Fan 1", Some(2500)),
        ("Fan 2", Some(1200)),
        ("Fan 3", Some(0)),
        ("Fan 4", Some(165)),
        ("Fan 5", Some(0)),
      ]
    );
    assert_eq!(sample.fan_speeds[2].status, FanSpeedStatus::Inactive);
    assert_eq!(sample.fan_speeds[3].status, FanSpeedStatus::Active);
    for source in sample
      .temperatures
      .iter()
      .map(|temperature| &temperature.source)
      .chain(sample.fan_speeds.iter().map(|fan| &fan.source))
    {
      assert_eq!(source, "Nuvoton / Super I/O");
    }
    assert!(partial_failures.is_empty());
    assert_eq!(active.fan_support(), SensorSupport::Supported);
  }

  #[test]
  fn monitoring_gate_failure_makes_every_input_unavailable() {
    for monitoring in [0x00, 0x09] {
      let chip = FakeSmiovtChip::nct6776();
      chip.set_hm(0, 0x40, monitoring);
      let active = active(chip);

      let failure = active.collect_sample_unlocked().unwrap_err();

      let error = failure.to_string();
      assert!(error.starts_with(HARDWARE_STATE_PREFIX), "{error}");
      assert!(
        error.contains(&format!("0x40=0x{monitoring:02X}")),
        "{error}"
      );
      assert!(!failure.offers_pawnio_guidance());
      // Nothing after the gate is read.
      assert_eq!(active.client.hm_reads.borrow().as_slice(), &[(0, 0x40)]);
    }
  }

  #[test]
  fn nct6775_ignores_int_clear() {
    let chip = FakeSmiovtChip::with_chip_id(0xB473);
    chip.set_hm(0, 0x40, 0x09);

    assert!(active(chip).collect_sample_unlocked().is_ok());
  }

  #[test]
  fn unrouted_fans_are_neither_read_nor_reported() {
    let chip = FakeSmiovtChip::nct6776();
    // FANINC2 clear; AUXFANIN0 on the floppy default; AUXFANIN1 moved to
    // GP72 with CR27 bit 7 clear; AUXFANIN2 on its CR1C pin.
    chip.set_hm(0, 0x4D, 0x01);
    chip.set_config(0x24, 0x40);
    chip.set_config(0x30, 0x41);
    let active = active(chip);

    let (sample, _) = active.collect_sample_unlocked().unwrap();

    assert_eq!(fans(&sample), [("Fan 1", Some(2500)), ("Fan 5", Some(0))]);
    let reads = active.client.hm_reads_after_gate();
    for register in [0x58, 0x59, 0x5A, 0x5B, 0x5C, 0x5D] {
      assert!(!reads.contains(&(6, register)), "0x{register:02X}");
    }
  }

  #[test]
  fn gpio_routed_aux_fans_follow_cr27_bit_7() {
    let chip = FakeSmiovtChip::nct6776();
    chip.set_hm(0, 0x4D, 0x00);
    chip.set_config(0x1C, 0x00);
    chip.set_config(0x24, 0x40);
    chip.set_config(0x27, 0x80);
    chip.set_config(0x30, 0xE1);
    let active = active(chip);

    let (sample, _) = active.collect_sample_unlocked().unwrap();

    assert_eq!(
      fans(&sample),
      [("Fan 3", Some(0)), ("Fan 4", Some(165)), ("Fan 5", Some(0))]
    );
  }

  #[test]
  fn implausible_preferred_slot_is_unavailable_without_failing_the_sample() {
    let chip = FakeSmiovtChip::nct6776();
    chip.set_hm(1, 0x50, 0x7E);
    let active = active(chip);

    let (sample, partial_failures) = active.collect_sample_unlocked().unwrap();

    assert_eq!(temperatures(&sample), [("SYSTIN", 32.5), ("AUXTIN", 35.0)]);
    assert_eq!(sample.fan_speeds.len(), 5);
    assert_eq!(
      partial_failures,
      ["CPUTIN (SMIOVT2) returned implausible value 126.5 C and is unavailable"]
    );
  }

  #[test]
  fn a_failed_read_aborts_the_sample_without_partial_values() {
    for failed in [(1, 0x51), (6, 0x2E), (6, 0x5D)] {
      let chip = FakeSmiovtChip::nct6776();
      chip.failed_hm_read.set(Some(failed));
      let active = active(chip);

      let failure = active.collect_sample_unlocked().unwrap_err();

      let error = failure.to_string();
      assert!(error.starts_with(NCT6776_FAILURE_PREFIX), "{error}");
      assert!(failure.offers_pawnio_guidance());
    }
  }

  #[test]
  fn sample_logs_partial_failures_once_per_run() {
    let chip = FakeSmiovtChip::nct6776();
    chip.set_hm(2, 0x50, 0x80);
    let mut active = active(chip);

    assert_eq!(
      active.sample_unlocked_logged().unwrap().temperatures.len(),
      2
    );
    assert!(active.partial_failure_logged);

    active.client.set_hm(2, 0x50, 0x23);
    assert_eq!(
      active.sample_unlocked_logged().unwrap().temperatures.len(),
      3
    );
    assert!(!active.partial_failure_logged);
  }

  // --- Spec "Status registers that clear on read" and "Safety notes" ---

  #[test]
  fn read_guard_refuses_never_read_registers_before_any_port_io() {
    let chip = FakeSmiovtChip::nct6776();
    chip.bars_authorized.set(true);
    let mut reader = HmBankReader::new(&chip, HM_BASE);

    for (bank, register) in [
      (0, 0x41),
      (0, 0x42),
      (0, 0x45),
      (4, 0x41),
      (4, 0x50),
      (3, 0x00),
      (5, 0x00),
      (7, 0x00),
    ] {
      let error = reader.read(bank, register).unwrap_err();
      assert!(error.starts_with("refused read"), "{error}");
    }
    assert!(chip.bank_select_writes.borrow().is_empty());
    assert!(chip.hm_reads.borrow().is_empty());
  }

  #[test]
  fn detection_and_sampling_write_only_the_allowed_values() {
    // The fake panics on any write outside the allow-lists; a full detection
    // and sample on both rows must complete.
    for raw_id in [0xC333, 0xB472] {
      let active = active(FakeSmiovtChip::with_chip_id(raw_id));
      active.collect_sample_unlocked().unwrap();
      for value in active.client.bank_select_writes.borrow().iter() {
        assert!(SMIOVT_ALLOWED_BANK_SELECT_VALUES.contains(value));
        assert_eq!(value & 0x80, 0, "sampling selects banks with bit 7 clear");
      }
    }
  }
}
