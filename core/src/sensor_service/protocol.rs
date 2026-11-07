use serde::{Deserialize, Serialize};

use crate::models::hardware::SuperIoChipIdDiagnostics;
use crate::models::{MotherboardSensorCollection, SensorSupport, TemperatureSample};

pub const VERSION: u32 = 2;
pub const MAX_FRAME: usize = 64 * 1024;
pub const FRESHNESS_MS: u64 = 5_000;

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
  Sample,
  Diagnostics,
}

impl Request {
  pub fn validate(&self) -> Result<(), &'static str> {
    if self.version != VERSION {
      return Err("Sensor service protocol mismatch");
    }

    Ok(())
  }
}

/// A provider result timed on the monotonic clock shared by both processes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reading<T> {
  pub acquired_at_ms: u64,
  pub value: T,
}

impl<T> Reading<T> {
  pub fn is_fresh(&self, now_ms: u64) -> bool {
    now_ms >= self.acquired_at_ms && now_ms - self.acquired_at_ms <= FRESHNESS_MS
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuPower {
  pub watts: Option<f32>,
  pub support: SensorSupport,
}

/// A complete batch whose provider results can be unavailable independently.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
  pub temperature: Reading<TemperatureSample>,
  pub power: Reading<CpuPower>,
  pub motherboard: Reading<MotherboardSensorCollection>,
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
  Hello { service_version: String },
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
      br#"{"operation":"Hello"}"#.as_slice(),
      br#"{"version":1,"operation":"Hello"}"#,
      br#"{"version":2,"operation":{"Sample":{"families":7}}}"#,
      br#"{"version":2,"operation":"Sample","path":"x.dll"}"#,
      br#"{"version":2,"operation":{"Execute":{"register":0}}}"#,
    ] {
      assert!(decode_request(bytes).is_err());
    }
    assert!(decode_request(&vec![b' '; MAX_FRAME + 1]).is_err());
    let sample = decode_request(br#"{"version":2,"operation":"Sample"}"#).unwrap();
    assert!(matches!(sample.operation, Operation::Sample));
  }

  #[test]
  fn cached_values_expire_and_future_timestamps_are_rejected() {
    let reading = Reading {
      acquired_at_ms: 100,
      value: (),
    };
    assert!(!reading.is_fresh(99));
    assert!(reading.is_fresh(100 + FRESHNESS_MS));
    assert!(!reading.is_fresh(101 + FRESHNESS_MS));
  }

  #[test]
  fn snapshot_preserves_partial_availability_and_channel_identity() {
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

    let snapshot = Snapshot {
      temperature: Reading {
        acquired_at_ms: 100,
        value: TemperatureSample::unavailable("no temperature provider"),
      },
      power: Reading {
        acquired_at_ms: 100,
        value: CpuPower {
          watts: None,
          support: SensorSupport::Unknown,
        },
      },
      motherboard: Reading {
        acquired_at_ms: 120,
        value: sample.clone(),
      },
    };

    let bytes = serde_json::to_vec(&snapshot).unwrap();
    let decoded: Snapshot = serde_json::from_slice(&bytes).unwrap();
    assert!(decoded.temperature.value.cpu_temperature.is_none());
    assert!(decoded.power.value.watts.is_none());
    assert_eq!(decoded.motherboard.acquired_at_ms, 120);
    assert_eq!(decoded.motherboard.value, sample);
    assert_eq!(decoded.motherboard.value.sample.fan_speeds[0].rpm, None);
  }
}
