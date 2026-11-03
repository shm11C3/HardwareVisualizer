use serde::{Deserialize, Serialize};

use crate::models::hardware::SuperIoChipIdDiagnostics;
use crate::models::{MotherboardSensorCollection, SensorSupport, TemperatureSample};

pub const VERSION: u32 = 1;
pub const MAX_FRAME: usize = 64 * 1024;
pub const TEMPERATURE: u8 = 1;
pub const POWER: u8 = 2;
pub const MOTHERBOARD: u8 = 4;
pub const ALL: u8 = TEMPERATURE | POWER | MOTHERBOARD;
pub const FRESHNESS_MS: u64 = 5_000;

#[derive(Debug, Clone)]
pub struct ServiceStatus {
  pub connected: bool,
  pub has_readings: bool,
  pub can_configure: bool,
  pub detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
  pub version: u32,
  pub operation: Operation,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Operation {
  Hello,
  Sample { families: u8 },
  Diagnostics,
}

impl Request {
  pub fn validate(&self) -> Result<(), &'static str> {
    if self.version != VERSION {
      return Err("Sensor service protocol mismatch");
    }

    if let Operation::Sample { families } = self.operation
      && (families == 0 || families & !ALL != 0)
    {
      return Err("Invalid sensor family selection");
    }

    Ok(())
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reading<T> {
  pub acquired_at_ms: u64,
  pub sequence: u64,
  pub value: T,
}

impl<T> Reading<T> {
  pub fn is_fresh(&self, now_ms: u64) -> bool {
    self.sequence > 0
      && now_ms >= self.acquired_at_ms
      && now_ms - self.acquired_at_ms <= FRESHNESS_MS
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuPower {
  pub watts: Option<f32>,
  pub support: SensorSupport,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
  pub temperature: Option<Reading<TemperatureSample>>,
  pub power: Option<Reading<CpuPower>>,
  pub motherboard: Option<Reading<MotherboardSensorCollection>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
  pub version: u32,
  pub generation: u64,
  pub payload: Payload,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Payload {
  Hello {
    capabilities: u8,
    service_version: String,
  },
  Snapshot(Box<Snapshot>),
  Diagnostics(SuperIoChipIdDiagnostics),
  Failure(String),
}

#[cfg(test)]
fn decode_request(bytes: &[u8]) -> Result<Request, String> {
  if bytes.len() > MAX_FRAME {
    return Err("Sensor service frame exceeds limit".to_string());
  }

  let request: Request = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
  request.validate().map_err(str::to_string)?;

  Ok(request)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rejects_unversioned_unknown_and_privileged_operations() {
    for bytes in [
      br#"{"version":0,"operation":"Hello"}"#.as_slice(),
      br#"{"version":1,"operation":{"Sample":{"families":128}}}"#,
      br#"{"version":1,"operation":{"Sample":{"families":0}}}"#,
      br#"{"version":1,"operation":"Hello","path":"x.dll"}"#,
      br#"{"version":1,"operation":{"Execute":{"register":0}}}"#,
    ] {
      assert!(decode_request(bytes).is_err());
    }
    assert!(decode_request(&vec![b' '; MAX_FRAME + 1]).is_err());
    assert!(
      decode_request(br#"{"version":1,"operation":{"Sample":{"families":7}}}"#).is_ok()
    );
  }

  #[test]
  fn cached_values_expire_and_future_timestamps_are_rejected() {
    let reading = Reading {
      acquired_at_ms: 100,
      sequence: 1,
      value: (),
    };
    assert!(!reading.is_fresh(99));
    assert!(reading.is_fresh(100 + FRESHNESS_MS));
    assert!(!reading.is_fresh(101 + FRESHNESS_MS));
    assert!(
      !Reading {
        sequence: 0,
        ..reading
      }
      .is_fresh(100)
    );
  }

  #[test]
  fn transport_preserves_source_channel_and_missing_fan_rpm() {
    use crate::models::{FanSpeedStatus, MotherboardFanSpeed, MotherboardSensorSample};

    let sample = MotherboardSensorCollection {
      sample: MotherboardSensorSample {
        temperatures: Vec::new(),
        fan_speeds: vec![MotherboardFanSpeed {
          name: "Fan 2".into(),
          rpm: None,
          status: FanSpeedStatus::Invalid,
          source: "NCT6799D / Super I/O".into(),
        }],
      },
      availability: crate::models::SensorAvailability::Available,
      fan_support: SensorSupport::Supported,
      guidance_candidates: Vec::new(),
    };

    let bytes = serde_json::to_vec(&sample).unwrap();
    let decoded: MotherboardSensorCollection = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded, sample);
    assert_eq!(decoded.sample.fan_speeds[0].rpm, None);
  }
}
