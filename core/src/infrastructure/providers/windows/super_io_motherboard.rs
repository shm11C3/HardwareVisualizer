use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::pawn_io::{ACCESS_ISABUS_MUTEX, NamedMutex, PawnIoClient, PawnIoModule};
use crate::models::{
  MotherboardFanSpeed, MotherboardSensorSample, MotherboardTemperature, SensorSupport,
};
use crate::utils::super_io::{
  NuvotonHmBaseCheck, NuvotonHmProfile, chip_id,
  decode_filtered_nuvoton_temperature_byte, decode_ite_temperature_byte,
  decode_nuvoton_direct_rpm, fan_speed_status_from_rpm, is_absent_id,
  is_ite_temperature_channel_eligible, is_nct6701d_usable_hm_base,
  is_nuvoton_hm_io_space_locked, is_plausible_ite_temperature, nuvoton_hm_profile,
  nuvoton_hm_profiles, slot_ports,
};
use crate::{log_debug, log_warn};

const ISABUS_MUTEX_TIMEOUT: Duration = Duration::from_millis(50);
const ITE_IT8728F_CHIP_ID: u16 = 0x8728;
const ITE_CHIP_LABEL: &str = "IT8728F/EX";
const ITE_SOURCE_LABEL: &str = "IT8728F/EX / Super I/O";
pub(crate) const UNSUPPORTED_SUPER_IO_HM_PATH_REASON: &str =
  "No supported Super I/O hardware-monitor path found";
pub(crate) const ITE_EXPERIMENTAL_FAILURE_PREFIX: &str =
  "Experimental IT8728F/EX motherboard temperature path failed";
pub(crate) const ITE_EXPERIMENTAL_NON_COMPONENT_FAILURE_PREFIX: &str =
  "Experimental IT8728F/EX motherboard temperature path failed: hardware state";
/// Experimental Nuvoton failures read
/// `Experimental <chip label> motherboard sensor path failed: <reason>`.
const NUVOTON_EXPERIMENTAL_FAILURE_LEAD: &str = "Experimental";
const NUVOTON_EXPERIMENTAL_FAILURE_TAIL: &str = "motherboard sensor path failed";
/// Experimental Nuvoton failures caused by firmware/hardware state that PawnIO
/// already read successfully (a set `CR28` HM I/O-space lock, or the NCT6701D
/// inactive-LDN / unusable-base quirks). These read
/// `<prefix> (<chip label>): <reason>` and must not produce PawnIO
/// install/permission guidance.
pub(crate) const NUVOTON_EXPERIMENTAL_NON_COMPONENT_FAILURE_PREFIX: &str =
  "Experimental Nuvoton motherboard sensor path failed: hardware state";
const NUVOTON_CONFIGURATION_EXIT_FAILURE_CONTEXT: &str = "Nuvoton config exit failed";
const ITE_CONFIGURATION_EXIT_FAILURE_CONTEXT: &str = "configuration exit failed";
const ITE_EC_AUTHORIZATION_PROBE_FAILURE_CONTEXT: &str =
  "EC port authorization probe failed";

const CHIP_ID_HIGH_REGISTER: u8 = 0x20;
const CHIP_ID_LOW_REGISTER: u8 = 0x21;
const NUVOTON_HM_IO_SPACE_LOCK_REGISTER: u8 = 0x28;
const LOGICAL_DEVICE_SELECT_REGISTER: u8 = 0x07;
const NUVOTON_HARDWARE_MONITOR_LDN: u8 = 0x0B;
const ITE_ENVIRONMENT_CONTROLLER_LDN: u8 = 0x04;
const LDN_ACTIVATION_REGISTER: u8 = 0x30;
const HM_BASE_HIGH_REGISTER: u8 = 0x60;
const HM_BASE_LOW_REGISTER: u8 = 0x61;

const HM_INDEX_OFFSET: u16 = 0x05;
const HM_DATA_OFFSET: u16 = 0x06;
const HM_BANK_SELECT_REGISTER: u8 = 0x4E;
const NUVOTON_BANK_4: u8 = 0x04;

const ITE_EC_INDEX_OFFSET: u16 = 0x05;
const ITE_EC_DATA_OFFSET: u16 = 0x06;
const ITE_EC_CONFIGURATION_REGISTER: u8 = 0x00;
const ITE_EC_CHANNEL_ENABLE_REGISTER: u8 = 0x51;
const ITE_ABSENT_EC_BASE: u16 = 0x0FF8;
const ITE_SAMPLE_MIN_INTERVAL: Duration = Duration::from_millis(1_500);

const ITE_TEMPERATURE_REGISTERS: [(&str, u8, u8); 3] = [
  ("TMPIN1", 0x29, 1),
  ("TMPIN2", 0x2A, 2),
  ("TMPIN3", 0x2B, 3),
];

static MOTHERBOARD_SENSOR_SAMPLER: OnceLock<Mutex<MotherboardSensorSampler>> =
  OnceLock::new();

/// A motherboard sample paired with the active provider path's fan capability.
pub(crate) struct MotherboardSensorReadout {
  /// The current reading attempt, which can fail independently of support.
  pub(crate) sample: Result<MotherboardSensorSample, String>,
  /// Fan support declared by the detected provider path, not inferred from rows.
  pub(crate) fan_support: SensorSupport,
}

/// Read live motherboard temperature and fan RPM values through the scoped
/// Nuvoton normal-HM or ITE Environment Controller path.
///
/// Implemented from:
/// - docs/specs/sensors/pawnio-interface.md revision 6
/// - docs/specs/sensors/superio-access.md revision 3
/// - docs/specs/sensors/superio-nuvoton-nct67xx.md revision 7
/// - docs/specs/sensors/superio-ite-it86xx-it87xx.md revision 2
///
/// No other external sensor implementation source was used.
pub(crate) fn sample_motherboard_sensors() -> MotherboardSensorReadout {
  let sampler = MOTHERBOARD_SENSOR_SAMPLER.get_or_init(|| {
    let sampler = MotherboardSensorSampler::new();
    log_debug!(
      "super_io_motherboard_sensor_sampler_initialized",
      "windows::super_io_motherboard::sample_motherboard_sensors",
      Some(sampler.diagnostic_summary())
    );
    Mutex::new(sampler)
  });

  let Ok(mut sampler) = sampler.lock() else {
    return MotherboardSensorReadout {
      sample: Err("Motherboard sensor sampler lock poisoned".to_string()),
      fan_support: SensorSupport::Unknown,
    };
  };
  let sample = sampler.sample();
  let fan_support = sampler.active.as_ref().map_or(
    SensorSupport::Unknown,
    ActiveMotherboardSensors::fan_support,
  );

  MotherboardSensorReadout {
    sample,
    fan_support,
  }
}

struct MotherboardSensorSampler {
  active: Option<ActiveMotherboardSensors<PawnIoClient>>,
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

    let result = match self.active.as_mut() {
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
    match ActiveMotherboardSensors::open() {
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
      Some(active) => active.diagnostic_summary(),
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
  profile: &'static NuvotonHmProfile,
}

struct ActiveIteMotherboardSensors<C: LpcIoOps> {
  client: C,
  slot: u8,
  ec_base: u16,
  chip_label: &'static str,
  source_label: &'static str,
  last_attempt: Option<(Instant, Result<MotherboardSensorSample, String>)>,
  partial_failure_logged: bool,
}

enum ActiveMotherboardSensors<C: LpcIoOps> {
  Nuvoton(ActiveNuvotonMotherboardSensors<C>),
  Ite(ActiveIteMotherboardSensors<C>),
}

impl ActiveMotherboardSensors<PawnIoClient> {
  fn open() -> Result<Self, String> {
    let (client, _discovery) =
      PawnIoClient::open(PawnIoModule::LpcIo).map_err(|error| error.reason)?;

    let _mutex = NamedMutex::acquire(ACCESS_ISABUS_MUTEX, ISABUS_MUTEX_TIMEOUT)?;
    let mut last_error: Option<String> = None;

    for slot in [0_u8, 1] {
      match ActiveNuvotonMotherboardSensors::discover_slot(&client, slot) {
        Ok(Some(discovered)) => {
          return Ok(Self::Nuvoton(ActiveNuvotonMotherboardSensors {
            client,
            slot: discovered.slot,
            hm_base: discovered.hm_base,
            profile: discovered.profile,
          }));
        }
        Ok(None) => {}
        Err(reason) => {
          last_error = Some(reason);
        }
      }

      match ActiveIteMotherboardSensors::discover_slot(&client, slot) {
        Ok(Some(discovered)) => {
          return Ok(Self::Ite(ActiveIteMotherboardSensors {
            client,
            slot: discovered.slot,
            ec_base: discovered.ec_base,
            chip_label: ITE_CHIP_LABEL,
            source_label: ITE_SOURCE_LABEL,
            last_attempt: None,
            partial_failure_logged: false,
          }));
        }
        Ok(None) => {}
        Err(reason) => {
          last_error = Some(reason);
        }
      }
    }

    Err(last_error.unwrap_or_else(|| UNSUPPORTED_SUPER_IO_HM_PATH_REASON.to_string()))
  }
}

impl<C: LpcIoOps> ActiveMotherboardSensors<C> {
  /// Report the capability of the selected hardware path independently of a sample.
  fn fan_support(&self) -> SensorSupport {
    match self {
      Self::Nuvoton(_) => SensorSupport::Supported,
      Self::Ite(_) => SensorSupport::Unsupported,
    }
  }

  fn sample(&mut self) -> Result<MotherboardSensorSample, String> {
    match self {
      Self::Nuvoton(active) => active.sample(),
      Self::Ite(active) => active.sample(),
    }
  }

  fn diagnostic_summary(&self) -> String {
    match self {
      Self::Nuvoton(active) => format!(
        "active chip={} slot={} hm_base=0x{:04X}",
        active.profile.chip_label, active.slot, active.hm_base
      ),
      Self::Ite(active) => format!(
        "active chip={} slot={} ec_base=0x{:04X}",
        active.chip_label, active.slot, active.ec_base
      ),
    }
  }
}

impl<C: LpcIoOps> ActiveNuvotonMotherboardSensors<C> {
  fn discover_slot(client: &C, slot: u8) -> Result<Option<DetectedSlot>, String> {
    let Some((index_port, _data_port)) = slot_ports(slot) else {
      return Ok(None);
    };

    client.select_lpc_slot(slot as u64)?;
    enter_nuvoton(client, index_port)?;
    let result = discover_nuvoton_hm_base(client, slot);
    let exit_result = exit_nuvoton(client, index_port);

    match (result, exit_result) {
      (Ok(value), Ok(())) => Ok(value),
      (Ok(value), Err(exit_error)) => {
        let reason =
          format!("{NUVOTON_CONFIGURATION_EXIT_FAILURE_CONTEXT}: {exit_error}");
        match value {
          Some(detected) if detected.profile.is_experimental() => Err(
            nuvoton_experimental_failure(detected.profile.chip_label, reason),
          ),
          _ => Err(reason),
        }
      }
      (Err(reason), Ok(())) => Err(reason),
      (Err(reason), Err(exit_error)) => {
        Err(format!("{reason}; exit failed: {exit_error}"))
      }
    }
  }

  fn sample(&self) -> Result<MotherboardSensorSample, String> {
    let _mutex = NamedMutex::acquire(ACCESS_ISABUS_MUTEX, ISABUS_MUTEX_TIMEOUT)?;
    self.sample_unlocked()
  }

  fn sample_unlocked(&self) -> Result<MotherboardSensorSample, String> {
    self
      .sample_unlocked_raw()
      .map_err(|reason| self.surface_sample_failure(reason))
  }

  fn sample_unlocked_raw(&self) -> Result<MotherboardSensorSample, String> {
    // `ioctl_find_bars` authorizes normal-HM ports for this loaded LpcIO
    // handle during discovery. Re-running `ioctl_select_slot` here can clear
    // that BAR authorization, which makes the subsequent HM index/data port
    // I/O fail with access denied on the validated NZXT N7 B650E path.
    self.write_hm_byte(HM_BANK_SELECT_REGISTER, NUVOTON_BANK_4)?;

    let profile = self.profile;
    let mut temperatures = Vec::with_capacity(profile.temperature_registers.len());
    for (name, register) in profile.temperature_registers {
      let raw = self.read_hm_byte(*register)?;
      // A filtered raw byte leaves that input unavailable for this sample.
      if let Some(temperature) =
        decode_filtered_nuvoton_temperature_byte(raw, profile.temperature_filter)
      {
        temperatures.push(MotherboardTemperature {
          name: (*name).to_string(),
          temperature,
          source: profile.source_label.to_string(),
        });
      }
    }

    let fan_speeds = profile
      .direct_rpm_registers()
      .iter()
      .map(|(name, high_register, low_register)| {
        let high = self.read_hm_byte(*high_register)?;
        let low = self.read_hm_byte(*low_register)?;
        let rpm = decode_nuvoton_direct_rpm(high, low);
        Ok(MotherboardFanSpeed {
          name: (*name).to_string(),
          rpm: Some(rpm),
          status: fan_speed_status_from_rpm(rpm),
          source: profile.source_label.to_string(),
        })
      })
      .collect::<Result<Vec<_>, String>>()?;

    Ok(MotherboardSensorSample {
      temperatures,
      fan_speeds,
    })
  }

  fn surface_sample_failure(&self, reason: String) -> String {
    surface_nuvoton_failure(self.profile, reason)
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

impl<C: LpcIoOps> ActiveIteMotherboardSensors<C> {
  fn discover_slot(client: &C, slot: u8) -> Result<Option<DetectedIteSlot>, String> {
    let Some((index_port, _data_port)) = slot_ports(slot) else {
      return Ok(None);
    };

    client.select_lpc_slot(slot as u64)?;
    enter_ite(client, slot, index_port)?;
    let result = discover_ite_ec_base(client, slot);
    let exit_result = exit_ite(client);

    match (result, exit_result) {
      (Ok(Some(discovered)), Ok(())) => {
        read_ite_ec_byte(client, discovered.ec_base, ITE_EC_CONFIGURATION_REGISTER)
          .map_err(|reason| {
            ite_experimental_failure(format!(
              "{ITE_EC_AUTHORIZATION_PROBE_FAILURE_CONTEXT}: {reason}"
            ))
          })?;
        Ok(Some(discovered))
      }
      (Ok(None), Ok(())) => Ok(None),
      (Ok(Some(_)), Err(exit_error)) => Err(ite_experimental_failure(format!(
        "{ITE_CONFIGURATION_EXIT_FAILURE_CONTEXT}: {exit_error}"
      ))),
      (Ok(None), Err(exit_error)) => Err(format!("ITE config exit failed: {exit_error}")),
      (Err(reason), Ok(())) => Err(reason),
      (Err(reason), Err(exit_error)) => Err(format!(
        "{reason}; {ITE_CONFIGURATION_EXIT_FAILURE_CONTEXT}: {exit_error}"
      )),
    }
  }

  fn sample(&mut self) -> Result<MotherboardSensorSample, String> {
    let now = Instant::now();
    if let Some(cached) = self.cached_result(now) {
      return cached;
    }

    let _mutex = NamedMutex::acquire(ACCESS_ISABUS_MUTEX, ISABUS_MUTEX_TIMEOUT)?;
    let collected = self.collect_sample_unlocked();
    self.finish_attempt(Instant::now(), collected)
  }

  #[cfg(test)]
  fn sample_unlocked_at(
    &mut self,
    attempted_at: Instant,
  ) -> Result<MotherboardSensorSample, String> {
    if let Some(cached) = self.cached_result(attempted_at) {
      return cached;
    }

    let collected = self.collect_sample_unlocked();
    self.finish_attempt(attempted_at, collected)
  }

  fn finish_attempt(
    &mut self,
    completed_at: Instant,
    collected: Result<(MotherboardSensorSample, Vec<String>), String>,
  ) -> Result<MotherboardSensorSample, String> {
    let result = match collected {
      Ok((sample, partial_failures)) => {
        if partial_failures.is_empty() {
          self.partial_failure_logged = false;
        } else if !self.partial_failure_logged {
          self.partial_failure_logged = true;
          log_warn!(
            "experimental_ite_motherboard_sensor_partial_sample",
            "windows::super_io_motherboard::ActiveIteMotherboardSensors::sample",
            Some(partial_failures.join("; "))
          );
        }
        Ok(sample)
      }
      Err(reason) => Err(ite_experimental_failure(reason)),
    };

    self.last_attempt = Some((completed_at, result.clone()));
    result
  }

  fn cached_result(
    &self,
    now: Instant,
  ) -> Option<Result<MotherboardSensorSample, String>> {
    self
      .last_attempt
      .as_ref()
      .and_then(|(attempted_at, result)| {
        (now.saturating_duration_since(*attempted_at) < ITE_SAMPLE_MIN_INTERVAL)
          .then(|| result.clone())
      })
  }

  fn collect_sample_unlocked(
    &self,
  ) -> Result<(MotherboardSensorSample, Vec<String>), String> {
    let monitoring_state = self.read_ec_byte(ITE_EC_CONFIGURATION_REGISTER)?;
    if (monitoring_state & 0x01) == 0 {
      return Err(ite_experimental_non_component_failure(
        "Environment Controller monitoring is in standby",
      ));
    }
    if (monitoring_state & 0x08) != 0 {
      return Err(ite_experimental_non_component_failure(
        "Environment Controller monitoring is stopped by INT_Clear",
      ));
    }

    let channel_config = self.read_ec_byte(ITE_EC_CHANNEL_ENABLE_REGISTER)?;
    let mut temperatures = Vec::new();
    let mut partial_failures = Vec::new();
    let mut eligible_channels = 0_usize;
    let mut had_io_failure = false;

    for (name, register, channel) in ITE_TEMPERATURE_REGISTERS {
      if !is_ite_temperature_channel_eligible(channel_config, channel) {
        continue;
      }
      eligible_channels += 1;

      match self.read_ec_byte(register) {
        Ok(raw) => {
          let temperature = decode_ite_temperature_byte(raw);
          if is_plausible_ite_temperature(temperature) {
            temperatures.push(MotherboardTemperature {
              name: name.to_string(),
              temperature,
              source: self.source_label.to_string(),
            });
          } else {
            partial_failures.push(format!(
              "{name} returned implausible raw value 0x{raw:02X} ({temperature} C)"
            ));
          }
        }
        Err(reason) => {
          had_io_failure = true;
          partial_failures.push(format!("{name} read failed: {reason}"));
        }
      }
    }

    if eligible_channels == 0 {
      return Err(ite_experimental_non_component_failure(
        "no eligible physical TMPIN channels are enabled",
      ));
    }
    if temperatures.is_empty() {
      let reason = format!("no usable TMPIN readings: {}", partial_failures.join("; "));
      return if had_io_failure {
        Err(reason)
      } else {
        Err(ite_experimental_non_component_failure(reason))
      };
    }

    Ok((
      MotherboardSensorSample {
        temperatures,
        fan_speeds: Vec::new(),
      },
      partial_failures,
    ))
  }

  fn read_ec_byte(&self, register: u8) -> Result<u8, String> {
    read_ite_ec_byte(&self.client, self.ec_base, register)
  }
}

#[derive(Debug)]
struct DetectedSlot {
  slot: u8,
  hm_base: u16,
  profile: &'static NuvotonHmProfile,
}

#[derive(Debug)]
struct DetectedIteSlot {
  slot: u8,
  ec_base: u16,
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

  let Some(profile) = nuvoton_hm_profile(chip_id(id_high, id_low)) else {
    return Ok(None);
  };

  // The NCT6701D row documents an inactive LDN B and an unusable CR60/61 base
  // as known firmware states; other rows treat them as ordinary failures.
  let firmware_quirks = profile.hm_base_check == NuvotonHmBaseCheck::Nct6701dUsableBase;

  let result = (|| {
    // The lock gate is a read of global CR28 while still in configuration
    // mode, before LDN selection. CR28 is never written.
    if profile.lock_gate {
      let lock_state = client
        .superio_inb(NUVOTON_HM_IO_SPACE_LOCK_REGISTER)
        .map_err(NuvotonDiscoveryFailure::Transport)?;
      if is_nuvoton_hm_io_space_locked(lock_state) {
        return Err(NuvotonDiscoveryFailure::HardwareState(format!(
          "Nuvoton hardware-monitor I/O space is locked (CR28=0x{lock_state:02X}, bit 4 set)"
        )));
      }
    }

    client
      .superio_outb(LOGICAL_DEVICE_SELECT_REGISTER, NUVOTON_HARDWARE_MONITOR_LDN)
      .map_err(NuvotonDiscoveryFailure::Transport)?;
    let activation = client
      .superio_inb(LDN_ACTIVATION_REGISTER)
      .map_err(NuvotonDiscoveryFailure::Transport)?;
    if (activation & 0x01) == 0 {
      return Err(NuvotonDiscoveryFailure::firmware(
        firmware_quirks,
        "Nuvoton hardware monitor logical device is inactive".to_string(),
      ));
    }

    let base_high = client
      .superio_inb(HM_BASE_HIGH_REGISTER)
      .map_err(NuvotonDiscoveryFailure::Transport)?;
    let base_low = client
      .superio_inb(HM_BASE_LOW_REGISTER)
      .map_err(NuvotonDiscoveryFailure::Transport)?;
    let hm_base = ((base_high as u16) << 8) | base_low as u16;
    if !is_valid_nuvoton_hm_base(profile.hm_base_check, hm_base) {
      return Err(NuvotonDiscoveryFailure::firmware(
        firmware_quirks,
        format!("invalid Nuvoton hardware-monitor base 0x{hm_base:04X}"),
      ));
    }

    client
      .find_lpc_bars()
      .map_err(NuvotonDiscoveryFailure::Transport)?;

    Ok(DetectedSlot {
      slot,
      hm_base,
      profile,
    })
  })();

  result.map(Some).map_err(|failure| match failure {
    NuvotonDiscoveryFailure::Transport(reason) => {
      surface_nuvoton_failure(profile, reason)
    }
    NuvotonDiscoveryFailure::HardwareState(reason) => {
      surface_nuvoton_hardware_state_failure(profile, reason)
    }
  })
}

/// Discovery failure classification used to pick the surfaced wording.
enum NuvotonDiscoveryFailure {
  /// PawnIO/transport or unexpected register failures; guidance may apply.
  Transport(String),
  /// Firmware/hardware state read successfully through PawnIO; no guidance.
  HardwareState(String),
}

impl NuvotonDiscoveryFailure {
  fn firmware(is_documented_quirk: bool, reason: String) -> Self {
    if is_documented_quirk {
      Self::HardwareState(reason)
    } else {
      Self::Transport(reason)
    }
  }
}

fn discover_ite_ec_base(
  client: &impl LpcIoOps,
  slot: u8,
) -> Result<Option<DetectedIteSlot>, String> {
  let id_high = client.superio_inb(CHIP_ID_HIGH_REGISTER)?;
  let id_low = client.superio_inb(CHIP_ID_LOW_REGISTER)?;
  if is_absent_id(id_high, id_low) {
    return Ok(None);
  }

  let raw_chip_id = chip_id(id_high, id_low);
  if raw_chip_id != ITE_IT8728F_CHIP_ID {
    return Ok(None);
  }

  client
    .superio_outb(
      LOGICAL_DEVICE_SELECT_REGISTER,
      ITE_ENVIRONMENT_CONTROLLER_LDN,
    )
    .map_err(|reason| {
      ite_experimental_failure(format!("LDN selection failed: {reason}"))
    })?;
  let activation = client
    .superio_inb(LDN_ACTIVATION_REGISTER)
    .map_err(|reason| ite_experimental_failure(format!("CR30 read failed: {reason}")))?;
  if (activation & 0x01) == 0 {
    return Err(ite_experimental_non_component_failure(
      "Environment Controller logical device is inactive",
    ));
  }

  let base_high = client
    .superio_inb(HM_BASE_HIGH_REGISTER)
    .map_err(|reason| ite_experimental_failure(format!("CR60 read failed: {reason}")))?;
  let base_low = client
    .superio_inb(HM_BASE_LOW_REGISTER)
    .map_err(|reason| ite_experimental_failure(format!("CR61 read failed: {reason}")))?;
  let ec_base = ((base_high as u16 & 0x0F) << 8) | (base_low as u16 & 0xF8);
  if !is_valid_ite_ec_base(base_high, base_low, ec_base) {
    return Err(ite_experimental_non_component_failure(format!(
      "invalid Environment Controller base from CR60/61=0x{base_high:02X}/0x{base_low:02X} (derived 0x{ec_base:04X})"
    )));
  }

  client.find_lpc_bars().map_err(|reason| {
    ite_experimental_failure(format!("LpcIO BAR discovery failed: {reason}"))
  })?;

  Ok(Some(DetectedIteSlot { slot, ec_base }))
}

fn enter_nuvoton(client: &impl LpcIoOps, index_port: u16) -> Result<(), String> {
  client.pio_outb(index_port, 0x87)?;
  client.pio_outb(index_port, 0x87)
}

fn exit_nuvoton(client: &impl LpcIoOps, index_port: u16) -> Result<(), String> {
  client.pio_outb(index_port, 0xAA)
}

fn enter_ite(client: &impl LpcIoOps, slot: u8, index_port: u16) -> Result<(), String> {
  let final_key = match slot {
    0 => 0x55,
    1 => 0xAA,
    _ => return Err(format!("unsupported Super I/O LpcIO slot {slot}")),
  };

  for value in [0x87, 0x01, 0x55, final_key] {
    client.pio_outb(index_port, value)?;
  }
  Ok(())
}

fn exit_ite(client: &impl LpcIoOps) -> Result<(), String> {
  client.superio_outb(0x02, 0x02)
}

fn read_ite_ec_byte(
  client: &impl LpcIoOps,
  ec_base: u16,
  register: u8,
) -> Result<u8, String> {
  let index_port = ec_base
    .checked_add(ITE_EC_INDEX_OFFSET)
    .ok_or_else(|| format!("ITE EC index port overflows base 0x{ec_base:04X}"))?;
  let data_port = ec_base
    .checked_add(ITE_EC_DATA_OFFSET)
    .ok_or_else(|| format!("ITE EC data port overflows base 0x{ec_base:04X}"))?;
  client.pio_outb(index_port, register)?;
  client.pio_inb(data_port)
}

fn is_valid_hm_base(base: u16) -> bool {
  base != 0x0000 && base != 0xFFFF && base <= u16::MAX - HM_DATA_OFFSET
}

fn is_valid_nuvoton_hm_base(check: NuvotonHmBaseCheck, base: u16) -> bool {
  is_valid_hm_base(base)
    && match check {
      NuvotonHmBaseCheck::Standard => true,
      NuvotonHmBaseCheck::Nct6701dUsableBase => is_nct6701d_usable_hm_base(base),
    }
}

fn is_valid_ite_ec_base(base_high: u8, base_low: u8, ec_base: u16) -> bool {
  (base_high & 0xF0) == 0
    && (base_low & 0x07) == 0
    && (base_high, base_low) != (0xFF, 0xFF)
    && ec_base != 0x0000
    && ec_base != ITE_ABSENT_EC_BASE
    && ec_base.checked_add(ITE_EC_DATA_OFFSET).is_some()
}

fn ite_experimental_failure(reason: impl AsRef<str>) -> String {
  let reason = reason.as_ref();
  if reason.starts_with(ITE_EXPERIMENTAL_FAILURE_PREFIX) {
    reason.to_string()
  } else {
    format!("{ITE_EXPERIMENTAL_FAILURE_PREFIX}: {reason}")
  }
}

fn ite_experimental_non_component_failure(reason: impl AsRef<str>) -> String {
  format!(
    "{ITE_EXPERIMENTAL_NON_COMPONENT_FAILURE_PREFIX}: {}",
    reason.as_ref()
  )
}

fn nuvoton_experimental_failure_prefix(chip_label: &str) -> String {
  format!(
    "{NUVOTON_EXPERIMENTAL_FAILURE_LEAD} {chip_label} {NUVOTON_EXPERIMENTAL_FAILURE_TAIL}"
  )
}

fn nuvoton_experimental_failure(chip_label: &str, reason: impl AsRef<str>) -> String {
  let reason = reason.as_ref();
  let prefix = nuvoton_experimental_failure_prefix(chip_label);
  if reason.starts_with(&prefix) {
    reason.to_string()
  } else {
    format!("{prefix}: {reason}")
  }
}

/// Identify a failure as experimental only for Experimental profiles; Verified
/// failures keep their plain reason.
fn surface_nuvoton_failure(profile: &NuvotonHmProfile, reason: String) -> String {
  if profile.is_experimental() {
    nuvoton_experimental_failure(profile.chip_label, reason)
  } else {
    reason
  }
}

fn nuvoton_experimental_hardware_state_failure(
  chip_label: &str,
  reason: impl AsRef<str>,
) -> String {
  format!(
    "{NUVOTON_EXPERIMENTAL_NON_COMPONENT_FAILURE_PREFIX} ({chip_label}): {}",
    reason.as_ref()
  )
}

/// Hardware-state failures of Experimental profiles carry the non-component
/// prefix so no PawnIO guidance is produced; Verified failures keep their
/// plain reason.
fn surface_nuvoton_hardware_state_failure(
  profile: &NuvotonHmProfile,
  reason: String,
) -> String {
  if profile.is_experimental() {
    nuvoton_experimental_hardware_state_failure(profile.chip_label, reason)
  } else {
    reason
  }
}

fn is_nuvoton_experimental_failure(reason: &str) -> bool {
  nuvoton_hm_profiles()
    .filter(|profile| profile.is_experimental())
    .any(|profile| {
      reason.starts_with(&nuvoton_experimental_failure_prefix(profile.chip_label))
    })
}

fn is_retryable_init_error(reason: &str) -> bool {
  let requires_nuvoton_rediscovery = is_nuvoton_experimental_failure(reason)
    && reason.contains(NUVOTON_CONFIGURATION_EXIT_FAILURE_CONTEXT);
  let requires_ite_rediscovery = reason.starts_with(ITE_EXPERIMENTAL_FAILURE_PREFIX)
    && (reason.contains(ITE_EC_AUTHORIZATION_PROBE_FAILURE_CONTEXT)
      || reason.contains(ITE_CONFIGURATION_EXIT_FAILURE_CONTEXT));

  reason.contains("timed out waiting for mutex")
    || reason.contains("failed waiting for mutex")
    || requires_nuvoton_rediscovery
    || requires_ite_rediscovery
}

#[cfg(test)]
mod tests {
  use std::cell::{Cell, RefCell};

  use super::*;

  struct FakeLpcIo {
    chip_id_high: Cell<u8>,
    chip_id_low: Cell<u8>,
    activation: Cell<u8>,
    selected_slot_calls: Cell<u32>,
    find_bars_calls: Cell<u32>,
    hm_bars_authorized: Cell<bool>,
    selected_hm_register: Cell<Option<u8>>,
    selected_hm_bank: Cell<u8>,
    selected_ldn: Cell<Option<u8>>,
    hm_base_high_read: Cell<bool>,
    hm_base_low_read: Cell<bool>,
    read_registers: RefCell<Vec<u8>>,
    hm_io_space_lock: Cell<u8>,
    hm_base: Cell<(u8, u8)>,
    temperature_override: Cell<Option<(u8, u8)>>,
    superio_reads: RefCell<Vec<u8>>,
    superio_writes: RefCell<Vec<(u8, u8)>>,
  }

  impl FakeLpcIo {
    fn new() -> Self {
      Self {
        chip_id_high: Cell::new(0xD8),
        chip_id_low: Cell::new(0x02),
        activation: Cell::new(0x09),
        selected_slot_calls: Cell::new(0),
        find_bars_calls: Cell::new(0),
        hm_bars_authorized: Cell::new(false),
        selected_hm_register: Cell::new(None),
        selected_hm_bank: Cell::new(0),
        selected_ldn: Cell::new(None),
        hm_base_high_read: Cell::new(false),
        hm_base_low_read: Cell::new(false),
        read_registers: RefCell::new(Vec::new()),
        hm_io_space_lock: Cell::new(0x00),
        hm_base: Cell::new((0x02, 0x90)),
        temperature_override: Cell::new(None),
        superio_reads: RefCell::new(Vec::new()),
        superio_writes: RefCell::new(Vec::new()),
      }
    }

    fn with_authorized_hm_bars() -> Self {
      let client = Self::new();
      client.hm_bars_authorized.set(true);
      client.selected_ldn.set(Some(NUVOTON_HARDWARE_MONITOR_LDN));
      client.hm_base_high_read.set(true);
      client.hm_base_low_read.set(true);
      client
    }

    fn with_chip_id(chip_id: u16) -> Self {
      let client = Self::new();
      client.chip_id_high.set((chip_id >> 8) as u8);
      client.chip_id_low.set(chip_id as u8);
      client
    }

    fn require_hm_bars(&self) -> Result<(), String> {
      if self.hm_bars_authorized.get() {
        Ok(())
      } else {
        Err("hm ports are not authorized".to_string())
      }
    }

    fn read_value_for(&self, register: u8) -> u8 {
      if let Some((overridden, raw)) = self.temperature_override.get()
        && overridden == register
      {
        return raw;
      }
      match register {
        0x90..=0x95 => 32,
        0xC0 | 0xC2 | 0xC4 | 0xC6 | 0xC8 | 0xCA | 0xCE => 0x03,
        0xC1 | 0xC3 | 0xC5 | 0xC7 | 0xC9 | 0xCB | 0xCF => 0x20,
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
      Ok(self.read_value_for(register))
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
      self.superio_reads.borrow_mut().push(register);
      match register {
        CHIP_ID_HIGH_REGISTER => Ok(self.chip_id_high.get()),
        CHIP_ID_LOW_REGISTER => Ok(self.chip_id_low.get()),
        NUVOTON_HM_IO_SPACE_LOCK_REGISTER => Ok(self.hm_io_space_lock.get()),
        LDN_ACTIVATION_REGISTER => {
          if self.selected_ldn.get() == Some(NUVOTON_HARDWARE_MONITOR_LDN) {
            Ok(self.activation.get())
          } else {
            Ok(0x00)
          }
        }
        HM_BASE_HIGH_REGISTER => {
          self.hm_base_high_read.set(true);
          Ok(self.hm_base.get().0)
        }
        HM_BASE_LOW_REGISTER => {
          self.hm_base_low_read.set(true);
          Ok(self.hm_base.get().1)
        }
        _ => Ok(0),
      }
    }

    fn superio_outb(&self, register: u8, value: u8) -> Result<(), String> {
      self.superio_writes.borrow_mut().push((register, value));
      if register == LOGICAL_DEVICE_SELECT_REGISTER {
        self.selected_ldn.set(Some(value));
      }
      Ok(())
    }
  }

  const D802_CHIP_ID: u16 = 0xD802;
  const D421_CHIP_ID: u16 = 0xD421;
  const D421_FAILURE_PREFIX: &str =
    "Experimental NCT6796D/NCT6796D-E motherboard sensor path failed";
  const SIX_PAIR_READS: [u8; 18] = [
    0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,
    0xC8, 0xC9, 0xCA, 0xCB,
  ];

  fn d802_profile() -> &'static NuvotonHmProfile {
    nuvoton_hm_profile(D802_CHIP_ID).unwrap()
  }

  /// Discover slot 0 with `client` and hand the client to an active path.
  fn discovered_active(client: FakeLpcIo) -> ActiveNuvotonMotherboardSensors<FakeLpcIo> {
    let detected = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0)
      .unwrap()
      .unwrap();
    ActiveNuvotonMotherboardSensors {
      client,
      slot: detected.slot,
      hm_base: detected.hm_base,
      profile: detected.profile,
    }
  }

  struct FakeIteLpcIo {
    chip_id_low: Cell<u8>,
    activation: Cell<u8>,
    base_high: Cell<u8>,
    base_low: Cell<u8>,
    ec_configuration: Cell<u8>,
    channel_configuration: Cell<u8>,
    temperature_values: Cell<[u8; 3]>,
    failed_temperature_register: Cell<Option<u8>>,
    fail_config_exit: Cell<bool>,
    selected_slot_calls: Cell<u32>,
    find_bars_calls: Cell<u32>,
    config_exit_calls: Cell<u32>,
    ec_bars_authorized: Cell<bool>,
    selected_ldn: Cell<Option<u8>>,
    base_high_read: Cell<bool>,
    base_low_read: Cell<bool>,
    selected_ec_register: Cell<Option<u8>>,
    read_registers: RefCell<Vec<u8>>,
    config_port_writes: RefCell<Vec<(u16, u8)>>,
  }

  impl FakeIteLpcIo {
    fn new() -> Self {
      Self {
        chip_id_low: Cell::new(0x28),
        activation: Cell::new(0x01),
        base_high: Cell::new(0x02),
        base_low: Cell::new(0x90),
        ec_configuration: Cell::new(0x01),
        channel_configuration: Cell::new(0b0010_1000),
        temperature_values: Cell::new([25, 30, 35]),
        failed_temperature_register: Cell::new(None),
        fail_config_exit: Cell::new(false),
        selected_slot_calls: Cell::new(0),
        find_bars_calls: Cell::new(0),
        config_exit_calls: Cell::new(0),
        ec_bars_authorized: Cell::new(false),
        selected_ldn: Cell::new(None),
        base_high_read: Cell::new(false),
        base_low_read: Cell::new(false),
        selected_ec_register: Cell::new(None),
        read_registers: RefCell::new(Vec::new()),
        config_port_writes: RefCell::new(Vec::new()),
      }
    }

    fn with_authorized_ec(channel_configuration: u8) -> Self {
      let client = Self::new();
      client.channel_configuration.set(channel_configuration);
      client.ec_bars_authorized.set(true);
      client
        .selected_ldn
        .set(Some(ITE_ENVIRONMENT_CONTROLLER_LDN));
      client.base_high_read.set(true);
      client.base_low_read.set(true);
      client.config_exit_calls.set(1);
      client
    }

    fn require_ec_bars(&self) -> Result<(), String> {
      if self.ec_bars_authorized.get() {
        Ok(())
      } else {
        Err("EC ports are not authorized".to_string())
      }
    }
  }

  impl LpcIoOps for FakeIteLpcIo {
    fn select_lpc_slot(&self, _slot: u64) -> Result<(), String> {
      self
        .selected_slot_calls
        .set(self.selected_slot_calls.get() + 1);
      self.ec_bars_authorized.set(false);
      Ok(())
    }

    fn find_lpc_bars(&self) -> Result<(), String> {
      self.find_bars_calls.set(self.find_bars_calls.get() + 1);
      if self.selected_ldn.get() != Some(ITE_ENVIRONMENT_CONTROLLER_LDN) {
        return Err("find_lpc_bars called before ITE LDN 4 selection".to_string());
      }
      if !self.base_high_read.get() || !self.base_low_read.get() {
        return Err("find_lpc_bars called before EC base discovery".to_string());
      }
      self.ec_bars_authorized.set(true);
      Ok(())
    }

    fn pio_inb(&self, port: u16) -> Result<u8, String> {
      if port != 0x0296 {
        return Err(format!("unexpected EC data port 0x{port:04X}"));
      }
      self.require_ec_bars()?;
      if self.config_exit_calls.get() == 0 {
        return Err("EC read attempted before ITE config exit".to_string());
      }

      let register = self
        .selected_ec_register
        .get()
        .ok_or_else(|| "EC index register was not selected".to_string())?;
      self.read_registers.borrow_mut().push(register);
      if self.failed_temperature_register.get() == Some(register) {
        return Err(format!("simulated EC register 0x{register:02X} failure"));
      }

      match register {
        ITE_EC_CONFIGURATION_REGISTER => Ok(self.ec_configuration.get()),
        ITE_EC_CHANNEL_ENABLE_REGISTER => Ok(self.channel_configuration.get()),
        0x29 => Ok(self.temperature_values.get()[0]),
        0x2A => Ok(self.temperature_values.get()[1]),
        0x2B => Ok(self.temperature_values.get()[2]),
        _ => Err(format!("unexpected EC register 0x{register:02X}")),
      }
    }

    fn pio_outb(&self, port: u16, value: u8) -> Result<(), String> {
      match port {
        0x0295 => {
          self.require_ec_bars()?;
          self.selected_ec_register.set(Some(value));
          Ok(())
        }
        0x0296 => Err("ITE EC data port must remain read-only".to_string()),
        0x002E | 0x004E => {
          self.config_port_writes.borrow_mut().push((port, value));
          Ok(())
        }
        _ => Ok(()),
      }
    }

    fn superio_inb(&self, register: u8) -> Result<u8, String> {
      match register {
        CHIP_ID_HIGH_REGISTER => Ok(0x87),
        CHIP_ID_LOW_REGISTER => Ok(self.chip_id_low.get()),
        LDN_ACTIVATION_REGISTER => Ok(self.activation.get()),
        HM_BASE_HIGH_REGISTER => {
          self.base_high_read.set(true);
          Ok(self.base_high.get())
        }
        HM_BASE_LOW_REGISTER => {
          self.base_low_read.set(true);
          Ok(self.base_low.get())
        }
        _ => Ok(0),
      }
    }

    fn superio_outb(&self, register: u8, value: u8) -> Result<(), String> {
      match (register, value) {
        (LOGICAL_DEVICE_SELECT_REGISTER, value) => {
          self.selected_ldn.set(Some(value));
        }
        (0x02, 0x02) => {
          self.config_exit_calls.set(self.config_exit_calls.get() + 1);
          if self.fail_config_exit.get() {
            return Err("simulated ITE config exit failure".to_string());
          }
        }
        _ => {}
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
  fn discover_slot_accepts_exact_d421_id() {
    let client = FakeLpcIo::with_chip_id(D421_CHIP_ID);

    let detected = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0)
      .unwrap()
      .unwrap();

    assert_eq!(detected.profile.chip_label, "NCT6796D/NCT6796D-E");
    assert_eq!(detected.profile.source_label, "Nuvoton / Super I/O");
    assert!(detected.profile.is_experimental());
    assert!(client.hm_bars_authorized.get());
  }

  #[test]
  fn discover_slot_rejects_another_nuvoton_id() {
    for raw_id in [0xD422, 0xC568, 0xC800, 0xD428, 0xD800, 0xD807] {
      let client = FakeLpcIo::with_chip_id(raw_id);

      let detected = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0).unwrap();

      assert!(detected.is_none(), "0x{raw_id:04X}");
      assert_eq!(client.find_bars_calls.get(), 0);
      assert_eq!(client.selected_ldn.get(), None);
      assert_eq!(
        client.superio_reads.borrow().as_slice(),
        &[CHIP_ID_HIGH_REGISTER, CHIP_ID_LOW_REGISTER]
      );
    }
  }

  #[test]
  fn d421_failure_is_identified_as_experimental() {
    let client = FakeLpcIo::with_chip_id(D421_CHIP_ID);
    client.activation.set(0x00);

    let error = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0).unwrap_err();

    assert!(error.starts_with(D421_FAILURE_PREFIX));
    assert!(error.contains("logical device is inactive"));
  }

  #[test]
  fn d802_failure_keeps_the_plain_verified_reason() {
    let client = FakeLpcIo::new();
    client.activation.set(0x00);

    let error = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0).unwrap_err();

    assert_eq!(error, "Nuvoton hardware monitor logical device is inactive");
  }

  #[test]
  fn d421_uses_the_scoped_bank4_temperature_and_direct_rpm_path() {
    let active = discovered_active(FakeLpcIo::with_chip_id(D421_CHIP_ID));

    let sample = active.sample_unlocked().unwrap();

    assert_eq!(sample.temperatures[0].temperature, 32.0);
    assert_eq!(sample.fan_speeds[0].rpm, Some(0x0320));
    assert_eq!(sample.temperatures[0].source, "Nuvoton / Super I/O");
    assert_eq!(sample.fan_speeds[0].source, "Nuvoton / Super I/O");
    assert!(!sample.temperatures[0].source.contains("Experimental"));
    assert!(!sample.fan_speeds[0].source.contains("Experimental"));
    assert_eq!(
      active.client.read_registers.borrow().as_slice(),
      &SIX_PAIR_READS
    );
  }

  #[test]
  fn d802_and_d421_discovery_skip_the_lock_gate() {
    for raw_id in [D802_CHIP_ID, D421_CHIP_ID] {
      let client = FakeLpcIo::with_chip_id(raw_id);
      // A set lock bit must not matter for rows without the lock gate.
      client.hm_io_space_lock.set(0x10);

      let detected = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0)
        .unwrap()
        .unwrap();

      assert_eq!(detected.hm_base, 0x0290, "0x{raw_id:04X}");
      assert!(
        !client
          .superio_reads
          .borrow()
          .contains(&NUVOTON_HM_IO_SPACE_LOCK_REGISTER),
        "0x{raw_id:04X}"
      );
    }
  }

  #[test]
  fn d421_sample_failure_is_identified_as_experimental() {
    let active = discovered_active(FakeLpcIo::with_chip_id(D421_CHIP_ID));
    active.client.hm_bars_authorized.set(false);

    let error = active.sample_unlocked().unwrap_err();

    assert!(error.starts_with(D421_FAILURE_PREFIX));
    assert!(error.contains("hm ports are not authorized"));
  }

  #[test]
  fn sample_preserves_discovered_hm_bar_authorization() {
    let active = ActiveNuvotonMotherboardSensors {
      client: FakeLpcIo::with_authorized_hm_bars(),
      slot: 0,
      hm_base: 0x0290,
      profile: d802_profile(),
    };

    let sample = active.sample_unlocked().unwrap();

    assert_eq!(active.client.selected_slot_calls.get(), 0);
    assert_eq!(sample.temperatures.len(), 6);
    assert_eq!(sample.fan_speeds.len(), 6);
    assert_eq!(sample.temperatures[0].source, "NCT6799D / Super I/O");
    assert_eq!(
      active.client.read_registers.borrow().as_slice(),
      &SIX_PAIR_READS
    );
  }

  #[test]
  fn seven_input_rows_read_the_seventh_direct_rpm_pair() {
    for raw_id in [0xD429, 0xD42B, 0xD451, 0xD801, 0xD806] {
      let active = discovered_active(FakeLpcIo::with_chip_id(raw_id));

      let sample = active.sample_unlocked().unwrap();

      assert_eq!(sample.temperatures.len(), 6, "0x{raw_id:04X}");
      assert_eq!(sample.fan_speeds.len(), 7, "0x{raw_id:04X}");
      assert_eq!(sample.fan_speeds[6].name, "Fan 7");
      assert_eq!(sample.fan_speeds[6].rpm, Some(0x0320));
      assert_eq!(sample.fan_speeds[6].source, "Nuvoton / Super I/O");
      assert_eq!(
        active.client.read_registers.borrow().as_slice(),
        &[
          0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6,
          0xC7, 0xC8, 0xC9, 0xCA, 0xCB, 0xCE, 0xCF
        ],
        "0x{raw_id:04X}"
      );
    }
  }

  #[test]
  fn six_input_lead_only_rows_do_not_read_the_seventh_pair() {
    for raw_id in [0xC803, 0xC911, 0xC913, 0xD121, 0xD352, 0xD423] {
      let active = discovered_active(FakeLpcIo::with_chip_id(raw_id));

      let sample = active.sample_unlocked().unwrap();

      assert_eq!(sample.fan_speeds.len(), 6, "0x{raw_id:04X}");
      assert_eq!(
        active.client.read_registers.borrow().as_slice(),
        &SIX_PAIR_READS,
        "0x{raw_id:04X}"
      );
    }
  }

  #[test]
  fn nct6779d_range_reads_five_pairs_and_skips_the_lock_gate() {
    for raw_id in [0xC560, 0xC567] {
      let client = FakeLpcIo::with_chip_id(raw_id);
      client.hm_io_space_lock.set(0x10);
      let active = discovered_active(client);

      let sample = active.sample_unlocked().unwrap();

      assert_eq!(active.profile.chip_label, "NCT6779D");
      assert!(
        !active
          .client
          .superio_reads
          .borrow()
          .contains(&NUVOTON_HM_IO_SPACE_LOCK_REGISTER)
      );
      assert_eq!(sample.fan_speeds.len(), 5, "0x{raw_id:04X}");
      assert_eq!(
        active.client.read_registers.borrow().as_slice(),
        &SIX_PAIR_READS[..16],
        "0x{raw_id:04X}"
      );
    }
  }

  #[test]
  fn d42a_reads_only_auxtin1_and_auxtin3() {
    let active = discovered_active(FakeLpcIo::with_chip_id(0xD42A));

    let sample = active.sample_unlocked().unwrap();

    assert_eq!(
      sample
        .temperatures
        .iter()
        .map(|temperature| temperature.name.as_str())
        .collect::<Vec<_>>(),
      ["AUXTIN1", "AUXTIN3"]
    );
    assert_eq!(sample.fan_speeds.len(), 7);
    assert_eq!(
      active.client.read_registers.borrow().as_slice(),
      &[
        0x93, 0x95, 0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA,
        0xCB, 0xCE, 0xCF
      ]
    );
  }

  #[test]
  fn lock_gate_reads_cr28_in_configuration_mode_before_ldn_selection() {
    let client = FakeLpcIo::with_chip_id(0xD42B);

    ActiveNuvotonMotherboardSensors::discover_slot(&client, 0)
      .unwrap()
      .unwrap();

    let reads = client.superio_reads.borrow();
    assert_eq!(
      &reads[..3],
      &[
        CHIP_ID_HIGH_REGISTER,
        CHIP_ID_LOW_REGISTER,
        NUVOTON_HM_IO_SPACE_LOCK_REGISTER
      ]
    );
    let writes = client.superio_writes.borrow();
    assert_eq!(
      writes.as_slice(),
      &[(LOGICAL_DEVICE_SELECT_REGISTER, NUVOTON_HARDWARE_MONITOR_LDN)]
    );
  }

  #[test]
  fn set_lock_bit_stops_before_ldn_selection_with_an_experimental_failure() {
    let client = FakeLpcIo::with_chip_id(0xD42B);
    client.hm_io_space_lock.set(0x10);

    let error = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0).unwrap_err();

    assert_eq!(
      error,
      "Experimental Nuvoton motherboard sensor path failed: hardware state (NCT6798D): \
       Nuvoton hardware-monitor I/O space is locked (CR28=0x10, bit 4 set)"
    );
    assert!(error.starts_with(NUVOTON_EXPERIMENTAL_NON_COMPONENT_FAILURE_PREFIX));
    assert!(!is_retryable_init_error(&error));
    assert_eq!(client.selected_ldn.get(), None);
    assert!(client.superio_writes.borrow().is_empty());
    assert!(!client.hm_base_high_read.get());
    assert_eq!(client.find_bars_calls.get(), 0);
    assert!(client.read_registers.borrow().is_empty());
  }

  #[test]
  fn lock_gate_transport_failures_keep_the_component_failure_wording() {
    // A failed CR28 read is a PawnIO/transport problem, not a hardware state.
    struct FailingCr28(FakeLpcIo);
    impl LpcIoOps for FailingCr28 {
      fn select_lpc_slot(&self, slot: u64) -> Result<(), String> {
        self.0.select_lpc_slot(slot)
      }
      fn find_lpc_bars(&self) -> Result<(), String> {
        self.0.find_lpc_bars()
      }
      fn pio_inb(&self, port: u16) -> Result<u8, String> {
        self.0.pio_inb(port)
      }
      fn pio_outb(&self, port: u16, value: u8) -> Result<(), String> {
        self.0.pio_outb(port, value)
      }
      fn superio_inb(&self, register: u8) -> Result<u8, String> {
        if register == NUVOTON_HM_IO_SPACE_LOCK_REGISTER {
          return Err("pawnio_execute ioctl_superio_inb failed".to_string());
        }
        self.0.superio_inb(register)
      }
      fn superio_outb(&self, register: u8, value: u8) -> Result<(), String> {
        self.0.superio_outb(register, value)
      }
    }

    let client = FailingCr28(FakeLpcIo::with_chip_id(0xD42B));

    let error = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0).unwrap_err();

    assert!(error.starts_with("Experimental NCT6798D motherboard sensor path failed: "));
    assert!(!error.starts_with(NUVOTON_EXPERIMENTAL_NON_COMPONENT_FAILURE_PREFIX));
    assert!(error.contains("ioctl_superio_inb failed"));
  }

  #[test]
  fn other_cr28_bits_do_not_engage_the_lock_gate() {
    let client = FakeLpcIo::with_chip_id(0xC803);
    client.hm_io_space_lock.set(0xEF);

    let detected = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0)
      .unwrap()
      .unwrap();

    assert_eq!(detected.profile.chip_label, "NCT6791D");
  }

  #[test]
  fn nct6701d_invalid_temperature_byte_is_reported_unavailable() {
    for invalid in [0x00, 0xA0, 0x7E, 0x7F, 0x80] {
      let client = FakeLpcIo::with_chip_id(0xD806);
      client.temperature_override.set(Some((0x91, invalid)));
      let active = discovered_active(client);

      let sample = active.sample_unlocked().unwrap();

      assert_eq!(
        sample
          .temperatures
          .iter()
          .map(|temperature| temperature.name.as_str())
          .collect::<Vec<_>>(),
        ["SYSTIN", "AUXTIN0", "AUXTIN1", "AUXTIN2", "AUXTIN3"],
        "0x{invalid:02X}"
      );
      assert_eq!(sample.fan_speeds.len(), 7);
    }
  }

  #[test]
  fn other_rows_decode_a_zero_temperature_byte() {
    let client = FakeLpcIo::with_chip_id(0xD801);
    client.temperature_override.set(Some((0x91, 0x00)));
    let active = discovered_active(client);

    let sample = active.sample_unlocked().unwrap();

    assert_eq!(sample.temperatures.len(), 6);
    assert_eq!(sample.temperatures[1].temperature, 0.0);
  }

  #[test]
  fn nct6701d_inactive_ldn_is_unavailable_and_never_activated() {
    let client = FakeLpcIo::with_chip_id(0xD806);
    client.activation.set(0x00);

    let error = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0).unwrap_err();

    assert_eq!(
      error,
      "Experimental Nuvoton motherboard sensor path failed: hardware state (NCT6701D): \
       Nuvoton hardware monitor logical device is inactive"
    );
    assert!(error.starts_with(NUVOTON_EXPERIMENTAL_NON_COMPONENT_FAILURE_PREFIX));
    assert_eq!(
      client.superio_writes.borrow().as_slice(),
      &[(LOGICAL_DEVICE_SELECT_REGISTER, NUVOTON_HARDWARE_MONITOR_LDN)]
    );
    assert_eq!(client.find_bars_calls.get(), 0);
  }

  #[test]
  fn nct6701d_invalid_base_is_unavailable_without_a_cr64_fallback() {
    for (high, low) in [(0x00, 0x90), (0x02, 0x91), (0x02, 0x94), (0x12, 0x90)] {
      let client = FakeLpcIo::with_chip_id(0xD806);
      client.hm_base.set((high, low));

      let error = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0).unwrap_err();

      assert!(
        error.starts_with(
          "Experimental Nuvoton motherboard sensor path failed: hardware state (NCT6701D): "
        ),
        "{error}"
      );
      assert!(error.contains("invalid Nuvoton hardware-monitor base"));
      assert!(!client.superio_reads.borrow().contains(&0x64));
      assert!(!client.superio_reads.borrow().contains(&0x65));
      assert_eq!(client.find_bars_calls.get(), 0);
    }
  }

  #[test]
  fn other_experimental_rows_keep_inactive_ldn_as_a_component_failure() {
    // Only the NCT6701D row documents an inactive LDN B as a firmware state.
    let client = FakeLpcIo::with_chip_id(0xD801);
    client.activation.set(0x00);

    let error = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0).unwrap_err();

    assert!(
      error.starts_with("Experimental NCT6796D-S motherboard sensor path failed: ")
    );
    assert!(!error.starts_with(NUVOTON_EXPERIMENTAL_NON_COMPONENT_FAILURE_PREFIX));
    assert!(error.contains("logical device is inactive"));
  }

  #[test]
  fn other_rows_keep_the_standard_base_check() {
    let client = FakeLpcIo::with_chip_id(0xD801);
    client.hm_base.set((0x00, 0x90));

    let detected = ActiveNuvotonMotherboardSensors::discover_slot(&client, 0)
      .unwrap()
      .unwrap();

    assert_eq!(detected.hm_base, 0x0090);
  }

  #[test]
  fn lead_only_experimental_configuration_exit_failure_requires_rediscovery() {
    let error = nuvoton_experimental_failure(
      "NCT6796D-R / NCT5585D",
      format!("{NUVOTON_CONFIGURATION_EXIT_FAILURE_CONTEXT}: simulated exit failure"),
    );

    assert!(error.starts_with(
      "Experimental NCT6796D-R / NCT5585D motherboard sensor path failed: "
    ));
    assert!(is_retryable_init_error(&error));
    assert!(!is_retryable_init_error(&format!(
      "{NUVOTON_CONFIGURATION_EXIT_FAILURE_CONTEXT}: verified D802 failure"
    )));
  }

  #[test]
  fn fan_support_follows_the_selected_provider_path() {
    let nuvoton = ActiveMotherboardSensors::Nuvoton(ActiveNuvotonMotherboardSensors {
      client: FakeLpcIo::with_authorized_hm_bars(),
      slot: 0,
      hm_base: 0x0290,
      profile: d802_profile(),
    });
    let ite = ActiveMotherboardSensors::Ite(ActiveIteMotherboardSensors {
      client: FakeIteLpcIo::new(),
      slot: 0,
      ec_base: 0x0290,
      chip_label: ITE_CHIP_LABEL,
      source_label: ITE_SOURCE_LABEL,
      last_attempt: None,
      partial_failure_logged: false,
    });

    assert_eq!(nuvoton.fan_support(), SensorSupport::Supported);
    assert_eq!(ite.fan_support(), SensorSupport::Unsupported);
  }

  #[test]
  fn ite_discovery_authorizes_ec_ports_only_after_config_exit() {
    let client = FakeIteLpcIo::new();

    let detected = ActiveIteMotherboardSensors::discover_slot(&client, 0)
      .unwrap()
      .unwrap();

    assert_eq!(detected.slot, 0);
    assert_eq!(detected.ec_base, 0x0290);
    assert_eq!(client.selected_slot_calls.get(), 1);
    assert_eq!(client.find_bars_calls.get(), 1);
    assert_eq!(client.config_exit_calls.get(), 1);
    assert!(client.ec_bars_authorized.get());
    assert_eq!(client.read_registers.borrow().as_slice(), &[0x00]);
    assert_eq!(
      client.config_port_writes.borrow().as_slice(),
      &[
        (0x002E, 0x87),
        (0x002E, 0x01),
        (0x002E, 0x55),
        (0x002E, 0x55)
      ]
    );
  }

  #[test]
  fn ite_discovery_uses_the_slot_one_entry_key() {
    let client = FakeIteLpcIo::new();

    let detected = ActiveIteMotherboardSensors::discover_slot(&client, 1)
      .unwrap()
      .unwrap();

    assert_eq!(detected.slot, 1);
    assert_eq!(
      client.config_port_writes.borrow().as_slice(),
      &[
        (0x004E, 0x87),
        (0x004E, 0x01),
        (0x004E, 0x55),
        (0x004E, 0xAA)
      ]
    );
  }

  #[test]
  fn ite_discovery_does_not_cache_when_the_post_exit_probe_fails() {
    let client = FakeIteLpcIo::new();
    client
      .failed_temperature_register
      .set(Some(ITE_EC_CONFIGURATION_REGISTER));

    let error = ActiveIteMotherboardSensors::discover_slot(&client, 0).unwrap_err();

    assert!(error.starts_with(ITE_EXPERIMENTAL_FAILURE_PREFIX));
    assert!(error.contains("EC port authorization probe failed"));
    assert!(is_retryable_init_error(&error));
    assert_eq!(client.find_bars_calls.get(), 1);
    assert_eq!(client.config_exit_calls.get(), 1);
    assert_eq!(client.read_registers.borrow().as_slice(), &[0x00]);
  }

  #[test]
  fn ite_configuration_exit_failure_requires_rediscovery() {
    let error = ite_experimental_failure(format!(
      "{ITE_CONFIGURATION_EXIT_FAILURE_CONTEXT}: simulated exit failure"
    ));

    assert!(is_retryable_init_error(&error));
  }

  #[test]
  fn nuvoton_experimental_configuration_exit_failure_requires_rediscovery() {
    let error = nuvoton_experimental_failure(
      d421_profile_label(),
      format!("{NUVOTON_CONFIGURATION_EXIT_FAILURE_CONTEXT}: simulated exit failure"),
    );

    assert!(error.starts_with(D421_FAILURE_PREFIX));
    assert!(is_retryable_init_error(&error));
  }

  fn d421_profile_label() -> &'static str {
    nuvoton_hm_profile(D421_CHIP_ID).unwrap().chip_label
  }

  #[test]
  fn ite_discovery_and_configuration_exit_failure_requires_rediscovery() {
    let client = FakeIteLpcIo::new();
    client.activation.set(0x00);
    client.fail_config_exit.set(true);

    let error = ActiveIteMotherboardSensors::discover_slot(&client, 0).unwrap_err();

    assert!(error.starts_with(ITE_EXPERIMENTAL_FAILURE_PREFIX));
    assert!(error.contains(ITE_CONFIGURATION_EXIT_FAILURE_CONTEXT));
    assert!(is_retryable_init_error(&error));
    assert_eq!(client.config_exit_calls.get(), 1);
  }

  #[test]
  fn ite_discovery_rejects_the_conflicting_8721_id() {
    let client = FakeIteLpcIo::new();
    client.chip_id_low.set(0x21);

    let detected = ActiveIteMotherboardSensors::discover_slot(&client, 0).unwrap();

    assert!(detected.is_none());
    assert_eq!(client.find_bars_calls.get(), 0);
    assert_eq!(client.config_exit_calls.get(), 1);
    assert!(client.read_registers.borrow().is_empty());
  }

  #[test]
  fn ite_discovery_rejects_invalid_raw_base_bits_before_bar_authorization() {
    let client = FakeIteLpcIo::new();
    client.base_high.set(0xF2);

    let error = ActiveIteMotherboardSensors::discover_slot(&client, 0).unwrap_err();

    assert!(error.starts_with(ITE_EXPERIMENTAL_FAILURE_PREFIX));
    assert!(error.contains("CR60/61=0xF2/0x90"));
    assert_eq!(client.find_bars_calls.get(), 0);
    assert_eq!(client.config_exit_calls.get(), 1);
    assert!(client.read_registers.borrow().is_empty());
  }

  #[test]
  fn ite_base_validation_rejects_alignment_reserved_bits_and_absent_value() {
    assert!(is_valid_ite_ec_base(0x02, 0x90, 0x0290));
    assert!(!is_valid_ite_ec_base(0xF2, 0x90, 0x0290));
    assert!(!is_valid_ite_ec_base(0x02, 0x91, 0x0290));
    assert!(!is_valid_ite_ec_base(0x00, 0x00, 0x0000));
    assert!(!is_valid_ite_ec_base(0x0F, 0xF8, ITE_ABSENT_EC_BASE));
  }

  #[test]
  fn ite_sample_keeps_successful_siblings_and_never_reads_fans() {
    let client = FakeIteLpcIo::with_authorized_ec(0b0010_1010);
    client.temperature_values.set([25, 30, 0x80]);
    client.failed_temperature_register.set(Some(0x2A));
    let mut active = ActiveIteMotherboardSensors {
      client,
      slot: 0,
      ec_base: 0x0290,
      chip_label: ITE_CHIP_LABEL,
      source_label: ITE_SOURCE_LABEL,
      last_attempt: None,
      partial_failure_logged: false,
    };

    let sample = active.sample_unlocked_at(Instant::now()).unwrap();

    assert_eq!(
      sample.temperatures,
      vec![MotherboardTemperature {
        name: "TMPIN1".to_string(),
        temperature: 25.0,
        source: ITE_SOURCE_LABEL.to_string(),
      }]
    );
    assert!(sample.fan_speeds.is_empty());
    assert_eq!(
      active.client.read_registers.borrow().as_slice(),
      &[0x00, 0x51, 0x29, 0x2A, 0x2B]
    );
    assert!(!sample.temperatures[0].source.contains("Experimental"));
  }

  #[test]
  fn ite_sample_stops_when_the_monitoring_state_is_not_running() {
    let client = FakeIteLpcIo::with_authorized_ec(0b0000_1000);
    client.ec_configuration.set(0x09);
    let mut active = ActiveIteMotherboardSensors {
      client,
      slot: 0,
      ec_base: 0x0290,
      chip_label: ITE_CHIP_LABEL,
      source_label: ITE_SOURCE_LABEL,
      last_attempt: None,
      partial_failure_logged: false,
    };

    let error = active.sample_unlocked_at(Instant::now()).unwrap_err();

    assert!(error.starts_with(ITE_EXPERIMENTAL_FAILURE_PREFIX));
    assert!(error.contains("stopped by INT_Clear"));
    assert_eq!(active.client.read_registers.borrow().as_slice(), &[0x00]);
  }

  #[test]
  fn ite_sample_reuses_the_last_result_inside_the_minimum_interval() {
    let client = FakeIteLpcIo::with_authorized_ec(0b0000_1000);
    let mut active = ActiveIteMotherboardSensors {
      client,
      slot: 0,
      ec_base: 0x0290,
      chip_label: ITE_CHIP_LABEL,
      source_label: ITE_SOURCE_LABEL,
      last_attempt: None,
      partial_failure_logged: false,
    };
    let first_at = Instant::now();

    let first = active.sample_unlocked_at(first_at).unwrap();
    let cached = active
      .sample_unlocked_at(first_at + Duration::from_secs(1))
      .unwrap();

    assert_eq!(cached, first);
    assert_eq!(
      active.client.read_registers.borrow().as_slice(),
      &[0x00, 0x51, 0x29]
    );

    active
      .sample_unlocked_at(first_at + ITE_SAMPLE_MIN_INTERVAL)
      .unwrap();
    assert_eq!(
      active.client.read_registers.borrow().as_slice(),
      &[0x00, 0x51, 0x29, 0x00, 0x51, 0x29]
    );
  }
}
