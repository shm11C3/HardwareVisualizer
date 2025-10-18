use serde::{Serialize, Serializer};
use specta::Type;

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone, Type)]
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

impl Serialize for BackendError {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let s = match *self {
      BackendError::CpuInfoNotAvailable => "cpuInfoNotAvailable",
      BackendError::StorageInfoNotAvailable => "storageInfoNotAvailable",
      BackendError::MemoryInfoNotAvailable => "memoryInfoNotAvailable",
      BackendError::GraphicInfoNotAvailable => "graphicInfoNotAvailable",
      BackendError::NetworkInfoNotAvailable => "networkInfoNotAvailable",
      BackendError::NetworkUsageNotAvailable => "networkUsageNotAvailable",
      BackendError::UnexpectedError => "unexpectedError",
      //   BackendError::SystemError(ref e) => e,
    };
    serializer.serialize_str(s)
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
    // すべてのエラーバリアントが定義されていることを確認
    let errors = vec![
      BackendError::CpuInfoNotAvailable,
      BackendError::StorageInfoNotAvailable,
      BackendError::MemoryInfoNotAvailable,
      BackendError::GraphicInfoNotAvailable,
      BackendError::NetworkInfoNotAvailable,
      BackendError::NetworkUsageNotAvailable,
      BackendError::UnexpectedError,
    ];

    // 各エラーがシリアライズ可能であることを確認
    for error in errors {
      assert!(serde_json::to_string(&error).is_ok());
    }
  }

  #[test]
  fn test_error_serialization_format() {
    let error = BackendError::CpuInfoNotAvailable;
    let serialized = serde_json::to_string(&error).unwrap();

    // キャメルケース形式であることを確認
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

    // すべてのシリアライズ結果が異なることを確認
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
