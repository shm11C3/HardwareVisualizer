#[cfg(test)]
mod traits_tests {
  use crate::platform::traits::*;
  use crate::structs::hardware::{GraphicInfo, MemoryInfo, NetworkInfo};
  use async_trait::async_trait;
  use std::sync::Arc;

  // テスト用の最小実装
  struct TestMemoryService;
  struct TestGpuService;
  struct TestNetworkService;
  struct TestPlatformServices;

  #[async_trait]
  impl MemoryService for TestMemoryService {
    async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
      Ok(create_test_memory_info())
    }

    async fn get_detailed_memory_info(&self) -> Result<Vec<MemoryInfo>, String> {
      Ok(vec![create_test_memory_info()])
    }
  }

  #[async_trait]
  impl GpuService for TestGpuService {
    async fn get_gpu_usage(&self) -> Result<f32, String> {
      Ok(75.5)
    }

    async fn get_nvidia_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
      Ok(vec![create_test_gpu_info()])
    }

    async fn get_amd_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
      Ok(vec![])
    }

    async fn get_intel_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
      Ok(vec![])
    }

    async fn get_all_gpus(&self) -> Result<Vec<GraphicInfo>, String> {
      Ok(vec![create_test_gpu_info()])
    }
  }

  #[async_trait]
  impl NetworkService for TestNetworkService {
    async fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String> {
      Ok(vec![create_test_network_info()])
    }
  }

  impl PlatformServices for TestPlatformServices {
    fn memory_service(&self) -> Box<dyn MemoryService> {
      Box::new(TestMemoryService)
    }

    fn gpu_service(&self) -> Box<dyn GpuService> {
      Box::new(TestGpuService)
    }

    fn network_service(&self) -> Box<dyn NetworkService> {
      Box::new(TestNetworkService)
    }
  }

  fn create_test_memory_info() -> MemoryInfo {
    MemoryInfo {
      size: "8 GB".to_string(),
      clock: 2400,
      clock_unit: "MHz".to_string(),
      memory_count: 1,
      total_slots: 2,
      memory_type: "DDR4".to_string(),
      is_detailed: false,
    }
  }

  fn create_test_gpu_info() -> GraphicInfo {
    GraphicInfo {
      id: "test-gpu".to_string(),
      name: "Test Graphics Card".to_string(),
      vendor_name: "Test Vendor".to_string(),
      clock: 1500,
      memory_size: "4 GB".to_string(),
      memory_size_dedicated: "4 GB".to_string(),
    }
  }

  fn create_test_network_info() -> NetworkInfo {
    NetworkInfo {
      description: Some("Test Adapter".to_string()),
      mac_address: Some("11:22:33:44:55:66".to_string()),
      ipv4: vec!["192.168.1.10".to_string()],
      ipv6: vec![],
      link_local_ipv6: vec![],
      ip_subnet: vec!["255.255.255.0".to_string()],
      default_ipv4_gateway: vec!["192.168.1.1".to_string()],
      default_ipv6_gateway: vec![],
    }
  }

  #[tokio::test]
  async fn test_memory_service_trait() {
    let service = TestMemoryService;

    let result = service.get_memory_info().await;
    assert!(result.is_ok());

    let memory_info = result.unwrap();
    assert_eq!(memory_info.size, "8 GB");
    assert_eq!(memory_info.clock, 2400);

    let detailed_result = service.get_detailed_memory_info().await;
    assert!(detailed_result.is_ok());
    assert_eq!(detailed_result.unwrap().len(), 1);
  }

  #[tokio::test]
  async fn test_gpu_service_trait() {
    let service = TestGpuService;

    let usage_result = service.get_gpu_usage().await;
    assert!(usage_result.is_ok());
    assert_eq!(usage_result.unwrap(), 75.5);

    let nvidia_result = service.get_nvidia_gpus().await;
    assert!(nvidia_result.is_ok());
    assert_eq!(nvidia_result.unwrap().len(), 1);

    let amd_result = service.get_amd_gpus().await;
    assert!(amd_result.is_ok());
    assert_eq!(amd_result.unwrap().len(), 0);

    let all_result = service.get_all_gpus().await;
    assert!(all_result.is_ok());
    assert_eq!(all_result.unwrap().len(), 1);
  }

  #[tokio::test]
  async fn test_network_service_trait() {
    let service = TestNetworkService;

    let result = service.get_network_info().await;
    assert!(result.is_ok());

    let network_infos = result.unwrap();
    assert_eq!(network_infos.len(), 1);
    assert_eq!(network_infos[0].ipv4, vec!["192.168.1.10"]);
  }

  #[tokio::test]
  async fn test_platform_services_trait() {
    let platform = TestPlatformServices;

    let memory_service = platform.memory_service();
    let gpu_service = platform.gpu_service();
    let network_service = platform.network_service();

    // 各サービスが正常に動作することを確認
    let memory_result = memory_service.get_memory_info().await;
    let gpu_result = gpu_service.get_gpu_usage().await;
    let network_result = network_service.get_network_info().await;

    assert!(memory_result.is_ok());
    assert!(gpu_result.is_ok());
    assert!(network_result.is_ok());
  }

  #[tokio::test]
  async fn test_send_sync_traits() {
    // Send + Sync トレイトが正しく実装されていることを確認
    let memory_service: Box<dyn MemoryService> = Box::new(TestMemoryService);
    let gpu_service: Box<dyn GpuService> = Box::new(TestGpuService);
    let network_service: Box<dyn NetworkService> = Box::new(TestNetworkService);

    // Arc で包んでマルチスレッド環境での使用をシミュレート
    let memory_arc = Arc::new(memory_service);
    let gpu_arc = Arc::new(gpu_service);
    let network_arc = Arc::new(network_service);

    let memory_clone = memory_arc.clone();
    let gpu_clone = gpu_arc.clone();
    let network_clone = network_arc.clone();

    // 別のタスクで実行
    let task = tokio::spawn(async move {
      let _memory_result = memory_clone.get_memory_info().await;
      let _gpu_result = gpu_clone.get_gpu_usage().await;
      let _network_result = network_clone.get_network_info().await;
    });

    task.await.unwrap();
  }

  #[tokio::test]
  async fn test_trait_error_handling() {
    struct FailingService;

    #[async_trait]
    impl MemoryService for FailingService {
      async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
        Err("Memory service failed".to_string())
      }

      async fn get_detailed_memory_info(&self) -> Result<Vec<MemoryInfo>, String> {
        Err("Detailed memory service failed".to_string())
      }
    }

    let service = FailingService;
    let result = service.get_memory_info().await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Memory service failed");
  }
}
