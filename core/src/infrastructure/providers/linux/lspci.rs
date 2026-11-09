/// Run `lspci -D -nn` and return the raw stdout with every PCI domain shown.
pub fn get_lspci_nn_output() -> Option<String> {
  use std::process::Command;

  let output = Command::new("lspci").args(["-D", "-nn"]).output().ok()?;

  if !output.status.success() {
    return None;
  }

  Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Look up a GPU name by its full PCI address.
///
/// Returns the full lspci line for the VGA/Display controller at the given slot.
/// Note: forks `lspci` on each call. For hot loops, prefer calling
/// [`get_lspci_nn_output`] once and then [`parse_gpu_name_by_bdf`] per card.
pub fn get_gpu_name_from_lspci_by_bdf(bdf: &str) -> Option<String> {
  let stdout = get_lspci_nn_output()?;
  parse_gpu_name_by_bdf(&stdout, bdf)
}

/// Extract the GPU name line matching the given PCI domain, bus, device and function.
#[cfg(any(target_os = "linux", test))]
pub fn parse_gpu_name_by_bdf(lspci_output: &str, bdf: &str) -> Option<String> {
  for line in lspci_output.lines() {
    // The address must be followed by a space (e.g. "0000:06:00.0 VGA ...").
    if !line.starts_with(bdf) || line.as_bytes().get(bdf.len()) != Some(&b' ') {
      continue;
    }
    if line.contains("VGA")
      || line.contains("Display controller")
      || line.contains("3D controller")
    {
      let name = line.trim_end();
      // Preserve the display name used when lspci omits the default domain.
      return Some(name.strip_prefix("0000:").unwrap_or(name).to_owned());
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
0000:06:00.0 VGA compatible controller [0300]: AMD/ATI Navi 31 [1002:744c]
0001:06:00.0 3D controller [0302]: NVIDIA AD107 [10de:2882]";
    assert_eq!(
      parse_gpu_name_by_bdf(output, "0000:06:00.0").as_deref(),
      Some("06:00.0 VGA compatible controller [0300]: AMD/ATI Navi 31 [1002:744c]")
    );
    assert_eq!(
      parse_gpu_name_by_bdf(output, "0001:06:00.0").as_deref(),
      Some("0001:06:00.0 3D controller [0302]: NVIDIA AD107 [10de:2882]")
    );
    assert!(parse_gpu_name_by_bdf(output, "06:00.0").is_none());
  }

  #[test]
  fn parse_gpu_name_by_bdf_no_match() {
    let output =
      "0000:06:00.0 VGA compatible controller [0300]: AMD/ATI Navi 31 [1002:744c]";
    assert!(parse_gpu_name_by_bdf(output, "0001:06:00.0").is_none());
  }

  #[test]
  fn parse_gpu_name_by_bdf_ignores_non_vga() {
    let output =
      "0000:06:00.1 Audio device [0403]: AMD/ATI Navi 31 HDMI Audio [1002:ab30]";
    assert!(parse_gpu_name_by_bdf(output, "0000:06:00.1").is_none());
  }

  #[test]
  fn parse_gpu_name_by_bdf_display_controller() {
    let output = "0000:06:00.0 Display controller [0380]: Advanced Micro Devices, Inc. [AMD/ATI] Renoir [1002:1636]";
    assert!(parse_gpu_name_by_bdf(output, "0000:06:00.0").is_some());
  }

  #[test]
  fn parse_gpu_name_by_bdf_no_prefix_false_positive() {
    let output = "0000:06:00.00 VGA compatible controller [0300]: Fake GPU [ffff:0000]";
    assert!(parse_gpu_name_by_bdf(output, "0000:06:00.0").is_none());
  }
}
