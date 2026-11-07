use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::windows::named_pipe::NamedPipeServer;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::{
  MAX_CLIENTS, core_temperatures::CoreTemperatures, providers::Providers, security,
};
use crate::sensor_service::protocol::*;

static INVALIDATE: AtomicBool = AtomicBool::new(false);

pub(super) fn invalidate() {
  INVALIDATE.store(true, Ordering::Release);
  super::core_temperatures::invalidate();
}

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
  let core_temperatures = Arc::new(CoreTemperatures::default());
  let clients = Arc::new(Semaphore::new(MAX_CLIENTS));
  let mut pipe = security::server_pipe(true)?;
  let mut housekeeping = tokio::time::interval(Duration::from_secs(1));

  while !stop.load(Ordering::Acquire) {
    tokio::select! {
      connected = pipe.connect() => {
        connected.map_err(|e| e.to_string())?;

        let Some(permit) = admit_client(&mut pipe, &clients) else {
          continue;
        };

        // Keep an instance alive while creating the next one so a caller
        // cannot squat the name between connections.
        let next = security::server_pipe(false)?;
        let connected = std::mem::replace(&mut pipe, next);

        let providers = Arc::clone(&providers);
        let core_temperatures = Arc::clone(&core_temperatures);
        tokio::spawn(async move {
          let _permit = permit;

          if let Err(error) = connection(connected, generation, providers, core_temperatures).await {
            crate::log_debug!("sensor_service_client_disconnected", "sensor_service::serve", Some(error));
          }
        });
      }

      _ = housekeeping.tick() => {
        if let Ok(mut state) = providers.try_lock() { state.expire_idle(); }
      }
    }
  }

  Ok(())
}

fn admit_client(
  pipe: &mut NamedPipeServer,
  clients: &Arc<Semaphore>,
) -> Option<OwnedSemaphorePermit> {
  match Arc::clone(clients).try_acquire_owned() {
    Ok(permit) => Some(permit),
    Err(_) => {
      // Reuse the listening instance at capacity instead of creating an
      // eighteenth instance or releasing ownership of the endpoint name.
      let _ = pipe.disconnect();
      None
    }
  }
}

async fn connection(
  mut pipe: NamedPipeServer,
  generation: u64,
  providers: Arc<Mutex<Providers>>,
  core_temperatures: Arc<CoreTemperatures>,
) -> Result<(), String> {
  let hello: Request =
    tokio::time::timeout(Duration::from_secs(3), read_frame(&mut pipe))
      .await
      .map_err(|_| "Sensor handshake timed out")??;
  hello.validate()?;

  if !matches!(hello.operation, Operation::Hello) {
    return Err("Hello must precede sensor requests".into());
  }

  let response = Reply {
    version: VERSION,
    generation,
    payload: Payload::Hello {
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
        .map_err(|_| "Sensor pipe read timed out")??;
    request.validate()?;

    let payload = match request.operation {
      Operation::Hello => return Err("Duplicate Hello".into()),
      Operation::CpuCoreTemperatures => {
        Payload::CpuCoreTemperatures(core_temperatures.request().await)
      }
      Operation::Sample | Operation::Diagnostics => {
        let owner = Arc::clone(&providers);
        let diagnostics = matches!(request.operation, Operation::Diagnostics);
        tokio::task::spawn_blocking(move || {
          let mut providers = owner.lock().map_err(|_| "Sensor sampler unavailable")?;

          if INVALIDATE.swap(false, Ordering::AcqRel) {
            providers.reset();
          }

          Ok::<_, String>(if diagnostics {
            Payload::Diagnostics(providers.diagnostics())
          } else {
            Payload::Snapshot(Box::new(providers.sample()))
          })
        })
        .await
        .map_err(|e| e.to_string())??
      }
    };

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
  async fn excess_clients_leave_the_listener_available_and_its_name_owned() {
    use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};

    let name = format!(r"\\.\pipe\Hardviz.CapacityTest.{}", std::process::id());
    let mut listener = ServerOptions::new()
      .first_pipe_instance(true)
      .max_instances(1)
      .create(&name)
      .unwrap();
    let clients = Arc::new(Semaphore::new(1));
    let held = Arc::clone(&clients).acquire_owned().await.unwrap();

    for _ in 0..3 {
      let (connected, client) = tokio::join!(listener.connect(), async {
        tokio::task::yield_now().await;
        ClientOptions::new().open(&name).unwrap()
      });
      connected.unwrap();
      let busy = ClientOptions::new().open(&name).unwrap_err();
      assert_eq!(
        busy.raw_os_error(),
        Some(windows::Win32::Foundation::ERROR_PIPE_BUSY.0 as i32)
      );

      assert!(admit_client(&mut listener, &clients).is_none());
      assert!(
        ServerOptions::new()
          .first_pipe_instance(true)
          .create(&name)
          .is_err()
      );
      drop(client);
    }

    drop(held);
    let (connected, _client) = tokio::join!(listener.connect(), async {
      tokio::task::yield_now().await;
      ClientOptions::new().open(&name).unwrap()
    });
    connected.unwrap();
    assert!(admit_client(&mut listener, &clients).is_some());
  }

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
  async fn fragmented_sample_request_is_decoded() {
    let (mut writer, mut reader) = tokio::io::duplex(8);
    let request = Request {
      version: VERSION,
      operation: Operation::Sample,
    };

    let send = tokio::spawn(async move {
      write_frame(&mut writer, &request).await.unwrap();
    });

    let parsed: Request = read_frame(&mut reader).await.unwrap();
    parsed.validate().unwrap();
    assert!(matches!(parsed.operation, Operation::Sample));
    send.await.unwrap();
  }
}
