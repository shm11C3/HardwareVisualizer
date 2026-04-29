//! Sensor collection — Tauri-independent.
//!
//! - [`history::HistoryStore`] owns the CPU / memory / GPU / process history
//!   ring buffers behind `Arc<Mutex<...>>` and exposes a Core read API.
//! - [`sampling`] provides the `sample_system` / `sample_gpu` cycle that
//!   feeds the history store and produces a [`crate::models::MetricsSnapshot`].
//! - [`system_monitor::SystemMonitorController`] drives the periodic
//!   sampling loop on a tokio task and publishes snapshots to a
//!   [`crate::event_bus::EventBus`].

pub mod history;
pub mod sampling;
pub mod system_monitor;

pub use history::HistoryStore;
pub use system_monitor::SystemMonitorController;
