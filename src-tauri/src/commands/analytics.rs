//! Product Analytics commands (ADR 0028). Enforcement lives in
//! `crate::analytics`; these commands only delegate.

use std::sync::Arc;

use crate::analytics::{AnalyticsService, ProductEvent};

/// Counts one Product Event if Analytics Consent is effective. Otherwise it
/// does nothing; it never fails and never reports whether analytics is on.
#[tauri::command]
#[specta::specta]
pub fn record_product_event(
  state: tauri::State<'_, Arc<AnalyticsService>>,
  event: ProductEvent,
) {
  state.record(event);
}
