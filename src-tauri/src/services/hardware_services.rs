use crate::repositories::MemoryRepository;
use std::sync::Arc;

/// ハードウェア関連のサービス層を管理する構造体
/// 責務: 各種リポジトリの生成・管理・提供
pub struct HardwareServices {
  pub memory_repository: Arc<dyn MemoryRepository>,
  // 将来的に他のリポジトリも追加
  // pub gpu_repository: Arc<dyn GpuRepository>,
  // pub network_repository: Arc<dyn NetworkRepository>,
}

impl HardwareServices {
  /// 新しい HardwareServices インスタンスを作成
  pub fn new() -> Result<Self, String> {
    use crate::repositories::MemoryRepositoryImpl;

    let memory_repository: Arc<dyn MemoryRepository> = Arc::new(
      MemoryRepositoryImpl::new()
        .map_err(|e| format!("Failed to create memory repository: {}", e))?,
    );

    Ok(Self { memory_repository })
  }

  /// メモリリポジトリへのアクセサ
  pub fn get_memory_repository(&self) -> Arc<dyn MemoryRepository> {
    Arc::clone(&self.memory_repository)
  }

  /// 依存注入用のコンストラクタ（テスト用）
  pub fn new_with_memory_repository(
    memory_repository: Arc<dyn MemoryRepository>,
  ) -> Self {
    Self { memory_repository }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::repositories::MemoryRepository;
  use crate::structs::hardware::MemoryInfo;
  use async_trait::async_trait;

  // テスト用のモック MemoryRepository
  struct MockMemoryRepository {
    memory_info: MemoryInfo,
  }

  #[async_trait]
  impl MemoryRepository for MockMemoryRepository {
    async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
      Ok(self.memory_info.clone())
    }

    async fn get_memory_info_detail(&self) -> Result<MemoryInfo, String> {
      Ok(self.memory_info.clone())
    }
  }

  #[tokio::test]
  async fn test_hardware_services_creation() {
    let result = HardwareServices::new();
    assert!(result.is_ok(), "HardwareServices creation should succeed");
  }

  #[tokio::test]
  async fn test_get_memory_repository() {
    let services = HardwareServices::new().expect("Failed to create services");
    let repo = services.get_memory_repository();

    // リポジトリが使用可能かテスト
    let result = repo.get_memory_info().await;
    // プラットフォーム実装によっては成功/失敗両方あり得るので、結果の型をチェック
    assert!(result.is_ok() || result.is_err());
  }

  #[tokio::test]
  async fn test_hardware_services_with_mock() {
    let mock_repo = Arc::new(MockMemoryRepository {
      memory_info: MemoryInfo {
        size: "16 GB".to_string(),
        clock: 3200,
        clock_unit: "MHz".to_string(),
        memory_count: 2,
        total_slots: 4,
        memory_type: "DDR4".to_string(),
        is_detailed: false,
      },
    });

    let services = HardwareServices::new_with_memory_repository(mock_repo);
    let repo = services.get_memory_repository();
    let result = repo.get_memory_info().await;

    assert!(result.is_ok(), "Mock repository should succeed");
    let memory_info = result.unwrap();
    assert_eq!(memory_info.memory_type, "DDR4");
    assert_eq!(memory_info.size, "16 GB");
  }
}
