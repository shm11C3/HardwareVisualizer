use std::fmt;

/// POJO mirror of `src-tauri/src/enums/hardware.rs::DiskKind`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DiskKind {
  Hdd,
  Ssd,
  Unknown,
}

impl From<sysinfo::DiskKind> for DiskKind {
  fn from(kind: sysinfo::DiskKind) -> Self {
    match kind {
      sysinfo::DiskKind::HDD => DiskKind::Hdd,
      sysinfo::DiskKind::SSD => DiskKind::Ssd,
      _ => DiskKind::Unknown,
    }
  }
}

impl fmt::Display for DiskKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(match *self {
      DiskKind::Hdd => "HDD",
      DiskKind::Ssd => "SSD",
      _ => "Other",
    })
  }
}
