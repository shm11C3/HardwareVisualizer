use std::sync::Mutex;
use std::time::Duration;

use tokio::net::windows::named_pipe::NamedPipeClient;
use tokio::sync::mpsc;
use tokio::time::Instant;

use super::{
  security,
  server::{read_frame, write_frame},
  uptime_ms,
};
use crate::models::{
  MotherboardSensorCollection, TemperatureSample, hardware::SuperIoChipIdDiagnostics,
};
use crate::sensor_service::protocol::*;

enum State {
  Connecting,
  Connected(Box<Snapshot>),
  Unavailable(String),
}

static CLIENT: Mutex<State> = Mutex::new(State::Connecting);

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

fn sample<T: Clone>(
  select: impl FnOnce(&Snapshot) -> &Reading<T>,
) -> Result<T, SampleError> {
  let state = CLIENT
    .lock()
    .map_err(|_| SampleError::Unavailable("Sensor service client unavailable".into()))?;

  match &*state {
    State::Connecting => Err(SampleError::Connecting),
    State::Unavailable(error) => Err(SampleError::Unavailable(error.clone())),
    State::Connected(snapshot) => {
      let reading = select(snapshot);

      if reading.is_fresh(uptime_ms()) {
        Ok(reading.value.clone())
      } else {
        Err(SampleError::Connecting)
      }
    }
  }
}

pub fn temperature() -> Result<TemperatureSample, SampleError> {
  sample(|v| &v.temperature)
}

pub fn power() -> Result<CpuPower, SampleError> {
  sample(|v| &v.power)
}

pub fn motherboard() -> Result<MotherboardSensorCollection, SampleError> {
  sample(|v| &v.motherboard)
}

pub async fn status() -> bool {
  handshake().await.is_ok()
}

pub async fn diagnostics() -> SuperIoChipIdDiagnostics {
  let result = async {
    let mut connection = handshake().await?;
    match connection.request(Operation::Diagnostics).await? {
      Payload::Diagnostics(value) => Ok(value),
      _ => Err("Unexpected sensor diagnostics response".into()),
    }
  }
  .await;

  result.unwrap_or_else(|error| SuperIoChipIdDiagnostics {
    platform_supported: true,
    pawnio: None,
    slots: Vec::new(),
    error: Some(error),
  })
}

pub(super) struct Connection {
  pipe: NamedPipeClient,
  generation: u64,
}

impl Connection {
  async fn request(&mut self, operation: Operation) -> Result<Payload, String> {
    let reply = exchange(&mut self.pipe, operation).await?;

    if reply.generation != self.generation {
      return Err("Sensor service generation changed".into());
    }

    Ok(reply.payload)
  }
}

pub(super) async fn handshake() -> Result<Connection, String> {
  let mut pipe = security::connect().await?;
  let reply = exchange(&mut pipe, Operation::Hello).await?;

  if reply.generation == 0 || !matches!(reply.payload, Payload::Hello { .. }) {
    return Err("Invalid sensor service handshake".into());
  }

  Ok(Connection {
    pipe,
    generation: reply.generation,
  })
}

async fn exchange(
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

/// Only collector ticks trigger sampling. The one-slot channel coalesces
/// ticks while a request is in flight without delaying other monitor data.
pub(crate) async fn collect(mut ticks: mpsc::Receiver<()>) {
  let mut connection = None;
  let mut retry_at = Instant::now();
  let mut backoff = Duration::from_secs(1);

  while ticks.recv().await.is_some() && !ticks.is_closed() {
    if Instant::now() < retry_at {
      continue;
    }

    let result = async {
      let pipe = match &mut connection {
        Some(pipe) => pipe,
        None => connection.insert(handshake().await?),
      };
      match pipe.request(Operation::Sample).await? {
        Payload::Snapshot(snapshot) => Ok(snapshot),
        _ => Err("Unexpected sensor response".to_string()),
      }
    }
    .await;

    let mut state = CLIENT.lock().unwrap_or_else(|error| error.into_inner());
    match result {
      Ok(snapshot) => {
        *state = State::Connected(snapshot);
        backoff = Duration::from_secs(1);
      }
      Err(error) => {
        connection = None;
        retry_at = Instant::now() + backoff;
        backoff = (backoff * 2).min(Duration::from_secs(30));

        if !matches!(&*state, State::Unavailable(previous) if previous == &error) {
          crate::log_warn!(
            "sensor_service_unavailable",
            "sensor_service::client",
            Some(error.clone())
          );
        }

        *state = State::Unavailable(error);
      }
    }
  }

  *CLIENT.lock().unwrap_or_else(|error| error.into_inner()) = State::Connecting;
}
