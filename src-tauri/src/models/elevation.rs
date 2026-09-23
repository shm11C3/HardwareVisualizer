use hardviz_core::platform::traits::ElevationAvailability as CoreElevationAvailability;
use serde::Serialize;
use specta::Type;

// No doc comments on enum variants: tauri-specta renders them as a multi-line
// union with trailing whitespace, which the CI whitespace gate rejects.

/// Whether the app can relaunch or run itself as administrator (#2216).
/// `unprotectedLocation` means it is installed outside Program Files, where a
/// same-user process could replace the executable before it is elevated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ElevationAvailability {
  Available,
  UnprotectedLocation,
  Unsupported,
}

impl From<CoreElevationAvailability> for ElevationAvailability {
  fn from(value: CoreElevationAvailability) -> Self {
    match value {
      CoreElevationAvailability::Available => Self::Available,
      CoreElevationAvailability::UnprotectedLocation => Self::UnprotectedLocation,
      CoreElevationAvailability::Unsupported => Self::Unsupported,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn serializes_as_camel_case() {
    assert_eq!(
      serde_json::to_string(&ElevationAvailability::UnprotectedLocation).unwrap(),
      "\"unprotectedLocation\""
    );
    assert_eq!(
      ElevationAvailability::from(CoreElevationAvailability::Available),
      ElevationAvailability::Available
    );
  }
}
