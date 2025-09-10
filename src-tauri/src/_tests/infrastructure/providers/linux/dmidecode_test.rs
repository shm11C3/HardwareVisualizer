#[cfg(target_os = "linux")]
#[cfg(test)]
mod tests {
  use crate::infrastructure::providers::linux::dmidecode::parse_dmidecode_memory_info;

  #[test]
  fn test_parse_normal_dmidecode_gb_output() {
    let raw = r#"
# dmidecode 3.3
Getting SMBIOS data from sysfs.
SMBIOS 3.0.0 present.

Handle 0x0010, DMI type 17, 40 bytes
Memory Device
        Array Handle: 0x000F
        Error Information Handle: Not Provided
        Total Width: 64 bits
        Data Width: 64 bits
        Size: 16 GB
        Form Factor: DIMM
        Set: None
        Locator: DIMM 0
        Bank Locator: P0_Node0_Channel0_Dimm0
        Type: DDR4
        Type Detail: Synchronous
        Speed: 3200 MT/s
        Manufacturer: Corsair
        Serial Number: 00000000
        Asset Tag: 9876543210
        Part Number: CMK16GX4M1D3000C16
        Rank: 2
        Configured Memory Speed: 3200 MT/s
        Minimum Voltage: 1.200 V
        Maximum Voltage: 1.200 V
        Configured Voltage: 1.200 V

Handle 0x0011, DMI type 17, 40 bytes
Memory Device
        Array Handle: 0x000F
        Error Information Handle: Not Provided
        Total Width: 64 bits
        Data Width: 64 bits
        Size: 16 GB
        Form Factor: DIMM
        Set: None
        Locator: DIMM 1
        Bank Locator: P0_Node0_Channel1_Dimm0
        Type: DDR4
        Type Detail: Synchronous
        Speed: 3200 MT/s
        Manufacturer: Corsair
        Serial Number: 00000000
        Asset Tag: 9876543210
        Part Number: CMK16GX4M1D3000C16
        Rank: 2
        Configured Memory Speed: 3200 MT/s
        Minimum Voltage: 1.200 V
        Maximum Voltage: 1.200 V
        Configured Voltage: 1.200 V
"#;

    let result = parse_dmidecode_memory_info(raw);

    assert_eq!(result.size, "32.0 GB");
    assert_eq!(result.memory_type, "DDR4");
    assert_eq!(result.clock, 1600); // 3200 MT/s / 2 = 1600 MHz
    assert_eq!(result.clock_unit, "MHz");
    assert_eq!(result.memory_count, 2);
    assert_eq!(result.total_slots, 2);
    assert!(result.is_detailed);
  }

  #[test]
  fn test_parse_mixed_gb_mb_output() {
    let raw = r#"
Handle 0x0010, DMI type 17, 40 bytes
Memory Device
        Size: 8 GB
        Type: DDR4
        Configured Memory Speed: 2400 MT/s

Handle 0x0011, DMI type 17, 40 bytes
Memory Device
        Size: 512 MB
        Type: DDR4
        Configured Memory Speed: 2400 MT/s
"#;

    let result = parse_dmidecode_memory_info(raw);

    assert_eq!(result.size, "8.5 GB");
    assert_eq!(result.memory_type, "DDR4");
    assert_eq!(result.clock, 1200); // 2400 MT/s / 2
    assert_eq!(result.memory_count, 2);
    assert_eq!(result.total_slots, 2);
  }

  #[test]
  fn test_parse_ddr5_memory() {
    let raw = r#"
Handle 0x0010, DMI type 17, 40 bytes
Memory Device
        Size: 32 GB
        Type: DDR5
        Configured Memory Speed: 4800 MT/s

Handle 0x0011, DMI type 17, 40 bytes
Memory Device
        Size: 32 GB
        Type: DDR5
        Configured Memory Speed: 4800 MT/s
"#;

    let result = parse_dmidecode_memory_info(raw);

    assert_eq!(result.size, "64.0 GB");
    assert_eq!(result.memory_type, "DDR5");
    assert_eq!(result.clock, 2400); // 4800 MT/s / 2
    assert_eq!(result.memory_count, 2);
    assert_eq!(result.total_slots, 2);
  }

  #[test]
  fn test_parse_empty_slots() {
    let raw = r#"
Handle 0x0010, DMI type 17, 40 bytes
Memory Device
        Size: 16 GB
        Type: DDR4
        Configured Memory Speed: 3200 MT/s

Handle 0x0011, DMI type 17, 40 bytes
Memory Device
        Size: No Module Installed
        Form Factor: DIMM

Handle 0x0012, DMI type 17, 40 bytes
Memory Device
        Size: Unknown
        Form Factor: DIMM

Handle 0x0013, DMI type 17, 40 bytes
Memory Device
        Size: 16 GB
        Type: DDR4
        Configured Memory Speed: 3200 MT/s
"#;

    let result = parse_dmidecode_memory_info(raw);

    assert_eq!(result.size, "32.0 GB");
    assert_eq!(result.memory_type, "DDR4");
    assert_eq!(result.memory_count, 2); // Only actual memory modules
    assert_eq!(result.total_slots, 4); // All slots including empty ones
  }

  #[test]
  fn test_parse_unknown_type_fallback() {
    let raw = r#"
Handle 0x0010, DMI type 17, 40 bytes
Memory Device
        Size: 8 GB
        Type: Unknown
        Configured Memory Speed: 2133 MT/s

Handle 0x0011, DMI type 17, 40 bytes
Memory Device
        Size: 8 GB
        Type: RAM
        Configured Memory Speed: 2133 MT/s
"#;

    let result = parse_dmidecode_memory_info(raw);

    assert_eq!(result.memory_type, "Unknown");
    assert_eq!(result.clock, 1066); // 2133 MT/s / 2
  }

  #[test]
  fn test_parse_multiple_types_first_valid_wins() {
    let raw = r#"
Handle 0x0010, DMI type 17, 40 bytes
Memory Device
        Size: 8 GB
        Type: DDR4
        Configured Memory Speed: 2400 MT/s

Handle 0x0011, DMI type 17, 40 bytes
Memory Device
        Size: 8 GB
        Type: DDR3
        Configured Memory Speed: 1600 MT/s
"#;

    let result = parse_dmidecode_memory_info(raw);

    assert_eq!(result.memory_type, "DDR4"); // First valid type found
    assert_eq!(result.clock, 1200); // First clock speed found (2400 MT/s / 2)
  }

  #[test]
  fn test_parse_no_clock_speed() {
    let raw = r#"
Handle 0x0010, DMI type 17, 40 bytes
Memory Device
        Size: 16 GB
        Type: DDR4
"#;

    let result = parse_dmidecode_memory_info(raw);

    assert_eq!(result.clock, 0);
    assert_eq!(result.clock_unit, "MHz");
  }

  #[test]
  fn test_parse_empty_input() {
    let raw = "";
    let result = parse_dmidecode_memory_info(raw);

    assert_eq!(result.size, "0.0 B");
    assert_eq!(result.memory_type, "Unknown");
    assert_eq!(result.clock, 0);
    assert_eq!(result.memory_count, 0);
    assert_eq!(result.total_slots, 0);
  }

  #[test]
  fn test_parse_malformed_size() {
    let raw = r#"
Handle 0x0010, DMI type 17, 40 bytes
Memory Device
        Size: Invalid GB
        Type: DDR4
        Configured Memory Speed: 2400 MT/s
"#;

    let result = parse_dmidecode_memory_info(raw);

    assert_eq!(result.size, "0.0 B"); // Should handle parse error gracefully
    assert_eq!(result.memory_count, 0);
    assert_eq!(result.total_slots, 1); // Slot is counted even if size is invalid
  }

  #[test]
  fn test_parse_malformed_clock_speed() {
    let raw = r#"
Handle 0x0010, DMI type 17, 40 bytes
Memory Device
        Size: 16 GB
        Type: DDR4
        Configured Memory Speed: Invalid MT/s
"#;

    let result = parse_dmidecode_memory_info(raw);

    assert_eq!(result.clock, 0); // Should handle parse error gracefully
  }

  #[test]
  fn test_parse_different_units() {
    let raw = r#"
Handle 0x0010, DMI type 17, 40 bytes
Memory Device
        Size: 1024 MB
        Type: DDR3

Handle 0x0011, DMI type 17, 40 bytes
Memory Device
        Size: 2 GB
        Type: DDR3
"#;

    let result = parse_dmidecode_memory_info(raw);

    assert_eq!(result.size, "3.0 GB"); // 1024 MB + 2 GB = 3 GB
    assert_eq!(result.memory_count, 2);
  }

  #[test]
  fn test_regex_edge_cases() {
    let raw = r#"
Handle 0x0010, DMI type 17, 40 bytes
Memory Device
	Size: 8 GB
	Type: DDR4
	Configured Memory Speed: 3000 MT/s

Handle 0x0011, DMI type 17, 40 bytes
    Memory Device
        Size:   16   GB   
        Type:   DDR4   
        Configured Memory Speed:   2666   MT/s   
"#;

    let result = parse_dmidecode_memory_info(raw);

    assert_eq!(result.size, "24.0 GB");
    assert_eq!(result.memory_type, "DDR4");
    assert_eq!(result.clock, 1500); // First speed found: 3000 MT/s / 2
    assert_eq!(result.memory_count, 2);
    assert_eq!(result.total_slots, 2);
  }
}
