//! `LpcIO` discovery and sampling for the Fintek F71882 and F71889 hardware
//! monitors.
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-fintek-f71882-f71889.md` revision 1
//! - `docs/specs/sensors/superio-access.md` revision 4 (configuration keys,
//!   slot mapping, chip-ID reads, absent-ID classification, the slot-1 exit
//!   withhold IDs and the ISA mutex)
//! - `docs/specs/sensors/pawnio-interface.md` revision 9 (`LpcIO` contract,
//!   port authorization and mutex conventions)
//! - `docs/specs/sensors/superio-nuvoton-nct67xx.md` revision 9 and
//!   `docs/specs/sensors/superio-nuvoton-nct6775-nct6776-nct610x.md`
//!   revision 1 (the shared procedure: BAR discovery after base discovery and
//!   the unavailable-input rule for label-keyed consumers)
//!
//! No other external sensor source was used.

use super::super::super::client::{ACCESS_ISABUS_MUTEX, MutexError, NamedMutex};
use super::super::access::{chip_id, is_absent_id, should_withhold_ite_exit, slot_ports};
use super::super::fintek::{
  F71889_HM_T1_SOURCE_SELECT_REGISTER, F71889_HM_VENDOR_ID_HIGH_REGISTER,
  F71889_HM_VENDOR_ID_LOW_REGISTER, FINTEK_HARDWARE_MONITOR_LDN,
  FINTEK_HM_CONFIGURATION_REGISTER, FINTEK_HM_DIODE_OPEN_REGISTER,
  FINTEK_HM_TEMP1_TYPE_REGISTER, FINTEK_MAX_FAN_INPUTS,
  FINTEK_SOFTWARE_POWER_DOWN_REGISTER, FINTEK_TEMPERATURE_INPUTS,
  FINTEK_VENDOR_ID_HIGH_REGISTER, FINTEK_VENDOR_ID_LOW_REGISTER, FintekHmPorts,
  FintekMonitorState, FintekProfile, FintekRoutingRegisters, FintekTemp1Source,
  decode_fintek_fan_rpm, decode_fintek_temperature, fintek_fan_count, fintek_hm_ports,
  fintek_monitor_state, fintek_profile, fintek_routed_fans, fintek_temp1_source,
  is_f71889_temp1_source_diode, is_fintek_diode_open, is_fintek_hardware_monitor_enabled,
  is_fintek_hardware_monitor_powered_down, is_fintek_vendor_id,
};
use super::super::nuvoton::fan_speed_status_from_rpm;
use super::{
  CHIP_ID_HIGH_REGISTER, CHIP_ID_LOW_REGISTER, ChipFailure, HM_BASE_HIGH_REGISTER,
  HM_BASE_LOW_REGISTER, ISABUS_MUTEX_TIMEOUT, LDN_ACTIVATION_REGISTER,
  LOGICAL_DEVICE_SELECT_REGISTER, LpcIoOps, MotherboardSensorError, SlotProbe,
  SuperIoVendor, enter_nuvoton, exit_nuvoton,
};
use crate::models::{
  FanSpeedStatus, MotherboardFanSpeed, MotherboardSensorSample, MotherboardTemperature,
  SensorSupport,
};
use crate::{log_debug, log_warn};

const FINTEK_CONFIGURATION_EXIT_FAILURE_CONTEXT: &str = "Fintek config exit failed";

/// A Fintek chip found on one `LpcIO` slot, cached while the ISA mutex is
/// still held ("Detection" step 8).
#[derive(Debug)]
pub(super) struct DetectedFintekSlot {
  slot: u8,
  raw_chip_id: u16,
  /// HM base B as read from `CR60`/`CR61`, for diagnostics.
  hm_base: u16,
  ports: FintekHmPorts,
  profile: &'static FintekProfile,
  /// Routed state of each profile fan, by position.
  routed_fans: [bool; FINTEK_MAX_FAN_INPUTS],
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
  /// Every Fintek row is Experimental.
  fn fintek(profile: &FintekProfile, detail: impl Into<String>) -> Self {
    Self {
      vendor: SuperIoVendor::Fintek,
      chip_label: profile.chip_label,
      experimental: profile.is_experimental(),
      hardware_state: false,
      requires_rediscovery: false,
      detail: detail.into(),
    }
  }
}

/// Read one HM register: an index-port write followed by one data-port read.
/// A register outside the profile's documented reads is refused before any
/// port I/O, so the clearing registers can never be read.
fn read_fintek_hm_byte(
  client: &impl LpcIoOps,
  ports: FintekHmPorts,
  profile: &FintekProfile,
  register: u8,
) -> Result<u8, String> {
  if !profile.is_allowed_hm_read(register) {
    return Err(format!("refused read of HM register 0x{register:02X}"));
  }
  client.pio_outb(ports.index, register)?;
  client.pio_inb(ports.data)
}

/// Detection steps 3-6 for a matched chip, in configuration mode.
fn discover_matched_chip(
  client: &impl LpcIoOps,
  slot: u8,
  raw_chip_id: u16,
  profile: &'static FintekProfile,
) -> Result<DetectedFintekSlot, DiscoveryFailure> {
  let read_global =
    |register, context| client.superio_inb(register).map_err(transport(context));

  // Step 3: Super I/O vendor ID, the CR25 power-down bit, then the routing
  // registers. None of them is written.
  let vendor_high = read_global(FINTEK_VENDOR_ID_HIGH_REGISTER, "CR23 read failed")?;
  let vendor_low = read_global(FINTEK_VENDOR_ID_LOW_REGISTER, "CR24 read failed")?;
  if !is_fintek_vendor_id(vendor_high, vendor_low) {
    return Err(DiscoveryFailure::HardwareState(format!(
      "unexpected Super I/O vendor ID 0x{vendor_high:02X}{vendor_low:02X} (CR23/CR24)"
    )));
  }
  let cr25 = read_global(FINTEK_SOFTWARE_POWER_DOWN_REGISTER, "CR25 read failed")?;
  if is_fintek_hardware_monitor_powered_down(cr25) {
    return Err(DiscoveryFailure::HardwareState(format!(
      "hardware monitor is powered down (CR25=0x{cr25:02X}, bit 4 SOFTPD_HM set)"
    )));
  }
  // A failed routing read leaves the fans that depend on it unavailable until
  // rediscovery; it does not stop detection.
  let mut routing = FintekRoutingRegisters::default();
  for &register in profile.routing_registers() {
    routing.record(register, client.superio_inb(register).ok());
  }

  // Step 4: LDN 4 enabled and the base gate. CR30 and CR60/61 are never
  // written.
  client
    .superio_outb(LOGICAL_DEVICE_SELECT_REGISTER, FINTEK_HARDWARE_MONITOR_LDN)
    .map_err(transport("LDN 4 selection failed"))?;
  let cr30 = read_global(LDN_ACTIVATION_REGISTER, "CR30 read failed")?;
  if !is_fintek_hardware_monitor_enabled(cr30) {
    return Err(DiscoveryFailure::HardwareState(format!(
      "hardware monitor logical device is disabled (CR30=0x{cr30:02X})"
    )));
  }
  let base_high = read_global(HM_BASE_HIGH_REGISTER, "CR60 read failed")?;
  let base_low = read_global(HM_BASE_LOW_REGISTER, "CR61 read failed")?;
  let hm_base = u16::from_be_bytes([base_high, base_low]);
  let ports = fintek_hm_ports(hm_base).ok_or_else(|| {
    DiscoveryFailure::HardwareState(format!(
      "unusable hardware-monitor base 0x{hm_base:04X}"
    ))
  })?;

  // Step 5: the last module-state change of the sequence. No slot selection
  // follows it once this slot passes.
  client
    .find_lpc_bars()
    .map_err(transport("LpcIO BAR discovery failed"))?;

  // Step 6 (F71889): HM vendor ID through the HM ports. A port failure asks
  // for a fresh detection; a vendor ID that reads but differs is a hardware
  // state.
  if profile.checks_hm_vendor_id() {
    let read_vendor_byte = |register| {
      read_fintek_hm_byte(client, ports, profile, register).map_err(|reason| {
        DiscoveryFailure::PortAccess(format!("HM vendor ID read failed: {reason}"))
      })
    };
    let high = read_vendor_byte(F71889_HM_VENDOR_ID_HIGH_REGISTER)?;
    let low = read_vendor_byte(F71889_HM_VENDOR_ID_LOW_REGISTER)?;
    if !is_fintek_vendor_id(high, low) {
      return Err(DiscoveryFailure::HardwareState(format!(
        "unexpected hardware-monitor vendor ID 0x{high:02X}{low:02X} (HM 0x5D/0x5E)"
      )));
    }
  }

  Ok(DetectedFintekSlot {
    slot,
    raw_chip_id,
    hm_base,
    ports,
    profile,
    routed_fans: fintek_routed_fans(profile, &routing),
  })
}

/// Match the chip ID read in step 2 and run steps 3-6 on a match.
fn discover_identified_chip(
  client: &impl LpcIoOps,
  slot: u8,
  id_high: u8,
  id_low: u8,
) -> Result<Option<DetectedFintekSlot>, MotherboardSensorError> {
  if is_absent_id(id_high, id_low) {
    return Ok(None);
  }
  let raw_chip_id = chip_id(id_high, id_low);
  let Some(profile) = fintek_profile(raw_chip_id) else {
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
          ChipFailure::fintek(profile, with_id(reason)).into()
        }
        DiscoveryFailure::PortAccess(reason) => {
          ChipFailure::fintek(profile, with_id(reason))
            .requiring_rediscovery()
            .into()
        }
        DiscoveryFailure::HardwareState(reason) => {
          ChipFailure::fintek(profile, with_id(reason))
            .hardware_state()
            .into()
        }
      }
    })
}

pub(super) struct ActiveFintekSensors<C: LpcIoOps> {
  client: C,
  detected: DetectedFintekSlot,
  /// Whether the current run of partial samples was already logged.
  partial_failure_logged: bool,
}

impl<C: LpcIoOps> ActiveFintekSensors<C> {
  pub(super) fn new(client: C, detected: DetectedFintekSlot) -> Self {
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
  ) -> Result<Option<DetectedFintekSlot>, MotherboardSensorError> {
    Self::probe_slot(client, slot).result
  }

  /// "Detection" for one slot. The caller holds the ISA mutex across every
  /// slot attempt and caches a returned path before releasing it. Once the
  /// slot is selected, configuration mode is exited with `0xAA` on every
  /// path, including a failed entry key write, except after a slot-1 chip ID
  /// that `superio-access.md` withholds the exit for (step 2).
  pub(super) fn probe_slot(client: &C, slot: u8) -> SlotProbe<DetectedFintekSlot> {
    let Some((index_port, _data_port)) = slot_ports(slot) else {
      return SlotProbe::exited(Ok(None));
    };

    if let Err(reason) = client.select_lpc_slot(slot as u64) {
      // Nothing was written to the configuration ports.
      return SlotProbe::exited(Err(reason.into()));
    }
    // The Fintek keys are the Winbond/Nuvoton-lineage `0x87`, `0x87` / `0xAA`.
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

    let id_bytes = client
      .superio_inb(CHIP_ID_HIGH_REGISTER)
      .and_then(|high| Ok((high, client.superio_inb(CHIP_ID_LOW_REGISTER)?)));
    if let Ok((id_high, id_low)) = id_bytes
      && should_withhold_ite_exit(slot, chip_id(id_high, id_low))
    {
      // Step 2 exception: no exit write of any kind on this slot. The chip is
      // left in configuration mode, so no later probe writes to this slot in
      // this attempt. None of these IDs is in the Fintek table.
      log_debug!(
        "fintek_super_io_slot1_exit_withheld",
        "windows::super_io_motherboard::ActiveFintekSensors::probe_slot",
        Some(format!(
          "configuration exit intentionally skipped on slot {slot} for raw chip ID \
           0x{id_high:02X}{id_low:02X}"
        ))
      );
      return SlotProbe::unconfirmed(Ok(None));
    }
    let result =
      id_bytes
        .map_err(MotherboardSensorError::from)
        .and_then(|(id_high, id_low)| {
          discover_identified_chip(client, slot, id_high, id_low)
        });
    let exit_result = exit_nuvoton(client, index_port);
    let configuration_exited = exit_result.is_ok();

    let result = match (result, exit_result) {
      (Ok(value), Ok(())) => Ok(value),
      (Ok(Some(detected)), Err(exit_error)) => Err(
        ChipFailure::fintek(
          detected.profile,
          format!(
            "{FINTEK_CONFIGURATION_EXIT_FAILURE_CONTEXT}: {exit_error} (raw chip ID 0x{:04X})",
            detected.raw_chip_id
          ),
        )
        .requiring_rediscovery()
        .into(),
      ),
      (Ok(None), Err(exit_error)) => {
        Err(format!("{FINTEK_CONFIGURATION_EXIT_FAILURE_CONTEXT}: {exit_error}").into())
      }
      (Err(reason), Ok(())) => Err(reason),
      (Err(reason), Err(exit_error)) => {
        Err(reason.with_exit_failure(format!("; exit failed: {exit_error}"), true))
      }
    };
    SlotProbe {
      result,
      configuration_exited,
    }
  }

  /// FAN1 and FAN2 are dedicated tachometer inputs on both rows.
  pub(super) fn fan_support(&self) -> SensorSupport {
    if self.detected.profile.fans.is_empty() {
      SensorSupport::Unsupported
    } else {
      SensorSupport::Supported
    }
  }

  pub(super) fn diagnostic_summary(&self) -> String {
    let detected = &self.detected;
    let routed_fans = detected
      .profile
      .fans
      .iter()
      .zip(detected.routed_fans)
      .filter_map(|(fan, routed)| routed.then_some(fan.label))
      .collect::<Vec<_>>()
      .join(",");
    format!(
      "active chip={} raw_id=0x{:04X} slot={} hm_base=0x{:04X} hm_ports=0x{:04X}/0x{:04X} routed_fans={}",
      detected.profile.chip_label,
      detected.raw_chip_id,
      detected.slot,
      detected.hm_base,
      detected.ports.index,
      detected.ports.data,
      routed_fans
    )
  }

  pub(super) fn sample(
    &mut self,
  ) -> Result<MotherboardSensorSample, MotherboardSensorError> {
    self
      .sample_with_lock(|| NamedMutex::acquire(ACCESS_ISABUS_MUTEX, ISABUS_MUTEX_TIMEOUT))
  }

  /// One sample under the ISA mutex taken by `acquire`. A failed acquisition
  /// skips the sample before any port I/O.
  fn sample_with_lock<G>(
    &mut self,
    acquire: impl FnOnce() -> Result<G, MutexError>,
  ) -> Result<MotherboardSensorSample, MotherboardSensorError> {
    let _mutex = acquire()?;
    let (sample, partial_failures) = self.collect_sample_unlocked()?;
    if partial_failures.is_empty() {
      self.partial_failure_logged = false;
    } else if !self.partial_failure_logged {
      self.partial_failure_logged = true;
      log_warn!(
        "fintek_motherboard_sensor_partial_sample",
        "windows::super_io_motherboard::ActiveFintekSensors::sample",
        Some(format!(
          "{}: {}",
          self.detected.profile.chip_label,
          partial_failures.join("; ")
        ))
      );
    }
    Ok(sample)
  }

  fn read_hm(&self, register: u8) -> Result<u8, String> {
    read_fintek_hm_byte(
      &self.client,
      self.detected.ports,
      self.detected.profile,
      register,
    )
  }

  /// "Sampling" through the HM ports only: no configuration entry and no
  /// slot selection. The `Vec<String>` carries inputs that are unavailable
  /// without failing the sample. An unavailable input is omitted; consumers
  /// of motherboard readings key them by label, so the others keep their
  /// identities.
  fn collect_sample_unlocked(
    &self,
  ) -> Result<(MotherboardSensorSample, Vec<String>), MotherboardSensorError> {
    let profile = self.detected.profile;

    // Step 1. This is the first HM access of the sample, so a failure here
    // means the cached port authorization may be gone (any slot selection on
    // the handle clears it): the path is dropped until detection reruns.
    let hm01 = self
      .read_hm(FINTEK_HM_CONFIGURATION_REGISTER)
      .map_err(|reason| {
        ChipFailure::fintek(profile, format!("HM 0x01 read failed: {reason}"))
          .requiring_rediscovery()
      })?;
    let (temperatures_started, fans_started) = match fintek_monitor_state(hm01) {
      FintekMonitorState::PoweredDown => {
        return Err(
          ChipFailure::fintek(
            profile,
            format!("hardware monitor is powered down (HM 0x01=0x{hm01:02X}, bit 2 set)"),
          )
          .hardware_state()
          .into(),
        );
      }
      FintekMonitorState::Running { temperatures, fans } => (temperatures, fans),
    };

    let mut sample = MotherboardSensorSample::default();
    let mut partial_failures = Vec::new();
    let mut had_io_failure = false;

    // Step 2.
    if temperatures_started {
      had_io_failure |=
        self.read_temperatures(&mut sample.temperatures, &mut partial_failures);
    } else {
      partial_failures.push(format!(
        "temperatures are in standby (HM 0x01=0x{hm01:02X}, bit 0 clear)"
      ));
    }
    // Step 3.
    if fans_started {
      had_io_failure |= self.read_fans(&mut sample.fan_speeds, &mut partial_failures);
    } else {
      partial_failures.push(format!(
        "fans are in standby (HM 0x01=0x{hm01:02X}, bit 1 clear)"
      ));
    }

    if sample.temperatures.is_empty() && sample.fan_speeds.is_empty() {
      let failure = ChipFailure::fintek(
        profile,
        format!("no usable sensor readings: {}", partial_failures.join("; ")),
      );
      return Err(
        if had_io_failure {
          failure
        } else {
          failure.hardware_state()
        }
        .into(),
      );
    }
    Ok((sample, partial_failures))
  }

  /// TEMP1's source gate: `Ok(None)` when TEMP1 reads the D1+ diode,
  /// `Ok(Some(reason))` when another source feeds it, `Err` on a failed read.
  fn temp1_source_rejection(&self) -> Result<Option<String>, String> {
    let chip = self.detected.profile.chip;
    let hm0a = self
      .read_hm(FINTEK_HM_TEMP1_TYPE_REGISTER)
      .map_err(|reason| format!("HM 0x0A read failed: {reason}"))?;
    match fintek_temp1_source(chip, hm0a) {
      FintekTemp1Source::ExternalDiode => Ok(None),
      FintekTemp1Source::Other => Ok(Some(format!(
        "TEMP1 is not sourced from the D1+ diode (HM 0x0A=0x{hm0a:02X})"
      ))),
      FintekTemp1Source::SourceSelected => {
        let hm7f = self
          .read_hm(F71889_HM_T1_SOURCE_SELECT_REGISTER)
          .map_err(|reason| format!("HM 0x7F read failed: {reason}"))?;
        Ok((!is_f71889_temp1_source_diode(hm7f)).then(|| {
          format!(
            "TEMP1 is not sourced from the D1+ diode (HM 0x0A=0x{hm0a:02X}, 0x7F=0x{hm7f:02X})"
          )
        }))
      }
    }
  }

  /// Step 2: `0x6F`, then `0x0A` (and `0x7F`), then each eligible reading.
  /// Returns whether any read failed.
  fn read_temperatures(
    &self,
    temperatures: &mut Vec<MotherboardTemperature>,
    partial_failures: &mut Vec<String>,
  ) -> bool {
    let profile = self.detected.profile;
    let diode_open = match self.read_hm(FINTEK_HM_DIODE_OPEN_REGISTER) {
      Ok(value) => value,
      Err(reason) => {
        partial_failures.push(format!(
          "temperatures are unavailable: HM 0x6F read failed: {reason}"
        ));
        return true;
      }
    };
    let mut had_io_failure = false;
    let temp1_rejection = self.temp1_source_rejection().unwrap_or_else(|reason| {
      had_io_failure = true;
      Some(format!("TEMP1 is unavailable: {reason}"))
    });

    for input in &FINTEK_TEMPERATURE_INPUTS {
      if input.source_gated
        && let Some(reason) = &temp1_rejection
      {
        partial_failures.push(reason.clone());
        continue;
      }
      if is_fintek_diode_open(diode_open, input) {
        partial_failures.push(format!(
          "{} reports an open diode (HM 0x6F=0x{diode_open:02X})",
          input.label
        ));
        continue;
      }
      match self.read_hm(input.register) {
        Ok(raw) => match decode_fintek_temperature(profile.chip, raw) {
          Some(temperature) => temperatures.push(MotherboardTemperature {
            name: input.label.to_string(),
            temperature,
            source: profile.source_label.to_string(),
          }),
          None => partial_failures.push(format!(
            "{} returned an out-of-range raw value 0x{raw:02X} and is unavailable",
            input.label
          )),
        },
        Err(reason) => {
          had_io_failure = true;
          partial_failures.push(format!("{} read failed: {reason}", input.label));
        }
      }
    }
    had_io_failure
  }

  /// Step 3: MSB then LSB of each routed input. An unrouted input is never
  /// read. Returns whether any read failed.
  fn read_fans(
    &self,
    fan_speeds: &mut Vec<MotherboardFanSpeed>,
    partial_failures: &mut Vec<String>,
  ) -> bool {
    let profile = self.detected.profile;
    let mut had_io_failure = false;
    for (fan, routed) in profile.fans.iter().zip(self.detected.routed_fans) {
      if !routed {
        continue;
      }
      let count = self.read_hm(fan.msb_register).and_then(|msb| {
        let lsb = self.read_hm(fan.lsb_register)?;
        Ok(fintek_fan_count(msb, lsb))
      });
      match count {
        Ok(count) => {
          let rpm = decode_fintek_fan_rpm(count);
          fan_speeds.push(MotherboardFanSpeed {
            name: fan.label.to_string(),
            rpm,
            status: rpm.map_or(FanSpeedStatus::Invalid, fan_speed_status_from_rpm),
            source: profile.source_label.to_string(),
          });
        }
        Err(reason) => {
          had_io_failure = true;
          partial_failures.push(format!("{} read failed: {reason}", fan.label));
        }
      }
    }
    had_io_failure
  }
}

#[cfg(test)]
mod tests {
  use std::cell::{Cell, RefCell};
  use std::collections::HashMap;

  use super::super::super::fintek::FINTEK_NEVER_READ_HM_REGISTERS;
  use super::super::ActiveMotherboardSensors;
  use super::*;

  /// One observable `LpcIO` call, in order.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  enum Op {
    SelectSlot(u8),
    FindBars,
    /// A write to a configuration index port (`0x2E` / `0x4E`).
    ConfigKey(u16, u8),
    ConfigRead(u8),
    ConfigWrite(u8, u8),
    HmIndex(u16, u8),
    HmRead(u16, u8),
  }

  /// A synthetic Fintek-style chip: `0x87`, `0x87` enters configuration mode
  /// and `0xAA` leaves it. All fixtures are synthetic.
  struct FakeChip {
    chip_id: u16,
    global: RefCell<HashMap<u8, u8>>,
    ldn4: RefCell<HashMap<u8, u8>>,
    hm: RefCell<HashMap<u8, u8>>,
    in_configuration: Cell<bool>,
    pending_entry_key: Cell<bool>,
    selected_ldn: Cell<Option<u8>>,
  }

  impl FakeChip {
    /// "Synthetic cases" fixture 1: F71882 with SPI enabled at base `0x0295`.
    fn f71882() -> Self {
      Self {
        chip_id: 0x0541,
        global: RefCell::new(HashMap::from([
          (0x23, 0x19),
          (0x24, 0x34),
          (0x25, 0x00),
          (0x27, 0x50),
          (0x28, 0x00),
          (0x2B, 0x00),
        ])),
        ldn4: RefCell::new(HashMap::from([(0x30, 0x01), (0x60, 0x02), (0x61, 0x95)])),
        hm: RefCell::new(HashMap::from([
          (0x01, 0x03),
          (0x0A, 0x00),
          (0x6F, 0x00),
          (0x72, 0x28),
          (0x74, 0x1E),
          (0x76, 0x7D),
          (0xA0, 0x02),
          (0xA1, 0xEE),
          (0xB0, 0x0F),
          (0xB1, 0xFF),
          (0xC0, 0x00),
          (0xC1, 0x00),
          (0xD0, 0x03),
          (0xD1, 0xE8),
        ])),
        in_configuration: Cell::new(false),
        pending_entry_key: Cell::new(false),
        selected_ldn: Cell::new(None),
      }
    }

    /// F71889 at base `0x0295`, FAN3 routed, TEMP1 on the D1+ diode
    /// (`0x0A` = `0x01`), HM vendor ID `0x19`/`0x34`.
    fn f71889() -> Self {
      let chip = Self {
        chip_id: 0x0723,
        ..Self::f71882()
      };
      chip.global.borrow_mut().remove(&0x27);
      chip.global.borrow_mut().remove(&0x28);
      chip.set_hm(0x0A, 0x01);
      chip.set_hm(0x5D, 0x19);
      chip.set_hm(0x5E, 0x34);
      chip.set_hm(0x7F, 0x00);
      chip.set_hm(0x72, 0xD8);
      chip
    }

    /// The F71882 register image under another raw chip ID.
    fn with_chip_id(chip_id: u16) -> Self {
      Self {
        chip_id,
        ..Self::f71882()
      }
    }

    fn set_global(&self, register: u8, value: u8) {
      self.global.borrow_mut().insert(register, value);
    }

    fn set_base(&self, base: u16) {
      let [high, low] = base.to_be_bytes();
      self.ldn4.borrow_mut().insert(0x60, high);
      self.ldn4.borrow_mut().insert(0x61, low);
    }

    fn set_hm(&self, register: u8, value: u8) {
      self.hm.borrow_mut().insert(register, value);
    }

    fn base(&self) -> u16 {
      let ldn4 = self.ldn4.borrow();
      u16::from_be_bytes([ldn4[&0x60], ldn4[&0x61]])
    }
  }

  /// A fake `LpcIO` handle with an optional chip behind each slot. It applies
  /// the module's port authorization (S11) and panics on any write outside
  /// the spec's allowed writes and on any never-read HM register.
  struct FakeBus {
    chips: [Option<FakeChip>; 2],
    selected_slot: Cell<Option<u8>>,
    /// Windows recorded by the last `ioctl_find_bars`, and their slot.
    bars: RefCell<Vec<u16>>,
    bars_slot: Cell<Option<u8>>,
    hm_index: Cell<Option<u8>>,
    ops: RefCell<Vec<Op>>,
    failed_hm_read: Cell<Option<u8>>,
    fail_config_exit: Cell<bool>,
    fail_second_entry_key: Cell<bool>,
  }

  impl FakeBus {
    fn new(slot0: Option<FakeChip>, slot1: Option<FakeChip>) -> Self {
      Self {
        chips: [slot0, slot1],
        selected_slot: Cell::new(None),
        bars: RefCell::new(Vec::new()),
        bars_slot: Cell::new(None),
        hm_index: Cell::new(None),
        ops: RefCell::new(Vec::new()),
        failed_hm_read: Cell::new(None),
        fail_config_exit: Cell::new(false),
        fail_second_entry_key: Cell::new(false),
      }
    }

    fn on_slot(slot: u8, chip: FakeChip) -> Self {
      match slot {
        0 => Self::new(Some(chip), None),
        _ => Self::new(None, Some(chip)),
      }
    }

    fn chip(&self, slot: u8) -> &FakeChip {
      self.chips[slot as usize].as_ref().expect("chip on slot")
    }

    fn selected_chip(&self) -> Option<&FakeChip> {
      let slot = self.selected_slot.get().expect("a slot is selected");
      self.chips[slot as usize].as_ref()
    }

    fn record(&self, op: Op) {
      self.ops.borrow_mut().push(op);
    }

    fn ops(&self) -> Vec<Op> {
      self.ops.borrow().clone()
    }

    fn hm_reads(&self) -> Vec<u8> {
      self
        .ops()
        .into_iter()
        .filter_map(|op| match op {
          Op::HmRead(_, register) => Some(register),
          _ => None,
        })
        .collect()
    }

    fn count(&self, wanted: Op) -> usize {
      self.ops().into_iter().filter(|op| *op == wanted).count()
    }

    fn has_hm_access(&self) -> bool {
      self
        .ops()
        .iter()
        .any(|op| matches!(op, Op::HmIndex(..) | Op::HmRead(..)))
    }

    fn require_authorized(&self, port: u16) -> Result<(), String> {
      if self.bars.borrow().contains(&(port & !0x0007)) {
        Ok(())
      } else {
        Err(format!("port 0x{port:04X} is not authorized"))
      }
    }
  }

  impl LpcIoOps for FakeBus {
    fn select_lpc_slot(&self, slot: u64) -> Result<(), String> {
      let slot = u8::try_from(slot).expect("slot fits u8");
      self.record(Op::SelectSlot(slot));
      // S11: selecting a slot resets the module state and its BAR list.
      self.bars.borrow_mut().clear();
      self.bars_slot.set(None);
      self.selected_slot.set(Some(slot));
      Ok(())
    }

    fn find_lpc_bars(&self) -> Result<(), String> {
      self.record(Op::FindBars);
      let chip = self.selected_chip().expect("BARs found on an empty slot");
      assert!(
        chip.in_configuration.get(),
        "BARs found outside configuration mode"
      );
      assert_eq!(
        chip.selected_ldn.get(),
        Some(0x04),
        "BARs found before LDN 4"
      );
      let ops = self.ops();
      assert!(
        ops.contains(&Op::ConfigRead(0x60)) && ops.contains(&Op::ConfigRead(0x61)),
        "BARs found before base discovery"
      );
      // S11: record a non-zero, non-0xFFFF value of at least 0x100, rounding
      // a `101b` value down to its 8-byte boundary.
      let base = chip.base();
      if base != 0x0000 && base != 0xFFFF && base >= 0x0100 {
        let recorded = if base & 0x0007 == 0x0005 {
          base & !0x0007
        } else {
          base
        };
        if !(0x0CF8..=0x0CFF).contains(&(recorded & !0x0007)) {
          self.bars.borrow_mut().push(recorded);
        }
      }
      self.bars_slot.set(self.selected_slot.get());
      Ok(())
    }

    fn pio_inb(&self, port: u16) -> Result<u8, String> {
      self.require_authorized(port)?;
      assert_eq!(port & 0x0007, 0x0006, "read of a port other than HM data");
      let register = self.hm_index.get().expect("HM index selected first");
      assert!(
        !FINTEK_NEVER_READ_HM_REGISTERS.contains(&register),
        "read of never-read HM register 0x{register:02X}"
      );
      self.record(Op::HmRead(port, register));
      if self.failed_hm_read.get() == Some(register) {
        return Err(format!("simulated HM 0x{register:02X} failure"));
      }
      let slot = self.bars_slot.get().expect("BARs belong to a slot");
      let chip = self.chips[slot as usize]
        .as_ref()
        .expect("chip on the BAR slot");
      Ok(chip.hm.borrow().get(&register).copied().unwrap_or(0x00))
    }

    fn pio_outb(&self, port: u16, value: u8) -> Result<(), String> {
      if port == 0x2E || port == 0x4E {
        let slot = self.selected_slot.get().expect("a slot is selected");
        let expected = slot_ports(slot).expect("known slot").0;
        assert_eq!(port, expected, "configuration write to another slot's pair");
        self.record(Op::ConfigKey(port, value));
        let Some(chip) = self.selected_chip() else {
          return Ok(());
        };
        match value {
          0x87 if chip.pending_entry_key.get() => {
            if self.fail_second_entry_key.get() {
              return Err("simulated entry key failure".to_string());
            }
            chip.pending_entry_key.set(false);
            chip.in_configuration.set(true);
          }
          0x87 => chip.pending_entry_key.set(true),
          0xAA => {
            chip.pending_entry_key.set(false);
            if self.fail_config_exit.get() {
              return Err("simulated config exit failure".to_string());
            }
            chip.in_configuration.set(false);
            chip.selected_ldn.set(None);
          }
          // The ITE probe's keys, which this chip ignores.
          _ => chip.pending_entry_key.set(false),
        }
        return Ok(());
      }

      self.require_authorized(port)?;
      // "Sampling" step 4: no HM data-port write ever occurs.
      assert_eq!(port & 0x0007, 0x0005, "HM data-port write 0x{value:02X}");
      self.record(Op::HmIndex(port, value));
      self.hm_index.set(Some(value));
      Ok(())
    }

    fn superio_inb(&self, register: u8) -> Result<u8, String> {
      self.record(Op::ConfigRead(register));
      let Some(chip) = self.selected_chip() else {
        return Ok(0xFF);
      };
      if !chip.in_configuration.get() {
        return Ok(0xFF);
      }
      Ok(match register {
        0x20 => (chip.chip_id >> 8) as u8,
        0x21 => chip.chip_id as u8,
        0x30 | 0x60 | 0x61 => {
          assert_eq!(
            chip.selected_ldn.get(),
            Some(0x04),
            "LDN register before LDN 4"
          );
          chip.ldn4.borrow()[&register]
        }
        0x23 | 0x24 | 0x25 | 0x27 | 0x28 | 0x2B => {
          chip.global.borrow().get(&register).copied().unwrap_or(0x00)
        }
        _ => panic!("read of undocumented configuration register 0x{register:02X}"),
      })
    }

    fn superio_outb(&self, register: u8, value: u8) -> Result<(), String> {
      self.record(Op::ConfigWrite(register, value));
      match (register, value) {
        (0x07, 0x04) => {
          if let Some(chip) = self.selected_chip()
            && chip.in_configuration.get()
          {
            chip.selected_ldn.set(Some(value));
          }
        }
        // The ITE probe's exit on the same handle; never from this document.
        (0x02, 0x02) => {}
        _ => panic!(
          "configuration write 0x{register:02X}=0x{value:02X} outside the allow-list"
        ),
      }
      Ok(())
    }
  }

  /// Lets `ActiveMotherboardSensors::discover` borrow the fake so a test can
  /// inspect it afterwards.
  impl LpcIoOps for &FakeBus {
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

  fn probe(bus: &FakeBus, slot: u8) -> SlotProbe<DetectedFintekSlot> {
    ActiveFintekSensors::probe_slot(bus, slot)
  }

  fn detect(bus: &FakeBus, slot: u8) -> DetectedFintekSlot {
    ActiveFintekSensors::discover_slot(bus, slot)
      .expect("detection succeeds")
      .expect("a Fintek path")
  }

  /// Detect on `slot`, then hand the bus to the active path with a clear
  /// operation log.
  fn activate(bus: FakeBus, slot: u8) -> ActiveFintekSensors<FakeBus> {
    let detected = detect(&bus, slot);
    bus.ops.borrow_mut().clear();
    ActiveFintekSensors::new(bus, detected)
  }

  /// Run the full motherboard discovery loop and require a Fintek path.
  fn discover_fintek(bus: &FakeBus) -> ActiveFintekSensors<&FakeBus> {
    match ActiveMotherboardSensors::discover(bus) {
      Ok(ActiveMotherboardSensors::Fintek(active)) => active,
      Ok(_) => panic!("another vendor path was selected"),
      Err(failure) => panic!("discovery failed: {failure}"),
    }
  }

  fn temperatures(sample: &MotherboardSensorSample) -> Vec<(&str, f32)> {
    sample
      .temperatures
      .iter()
      .map(|temperature| (temperature.name.as_str(), temperature.temperature))
      .collect()
  }

  fn fans(sample: &MotherboardSensorSample) -> Vec<(&str, Option<u32>, FanSpeedStatus)> {
    sample
      .fan_speeds
      .iter()
      .map(|fan| (fan.name.as_str(), fan.rpm, fan.status))
      .collect()
  }

  fn exits(bus: &FakeBus, port: u16) -> usize {
    bus.count(Op::ConfigKey(port, 0xAA))
  }

  const F71882_FAILURE_PREFIX: &str =
    "Experimental Fintek F71882 motherboard sensor path failed";
  const HARDWARE_STATE_PREFIX: &str =
    "Experimental Fintek motherboard sensor path failed: hardware state";

  // --- "Synthetic cases" fixture 1 ("Detection", "Sampling", decodes) ---

  #[test]
  fn fixture_one_f71882_on_slot_one() {
    let bus = FakeBus::on_slot(1, FakeChip::f71882());

    let detected = detect(&bus, 1);

    assert_eq!(detected.slot, 1);
    assert_eq!(detected.raw_chip_id, 0x0541);
    assert_eq!(detected.hm_base, 0x0295);
    assert_eq!(
      detected.ports,
      FintekHmPorts {
        index: 0x0295,
        data: 0x0296
      }
    );
    // Detection steps 2-5 in order, then the step 7 exit.
    assert_eq!(
      bus.ops(),
      [
        Op::SelectSlot(1),
        Op::ConfigKey(0x4E, 0x87),
        Op::ConfigKey(0x4E, 0x87),
        Op::ConfigRead(0x20),
        Op::ConfigRead(0x21),
        Op::ConfigRead(0x23),
        Op::ConfigRead(0x24),
        Op::ConfigRead(0x25),
        Op::ConfigRead(0x27),
        Op::ConfigRead(0x28),
        Op::ConfigRead(0x2B),
        Op::ConfigWrite(0x07, 0x04),
        Op::ConfigRead(0x30),
        Op::ConfigRead(0x60),
        Op::ConfigRead(0x61),
        Op::FindBars,
        Op::ConfigKey(0x4E, 0xAA),
      ]
    );
    assert!(!bus.chip(1).in_configuration.get());

    bus.ops.borrow_mut().clear();
    let active = ActiveFintekSensors::new(bus, detected);
    let (sample, _) = active.collect_sample_unlocked().unwrap();

    assert_eq!(
      temperatures(&sample),
      [("TEMP1", 40.0), ("TEMP2", 30.0), ("TEMP3", 125.0)]
    );
    assert_eq!(
      fans(&sample),
      [
        ("FAN1", Some(2000), FanSpeedStatus::Active),
        ("FAN2", Some(0), FanSpeedStatus::Inactive),
        ("FAN3", None, FanSpeedStatus::Invalid),
      ]
    );
    // FAN4 is omitted before any count read (SPI enabled).
    assert_eq!(
      active.client.hm_reads(),
      [
        0x01, 0x6F, 0x0A, 0x72, 0x74, 0x76, 0xA0, 0xA1, 0xB0, 0xB1, 0xC0, 0xC1
      ]
    );
    for op in active.client.ops() {
      match op {
        Op::HmIndex(port, _) => assert_eq!(port, 0x0295),
        Op::HmRead(port, _) => assert_eq!(port, 0x0296),
        other => panic!("sampling issued {other:?}"),
      }
    }
    for source in sample
      .temperatures
      .iter()
      .map(|temperature| &temperature.source)
      .chain(sample.fan_speeds.iter().map(|fan| &fan.source))
    {
      assert_eq!(source, "Fintek F71882 / Super I/O");
    }
    assert_eq!(active.fan_support(), SensorSupport::Supported);
    assert_eq!(
      active.diagnostic_summary(),
      "active chip=Fintek F71882 raw_id=0x0541 slot=1 hm_base=0x0295 \
       hm_ports=0x0295/0x0296 routed_fans=FAN1,FAN2,FAN3"
    );
  }

  #[test]
  fn fixture_bases_derive_the_hm_ports() {
    // "Synthetic cases": base 0x0290, then 0x0A40, then 0x0A45.
    for (base, index, data) in [
      (0x0290, 0x0295, 0x0296),
      (0x0A40, 0x0A45, 0x0A46),
      (0x0A45, 0x0A45, 0x0A46),
    ] {
      let chip = FakeChip::f71882();
      chip.set_base(base);
      let active = activate(FakeBus::on_slot(1, chip), 1);

      assert_eq!(active.detected.ports, FintekHmPorts { index, data });
      let (sample, _) = active.collect_sample_unlocked().unwrap();
      assert_eq!(sample.temperatures.len(), 3, "0x{base:04X}");
      assert!(
        active.client.ops().iter().all(|op| match op {
          Op::HmIndex(port, _) => *port == index,
          Op::HmRead(port, _) => *port == data,
          _ => false,
        }),
        "0x{base:04X}"
      );
    }
  }

  #[test]
  fn fixture_unusable_base_or_disabled_ldn_issues_no_hm_index_write() {
    // "Synthetic cases": base 0x0000, 0x0293, 0x00F8, 0x1000 or CR30=0x00.
    let mut cases: Vec<FakeChip> = [0x0000, 0x0293, 0x00F8, 0x1000]
      .into_iter()
      .map(|base| {
        let chip = FakeChip::f71882();
        chip.set_base(base);
        chip
      })
      .collect();
    let disabled = FakeChip::f71882();
    disabled.ldn4.borrow_mut().insert(0x30, 0x00);
    cases.push(disabled);

    for chip in cases {
      let bus = FakeBus::on_slot(0, chip);

      let failure = ActiveFintekSensors::discover_slot(&bus, 0).unwrap_err();

      let error = failure.to_string();
      assert!(error.starts_with(HARDWARE_STATE_PREFIX), "{error}");
      assert!(error.ends_with("(raw chip ID 0x0541)"), "{error}");
      assert!(!failure.offers_pawnio_guidance());
      assert!(!failure.is_retryable());
      assert!(!bus.has_hm_access(), "{error}");
      assert_eq!(bus.count(Op::FindBars), 0, "{error}");
      assert_eq!(exits(&bus, 0x2E), 1, "{error}");
    }
  }

  #[test]
  fn fixture_failed_vendor_id_or_cr25_stops_before_ldn_selection() {
    // "Synthetic cases": vendor 0xFF/0xFF, or CR25 bit 4 set.
    let bad_vendor = FakeChip::f71882();
    bad_vendor.set_global(0x23, 0xFF);
    bad_vendor.set_global(0x24, 0xFF);
    let powered_down = FakeChip::f71882();
    powered_down.set_global(0x25, 0x10);

    for (chip, expected) in [
      (
        bad_vendor,
        "unexpected Super I/O vendor ID 0xFFFF (CR23/CR24)",
      ),
      (
        powered_down,
        "hardware monitor is powered down (CR25=0x10, bit 4 SOFTPD_HM set)",
      ),
    ] {
      let bus = FakeBus::on_slot(0, chip);

      let failure = ActiveFintekSensors::discover_slot(&bus, 0).unwrap_err();

      assert_eq!(
        failure.to_string(),
        format!(
          "{HARDWARE_STATE_PREFIX} (Fintek F71882): {expected} (raw chip ID 0x0541)"
        )
      );
      assert_eq!(bus.count(Op::ConfigWrite(0x07, 0x04)), 0);
      assert_eq!(exits(&bus, 0x2E), 1);
      assert!(!bus.has_hm_access());
    }
  }

  #[test]
  fn fixture_ids_outside_the_table_are_left_alone() {
    // "Synthetic cases": raw IDs of the deferred targets, or 0xFFFF.
    for raw_id in [
      0x0814, 0x1005, 0x0909, 0x1106, 0x1007, 0x0601, 0x0901, 0x0507, 0x1010, 0x1118,
      0xFFFF, 0x0000,
    ] {
      let bus = FakeBus::on_slot(0, FakeChip::with_chip_id(raw_id));

      let probe = probe(&bus, 0);

      assert!(probe.configuration_exited);
      assert!(probe.result.unwrap().is_none(), "0x{raw_id:04X}");
      assert_eq!(
        bus.ops(),
        [
          Op::SelectSlot(0),
          Op::ConfigKey(0x2E, 0x87),
          Op::ConfigKey(0x2E, 0x87),
          Op::ConfigRead(0x20),
          Op::ConfigRead(0x21),
          Op::ConfigKey(0x2E, 0xAA),
        ],
        "0x{raw_id:04X}"
      );
    }
  }

  // --- "Synthetic cases": F71889 ("Detection" step 6, routing, TEMP1 source) ---

  #[test]
  fn f71889_detection_checks_the_hm_vendor_id_after_bar_discovery() {
    let bus = FakeBus::on_slot(0, FakeChip::f71889());

    let detected = detect(&bus, 0);

    assert_eq!(detected.profile.chip_label, "Fintek F71889");
    let ops = bus.ops();
    let find_bars = ops.iter().position(|op| *op == Op::FindBars).unwrap();
    assert_eq!(
      ops[find_bars..],
      [
        Op::FindBars,
        Op::HmIndex(0x0295, 0x5D),
        Op::HmRead(0x0296, 0x5D),
        Op::HmIndex(0x0295, 0x5E),
        Op::HmRead(0x0296, 0x5E),
        Op::ConfigKey(0x2E, 0xAA),
      ]
    );
    // Only CR2B is a routing register on this row.
    assert!(!ops.contains(&Op::ConfigRead(0x27)));
    assert!(!ops.contains(&Op::ConfigRead(0x28)));
    assert!(ops.contains(&Op::ConfigRead(0x2B)));
  }

  #[test]
  fn fixture_f71889_hm_vendor_id_mismatch_caches_no_path() {
    // "Synthetic cases": HM 0x5D/0x5E = 0xFF/0xFF.
    let chip = FakeChip::f71889();
    chip.set_hm(0x5D, 0xFF);
    chip.set_hm(0x5E, 0xFF);
    let bus = FakeBus::on_slot(0, chip);

    let failure = ActiveFintekSensors::discover_slot(&bus, 0).unwrap_err();

    assert_eq!(
      failure.to_string(),
      format!(
        "{HARDWARE_STATE_PREFIX} (Fintek F71889): unexpected hardware-monitor vendor ID \
         0xFFFF (HM 0x5D/0x5E) (raw chip ID 0x0723)"
      )
    );
    assert!(!failure.is_retryable());
    assert_eq!(bus.hm_reads(), [0x5D, 0x5E]);
    assert_eq!(exits(&bus, 0x2E), 1);
  }

  #[test]
  fn f71889_hm_vendor_id_port_failure_requests_rediscovery() {
    let bus = FakeBus::on_slot(0, FakeChip::f71889());
    bus.failed_hm_read.set(Some(0x5D));

    let failure = ActiveFintekSensors::discover_slot(&bus, 0).unwrap_err();

    let error = failure.to_string();
    assert!(
      error.starts_with(
        "Experimental Fintek F71889 motherboard sensor path failed: HM vendor ID read failed"
      ),
      "{error}"
    );
    assert!(failure.is_retryable());
    assert!(failure.offers_pawnio_guidance());
  }

  #[test]
  fn fixture_f71889_cr2b_omits_fan3_before_its_count_reads() {
    // "Synthetic cases": CR2B bits 1:0 = 01 or 11.
    for cr2b in [0x01, 0x03] {
      let chip = FakeChip::f71889();
      chip.set_global(0x2B, cr2b);
      let active = activate(FakeBus::on_slot(0, chip), 0);

      let (sample, _) = active.collect_sample_unlocked().unwrap();

      let labels = fans(&sample)
        .into_iter()
        .map(|(label, ..)| label)
        .collect::<Vec<_>>();
      assert_eq!(labels, ["FAN1", "FAN2"], "0x{cr2b:02X}");
      let reads = active.client.hm_reads();
      for register in [0xC0, 0xC1, 0xD0, 0xD1] {
        assert!(!reads.contains(&register), "0x{cr2b:02X} 0x{register:02X}");
      }
    }
  }

  #[test]
  fn f71889_sample_decodes_signed_bytes_and_three_fans() {
    let active = activate(FakeBus::on_slot(0, FakeChip::f71889()), 0);

    let (sample, partial_failures) = active.collect_sample_unlocked().unwrap();

    assert_eq!(
      temperatures(&sample),
      [("TEMP1", -40.0), ("TEMP2", 30.0), ("TEMP3", 125.0)]
    );
    assert_eq!(sample.fan_speeds.len(), 3);
    assert!(partial_failures.is_empty(), "{partial_failures:?}");
    assert_eq!(sample.temperatures[0].source, "Fintek F71889 / Super I/O");
  }

  #[test]
  fn fixture_f71889_temp1_source_gate() {
    // "Synthetic cases": 0x0A = 0x01; 0x03 with 0x7F=0x00; 0x03 with
    // 0x7F=0x01, 0x02 or 0x03; 0x02.
    for (hm0a, hm7f, temp1_read, reads_7f) in [
      (0x01, 0x00, true, false),
      (0x03, 0x00, true, true),
      (0x03, 0x01, false, true),
      (0x03, 0x02, false, true),
      (0x03, 0x03, false, true),
      (0x02, 0x00, false, false),
    ] {
      let chip = FakeChip::f71889();
      chip.set_hm(0x0A, hm0a);
      chip.set_hm(0x7F, hm7f);
      let active = activate(FakeBus::on_slot(0, chip), 0);

      let (sample, _) = active.collect_sample_unlocked().unwrap();

      let labels = temperatures(&sample)
        .into_iter()
        .map(|(label, _)| label)
        .collect::<Vec<_>>();
      let case = format!("0x0A=0x{hm0a:02X} 0x7F=0x{hm7f:02X}");
      if temp1_read {
        assert_eq!(labels, ["TEMP1", "TEMP2", "TEMP3"], "{case}");
      } else {
        assert_eq!(labels, ["TEMP2", "TEMP3"], "{case}");
      }
      let reads = active.client.hm_reads();
      assert_eq!(reads.contains(&0x7F), reads_7f, "{case}");
      assert_eq!(reads.contains(&0x72), temp1_read, "{case}");
    }
  }

  // --- "Synthetic cases": sampling gates and partial results ---

  #[test]
  fn fixture_f71882_temperature_bytes() {
    // "Synthetic cases": bytes 0x7E, 0x80, 0xD8, 0x00.
    for (raw, expected) in [(0x7E, None), (0x80, None), (0xD8, None), (0x00, Some(0.0))] {
      let chip = FakeChip::f71882();
      chip.set_hm(0x74, raw);
      let active = activate(FakeBus::on_slot(0, chip), 0);

      let (sample, partial_failures) = active.collect_sample_unlocked().unwrap();

      let temp2 = sample
        .temperatures
        .iter()
        .find(|temperature| temperature.name == "TEMP2")
        .map(|temperature| temperature.temperature);
      assert_eq!(temp2, expected, "0x{raw:02X}");
      if expected.is_none() {
        assert_eq!(
          temperatures(&sample),
          [("TEMP1", 40.0), ("TEMP3", 125.0)],
          "0x{raw:02X}"
        );
        assert_eq!(
          partial_failures,
          [format!(
            "TEMP2 returned an out-of-range raw value 0x{raw:02X} and is unavailable"
          )]
        );
      }
    }
  }

  #[test]
  fn fixture_hm01_start_and_power_down_bits() {
    // "Synthetic cases": HM 0x01 = 0xFF; 0x02; 0x01.
    let chip = FakeChip::f71882();
    chip.set_hm(0x01, 0xFF);
    let powered_down = activate(FakeBus::on_slot(0, chip), 0);
    let failure = powered_down.collect_sample_unlocked().unwrap_err();
    assert_eq!(
      failure.to_string(),
      format!(
        "{HARDWARE_STATE_PREFIX} (Fintek F71882): hardware monitor is powered down \
         (HM 0x01=0xFF, bit 2 set)"
      )
    );
    assert!(!failure.invalidates_cached_path());
    assert_eq!(powered_down.client.hm_reads(), [0x01]);

    let chip = FakeChip::f71882();
    chip.set_hm(0x01, 0x02);
    let fans_only = activate(FakeBus::on_slot(0, chip), 0);
    let (sample, partial_failures) = fans_only.collect_sample_unlocked().unwrap();
    assert!(sample.temperatures.is_empty());
    assert_eq!(sample.fan_speeds.len(), 3);
    assert_eq!(
      partial_failures,
      ["temperatures are in standby (HM 0x01=0x02, bit 0 clear)"]
    );
    assert_eq!(
      fans_only.client.hm_reads(),
      [0x01, 0xA0, 0xA1, 0xB0, 0xB1, 0xC0, 0xC1]
    );

    let chip = FakeChip::f71882();
    chip.set_hm(0x01, 0x01);
    let temperatures_only = activate(FakeBus::on_slot(0, chip), 0);
    let (sample, partial_failures) = temperatures_only.collect_sample_unlocked().unwrap();
    assert_eq!(sample.temperatures.len(), 3);
    assert!(sample.fan_speeds.is_empty());
    assert_eq!(
      partial_failures,
      ["fans are in standby (HM 0x01=0x01, bit 1 clear)"]
    );
    assert_eq!(
      temperatures_only.client.hm_reads(),
      [0x01, 0x6F, 0x0A, 0x72, 0x74, 0x76]
    );
  }

  #[test]
  fn both_start_bits_clear_is_a_hardware_state() {
    let chip = FakeChip::f71882();
    chip.set_hm(0x01, 0x00);
    let active = activate(FakeBus::on_slot(0, chip), 0);

    let failure = active.collect_sample_unlocked().unwrap_err();

    let error = failure.to_string();
    assert!(error.starts_with(HARDWARE_STATE_PREFIX), "{error}");
    assert!(error.contains("no usable sensor readings"), "{error}");
    assert!(!failure.offers_pawnio_guidance());
    assert_eq!(active.client.hm_reads(), [0x01]);
  }

  #[test]
  fn fixture_f71882_temp1_measurement_type() {
    // "Synthetic cases": F71882 0x0A bits 1:0 = 01 or 10.
    for hm0a in [0x01, 0x02] {
      let chip = FakeChip::f71882();
      chip.set_hm(0x0A, hm0a);
      let active = activate(FakeBus::on_slot(0, chip), 0);

      let (sample, partial_failures) = active.collect_sample_unlocked().unwrap();

      assert_eq!(temperatures(&sample), [("TEMP2", 30.0), ("TEMP3", 125.0)]);
      assert_eq!(
        partial_failures,
        [format!(
          "TEMP1 is not sourced from the D1+ diode (HM 0x0A=0x{hm0a:02X})"
        )]
      );
      assert!(!active.client.hm_reads().contains(&0x72));
    }
  }

  #[test]
  fn fixture_diode_open_keeps_the_other_labels() {
    // "Synthetic cases": 0x6F = 0x04.
    let chip = FakeChip::f71882();
    chip.set_hm(0x6F, 0x04);
    let active = activate(FakeBus::on_slot(0, chip), 0);

    let (sample, partial_failures) = active.collect_sample_unlocked().unwrap();

    assert_eq!(temperatures(&sample), [("TEMP1", 40.0), ("TEMP3", 125.0)]);
    assert_eq!(
      partial_failures,
      ["TEMP2 reports an open diode (HM 0x6F=0x04)"]
    );
  }

  #[test]
  fn failed_diode_open_or_source_reads_limit_their_inputs() {
    // "Sampling" step 2: a failed 0x6F read makes every temperature
    // unavailable; a failed 0x0A read makes TEMP1 unavailable.
    let active = activate(FakeBus::on_slot(0, FakeChip::f71882()), 0);
    active.client.failed_hm_read.set(Some(0x6F));
    let (sample, partial_failures) = active.collect_sample_unlocked().unwrap();
    assert!(sample.temperatures.is_empty());
    assert_eq!(sample.fan_speeds.len(), 3);
    assert_eq!(
      partial_failures,
      ["temperatures are unavailable: HM 0x6F read failed: simulated HM 0x6F failure"]
    );

    active.client.failed_hm_read.set(Some(0x0A));
    let (sample, partial_failures) = active.collect_sample_unlocked().unwrap();
    assert_eq!(temperatures(&sample), [("TEMP2", 30.0), ("TEMP3", 125.0)]);
    assert_eq!(
      partial_failures,
      ["TEMP1 is unavailable: HM 0x0A read failed: simulated HM 0x0A failure"]
    );
  }

  #[test]
  fn fixture_failed_fan2_lsb_keeps_fan1() {
    // "Synthetic cases": a failed FAN2 LSB read with valid FAN1.
    let active = activate(FakeBus::on_slot(0, FakeChip::f71882()), 0);
    active.client.failed_hm_read.set(Some(0xB1));

    let (sample, partial_failures) = active.collect_sample_unlocked().unwrap();

    assert_eq!(
      fans(&sample),
      [
        ("FAN1", Some(2000), FanSpeedStatus::Active),
        ("FAN3", None, FanSpeedStatus::Invalid),
      ]
    );
    assert_eq!(
      partial_failures,
      ["FAN2 read failed: simulated HM 0xB1 failure"]
    );
    // MSB first, then LSB.
    let reads = active.client.hm_reads();
    let msb = reads.iter().position(|register| *register == 0xB0).unwrap();
    assert_eq!(reads[msb + 1], 0xB1);
  }

  #[test]
  fn fan4_is_read_when_spi_is_disabled_and_cr2b_selects_fanin4() {
    let chip = FakeChip::f71882();
    chip.set_global(0x27, 0x10);
    chip.set_global(0x2B, 0x01);
    let active = activate(FakeBus::on_slot(0, chip), 0);

    let (sample, _) = active.collect_sample_unlocked().unwrap();

    assert_eq!(
      fans(&sample).last().copied(),
      Some(("FAN4", Some(1500), FanSpeedStatus::Active))
    );
    assert_eq!(active.client.hm_reads()[10..], [0xC0, 0xC1, 0xD0, 0xD1]);
  }

  #[test]
  fn a_sample_without_readings_fails_with_its_cause() {
    // Every temperature lost to a failed read keeps the component wording.
    let chip = FakeChip::f71882();
    chip.set_hm(0x01, 0x01);
    let active = activate(FakeBus::on_slot(0, chip), 0);
    active.client.failed_hm_read.set(Some(0x6F));
    let failure = active.collect_sample_unlocked().unwrap_err();
    let error = failure.to_string();
    assert!(error.starts_with(F71882_FAILURE_PREFIX), "{error}");
    assert!(failure.offers_pawnio_guidance());
    assert!(!failure.invalidates_cached_path());

    // Every input gated off by hardware state reads as a hardware state.
    let chip = FakeChip::f71882();
    chip.set_hm(0x01, 0x01);
    chip.set_hm(0x6F, 0x0E);
    let active = activate(FakeBus::on_slot(0, chip), 0);
    let failure = active.collect_sample_unlocked().unwrap_err();
    let error = failure.to_string();
    assert!(error.starts_with(HARDWARE_STATE_PREFIX), "{error}");
    assert!(error.contains("no usable sensor readings"), "{error}");
  }

  // --- "Detection" step 2 exception: the slot-1 exit withhold ---

  #[test]
  fn fixture_slot1_withhold_ids_issue_no_exit_write() {
    // "Synthetic cases": slot 1 reads 0x8733 (likewise 0x8790, 0x8695, 0x8883).
    for raw_id in [0x8733, 0x8790, 0x8695, 0x8883] {
      let bus = FakeBus::on_slot(1, FakeChip::with_chip_id(raw_id));

      let probe = probe(&bus, 1);

      assert!(!probe.configuration_exited, "0x{raw_id:04X}");
      assert!(probe.result.unwrap().is_none(), "0x{raw_id:04X}");
      assert_eq!(
        bus.ops(),
        [
          Op::SelectSlot(1),
          Op::ConfigKey(0x4E, 0x87),
          Op::ConfigKey(0x4E, 0x87),
          Op::ConfigRead(0x20),
          Op::ConfigRead(0x21),
        ],
        "0x{raw_id:04X}"
      );
    }
  }

  #[test]
  fn slot0_never_withholds_the_exit() {
    let bus = FakeBus::on_slot(0, FakeChip::with_chip_id(0x8733));

    let probe = probe(&bus, 0);

    assert!(probe.configuration_exited);
    assert_eq!(exits(&bus, 0x2E), 1);
  }

  #[test]
  fn slot1_withhold_ends_every_later_probe_on_that_slot() {
    let bus = FakeBus::new(None, Some(FakeChip::with_chip_id(0x8733)));

    let failure = match ActiveMotherboardSensors::discover(&bus) {
      Ok(_) => panic!("no path is expected"),
      Err(failure) => failure,
    };

    assert!(failure.is_unsupported());
    // The ITE keys reach slot 0 only.
    assert_eq!(bus.count(Op::ConfigKey(0x4E, 0x01)), 0);
    assert_eq!(bus.count(Op::ConfigKey(0x2E, 0x01)), 1);
    let ops = bus.ops();
    let last_slot1 = ops.iter().rposition(|op| *op == Op::SelectSlot(1)).unwrap();
    // The Fintek probe is the last one on slot 1 and ends without an exit.
    assert_eq!(
      ops[last_slot1..],
      [
        Op::SelectSlot(1),
        Op::ConfigKey(0x4E, 0x87),
        Op::ConfigKey(0x4E, 0x87),
        Op::ConfigRead(0x20),
        Op::ConfigRead(0x21),
      ]
    );
  }

  // --- "Detection": slot order, BAR discovery and the cached path ---

  #[test]
  fn fixture_slot0_pass_ends_detection_without_selecting_slot1() {
    // "Synthetic cases": slot 0 reports 0x0541 and passes every gate; slot 1
    // holds another chip.
    let bus = FakeBus::new(
      Some(FakeChip::f71882()),
      Some(FakeChip::with_chip_id(0xD802)),
    );

    let mut active = discover_fintek(&bus);

    assert_eq!(active.detected.slot, 0);
    assert_eq!(bus.count(Op::FindBars), 1);
    assert!(!bus.ops().contains(&Op::SelectSlot(1)));

    bus.ops.borrow_mut().clear();
    let sample = active.sample_with_lock(|| Ok(())).unwrap();
    assert_eq!(sample.temperatures.len(), 3);
    assert!(
      bus
        .ops()
        .iter()
        .all(|op| matches!(op, Op::HmIndex(0x0295, _) | Op::HmRead(0x0296, _)))
    );
  }

  #[test]
  fn fixture_slot1_match_is_used_only_after_slot0_fails() {
    // "Synthetic cases": slot 0 reports 0x0541 but fails the base gate; slot 1
    // reports 0x0723 and passes.
    let slot0 = FakeChip::f71882();
    slot0.set_base(0x0293);
    let bus = FakeBus::new(Some(slot0), Some(FakeChip::f71889()));

    let active = discover_fintek(&bus);

    assert_eq!(active.detected.slot, 1);
    assert_eq!(active.detected.profile.chip_label, "Fintek F71889");
    let ops = bus.ops();
    assert_eq!(bus.count(Op::FindBars), 1);
    let find_bars = ops.iter().position(|op| *op == Op::FindBars).unwrap();
    let last_selection = ops[..find_bars]
      .iter()
      .rposition(|op| matches!(op, Op::SelectSlot(_)))
      .unwrap();
    assert_eq!(ops[last_selection], Op::SelectSlot(1));
    assert!(
      !ops[find_bars..]
        .iter()
        .any(|op| matches!(op, Op::SelectSlot(_)))
    );
    // Slot 0's Fintek attempt read the base and ended with 0xAA.
    let slot0_base_read = ops
      .iter()
      .position(|op| *op == Op::ConfigRead(0x61))
      .unwrap();
    assert_eq!(ops[slot0_base_read + 1], Op::ConfigKey(0x2E, 0xAA));
  }

  #[test]
  fn fixture_slot_selection_after_caching_invalidates_the_path() {
    // "Synthetic cases": any ioctl_select_slot on the shared handle after the
    // path is cached.
    let mut active = activate(FakeBus::on_slot(0, FakeChip::f71882()), 0);
    active.client.select_lpc_slot(1).unwrap();
    active.client.ops.borrow_mut().clear();

    let failure = active.sample_with_lock(|| Ok(())).unwrap_err();

    assert!(failure.invalidates_cached_path());
    assert!(failure.is_retryable());
    let error = failure.to_string();
    assert!(
      error.starts_with(&format!("{F71882_FAILURE_PREFIX}: HM 0x01 read failed")),
      "{error}"
    );
    // The module refused the index write, so no HM read happened.
    assert!(active.client.ops().is_empty());

    // Only the full detection sequence, ending with BAR discovery, restores
    // the path.
    let bus = active.client;
    let detected = detect(&bus, 0);
    let mut active = ActiveFintekSensors::new(bus, detected);
    let sample = active.sample_with_lock(|| Ok(())).unwrap();
    assert_eq!(sample.temperatures.len(), 3);
  }

  #[test]
  fn fixture_mutex_timeout_skips_the_sample_without_port_io() {
    // "Synthetic cases": mutex timeout.
    let mut active = activate(FakeBus::on_slot(0, FakeChip::f71882()), 0);

    let failure = active
      .sample_with_lock(|| -> Result<(), MutexError> {
        Err(MutexError::Timeout {
          name: ACCESS_ISABUS_MUTEX,
        })
      })
      .unwrap_err();

    assert!(matches!(failure, MotherboardSensorError::Mutex(_)));
    assert!(!failure.invalidates_cached_path());
    assert!(active.client.ops().is_empty());
  }

  // --- "Detection" step 7: the exit on every path ---

  #[test]
  fn exit_failure_after_a_match_requires_rediscovery() {
    let bus = FakeBus::on_slot(0, FakeChip::f71882());
    bus.fail_config_exit.set(true);

    let probe = probe(&bus, 0);

    assert!(!probe.configuration_exited);
    let failure = probe.result.unwrap_err();
    assert_eq!(
      failure.to_string(),
      format!(
        "{F71882_FAILURE_PREFIX}: {FINTEK_CONFIGURATION_EXIT_FAILURE_CONTEXT}: simulated \
         config exit failure (raw chip ID 0x0541)"
      )
    );
    assert!(failure.is_retryable());
  }

  #[test]
  fn a_failed_entry_key_still_attempts_the_exit() {
    let bus = FakeBus::on_slot(0, FakeChip::f71882());
    bus.fail_second_entry_key.set(true);

    let probe = probe(&bus, 0);

    assert!(probe.configuration_exited);
    assert_eq!(
      probe.result.unwrap_err().to_string(),
      "simulated entry key failure"
    );
    assert_eq!(exits(&bus, 0x2E), 1);
    assert!(!bus.ops().contains(&Op::ConfigRead(0x20)));
  }

  // --- "Synthetic cases": recorded transaction ("Safety notes") ---

  #[test]
  fn fixture_recorded_transaction_writes_only_the_allowed_plumbing() {
    for chip in [FakeChip::f71882(), FakeChip::f71889()] {
      let bus = FakeBus::on_slot(0, chip);
      let detected = detect(&bus, 0);
      let active = ActiveFintekSensors::new(bus, detected);
      active.collect_sample_unlocked().unwrap();

      for op in active.client.ops() {
        match op {
          Op::ConfigKey(port, value) => {
            assert_eq!(port, 0x2E);
            assert!(matches!(value, 0x87 | 0xAA), "key 0x{value:02X}");
          }
          Op::ConfigWrite(register, value) => {
            assert_eq!((register, value), (0x07, 0x04));
          }
          Op::HmRead(_, register) => {
            assert!(!FINTEK_NEVER_READ_HM_REGISTERS.contains(&register));
            assert!(active.detected.profile.is_allowed_hm_read(register));
          }
          Op::SelectSlot(_) | Op::FindBars | Op::ConfigRead(_) | Op::HmIndex(..) => {}
        }
      }
    }
  }

  #[test]
  fn refused_hm_reads_issue_no_port_io() {
    let active = activate(FakeBus::on_slot(0, FakeChip::f71882()), 0);

    for register in FINTEK_NEVER_READ_HM_REGISTERS {
      let error = active.read_hm(register).unwrap_err();
      assert!(error.starts_with("refused read"), "{error}");
    }
    assert!(active.client.ops().is_empty());
  }

  #[test]
  fn partial_samples_are_logged_once_per_run() {
    let chip = FakeChip::f71882();
    chip.set_hm(0x6F, 0x04);
    let mut active = activate(FakeBus::on_slot(0, chip), 0);

    active.sample_with_lock(|| Ok(())).unwrap();
    assert!(active.partial_failure_logged);

    active.client.chip(0).set_hm(0x6F, 0x00);
    active.sample_with_lock(|| Ok(())).unwrap();
    assert!(!active.partial_failure_logged);
  }
}
