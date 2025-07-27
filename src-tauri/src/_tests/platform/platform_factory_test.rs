#[cfg(test)]
mod factory_tests {
  use crate::platform::factory::PlatformFactory;

  #[test]
  fn test_factory_create_returns_valid_instance() {
    let platform = PlatformFactory::create();

    // PlatformServices トレイトのメソッドが呼び出せることを確認
    let _memory_service = platform.memory_service();
    let _gpu_service = platform.gpu_service();
    let _network_service = platform.network_service();
  }

  #[test]
  fn test_factory_create_multiple_instances() {
    // 複数のインスタンスを作成できることを確認
    let platform1 = PlatformFactory::create();
    let platform2 = PlatformFactory::create();

    // 両方とも有効であることを確認
    let _memory1 = platform1.memory_service();
    let _memory2 = platform2.memory_service();
  }

  #[test]
  fn test_platform_name_consistency() {
    let name1 = PlatformFactory::get_platform_name();
    let name2 = PlatformFactory::get_platform_name();

    // 一貫した名前が返されることを確認
    assert_eq!(name1, name2);
    assert!(!name1.is_empty());
  }

  #[test]
  fn test_platform_name_matches_os() {
    let platform_name = PlatformFactory::get_platform_name();

    // OS に応じた適切な名前が返されることを確認
    #[cfg(target_os = "windows")]
    assert_eq!(platform_name, "Windows");

    #[cfg(target_os = "linux")]
    assert_eq!(platform_name, "Linux");

    #[cfg(target_os = "macos")]
    assert_eq!(platform_name, "macOS");

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    assert_eq!(platform_name, "Unknown");
  }

  #[tokio::test]
  async fn test_services_are_functional() {
    let platform = PlatformFactory::create();

    let memory_service = platform.memory_service();
    let gpu_service = platform.gpu_service();
    let network_service = platform.network_service();

    // 各サービスがパニックせずに呼び出せることを確認
    let _memory_result = memory_service.get_memory_info().await;
    let _gpu_result = gpu_service.get_gpu_usage().await;
    let _network_result = network_service.get_network_info().await;
  }

  #[test]
  fn test_factory_thread_safety() {
    use std::thread;

    // 複数のスレッドから同時にファクトリーを呼び出す
    let handles: Vec<_> = (0..4)
      .map(|_| {
        thread::spawn(|| {
          let platform = PlatformFactory::create();
          let name = PlatformFactory::get_platform_name();
          (platform.memory_service(), name)
        })
      })
      .collect();

    // すべてのスレッドが正常に完了することを確認
    for handle in handles {
      let (_, name) = handle.join().unwrap();
      assert!(!name.is_empty());
    }
  }
}
