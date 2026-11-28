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

/// Reports whether the consent dialog is open and unanswered, so quitting
/// the app while it is open can count as a decline.
#[tauri::command]
#[specta::specta]
pub fn set_product_analytics_prompt_open(
  state: tauri::State<'_, Arc<AnalyticsService>>,
  open: bool,
) {
  state.set_prompt_open(open);
}
