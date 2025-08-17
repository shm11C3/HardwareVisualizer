use crate::infrastructure;
use crate::structs;

pub async fn get_gpu_usage() -> Result<f32, String> {
  let cards = infrastructure::drm_sys::get_card_ids().await?;

  for card in cards {
    // TODO Vendor ID の判定もインフラ層でやる
    match card.vendor_id.as_str() {
      "0x1002" => {
        if let Ok(usage) = infrastructure::drm_sys::get_amd_gpu_usage(card.id).await {
          return Ok((usage * 100.0) as f32);
        }
      }
      "0x8086" => {
        if let Ok(usage) = infrastructure::drm_sys::get_intel_gpu_usage().await {
          return Ok((usage * 100.0) as f32);
        }
      }
      _ => {}
    }
  }

  Err("Failed to get GPU usage on Linux (non-NVIDIA fallback)".to_string())
}

pub async fn get_gpu_info() -> Result<Vec<structs::hardware::GraphicInfo>, String> {
  use tokio::task::JoinSet;

  let card_ids = infrastructure::drm_sys::get_all_card_ids();
  let mut join_set = JoinSet::new();

  for card_id in card_ids {
    join_set.spawn(async move {
      match infrastructure::drm_sys::detect_gpu_vendor(card_id) {
        infrastructure::drm_sys::GpuVendor::Amd => {
          get_amd_graphic_info(card_id).await.ok()
        }
        infrastructure::drm_sys::GpuVendor::Intel => {
          get_intel_graphic_info(card_id).await.ok()
        }
        _ => None,
      }
      .map(|info| (card_id, info)) // ソート用に card_id を付加
    });
  }

  let mut infos: Vec<(u8, structs::hardware::GraphicInfo)> = Vec::new();
  while let Some(res) = join_set.join_next().await {
    if let Ok(Some((card_id, info))) = res {
      infos.push((card_id, info));
    }
  }

  // 元の card_id 昇順に安定化
  infos.sort_by_key(|(id, _)| *id);

  Ok(infos.into_iter().map(|(_, info)| info).collect())
}

async fn get_amd_graphic_info(
  card_id: u8,
) -> Result<structs::hardware::GraphicInfo, String> {
  const VENDOR_ID: &str = "1002";

  let name = infrastructure::lspci::get_gpu_name_from_lspci_by_vendor_id(VENDOR_ID)
    .unwrap_or_else(|| "Unknown AMD GPU".to_string());

  let clock = infrastructure::kernel::read_pm_info_sclk(card_id).unwrap_or(0);
  let memory_total = infrastructure::drm_sys::read_vram_total_bytes(card_id).unwrap_or(0);

  Ok(structs::hardware::GraphicInfo {
    id: format!("card{card_id}"),
    name,
    vendor_name: "AMD".into(),
    clock,
    memory_size: crate::utils::formatter::format_size(memory_total, 1),
    memory_size_dedicated: crate::utils::formatter::format_size(memory_total, 1),
  })
}

pub async fn get_intel_graphic_info(
  card_id: u8,
) -> Result<structs::hardware::GraphicInfo, String> {
  Ok(structs::hardware::GraphicInfo {
    id: format!("card{card_id}"),
    name: "Intel Integrated Graphics".into(),
    vendor_name: "Intel".into(),
    clock: 0, // 取得困難。未対応で0にしておく
    memory_size: "N/A".into(),
    memory_size_dedicated: "N/A".into(),
  })
}
