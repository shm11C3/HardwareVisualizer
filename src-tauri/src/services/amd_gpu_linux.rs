use std::fs;

pub async fn get_amd_gpu_usage(card_id: u8) -> Result<f32, String> {
  let path = format!("/sys/class/drm/card{}/device/gpu_busy_percent", card_id);
  let content = fs::read_to_string(&path)
    .map_err(|e| format!("Failed to read AMD gpu_busy_percent: {e}"))?;

  let percent = content
    .trim()
    .parse::<f32>()
    .map_err(|e| format!("Failed to parse gpu_busy_percent: {e}"))?;

  Ok(percent / 100.0)
}
