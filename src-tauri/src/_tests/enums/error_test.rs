#[cfg(test)]
mod tests {
  use crate::enums::error::BackendError;
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
    // Verify that all error variants are defined
    let errors = vec![
      BackendError::CpuInfoNotAvailable,
      BackendError::StorageInfoNotAvailable,
      BackendError::MemoryInfoNotAvailable,
      BackendError::GraphicInfoNotAvailable,
      BackendError::NetworkInfoNotAvailable,
      BackendError::NetworkUsageNotAvailable,
      BackendError::UnexpectedError,
    ];

    // Verify that each error can be serialized
    for error in errors {
      assert!(serde_json::to_string(&error).is_ok());
    }
  }

  #[test]
  fn test_error_serialization_format() {
    let error = BackendError::CpuInfoNotAvailable;
    let serialized = serde_json::to_string(&error).unwrap();

    // Verify that it's in camelCase format
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

    // Verify that all serialization results are unique
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
