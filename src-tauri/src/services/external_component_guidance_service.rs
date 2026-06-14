use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use hardviz_core::models::{
  ExternalComponent, ExternalComponentGuidanceCandidate, ExternalComponentUsage,
};
use serde::Deserialize;
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ExternalComponentGuidanceView {
  Dashboard,
  CpuDetail,
  StorageHealth,
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
    view: ExternalComponentGuidanceView,
    acknowledged_keys: &[String],
  ) -> Vec<ExternalComponentGuidanceCandidate> {
    let acknowledged: BTreeSet<&str> =
      acknowledged_keys.iter().map(String::as_str).collect();
    let deferred = self
      .deferred_for_session
      .lock()
      .map(|keys| keys.clone())
      .unwrap_or_default();
    let Ok(pending) = self.pending.lock() else {
      return Vec::new();
    };

    pending
      .values()
      .filter(|candidate| !acknowledged.contains(candidate.key.as_str()))
      .filter(|candidate| !deferred.contains(candidate.key.as_str()))
      .filter(|candidate| is_relevant_to_view(candidate, view))
      .cloned()
      .collect()
  }

  pub fn defer_for_session(&self, key: &str) {
    if let Ok(mut deferred) = self.deferred_for_session.lock() {
      deferred.insert(key.to_string());
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

fn is_relevant_to_view(
  candidate: &ExternalComponentGuidanceCandidate,
  view: ExternalComponentGuidanceView,
) -> bool {
  match view {
    ExternalComponentGuidanceView::Dashboard => true,
    ExternalComponentGuidanceView::CpuDetail => {
      candidate.component == ExternalComponent::Pawnio
        && candidate.usage == ExternalComponentUsage::CpuPackageTemperature
    }
    ExternalComponentGuidanceView::StorageHealth => {
      candidate.component == ExternalComponent::Smartctl
        && candidate.usage == ExternalComponentUsage::StorageHealth
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn pawnio_candidate() -> hardviz_core::models::ExternalComponentGuidanceCandidate {
    hardviz_core::models::ExternalComponentGuidanceCandidate::pawnio_cpu_package_temperature(
      "PawnIOLib.dll not found".to_string(),
    )
  }

  fn smartctl_candidate() -> hardviz_core::models::ExternalComponentGuidanceCandidate {
    hardviz_core::models::ExternalComponentGuidanceCandidate::smartctl_storage_health(
      vec!["temperature".to_string()],
      Some(1),
      "smartctl is not installed".to_string(),
    )
  }

  #[test]
  fn pending_candidates_filter_acknowledged_keys() {
    let state = ExternalComponentGuidanceState::default();
    state.record_candidates(vec![pawnio_candidate(), smartctl_candidate()]);

    let pending = state.pending_candidates(
      ExternalComponentGuidanceView::Dashboard,
      &["pawnio:cpu-package-temperature:v1".to_string()],
    );

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].key, "smartctl:storage-health:v1");
  }

  #[test]
  fn defer_for_session_hides_key_until_process_restart() {
    let state = ExternalComponentGuidanceState::default();
    state.record_candidates(vec![pawnio_candidate()]);

    state.defer_for_session("pawnio:cpu-package-temperature:v1");

    let pending = state.pending_candidates(ExternalComponentGuidanceView::Dashboard, &[]);
    assert!(pending.is_empty());
  }

  #[test]
  fn pending_candidates_are_filtered_by_view() {
    let state = ExternalComponentGuidanceState::default();
    state.record_candidates(vec![pawnio_candidate(), smartctl_candidate()]);

    let pending = state.pending_candidates(ExternalComponentGuidanceView::CpuDetail, &[]);

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].key, "pawnio:cpu-package-temperature:v1");
  }

  #[test]
  fn pending_candidates_filter_storage_health_view() {
    let state = ExternalComponentGuidanceState::default();
    state.record_candidates(vec![pawnio_candidate(), smartctl_candidate()]);

    let pending =
      state.pending_candidates(ExternalComponentGuidanceView::StorageHealth, &[]);

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].key, "smartctl:storage-health:v1");
  }

  #[test]
  fn normalize_external_component_guidance_key_trims_valid_keys() {
    assert_eq!(
      normalize_external_component_guidance_key(" pawnio:cpu-package-temperature:v1 ")
        .unwrap(),
      "pawnio:cpu-package-temperature:v1"
    );
  }

  #[test]
  fn normalize_external_component_guidance_key_rejects_empty_keys() {
    assert!(normalize_external_component_guidance_key(" \t").is_err());
  }
}
