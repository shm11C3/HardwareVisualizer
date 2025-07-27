#[cfg(test)]
mod edge_cases_tests {
  use crate::platform::{PlatformFactory, traits::*};
  use crate::structs::hardware::{GraphicInfo, MemoryInfo, NetworkInfo};
  use async_trait::async_trait;
  use std::time::Duration;
  use tokio::time::{sleep, timeout};

  // 様々なエラーパターンをテストするためのモックサービス
  struct SlowMemoryService;
  struct EmptyGpuService;
  struct LargeDataNetworkService;

  #[async_trait]
  impl MemoryService for SlowMemoryService {
    async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
      sleep(Duration::from_millis(100)).await;
      Ok(create_minimal_memory_info())
    }

    async fn get_detailed_memory_info(&self) -> Result<Vec<MemoryInfo>, String> {
      sleep(Duration::from_millis(200)).await;
      Ok(vec![create_minimal_memory_info(); 10])
    }
  }

  #[async_trait]
  impl GpuService for EmptyGpuService {
    async fn get_gpu_usage(&self) -> Result<f32, String> {
      Ok(0.0) // 最小値
    }

    async fn get_nvidia_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
      Ok(vec![]) // 空のリスト
    }

    async fn get_amd_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
      Ok(vec![])
    }

    async fn get_intel_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
      Ok(vec![])
    }

    async fn get_all_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
      Ok(vec![])
    }
  }

  #[async_trait]
  impl NetworkService for LargeDataNetworkService {
    async fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String> {
      // 大量のネットワークインターフェースをシミュレート
      let mut networks = Vec::new();
      for i in 0..50 {
        networks.push(NetworkInfo {
          description: Some(format!("Network Adapter {}", i)),
          mac_address: Some(format!("00:11:22:33:44:{:02x}", i)),
          ipv4: vec![format!("192.168.{}.{}", i / 10, i % 10)],
          ipv6: vec![],
          link_local_ipv6: vec![],
          ip_subnet: vec!["255.255.255.0".to_string()],
          default_ipv4_gateway: vec!["192.168.1.1".to_string()],
          default_ipv6_gateway: vec![],
        });
      }
      Ok(networks)
    }
  }

  fn create_minimal_memory_info() -> MemoryInfo {
    MemoryInfo {
      size: "0 MB".to_string(),
      clock: 0,
      clock_unit: "MHz".to_string(),
      memory_count: 0,
      total_slots: 0,
      memory_type: "Unknown".to_string(),
      is_detailed: false,
    }
  }

  #[tokio::test]
  async fn test_boundary_gpu_usage_values() {
    struct BoundaryGpuService {
      usage: f32,
    }

    #[async_trait]
    impl GpuService for BoundaryGpuService {
      async fn get_gpu_usage(&self) -> Result<f32, String> {
        Ok(self.usage)
      }

      async fn get_nvidia_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
        Ok(vec![])
      }

      async fn get_amd_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
        Ok(vec![])
      }

      async fn get_intel_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
        Ok(vec![])
      }

      async fn get_all_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
        Ok(vec![])
      }
    }

    // 境界値テスト
    let test_cases = vec![0.0, 0.1, 50.0, 99.9, 100.0];

    for usage in test_cases {
      let service = BoundaryGpuService { usage };
      let result = service.get_gpu_usage().await;
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), usage);
    }
  }

  #[tokio::test]
  async fn test_empty_collections() {
    let service = EmptyGpuService;

    let nvidia_result = service.get_nvidia_gpus().await;
    assert!(nvidia_result.is_ok());
    assert!(nvidia_result.unwrap().is_empty());

    let all_result = service.get_all_gpus().await;
    assert!(all_result.is_ok());
    assert!(all_result.unwrap().is_empty());
  }

  #[tokio::test]
  async fn test_large_data_handling() {
    let service = LargeDataNetworkService;
    let result = service.get_network_info().await;

    assert!(result.is_ok());
    let networks = result.unwrap();
    assert_eq!(networks.len(), 50);

    // 各ネットワークが有効なデータを持つことを確認
    for (i, network) in networks.iter().enumerate() {
      assert!(network.description.is_some());
      assert!(network.mac_address.is_some());
      assert!(!network.ipv4.is_empty());
      assert_eq!(network.ipv4[0], format!("192.168.{}.{}", i / 10, i % 10));
    }
  }

  #[tokio::test]
  async fn test_slow_service_performance() {
    let service = SlowMemoryService;

    // 通常の呼び出しが適切な時間内に完了することを確認
    let start = std::time::Instant::now();
    let result = service.get_memory_info().await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    assert!(duration < Duration::from_millis(500));

    // 詳細情報の取得も適切な時間内に完了することを確認
    let start = std::time::Instant::now();
    let detailed_result = service.get_detailed_memory_info().await;
    let duration = start.elapsed();

    assert!(detailed_result.is_ok());
    assert!(duration < Duration::from_millis(1000));
    assert_eq!(detailed_result.unwrap().len(), 10);
  }

  #[tokio::test]
  async fn test_concurrent_access() {
    let platform = PlatformFactory::create();

    // 同じサービスに同時に複数回アクセス
    let _memory_service = platform.memory_service();
    let tasks: Vec<_> = (0..10)
      .map(|_| {
        let service = platform.memory_service();
        tokio::spawn(async move { service.get_memory_info().await })
      })
      .collect();

    // すべてのタスクが完了することを確認
    for task in tasks {
      let result = task.await.unwrap();
      // 結果の成功/失敗は問わないが、パニックしないことを確認
      let _ = result;
    }
  }

  #[tokio::test]
  async fn test_timeout_handling() {
    struct HangingService;

    #[async_trait]
    impl MemoryService for HangingService {
      async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
        sleep(Duration::from_secs(10)).await; // 長時間待機
        Ok(create_minimal_memory_info())
      }

      async fn get_detailed_memory_info(&self) -> Result<Vec<MemoryInfo>, String> {
        Ok(vec![])
      }
    }

    let service = HangingService;
    let result = timeout(Duration::from_millis(100), service.get_memory_info()).await;

    // タイムアウトが正常に動作することを確認
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_invalid_data_handling() {
    struct InvalidDataService;

    #[async_trait]
    impl NetworkService for InvalidDataService {
      async fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String> {
        Ok(vec![NetworkInfo {
          description: None, // 無効なデータ
          mac_address: None,
          ipv4: vec![],
          ipv6: vec![],
          link_local_ipv6: vec![],
          ip_subnet: vec![],
          default_ipv4_gateway: vec![],
          default_ipv6_gateway: vec![],
        }])
      }
    }

    let service = InvalidDataService;
    let result = service.get_network_info().await;

    assert!(result.is_ok());
    let networks = result.unwrap();
    assert_eq!(networks.len(), 1);

    // 無効なデータでもパニックしないことを確認
    let network = &networks[0];
    assert!(network.description.is_none());
    assert!(network.mac_address.is_none());
    assert!(network.ipv4.is_empty());
  }

  #[tokio::test]
  async fn test_error_message_consistency() {
    struct ConsistentErrorService;

    #[async_trait]
    impl GpuService for ConsistentErrorService {
      async fn get_gpu_usage(&self) -> Result<f32, String> {
        Err("GPU_USAGE_ERROR".to_string())
      }

      async fn get_nvidia_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
        Err("NVIDIA_ERROR".to_string())
      }

      async fn get_amd_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
        Err("AMD_ERROR".to_string())
      }

      async fn get_intel_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
        Err("INTEL_ERROR".to_string())
      }

      async fn get_all_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
        Err("ALL_GPU_ERROR".to_string())
      }
    }

    let service = ConsistentErrorService;

    let usage_result = service.get_gpu_usage().await;
    assert!(usage_result.is_err());
    assert_eq!(usage_result.unwrap_err(), "GPU_USAGE_ERROR");

    let nvidia_result = service.get_nvidia_gpus().await;
    assert!(nvidia_result.is_err());
    assert_eq!(nvidia_result.unwrap_err(), "NVIDIA_ERROR");

    let all_result = service.get_all_gpus().await;
    assert!(all_result.is_err());
    assert_eq!(all_result.unwrap_err(), "ALL_GPU_ERROR");
  }

  #[tokio::test]
  async fn test_memory_edge_cases() {
    struct EdgeCaseMemoryService;

    #[async_trait]
    impl MemoryService for EdgeCaseMemoryService {
      async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
        Ok(MemoryInfo {
          size: "999999999 TB".to_string(), // 非現実的に大きな値
          clock: u32::MAX,                  // 最大値
          clock_unit: "".to_string(),       // 空文字列
          memory_count: 0,                  // 最小値
          total_slots: u32::MAX,            // 最大値
          memory_type: "🚀SUPER_MEMORY🚀".to_string(), // 特殊文字
          is_detailed: true,
        })
      }

      async fn get_detailed_memory_info(&self) -> Result<Vec<MemoryInfo>, String> {
        Ok(vec![]) // 空のベクター
      }
    }

    let service = EdgeCaseMemoryService;
    let result = service.get_memory_info().await;

    assert!(result.is_ok());
    let memory_info = result.unwrap();
    assert_eq!(memory_info.clock, u32::MAX);
    assert_eq!(memory_info.memory_count, 0);
    assert!(memory_info.memory_type.contains("🚀"));

    let detailed_result = service.get_detailed_memory_info().await;
    assert!(detailed_result.is_ok());
    assert!(detailed_result.unwrap().is_empty());
  }
}
