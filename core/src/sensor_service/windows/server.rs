use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::windows::named_pipe::NamedPipeServer;
use tokio::sync::Semaphore;

use super::{providers::Providers, security};
use crate::sensor_service::protocol::*;

pub(super) static INVALIDATE: AtomicBool = AtomicBool::new(false);

pub(super) async fn read_frame<T: DeserializeOwned>(
  stream: &mut (impl AsyncRead + Unpin),
) -> Result<T, String> {
  let length = stream.read_u32_le().await.map_err(|e| e.to_string())? as usize;

  if length == 0 || length > MAX_FRAME {
    return Err("Invalid sensor service frame length".into());
  }

  let mut bytes = vec![0; length];
  stream
    .read_exact(&mut bytes)
    .await
    .map_err(|e| e.to_string())?;

  serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub(super) async fn write_frame<T: Serialize>(
  stream: &mut (impl AsyncWrite + Unpin),
  value: &T,
) -> Result<(), String> {
  let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;

  if bytes.is_empty() || bytes.len() > MAX_FRAME {
    return Err("Sensor service response exceeds frame limit".into());
  }

  stream
    .write_u32_le(bytes.len() as u32)
    .await
    .map_err(|e| e.to_string())?;
  stream.write_all(&bytes).await.map_err(|e| e.to_string())
}

pub(super) async fn serve(stop: &'static AtomicBool) -> Result<(), String> {
  let mut random = [0u8; 8];
  getrandom::fill(&mut random).map_err(|e| e.to_string())?;

  let generation = u64::from_le_bytes(random).max(1);
  let providers = Arc::new(Mutex::new(Providers::new()));
  let clients = Arc::new(Semaphore::new(16));
  let users = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
  let mut pipe = security::server_pipe(true)?;
  let mut housekeeping = tokio::time::interval(Duration::from_secs(1));

  while !stop.load(Ordering::Acquire) {
    tokio::select! {
      connected = pipe.connect() => {
        connected.map_err(|e| e.to_string())?;

        // Keep an instance alive while creating the next one so a caller
        // cannot squat the name between connections.
        let next = security::server_pipe(false)?;
        let connected = std::mem::replace(&mut pipe, next);

        if let Ok(permit) = Arc::clone(&clients).try_acquire_owned() {
          let providers = Arc::clone(&providers);
          let users = Arc::clone(&users);
          tokio::spawn(async move {
            let _permit = permit;

            if let Err(error) = connection(connected, generation, providers, users).await {
              crate::log_debug!("sensor_service_client_disconnected", "sensor_service::serve", Some(error));
            }
          });
        }
      }

      _ = housekeeping.tick() => {
        if let Ok(mut state) = providers.try_lock() { state.expire_idle(); }
      }
    }
  }

  Ok(())
}

struct UserLease {
  sid: String,
  users: Arc<Mutex<HashMap<String, usize>>>,
}

impl Drop for UserLease {
  fn drop(&mut self) {
    let mut users = self.users.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(count) = users.get_mut(&self.sid) {
      *count -= 1;

      if *count == 0 {
        users.remove(&self.sid);
      }
    }
  }
}

async fn connection(
  mut pipe: NamedPipeServer,
  generation: u64,
  providers: Arc<Mutex<Providers>>,
  users: Arc<Mutex<HashMap<String, usize>>>,
) -> Result<(), String> {
  let hello: Request =
    tokio::time::timeout(Duration::from_secs(3), read_frame(&mut pipe))
      .await
      .map_err(|_| "Sensor handshake timed out")??;
  hello.validate()?;

  if !matches!(hello.operation, Operation::Hello) {
    return Err("Hello must precede sensor requests".into());
  }

  let sid = security::client_sid(&pipe)?;

  {
    let mut counts = users.lock().map_err(|_| "Client accounting unavailable")?;
    let count = counts.entry(sid.clone()).or_default();

    if *count >= 4 {
      return Err("Too many sensor clients for this user".into());
    }

    *count += 1;
  }

  let _lease = UserLease { sid, users };
  let response = Reply {
    version: VERSION,
    generation,
    payload: Payload::Hello {
      capabilities: ALL,
      service_version: super::host::build_version().to_string(),
    },
  };
  tokio::time::timeout(Duration::from_secs(2), write_frame(&mut pipe, &response))
    .await
    .map_err(|_| "Sensor handshake write timed out")??;

  loop {
    let request: Request =
      tokio::time::timeout(Duration::from_secs(15), read_frame(&mut pipe))
        .await
        .map_err(|_| "Sensor demand lease expired")??;
    request.validate()?;

    if matches!(request.operation, Operation::Hello) {
      return Err("Duplicate Hello".into());
    }

    let owner = Arc::clone(&providers);
    let payload = tokio::task::spawn_blocking(move || {
      let mut providers = owner.lock().map_err(|_| "Sensor sampler unavailable")?;

      if INVALIDATE.swap(false, Ordering::AcqRel) {
        providers.reset();
      }

      Ok::<_, String>(match request.operation {
        Operation::Sample { families } => {
          Payload::Snapshot(Box::new(providers.sample(families)))
        }
        Operation::Diagnostics => Payload::Diagnostics(providers.diagnostics()),
        Operation::Hello => unreachable!(),
      })
    })
    .await
    .map_err(|e| e.to_string())??;

    let reply = Reply {
      version: VERSION,
      generation,
      payload,
    };
    tokio::time::timeout(Duration::from_secs(2), write_frame(&mut pipe, &reply))
      .await
      .map_err(|_| "Slow sensor client")??;

    // A client cannot turn IPC requests into unbounded hardware work.
    tokio::time::sleep(Duration::from_millis(250)).await;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn framing_rejects_length_before_payload_allocation() {
    let (mut writer, mut reader) = tokio::io::duplex(16);
    writer.write_u32_le(MAX_FRAME as u32 + 1).await.unwrap();
    assert!(
      read_frame::<Request>(&mut reader)
        .await
        .unwrap_err()
        .contains("length")
    );
  }

  #[tokio::test]
  async fn fragmented_frames_preserve_channel_identity_and_missing_values() {
    let (mut writer, mut reader) = tokio::io::duplex(8);
    let request = Request {
      version: VERSION,
      operation: Operation::Sample {
        families: MOTHERBOARD,
      },
    };

    let send = tokio::spawn(async move {
      write_frame(&mut writer, &request).await.unwrap();
    });

    let parsed: Request = read_frame(&mut reader).await.unwrap();
    parsed.validate().unwrap();
    assert!(matches!(
      parsed.operation,
      Operation::Sample {
        families: MOTHERBOARD
      }
    ));
    send.await.unwrap();
  }
}
