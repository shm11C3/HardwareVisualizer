use crate::infrastructure::providers::smartctl::{self, SmartctlDevice};
use crate::models::hardware::SmartDiskInfo;
use std::process::Command;

pub fn get_smart_info() -> Result<Vec<SmartDiskInfo>, String> {
  match smartctl::collect_smart_info_from_scan() {
    Ok(disks) => Ok(disks),
    Err(scan_error) => {
      let devices = diskutil_device_candidates();
      if devices.is_empty() {
        return Err(scan_error);
      }

      smartctl::collect_smart_info_from_devices(&devices).map_err(|fallback_error| {
        format!(
          "Failed to collect SMART info from smartctl scan ({scan_error}) and macOS diskutil candidates ({fallback_error})"
        )
      })
    }
  }
}

fn diskutil_device_candidates() -> Vec<SmartctlDevice> {
  let output = Command::new("diskutil").arg("list").output();
  let Ok(output) = output else {
    return Vec::new();
  };
  if !output.status.success() {
    return Vec::new();
  }

  parse_diskutil_list(&String::from_utf8_lossy(&output.stdout))
    .into_iter()
    .map(|device| SmartctlDevice::new(device, None))
    .collect()
}

fn parse_diskutil_list(output: &str) -> Vec<String> {
  let mut devices = output
    .lines()
    .filter_map(|line| {
      let line = line.trim();
      if !(line.contains("physical") && line.starts_with("/dev/disk")) {
        return None;
      }

      line
        .split_whitespace()
        .next()
        .map(str::trim)
        .filter(|device| is_whole_disk_identifier(device))
        .map(ToOwned::to_owned)
    })
    .collect::<Vec<_>>();

  devices.sort();
  devices.dedup();
  devices
}

fn is_whole_disk_identifier(device: &str) -> bool {
  let Some(rest) = device.strip_prefix("/dev/disk") else {
    return false;
  };

  !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_physical_diskutil_devices() {
    let output = r#"
/dev/disk0 (internal, physical):
   #:                       TYPE NAME                    SIZE       IDENTIFIER
   0:      GUID_partition_scheme                        *500.3 GB   disk0
   1:                        EFI EFI                     314.6 MB   disk0s1

/dev/disk3 (synthesized):
   #:                       TYPE NAME                    SIZE       IDENTIFIER
   0:      APFS Container Scheme -                      +499.8 GB   disk3

/dev/disk4 (external, physical):
   #:                       TYPE NAME                    SIZE       IDENTIFIER
   0:      GUID_partition_scheme                        *1.0 TB     disk4
"#;

    let devices = parse_diskutil_list(output);

    assert_eq!(devices, vec!["/dev/disk0", "/dev/disk4"]);
  }

  #[test]
  fn whole_disk_identifier_excludes_partitions() {
    assert!(is_whole_disk_identifier("/dev/disk0"));
    assert!(!is_whole_disk_identifier("/dev/disk0s1"));
    assert!(!is_whole_disk_identifier("disk0"));
  }
}
