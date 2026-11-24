use crate::infrastructure::providers::smartctl::{self, SmartctlDevice};
use crate::models::external_component_guidance::all_storage_health_signal_names;
use crate::models::hardware::SmartDiskInfo;
use crate::models::{ExternalComponentGuidanceCandidate, SmartInfoCollectionOutcome};
use std::fs;

pub fn get_smart_info() -> Result<Vec<SmartDiskInfo>, String> {
  get_smart_info_with_guidance().disks
}

pub fn get_smart_info_with_guidance() -> SmartInfoCollectionOutcome {
  match smartctl::collect_smart_info_from_scan() {
    Ok(disks) => SmartInfoCollectionOutcome {
      disks: Ok(disks),
      guidance_candidates: Vec::new(),
    },
    Err(scan_error) => {
      let devices = linux_device_candidates();
      if devices.is_empty() {
        return SmartInfoCollectionOutcome {
          disks: Err(scan_error.clone()),
          guidance_candidates: vec![
            ExternalComponentGuidanceCandidate::smartctl_storage_health(
              all_storage_health_signal_names(),
              None,
              scan_error,
            ),
          ],
        };
      }

      match smartctl::collect_smart_info_from_devices(&devices) {
        Ok(disks) => SmartInfoCollectionOutcome {
          disks: Ok(disks),
          guidance_candidates: Vec::new(),
        },
        Err(fallback_error) => {
          let detail = format!(
            "Failed to collect SMART info from smartctl scan ({scan_error}) and Linux device candidates ({fallback_error})"
          );
          SmartInfoCollectionOutcome {
            disks: Err(detail.clone()),
            guidance_candidates: vec![
              ExternalComponentGuidanceCandidate::smartctl_storage_health(
                all_storage_health_signal_names(),
                None,
                detail,
              ),
            ],
          }
        }
      }
    }
  }
}

fn linux_device_candidates() -> Vec<SmartctlDevice> {
  let mut devices = fs::read_dir("/dev")
    .map(|entries| {
      entries
        .flatten()
        .filter_map(|entry| {
          let name = entry.file_name();
          let name = name.to_string_lossy();
          let device_type = linux_smartctl_device_type(&name)?;
          Some(SmartctlDevice::new(
            format!("/dev/{name}"),
            device_type.map(ToOwned::to_owned),
          ))
        })
        .collect::<Vec<_>>()
    })
    .unwrap_or_default();

  devices.sort_by(|a, b| a.name.cmp(&b.name));
  devices.dedup_by(|a, b| a.name == b.name);
  devices
}

fn linux_smartctl_device_type(device_name: &str) -> Option<Option<&'static str>> {
  if is_ata_block_device(device_name) {
    Some(None)
  } else if is_nvme_namespace(device_name) {
    Some(Some("nvme"))
  } else {
    None
  }
}

fn is_ata_block_device(device_name: &str) -> bool {
  let bytes = device_name.as_bytes();
  (bytes.len() == 3 && device_name.starts_with("sd") && bytes[2].is_ascii_lowercase())
    || (bytes.len() == 3
      && device_name.starts_with("hd")
      && bytes[2].is_ascii_lowercase())
    || (bytes.len() == 7
      && device_name.starts_with("mmcblk")
      && bytes[6].is_ascii_digit())
}

fn is_nvme_namespace(device_name: &str) -> bool {
  let Some(rest) = device_name.strip_prefix("nvme") else {
    return false;
  };
  let Some((controller, namespace)) = rest.split_once('n') else {
    return false;
  };

  !controller.is_empty()
    && !namespace.is_empty()
    && controller.chars().all(|c| c.is_ascii_digit())
    && namespace.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn detects_linux_block_devices() {
    assert!(is_ata_block_device("sda"));
    assert!(is_ata_block_device("hdb"));
    assert!(is_ata_block_device("mmcblk0"));
    assert!(!is_ata_block_device("sda1"));
    assert!(!is_ata_block_device("loop0"));
  }

  #[test]
  fn detects_nvme_namespaces() {
    assert!(is_nvme_namespace("nvme0n1"));
    assert!(is_nvme_namespace("nvme12n3"));
    assert!(!is_nvme_namespace("nvme0"));
    assert!(!is_nvme_namespace("nvme0n1p1"));
  }
}
