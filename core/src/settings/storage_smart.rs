use serde::{Deserialize, Serialize};

pub const DEFAULT_STORAGE_SMART_RETENTION_DAYS: u32 = 365 * 3;
pub const STORAGE_SMART_IDENTITY_HASH_KEY_BYTES: usize = 32;

const STORAGE_SMART_IDENTITY_HASH_KEY_VERSION: &str = "v1";

/// Core-owned SMART daily snapshot settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StorageSmartSettings {
  pub enabled: bool,
  pub retention_days: u32,
}

impl Default for StorageSmartSettings {
  fn default() -> Self {
    Self {
      enabled: true,
      retention_days: DEFAULT_STORAGE_SMART_RETENTION_DAYS,
    }
  }
}

/// Core-owned local secret used to derive stable storage identifiers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StorageSmartIdentitySettings {
  pub hash_key: String,
}

impl StorageSmartIdentitySettings {
  pub fn ensure_hash_key(&mut self) -> Result<bool, String> {
    if !self.hash_key.trim().is_empty() {
      self.hash_key_bytes()?;
      return Ok(false);
    }

    let mut key = [0u8; STORAGE_SMART_IDENTITY_HASH_KEY_BYTES];
    getrandom::fill(&mut key)
      .map_err(|e| format!("Failed to generate storage SMART identity key: {e}"))?;
    self.hash_key = format!(
      "{}:{}",
      STORAGE_SMART_IDENTITY_HASH_KEY_VERSION,
      hex_encode(&key)
    );
    Ok(true)
  }

  pub fn hash_key_bytes(
    &self,
  ) -> Result<[u8; STORAGE_SMART_IDENTITY_HASH_KEY_BYTES], String> {
    let Some(encoded) = self
      .hash_key
      .trim()
      .strip_prefix(STORAGE_SMART_IDENTITY_HASH_KEY_VERSION)
      .and_then(|value| value.strip_prefix(':'))
    else {
      return Err(
        "Storage SMART identity hash key has an unsupported version".to_string(),
      );
    };

    if encoded.len() != STORAGE_SMART_IDENTITY_HASH_KEY_BYTES * 2 {
      return Err("Storage SMART identity hash key has an invalid length".to_string());
    }

    let mut key = [0u8; STORAGE_SMART_IDENTITY_HASH_KEY_BYTES];
    for (index, chunk) in encoded.as_bytes().chunks_exact(2).enumerate() {
      key[index] = decode_hex_pair(chunk).ok_or_else(|| {
        "Storage SMART identity hash key contains invalid hex".to_string()
      })?;
    }
    Ok(key)
  }
}

fn hex_encode(bytes: &[u8]) -> String {
  const HEX: &[u8; 16] = b"0123456789abcdef";
  let mut output = String::with_capacity(bytes.len() * 2);
  for byte in bytes {
    output.push(HEX[(byte >> 4) as usize] as char);
    output.push(HEX[(byte & 0x0f) as usize] as char);
  }
  output
}

fn decode_hex_pair(bytes: &[u8]) -> Option<u8> {
  if bytes.len() != 2 {
    return None;
  }
  Some((decode_hex_nibble(bytes[0])? << 4) | decode_hex_nibble(bytes[1])?)
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
  match byte {
    b'0'..=b'9' => Some(byte - b'0'),
    b'a'..=b'f' => Some(byte - b'a' + 10),
    b'A'..=b'F' => Some(byte - b'A' + 10),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn defaults_to_three_year_retention() {
    let s = StorageSmartSettings::default();
    assert!(s.enabled);
    assert_eq!(s.retention_days, DEFAULT_STORAGE_SMART_RETENTION_DAYS);
    assert_eq!(s.retention_days, 1_095);
  }

  #[test]
  fn missing_fields_fall_back_to_defaults() {
    let s: StorageSmartSettings = serde_json::from_str(r#"{"enabled": false}"#).unwrap();
    assert!(!s.enabled);
    assert_eq!(s.retention_days, DEFAULT_STORAGE_SMART_RETENTION_DAYS);
  }

  #[test]
  fn serializes_in_camel_case() {
    let s = StorageSmartSettings::default();
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("\"retentionDays\""));
  }

  #[test]
  fn identity_key_is_generated_and_versioned() {
    let mut s = StorageSmartIdentitySettings::default();

    assert!(s.ensure_hash_key().unwrap());
    assert!(s.hash_key.starts_with("v1:"));
    assert_eq!(
      s.hash_key_bytes().unwrap().len(),
      STORAGE_SMART_IDENTITY_HASH_KEY_BYTES
    );
    assert!(!s.ensure_hash_key().unwrap());
  }

  #[test]
  fn invalid_identity_key_is_rejected() {
    let s = StorageSmartIdentitySettings {
      hash_key: "v1:not-hex".to_string(),
    };

    assert!(s.hash_key_bytes().is_err());
  }
}
