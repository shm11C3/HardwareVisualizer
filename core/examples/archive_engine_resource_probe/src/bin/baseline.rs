use archive_engine_resource_probe::{emit_and_wait, parse_args, runtime_workers};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), String> {
  let args = parse_args()?;
  if args.prepare || args.database.is_some() || args.seed_rows != 0 {
    return Err("baseline accepts no database, prepare, or seed rows".into());
  }
  let config = || {
    json!({
      "profile": "release-size",
      "engine_linked": false,
      "tokio_runtime": "multi_thread",
      "tokio_worker_threads": runtime_workers(),
      "handshake": "serde_json"
    })
  };
  emit_and_wait("baseline", "before_open", 0, 0, None, config())?;
  emit_and_wait("baseline", "after_close", 0, 0, None, config())
}
