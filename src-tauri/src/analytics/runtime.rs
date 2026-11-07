//! Production wiring: the settings and hardware ports the service reads,
//! and the flush loop worker.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use hardviz_core::collector::HistoryStore;
use hardviz_core::infrastructure::providers::sysinfo_provider;
use hardviz_core::models::{MetricsSnapshot, OptionalSensorProviderOutcomes};
use hardviz_core::platform::factory::PlatformFactory;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use tokio::sync::broadcast;

use super::batch::ReportedPeriods;
use super::flags::{self, FlagSources, PreferenceFacts};
use super::service::{AnalyticsService, ConsentStore, FlagSourceReader};
use super::vocabulary::PerformanceView;
use crate::commands::settings::AppState;
use crate::models::settings::{ProductAnalyticsDecision, ProductAnalyticsSettings};
use crate::services::ui_service;

/// The first flush waits so the upload never competes with startup
/// collection and the first window paint.
const FIRST_FLUSH_DELAY: Duration = Duration::from_secs(60);
const FLUSH_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
/// The key the frontend stores its Performance view under in the UI store
/// (`usePerformanceLayout`).
const PERFORMANCE_VIEW_KEY: &str = "performanceLayoutPreset";

/// Reads and writes the `productAnalytics` section through the App
/// settings service.
pub(crate) struct AppConsentStore {
  pub(crate) app: AppHandle,
}

impl ConsentStore for AppConsentStore {
  fn load(&self) -> ProductAnalyticsSettings {
    let state = self.app.state::<AppState>();
    let settings = state.settings.lock().unwrap();
    settings.product_analytics.clone()
  }

  fn persist_decision(&self, decision: ProductAnalyticsDecision) -> Result<(), String> {
    let state = self.app.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    settings.set_product_analytics_consent(decision)
  }

  fn persist_reported_periods(&self, periods: ReportedPeriods) -> Result<(), String> {
    let state = self.app.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    settings.set_product_analytics_reported_periods(
      periods.day,
      periods.month.map(|month| month.first_day()),
    )
  }
}

/// Reads the facts State Flags are classified from.
pub(crate) struct AppFlagSources {
  pub(crate) app: AppHandle,
  /// Provider outcomes from the latest `MetricsSnapshot`, kept by the flush
  /// loop; `None` before the first snapshot.
  pub(crate) latest_providers: Arc<Mutex<Option<OptionalSensorProviderOutcomes>>>,
}

impl AppFlagSources {
  /// `None` while the UI store cannot be read, which leaves the day's flag
  /// set unsettled like a pending provider.
  fn preferences(&self) -> Option<PreferenceFacts> {
    let performance_view = self.performance_view()?;
    let state = self.app.state::<AppState>();
    let settings = state.settings.lock().unwrap().clone();
    let core_settings = state.core_settings.lock().unwrap().clone();
    Some(PreferenceFacts::from_settings(
      &settings,
      &core_settings,
      performance_view,
    ))
  }

  /// The Performance view is UI-local state in the frontend's Tauri Store,
  /// not an Application Preference in `settings.json`. `StoreExt::store`
  /// returns the instance the frontend already loaded, or loads it with the
  /// same defaults the frontend uses. An absent key is the default view the
  /// app really shows; a store that cannot be opened is no answer at all.
  fn performance_view(&self) -> Option<PerformanceView> {
    match self.app.store(ui_service::STORE_FILENAME) {
      Ok(store) => Some(PerformanceView::from_stored(
        store.get(PERFORMANCE_VIEW_KEY).as_ref(),
      )),
      Err(error) => {
        crate::log_warn!(
          "Failed to read the UI store for product analytics",
          "analytics::AppFlagSources::performance_view",
          Some(error.to_string())
        );
        None
      }
    }
  }

  fn cpu_vendor(&self) -> Option<super::vocabulary::CpuVendor> {
    let store = self.app.state::<Arc<HistoryStore>>();
    let system = store.system().lock().ok()?;
    let cpu = sysinfo_provider::get_cpu_info(system).ok()?;
    Some(flags::cpu_vendor_class(&cpu.vendor))
  }

  async fn gpu_vendors(&self) -> Option<Vec<super::vocabulary::GpuVendor>> {
    let platform = PlatformFactory::shared().ok()?;
    let gpus = platform.get_gpu_info().await.ok()?;
    Some(
      gpus
        .iter()
        .map(|gpu| flags::gpu_vendor_class(&gpu.vendor_name))
        .collect(),
    )
  }
}

#[async_trait::async_trait]
impl FlagSourceReader for AppFlagSources {
  async fn read(&self) -> FlagSources {
    FlagSources {
      preferences: self.preferences(),
      cpu_vendor: self.cpu_vendor(),
      gpu_vendors: self.gpu_vendors().await,
      providers: *self.latest_providers.lock().unwrap(),
    }
  }
}

/// Runs the flush loop: the first flush after [`FIRST_FLUSH_DELAY`], then
/// every [`FLUSH_INTERVAL`], plus one whenever a grant or renew asks for
/// it. There is no shutdown flush: `WorkersState::terminate_all` aborts this
/// task, which aborts an in-flight request with it.
pub(crate) fn spawn_flush_loop(
  runtime: &tokio::runtime::Handle,
  service: Arc<AnalyticsService>,
  snapshots: Option<broadcast::Receiver<MetricsSnapshot>>,
  latest_providers: Arc<Mutex<Option<OptionalSensorProviderOutcomes>>>,
) -> tokio::task::JoinHandle<()> {
  runtime.spawn(flush_loop(
    service,
    snapshots,
    latest_providers,
    FIRST_FLUSH_DELAY,
    FLUSH_INTERVAL,
  ))
}

async fn flush_loop(
  service: Arc<AnalyticsService>,
  mut snapshots: Option<broadcast::Receiver<MetricsSnapshot>>,
  latest_providers: Arc<Mutex<Option<OptionalSensorProviderOutcomes>>>,
  first_delay: Duration,
  interval: Duration,
) {
  let next_flush = tokio::time::sleep(first_delay);
  tokio::pin!(next_flush);
  loop {
    tokio::select! {
      () = &mut next_flush => {
        service.flush().await;
        next_flush.as_mut().reset(tokio::time::Instant::now() + interval);
      }
      () = service.flush_requested() => service.flush().await,
      received = next_snapshot(&mut snapshots) => {
        match received {
          Ok(snapshot) => {
            *latest_providers.lock().unwrap() = Some(snapshot.optional_sensor_providers);
          }
          Err(broadcast::error::RecvError::Lagged(_)) => {}
          Err(broadcast::error::RecvError::Closed) => snapshots = None,
        }
      }
    }
  }
}

async fn next_snapshot(
  snapshots: &mut Option<broadcast::Receiver<MetricsSnapshot>>,
) -> Result<MetricsSnapshot, broadcast::error::RecvError> {
  match snapshots {
    Some(receiver) => receiver.recv().await,
    None => std::future::pending().await,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::analytics::service::test_support::Harness;

  #[tokio::test(start_paused = true)]
  async fn the_loop_flushes_after_the_first_delay_and_every_interval() {
    let harness = Harness::granted();
    let (sender, receiver) = broadcast::channel(4);
    let latest = Arc::new(Mutex::new(None));
    let worker = tokio::spawn(flush_loop(
      Arc::clone(&harness.service),
      Some(receiver),
      Arc::clone(&latest),
      FIRST_FLUSH_DELAY,
      FLUSH_INTERVAL,
    ));

    tokio::time::sleep(FIRST_FLUSH_DELAY - Duration::from_secs(1)).await;
    assert_eq!(harness.transport.calls(), 0);
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(harness.transport.calls(), 1);
    tokio::time::sleep(FLUSH_INTERVAL).await;
    assert_eq!(harness.transport.calls(), 2);

    drop(sender);
    worker.abort();
  }

  #[tokio::test]
  async fn a_grant_flushes_immediately() {
    let harness = Harness::new(ProductAnalyticsSettings::default());
    let worker = tokio::spawn(flush_loop(
      Arc::clone(&harness.service),
      None,
      Arc::new(Mutex::new(None)),
      FLUSH_INTERVAL,
      FLUSH_INTERVAL,
    ));

    harness
      .service
      .set_consent(ProductAnalyticsDecision::Granted)
      .await
      .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
      while harness.transport.calls() == 0 {
        tokio::time::sleep(Duration::from_millis(5)).await;
      }
    })
    .await
    .expect("the grant's flush runs without waiting for the timer");
    worker.abort();
  }

  fn snapshot() -> MetricsSnapshot {
    MetricsSnapshot {
      cpu_usage: 0.0,
      memory_usage: 0.0,
      processors_usage: vec![],
      gpus: vec![],
      processes: vec![],
      cpu_temperature: None,
      cpu_package_thermal_status: None,
      sensor_temperatures: vec![],
      motherboard_temperatures: vec![],
      motherboard_fan_speeds: vec![],
      power_draw: Default::default(),
      cpu_power_support: Default::default(),
      motherboard_fan_support: Default::default(),
      external_component_guidance_candidates: vec![],
      optional_sensor_providers: Default::default(),
    }
  }

  #[tokio::test(start_paused = true)]
  async fn the_loop_keeps_the_latest_provider_outcomes() {
    let harness = Harness::granted();
    let (sender, receiver) = broadcast::channel(4);
    let latest = Arc::new(Mutex::new(None));
    let worker = tokio::spawn(flush_loop(
      Arc::clone(&harness.service),
      Some(receiver),
      Arc::clone(&latest),
      FIRST_FLUSH_DELAY,
      FLUSH_INTERVAL,
    ));
    let outcomes = OptionalSensorProviderOutcomes {
      cpu_package_temperature: Some(
        hardviz_core::models::ProviderAttemptOutcome::Answered,
      ),
      motherboard_sensors: Some(hardviz_core::models::ProviderAttemptOutcome::Pending),
    };
    sender
      .send(MetricsSnapshot {
        optional_sensor_providers: outcomes,
        ..snapshot()
      })
      .unwrap();
    tokio::time::sleep(Duration::from_millis(1)).await;
    assert_eq!(*latest.lock().unwrap(), Some(outcomes));

    // A closed bus does not spin the loop.
    drop(sender);
    tokio::time::sleep(FIRST_FLUSH_DELAY).await;
    assert_eq!(harness.transport.calls(), 1);
    worker.abort();
  }

  #[tokio::test]
  async fn aborting_the_loop_aborts_an_in_flight_request() {
    let harness = Harness::granted();
    let in_flight = harness.transport.in_flight.arm();
    let (_sender, receiver) = broadcast::channel(4);
    let worker = tokio::spawn(flush_loop(
      Arc::clone(&harness.service),
      Some(receiver),
      Arc::new(Mutex::new(None)),
      Duration::ZERO,
      FLUSH_INTERVAL,
    ));
    in_flight.reached().await;

    worker.abort();
    tokio::time::timeout(Duration::from_secs(1), async {
      let _ = worker.await;
    })
    .await
    .expect("the worker stops within the shutdown deadline");
    tokio::task::yield_now().await;

    assert_eq!(
      harness
        .transport
        .completed
        .load(std::sync::atomic::Ordering::SeqCst),
      0
    );
  }
}
