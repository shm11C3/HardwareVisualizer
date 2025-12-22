use std::process::Command;

use serde_json::Value;

fn get_spmemory_json() -> Result<Value, String> {
  let output = Command::new("system_profiler")
    .arg("-json")
    .arg("SPMemoryDataType")
    .output()
    .map_err(|e| format!("Failed to execute system_profiler: {e}"))?;

  if !output.status.success() {
    return Err("system_profiler command failed".to_string());
  }

  serde_json::from_slice::<Value>(&output.stdout)
    .map_err(|e| format!("Failed to parse output as JSON: {e}"))
}

fn parse_speed_with_unit(value: &str) -> Option<(u32, String)> {
  let mut parts = value.split_whitespace();
  let speed = parts.next()?.parse::<u32>().ok()?;
  let unit = parts.next().unwrap_or("MHz").to_string();
  Some((speed, unit))
}

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
  let json = get_spmemory_json()?;
  let items = json
    .get("SPMemoryDataType")
    .and_then(|v| v.as_array())
    .ok_or_else(|| "Memory type not found".to_string())?;

  items
    .iter()
    .find_map(|item| item.get("dimm_type").and_then(|v| v.as_str()))
    .map(|s| s.to_string())
    .ok_or_else(|| "Memory type not found".to_string())
}

pub fn get_memory_speed_with_unit() -> Result<(u32, String), String> {
  let json = get_spmemory_json()?;
  let items = json
    .get("SPMemoryDataType")
    .and_then(|v| v.as_array())
    .ok_or_else(|| "Memory speed not found".to_string())?;

  items
    .iter()
    .find_map(|item| item.get("dimm_speed").and_then(|v| v.as_str()))
    .and_then(parse_speed_with_unit)
    .ok_or_else(|| {
      "Memory speed not available from system_profiler on this system".to_string()
    })
}

pub fn get_memory_count() -> Result<u32, String> {
  let json = get_spmemory_json()?;
  let items = json
    .get("SPMemoryDataType")
    .and_then(|v| v.as_array())
    .ok_or_else(|| "Memory count not found".to_string())?;

  let count = items.len() as u32;
  if count > 0 { Ok(count) } else { Ok(1) }
}
