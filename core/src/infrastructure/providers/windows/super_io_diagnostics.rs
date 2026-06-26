use std::time::Duration;

use super::pawn_io::{
  ACCESS_ISABUS_MUTEX, NamedMutex, PawnIoClient, PawnIoDiscovery, PawnIoModule,
};
use crate::models::hardware::{
  PawnIoRuntimeDiagnostics, SuperIoChipIdAttempt, SuperIoChipIdDiagnostics,
  SuperIoChipIdSlotProbe, SuperIoVendor,
};
use crate::utils::super_io::{chip_id, is_absent_id, slot_ports};

const ISABUS_MUTEX_TIMEOUT: Duration = Duration::from_millis(50);
const CHIP_ID_HIGH_REGISTER: u8 = 0x20;
const CHIP_ID_LOW_REGISTER: u8 = 0x21;

/// Read raw Super I/O chip-id registers through PawnIO's `LpcIO` module.
///
/// This diagnostic intentionally stops at chip-id discovery. It does not read
/// hardware-monitor registers, fan counters, voltages, or temperature sensors.
/// Writes are limited to configuration-mode enter/exit key sequences and the
/// ITE documented exit register, matching `docs/specs/sensors/superio-access.md`.
pub fn read_super_io_chip_id_diagnostics() -> SuperIoChipIdDiagnostics {
  let (client, discovery) = match PawnIoClient::open(PawnIoModule::LpcIo) {
    Ok((client, discovery)) => (client, discovery),
    Err(error) => {
      return SuperIoChipIdDiagnostics {
        platform_supported: true,
        pawnio: Some(map_pawnio_discovery(&error.discovery)),
        slots: Vec::new(),
        error: Some(error.reason),
      };
    }
  };

  let pawnio = map_pawnio_discovery(&discovery);
  let _mutex = match NamedMutex::acquire(ACCESS_ISABUS_MUTEX, ISABUS_MUTEX_TIMEOUT) {
    Ok(mutex) => mutex,
    Err(reason) => {
      return SuperIoChipIdDiagnostics {
        platform_supported: true,
        pawnio: Some(pawnio),
        slots: Vec::new(),
        error: Some(reason),
      };
    }
  };

  SuperIoChipIdDiagnostics {
    platform_supported: true,
    pawnio: Some(pawnio),
    slots: vec![probe_slot(&client, 0), probe_slot(&client, 1)],
    error: None,
  }
}

fn probe_slot(client: &PawnIoClient, slot: u8) -> SuperIoChipIdSlotProbe {
  let Some((index_port, data_port)) = slot_ports(slot) else {
    return SuperIoChipIdSlotProbe {
      slot,
      index_port: 0,
      data_port: 0,
      attempts: Vec::new(),
      error: Some(format!("unsupported Super I/O LpcIO slot {slot}")),
    };
  };
  let mut probe = SuperIoChipIdSlotProbe {
    slot,
    index_port,
    data_port,
    attempts: Vec::new(),
    error: None,
  };

  if let Err(reason) = client.select_lpc_slot(slot as u64) {
    probe.error = Some(reason);
    return probe;
  }

  probe.attempts.push(probe_nuvoton(client, index_port));
  probe.attempts.push(probe_ite(client, slot, index_port));
  probe
}

fn probe_nuvoton(client: &PawnIoClient, index_port: u16) -> SuperIoChipIdAttempt {
  let result = enter_nuvoton(client, index_port).and_then(|_| read_chip_id(client));
  let exit_error = exit_nuvoton(client, index_port).err();
  attempt_from_result(SuperIoVendor::Nuvoton, result, exit_error)
}

fn probe_ite(client: &PawnIoClient, slot: u8, index_port: u16) -> SuperIoChipIdAttempt {
  let result = enter_ite(client, slot, index_port).and_then(|_| read_chip_id(client));
  let exit_error = exit_ite(client).err();
  attempt_from_result(SuperIoVendor::Ite, result, exit_error)
}

fn enter_nuvoton(client: &PawnIoClient, index_port: u16) -> Result<(), String> {
  client.pio_outb(index_port, 0x87)?;
  client.pio_outb(index_port, 0x87)
}

fn exit_nuvoton(client: &PawnIoClient, index_port: u16) -> Result<(), String> {
  client.pio_outb(index_port, 0xAA)
}

fn enter_ite(client: &PawnIoClient, slot: u8, index_port: u16) -> Result<(), String> {
  let fourth_key = if slot == 0 { 0x55 } else { 0xAA };
  for key in [0x87, 0x01, 0x55, fourth_key] {
    client.pio_outb(index_port, key)?;
  }
  Ok(())
}

fn exit_ite(client: &PawnIoClient) -> Result<(), String> {
  client.superio_outb(0x02, 0x02)
}

fn read_chip_id(client: &PawnIoClient) -> Result<(u8, u8), String> {
  let high = client.superio_inb(CHIP_ID_HIGH_REGISTER)?;
  let low = client.superio_inb(CHIP_ID_LOW_REGISTER)?;
  Ok((high, low))
}

fn attempt_from_result(
  vendor: SuperIoVendor,
  result: Result<(u8, u8), String>,
  exit_error: Option<String>,
) -> SuperIoChipIdAttempt {
  match result {
    Ok((id_high, id_low)) => {
      let absent = is_absent_id(id_high, id_low);
      SuperIoChipIdAttempt {
        vendor,
        id_high: Some(id_high),
        id_low: Some(id_low),
        chip_id: (!absent).then_some(chip_id(id_high, id_low)),
        absent,
        error: None,
        exit_error,
      }
    }
    Err(error) => SuperIoChipIdAttempt {
      vendor,
      id_high: None,
      id_low: None,
      chip_id: None,
      absent: false,
      error: Some(error),
      exit_error,
    },
  }
}

fn map_pawnio_discovery(discovery: &PawnIoDiscovery) -> PawnIoRuntimeDiagnostics {
  PawnIoRuntimeDiagnostics {
    install_location: discovery
      .install_location
      .as_ref()
      .map(|p| p.display().to_string()),
    dll_path: discovery.dll_path.as_ref().map(|p| p.display().to_string()),
    module_path: discovery
      .module_path
      .as_ref()
      .map(|p| p.display().to_string()),
    pawnio_available: discovery.pawnio_available,
    library_loadable: discovery.library_loadable,
    driver_openable: discovery.driver_openable,
    module_loadable: discovery.module_loadable,
    version: discovery.version,
    fallback_reason: discovery.fallback_reason.clone(),
  }
}
