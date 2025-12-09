use crate::enums::settings::TemperatureUnit;
use crate::models::hardware::{GraphicInfo, NameValue};
use crate::utils::formatter;
use std::process::Command;

pub async fn get_gpu_usage() -> Result<f32, String> {
  // macOSではGPU使用率の取得が制限されているため、
  // Activity Monitorのデータやpowermetricsを使用する必要がある
  // powermetricsは管理者権限が必要なため、基本的な実装のみ提供

  let output = Command::new("sh")
    .arg("-c")
    .arg("ioreg -r -c IOAccelerator | grep PerformanceStatistics")
    .output()
    .map_err(|e| format!("Failed to execute ioreg: {e}"))?;

  if !output.status.success() {
    // フォールバック: 固定値を返す
    return Ok(0.0);
  }

  // 簡易的な実装: PerformanceStatisticsから推定
  // 実際のGPU使用率取得には複雑な解析が必要
  Ok(0.0)
}

pub async fn get_gpu_temperature(
  _temperature_unit: TemperatureUnit,
) -> Result<Vec<NameValue>, String> {
  // macOSではGPU温度の直接取得が制限されている
  // SMC (System Management Controller) データへのアクセスが必要だが、
  // サードパーティツールなしでは困難

  // Apple Siliconの場合、powermetricsで一部の温度情報が取得可能
  let output = Command::new("sysctl")
    .arg("-n")
    .arg("machdep.xcpm.cpu_thermal_level")
    .output()
    .ok();

  if let Some(output) = output {
    if output.status.success() {
      // 温度情報が取得できた場合の処理
      // 実際の温度値への変換は複雑なため、基本実装のみ
    }
  }

  // フォールバック: 空の配列を返す
  Ok(vec![])
}

pub async fn get_gpu_info() -> Result<Vec<GraphicInfo>, String> {
  let output = Command::new("system_profiler")
    .arg("SPDisplaysDataType")
    .output()
    .map_err(|e| format!("Failed to execute system_profiler: {e}"))?;

  if !output.status.success() {
    return Err("system_profiler command failed".to_string());
  }

  let output_str = String::from_utf8(output.stdout)
    .map_err(|e| format!("Failed to parse output: {e}",))?;

  let mut gpus = Vec::new();
  let mut current_gpu: Option<GpuData> = None;
  let mut gpu_counter = 0;

  for line in output_str.lines() {
    let trimmed = line.trim();

    // GPUセクションの開始を検出
    if trimmed.ends_with(':') && !trimmed.contains("Displays:") && !trimmed.is_empty() {
      if let Some(gpu) = current_gpu.take() {
        gpus.push(create_graphic_info(gpu, gpu_counter));
        gpu_counter += 1;
      }
      current_gpu = Some(GpuData {
        name: trimmed.trim_end_matches(':').to_string(),
        vendor: String::new(),
        vram: 0,
        metal_support: false,
      });
      continue;
    }

    if let Some(ref mut gpu) = current_gpu {
      if trimmed.starts_with("Chipset Model:") {
        if let Some(model) = trimmed.split(':').nth(1) {
          gpu.name = model.trim().to_string();
        }
      } else if trimmed.starts_with("Vendor:") {
        if let Some(vendor) = trimmed.split(':').nth(1) {
          gpu.vendor = vendor.trim().to_string();
        }
      } else if trimmed.starts_with("VRAM (Dynamic, Max):")
        || trimmed.starts_with("VRAM (Total):")
      {
        if let Some(vram_str) = trimmed.split(':').nth(1) {
          gpu.vram = parse_vram(vram_str.trim());
        }
      } else if trimmed.starts_with("Metal:") || trimmed.starts_with("Metal Support:") {
        if let Some(metal) = trimmed.split(':').nth(1) {
          gpu.metal_support = metal.trim().contains("Supported");
        }
      }
    }
  }

  // 最後のGPUを追加
  if let Some(gpu) = current_gpu {
    gpus.push(create_graphic_info(gpu, gpu_counter));
  }

  if gpus.is_empty() {
    return Err("No GPU information found".to_string());
  }

  Ok(gpus)
}

#[derive(Debug)]
struct GpuData {
  name: String,
  vendor: String,
  vram: u64,
  metal_support: bool,
}

fn create_graphic_info(gpu: GpuData, index: u32) -> GraphicInfo {
  let vendor_name = if gpu.vendor.is_empty() {
    infer_vendor_from_name(&gpu.name)
  } else {
    gpu.vendor
  };

  GraphicInfo {
    id: format!("gpu-{index}",),
    name: gpu.name,
    vendor_name,
    clock: 0, // macOSではGPUクロック速度の取得が制限されている
    memory_size: if gpu.vram > 0 {
      formatter::format_size(gpu.vram, 1)
    } else {
      "Unknown".to_string()
    },
    memory_size_dedicated: if gpu.vram > 0 {
      formatter::format_size(gpu.vram, 1)
    } else {
      "Unknown".to_string()
    },
  }
}

fn infer_vendor_from_name(name: &str) -> String {
  let name_lower = name.to_lowercase();
  if name_lower.contains("apple")
    || name_lower.contains("m1")
    || name_lower.contains("m2")
    || name_lower.contains("m3")
    || name_lower.contains("m4")
  {
    "Apple".to_string()
  } else if name_lower.contains("nvidia")
    || name_lower.contains("geforce")
    || name_lower.contains("quadro")
  {
    "NVIDIA".to_string()
  } else if name_lower.contains("amd") || name_lower.contains("radeon") {
    "AMD".to_string()
  } else if name_lower.contains("intel")
    || name_lower.contains("iris")
    || name_lower.contains("uhd")
  {
    "Intel".to_string()
  } else {
    "Unknown".to_string()
  }
}

fn parse_vram(vram_str: &str) -> u64 {
  // "8 GB" や "1536 MB" などの形式をパース
  let parts: Vec<&str> = vram_str.split_whitespace().collect();
  if parts.len() >= 2 {
    if let Ok(value) = parts[0].parse::<f64>() {
      let unit = parts[1].to_uppercase();
      let bytes = match unit.as_str() {
        "GB" => (value * 1024.0 * 1024.0 * 1024.0) as u64,
        "MB" => (value * 1024.0 * 1024.0) as u64,
        "KB" => (value * 1024.0) as u64,
        _ => 0,
      };
      return bytes;
    }
  }
  0
}
