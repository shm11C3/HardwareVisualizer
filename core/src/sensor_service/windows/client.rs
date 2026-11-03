use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use tokio::net::windows::named_pipe::NamedPipeClient;

use super::{
  security,
  server::{read_frame, write_frame},
  uptime_ms,
};
use crate::models::{
  MotherboardSensorCollection, TemperatureSample, hardware::SuperIoChipIdDiagnostics,
};
use crate::sensor_service::protocol::*;

#[derive(Default)]
struct State {
  demand: [u64; 3],
  snapshot: Snapshot,
  error: Option<String>,
}

static CLIENT: OnceLock<Arc<Mutex<State>>> = OnceLock::new();

#[derive(Debug)]
pub enum SampleError {
  Connecting,
  Unavailable(String),
}

impl std::fmt::Display for SampleError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Connecting => f.write_str("Connecting to the Windows sensor service"),
      Self::Unavailable(reason) => f.write_str(reason),
    }
  }
}

fn client() -> &'static Arc<Mutex<State>> {
  CLIENT.get_or_init(|| {
    let state = Arc::new(Mutex::new(State::default()));
    let worker = Arc::clone(&state);

    if let Err(error) = std::thread::Builder::new()
      .name("sensor-service-client".into())
      .spawn(move || {
        match tokio::runtime::Builder::new_current_thread()
          .enable_all()
          .build()
        {
          Ok(runtime) => runtime.block_on(poll(worker)),
          Err(error) => {
            worker.lock().unwrap_or_else(|p| p.into_inner()).error =
              Some(error.to_string());
          }
        }
      })
    {
      state.lock().unwrap_or_else(|p| p.into_inner()).error = Some(error.to_string());
    }
    state
  })
}

fn sample<T: Clone>(
  family: u8,
  select: impl FnOnce(&Snapshot) -> &Option<Reading<T>>,
) -> Result<T, SampleError> {
  let now = uptime_ms();
  let mut state = client()
    .lock()
    .map_err(|_| SampleError::Unavailable("Sensor service client unavailable".into()))?;
  state.demand[family.trailing_zeros() as usize] = now;

  select(&state.snapshot)
    .as_ref()
    .filter(|v| v.is_fresh(now))
    .map(|v| v.value.clone())
    .ok_or_else(|| {
      state
        .error
        .clone()
        .map(SampleError::Unavailable)
        .unwrap_or(SampleError::Connecting)
    })
}

pub fn temperature() -> Result<TemperatureSample, SampleError> {
  sample(TEMPERATURE, |v| &v.temperature)
}

pub fn power() -> Result<CpuPower, SampleError> {
  sample(POWER, |v| &v.power)
}

pub fn motherboard() -> Result<MotherboardSensorCollection, SampleError> {
  sample(MOTHERBOARD, |v| &v.motherboard)
}

pub fn diagnostics() -> SuperIoChipIdDiagnostics {
  let result = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .map_err(|e| e.to_string())
    .and_then(|runtime| {
      runtime.block_on(async {
        let (mut pipe, generation) = handshake().await?;
        let reply = exchange(&mut pipe, Operation::Diagnostics).await?;

        if reply.generation != generation {
          return Err("Sensor service restarted".into());
        }

        match reply.payload {
          Payload::Diagnostics(value) => Ok(value),
          _ => Err("Unexpected sensor diagnostics response".into()),
        }
      })
    });
  result.unwrap_or_else(|error| SuperIoChipIdDiagnostics {
    platform_supported: true,
    pawnio: None,
    slots: Vec::new(),
    error: Some(error),
  })
}

pub(super) async fn handshake() -> Result<(NamedPipeClient, u64), String> {
  let mut pipe = security::connect()?;
  let reply = exchange(&mut pipe, Operation::Hello).await?;

  if reply.generation == 0
    || !matches!(
      reply.payload,
      Payload::Hello {
        capabilities: ALL,
        ..
      }
    )
  {
    return Err("Incompatible sensor service capabilities".into());
  }

  Ok((pipe, reply.generation))
}

pub(super) async fn exchange(
  pipe: &mut NamedPipeClient,
  operation: Operation,
) -> Result<Reply, String> {
  tokio::time::timeout(Duration::from_secs(2), async {
    write_frame(
      pipe,
      &Request {
        version: VERSION,
        operation,
      },
    )
    .await?;

    let reply: Reply = read_frame(pipe).await?;

    if reply.version != VERSION {
      return Err("Sensor service protocol mismatch".into());
    }

    if let Payload::Failure(error) = &reply.payload {
      return Err(error.clone());
    }

    Ok(reply)
  })
  .await
  .map_err(|_| "Sensor service request timed out".to_string())?
}

async fn poll(state: Arc<Mutex<State>>) {
  let mut connection = None;
  let mut backoff = 1u64;
  let mut observed = Snapshot::default();
  let mut observed_generation = 0;
  let mut awaiting_fresh = false;

  loop {
    let mask = {
      let state = state.lock().unwrap_or_else(|p| p.into_inner());
      demand_mask(state.demand, uptime_ms())
    };

    if mask == 0 {
      connection = None;
      tokio::time::sleep(Duration::from_millis(250)).await;
      continue;
    }

    let result = async {
      if connection.is_none() {
        state
          .lock()
          .map_err(|_| "Sensor service cache unavailable")?
          .snapshot = Snapshot::default();

        let established = handshake().await?;

        if established.1 != observed_generation {
          observed = Snapshot::default();
          observed_generation = established.1;
        }

        awaiting_fresh = true;
        connection = Some(established);
      }

      let (pipe, generation) = connection.as_mut().unwrap();
      let reply = exchange(pipe, Operation::Sample { families: mask }).await?;

      if reply.generation != *generation {
        return Err("Sensor service generation changed".to_string());
      }

      let Payload::Snapshot(snapshot) = reply.payload else {
        return Err("Unexpected sensor response".into());
      };
      validate_snapshot(&snapshot, &observed, mask, uptime_ms())?;

      if awaiting_fresh && !new_acquisitions(&snapshot, &observed, mask) {
        // Keep the authenticated connection, but wait for a new acquisition
        // instead of replaying the last observation after a disconnect.
        return Ok(());
      }

      observed = (*snapshot).clone();
      awaiting_fresh = false;

      let mut state = state
        .lock()
        .map_err(|_| "Sensor service cache unavailable")?;
      state.snapshot = *snapshot;
      state.error = None;
      Ok::<_, String>(())
    }
    .await;

    match result {
      Ok(()) => {
        backoff = 1;
        tokio::time::sleep(Duration::from_secs(1)).await;
      }
      Err(error) => {
        connection = None;
        {
          let mut state = state.lock().unwrap_or_else(|p| p.into_inner());
          state.snapshot = Snapshot::default();

          if state.error.as_ref() != Some(&error) {
            crate::log_warn!(
              "sensor_service_unavailable",
              "sensor_service::client",
              Some(error.clone())
            );
          }

          state.error = Some(error);
        }
        tokio::time::sleep(Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(30);
      }
    }
  }
}

fn new_acquisitions(value: &Snapshot, previous: &Snapshot, mask: u8) -> bool {
  fn newer<T>(next: &Option<Reading<T>>, old: &Option<Reading<T>>) -> bool {
    next
      .as_ref()
      .is_some_and(|next| old.as_ref().is_none_or(|old| next.sequence > old.sequence))
  }

  (mask & TEMPERATURE == 0 || newer(&value.temperature, &previous.temperature))
    && (mask & POWER == 0 || newer(&value.power, &previous.power))
    && (mask & MOTHERBOARD == 0 || newer(&value.motherboard, &previous.motherboard))
}

fn demand_mask(demand: [u64; 3], now: u64) -> u8 {
  demand.iter().enumerate().fold(0, |mask, (i, at)| {
    if *at != 0 && now.saturating_sub(*at) <= 5_000 {
      mask | (1 << i)
    } else {
      mask
    }
  })
}

fn validate_snapshot(
  value: &Snapshot,
  previous: &Snapshot,
  mask: u8,
  now: u64,
) -> Result<(), String> {
  fn check<T>(
    next: &Option<Reading<T>>,
    previous: &Option<Reading<T>>,
    required: bool,
    now: u64,
  ) -> bool {
    if !required {
      return next.is_none();
    }
    next.as_ref().is_some_and(|next| {
      next.is_fresh(now)
        && previous.as_ref().is_none_or(|old| {
          next.sequence > old.sequence
            || (next.sequence == old.sequence
              && next.acquired_at_ms == old.acquired_at_ms)
        })
    })
  }

  if check(
    &value.temperature,
    &previous.temperature,
    mask & TEMPERATURE != 0,
    now,
  ) && check(&value.power, &previous.power, mask & POWER != 0, now)
    && check(
      &value.motherboard,
      &previous.motherboard,
      mask & MOTHERBOARD != 0,
      now,
    )
  {
    Ok(())
  } else {
    Err("Sensor service sent stale or out-of-order readings".into())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_crashed_app_cannot_keep_demand_alive() {
    assert_eq!(demand_mask([100, 0, 200], 500), TEMPERATURE | MOTHERBOARD);
    assert_eq!(demand_mask([100, 0, 200], 5_201), 0);
  }

  #[test]
  fn replay_cannot_refresh_acquisition_time() {
    let reading = Reading {
      acquired_at_ms: 100,
      sequence: 2,
      value: TemperatureSample::default(),
    };

    let previous = Snapshot {
      temperature: Some(reading.clone()),
      ..Snapshot::default()
    };

    let mut next = previous.clone();

    assert!(!new_acquisitions(&next, &previous, TEMPERATURE));
    assert!(validate_snapshot(&next, &previous, TEMPERATURE, 200).is_ok());
    next.temperature.as_mut().unwrap().acquired_at_ms = 200;
    assert!(validate_snapshot(&next, &previous, TEMPERATURE, 200).is_err());

    next.temperature.as_mut().unwrap().sequence = 3;
    assert!(new_acquisitions(&next, &previous, TEMPERATURE));
    assert!(validate_snapshot(&next, &previous, TEMPERATURE, 200).is_ok());

    next.temperature = Some(Reading {
      sequence: 1,
      ..reading
    });
    assert!(validate_snapshot(&next, &previous, TEMPERATURE, 200).is_err());
  }
}
