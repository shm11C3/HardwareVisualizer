#[cfg(test)]
mod tests {
  use crate::platform::{PlatformFactory, traits::*};
  use std::time::Duration;
  use tokio::time::timeout;

  #[tokio::test]
  async fn test_platform_factory_create() {
    let platform = PlatformFactory::create();

    // プラットフォームサービスが正常に作成できることを確認
    let _memory_service = platform.memory_service();
    let _gpu_service = platform.gpu_service();
    let _network_service = platform.network_service();
  }

  #[test]
  fn test_platform_factory_get_platform_name() {
    let platform_name = PlatformFactory::get_platform_name();

    // プラットフォーム名が適切に設定されていることを確認
    #[cfg(target_os = "windows")]
    assert_eq!(platform_name, "Windows");

    #[cfg(target_os = "linux")]
    assert_eq!(platform_name, "Linux");

    #[cfg(target_os = "macos")]
    assert_eq!(platform_name, "macOS");
  }

  #[tokio::test]
  async fn test_memory_service_interface() {
    let platform = PlatformFactory::create();
    let memory_service = platform.memory_service();

    // メモリサービスのインターフェースが実装されていることを確認
    let _memory_result = memory_service.get_memory_info().await;
    let _detailed_memory_result = memory_service.get_detailed_memory_info().await;
  }

  #[tokio::test]
  async fn test_memory_service_timeout() {
    let platform = PlatformFactory::create();
    let memory_service = platform.memory_service();

    // タイムアウトテスト（通常は即座に完了するはず）
    let result = timeout(Duration::from_secs(5), memory_service.get_memory_info()).await;

    assert!(
      result.is_ok(),
      "Memory service should complete within timeout"
    );
  }

  #[tokio::test]
  async fn test_memory_service_consistency() {
    let platform = PlatformFactory::create();
    let memory_service = platform.memory_service();

    // 複数回呼び出して一貫性を確認
    let result1 = memory_service.get_memory_info().await;
    let result2 = memory_service.get_memory_info().await;

    // 両方とも同じ結果タイプ（成功/失敗）であることを確認
    assert_eq!(result1.is_ok(), result2.is_ok());
  }

  #[tokio::test]
  async fn test_gpu_service_interface() {
    let platform = PlatformFactory::create();
    let gpu_service = platform.gpu_service();

    // GPU サービスのインターフェースが実装されていることを確認
    let _usage_result = gpu_service.get_gpu_usage().await;
    let _nvidia_result = gpu_service.get_nvidia_gpus().await;
    let _amd_result = gpu_service.get_amd_gpus().await;
    let _intel_result = gpu_service.get_intel_gpus().await;
    let _all_result = gpu_service.get_all_gpus().await;
  }

  #[tokio::test]
  async fn test_gpu_service_usage_range() {
    let platform = PlatformFactory::create();
    let gpu_service = platform.gpu_service();

    match gpu_service.get_gpu_usage().await {
      Ok(usage) => {
        // GPU 使用率は 0-100% の範囲内であることを確認
        assert!(
          usage >= 0.0 && usage <= 100.0,
          "GPU usage should be between 0 and 100, got: {}",
          usage
        );
      }
      Err(_) => {
        // エラーの場合はスキップ（GPU が利用できない環境など）
      }
    }
  }

  #[tokio::test]
  async fn test_gpu_service_all_vs_individual() {
    let platform = PlatformFactory::create();
    let gpu_service = platform.gpu_service();

    let all_gpus_result = gpu_service.get_all_gpus().await;
    let nvidia_result = gpu_service.get_nvidia_gpus().await;
    let amd_result = gpu_service.get_amd_gpus().await;
    let intel_result = gpu_service.get_intel_gpus().await;

    match all_gpus_result {
      Ok(all_gpus) => {
        let nvidia_count = nvidia_result.unwrap_or_default().len();
        let amd_count = amd_result.unwrap_or_default().len();
        let intel_count = intel_result.unwrap_or_default().len();

        // all_gpus の数は個別 GPU 数の合計以下であることを確認
        assert!(all_gpus.len() <= nvidia_count + amd_count + intel_count);
      }
      Err(_) => {
        // エラーの場合はスキップ
      }
    }
  }

  #[tokio::test]
  async fn test_network_service_interface() {
    let platform = PlatformFactory::create();
    let network_service = platform.network_service();

    // ネットワークサービスのインターフェースが実装されていることを確認
    let _network_result = network_service.get_network_info().await;
  }

  #[tokio::test]
  async fn test_network_service_data_validity() {
    let platform = PlatformFactory::create();
    let network_service = platform.network_service();

    match network_service.get_network_info().await {
      Ok(network_infos) => {
        for info in network_infos {
          // 各ネットワーク情報が有効なデータを持つことを確認
          if let Some(desc) = &info.description {
            assert!(!desc.is_empty(), "Network description should not be empty");
          }
          // 他の検証も必要に応じて追加
        }
      }
      Err(_) => {
        // エラーの場合はスキップ
      }
    }
  }

  #[tokio::test]
  async fn test_concurrent_service_calls() {
    let platform = PlatformFactory::create();
    let memory_service = platform.memory_service();
    let gpu_service = platform.gpu_service();
    let network_service = platform.network_service();

    // 複数のサービスを同時に呼び出してデッドロックしないことを確認
    let (memory_result, gpu_result, network_result) = tokio::join!(
      memory_service.get_memory_info(),
      gpu_service.get_gpu_usage(),
      network_service.get_network_info()
    );

    // 少なくとも呼び出しが完了することを確認（結果の成功/失敗は問わない）
    let _ = (memory_result, gpu_result, network_result);
  }

  #[test]
  fn test_platform_factory_unsupported_os() {
    // 現在サポートされている OS でのテスト
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
      let platform = PlatformFactory::create();
      let _memory_service = platform.memory_service();
    }

    // サポートされていない OS でのテストは実行環境に依存するためスキップ
  }

  #[tokio::test]
  async fn test_error_handling_consistency() {
    let platform = PlatformFactory::create();
    let memory_service = platform.memory_service();
    let gpu_service = platform.gpu_service();
    let network_service = platform.network_service();

    // エラーメッセージが適切な形式であることを確認
    let memory_result = memory_service.get_memory_info().await;
    let gpu_result = gpu_service.get_gpu_usage().await;
    let network_result = network_service.get_network_info().await;

    if let Err(err) = memory_result {
      assert!(!err.is_empty(), "Error message should not be empty");
    }
    if let Err(err) = gpu_result {
      assert!(!err.is_empty(), "Error message should not be empty");
    }
    if let Err(err) = network_result {
      assert!(!err.is_empty(), "Error message should not be empty");
    }
  }

  #[cfg(target_os = "windows")]
  mod windows_tests {
    use super::*;
    use crate::platform::windows::WindowsPlatform;

    #[tokio::test]
    async fn test_windows_platform_specific() {
      let platform = WindowsPlatform::new();

      let memory_service = platform.memory_service();
      let gpu_service = platform.gpu_service();
      let network_service = platform.network_service();

      // Windows 固有のサービスが正常に動作することを確認
      let _memory_result = memory_service.get_memory_info().await;
      let _gpu_result = gpu_service.get_gpu_usage().await;
      let _network_result = network_service.get_network_info().await;
    }
  }

  #[cfg(target_os = "linux")]
  mod linux_tests {
    use super::*;
    use crate::platform::linux::LinuxPlatform;

    #[tokio::test]
    async fn test_linux_platform_specific() {
      let platform = LinuxPlatform::new();

      let memory_service = platform.memory_service();
      let gpu_service = platform.gpu_service();
      let network_service = platform.network_service();

      // Linux 固有のサービスが正常に動作することを確認
      let _memory_result = memory_service.get_memory_info().await;
      let _gpu_result = gpu_service.get_gpu_usage().await;
      let _network_result = network_service.get_network_info().await;
    }
  }

  #[cfg(target_os = "macos")]
  mod macos_tests {
    use super::*;
    use crate::platform::macos::MacOSPlatform;

    #[tokio::test]
    async fn test_macos_platform_specific() {
      let platform = MacOSPlatform::new();

      let memory_service = platform.memory_service();
      let gpu_service = platform.gpu_service();
      let network_service = platform.network_service();

      // macOS のサービスは現在未実装なのでエラーが返されることを確認
      assert!(memory_service.get_memory_info().await.is_err());
      assert!(gpu_service.get_gpu_usage().await.is_err());
      assert!(network_service.get_network_info().await.is_err());
    }
  }
}
