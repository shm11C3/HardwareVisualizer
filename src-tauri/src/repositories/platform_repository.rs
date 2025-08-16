use crate::platform::PlatformFactory;
use crate::platform::traits::Platform;
use crate::structs;
use async_trait::async_trait;

/// メモリ情報取得のための Repository trait
#[async_trait]
pub trait MemoryRepository: Send + Sync {
  /// 基本的なメモリ情報を取得
  async fn get_memory_info(&self) -> Result<structs::hardware::MemoryInfo, String>;

  /// 詳細なメモリ情報を取得
  async fn get_memory_info_detail(&self)
  -> Result<structs::hardware::MemoryInfo, String>;
}

/// Box<dyn Platform> を使用した MemoryRepository の実装
pub struct MemoryRepositoryImpl {
  platform: Box<dyn Platform>,
}

impl MemoryRepositoryImpl {
  /// 新しい MemoryRepositoryImpl インスタンスを作成
  pub fn new() -> Result<Self, String> {
    let platform =
      PlatformFactory::create().map_err(|e| format!("Failed to create platform: {e}"))?;

    Ok(Self { platform })
  }

  /// 依存注入用のコンストラクタ（テスト用）
  #[cfg(test)]
  pub fn new_with_platform(platform: Box<dyn Platform>) -> Self {
    Self { platform }
  }
}

#[async_trait]
impl MemoryRepository for MemoryRepositoryImpl {
  async fn get_memory_info(&self) -> Result<structs::hardware::MemoryInfo, String> {
    // Repository が Platform の分岐ロジックを管理
    self.platform.get_memory_info().await
  }

  async fn get_memory_info_detail(
    &self,
  ) -> Result<structs::hardware::MemoryInfo, String> {
    // Repository が Platform の分岐ロジックを管理
    self.platform.get_memory_info_detail().await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::platform::traits::{GpuPlatform, MemoryPlatform, NetworkPlatform};
  use crate::structs::hardware::{NetworkInfo, SysInfo};
  use std::future::Future;
  use std::pin::Pin;

  // テスト用のモック Platform
  struct MockPlatform {
    memory_info: structs::hardware::MemoryInfo,
    memory_info_detail: structs::hardware::MemoryInfo,
  }

  impl MemoryPlatform for MockPlatform {
    fn get_memory_info(
      &self,
    ) -> Pin<
      Box<dyn Future<Output = Result<structs::hardware::MemoryInfo, String>> + Send + '_>,
    > {
      let memory_info = self.memory_info.clone();
      Box::pin(async move { Ok(memory_info) })
    }

    fn get_memory_info_detail(
      &self,
    ) -> Pin<
      Box<dyn Future<Output = Result<structs::hardware::MemoryInfo, String>> + Send + '_>,
    > {
      let memory_info = self.memory_info_detail.clone();
      Box::pin(async move { Ok(memory_info) })
    }
  }

  impl GpuPlatform for MockPlatform {
    fn get_gpu_usage(
      &self,
    ) -> Pin<Box<dyn Future<Output = Result<f32, String>> + Send + '_>> {
      Box::pin(async { Ok(50.0) })
    }

    fn get_gpu_info(
      &self,
    ) -> Pin<
      Box<
        dyn Future<Output = Result<Vec<structs::hardware::GraphicInfo>, String>>
          + Send
          + '_,
      >,
    > {
      Box::pin(async { Ok(vec![]) })
    }

    fn get_gpu_temperature(
      &self,
      _temperature_unit: crate::enums::settings::TemperatureUnit,
    ) -> Pin<
      Box<
        dyn Future<Output = Result<Vec<crate::structs::hardware::NameValue>, String>>
          + Send
          + '_,
      >,
    > {
      Box::pin(async { Ok(vec![]) })
    }
  }

  impl NetworkPlatform for MockPlatform {
    fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String> {
      Ok(vec![])
    }
  }

  impl Platform for MockPlatform {
    fn get_system_info(
      &self,
    ) -> Pin<Box<dyn Future<Output = Result<SysInfo, String>> + Send + '_>> {
      Box::pin(async {
        Ok(SysInfo {
          cpu: None,
          memory: None,
          gpus: None,
          storage: vec![],
        })
      })
    }
  }

  #[tokio::test]
  async fn test_memory_repository_creation() {
    let result = MemoryRepositoryImpl::new();
    assert!(
      result.is_ok(),
      "MemoryRepositoryImpl creation should succeed"
    );
  }

  #[tokio::test]
  async fn test_get_memory_info_with_mock() {
    let mock_platform = MockPlatform {
      memory_info: structs::hardware::MemoryInfo {
        size: "16 GB".to_string(),
        clock: 3200,
        clock_unit: "MHz".to_string(),
        memory_count: 2,
        total_slots: 4,
        memory_type: "DDR4".to_string(),
        is_detailed: false,
      },
      memory_info_detail: structs::hardware::MemoryInfo {
        size: "16 GB".to_string(),
        clock: 3200,
        clock_unit: "MHz".to_string(),
        memory_count: 2,
        total_slots: 4,
        memory_type: "DDR4".to_string(),
        is_detailed: true,
      },
    };

    let repository = MemoryRepositoryImpl::new_with_platform(Box::new(mock_platform));
    let result = repository.get_memory_info().await;

    assert!(result.is_ok(), "get_memory_info should succeed with mock");
    let memory_info = result.unwrap();
    assert_eq!(memory_info.memory_type, "DDR4");
    assert_eq!(memory_info.size, "16 GB");
  }

  #[tokio::test]
  async fn test_get_memory_info_detail_with_mock() {
    let mock_platform = MockPlatform {
      memory_info: structs::hardware::MemoryInfo {
        size: "16 GB".to_string(),
        clock: 3200,
        clock_unit: "MHz".to_string(),
        memory_count: 2,
        total_slots: 4,
        memory_type: "DDR4".to_string(),
        is_detailed: false,
      },
      memory_info_detail: structs::hardware::MemoryInfo {
        size: "16 GB".to_string(),
        clock: 3200,
        clock_unit: "MHz".to_string(),
        memory_count: 2,
        total_slots: 4,
        memory_type: "DDR4".to_string(),
        is_detailed: true,
      },
    };

    let repository = MemoryRepositoryImpl::new_with_platform(Box::new(mock_platform));
    let result = repository.get_memory_info_detail().await;

    assert!(
      result.is_ok(),
      "get_memory_info_detail should succeed with mock"
    );
    let memory_info = result.unwrap();
    assert!(
      memory_info.is_detailed,
      "Detailed memory info should have is_detailed = true"
    );
  }
}
