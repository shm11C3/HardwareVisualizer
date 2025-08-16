use crate::enums;
use crate::enums::error::BackendError;
use crate::structs;
use std::future::Future;
use std::pin::Pin;

/// プラットフォーム固有のメモリ操作を定義する trait
pub trait MemoryPlatform: Send + Sync {
  /// 基本的なメモリ情報を取得
  fn get_memory_info(
    &self,
  ) -> Pin<
    Box<dyn Future<Output = Result<structs::hardware::MemoryInfo, String>> + Send + '_>,
  >;

  /// 詳細なメモリ情報を取得（対応プラットフォームのみ）
  fn get_memory_info_detail(
    &self,
  ) -> Pin<
    Box<dyn Future<Output = Result<structs::hardware::MemoryInfo, String>> + Send + '_>,
  >;
}

/// プラットフォーム固有の GPU 操作を定義する trait
pub trait GpuPlatform: Send + Sync {
  /// GPU 使用率を取得
  fn get_gpu_usage(
    &self,
  ) -> Pin<Box<dyn Future<Output = Result<f32, String>> + Send + '_>>;

  /// GPU 温度を取得
  fn get_gpu_temperature(
    &self,
    temperature_unit: enums::settings::TemperatureUnit,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Vec<structs::hardware::NameValue>, String>> + Send + '_,
    >,
  >;

  /// GPU 情報を取得
  fn get_gpu_info(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<Vec<structs::hardware::GraphicInfo>, String>>
        + Send
        + '_,
    >,
  >;
}

/// プラットフォーム固有のネットワーク操作を定義する trait
pub trait NetworkPlatform: Send + Sync {
  /// ネットワーク情報を取得
  #[allow(dead_code)]
  fn get_network_info(
    &self,
  ) -> Result<Vec<crate::structs::hardware::NetworkInfo>, BackendError>;
}

/// 全てのプラットフォーム機能を統合する trait
pub trait Platform: MemoryPlatform + GpuPlatform + NetworkPlatform {
  /// システム情報を取得
  #[allow(dead_code)]
  fn get_system_info(
    &self,
  ) -> Pin<
    Box<
      dyn Future<Output = Result<crate::structs::hardware::SysInfo, String>> + Send + '_,
    >,
  >;
}
