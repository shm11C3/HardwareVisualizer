//! Sampling-path rediscovery cadence and owed configuration cleanup for the
//! Super I/O motherboard sampler.
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-access.md` revision 5, "Sampling-path
//!   rediscovery cadence" (eligibility classes, cleanup first, rules R1-R8)
//!
//! No other external sensor source was used.
//!
//! The state here outlives every sampler: the sensor service recreates an
//! unavailable sampler, and rule R4 makes the recreated sampler inherit the
//! backoff. The failure count and the last event belong to one monitoring
//! session; owed cleanup survives session ends, because only a completed
//! cleanup clears it.

use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use super::super::access::{chip_id, should_withhold_ite_exit, slot_ports};
use super::{
  CHIP_ID_HIGH_REGISTER, CHIP_ID_LOW_REGISTER, LpcIoOps, MotherboardSensorError,
  MotherboardSensorSample, enter_ite, enter_nuvoton, exit_ite, exit_nuvoton,
};

/// R2: the first backoff delay, `D(1)`.
const FIRST_REDISCOVERY_DELAY: Duration = Duration::from_secs(2);
/// R2: the cap on `D(n)`. R4 also uses it as the delay of a recreated
/// sampler after a not-eligible failure.
const MAX_REDISCOVERY_DELAY: Duration = Duration::from_secs(30);
/// `2 s × 2^5` already exceeds the cap, so larger exponents are not needed.
const MAX_DELAY_EXPONENT: u32 = 5;

/// R2: `D(n) = min(2 s × 2^(n−1), 30 s)`. A count of 0 is treated as 1.
pub(super) fn rediscovery_delay(failure_count: u32) -> Duration {
  let exponent = failure_count.saturating_sub(1).min(MAX_DELAY_EXPONENT);
  (FIRST_REDISCOVERY_DELAY * (1_u32 << exponent)).min(MAX_REDISCOVERY_DELAY)
}

/// The configuration-mode key sequence a probe wrote to a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConfigurationSequence {
  /// Winbond/Nuvoton lineage: `0x87`, `0x87` to enter, `0xAA` to exit.
  NuvotonLineage,
  /// ITE MB PnP: the slot's four entry keys, then register `0x02` = `0x02`.
  Ite,
}

impl ConfigurationSequence {
  fn enter(
    self,
    client: &impl LpcIoOps,
    slot: u8,
    index_port: u16,
  ) -> Result<(), String> {
    match self {
      Self::NuvotonLineage => enter_nuvoton(client, index_port),
      Self::Ite => enter_ite(client, slot, index_port),
    }
  }

  fn exit(self, client: &impl LpcIoOps, index_port: u16) -> Result<(), String> {
    match self {
      Self::NuvotonLineage => exit_nuvoton(client, index_port),
      Self::Ite => exit_ite(client),
    }
  }
}

/// Slots whose entry-key or exit write failed, with the sequence to clean up
/// with. Only a completed cleanup clears a slot.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct OwedCleanup {
  slots: [Option<ConfigurationSequence>; 2],
}

impl OwedCleanup {
  pub(super) fn record(&mut self, slot: u8, sequence: ConfigurationSequence) {
    if let Some(owed) = self.slots.get_mut(usize::from(slot)) {
      *owed = Some(sequence);
    }
  }

  pub(super) fn owed(&self, slot: u8) -> Option<ConfigurationSequence> {
    self.slots.get(usize::from(slot)).copied().flatten()
  }

  fn clear(&mut self, slot: u8) {
    if let Some(owed) = self.slots.get_mut(usize::from(slot)) {
      *owed = None;
    }
  }
}

/// "Cleanup first": run the owed cleanup of each slot in slot order before
/// the normal ordered scan. The caller holds the ISA mutex of the attempt.
///
/// A cleanup selects the slot, writes the recorded entry keys and then the
/// recorded exit, and makes no other access. On slot 1 it reads `0x20`/`0x21`
/// between entry and exit: an ID in the slot-1 withhold set gets no exit
/// write and counts as complete, and a failed ID read gets no exit write and
/// keeps the cleanup owed. Any failure stops the attempt with the cleanup
/// still owed, so no path can be cached.
pub(super) fn run_owed_cleanup(
  client: &impl LpcIoOps,
  owed: &mut OwedCleanup,
) -> Result<(), MotherboardSensorError> {
  for slot in [0_u8, 1] {
    let Some(sequence) = owed.owed(slot) else {
      continue;
    };
    clean_up_slot(client, slot, sequence)?;
    owed.clear(slot);
  }
  Ok(())
}

fn clean_up_slot(
  client: &impl LpcIoOps,
  slot: u8,
  sequence: ConfigurationSequence,
) -> Result<(), MotherboardSensorError> {
  let failure = |detail: String| {
    format!("Super I/O slot {slot} configuration cleanup failed: {detail}")
  };
  let Some((index_port, _data_port)) = slot_ports(slot) else {
    return Err(failure(format!("unsupported LpcIO slot {slot}")).into());
  };

  // A slot-selection failure is not listed as eligible, so it keeps the
  // default not-eligible class. The cleanup stays owed either way.
  client
    .select_lpc_slot(u64::from(slot))
    .map_err(|reason| failure(format!("slot selection failed: {reason}")))?;

  // The remaining failures leave the chip possibly in configuration mode.
  let unconfirmed = |detail: String| {
    MotherboardSensorError::Unclassified(failure(detail)).exit_unconfirmed()
  };
  sequence
    .enter(client, slot, index_port)
    .map_err(|reason| unconfirmed(format!("entry key write failed: {reason}")))?;

  if slot == 1 {
    // The slot-1 exception applies whatever the recorded sequence, and even
    // when the failed attempt never read the ID.
    let (id_high, id_low) = client
      .superio_inb(CHIP_ID_HIGH_REGISTER)
      .and_then(|high| Ok((high, client.superio_inb(CHIP_ID_LOW_REGISTER)?)))
      .map_err(|reason| {
        unconfirmed(format!(
          "chip ID read failed, so the exit stays withheld: {reason}"
        ))
      })?;
    // The withhold set of the slot-1 ITE exit withhold rule.
    if should_withhold_ite_exit(slot, chip_id(id_high, id_low)) {
      // A withheld exit is intentional and completes the cleanup.
      return Ok(());
    }
  }

  sequence
    .exit(client, index_port)
    .map_err(|reason| unconfirmed(format!("exit write failed: {reason}")))
}

/// The session's last cadence event (R2, R4, R6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CadenceEvent {
  EligibleFailure { started_at: Instant },
  NotEligibleFailure { started_at: Instant },
  DiscardedPath { at: Instant },
}

/// The cadence state of the current monitoring session plus the owed cleanup
/// that outlives it.
#[derive(Debug)]
pub(super) struct RediscoveryState {
  session: u64,
  /// `n`: failed attempts and discarded paths since the last successful
  /// sample or the session start.
  failure_count: u32,
  last_event: Option<CadenceEvent>,
  /// The session's most recent failure, reported unchanged while no attempt
  /// is due (R3).
  last_failure: Option<MotherboardSensorError>,
  owed_cleanup: OwedCleanup,
}

impl RediscoveryState {
  const fn new() -> Self {
    Self {
      session: 0,
      failure_count: 0,
      last_event: None,
      last_failure: None,
      owed_cleanup: OwedCleanup {
        slots: [None, None],
      },
    }
  }

  pub(super) const fn session(&self) -> u64 {
    self.session
  }

  pub(super) const fn failure_count(&self) -> u32 {
    self.failure_count
  }

  pub(super) fn owed_cleanup_mut(&mut self) -> &mut OwedCleanup {
    &mut self.owed_cleanup
  }

  #[cfg(test)]
  pub(super) const fn owed_cleanup(&self) -> &OwedCleanup {
    &self.owed_cleanup
  }

  /// The event the next attempt waits from and the delay it waits (R2, R4,
  /// R6). `None` when the session has no event, so an attempt runs at once
  /// (R5). A not-eligible failure stops the sampler that made it; the delay
  /// applies to the first attempt of a recreated sampler.
  fn pending_delay(&self) -> Option<(Instant, Duration)> {
    self.last_event.map(|event| match event {
      CadenceEvent::EligibleFailure { started_at } => {
        (started_at, rediscovery_delay(self.failure_count))
      }
      CadenceEvent::DiscardedPath { at } => (at, rediscovery_delay(self.failure_count)),
      CadenceEvent::NotEligibleFailure { started_at } => {
        (started_at, MAX_REDISCOVERY_DELAY)
      }
    })
  }

  /// When a sampler that has not stopped may start its next attempt. `None`
  /// means at once.
  #[cfg(test)]
  pub(super) fn next_attempt_at(&self) -> Option<Instant> {
    self.pending_delay().map(|(since, delay)| since + delay)
  }

  /// R7: elapsed time is measured on the monotonic clock, and a clock that
  /// reads earlier than the recorded event counts as zero elapsed time.
  pub(super) fn is_attempt_due(&self, now: Instant) -> bool {
    self
      .pending_delay()
      .is_none_or(|(since, delay)| now.saturating_duration_since(since) >= delay)
  }

  /// R3: the session's most recent failure, or *rediscovery pending* when the
  /// session has none.
  pub(super) fn current_failure(&self) -> MotherboardSensorError {
    self
      .last_failure
      .clone()
      .unwrap_or(MotherboardSensorError::RediscoveryPending)
  }

  /// Record an attempt that cached no path. Returns whether the failure is
  /// eligible for rediscovery.
  pub(super) fn record_failed_attempt(
    &mut self,
    started_at: Instant,
    failure: MotherboardSensorError,
  ) -> bool {
    let eligible = failure.is_eligible_for_rediscovery();
    self.failure_count = self.failure_count.saturating_add(1);
    self.last_event = Some(if eligible {
      CadenceEvent::EligibleFailure { started_at }
    } else {
      CadenceEvent::NotEligibleFailure { started_at }
    });
    self.last_failure = Some(failure);
    eligible
  }

  /// Record a sample transaction on a cached path. R5: the first successful
  /// sample resets `n`. A failed sample is not a discovery failure (R1); it
  /// is kept as the failure a later discard reports (R3).
  pub(super) fn record_sample(
    &mut self,
    sample: &Result<MotherboardSensorSample, MotherboardSensorError>,
  ) {
    match sample {
      Ok(_) => {
        self.failure_count = 0;
        self.last_event = None;
        self.last_failure = None;
      }
      Err(failure) => self.last_failure = Some(failure.clone()),
    }
  }

  /// R6: a discarded path increments `n`, and the next attempt waits `D(n)`
  /// from the discard.
  pub(super) fn record_discarded_path(&mut self, at: Instant) {
    self.failure_count = self.failure_count.saturating_add(1);
    self.last_event = Some(CadenceEvent::DiscardedPath { at });
  }

  /// End the monitoring session. R5: the next session starts with `n` = 0
  /// and attempts at its first request. Owed cleanup is kept.
  pub(super) fn end_session(&mut self) {
    self.session = self.session.wrapping_add(1);
    self.failure_count = 0;
    self.last_event = None;
    self.last_failure = None;
  }
}

/// A handle to cadence state shared by every sampler of one owner.
#[derive(Debug, Clone)]
pub(super) struct SharedRediscovery(Arc<Mutex<RediscoveryState>>);

impl SharedRediscovery {
  pub(super) fn new() -> Self {
    Self(Arc::new(Mutex::new(RediscoveryState::new())))
  }

  pub(super) fn lock(&self) -> MutexGuard<'_, RediscoveryState> {
    self
      .0
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
  }
}

/// The process-level holder the sensor service's samplers share.
static PROCESS_REDISCOVERY: LazyLock<SharedRediscovery> =
  LazyLock::new(SharedRediscovery::new);

pub(super) fn process_rediscovery() -> SharedRediscovery {
  PROCESS_REDISCOVERY.clone()
}

/// Called by the sensor service after it releases its motherboard sampling
/// state, which ends the monitoring session.
pub(crate) fn end_motherboard_monitoring_session() {
  PROCESS_REDISCOVERY.lock().end_session();
}

/// A controllable monotonic clock and a fake backend that records every
/// PawnIO open and ISA mutex acquisition (`superio-access.md` rev 5
/// "Fixtures").
#[cfg(test)]
pub(super) mod test_support {
  use std::cell::{Cell, RefCell};
  use std::rc::Rc;
  use std::time::{Duration, Instant};

  use super::super::super::super::client::{ACCESS_ISABUS_MUTEX, MutexError};
  use super::super::{LpcIoOps, MotherboardSensorError, SuperIoBackend};

  /// One recorded backend or bus operation.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub(in super::super) enum BusEvent {
    Open { at_ms: u64 },
    Acquire,
    AcquireTimedOut,
    Release,
    SelectSlot(u8),
    KeyWrite { slot: u8, value: u8 },
    ConfigRead { slot: u8, register: u8 },
    ConfigWrite { slot: u8, register: u8, value: u8 },
    FindBars,
    HmWrite { port: u16, value: u8 },
    HmRead { port: u16 },
  }

  /// A cloneable fake backend. Clones share the clock, the event log, the
  /// mutex state and the executor factory.
  pub(in super::super) struct TestBackend<C> {
    start: Instant,
    now: Rc<Cell<Instant>>,
    pub(in super::super) events: Rc<RefCell<Vec<BusEvent>>>,
    pub(in super::super) mutex_times_out: Rc<Cell<bool>>,
    open: Rc<dyn Fn() -> C>,
  }

  impl<C> Clone for TestBackend<C> {
    fn clone(&self) -> Self {
      Self {
        start: self.start,
        now: Rc::clone(&self.now),
        events: Rc::clone(&self.events),
        mutex_times_out: Rc::clone(&self.mutex_times_out),
        open: Rc::clone(&self.open),
      }
    }
  }

  impl<C> TestBackend<C> {
    /// A backend whose clock starts at "0" with an offset, so a fixture can
    /// also read the clock earlier than an event.
    pub(in super::super) fn new(
      events: Rc<RefCell<Vec<BusEvent>>>,
      open: impl Fn() -> C + 'static,
    ) -> Self {
      let start = Instant::now() + Duration::from_secs(3_600);
      Self {
        start,
        now: Rc::new(Cell::new(start)),
        events,
        mutex_times_out: Rc::new(Cell::new(false)),
        open: Rc::new(open),
      }
    }

    /// The fixture time `ms` milliseconds after the first attempt starts.
    pub(in super::super) fn at(&self, ms: u64) -> Instant {
      self.start + Duration::from_millis(ms)
    }

    pub(in super::super) fn set_ms(&self, ms: u64) {
      self.now.set(self.at(ms));
    }

    pub(in super::super) fn set_instant(&self, at: Instant) {
      self.now.set(at);
    }

    /// The fixture times of every PawnIO open, in order.
    pub(in super::super) fn open_times_ms(&self) -> Vec<u64> {
      self
        .events
        .borrow()
        .iter()
        .filter_map(|event| match event {
          BusEvent::Open { at_ms } => Some(*at_ms),
          _ => None,
        })
        .collect()
    }

    pub(in super::super) fn count(&self, matches: impl Fn(&BusEvent) -> bool) -> usize {
      self
        .events
        .borrow()
        .iter()
        .filter(|event| matches(event))
        .count()
    }

    /// The events recorded after the last PawnIO open.
    pub(in super::super) fn events_of_last_attempt(&self) -> Vec<BusEvent> {
      let events = self.events.borrow();
      let start = events
        .iter()
        .rposition(|event| matches!(event, BusEvent::Open { .. }))
        .expect("an attempt ran");
      events[start..].to_vec()
    }
  }

  /// Releases the fake ISA mutex on drop.
  pub(in super::super) struct TestGuard(Rc<RefCell<Vec<BusEvent>>>);

  impl Drop for TestGuard {
    fn drop(&mut self) {
      self.0.borrow_mut().push(BusEvent::Release);
    }
  }

  impl<C: LpcIoOps> SuperIoBackend for TestBackend<C> {
    type Client = C;
    type IsaGuard = TestGuard;

    fn now(&self) -> Instant {
      self.now.get()
    }

    fn open_lpc_io(&self) -> Result<C, MotherboardSensorError> {
      let at_ms = self
        .now
        .get()
        .saturating_duration_since(self.start)
        .as_millis();
      self.events.borrow_mut().push(BusEvent::Open {
        at_ms: u64::try_from(at_ms).expect("fixture times fit in u64"),
      });
      Ok((self.open)())
    }

    fn acquire_isa_bus(&self) -> Result<TestGuard, MutexError> {
      if self.mutex_times_out.get() {
        // A timed-out wait holds nothing, so nothing is released.
        self.events.borrow_mut().push(BusEvent::AcquireTimedOut);
        return Err(MutexError::Timeout {
          name: ACCESS_ISABUS_MUTEX,
        });
      }
      self.events.borrow_mut().push(BusEvent::Acquire);
      Ok(TestGuard(Rc::clone(&self.events)))
    }
  }
}

#[cfg(test)]
mod tests {
  use std::cell::{Cell, RefCell};
  use std::rc::Rc;

  use super::super::super::super::client::{ACCESS_ISABUS_MUTEX, MutexError};
  use super::super::{
    ActiveMotherboardSensors, MotherboardSamplerCore, MotherboardSensorError,
  };
  use super::test_support::{BusEvent, TestBackend};
  use super::*;

  // --- A slot-aware fake bus -------------------------------------------------

  /// One chip strapped to a slot. Configuration registers other than the
  /// chip ID, `CR30` and `CR60/61` read as `0x00`.
  struct SlotChip {
    chip_id: Cell<u16>,
    activation: Cell<u8>,
    base: Cell<(u8, u8)>,
    fail_nuvoton_entry: Cell<bool>,
    fail_nuvoton_exit: Cell<bool>,
    fail_ite_entry: Cell<bool>,
    fail_ite_exit: Cell<bool>,
    fail_id_read: Cell<bool>,
  }

  impl SlotChip {
    fn absent() -> Self {
      Self {
        chip_id: Cell::new(0xFFFF),
        activation: Cell::new(0x01),
        base: Cell::new((0x02, 0x90)),
        fail_nuvoton_entry: Cell::new(false),
        fail_nuvoton_exit: Cell::new(false),
        fail_ite_entry: Cell::new(false),
        fail_ite_exit: Cell::new(false),
        fail_id_read: Cell::new(false),
      }
    }
  }

  /// Both slots of the shared `LpcIO` executor. Every operation is recorded in
  /// the backend's event log, so bus and mutex events interleave in order.
  struct ScriptedBus {
    events: Rc<RefCell<Vec<BusEvent>>>,
    slots: [SlotChip; 2],
    selected: Cell<Option<u8>>,
    keys: RefCell<Vec<u8>>,
    /// Every hardware-monitor port access fails.
    fail_hm_ports: Cell<bool>,
  }

  #[derive(Clone)]
  struct ScriptedClient(Rc<ScriptedBus>);

  impl ScriptedClient {
    fn record(&self, event: BusEvent) {
      self.0.events.borrow_mut().push(event);
    }

    fn selected(&self) -> u8 {
      self.0.selected.get().expect("a slot is selected")
    }

    fn chip(&self) -> &SlotChip {
      &self.0.slots[usize::from(self.selected())]
    }
  }

  impl LpcIoOps for ScriptedClient {
    fn select_lpc_slot(&self, slot: u64) -> Result<(), String> {
      let slot = u8::try_from(slot).expect("slot 0 or 1");
      self.record(BusEvent::SelectSlot(slot));
      self.0.selected.set(Some(slot));
      self.0.keys.borrow_mut().clear();
      Ok(())
    }

    fn find_lpc_bars(&self) -> Result<(), String> {
      self.record(BusEvent::FindBars);
      Ok(())
    }

    fn pio_inb(&self, port: u16) -> Result<u8, String> {
      self.record(BusEvent::HmRead { port });
      assert_eq!(port, 0x0296, "unexpected port read");
      if self.0.fail_hm_ports.get() {
        return Err("simulated HM port denial".to_string());
      }
      Ok(0x20)
    }

    fn pio_outb(&self, port: u16, value: u8) -> Result<(), String> {
      if matches!(port, 0x0295 | 0x0296) {
        self.record(BusEvent::HmWrite { port, value });
        if self.0.fail_hm_ports.get() {
          return Err("simulated HM port denial".to_string());
        }
        return Ok(());
      }
      let slot = self.selected();
      assert_eq!(
        Some((port, port + 1)),
        slot_ports(slot),
        "configuration key written to another slot's index port"
      );
      self.record(BusEvent::KeyWrite { slot, value });
      let chip = self.chip();
      let mut keys = self.0.keys.borrow_mut();
      keys.push(value);
      let (complete, fails) = match keys.as_slice() {
        [0x87, 0x87] => (true, chip.fail_nuvoton_entry.get()),
        [0x87, 0x01, 0x55, 0x55 | 0xAA] => (true, chip.fail_ite_entry.get()),
        [0xAA] => (true, chip.fail_nuvoton_exit.get()),
        [0x87] | [0x87, 0x01] | [0x87, 0x01, 0x55] => (false, false),
        other => panic!("unexpected key sequence {other:02X?}"),
      };
      if complete {
        keys.clear();
      }
      if fails {
        Err(format!("simulated slot {slot} key write failure"))
      } else {
        Ok(())
      }
    }

    fn superio_inb(&self, register: u8) -> Result<u8, String> {
      let slot = self.selected();
      self.record(BusEvent::ConfigRead { slot, register });
      let chip = self.chip();
      let [id_high, id_low] = chip.chip_id.get().to_be_bytes();
      Ok(match register {
        0x20 | 0x21 if chip.fail_id_read.get() => {
          return Err(format!("simulated slot {slot} chip ID read failure"));
        }
        0x20 => id_high,
        0x21 => id_low,
        0x30 => chip.activation.get(),
        0x60 => chip.base.get().0,
        0x61 => chip.base.get().1,
        _ => 0x00,
      })
    }

    fn superio_outb(&self, register: u8, value: u8) -> Result<(), String> {
      let slot = self.selected();
      self.record(BusEvent::ConfigWrite {
        slot,
        register,
        value,
      });
      if (register, value) == (0x02, 0x02) && self.chip().fail_ite_exit.get() {
        return Err(format!("simulated slot {slot} ITE exit failure"));
      }
      Ok(())
    }
  }

  const D802: u16 = 0xD802;
  const NCT6776: u16 = 0xC333;
  const UNMATCHED: u16 = 0x1234;

  struct Fixture {
    bus: Rc<ScriptedBus>,
    backend: TestBackend<ScriptedClient>,
    rediscovery: SharedRediscovery,
  }

  impl Fixture {
    fn new() -> Self {
      let events = Rc::new(RefCell::new(Vec::new()));
      let bus = Rc::new(ScriptedBus {
        events: Rc::clone(&events),
        slots: [SlotChip::absent(), SlotChip::absent()],
        selected: Cell::new(None),
        keys: RefCell::new(Vec::new()),
        fail_hm_ports: Cell::new(false),
      });
      let shared_bus = Rc::clone(&bus);
      let backend =
        TestBackend::new(events, move || ScriptedClient(Rc::clone(&shared_bus)));
      Self {
        bus,
        backend,
        rediscovery: SharedRediscovery::new(),
      }
    }

    fn slot(&self, slot: usize) -> &SlotChip {
      &self.bus.slots[slot]
    }

    fn sampler(&self) -> MotherboardSamplerCore<TestBackend<ScriptedClient>> {
      MotherboardSamplerCore::new(self.backend.clone(), self.rediscovery.clone())
    }

    fn request(
      &self,
      sampler: &mut MotherboardSamplerCore<TestBackend<ScriptedClient>>,
      ms: u64,
    ) -> Result<crate::models::MotherboardSensorSample, MotherboardSensorError> {
      self.backend.set_ms(ms);
      sampler.sample()
    }

    fn requests(
      &self,
      sampler: &mut MotherboardSamplerCore<TestBackend<ScriptedClient>>,
      times_ms: impl IntoIterator<Item = u64>,
    ) {
      for ms in times_ms {
        let _ = self.request(sampler, ms);
      }
    }

    fn mutex_times_out(&self, times_out: bool) {
      self.backend.mutex_times_out.set(times_out);
    }

    fn failure_count(&self) -> u32 {
      self.rediscovery.lock().failure_count()
    }

    fn owed(&self, slot: u8) -> Option<ConfigurationSequence> {
      self.rediscovery.lock().owed_cleanup().owed(slot)
    }

    /// Requests every `step_ms` from `from_ms` to `to_ms` inclusive.
    fn every(step_ms: u64, from_ms: u64, to_ms: u64) -> impl Iterator<Item = u64> {
      (from_ms..=to_ms).step_by(usize::try_from(step_ms).expect("small step"))
    }
  }

  fn timeout() -> MotherboardSensorError {
    MutexError::Timeout {
      name: ACCESS_ISABUS_MUTEX,
    }
    .into()
  }

  fn is_slot_access(event: &BusEvent, slot: u8) -> bool {
    match event {
      BusEvent::SelectSlot(selected) => *selected == slot,
      BusEvent::KeyWrite { slot: s, .. }
      | BusEvent::ConfigRead { slot: s, .. }
      | BusEvent::ConfigWrite { slot: s, .. } => *s == slot,
      _ => false,
    }
  }

  /// The events of the last attempt between its mutex acquisition and the
  /// first slot selection of the normal scan, i.e. the cleanup step.
  fn cleanup_events(events: &[BusEvent], scan_start_slot: u8) -> Vec<BusEvent> {
    let acquired = events
      .iter()
      .position(|event| *event == BusEvent::Acquire)
      .expect("mutex acquired");
    let body = &events[acquired + 1..];
    let scan = body
      .iter()
      .enumerate()
      .skip(1)
      .find(|(_, event)| **event == BusEvent::SelectSlot(scan_start_slot))
      .map_or(body.len(), |(index, _)| index);
    body[..scan].to_vec()
  }

  // --- R2 delay ---------------------------------------------------------------

  #[test]
  fn delay_doubles_from_two_seconds_and_caps_at_thirty() {
    let delays: Vec<u64> = (1..=8).map(|n| rediscovery_delay(n).as_secs()).collect();
    assert_eq!(delays, [2, 4, 8, 16, 30, 30, 30, 30]);
    assert_eq!(rediscovery_delay(u32::MAX).as_secs(), 30);
  }

  // --- superio-access.md rev 5 "Fixtures" --------------------------------------

  /// Fixtures: "The first attempt ends with an eligible failure (mutex
  /// timeout). Requests at 0.5, 1.0 and 1.999" and "Same, request at 2.0".
  #[test]
  fn requests_before_the_backoff_make_no_access_and_repeat_the_failure() {
    let fixture = Fixture::new();
    fixture.mutex_times_out(true);
    let mut sampler = fixture.sampler();

    let first = fixture.request(&mut sampler, 0).unwrap_err();
    assert_eq!(first, timeout());
    let events_after_first = fixture.backend.events.borrow().len();

    for ms in [500, 1_000, 1_999] {
      let repeated = fixture.request(&mut sampler, ms).unwrap_err();
      assert_eq!(repeated, first);
      assert_eq!(repeated.to_string(), first.to_string());
      assert_eq!(
        repeated.is_eligible_for_rediscovery(),
        first.is_eligible_for_rediscovery()
      );
      assert_eq!(
        repeated.offers_pawnio_guidance(),
        first.offers_pawnio_guidance()
      );
    }
    // No executor open, mutex acquisition or port I/O.
    assert_eq!(fixture.backend.events.borrow().len(), events_after_first);

    let _ = fixture.request(&mut sampler, 2_000);
    assert_eq!(fixture.backend.open_times_ms(), [0, 2_000]);
  }

  /// Fixture: "An eligible failure persists, with requests every 0.1 from 0
  /// to 120".
  #[test]
  fn persistent_eligible_failure_backs_off_to_thirty_seconds() {
    let fixture = Fixture::new();
    fixture.mutex_times_out(true);
    let mut sampler = fixture.sampler();

    fixture.requests(&mut sampler, Fixture::every(100, 0, 120_000));

    assert_eq!(
      fixture.backend.open_times_ms(),
      [0, 2_000, 6_000, 14_000, 30_000, 60_000, 90_000, 120_000]
    );
  }

  /// Fixture: "An eligible failure persists, with requests every 1.6 (service
  /// cadence) from 0 to 64".
  #[test]
  fn service_cadence_rounds_the_backoff_up_to_the_next_request() {
    let fixture = Fixture::new();
    fixture.mutex_times_out(true);
    let mut sampler = fixture.sampler();

    fixture.requests(&mut sampler, Fixture::every(1_600, 0, 64_000));

    assert_eq!(
      fixture.backend.open_times_ms(),
      [0, 3_200, 8_000, 16_000, 32_000, 62_400]
    );
  }

  /// Fixture: "Eligible failures at 0 and 2, a successful attempt and a
  /// successful sample at 6, then the path is discarded at 100".
  #[test]
  fn successful_sample_resets_the_backoff_and_a_discard_counts_once() {
    let fixture = Fixture::new();
    fixture.slot(0).chip_id.set(D802);
    fixture.mutex_times_out(true);
    let mut sampler = fixture.sampler();
    fixture.requests(&mut sampler, [0, 2_000]);
    fixture.mutex_times_out(false);

    assert!(fixture.request(&mut sampler, 6_000).is_ok());
    assert_eq!(fixture.failure_count(), 0);
    fixture.requests(&mut sampler, Fixture::every(1_000, 7_000, 99_000));
    assert_eq!(fixture.backend.open_times_ms(), [0, 2_000, 6_000]);

    // The service recreates the sampler at 100, which discards its path.
    fixture.backend.set_ms(100_000);
    drop(sampler);
    assert_eq!(fixture.failure_count(), 1);
    let mut sampler = fixture.sampler();
    fixture.requests(&mut sampler, [100_000, 101_000, 101_999]);
    assert_eq!(fixture.backend.open_times_ms(), [0, 2_000, 6_000]);
    let _ = fixture.request(&mut sampler, 102_000);
    assert_eq!(fixture.backend.open_times_ms(), [0, 2_000, 6_000, 102_000]);
  }

  /// Fixture: "A successful attempt and sample at 0, the path is invalidated
  /// at 50 with no failure in the session, then a request at 51".
  #[test]
  fn discarded_path_without_a_failure_reports_rediscovery_pending() {
    let fixture = Fixture::new();
    fixture.slot(0).chip_id.set(D802);
    let mut sampler = fixture.sampler();
    assert!(fixture.request(&mut sampler, 0).is_ok());

    fixture.backend.set_ms(50_000);
    drop(sampler);
    let mut sampler = fixture.sampler();

    let pending = fixture.request(&mut sampler, 51_000).unwrap_err();
    assert_eq!(pending, MotherboardSensorError::RediscoveryPending);
    assert!(!pending.is_unsupported());
    assert!(pending.is_eligible_for_rediscovery());
    assert!(!pending.offers_pawnio_guidance());
    assert!(pending.to_string().contains("discarded"));
    assert!(pending.to_string().contains("discovery will run again"));
    assert_eq!(fixture.backend.open_times_ms(), [0]);

    assert!(fixture.request(&mut sampler, 52_000).is_ok());
    assert_eq!(fixture.backend.open_times_ms(), [0, 52_000]);
  }

  /// Fixture: "A successful path whose sample fails at 50, after which the
  /// sampler is recreated".
  #[test]
  fn recreated_sampler_reports_the_sample_failure_before_the_discard() {
    let fixture = Fixture::new();
    fixture.slot(0).chip_id.set(D802);
    let mut sampler = fixture.sampler();
    assert!(fixture.request(&mut sampler, 0).is_ok());

    fixture.bus.fail_hm_ports.set(true);
    let sample_failure = fixture.request(&mut sampler, 50_000).unwrap_err();
    assert!(sample_failure.offers_pawnio_guidance());
    drop(sampler);
    let mut sampler = fixture.sampler();

    for ms in [50_000, 51_000, 51_999] {
      let reported = fixture.request(&mut sampler, ms).unwrap_err();
      assert_eq!(reported, sample_failure);
      assert_eq!(reported.to_string(), sample_failure.to_string());
      assert!(reported.offers_pawnio_guidance());
    }
    assert_eq!(fixture.backend.open_times_ms(), [0]);
    let _ = fixture.request(&mut sampler, 52_000);
    assert_eq!(fixture.backend.open_times_ms(), [0, 52_000]);
  }

  /// Fixture: "Requests every 0.1. Every attempt caches a path, its first
  /// sample fails as a whole, and the path is then discarded".
  #[test]
  fn discovery_success_alone_never_resets_the_backoff() {
    let fixture = Fixture::new();
    fixture.slot(0).chip_id.set(D802);
    fixture.bus.fail_hm_ports.set(true);
    let mut sampler = fixture.sampler();

    for ms in Fixture::every(100, 0, 89_900) {
      let _ = fixture.request(&mut sampler, ms);
      if sampler.active.is_some() {
        drop(sampler);
        sampler = fixture.sampler();
      }
    }

    assert_eq!(
      fixture.backend.open_times_ms(),
      [0, 2_000, 6_000, 14_000, 30_000, 60_000]
    );
  }

  /// A slot-0 D802 with an invalid base: a refused value, not eligible.
  fn refused_base(fixture: &Fixture) {
    fixture.slot(0).chip_id.set(D802);
    fixture.slot(0).base.set((0x00, 0x00));
  }

  /// Fixture: "A not-eligible failure at 0, such as a refused lock bit or
  /// vendor-ID mismatch, with requests every 1.6 for 120 on the same
  /// sampler".
  #[test]
  fn not_eligible_failure_stops_the_sampler() {
    let fixture = Fixture::new();
    refused_base(&fixture);
    let mut sampler = fixture.sampler();

    let first = fixture.request(&mut sampler, 0).unwrap_err();
    assert!(!first.is_eligible_for_rediscovery());
    for ms in Fixture::every(1_600, 1_600, 120_000) {
      assert_eq!(fixture.request(&mut sampler, ms).unwrap_err(), first);
    }

    assert_eq!(fixture.backend.open_times_ms(), [0]);
  }

  /// Fixture: "Same failure, and the sampler is recreated at 10 in the same
  /// session".
  #[test]
  fn recreated_sampler_waits_thirty_seconds_after_a_not_eligible_failure() {
    let fixture = Fixture::new();
    refused_base(&fixture);
    let mut sampler = fixture.sampler();
    let first = fixture.request(&mut sampler, 0).unwrap_err();

    fixture.backend.set_ms(10_000);
    drop(sampler);
    let mut sampler = fixture.sampler();
    // No attempt at creation.
    assert_eq!(fixture.backend.open_times_ms(), [0]);

    for ms in Fixture::every(1_000, 10_000, 29_000) {
      assert_eq!(fixture.request(&mut sampler, ms).unwrap_err(), first);
    }
    assert_eq!(fixture.backend.open_times_ms(), [0]);
    let _ = fixture.request(&mut sampler, 30_000);
    assert_eq!(fixture.backend.open_times_ms(), [0, 30_000]);
  }

  /// Fixture: "Eligible failures at 0, 2 and 6, and the sampler is recreated
  /// at 10".
  #[test]
  fn recreated_sampler_continues_the_eligible_backoff() {
    let fixture = Fixture::new();
    fixture.mutex_times_out(true);
    let mut sampler = fixture.sampler();
    fixture.requests(&mut sampler, Fixture::every(100, 0, 9_900));
    assert_eq!(fixture.backend.open_times_ms(), [0, 2_000, 6_000]);
    assert_eq!(fixture.failure_count(), 3);

    fixture.backend.set_ms(10_000);
    drop(sampler);
    let mut sampler = fixture.sampler();
    fixture.requests(&mut sampler, Fixture::every(100, 10_000, 14_000));

    assert_eq!(fixture.backend.open_times_ms(), [0, 2_000, 6_000, 14_000]);
    assert_eq!(fixture.failure_count(), 4);
  }

  /// Fixture: "In one attempt, slot 0 ends not eligible and slot 1 eligible,
  /// and separately the reverse order".
  #[test]
  fn an_eligible_probe_failure_makes_the_attempt_eligible_in_either_order() {
    for eligible_slot in [1_usize, 0] {
      let fixture = Fixture::new();
      let refused_slot = 1 - eligible_slot;
      fixture.slot(refused_slot).chip_id.set(D802);
      fixture.slot(refused_slot).base.set((0x00, 0x00));
      // A source-selected chip whose vendor-ID port read fails after
      // `ioctl_find_bars`: eligible, with the exit confirmed.
      fixture.slot(eligible_slot).chip_id.set(NCT6776);
      fixture.bus.fail_hm_ports.set(true);
      let mut sampler = fixture.sampler();

      let failure = fixture.request(&mut sampler, 0).unwrap_err();
      assert!(failure.is_eligible_for_rediscovery(), "{failure}");
      assert!(
        failure.to_string().contains("vendor ID read failed"),
        "{failure}"
      );
      fixture.requests(&mut sampler, Fixture::every(100, 100, 2_000));

      assert_eq!(fixture.backend.open_times_ms(), [0, 2_000]);
    }
  }

  /// Fixture: "Slot 0 reads an unmatched chip ID and its exit write fails,
  /// while slot 1 holds a chip that would qualify".
  #[test]
  fn unconfirmed_slot0_exit_stops_the_attempt_before_slot1() {
    let fixture = Fixture::new();
    fixture.slot(0).chip_id.set(UNMATCHED);
    fixture.slot(0).fail_nuvoton_exit.set(true);
    fixture.slot(1).chip_id.set(D802);
    let mut sampler = fixture.sampler();

    let failure = fixture.request(&mut sampler, 0).unwrap_err();

    assert!(failure.is_eligible_for_rediscovery());
    assert!(sampler.active.is_none());
    assert_eq!(fixture.backend.count(|event| is_slot_access(event, 1)), 0);
    assert_eq!(fixture.owed(0), Some(ConfigurationSequence::NuvotonLineage));

    // The next attempt, no earlier than D(1), starts again at slot 0 with its
    // entry keys and exit, and only then may slot 1 qualify.
    fixture.slot(0).fail_nuvoton_exit.set(false);
    fixture.requests(&mut sampler, [1_000, 1_999]);
    assert_eq!(fixture.backend.open_times_ms(), [0]);
    assert!(fixture.request(&mut sampler, 2_000).is_ok());
    let events = fixture.backend.events_of_last_attempt();
    assert_eq!(
      cleanup_events(&events, 0),
      [
        BusEvent::SelectSlot(0),
        BusEvent::KeyWrite {
          slot: 0,
          value: 0x87
        },
        BusEvent::KeyWrite {
          slot: 0,
          value: 0x87
        },
        BusEvent::KeyWrite {
          slot: 0,
          value: 0xAA
        },
      ]
    );
    assert_eq!(fixture.owed(0), None);
    assert!(matches!(
      sampler.active,
      Some(ActiveMotherboardSensors::Nuvoton(ref active)) if active.slot == 1
    ));
  }

  /// Attempt 1 of the slot-1 cleanup fixtures: slot 0 fails its base gate,
  /// then the Nuvoton exit write on slot 1 fails.
  fn slot1_exit_failure_after_a_slot0_refusal(fixture: &Fixture) {
    refused_base(fixture);
    fixture.slot(1).chip_id.set(UNMATCHED);
    fixture.slot(1).fail_nuvoton_exit.set(true);
  }

  const SLOT1_NUVOTON_CLEANUP: [BusEvent; 6] = [
    BusEvent::SelectSlot(1),
    BusEvent::KeyWrite {
      slot: 1,
      value: 0x87,
    },
    BusEvent::KeyWrite {
      slot: 1,
      value: 0x87,
    },
    BusEvent::ConfigRead {
      slot: 1,
      register: 0x20,
    },
    BusEvent::ConfigRead {
      slot: 1,
      register: 0x21,
    },
    BusEvent::KeyWrite {
      slot: 1,
      value: 0xAA,
    },
  ];

  /// Fixture: "Attempt 1: slot 0 fails its base gate, then the exit write on
  /// slot 1 fails. Attempt 2 is due, and slot 0 now qualifies."
  #[test]
  fn owed_slot1_cleanup_runs_before_slot0_is_cached() {
    let fixture = Fixture::new();
    slot1_exit_failure_after_a_slot0_refusal(&fixture);
    let mut sampler = fixture.sampler();
    let failure = fixture.request(&mut sampler, 0).unwrap_err();
    // The unconfirmed exit takes precedence over the earlier refusal.
    assert!(failure.is_eligible_for_rediscovery());
    assert_eq!(fixture.owed(1), Some(ConfigurationSequence::NuvotonLineage));

    fixture.slot(0).base.set((0x02, 0x90));
    fixture.slot(1).fail_nuvoton_exit.set(false);
    assert!(fixture.request(&mut sampler, 2_000).is_ok());

    let events = fixture.backend.events_of_last_attempt();
    assert_eq!(cleanup_events(&events, 0), SLOT1_NUVOTON_CLEANUP);
    assert_eq!(fixture.owed(1), None);
    assert!(matches!(
      sampler.active,
      Some(ActiveMotherboardSensors::Nuvoton(ref active)) if active.slot == 0
    ));
  }

  /// Same fixture: "If the slot 1 cleanup write fails again, attempt 2 makes
  /// no slot 0 access, caches nothing and ends eligible."
  #[test]
  fn failed_slot1_cleanup_makes_no_slot0_access() {
    let fixture = Fixture::new();
    slot1_exit_failure_after_a_slot0_refusal(&fixture);
    let mut sampler = fixture.sampler();
    let _ = fixture.request(&mut sampler, 0);
    fixture.slot(0).base.set((0x02, 0x90));

    let failure = fixture.request(&mut sampler, 2_000).unwrap_err();

    assert!(failure.is_eligible_for_rediscovery());
    assert!(
      failure
        .to_string()
        .contains("slot 1 configuration cleanup failed")
    );
    assert!(sampler.active.is_none());
    let events = fixture.backend.events_of_last_attempt();
    assert!(!events.iter().any(|event| is_slot_access(event, 0)));
    assert_eq!(fixture.owed(1), Some(ConfigurationSequence::NuvotonLineage));
    fixture.requests(&mut sampler, Fixture::every(100, 2_100, 6_000));
    assert_eq!(fixture.backend.open_times_ms(), [0, 2_000, 6_000]);
  }

  /// Fixture: "Cleanup is owed on slot 1, and the service releases the
  /// session before the next attempt".
  #[test]
  fn owed_cleanup_survives_the_session_end() {
    let fixture = Fixture::new();
    slot1_exit_failure_after_a_slot0_refusal(&fixture);
    let mut sampler = fixture.sampler();
    let _ = fixture.request(&mut sampler, 0);
    drop(sampler);
    fixture.rediscovery.lock().end_session();
    fixture.slot(1).fail_nuvoton_exit.set(false);

    // The new session attempts at once, and begins with the cleanup.
    let mut sampler = fixture.sampler();
    let _ = fixture.request(&mut sampler, 500);

    assert_eq!(fixture.backend.open_times_ms(), [0, 500]);
    let events = fixture.backend.events_of_last_attempt();
    assert_eq!(cleanup_events(&events, 0), SLOT1_NUVOTON_CLEANUP);
    assert_eq!(fixture.owed(1), None);
  }

  const SLOT1_ITE_CLEANUP_ENTRY: [BusEvent; 7] = [
    BusEvent::SelectSlot(1),
    BusEvent::KeyWrite {
      slot: 1,
      value: 0x87,
    },
    BusEvent::KeyWrite {
      slot: 1,
      value: 0x01,
    },
    BusEvent::KeyWrite {
      slot: 1,
      value: 0x55,
    },
    BusEvent::KeyWrite {
      slot: 1,
      value: 0xAA,
    },
    BusEvent::ConfigRead {
      slot: 1,
      register: 0x20,
    },
    BusEvent::ConfigRead {
      slot: 1,
      register: 0x21,
    },
  ];

  /// Fixture: "Cleanup owed on slot 1 for an ITE sequence. The cleanup ID
  /// read returns `0x87`/`0x33`".
  #[test]
  fn slot1_ite_cleanup_withholds_the_exit_for_a_protected_id() {
    let fixture = Fixture::new();
    fixture.slot(0).chip_id.set(D802);
    fixture.slot(1).chip_id.set(0x8733);
    fixture
      .rediscovery
      .lock()
      .owed_cleanup_mut()
      .record(1, ConfigurationSequence::Ite);
    let mut sampler = fixture.sampler();

    assert!(fixture.request(&mut sampler, 0).is_ok());

    let events = fixture.backend.events_of_last_attempt();
    // No exit write: the withheld exit completes the cleanup, and the attempt
    // continues to the normal scan.
    assert_eq!(cleanup_events(&events, 0), SLOT1_ITE_CLEANUP_ENTRY);
    assert_eq!(fixture.owed(1), None);
    assert!(sampler.active.is_some());
  }

  /// `superio-access.md` rev 5 "Cleanup first" (also the
  /// `superio-fintek-f71882-f71889.md` rev 2 synthetic case "Slot 1 owes
  /// cleanup recorded by an ITE probe"): an ITE debt is cleaned up with the
  /// ITE slot-1 keys and, for an ID outside the withhold set, the ITE exit;
  /// no Nuvoton-lineage sequence is written.
  #[test]
  fn slot1_ite_cleanup_writes_the_ite_exit_for_another_id() {
    let fixture = Fixture::new();
    fixture.slot(0).chip_id.set(D802);
    fixture.slot(1).chip_id.set(UNMATCHED);
    fixture
      .rediscovery
      .lock()
      .owed_cleanup_mut()
      .record(1, ConfigurationSequence::Ite);
    let mut sampler = fixture.sampler();

    assert!(fixture.request(&mut sampler, 0).is_ok());

    let events = fixture.backend.events_of_last_attempt();
    let mut expected = SLOT1_ITE_CLEANUP_ENTRY.to_vec();
    expected.push(BusEvent::ConfigWrite {
      slot: 1,
      register: 0x02,
      value: 0x02,
    });
    assert_eq!(cleanup_events(&events, 0), expected);
    assert_eq!(fixture.owed(1), None);
  }

  /// Fixture: "Cleanup owed on slot 1 for an ITE sequence, and the cleanup ID
  /// read fails".
  #[test]
  fn slot1_cleanup_with_a_failed_id_read_keeps_the_debt() {
    let fixture = Fixture::new();
    fixture.slot(0).chip_id.set(D802);
    fixture.slot(1).fail_id_read.set(true);
    fixture
      .rediscovery
      .lock()
      .owed_cleanup_mut()
      .record(1, ConfigurationSequence::Ite);
    let mut sampler = fixture.sampler();

    let failure = fixture.request(&mut sampler, 0).unwrap_err();

    assert!(failure.is_eligible_for_rediscovery());
    assert!(sampler.active.is_none());
    let events = fixture.backend.events_of_last_attempt();
    assert!(
      !events
        .iter()
        .any(|event| matches!(event, BusEvent::ConfigWrite { register: 0x02, .. }))
    );
    assert!(!events.iter().any(|event| is_slot_access(event, 0)));
    assert_eq!(fixture.owed(1), Some(ConfigurationSequence::Ite));
    fixture.requests(&mut sampler, Fixture::every(100, 100, 2_000));
    assert_eq!(fixture.backend.open_times_ms(), [0, 2_000]);
  }

  /// Fixture: "Cleanup owed on slot 1 for a Nuvoton-lineage sequence whose
  /// entry write failed before any ID read. The cleanup ID read returns
  /// `0x8790`, and separately an ID outside the set".
  #[test]
  fn slot1_nuvoton_cleanup_applies_the_withhold_set() {
    for (cleanup_id, exit_written) in [(0x8790_u16, false), (UNMATCHED, true)] {
      let fixture = Fixture::new();
      fixture.slot(1).fail_nuvoton_entry.set(true);
      let mut sampler = fixture.sampler();
      let _ = fixture.request(&mut sampler, 0);
      // The failed entry stopped the attempt before any slot-1 ID read.
      assert!(
        !fixture
          .backend
          .events
          .borrow()
          .iter()
          .any(|event| matches!(event, BusEvent::ConfigRead { slot: 1, .. }))
      );
      assert_eq!(fixture.owed(1), Some(ConfigurationSequence::NuvotonLineage));

      fixture.slot(1).fail_nuvoton_entry.set(false);
      fixture.slot(1).chip_id.set(cleanup_id);
      let _ = fixture.request(&mut sampler, 2_000);

      let events = fixture.backend.events_of_last_attempt();
      let mut expected = SLOT1_NUVOTON_CLEANUP.to_vec();
      if !exit_written {
        expected.pop();
      }
      assert_eq!(cleanup_events(&events, 0), expected, "0x{cleanup_id:04X}");
      assert_eq!(fixture.owed(1), None);
    }
  }

  /// Fixture: "An unconfirmed exit on a later probe of slot 0, after an
  /// earlier probe on the same slot found nothing".
  #[test]
  fn unconfirmed_exit_on_a_later_probe_stops_the_attempt() {
    let fixture = Fixture::new();
    fixture.slot(0).chip_id.set(UNMATCHED);
    fixture.slot(0).fail_ite_exit.set(true);
    fixture.slot(1).chip_id.set(D802);
    let mut sampler = fixture.sampler();

    let failure = fixture.request(&mut sampler, 0).unwrap_err();

    assert!(failure.is_eligible_for_rediscovery());
    assert!(sampler.active.is_none());
    assert_eq!(fixture.backend.count(|event| is_slot_access(event, 1)), 0);
    assert_eq!(fixture.owed(0), Some(ConfigurationSequence::Ite));
  }

  /// Fixture: "A due request whose attempt caches a path, followed by a sample
  /// transaction in the same request".
  #[test]
  fn attempt_and_sample_use_two_ordered_mutex_acquisitions() {
    let fixture = Fixture::new();
    fixture.slot(0).chip_id.set(D802);
    let mut sampler = fixture.sampler();

    assert!(fixture.request(&mut sampler, 0).is_ok());

    let events = fixture.backend.events.borrow().clone();
    let acquisitions: Vec<usize> = events
      .iter()
      .enumerate()
      .filter(|(_, event)| **event == BusEvent::Acquire)
      .map(|(index, _)| index)
      .collect();
    let releases: Vec<usize> = events
      .iter()
      .enumerate()
      .filter(|(_, event)| **event == BusEvent::Release)
      .map(|(index, _)| index)
      .collect();
    assert_eq!(acquisitions.len(), 2);
    assert_eq!(releases.len(), 2);
    assert!(acquisitions[0] < releases[0]);
    assert!(releases[0] < acquisitions[1]);
    assert!(acquisitions[1] < releases[1]);

    let is_configuration_key = |event: &BusEvent| {
      matches!(
        event,
        BusEvent::KeyWrite { .. } | BusEvent::ConfigWrite { register: 0x02, .. }
      )
    };
    let key_writes = events
      .iter()
      .filter(|event| is_configuration_key(event))
      .count();
    let first = &events[acquisitions[0]..releases[0]];
    let second = &events[acquisitions[1]..releases[1]];
    assert_eq!(
      first
        .iter()
        .filter(|event| is_configuration_key(event))
        .count(),
      key_writes
    );
    assert!(!second.iter().any(is_configuration_key));
    assert!(
      second
        .iter()
        .any(|event| matches!(event, BusEvent::HmRead { .. }))
    );
  }

  /// Fixture: "A cached path whose samples fail as a whole on several
  /// requests ... while the path stays cached" (shown on the Nuvoton path;
  /// the banked ITE opening check is covered with the ITE fakes).
  #[test]
  fn failed_samples_on_a_cached_path_start_no_attempt() {
    let fixture = Fixture::new();
    fixture.slot(0).chip_id.set(D802);
    let mut sampler = fixture.sampler();
    assert!(fixture.request(&mut sampler, 0).is_ok());
    let key_writes_before = fixture
      .backend
      .count(|event| matches!(event, BusEvent::KeyWrite { .. }));
    let acquisitions_before = fixture.backend.count(|event| *event == BusEvent::Acquire);

    fixture.bus.fail_hm_ports.set(true);
    for ms in Fixture::every(1_600, 1_600, 64_000) {
      assert!(fixture.request(&mut sampler, ms).is_err());
    }

    assert_eq!(fixture.backend.open_times_ms(), [0]);
    assert_eq!(
      fixture
        .backend
        .count(|event| matches!(event, BusEvent::KeyWrite { .. })),
      key_writes_before
    );
    // Samples continue at their own cadence.
    assert_eq!(
      fixture.backend.count(|event| *event == BusEvent::Acquire),
      acquisitions_before + 40
    );
  }

  /// Fixture: "`n` = 5, then the service releases sampling state for
  /// idleness or a sampling gap, then a new request".
  #[test]
  fn session_end_resets_the_backoff() {
    let fixture = Fixture::new();
    fixture.mutex_times_out(true);
    let mut sampler = fixture.sampler();
    fixture.requests(&mut sampler, Fixture::every(100, 0, 30_000));
    assert_eq!(fixture.failure_count(), 5);

    drop(sampler);
    fixture.rediscovery.lock().end_session();
    let mut sampler = fixture.sampler();
    fixture.requests(&mut sampler, Fixture::every(100, 31_000, 33_000));

    assert_eq!(
      fixture.backend.open_times_ms(),
      [0, 2_000, 6_000, 14_000, 30_000, 31_000, 33_000]
    );
  }

  /// A sampler released with its session records no discard in the next
  /// session (the service drops it before ending the session).
  #[test]
  fn a_sampler_dropped_after_its_session_ended_records_no_discard() {
    let fixture = Fixture::new();
    fixture.slot(0).chip_id.set(D802);
    let mut sampler = fixture.sampler();
    assert!(fixture.request(&mut sampler, 0).is_ok());

    fixture.rediscovery.lock().end_session();
    drop(sampler);

    assert_eq!(fixture.failure_count(), 0);
    assert_eq!(fixture.rediscovery.lock().next_attempt_at(), None);
  }

  /// Fixture: "The clock reads earlier than the start of the failed attempt".
  #[test]
  fn a_clock_earlier_than_the_failed_attempt_is_not_due() {
    let fixture = Fixture::new();
    fixture.mutex_times_out(true);
    let mut sampler = fixture.sampler();
    let _ = fixture.request(&mut sampler, 10_000);

    fixture
      .backend
      .set_instant(fixture.backend.at(10_000) - Duration::from_secs(5));
    assert!(sampler.sample().is_err());

    assert_eq!(fixture.backend.open_times_ms(), [10_000]);
    let state = fixture.rediscovery.lock();
    assert!(!state.is_attempt_due(fixture.backend.at(5_000)));
    assert!(!state.is_attempt_due(fixture.backend.at(11_999)));
    assert!(state.is_attempt_due(fixture.backend.at(12_000)));
  }

  /// `superio-access.md` rev 5 shared table: an executor that cannot be
  /// opened is not eligible and is retried only through recreation (R4).
  #[test]
  fn executor_open_failure_is_not_eligible() {
    let failure =
      MotherboardSensorError::Unclassified("PawnIO.dll not found".to_string());
    assert!(!failure.is_eligible_for_rediscovery());

    let mut state = RediscoveryState::new();
    let start = Instant::now();
    assert!(!state.record_failed_attempt(start, failure));
    assert_eq!(
      state.next_attempt_at(),
      Some(start + Duration::from_secs(30))
    );
  }
}
