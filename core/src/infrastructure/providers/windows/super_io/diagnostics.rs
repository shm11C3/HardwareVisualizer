use std::time::Duration;

use super::super::pawn_io::{
  ACCESS_ISABUS_MUTEX, NamedMutex, PawnIoClient, PawnIoDiscovery, PawnIoModule,
};
use super::access::{chip_id, is_absent_id, should_withhold_ite_exit, slot_ports};
use super::motherboard::LpcIoOps;
use crate::models::hardware::{
  PawnIoRuntimeDiagnostics, SuperIoChipIdAttempt, SuperIoChipIdDiagnostics,
  SuperIoChipIdSlotProbe, SuperIoVendor,
};

const ISABUS_MUTEX_TIMEOUT: Duration = Duration::from_millis(50);
const CHIP_ID_HIGH_REGISTER: u8 = 0x20;
const CHIP_ID_LOW_REGISTER: u8 = 0x21;

/// Read raw Super I/O chip-id registers through PawnIO's `LpcIO` module.
///
/// This diagnostic intentionally stops at chip-id discovery. It does not read
/// hardware-monitor registers, fan counters, voltages, or temperature sensors.
/// Writes are limited to configuration-mode enter/exit key sequences and the
/// ITE documented exit register, matching `docs/specs/sensors/superio-access.md`
/// revision 4. On slot 1 the ITE exit is withheld for the chip IDs of that
/// spec's "Slot-1 ITE exit withhold rule" and reported as skipped.
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

fn probe_slot(client: &impl LpcIoOps, slot: u8) -> SuperIoChipIdSlotProbe {
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

fn probe_nuvoton(client: &impl LpcIoOps, index_port: u16) -> SuperIoChipIdAttempt {
  let result = enter_nuvoton(client, index_port).and_then(|_| read_chip_id(client));
  let exit_error = exit_nuvoton(client, index_port).err();
  attempt_from_result(SuperIoVendor::Nuvoton, result, exit_error, false)
}

fn probe_ite(client: &impl LpcIoOps, slot: u8, index_port: u16) -> SuperIoChipIdAttempt {
  let result = enter_ite(client, slot, index_port).and_then(|_| read_chip_id(client));
  let exit_skipped = matches!(
    result,
    Ok((id_high, id_low)) if should_withhold_ite_exit(slot, chip_id(id_high, id_low))
  );
  let exit_error = if exit_skipped {
    None
  } else {
    exit_ite(client).err()
  };
  attempt_from_result(SuperIoVendor::Ite, result, exit_error, exit_skipped)
}

fn enter_nuvoton(client: &impl LpcIoOps, index_port: u16) -> Result<(), String> {
  client.pio_outb(index_port, 0x87)?;
  client.pio_outb(index_port, 0x87)
}

fn exit_nuvoton(client: &impl LpcIoOps, index_port: u16) -> Result<(), String> {
  client.pio_outb(index_port, 0xAA)
}

fn enter_ite(client: &impl LpcIoOps, slot: u8, index_port: u16) -> Result<(), String> {
  let fourth_key = if slot == 0 { 0x55 } else { 0xAA };
  for key in [0x87, 0x01, 0x55, fourth_key] {
    client.pio_outb(index_port, key)?;
  }
  Ok(())
}

fn exit_ite(client: &impl LpcIoOps) -> Result<(), String> {
  client.superio_outb(0x02, 0x02)
}

fn read_chip_id(client: &impl LpcIoOps) -> Result<(u8, u8), String> {
  let high = client.superio_inb(CHIP_ID_HIGH_REGISTER)?;
  let low = client.superio_inb(CHIP_ID_LOW_REGISTER)?;
  Ok((high, low))
}

fn attempt_from_result(
  vendor: SuperIoVendor,
  result: Result<(u8, u8), String>,
  exit_error: Option<String>,
  exit_skipped: bool,
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
        exit_skipped,
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
      exit_skipped,
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

#[cfg(test)]
mod tests {
  use std::cell::{Cell, RefCell};

  use super::*;

  /// Chip-id fake: every slot answers with the same raw bytes, and every
  /// configuration-register write is recorded.
  struct FakeChipIdLpcIo {
    chip_id: u16,
    superio_writes: RefCell<Vec<(u8, u8)>>,
    selected_slot: Cell<Option<u64>>,
  }

  impl FakeChipIdLpcIo {
    fn with_chip_id(chip_id: u16) -> Self {
      Self {
        chip_id,
        superio_writes: RefCell::new(Vec::new()),
        selected_slot: Cell::new(None),
      }
    }

    fn exit_writes(&self) -> usize {
      self
        .superio_writes
        .borrow()
        .iter()
        .filter(|(register, _)| *register == 0x02)
        .count()
    }
  }

  impl LpcIoOps for FakeChipIdLpcIo {
    fn select_lpc_slot(&self, slot: u64) -> Result<(), String> {
      self.selected_slot.set(Some(slot));
      Ok(())
    }

    fn find_lpc_bars(&self) -> Result<(), String> {
      Err("chip-id diagnostic must not discover BARs".to_string())
    }

    fn pio_inb(&self, port: u16) -> Result<u8, String> {
      Err(format!(
        "chip-id diagnostic must not read port 0x{port:04X}"
      ))
    }

    fn pio_outb(&self, port: u16, _value: u8) -> Result<(), String> {
      let expected = match self.selected_slot.get() {
        Some(0) => 0x2E,
        Some(1) => 0x4E,
        _ => return Err("no slot selected".to_string()),
      };
      if port == expected {
        Ok(())
      } else {
        Err(format!("unexpected index port 0x{port:04X}"))
      }
    }

    fn superio_inb(&self, register: u8) -> Result<u8, String> {
      match register {
        0x20 => Ok((self.chip_id >> 8) as u8),
        0x21 => Ok(self.chip_id as u8),
        _ => Err(format!("unexpected config read 0x{register:02X}")),
      }
    }

    fn superio_outb(&self, register: u8, value: u8) -> Result<(), String> {
      self.superio_writes.borrow_mut().push((register, value));
      Ok(())
    }
  }

  fn ite_attempt(probe: &SuperIoChipIdSlotProbe) -> &SuperIoChipIdAttempt {
    probe
      .attempts
      .iter()
      .find(|attempt| attempt.vendor == SuperIoVendor::Ite)
      .expect("ITE attempt")
  }

  #[test]
  fn slot_one_hazard_id_withholds_the_ite_exit_and_reports_it_skipped() {
    for id in [0x8733, 0x8790, 0x8695, 0x8883] {
      let client = FakeChipIdLpcIo::with_chip_id(id);

      let probe = probe_slot(&client, 1);

      assert_eq!(client.exit_writes(), 0, "id 0x{id:04X}");
      let attempt = ite_attempt(&probe);
      assert!(attempt.exit_skipped);
      assert_eq!(attempt.exit_error, None);
      assert_eq!(attempt.error, None);
      assert_eq!(attempt.id_high, Some((id >> 8) as u8));
      assert_eq!(attempt.id_low, Some(id as u8));
      assert_eq!(attempt.chip_id, Some(id));
      assert!(!attempt.absent);
    }
  }

  #[test]
  fn slot_one_non_hazard_ite_id_still_issues_the_exit() {
    let client = FakeChipIdLpcIo::with_chip_id(0x8728);

    let probe = probe_slot(&client, 1);

    assert_eq!(*client.superio_writes.borrow(), vec![(0x02, 0x02)]);
    let attempt = ite_attempt(&probe);
    assert!(!attempt.exit_skipped);
    assert_eq!(attempt.exit_error, None);
  }

  #[test]
  fn slot_zero_hazard_id_still_issues_the_exit() {
    let client = FakeChipIdLpcIo::with_chip_id(0x8733);

    let probe = probe_slot(&client, 0);

    assert_eq!(*client.superio_writes.borrow(), vec![(0x02, 0x02)]);
    assert!(!ite_attempt(&probe).exit_skipped);
  }

  #[test]
  fn nuvoton_attempt_never_reports_a_skipped_exit() {
    let client = FakeChipIdLpcIo::with_chip_id(0x8733);

    let probe = probe_slot(&client, 1);

    let nuvoton = probe
      .attempts
      .iter()
      .find(|attempt| attempt.vendor == SuperIoVendor::Nuvoton)
      .expect("Nuvoton attempt");
    assert!(!nuvoton.exit_skipped);
    assert_eq!(nuvoton.exit_error, None);
  }
}
