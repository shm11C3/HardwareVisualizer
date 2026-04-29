/// POJO mirror of the Tauri-side `BackendError` enum.
///
/// `src-tauri/src/enums/error.rs::BackendError` is the wire type derived
/// with `specta::Type` for tauri-specta. This Core enum is the platform
/// layer's error type; the App-side `commands::*` translate Core errors
/// into the wire variant before returning them to the frontend.
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BackendError {
  CpuInfoNotAvailable,
  StorageInfoNotAvailable,
  MemoryInfoNotAvailable,
  GraphicInfoNotAvailable,
  NetworkInfoNotAvailable,
  NetworkUsageNotAvailable,
  UnexpectedError,
}
