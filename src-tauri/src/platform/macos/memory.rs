use crate::infrastructure::providers;
use crate::models::hardware::MemoryInfo;
use crate::utils::formatter::format_size;

/// Returns memory information on macOS.
///
/// - Total physical memory is obtained via `sysctl` (`hw.memsize`).
/// - Detailed module-level information (type / speed / slot counts) is best-effort and comes from
///   `system_profiler SPMemoryDataType -json`.
///
/// `is_detailed` is set to `true` only when at least one detailed field can be extracted.
pub fn get_memory_info() -> Result<MemoryInfo, String> {
  // Total physical memory bytes.
  let total_bytes = providers::sysctl::sysctl_u64("hw.memsize")?;

  let details = providers::system_profiler::get_raw_spmemory_json()
    .ok()
    .and_then(|raw| providers::system_profiler::parse_spmemory_json(&raw).ok());

  let clock = details
    .as_ref()
    .and_then(|d| d.clock_mhz)
    .filter(|v| *v > 0)
    .unwrap_or(0);

  let memory_count = details
    .as_ref()
    .and_then(|d| d.memory_count)
    .filter(|v| *v > 0)
    .unwrap_or(0);

  let total_slots = details
    .as_ref()
    .and_then(|d| d.total_slots)
    .filter(|v| *v > 0)
    .unwrap_or(0);

  let memory_type = details
    .as_ref()
    .and_then(|d| d.memory_type.as_deref())
    .map(str::trim)
    .filter(|typ| !typ.is_empty() && *typ != "Unknown")
    .unwrap_or("Unknown")
    .to_string();

  let has_any_detail =
    clock > 0 || memory_count > 0 || total_slots > 0 || memory_type != "Unknown";

  Ok(MemoryInfo {
    size: format_size(total_bytes, 1),
    clock,
    clock_unit: "MHz".to_string(),
    memory_count,
    total_slots,
    memory_type,
    is_detailed: has_any_detail,
  })
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
  use super::*;

  #[test]
  fn get_memory_info_returns_sane_values() {
    let info = get_memory_info().expect("get_memory_info should succeed on macOS");

    assert!(!info.size.is_empty());
    assert!(info.size.contains(' '));

    assert_eq!(info.clock_unit, "MHz");
    assert!(!info.memory_type.is_empty());
  }

  #[test]
  fn get_memory_info_size_matches_sysctl_formatted_value() {
    let total_bytes = providers::sysctl::sysctl_u64("hw.memsize")
      .expect("sysctl hw.memsize should succeed");
    let expected = format_size(total_bytes, 1);

    let info = get_memory_info().expect("get_memory_info should succeed on macOS");
    assert_eq!(info.size, expected);
  }

  #[test]
  fn get_memory_info_is_detailed_is_consistent_with_returned_fields() {
    let info = get_memory_info().expect("get_memory_info should succeed on macOS");

    let expected = info.clock > 0
      || info.memory_count > 0
      || info.total_slots > 0
      || info.memory_type != "Unknown";

    assert_eq!(info.is_detailed, expected);
  }

  #[test]
  fn get_memory_info_memory_type_is_trimmed() {
    let info = get_memory_info().expect("get_memory_info should succeed on macOS");
    assert_eq!(info.memory_type, info.memory_type.trim());
  }

  #[test]
  fn get_memory_info_size_contains_digits() {
    let info = get_memory_info().expect("get_memory_info should succeed on macOS");
    assert!(info.size.chars().any(|c| c.is_ascii_digit()));
  }
}
