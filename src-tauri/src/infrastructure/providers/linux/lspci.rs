/// Look up a GPU name from `lspci -nn` output by PCI BDF (bus:device.function).
///
/// Returns the full lspci line for the VGA/Display controller at the given slot.
pub fn get_gpu_name_from_lspci_by_bdf(bdf: &str) -> Option<String> {
  use std::process::Command;

  let output = Command::new("lspci").arg("-nn").output().ok()?;

  if !output.status.success() {
    return None;
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  parse_gpu_name_by_bdf(&stdout, bdf)
}

/// Pure parsing function: extract the GPU name line from lspci output matching
/// the given BDF slot. Matches lines containing "VGA" or "Display controller".
#[cfg(any(target_os = "linux", test))]
pub fn parse_gpu_name_by_bdf(lspci_output: &str, bdf: &str) -> Option<String> {
  for line in lspci_output.lines() {
    // BDF must be followed by a space (e.g. "06:00.0 VGA ...")
    if !line.starts_with(bdf) || line.as_bytes().get(bdf.len()) != Some(&b' ') {
      continue;
    }
    if line.contains("VGA") || line.contains("Display controller") {
      return Some(line.trim().to_string());
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  // ── parse_gpu_name_by_bdf ──

  #[test]
  fn parse_gpu_name_by_bdf_matches_exact_slot() {
    let output = "\
06:00.0 VGA compatible controller [0300]: Advanced Micro Devices, Inc. [AMD/ATI] Navi 31 [Radeon RX 7900 XTX] [1002:744c]
0a:00.0 VGA compatible controller [0300]: Advanced Micro Devices, Inc. [AMD/ATI] Renoir [Radeon Vega Series] [1002:1636]";
    assert!(
      parse_gpu_name_by_bdf(output, "06:00.0")
        .unwrap()
        .contains("Navi 31")
    );
    assert!(
      parse_gpu_name_by_bdf(output, "0a:00.0")
        .unwrap()
        .contains("Renoir")
    );
  }

  #[test]
  fn parse_gpu_name_by_bdf_no_match() {
    let output = "06:00.0 VGA compatible controller [0300]: AMD/ATI Navi 31 [1002:744c]";
    assert!(parse_gpu_name_by_bdf(output, "0a:00.0").is_none());
  }

  #[test]
  fn parse_gpu_name_by_bdf_ignores_non_vga() {
    let output = "06:00.1 Audio device [0403]: AMD/ATI Navi 31 HDMI Audio [1002:ab30]";
    assert!(parse_gpu_name_by_bdf(output, "06:00.1").is_none());
  }

  #[test]
  fn parse_gpu_name_by_bdf_display_controller() {
    let output = "06:00.0 Display controller [0380]: Advanced Micro Devices, Inc. [AMD/ATI] Renoir [1002:1636]";
    assert!(parse_gpu_name_by_bdf(output, "06:00.0").is_some());
  }

  #[test]
  fn parse_gpu_name_by_bdf_no_prefix_false_positive() {
    // "06:00.0" should NOT match a line starting with "06:00.00"
    let output = "06:00.00 VGA compatible controller [0300]: Fake GPU [ffff:0000]";
    assert!(parse_gpu_name_by_bdf(output, "06:00.0").is_none());
  }
}
