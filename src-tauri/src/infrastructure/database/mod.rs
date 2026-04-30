//! Tauri-aware database glue.
//!
//! Schema migrations live here because they consume
//! `tauri_plugin_sql::Migration`, which is App-only. Read/write
//! workloads (archive writers, scheduled deletion, schema preflight)
//! moved to [`hardviz_core::infrastructure::database`] and
//! [`hardviz_core::persistence`] in #1407.

pub mod migration;
