use std::process::Command;

pub async fn get_intel_gpu_usage(_card_id: u8) -> Result<f32, String> {
  let output = Command::new("intel_gpu_top")
    .args(&["-J", "-s", "1000"]) // JSON出力で1秒だけ取得
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
          return Ok(value / 100.0);
        }
      }
    }
  }

  Err("Could not parse intel_gpu_top output".to_string())
}
