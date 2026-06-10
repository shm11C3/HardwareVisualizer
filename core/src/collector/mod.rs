//! Sensor collection — Tauri-independent.
//!
//! - [`history::HistoryStore`] owns the CPU / memory / GPU / process history
//!   ring buffers behind `Arc<Mutex<...>>` and exposes a Core read API.
//! - [`sampling`] provides the `sample_system` / `sample_gpu` cycle that
//!   feeds the history store and produces a [`crate::models::MetricsSnapshot`].
//! - [`system_monitor::SystemMonitorController`] drives the periodic
//!   sampling loop on a tokio task and publishes snapshots to a
//!   [`crate::event_bus::EventBus`].
//! - [`live_storage_health::LiveStorageHealthCollector`] serves on-demand
//!   Live Storage Health reads against a startup-cached device list
//!   (ADR 0006); nothing is persisted.

pub mod history;
pub mod live_storage_health;
pub mod sampling;
pub mod system_monitor;

pub use history::HistoryStore;
pub use live_storage_health::LiveStorageHealthCollector;
pub use system_monitor::SystemMonitorController;
