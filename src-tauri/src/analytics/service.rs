//! The managed analytics service: consent enforcement, counters, sealing,
//! and delivery.
//!
//! Two locks:
//! - the **consent lock** serializes whole consent transitions, runtime
//!   change and settings write together, so two transitions never
//!   interleave;
//! - the **state lock** guards counters, the queue, the consent snapshot,
//!   and the consent generation. Recording and flushing take only this one.
//!
//! The generation is how a decline wins against a flush that is already
//! running: the decline advances it under the state lock before any
//! persistence can yield, and a flush re-checks it before starting
//! transport and again before touching state after completion.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use tokio::sync::Notify;

use super::batch::{LocalAggregate, ReportedPeriods, SealedBatch};
use super::consent::GrantedConsent;
use super::flags::{self, FlagSources};
use super::transport::{Delivery, Transport};
use super::vocabulary::{OsFamily, ProductEvent};
use crate::models::settings::{ProductAnalyticsDecision, ProductAnalyticsSettings};

/// The Worker accepts `installation.day` within this many days of its own
/// UTC date; a batch outside it is dropped without being sent.
const WORKER_DAY_WINDOW_DAYS: i64 = 2;

pub(crate) trait Clock: Send + Sync + 'static {
  fn now(&self) -> DateTime<Utc>;
}

pub(crate) struct SystemClock;

impl Clock for SystemClock {
  fn now(&self) -> DateTime<Utc> {
    Utc::now()
  }
}

/// The settings-service operations the runtime needs. Calls block on file
/// I/O.
pub(crate) trait ConsentStore: Send + Sync + 'static {
  fn load(&self) -> ProductAnalyticsSettings;
  fn persist_decision(&self, decision: ProductAnalyticsDecision) -> Result<(), String>;
  fn persist_reported_periods(&self, periods: ReportedPeriods) -> Result<(), String>;
}

#[async_trait::async_trait]
pub(crate) trait FlagSourceReader: Send + Sync + 'static {
  async fn read(&self) -> FlagSources;
}

pub(crate) struct Ports {
  pub(crate) store: Arc<dyn ConsentStore>,
  pub(crate) flag_sources: Arc<dyn FlagSourceReader>,
  pub(crate) transport: Arc<dyn Transport>,
  pub(crate) clock: Arc<dyn Clock>,
}

#[derive(Debug, Default)]
struct State {
  aggregate: LocalAggregate,
  /// `None` closes recording and flush admission.
  consent: Option<GrantedConsent>,
  generation: u64,
  in_flight: Option<tokio::task::AbortHandle>,
}

pub(crate) struct AnalyticsService {
  state: Mutex<State>,
  consent_lock: tokio::sync::Mutex<()>,
  flush_requested: Notify,
  ports: Ports,
  #[cfg(test)]
  gates: test_support::FlushGates,
}

/// Aborts the send task when the flush awaiting it is dropped, so aborting
/// the flush loop at shutdown also aborts its in-flight request.
struct AbortOnDrop<T>(tokio::task::JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
  fn drop(&mut self) {
    self.0.abort();
  }
}

impl AnalyticsService {
  /// Starts from the persisted consent: a grant behind the current contract
  /// version is not effective until it is renewed.
  pub(crate) fn new(ports: Ports) -> Self {
    let consent = GrantedConsent::from_settings(&ports.store.load());
    Self {
      state: Mutex::new(State {
        consent,
        ..State::default()
      }),
      consent_lock: tokio::sync::Mutex::new(()),
      flush_requested: Notify::new(),
      ports,
      #[cfg(test)]
      gates: test_support::FlushGates::default(),
    }
  }

  /// Counts an event while consent is effective; otherwise does nothing.
  /// It never fails and never tells the caller whether analytics is on.
  pub(crate) fn record(&self, event: ProductEvent) {
    let mut state = self.state.lock().unwrap();
    if state.consent.is_some() {
      let now = self.ports.clock.now();
      state.aggregate.record(event, now);
    }
  }

  /// Resolves when a grant or renew asks for an immediate flush.
  pub(crate) async fn flush_requested(&self) {
    self.flush_requested.notified().await;
  }

  /// Applies a consent decision to the runtime and persists it as one
  /// transition.
  ///
  /// A decline is effective in the runtime first, then persisted, so a
  /// failed write still leaves nothing sent for the rest of the process. A
  /// grant or renew is persisted first, then loaded, so nothing is sent for
  /// a consent that is not on disk.
  pub(crate) async fn set_consent(
    &self,
    decision: ProductAnalyticsDecision,
  ) -> Result<(), String> {
    let _transition = self.consent_lock.lock().await;
    match decision {
      ProductAnalyticsDecision::Declined => {
        {
          let mut state = self.state.lock().unwrap();
          state.generation += 1;
          state.consent = None;
          state.aggregate.clear();
          if let Some(in_flight) = state.in_flight.take() {
            in_flight.abort();
          }
        }
        self.persist_decision(decision).await
      }
      ProductAnalyticsDecision::Granted => {
        self.persist_decision(decision).await?;
        {
          let mut state = self.state.lock().unwrap();
          // Loaded under the state lock so a concurrent seal's reported
          // day or month cannot be overwritten by an older read.
          let consent = GrantedConsent::from_settings(&self.ports.store.load());
          if state.consent != consent {
            state.generation += 1;
            state.consent = consent;
          }
        }
        self.flush_requested.notify_one();
        Ok(())
      }
    }
  }

  async fn persist_decision(
    &self,
    decision: ProductAnalyticsDecision,
  ) -> Result<(), String> {
    let store = Arc::clone(&self.ports.store);
    tokio::task::spawn_blocking(move || store.persist_decision(decision))
      .await
      .map_err(|error| format!("Failed to persist the analytics decision: {error}"))?
  }

  /// Seals a batch from everything counted since the previous one, then
  /// sends queued batches from the head until the queue is empty or a send
  /// is deferred. Does nothing while consent is not effective.
  pub(crate) async fn flush(&self) {
    let Some(generation) = self.seal().await else {
      return;
    };
    while let Some(batch) = self.next_batch(generation) {
      #[cfg(test)]
      self.gates.before_admission.pass().await;

      let send = {
        let mut state = self.state.lock().unwrap();
        if state.generation != generation {
          return;
        }
        let transport = Arc::clone(&self.ports.transport);
        let batch = batch.clone();
        let send = tokio::spawn(async move { transport.send(&batch).await });
        state.in_flight = Some(send.abort_handle());
        AbortOnDrop(send)
      };
      let mut send = send;
      let delivery = match (&mut send.0).await {
        Ok(delivery) => delivery,
        // Aborted by a decline.
        Err(_) => return,
      };

      #[cfg(test)]
      self.gates.after_delivery.pass().await;

      let mut state = self.state.lock().unwrap();
      if state.generation != generation {
        // A stale completion must not repopulate a cleared queue.
        return;
      }
      state.in_flight = None;
      match delivery {
        Delivery::Accepted | Delivery::Rejected => {
          state.aggregate.remove_head(batch.id())
        }
        Delivery::Deferred => return,
      }
    }
  }

  /// Seals one batch and returns the generation it belongs to.
  async fn seal(&self) -> Option<u64> {
    let (generation, wants_flags) = {
      let state = self.state.lock().unwrap();
      let consent = state.consent.as_ref()?;
      let today = self.ports.clock.now().date_naive();
      (
        state.generation,
        consent.reported_day.is_none_or(|reported| today > reported),
      )
    };
    // Reading hardware facts can take a moment, so it happens outside the
    // state lock and only while the day's flags are still owed.
    let flags = if wants_flags {
      flags::sample(&self.ports.flag_sources.read().await, OsFamily::CURRENT)
    } else {
      None
    };

    let mut state = self.state.lock().unwrap();
    if state.generation != generation {
      return None;
    }
    let State {
      aggregate, consent, ..
    } = &mut *state;
    let consent = consent.as_mut()?;
    let now = self.ports.clock.now();
    aggregate.seal(consent, now, flags, |periods| {
      self.ports.store.persist_reported_periods(periods)
    });
    Some(generation)
  }

  /// The head batch to send, after dropping heads whose day the Worker
  /// would refuse anyway.
  fn next_batch(&self, generation: u64) -> Option<SealedBatch> {
    let mut state = self.state.lock().unwrap();
    if state.generation != generation {
      return None;
    }
    let today = self.ports.clock.now().date_naive();
    while let Some(head) = state.aggregate.head() {
      if (today - head.day()).num_days().abs() <= WORKER_DAY_WINDOW_DAYS {
        return Some(head.clone());
      }
      let id = head.id();
      state.aggregate.remove_head(id);
    }
    None
  }

  #[cfg(test)]
  fn queued(&self) -> Vec<SealedBatch> {
    self.state.lock().unwrap().aggregate.queued()
  }

  #[cfg(test)]
  fn is_admitting(&self) -> bool {
    self.state.lock().unwrap().consent.is_some()
  }
}

#[cfg(test)]
pub(crate) mod test_support {
  use std::collections::VecDeque;
  use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

  use super::*;
  use crate::analytics::flags::PreferenceFacts;
  use crate::analytics::vocabulary::{CpuVendor, GpuVendor, Navigation, PerformanceView};
  use crate::models::settings::ProductAnalyticsConsent;
  use crate::services::settings_service::CURRENT_PRODUCT_ANALYTICS_CONSENT_VERSION;
  use hardviz_core::models::{OptionalSensorProviderOutcomes, ProviderAttemptOutcome};

  /// A point a flush can be held at. Unarmed gates pass immediately.
  #[derive(Default)]
  pub(crate) struct Gate {
    armed: Mutex<Option<(Arc<tokio::sync::Barrier>, Arc<tokio::sync::Barrier>)>>,
  }

  impl Gate {
    /// Arms the gate; the returned handle waits for the flush to arrive and
    /// then lets it go.
    pub(crate) fn arm(&self) -> GateHandle {
      let reached = Arc::new(tokio::sync::Barrier::new(2));
      let release = Arc::new(tokio::sync::Barrier::new(2));
      *self.armed.lock().unwrap() = Some((Arc::clone(&reached), Arc::clone(&release)));
      GateHandle { reached, release }
    }

    pub(crate) async fn pass(&self) {
      let armed = self.armed.lock().unwrap().take();
      if let Some((reached, release)) = armed {
        reached.wait().await;
        release.wait().await;
      }
    }
  }

  pub(crate) struct GateHandle {
    reached: Arc<tokio::sync::Barrier>,
    release: Arc<tokio::sync::Barrier>,
  }

  impl GateHandle {
    pub(crate) async fn reached(&self) {
      self.reached.wait().await;
    }

    pub(crate) async fn release(&self) {
      self.release.wait().await;
    }
  }

  #[derive(Default)]
  pub(crate) struct FlushGates {
    pub(crate) before_admission: Gate,
    pub(crate) after_delivery: Gate,
  }

  /// Records every batch it is asked to send and answers from a script
  /// (default `Accepted`). An armed gate holds the request in flight.
  #[derive(Default)]
  pub(crate) struct RecordingTransport {
    pub(crate) sent: Mutex<Vec<SealedBatch>>,
    pub(crate) completed: AtomicUsize,
    pub(crate) answers: Mutex<VecDeque<Delivery>>,
    pub(crate) in_flight: Gate,
  }

  impl RecordingTransport {
    pub(crate) fn calls(&self) -> usize {
      self.sent.lock().unwrap().len()
    }

    pub(crate) fn answer(&self, delivery: Delivery) {
      self.answers.lock().unwrap().push_back(delivery);
    }
  }

  #[async_trait::async_trait]
  impl Transport for RecordingTransport {
    async fn send(&self, batch: &SealedBatch) -> Delivery {
      self.sent.lock().unwrap().push(batch.clone());
      self.in_flight.pass().await;
      self.completed.fetch_add(1, Ordering::SeqCst);
      self
        .answers
        .lock()
        .unwrap()
        .pop_front()
        .unwrap_or(Delivery::Accepted)
    }
  }

  /// A settings double with the persisted section, injectable write
  /// failures, and a decision write that can be held.
  #[derive(Default)]
  pub(crate) struct MemoryStore {
    pub(crate) section: Mutex<ProductAnalyticsSettings>,
    pub(crate) fail_decisions: AtomicBool,
    pub(crate) fail_periods: AtomicBool,
    pub(crate) held_decision:
      Mutex<Option<(std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>)>>,
    pub(crate) next_seed: AtomicUsize,
  }

  impl MemoryStore {
    pub(crate) fn with(section: ProductAnalyticsSettings) -> Self {
      Self {
        section: Mutex::new(section),
        ..Self::default()
      }
    }

    pub(crate) fn section(&self) -> ProductAnalyticsSettings {
      self.section.lock().unwrap().clone()
    }

    /// Holds the next decision write until the returned sender is used;
    /// the returned receiver reports that the write has started.
    pub(crate) fn hold_next_decision(
      &self,
    ) -> (std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>) {
      let (started_tx, started_rx) = std::sync::mpsc::channel();
      let (release_tx, release_rx) = std::sync::mpsc::channel();
      *self.held_decision.lock().unwrap() = Some((started_tx, release_rx));
      (release_tx, started_rx)
    }
  }

  impl ConsentStore for MemoryStore {
    fn load(&self) -> ProductAnalyticsSettings {
      self.section()
    }

    fn persist_decision(&self, decision: ProductAnalyticsDecision) -> Result<(), String> {
      if let Some((started, release)) = self.held_decision.lock().unwrap().take() {
        let _ = started.send(());
        let _ = release.recv();
      }
      if self.fail_decisions.load(Ordering::SeqCst) {
        return Err("injected settings write failure".to_string());
      }
      let mut section = self.section.lock().unwrap();
      *section = match decision {
        ProductAnalyticsDecision::Declined => ProductAnalyticsSettings {
          consent: ProductAnalyticsConsent::Declined,
          consent_version: CURRENT_PRODUCT_ANALYTICS_CONSENT_VERSION,
          ..ProductAnalyticsSettings::default()
        },
        ProductAnalyticsDecision::Granted
          if section.consent == ProductAnalyticsConsent::Granted =>
        {
          ProductAnalyticsSettings {
            consent_version: CURRENT_PRODUCT_ANALYTICS_CONSENT_VERSION,
            ..section.clone()
          }
        }
        ProductAnalyticsDecision::Granted => {
          let seed = self.next_seed.fetch_add(1, Ordering::SeqCst);
          ProductAnalyticsSettings {
            seed: Some(format!("{seed:064x}")),
            ..crate::analytics::consent::test_support::granted_section()
          }
        }
      };
      Ok(())
    }

    fn persist_reported_periods(&self, periods: ReportedPeriods) -> Result<(), String> {
      if self.fail_periods.load(Ordering::SeqCst) {
        return Err("injected settings write failure".to_string());
      }
      let mut section = self.section.lock().unwrap();
      if section.consent != ProductAnalyticsConsent::Granted {
        return Err("not granted".to_string());
      }
      if let Some(day) = periods.day {
        section.reported_day = Some(day.format("%Y-%m-%d").to_string());
      }
      if let Some(month) = periods.month {
        section.reported_month = Some(month.to_string());
      }
      Ok(())
    }
  }

  pub(crate) struct FixedFlagSources(pub(crate) Mutex<FlagSources>);

  impl Default for FixedFlagSources {
    fn default() -> Self {
      Self(Mutex::new(FlagSources {
        preferences: PreferenceFacts {
          features_on: Vec::new(),
          navigation: Navigation::Grouped,
          performance_view: PerformanceView::Panels,
        },
        cpu_vendor: Some(CpuVendor::Intel),
        gpu_vendors: Some(vec![GpuVendor::Nvidia]),
        providers: Some(OptionalSensorProviderOutcomes {
          cpu_package_temperature: Some(ProviderAttemptOutcome::Answered),
          motherboard_sensors: Some(ProviderAttemptOutcome::NotAnswered),
        }),
      }))
    }
  }

  #[async_trait::async_trait]
  impl FlagSourceReader for FixedFlagSources {
    async fn read(&self) -> FlagSources {
      self.0.lock().unwrap().clone()
    }
  }

  pub(crate) struct ManualClock(pub(crate) Mutex<DateTime<Utc>>);

  impl ManualClock {
    pub(crate) fn at(value: &str) -> Self {
      Self(Mutex::new(
        DateTime::parse_from_rfc3339(value)
          .unwrap()
          .with_timezone(&Utc),
      ))
    }

    pub(crate) fn set(&self, value: &str) {
      *self.0.lock().unwrap() = DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc);
    }
  }

  impl Clock for ManualClock {
    fn now(&self) -> DateTime<Utc> {
      *self.0.lock().unwrap()
    }
  }

  pub(crate) struct Harness {
    pub(crate) service: Arc<AnalyticsService>,
    pub(crate) store: Arc<MemoryStore>,
    pub(crate) transport: Arc<RecordingTransport>,
    pub(crate) clock: Arc<ManualClock>,
  }

  impl Harness {
    pub(crate) fn new(section: ProductAnalyticsSettings) -> Self {
      let store = Arc::new(MemoryStore::with(section));
      let transport = Arc::new(RecordingTransport::default());
      let clock = Arc::new(ManualClock::at("2026-11-03T12:00:00Z"));
      let service = Arc::new(AnalyticsService::new(Ports {
        store: store.clone(),
        flag_sources: Arc::new(FixedFlagSources::default()),
        transport: transport.clone(),
        clock: clock.clone(),
      }));
      Self {
        service,
        store,
        transport,
        clock,
      }
    }

    pub(crate) fn granted() -> Self {
      Self::new(crate::analytics::consent::test_support::granted_section())
    }

    pub(crate) fn gates(&self) -> &FlushGates {
      &self.service.gates
    }
  }
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::Ordering;
  use std::time::Duration;

  use super::test_support::Harness;
  use super::*;
  use crate::analytics::vocabulary::{Capability, Screen};
  use crate::models::settings::ProductAnalyticsConsent;
  use crate::services::settings_service::CURRENT_PRODUCT_ANALYTICS_CONSENT_VERSION;

  const PERFORMANCE: ProductEvent = ProductEvent::ScreenView(Screen::Performance);
  const EXPORTED: ProductEvent =
    ProductEvent::Capability(Capability::HardwareReportExported);

  fn declined() -> ProductAnalyticsSettings {
    ProductAnalyticsSettings {
      consent: ProductAnalyticsConsent::Declined,
      consent_version: CURRENT_PRODUCT_ANALYTICS_CONSENT_VERSION,
      ..ProductAnalyticsSettings::default()
    }
  }

  fn counters(batch: &SealedBatch) -> Vec<(&'static str, u32)> {
    batch
      .counters()
      .into_iter()
      .map(|(_, name, count)| (name, count))
      .collect()
  }

  // No request before opt-in, none after opt-out.

  #[tokio::test]
  async fn undecided_and_declined_installations_send_nothing() {
    for section in [ProductAnalyticsSettings::default(), declined()] {
      let harness = Harness::new(section);
      harness.service.record(PERFORMANCE);
      harness.service.flush().await;
      assert_eq!(harness.transport.calls(), 0);
      assert!(harness.service.queued().is_empty());
    }
  }

  #[tokio::test]
  async fn grant_then_decline_leaves_nothing_to_send() {
    let harness = Harness::new(ProductAnalyticsSettings::default());
    harness
      .service
      .set_consent(ProductAnalyticsDecision::Granted)
      .await
      .unwrap();
    harness.service.record(PERFORMANCE);
    harness
      .service
      .set_consent(ProductAnalyticsDecision::Declined)
      .await
      .unwrap();

    assert!(harness.service.queued().is_empty());
    harness.service.record(PERFORMANCE);
    harness.service.flush().await;
    assert_eq!(harness.transport.calls(), 0);
    assert_eq!(harness.store.section(), declined());
  }

  #[tokio::test]
  async fn a_decline_aborts_the_in_flight_request() {
    let harness = Harness::granted();
    let in_flight = harness.transport.in_flight.arm();
    let flush = tokio::spawn({
      let service = Arc::clone(&harness.service);
      async move { service.flush().await }
    });
    in_flight.reached().await;

    harness
      .service
      .set_consent(ProductAnalyticsDecision::Declined)
      .await
      .unwrap();
    flush.await.unwrap();

    assert_eq!(harness.transport.calls(), 1);
    assert_eq!(
      harness.transport.completed.load(Ordering::SeqCst),
      0,
      "the aborted request never completes"
    );
    assert!(harness.service.queued().is_empty());
  }

  #[tokio::test]
  async fn a_flush_admitted_after_a_decline_makes_no_call() {
    let harness = Harness::granted();
    harness.service.record(PERFORMANCE);
    let admission = harness.gates().before_admission.arm();
    let flush = tokio::spawn({
      let service = Arc::clone(&harness.service);
      async move { service.flush().await }
    });
    // The flush has read `granted` and sealed its batch.
    admission.reached().await;
    assert_eq!(harness.service.queued().len(), 1);

    harness
      .service
      .set_consent(ProductAnalyticsDecision::Declined)
      .await
      .unwrap();
    admission.release().await;
    flush.await.unwrap();

    assert_eq!(harness.transport.calls(), 0);
  }

  #[tokio::test]
  async fn a_completion_after_a_decline_leaves_state_untouched() {
    let harness = Harness::granted();
    let delivered = harness.gates().after_delivery.arm();
    let flush = tokio::spawn({
      let service = Arc::clone(&harness.service);
      async move { service.flush().await }
    });
    delivered.reached().await;

    harness
      .service
      .set_consent(ProductAnalyticsDecision::Declined)
      .await
      .unwrap();
    assert_eq!(harness.store.section(), declined());
    // A new grant queues a batch of its own before the stale completion.
    harness
      .service
      .set_consent(ProductAnalyticsDecision::Granted)
      .await
      .unwrap();
    harness.transport.answer(Delivery::Deferred);
    harness.service.flush().await;
    let regranted = harness.store.section();
    let queued = harness.service.queued();
    assert_eq!(queued.len(), 1);

    delivered.release().await;
    flush.await.unwrap();

    assert_eq!(harness.transport.calls(), 2);
    assert_eq!(harness.service.queued(), queued);
    assert_eq!(harness.store.section(), regranted);
  }

  #[tokio::test]
  async fn a_flush_during_a_pending_decline_write_makes_no_call() {
    let harness = Harness::granted();
    harness.service.record(PERFORMANCE);
    let (release_write, write_started) = harness.store.hold_next_decision();
    let decline = tokio::spawn({
      let service = Arc::clone(&harness.service);
      async move {
        service
          .set_consent(ProductAnalyticsDecision::Declined)
          .await
      }
    });
    tokio::task::spawn_blocking(move || write_started.recv().unwrap())
      .await
      .unwrap();

    harness.service.flush().await;
    assert_eq!(harness.transport.calls(), 0);

    release_write.send(()).unwrap();
    decline.await.unwrap().unwrap();
    assert_eq!(harness.store.section(), declined());
  }

  #[tokio::test]
  async fn a_failed_decline_write_keeps_admission_closed_and_the_grant_persisted() {
    let harness = Harness::granted();
    harness.store.fail_decisions.store(true, Ordering::SeqCst);

    let result = harness
      .service
      .set_consent(ProductAnalyticsDecision::Declined)
      .await;

    assert!(result.is_err());
    assert_eq!(
      harness.store.section().consent,
      ProductAnalyticsConsent::Granted
    );
    harness.service.record(PERFORMANCE);
    harness.service.flush().await;
    assert_eq!(harness.transport.calls(), 0);
    assert!(!harness.service.is_admitting());
  }

  #[tokio::test]
  async fn a_failed_grant_write_sends_nothing() {
    let harness = Harness::new(ProductAnalyticsSettings::default());
    harness.store.fail_decisions.store(true, Ordering::SeqCst);

    let result = harness
      .service
      .set_consent(ProductAnalyticsDecision::Granted)
      .await;

    assert!(result.is_err());
    harness.service.flush().await;
    assert_eq!(harness.transport.calls(), 0);
  }

  #[tokio::test]
  async fn a_grant_started_during_a_decline_runs_after_it() {
    let harness = Harness::granted();
    let (release_write, write_started) = harness.store.hold_next_decision();
    let decline = tokio::spawn({
      let service = Arc::clone(&harness.service);
      async move {
        service
          .set_consent(ProductAnalyticsDecision::Declined)
          .await
      }
    });
    tokio::task::spawn_blocking(move || write_started.recv().unwrap())
      .await
      .unwrap();
    let grant = tokio::spawn({
      let service = Arc::clone(&harness.service);
      async move { service.set_consent(ProductAnalyticsDecision::Granted).await }
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!grant.is_finished(), "the grant waits for the decline");

    release_write.send(()).unwrap();
    decline.await.unwrap().unwrap();
    grant.await.unwrap().unwrap();

    assert_eq!(
      harness.store.section().consent,
      ProductAnalyticsConsent::Granted
    );
    assert!(harness.service.is_admitting());
  }

  #[tokio::test]
  async fn overlapping_transitions_end_with_persisted_and_runtime_agreeing() {
    let outdated = || ProductAnalyticsSettings {
      consent_version: CURRENT_PRODUCT_ANALYTICS_CONSENT_VERSION - 1,
      ..crate::analytics::consent::test_support::granted_section()
    };
    let cases = [
      // decline/grant in both orders
      (
        crate::analytics::consent::test_support::granted_section(),
        [
          ProductAnalyticsDecision::Declined,
          ProductAnalyticsDecision::Granted,
        ],
      ),
      (
        crate::analytics::consent::test_support::granted_section(),
        [
          ProductAnalyticsDecision::Granted,
          ProductAnalyticsDecision::Declined,
        ],
      ),
      // decline/renew in both orders
      (
        outdated(),
        [
          ProductAnalyticsDecision::Declined,
          ProductAnalyticsDecision::Granted,
        ],
      ),
      (
        outdated(),
        [
          ProductAnalyticsDecision::Granted,
          ProductAnalyticsDecision::Declined,
        ],
      ),
    ];
    for (section, [first, second]) in cases {
      let harness = Harness::new(section);
      let (release_write, write_started) = harness.store.hold_next_decision();
      let first_task = tokio::spawn({
        let service = Arc::clone(&harness.service);
        async move { service.set_consent(first).await }
      });
      tokio::task::spawn_blocking(move || write_started.recv().unwrap())
        .await
        .unwrap();
      let second_task = tokio::spawn({
        let service = Arc::clone(&harness.service);
        async move { service.set_consent(second).await }
      });
      release_write.send(()).unwrap();
      first_task.await.unwrap().unwrap();
      second_task.await.unwrap().unwrap();

      let persisted = harness.store.section();
      assert_eq!(
        harness.service.is_admitting(),
        GrantedConsent::from_settings(&persisted).is_some(),
        "{first:?} then {second:?}"
      );
      assert_eq!(
        persisted.consent == ProductAnalyticsConsent::Granted,
        second == ProductAnalyticsDecision::Granted,
        "the last transition wins"
      );
    }
  }

  // Sealed-batch retry policy and idempotent retry.

  #[tokio::test]
  async fn a_deferred_batch_is_retried_unchanged_while_new_events_go_to_a_new_batch() {
    let harness = Harness::granted();
    harness.service.record(PERFORMANCE);
    harness.transport.answer(Delivery::Deferred);
    harness.service.flush().await;
    let first = harness.transport.sent.lock().unwrap()[0].clone();
    assert_eq!(harness.service.queued(), [first.clone()]);

    harness.service.record(EXPORTED);
    harness.service.flush().await;

    let sent = harness.transport.sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 3);
    assert_eq!(sent[1], first, "same id and counters");
    assert_ne!(sent[2].id(), first.id());
    assert_eq!(
      counters(&sent[2]),
      [("capability.hardware_report_exported", 1)]
    );
    assert!(harness.service.queued().is_empty());
  }

  #[tokio::test]
  async fn a_rejected_batch_is_dropped_and_the_next_one_is_sent() {
    let harness = Harness::granted();
    harness.transport.answer(Delivery::Deferred);
    harness.service.flush().await;
    let first = harness.service.queued()[0].clone();

    harness.transport.answer(Delivery::Rejected);
    harness.service.flush().await;

    let sent = harness.transport.sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 3);
    assert_eq!(sent[1], first);
    assert_ne!(sent[2].id(), first.id());
    assert!(harness.service.queued().is_empty());
  }

  #[tokio::test]
  async fn a_batch_outside_the_worker_day_window_is_dropped_unsent() {
    let harness = Harness::granted();
    harness.transport.answer(Delivery::Deferred);
    harness.service.flush().await;
    let stale = harness.service.queued()[0].clone();

    // Two days later is still inside the window.
    harness.clock.set("2026-11-05T12:00:00Z");
    harness.transport.answer(Delivery::Deferred);
    harness.service.flush().await;
    assert_eq!(harness.transport.sent.lock().unwrap()[1], stale);

    harness.clock.set("2026-11-06T00:00:00Z");
    harness.service.flush().await;
    let sent = harness.transport.sent.lock().unwrap().clone();
    assert!(sent[2..].iter().all(|batch| batch.id() != stale.id()));
    assert!(harness.service.queued().is_empty());
  }

  #[tokio::test]
  async fn upload_failures_never_reach_the_caller() {
    let harness = Harness::granted();
    for _ in 0..3 {
      harness.transport.answer(Delivery::Deferred);
    }
    harness.service.flush().await;
    harness.service.record(PERFORMANCE);
    harness.service.flush().await;
    assert!(harness.service.is_admitting());
    assert_eq!(
      harness
        .service
        .set_consent(ProductAnalyticsDecision::Granted)
        .await,
      Ok(())
    );
  }

  // Consent versions and flags.

  #[tokio::test]
  async fn a_grant_behind_the_current_version_records_nothing_until_renewed() {
    let outdated = ProductAnalyticsSettings {
      consent_version: CURRENT_PRODUCT_ANALYTICS_CONSENT_VERSION - 1,
      ..crate::analytics::consent::test_support::granted_section()
    };
    let harness = Harness::new(outdated.clone());
    harness.service.record(PERFORMANCE);
    harness.service.flush().await;
    assert_eq!(harness.transport.calls(), 0);

    harness
      .service
      .set_consent(ProductAnalyticsDecision::Granted)
      .await
      .unwrap();
    let renewed = harness.store.section();
    assert_eq!(renewed.seed, outdated.seed, "a renewal keeps the seed");
    assert_eq!(renewed.cohort_month, outdated.cohort_month);
    harness.service.record(PERFORMANCE);
    harness.service.flush().await;
    assert_eq!(harness.transport.calls(), 1);
  }

  #[tokio::test]
  async fn a_grant_requests_an_immediate_flush() {
    let harness = Harness::new(ProductAnalyticsSettings::default());
    harness
      .service
      .set_consent(ProductAnalyticsDecision::Granted)
      .await
      .unwrap();
    tokio::time::timeout(Duration::from_secs(1), harness.service.flush_requested())
      .await
      .expect("a grant asks for a flush");
  }

  #[tokio::test]
  async fn the_first_settled_batch_of_the_day_carries_flags_and_records_the_periods() {
    let harness = Harness::granted();
    harness.service.flush().await;
    harness.service.flush().await;

    let sent = harness.transport.sent.lock().unwrap().clone();
    assert!(sent[0].flags().is_some());
    assert!(sent[0].has_month_token());
    assert!(sent[1].flags().is_none());
    assert!(!sent[1].has_month_token());
    let section = harness.store.section();
    assert_eq!(section.reported_day.as_deref(), Some("2026-11-03"));
    assert_eq!(section.reported_month.as_deref(), Some("2026-11"));
  }

  #[tokio::test]
  async fn a_failed_reported_period_write_sends_the_batch_without_flags() {
    let harness = Harness::granted();
    harness.store.fail_periods.store(true, Ordering::SeqCst);
    harness.service.flush().await;

    let sent = harness.transport.sent.lock().unwrap().clone();
    assert!(sent[0].flags().is_none());
    assert!(!sent[0].has_month_token());
    assert_eq!(harness.store.section().reported_day, None);
  }

  #[tokio::test]
  async fn a_restart_on_the_reported_day_seals_no_flags() {
    let mut section = crate::analytics::consent::test_support::granted_section();
    section.reported_day = Some("2026-11-03".to_string());
    section.reported_month = Some("2026-11".to_string());
    let harness = Harness::new(section);
    harness.service.flush().await;

    let sent = harness.transport.sent.lock().unwrap().clone();
    assert!(sent[0].flags().is_none());
    assert!(!sent[0].has_month_token());
  }
}
