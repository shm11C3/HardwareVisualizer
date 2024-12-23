use serde::{Serialize, Serializer};
use specta::Type;

#[derive(Debug, PartialEq, Eq, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub enum BackendError {
  CpuInfoNotAvailable,
  StorageInfoNotAvailable,
  MemoryInfoNotAvailable,
  GraphicInfoNotAvailable,
  NetworkInfoNotAvailable,
  NetworkUsageNotAvailable,
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
      //   BackendError::SystemError(ref e) => e,
    };
    serializer.serialize_str(s)
  }
}
