use crate::structs::hardware::GraphicInfo;

use crate::{log_debug, log_error, log_info, log_internal, log_warn};
use dxgi::adapter::{Adapter, AdapterDesc};
use dxgi::Factory;
use std::error::Error;
use tokio::task::spawn_blocking;

/// Intel GPU情報を取得する
//pub async fn get_intel_gpu_info() -> Result<Vec<GraphicInfo>, String> {
//  let handle = spawn_blocking(|| {
//    log_debug!("start", "get_intel_gpu_info", None::<&str>);
//
//    let factory = Factory::new().map_err(|e| {
//        log_error!("factory_creation_failed", "get_intel_gpu_info", Some(e.to_string()));
//        "Failed to create DXGI Factory".to_string()
//    })?;
//    for adapter in factory.adapters().map_err(|e| {
//        log_error!("adapter_enumeration_failed", "get_intel_gpu_info", Some(e.to_string()));
//        "Failed to enumerate adapters".to_string()
//    })? {
//    let mut gpu_info_list = Vec::new();
//
//    for adapter in factory.adapters() {
//      let desc = match adapter.get_desc() {
//        Ok(desc) => desc,
//        Err(e) => {
//          log_error!(
//            "description_failed",
//            "get_intel_gpu_info",
//            Some(e.to_string())
//          );
//          continue;
//        }
//      };
//
//      let gpu_name = String::from_utf16_lossy(&desc.Description);
//      let memory_size_dedicated = desc.DedicatedVideoMemory / 1024 / 1024;
//      let memory_size_shared = desc.SharedSystemMemory / 1024 / 1024;
//
//      // Intel GPU の場合のみ追加
//      if gpu_name.contains("Intel") {
//        let gpu_info = GraphicInfo {
//          name: gpu_name.trim_end_matches('\0').to_string(),
//          vendor_name: "Intel".to_string(),
//          clock: 0, // Intelのクロック周波数取得が難しいため0を設定
//          memory_size: format!("{} MB", memory_size_shared),
//          memory_size_dedicated: format!("{} MB", memory_size_dedicated),
//        };
//
//        gpu_info_list.push(gpu_info);
//      }
//    }
//
//    log_debug!("end", "get_intel_gpu_info", None::<&str>);
//
//    Ok(gpu_info_list)
//  })
//
//  handle.await.map_err(|e| {
//    log_error!("join_error", "get_intel_gpu_info", Some(e.to_string()));
//    "Intel GPU info retrieval failed".to_string()
//  })?
//}

/// AMD GPU情報を取得する
pub async fn get_amd_gpu_info() -> Result<Vec<GraphicInfo>, String> {
  let handle = spawn_blocking(|| {
    log_debug!("start", "get_amd_gpu_info", None::<&str>);

    // DXGIファクトリのインスタンスを作成
    let factory = Factory::new().expect("Failed to create DXGI Factory");
    let mut gpu_info_list = Vec::new();

    // アダプタの列挙
    for adapter in factory.adapters() {
      // get_desc() の結果をアンラップしてエラーハンドリング
      let desc: AdapterDesc = adapter.get_desc();

      // GPU 名の取得
      let gpu_name = desc.description();
      if gpu_name.contains("AMD") || gpu_name.contains("Radeon") {
        let memory_size_dedicated = desc.dedicated_video_memory() / 1024 / 1024;
        let memory_size_shared = desc.shared_system_memory() / 1024 / 1024;

        let gpu_info = GraphicInfo {
          name: gpu_name.trim_end_matches('\0').to_string(),
          vendor_name: "AMD".to_string(),
          clock: 0, // クロック取得が難しいため 0 に設定
          memory_size: format!("{} MB", memory_size_shared),
          memory_size_dedicated: format!("{} MB", memory_size_dedicated),
        };

        gpu_info_list.push(gpu_info);
      }
    }

    log_debug!("end", "get_amd_gpu_info", None::<&str>);
    Ok(gpu_info_list)
  });

  handle.await.map_err(|e| {
    log_error!("join_error", "get_amd_gpu_info", Some(e.to_string()));
    "AMD GPU info retrieval failed".to_string()
  })?
}
