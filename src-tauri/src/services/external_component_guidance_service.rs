use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use hardviz_core::models::external_component_guidance::{
  PAWNIO_CPU_PACKAGE_TEMPERATURE_KEY, PAWNIO_MOTHERBOARD_SENSORS_KEY,
  SMARTCTL_STORAGE_HEALTH_KEY,
};
use hardviz_core::models::{ExternalComponent, ExternalComponentGuidanceCandidate};

/// Every guidance key a component can produce. Core forms the keys as
/// `<component>:<usage>:v1`; the mapping stays here because only the App
/// needs to address all of a component's keys at once.
pub(crate) fn guidance_keys_for(component: ExternalComponent) -> &'static [&'static str] {
  match component {
    ExternalComponent::Pawnio => &[
      PAWNIO_CPU_PACKAGE_TEMPERATURE_KEY,
      PAWNIO_MOTHERBOARD_SENSORS_KEY,
    ],
    ExternalComponent::Smartctl => &[SMARTCTL_STORAGE_HEALTH_KEY],
  }
}

pub(crate) fn normalize_external_component_guidance_key(
  key: &str,
) -> Result<String, String> {
  let key = key.trim();
  if key.is_empty() {
    return Err("external component guidance key must not be empty".to_string());
  }

  Ok(key.to_string())
}

#[derive(Default)]
pub struct ExternalComponentGuidanceState {
  pending: Mutex<BTreeMap<String, ExternalComponentGuidanceCandidate>>,
  deferred_for_session: Mutex<BTreeSet<String>>,
}

impl ExternalComponentGuidanceState {
  pub fn record_candidates(
    &self,
    candidates: impl IntoIterator<Item = ExternalComponentGuidanceCandidate>,
  ) {
    let Ok(mut pending) = self.pending.lock() else {
      return;
    };

    for candidate in candidates {
      if candidate.key.trim().is_empty() {
        continue;
      }
      pending.insert(candidate.key.clone(), candidate);
    }
  }

  pub fn pending_candidates(
    &self,
    acknowledged_keys: &[String],
  ) -> Vec<ExternalComponentGuidanceCandidate> {
    let candidates = match self.pending.lock() {
      Ok(pending) => pending.values().cloned().collect(),
      Err(_) => return Vec::new(),
    };
    self.unsuppressed_candidates(candidates, acknowledged_keys)
  }

  pub fn unsuppressed_candidates(
    &self,
    candidates: Vec<ExternalComponentGuidanceCandidate>,
    acknowledged_keys: &[String],
  ) -> Vec<ExternalComponentGuidanceCandidate> {
    let acknowledged: BTreeSet<&str> =
      acknowledged_keys.iter().map(String::as_str).collect();
    let deferred = self
      .deferred_for_session
      .lock()
      .map(|keys| keys.clone())
      .unwrap_or_default();
    candidates
      .into_iter()
      .filter(|candidate| !acknowledged.contains(candidate.key.as_str()))
      .filter(|candidate| !deferred.contains(candidate.key.as_str()))
      .collect()
  }

  pub fn defer_for_session(&self, key: &str) {
    if let Ok(mut deferred) = self.deferred_for_session.lock() {
      deferred.insert(key.to_string());
    }
  }

  /// Hide every guidance key of `component` until the process restarts.
  /// Session-only on purpose: the acknowledged ("never show again") keys in
  /// Application Preferences are untouched, so a component that goes missing
  /// again in a later session is offered again.
  pub fn defer_component_for_session(&self, component: ExternalComponent) {
    for key in guidance_keys_for(component) {
      self.defer_for_session(key);
    }
  }

  pub fn remove_key(&self, key: &str) {
    if let Ok(mut pending) = self.pending.lock() {
      pending.remove(key);
    }
    if let Ok(mut deferred) = self.deferred_for_session.lock() {
      deferred.remove(key);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn cpu() -> ExternalComponentGuidanceCandidate {
    ExternalComponentGuidanceCandidate::pawnio_cpu_package_temperature(
      "Service unavailable".into(),
    )
  }

  fn board() -> ExternalComponentGuidanceCandidate {
    ExternalComponentGuidanceCandidate::pawnio_motherboard_sensors(
      "Service unavailable".into(),
    )
  }

  fn storage() -> ExternalComponentGuidanceCandidate {
    ExternalComponentGuidanceCandidate::smartctl_storage_health(
      vec!["temperature".into()],
      Some(1),
      "smartctl not installed".into(),
    )
  }

  #[test]
  fn pending_and_streamed_candidates_share_the_same_user_suppression() {
    let state = ExternalComponentGuidanceState::default();
    let candidates = vec![cpu(), storage()];
    state.record_candidates(candidates.clone());
    state.defer_for_session(&cpu().key);
    assert_eq!(state.pending_candidates(&[]), vec![storage()]);
    assert_eq!(
      state.unsuppressed_candidates(candidates.clone(), &[]),
      vec![storage()]
    );
    assert!(
      state
        .unsuppressed_candidates(candidates, &[storage().key])
        .is_empty()
    );
    assert!(state.unsuppressed_candidates(vec![], &[]).is_empty());
  }

  #[test]
  fn component_setup_defers_both_sensor_keys_without_hiding_storage() {
    let state = ExternalComponentGuidanceState::default();
    state.defer_component_for_session(ExternalComponent::Pawnio);
    assert_eq!(
      state.unsuppressed_candidates(vec![cpu(), board(), storage()], &[]),
      vec![storage()]
    );
  }

  #[test]
  fn acknowledgement_removes_queued_guidance() {
    let state = ExternalComponentGuidanceState::default();
    state.record_candidates(vec![storage()]);
    state.remove_key(&storage().key);
    assert!(state.pending_candidates(&[]).is_empty());
  }

  #[test]
  fn guidance_keys_are_normalized_without_changing_the_core_namespace() {
    assert_eq!(
      normalize_external_component_guidance_key(" pawnio:cpu-package-temperature:v1 ")
        .unwrap(),
      cpu().key
    );
    assert!(normalize_external_component_guidance_key(" ").is_err());
  }
}
