pub fn get_gpu_name_from_lspci_by_vendor_id(vendor_id: &str) -> Option<String> {
  use std::process::Command;

  let output = Command::new("lspci").arg("-nn").output().ok()?;

  if !output.status.success() {
    return None;
  }

  let stdout = String::from_utf8_lossy(&output.stdout);

  for line in stdout.lines() {
    if line.contains("VGA") && line.contains(vendor_id) {
      // Example: "03:00.0 VGA compatible controller [0300]: AMD/ATI Renoir [1002:1636]"
      return Some(line.trim().to_string());
    }
  }

  None
}
