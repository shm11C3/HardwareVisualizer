//! Tauri-independent data models shared across the core crate.

pub mod hardware;
mod metrics;

pub use metrics::{GpuMetric, MetricsSnapshot, ProcessSample, SensorTemperature};
