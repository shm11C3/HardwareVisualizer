#[cfg(test)]
mod tests {
  use crate::enums::hardware::*;
  use serde_json;

  #[test]
  fn test_hardware_type_serialization() {
    let test_cases = vec![
      (HardwareType::Cpu, "cpu"),
      (HardwareType::Memory, "memory"),
      (HardwareType::Gpu, "gpu"),
    ];

    for (hardware_type, expected_json) in test_cases {
      let serialized = serde_json::to_string(&hardware_type).unwrap();
      assert_eq!(serialized, format!("\"{}\"", expected_json));
    }
  }

  #[test]
  fn test_hardware_type_deserialization() {
    let test_cases = vec![
      ("\"cpu\"", HardwareType::Cpu),
      ("\"memory\"", HardwareType::Memory),
      ("\"gpu\"", HardwareType::Gpu),
      ("\"CPU\"", HardwareType::Cpu), // 大文字でも処理できることを確認
      ("\"MEMORY\"", HardwareType::Memory),
      ("\"GPU\"", HardwareType::Gpu),
    ];

    for (json_str, expected_type) in test_cases {
      let deserialized: HardwareType = serde_json::from_str(json_str).unwrap();
      assert_eq!(deserialized, expected_type);
    }
  }

  #[test]
  fn test_hardware_type_deserialization_invalid() {
    let invalid_cases = vec!["\"invalid\"", "\"network\"", "\"storage\""];

    for invalid_json in invalid_cases {
      let result: Result<HardwareType, _> = serde_json::from_str(invalid_json);
      assert!(result.is_err());
    }
  }

  #[test]
  fn test_hardware_type_clone_and_eq() {
    let original = HardwareType::Cpu;
    let cloned = original.clone();
    assert_eq!(original, cloned);

    assert_eq!(HardwareType::Memory, HardwareType::Memory);
    assert_ne!(HardwareType::Cpu, HardwareType::Memory);
  }

  #[test]
  fn test_disk_kind_serialization() {
    let test_cases = vec![
      (DiskKind::Hdd, "hdd"),
      (DiskKind::Ssd, "ssd"),
      (DiskKind::Unknown, "other"),
    ];

    for (disk_kind, expected_json) in test_cases {
      let serialized = serde_json::to_string(&disk_kind).unwrap();
      assert_eq!(serialized, format!("\"{}\"", expected_json));
    }
  }

  #[test]
  fn test_disk_kind_deserialization() {
    let test_cases = vec![
      ("\"hdd\"", DiskKind::Hdd),
      ("\"ssd\"", DiskKind::Ssd),
      ("\"other\"", DiskKind::Unknown),
    ];

    for (json_str, expected_type) in test_cases {
      let deserialized: DiskKind = serde_json::from_str(json_str).unwrap();
      assert_eq!(deserialized, expected_type);
    }
  }

  #[test]
  fn test_disk_kind_display() {
    assert_eq!(DiskKind::Hdd.to_string(), "HDD");
    assert_eq!(DiskKind::Ssd.to_string(), "SSD");
    assert_eq!(DiskKind::Unknown.to_string(), "Other");
  }

  #[test]
  fn test_disk_kind_from_sysinfo() {
    assert_eq!(DiskKind::from(sysinfo::DiskKind::HDD), DiskKind::Hdd);
    assert_eq!(DiskKind::from(sysinfo::DiskKind::SSD), DiskKind::Ssd);
    assert_eq!(
      DiskKind::from(sysinfo::DiskKind::Unknown(-1)),
      DiskKind::Unknown
    );
  }

  #[test]
  fn test_disk_kind_clone_and_eq() {
    let original = DiskKind::Ssd;
    let cloned = original.clone();
    assert_eq!(original, cloned);

    assert_eq!(DiskKind::Hdd, DiskKind::Hdd);
    assert_ne!(DiskKind::Ssd, DiskKind::Hdd);
  }

  #[test]
  fn test_all_hardware_types_covered() {
    // すべてのHardwareTypeバリアントがテストされていることを確認
    let all_types = vec![HardwareType::Cpu, HardwareType::Memory, HardwareType::Gpu];

    // 各タイプがシリアライズ可能であることを確認
    for hardware_type in all_types {
      assert!(serde_json::to_string(&hardware_type).is_ok());
    }
  }

  #[test]
  fn test_all_disk_kinds_covered() {
    // すべてのDiskKindバリアントがテストされていることを確認
    let all_kinds = vec![DiskKind::Hdd, DiskKind::Ssd, DiskKind::Unknown];

    // 各タイプがシリアライズ可能であることを確認
    for disk_kind in all_kinds {
      assert!(serde_json::to_string(&disk_kind).is_ok());
    }
  }
}
