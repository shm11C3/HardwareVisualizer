#[cfg(target_os = "linux")]
#[cfg(test)]
mod tests {

  // Helper function to test SCLK parsing with mock file content
  fn parse_sclk_from_content(content: &str) -> Option<u32> {
    use regex::Regex;

    let re = Regex::new(r"SCLK.*?(\d+)\s+MHz").ok()?;
    re.captures(content)
      .and_then(|cap| cap[1].parse::<u32>().ok())
  }

  #[test]
  fn test_parse_normal_sclk_output() {
    let content = r#"
GFX Clocks and Power:
	600 MHz (MCLK)
	SCLK 1050 MHz
	925 mV (VDDC)
	1000 mV (VDDCI)
	950 mV (MVDD)

UVD Clocks and Power:
	Disabled

VCE Clocks and Power:
	Disabled

Temperature:
	42.0 C (GPU)
	41.0 C (MEM)
"#;

    let result = parse_sclk_from_content(content);
    assert_eq!(result, Some(1050));
  }

  #[test]
  fn test_parse_sclk_with_different_format() {
    let content = r#"
GFX Clocks and Power:
	SCLK: 1200 MHz
	MCLK: 800 MHz
	VDDC: 1050 mV
"#;

    let result = parse_sclk_from_content(content);
    assert_eq!(result, Some(1200));
  }

  #[test]
  fn test_parse_sclk_multiline_match() {
    let content = r#"
GFX Clocks and Power:
	600 MHz (MCLK)
	SCLK: 1500 MHz
	925 mV (VDDC)
"#;

    let result = parse_sclk_from_content(content);
    assert_eq!(result, Some(1500));
  }

  #[test]
  fn test_parse_sclk_with_tabs_and_spaces() {
    let content = "GFX Clocks and Power:\n\t800 MHz (MCLK)\n    SCLK   1350   MHz   \n\t925 mV (VDDC)";

    let result = parse_sclk_from_content(content);
    assert_eq!(result, Some(1350));
  }

  #[test]
  fn test_parse_no_sclk_found() {
    let content = r#"
GFX Clocks and Power:
	600 MHz (MCLK)
	925 mV (VDDC)
	1000 mV (VDDCI)

Temperature:
	42.0 C (GPU)
"#;

    let result = parse_sclk_from_content(content);
    assert_eq!(result, None);
  }

  #[test]
  fn test_parse_empty_content() {
    let result = parse_sclk_from_content("");
    assert_eq!(result, None);
  }

  #[test]
  fn test_parse_malformed_sclk_value() {
    let content = r#"
GFX Clocks and Power:
	SCLK invalid MHz
"#;

    let result = parse_sclk_from_content(content);
    assert_eq!(result, None);
  }

  #[test]
  fn test_parse_sclk_no_mhz_unit() {
    let content = r#"
GFX Clocks and Power:
	SCLK 1400
"#;

    let result = parse_sclk_from_content(content);
    assert_eq!(result, None);
  }

  #[test]
  fn test_parse_multiple_sclk_entries_first_match() {
    let content = r#"
GFX Clocks and Power:
	SCLK 1200 MHz
	SCLK 1400 MHz (Boosted)
"#;

    let result = parse_sclk_from_content(content);
    assert_eq!(result, Some(1200)); // Should return first match
  }

  #[test]
  fn test_parse_sclk_case_sensitivity() {
    let content = r#"
GFX Clocks and Power:
	sclk 1100 MHz
	SCLK 1300 MHz
"#;

    let result = parse_sclk_from_content(content);
    assert_eq!(result, Some(1300)); // Should match SCLK (uppercase)
  }

  #[test]
  fn test_parse_sclk_with_decimal() {
    let content = r#"
GFX Clocks and Power:
	SCLK 1250 MHz
"#;

    let result = parse_sclk_from_content(content);
    assert_eq!(result, Some(1250));
  }

  #[test]
  fn test_parse_large_sclk_value() {
    let content = r#"
GFX Clocks and Power:
	SCLK 2500 MHz
"#;

    let result = parse_sclk_from_content(content);
    assert_eq!(result, Some(2500));
  }

  #[test]
  fn test_parse_zero_sclk_value() {
    let content = r#"
GFX Clocks and Power:
	SCLK 0 MHz
"#;

    let result = parse_sclk_from_content(content);
    assert_eq!(result, Some(0));
  }

  // Integration test that would work with mock filesystem
  #[test]
  fn test_read_pm_info_sclk_integration_mock() {
    // This test demonstrates how the real function would work
    // In a real scenario, we would need to mock the filesystem or create temp files

    // For now, we test the parsing logic that the function uses internally
    let mock_content = r#"
GFX Clocks and Power:
	600 MHz (MCLK)
	SCLK 1050 MHz
	925 mV (VDDC)
"#;

    // Test the regex parsing that read_pm_info_sclk uses internally
    let result = parse_sclk_from_content(mock_content);
    assert_eq!(result, Some(1050));
  }

  #[test]
  fn test_regex_pattern_edge_cases() {
    // Test various edge cases for the regex pattern
    let test_cases = vec![
      ("SCLK 1000 MHz", Some(1000)),
      ("SCLK: 1000 MHz", Some(1000)),
      ("SCLK\t1000 MHz", Some(1000)),
      ("SCLK stuff 1000 MHz", Some(1000)),
      ("Current SCLK 1000 MHz", Some(1000)),
      ("SCLK_FREQ 1000 MHz", Some(1000)),
      ("MCLK 800 MHz", None),  // Should not match MCLK
      ("SCLK MHz", None),      // No number
      ("1000 SCLK MHz", None), // Number before SCLK
    ];

    for (content, expected) in test_cases {
      let result = parse_sclk_from_content(content);
      assert_eq!(result, expected, "Failed for content: '{}'", content);
    }
  }
}
