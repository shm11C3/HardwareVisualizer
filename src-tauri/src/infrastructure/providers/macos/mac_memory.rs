use std::process::Command;

pub fn get_hw_memsize() -> Result<u64, String> {
  let output = Command::new("sysctl")
    .arg("-n")
    .arg("hw.memsize")
    .output()
    .map_err(|e| format!("Failed to execute sysctl: {e}"))?;

  if !output.status.success() {
    return Err("sysctl command failed".to_string());
  }

  let memory_str = String::from_utf8(output.stdout)
    .map_err(|e| format!("Failed to parse output: {e}"))?;

  memory_str
    .trim()
    .parse::<u64>()
    .map_err(|e| format!("Failed to parse memory size: {e}"))
}

pub fn get_memory_type() -> Result<String, String> {
  let output = Command::new("system_profiler")
    .arg("SPMemoryDataType")
    .output()
    .map_err(|e| format!("Failed to execute system_profiler: {e}"))?;

  if !output.status.success() {
    return Err("system_profiler command failed".to_string());
  }

  let output_str = String::from_utf8(output.stdout)
    .map_err(|e| format!("Failed to parse output: {e}"))?;

  // "Type: DDR4" のような行を探す
  for line in output_str.lines() {
    if line.trim().starts_with("Type:") {
      if let Some(mem_type) = line.split(':').nth(1) {
        return Ok(mem_type.trim().to_string());
      }
    }
  }

  Err("Memory type not found".to_string())
}

pub fn get_memory_speed() -> Result<u32, String> {
  let output = Command::new("system_profiler")
    .arg("SPMemoryDataType")
    .output()
    .map_err(|e| format!("Failed to execute system_profiler: {e}"))?;

  if !output.status.success() {
    return Err("system_profiler command failed".to_string());
  }

  let output_str = String::from_utf8(output.stdout)
    .map_err(|e| format!("Failed to parse output: {e}"))?;

  // "Speed: 2667 MHz" のような行を探す
  for line in output_str.lines() {
    if line.trim().starts_with("Speed:") {
      if let Some(speed_part) = line.split(':').nth(1) {
        // "2667 MHz" から数字部分を抽出
        let speed_str = speed_part.split_whitespace().next().unwrap_or("0");
        if let Ok(speed) = speed_str.parse::<u32>() {
          return Ok(speed);
        }
      }
    }
  }

  Err("Memory speed not found".to_string())
}

pub fn get_memory_count() -> Result<u32, String> {
  let output = Command::new("system_profiler")
    .arg("SPMemoryDataType")
    .output()
    .map_err(|e| format!("Failed to execute system_profiler: {e}"))?;

  if !output.status.success() {
    return Err("system_profiler command failed".to_string());
  }

  let output_str = String::from_utf8(output.stdout)
    .map_err(|e| format!("Failed to parse output: {e}"))?;

  // メモリスロット情報をカウント（"DIMM" や "BANK" で始まる行）
  let count = output_str
    .lines()
    .filter(|line| {
      let trimmed = line.trim();
      trimmed.starts_with("DIMM") || trimmed.starts_with("BANK")
    })
    .count();

  if count > 0 {
    Ok(count as u32)
  } else {
    // Apple Siliconの場合は1とする
    Ok(1)
  }
}
