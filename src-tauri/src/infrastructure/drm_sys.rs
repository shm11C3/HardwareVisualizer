use std::fs;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CardInfo {
  pub id: u32,
  pub vendor_id: String,
}

pub async fn get_card_ids() -> Result<Vec<CardInfo>, String> {
  use futures::future::join_all;

  let card_futures = (0..=9)
    .filter_map(|card_id| {
      let vendor_path = format!("/sys/class/drm/card{card_id}/device/vendor");
      std::path::Path::new(&vendor_path)
        .exists()
        .then(|| async move {
          tokio::fs::read_to_string(&vendor_path)
            .await
            .map(|content| CardInfo {
              id: card_id,
              vendor_id: content.trim().to_string(),
            })
            .map_err(|e| format!("Failed to read vendor for card{card_id}: {e}"))
        })
    })
    .collect::<Vec<_>>();

  let results = join_all(card_futures).await;

  results.into_iter().collect::<Result<Vec<_>, _>>()
}

pub async fn get_amd_gpu_usage(card_id: u32) -> Result<f64, String> {
  let path = format!("/sys/class/drm/card{card_id}/device/gpu_busy_percent");
  let content = fs::read_to_string(&path)
    .map_err(|e| format!("Failed to read AMD gpu_busy_percent: {e}"))?;

  let percent = content
    .trim()
    .parse::<f32>()
    .map_err(|e| format!("Failed to parse gpu_busy_percent: {e}"))?;

  Ok((percent / 100.0).into())
}

pub async fn get_intel_gpu_usage(card_id: u32) -> Result<f64, String> {
  let output = Command::new("intel_gpu_top")
    .args(["-J", "-s", "1000"]) // JSON出力で1秒だけ取得
    .output()
    .map_err(|e| format!("Failed to run intel_gpu_top: {e}"))?;

  if !output.status.success() {
    return Err("intel_gpu_top failed".to_string());
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  for line in stdout.lines() {
    if line.contains("\"render busy\"") {
      if let Some(value_str) = line.split(':').nth(1) {
        if let Ok(value) = value_str.trim().trim_end_matches(',').parse::<f32>() {
          return Ok((value / 100.0).into());
        }
      }
    }
  }

  Err("Could not parse intel_gpu_top output".to_string())
}
