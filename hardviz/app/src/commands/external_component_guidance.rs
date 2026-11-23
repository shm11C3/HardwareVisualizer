use std::sync::Arc;

use crate::commands::settings;
use crate::models;
use crate::services::external_component_guidance_service::{
  ExternalComponentGuidanceState, normalize_external_component_guidance_key,
};
use tauri::command;

#[command]
#[specta::specta]
pub async fn get_external_component_guidance_candidates(
  state: tauri::State<'_, settings::AppState>,
  guidance_state: tauri::State<'_, Arc<ExternalComponentGuidanceState>>,
) -> Result<
  Vec<models::external_component_guidance::ExternalComponentGuidanceCandidate>,
  String,
> {
  let acknowledged_keys = state
    .settings
    .lock()
    .map_err(|_| "settings state lock poisoned".to_string())?
    .external_component_guidance
    .acknowledged_keys
    .clone();

  Ok(
    guidance_state
      .pending_candidates(&acknowledged_keys)
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
