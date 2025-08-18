pub fn read_pm_info_sclk(card_id: u8) -> Option<u32> {
  use regex::Regex;

  let path = format!("/sys/kernel/debug/dri/{card_id}/amdgpu_pm_info");
  let content = std::fs::read_to_string(path).ok()?;
  let re = Regex::new(r"SCLK.*?(\d+)\s+MHz").ok()?;
  re.captures(&content)
    .and_then(|cap| cap[1].parse::<u32>().ok())
}
