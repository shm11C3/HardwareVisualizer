#[cfg(test)]
mod tests {
  use crate::utils::color::hex_to_rgb;

  #[test]
  fn test_valid_hex() {
    assert_eq!(hex_to_rgb("#FFFFFF").unwrap(), [255, 255, 255]);
    assert_eq!(hex_to_rgb("#000000").unwrap(), [0, 0, 0]);
    assert_eq!(hex_to_rgb("#123ABC").unwrap(), [18, 58, 188]);
    assert_eq!(hex_to_rgb("#abcdef").unwrap(), [171, 205, 239]);
  }

  #[test]
  fn test_invalid_format() {
    assert!(hex_to_rgb("123456").is_err());
    assert!(hex_to_rgb("#12345").is_err());
    assert!(hex_to_rgb("#1234567").is_err());
    assert!(hex_to_rgb("123ABC").is_err());
  }

  #[test]
  fn test_invalid_characters() {
    assert!(hex_to_rgb("#GGGGGG").is_err());
    assert!(hex_to_rgb("#ZZZZZZ").is_err());
    assert!(hex_to_rgb("#12345G").is_err());
  }
}
