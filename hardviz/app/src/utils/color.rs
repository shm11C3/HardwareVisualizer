pub fn hex_to_rgb(hex: &str) -> Result<[u8; 3], &str> {
  if hex.len() != 7 || !hex.starts_with('#') {
    return Err("Invalid hex format");
  }
  let r = u8::from_str_radix(&hex[1..3], 16).map_err(|_| "Invalid hex value")?;
  let g = u8::from_str_radix(&hex[3..5], 16).map_err(|_| "Invalid hex value")?;
  let b = u8::from_str_radix(&hex[5..7], 16).map_err(|_| "Invalid hex value")?;
  Ok([r, g, b])
}

#[cfg(test)]
mod tests {
  use super::*;

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
