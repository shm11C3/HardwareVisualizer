use crate::infrastructure;

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
        if let Ok(usage) = infrastructure::drm_sys::get_intel_gpu_usage(card.id).await {
          return Ok((usage * 100.0) as f32);
        }
      }
      _ => {}
    }
  }

  Err("Failed to get GPU usage on Linux (non-NVIDIA fallback)".to_string())
}
