use std::sync::Arc;

use crate::commands::settings;
use crate::models;
use crate::services::external_component_guidance_service::{
  ExternalComponentGuidanceState, ExternalComponentGuidanceView,
  normalize_external_component_guidance_key,
};
use tauri::command;

#[command]
#[specta::specta]
pub async fn get_external_component_guidance_candidates(
  view: ExternalComponentGuidanceView,
  state: tauri::State<'_, settings::AppState>,
  guidance_state: tauri::State<'_, Arc<ExternalComponentGuidanceState>>,
  workers: tauri::State<'_, crate::workers::WorkersState>,
) -> Result<
  Vec<models::external_component_guidance::ExternalComponentGuidanceCandidate>,
  String,
> {
  let latest_snapshot_candidates = workers
    .window_adapter
    .lock()
    .map_err(|_| "window adapter state lock poisoned".to_string())?
    .as_ref()
    .map(|adapter| adapter.latest_external_component_guidance_candidates())
    .unwrap_or_default();
  guidance_state.record_candidates(latest_snapshot_candidates);

  let acknowledged_keys = state
    .settings
    .lock()
    .map_err(|_| "settings state lock poisoned".to_string())?
    .external_component_guidance
    .acknowledged_keys
    .clone();

  Ok(
    guidance_state
      .pending_candidates(view, &acknowledged_keys)
      .into_iter()
      .map(Into::into)
      .collect(),
  )
}

#[command]
#[specta::specta]
pub async fn defer_external_component_guidance_for_session(
  key: String,
  guidance_state: tauri::State<'_, Arc<ExternalComponentGuidanceState>>,
) -> Result<(), String> {
  let key = normalize_external_component_guidance_key(&key)?;
  guidance_state.defer_for_session(&key);
  Ok(())
}
