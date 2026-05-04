use hardviz_core::enums::error as core_err;
use serde::Serialize;
use specta::Type;

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone, Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BackendError {
  CpuInfoNotAvailable,
  StorageInfoNotAvailable,
  MemoryInfoNotAvailable,
  GraphicInfoNotAvailable,
  NetworkInfoNotAvailable,
  NetworkUsageNotAvailable,
  UnexpectedError,
  // SystemError(String),
}

impl From<core_err::BackendError> for BackendError {
  fn from(src: core_err::BackendError) -> Self {
    match src {
      core_err::BackendError::CpuInfoNotAvailable => Self::CpuInfoNotAvailable,
      core_err::BackendError::StorageInfoNotAvailable => Self::StorageInfoNotAvailable,
      core_err::BackendError::MemoryInfoNotAvailable => Self::MemoryInfoNotAvailable,
      core_err::BackendError::GraphicInfoNotAvailable => Self::GraphicInfoNotAvailable,
      core_err::BackendError::NetworkInfoNotAvailable => Self::NetworkInfoNotAvailable,
      core_err::BackendError::NetworkUsageNotAvailable => Self::NetworkUsageNotAvailable,
      core_err::BackendError::UnexpectedError => Self::UnexpectedError,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json;

  #[test]
  fn test_backend_error_debug() {
    let error = BackendError::CpuInfoNotAvailable;
    let debug_string = format!("{:?}", error);
    assert_eq!(debug_string, "CpuInfoNotAvailable");
  }

  #[test]
  fn test_backend_error_clone() {
    let original = BackendError::MemoryInfoNotAvailable;
    let cloned = original.clone();
    assert_eq!(original, cloned);
  }

  #[test]
  fn test_backend_error_partial_eq() {
    assert_eq!(
      BackendError::CpuInfoNotAvailable,
      BackendError::CpuInfoNotAvailable
    );
    assert_ne!(
      BackendError::CpuInfoNotAvailable,
      BackendError::MemoryInfoNotAvailable
    );
    assert_eq!(BackendError::UnexpectedError, BackendError::UnexpectedError);
  }

  #[test]
  fn test_backend_error_serialization() {
    let test_cases = vec![
      (BackendError::CpuInfoNotAvailable, "cpuInfoNotAvailable"),
      (
        BackendError::StorageInfoNotAvailable,
        "storageInfoNotAvailable",
      ),
      (
        BackendError::MemoryInfoNotAvailable,
        "memoryInfoNotAvailable",
      ),
      (
        BackendError::GraphicInfoNotAvailable,
        "graphicInfoNotAvailable",
      ),
      (
        BackendError::NetworkInfoNotAvailable,
        "networkInfoNotAvailable",
      ),
      (
        BackendError::NetworkUsageNotAvailable,
        "networkUsageNotAvailable",
      ),
      (BackendError::UnexpectedError, "unexpectedError"),
    ];

    for (error, expected_json) in test_cases {
      let serialized = serde_json::to_string(&error).unwrap();
      assert_eq!(serialized, format!("\"{}\"", expected_json));
    }
  }

  #[test]
  fn test_all_error_variants_exist() {
    let errors = vec![
      BackendError::CpuInfoNotAvailable,
      BackendError::StorageInfoNotAvailable,
      BackendError::MemoryInfoNotAvailable,
      BackendError::GraphicInfoNotAvailable,
      BackendError::NetworkInfoNotAvailable,
      BackendError::NetworkUsageNotAvailable,
      BackendError::UnexpectedError,
    ];

    for error in errors {
      assert!(serde_json::to_string(&error).is_ok());
    }
  }

  #[test]
  fn test_error_serialization_format() {
    let error = BackendError::CpuInfoNotAvailable;
    let serialized = serde_json::to_string(&error).unwrap();

    assert!(serialized.contains("cpuInfoNotAvailable"));
    assert!(!serialized.contains("cpu_info_not_available"));
  }

  #[test]
  fn test_error_variants_uniqueness() {
    let errors = vec![
      BackendError::CpuInfoNotAvailable,
      BackendError::StorageInfoNotAvailable,
      BackendError::MemoryInfoNotAvailable,
      BackendError::GraphicInfoNotAvailable,
      BackendError::NetworkInfoNotAvailable,
      BackendError::NetworkUsageNotAvailable,
      BackendError::UnexpectedError,
    ];

    let mut serialized_values = std::collections::HashSet::new();
    for error in errors {
      let serialized = serde_json::to_string(&error).unwrap();
      assert!(
        serialized_values.insert(serialized),
        "Duplicate serialization found"
      );
    }
  }
}
