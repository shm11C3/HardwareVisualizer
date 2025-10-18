pub fn get_mem_total_kb() -> std::io::Result<u64> {
  use std::fs;
  use std::io;

  let content = fs::read_to_string("/proc/meminfo")?;
  for line in content.lines() {
    if let Some(mem_kb_str) = line.strip_prefix("MemTotal:") {
      let kb = mem_kb_str
        .split_whitespace()
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No value found"))?;
      return kb
        .parse::<u64>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
    }
  }
  Err(io::Error::new(
    io::ErrorKind::NotFound,
    "MemTotal entry not found",
  ))
}

#[cfg(target_os = "linux")]
#[cfg(test)]
mod tests {
  use std::io;

  // Helper function to test meminfo parsing with mock content
  fn parse_mem_total_from_content(content: &str) -> std::io::Result<u64> {
    for line in content.lines() {
      if let Some(mem_kb_str) = line.strip_prefix("MemTotal:") {
        let kb = mem_kb_str
          .split_whitespace()
          .next()
          .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No value found"))?;
        return kb
          .parse::<u64>()
          .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
      }
    }
    Err(io::Error::new(
      io::ErrorKind::NotFound,
      "MemTotal entry not found",
    ))
  }

  #[test]
  fn test_parse_normal_meminfo() {
    let content = r#"MemTotal:       16384000 kB
MemFree:         8192000 kB
MemAvailable:   12288000 kB
Buffers:          512000 kB
Cached:          2048000 kB
SwapCached:            0 kB
Active:          4096000 kB
Inactive:        1024000 kB"#;

    let result = parse_mem_total_from_content(content);
    assert_eq!(result.unwrap(), 16384000);
  }

  #[test]
  fn test_parse_memtotal_with_spaces() {
    let content = r#"MemTotal:    16777216     kB
MemFree:      8388608 kB"#;

    let result = parse_mem_total_from_content(content);
    assert_eq!(result.unwrap(), 16777216);
  }

  #[test]
  fn test_parse_memtotal_with_tabs() {
    let content = "MemTotal:\t\t33554432\tkB\nMemFree:\t\t16777216\tkB";

    let result = parse_mem_total_from_content(content);
    assert_eq!(result.unwrap(), 33554432);
  }

  #[test]
  fn test_parse_memtotal_first_line() {
    let content = r#"MemTotal:       8388608 kB
MemFree:         4194304 kB
MemAvailable:    6291456 kB"#;

    let result = parse_mem_total_from_content(content);
    assert_eq!(result.unwrap(), 8388608);
  }

  #[test]
  fn test_parse_memtotal_middle_of_file() {
    let content = r#"SomeOtherEntry:  1000 kB
AnotherEntry:    2000 kB
MemTotal:        4194304 kB
MemFree:         2097152 kB"#;

    let result = parse_mem_total_from_content(content);
    assert_eq!(result.unwrap(), 4194304);
  }

  #[test]
  fn test_parse_large_memory_value() {
    let content = r#"MemTotal:       67108864 kB
MemFree:        33554432 kB"#;

    let result = parse_mem_total_from_content(content);
    assert_eq!(result.unwrap(), 67108864);
  }

  #[test]
  fn test_parse_small_memory_value() {
    let content = r#"MemTotal:       1048576 kB
MemFree:         524288 kB"#;

    let result = parse_mem_total_from_content(content);
    assert_eq!(result.unwrap(), 1048576);
  }

  #[test]
  fn test_parse_zero_memory_value() {
    let content = r#"MemTotal:       0 kB
MemFree:        0 kB"#;

    let result = parse_mem_total_from_content(content);
    assert_eq!(result.unwrap(), 0);
  }

  #[test]
  fn test_parse_memtotal_not_found() {
    let content = r#"MemFree:         4194304 kB
MemAvailable:    6291456 kB
Buffers:          262144 kB"#;

    let result = parse_mem_total_from_content(content);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
  }

  #[test]
  fn test_parse_empty_content() {
    let content = "";

    let result = parse_mem_total_from_content(content);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
  }

  #[test]
  fn test_parse_memtotal_no_value() {
    let content = r#"MemTotal:
MemFree:         4194304 kB"#;

    let result = parse_mem_total_from_content(content);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
  }

  #[test]
  fn test_parse_memtotal_invalid_number() {
    let content = r#"MemTotal:       invalid kB
MemFree:         4194304 kB"#;

    let result = parse_mem_total_from_content(content);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
  }

  #[test]
  fn test_parse_memtotal_negative_number() {
    let content = r#"MemTotal:       -1000 kB
MemFree:         4194304 kB"#;

    let result = parse_mem_total_from_content(content);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
  }

  #[test]
  fn test_parse_memtotal_float_number() {
    let content = r#"MemTotal:       1000.5 kB
MemFree:         4194304 kB"#;

    let result = parse_mem_total_from_content(content);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
  }

  #[test]
  fn test_parse_memtotal_with_extra_text() {
    let content = r#"MemTotal:       8388608 kB (some comment)
MemFree:         4194304 kB"#;

    let result = parse_mem_total_from_content(content);
    assert_eq!(result.unwrap(), 8388608);
  }

  #[test]
  fn test_parse_memtotal_case_sensitivity() {
    // Test that it requires exact case matching
    let content = r#"memtotal:       8388608 kB
MEMTOTAL:       8388608 kB
MemTotal:       4194304 kB"#;

    let result = parse_mem_total_from_content(content);
    assert_eq!(result.unwrap(), 4194304); // Should match the correct case
  }

  #[test]
  fn test_parse_multiple_memtotal_entries_first_wins() {
    let content = r#"MemTotal:       8388608 kB
MemTotal:       4194304 kB
MemFree:        2097152 kB"#;

    let result = parse_mem_total_from_content(content);
    assert_eq!(result.unwrap(), 8388608); // Should return first match
  }

  #[test]
  fn test_parse_memtotal_with_different_whitespace() {
    let test_cases = vec![
      "MemTotal:8388608 kB",           // No space after colon
      "MemTotal: 8388608 kB",          // Single space
      "MemTotal:  8388608 kB",         // Double space
      "MemTotal:\t8388608 kB",         // Tab
      "MemTotal:\t\t8388608\tkB",      // Multiple tabs
      "MemTotal:   \t 8388608  \t kB", // Mixed spaces and tabs
    ];

    for content in test_cases {
      let result = parse_mem_total_from_content(content);
      assert_eq!(
        result.unwrap(),
        8388608,
        "Failed for content: '{}'",
        content
      );
    }
  }

  #[test]
  fn test_parse_realistic_meminfo_file() {
    let content = r#"MemTotal:       32768000 kB
MemFree:         1234567 kB
MemAvailable:   24567890 kB
Buffers:          567890 kB
Cached:          8901234 kB
SwapCached:            0 kB
Active:         12345678 kB
Inactive:        3456789 kB
Active(anon):    4567890 kB
Inactive(anon):   234567 kB
Active(file):    7777788 kB
Inactive(file):  3222222 kB
Unevictable:           0 kB
Mlocked:               0 kB
SwapTotal:       8388608 kB
SwapFree:        8388608 kB
Dirty:               256 kB
Writeback:             0 kB
AnonPages:       4444444 kB
Mapped:          1111111 kB
Shmem:            333333 kB
KReclaimable:    2222222 kB
Slab:            2345678 kB
SReclaimable:    1234567 kB
SUnreclaim:      1111111 kB
KernelStack:       12345 kB
PageTables:        67890 kB
NFS_Unstable:          0 kB
Bounce:                0 kB
WritebackTmp:          0 kB
CommitLimit:    24772608 kB
Committed_AS:   10987654 kB
VmallocTotal:   34359738367 kB
VmallocUsed:       34567 kB
VmallocChunk:          0 kB
Percpu:             5432 kB
HardwareCorrupted:     0 kB
AnonHugePages:         0 kB
ShmemHugePages:        0 kB
ShmemPmdMapped:        0 kB
CmaTotal:              0 kB
CmaFree:               0 kB
HugePages_Total:       0
HugePages_Free:        0
HugePages_Rsvd:        0
HugePages_Surp:        0
Hugepagesize:       2048 kB
Hugetlb:               0 kB
DirectMap4k:      567890 kB
DirectMap2M:    32212992 kB
DirectMap1G:     2097152 kB"#;

    let result = parse_mem_total_from_content(content);
    assert_eq!(result.unwrap(), 32768000);
  }
}
