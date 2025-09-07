#[cfg(test)]
mod tests {
  use crate::models::hardware::HardwareMonitorState;
  use crate::services::memory_service::*;
  use std::collections::VecDeque;
  use std::sync::{Arc, Mutex};
  use sysinfo::System;

  #[test]
  fn test_memory_usage_percent_normal() {
    let mut system = System::new();
    system.refresh_memory();

    let state = HardwareMonitorState {
      system: Arc::new(Mutex::new(system)),
      cpu_history: Arc::new(Mutex::new(VecDeque::new())),
      memory_history: Arc::new(Mutex::new(VecDeque::new())),
      gpu_history: Arc::new(Mutex::new(VecDeque::new())),
      process_cpu_histories: Arc::new(Mutex::new(std::collections::HashMap::new())),
      process_memory_histories: Arc::new(Mutex::new(std::collections::HashMap::new())),
      nv_gpu_usage_histories: Arc::new(Mutex::new(std::collections::HashMap::new())),
      nv_gpu_temperature_histories: Arc::new(Mutex::new(std::collections::HashMap::new())),
    };

    let usage = memory_usage_percent(&state);
    assert!(
      usage >= 0 && usage <= 100,
      "Memory usage should be between 0 and 100%"
    );
  }

  #[test]
  fn test_memory_usage_percent_zero_total() {
    let system = System::new();

    let state = HardwareMonitorState {
      system: Arc::new(Mutex::new(system)),
      cpu_history: Arc::new(Mutex::new(VecDeque::new())),
      memory_history: Arc::new(Mutex::new(VecDeque::new())),
      gpu_history: Arc::new(Mutex::new(VecDeque::new())),
      process_cpu_histories: Arc::new(Mutex::new(std::collections::HashMap::new())),
      process_memory_histories: Arc::new(Mutex::new(std::collections::HashMap::new())),
      nv_gpu_usage_histories: Arc::new(Mutex::new(std::collections::HashMap::new())),
      nv_gpu_temperature_histories: Arc::new(Mutex::new(std::collections::HashMap::new())),
    };

    let usage = memory_usage_percent(&state);
    assert_eq!(usage, 0, "Memory usage should be 0% when total memory is 0");
  }

  #[tokio::test]
  async fn test_fetch_memory_detail_error_handling() {
    // Platform が見つからない場合のエラーハンドリングをテスト
    // 実際のプラットフォーム実装に依存するため、エラーメッセージの形式をチェック
    match fetch_memory_detail().await {
      Ok(_) => {
        // プラットフォームが正常に動作する場合
        assert!(true);
      }
      Err(error) => {
        // プラットフォーム初期化エラーの場合
        assert!(
          error.contains("Failed to create platform") || error.contains("platform"),
          "Error message should contain platform-related information: {}",
          error
        );
      }
    }
  }
}
