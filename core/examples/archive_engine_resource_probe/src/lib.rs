use std::env;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

pub const PROCESS_QUERY: &str = "SELECT pid, process_name, AVG(cpu_usage), AVG(memory_usage), COUNT(*), MAX(execution_sec), MAX(timestamp) FROM PROCESS_STATS WHERE timestamp >= ? AND timestamp <= ? GROUP BY pid, process_name ORDER BY AVG(cpu_usage) DESC, pid ASC, process_name ASC";
pub const AMBIENT_QUERY: &str = "SELECT id, source, temperature, humidity, timestamp FROM AMBIENT_ARCHIVE WHERE epoch_ms >= ? AND epoch_ms <= ? ORDER BY id";
pub const START_TEXT: &str = "2026-01-02T00:00:00.000Z";
pub const END_TEXT: &str = "2026-01-04T00:00:00.000Z";
pub const START_MS: i64 = 86_400_000;
pub const END_MS: i64 = 259_200_000;

#[derive(Debug)]
pub struct Args {
  pub database: Option<PathBuf>,
  pub prepare: bool,
  pub seed_rows: u64,
}

pub fn parse_args() -> Result<Args, String> {
  let mut database = None;
  let mut prepare = false;
  let mut seed_rows = 0_u64;
  let mut args = env::args().skip(1);
  while let Some(arg) = args.next() {
    match arg.as_str() {
      "--database" => {
        database = Some(PathBuf::from(
          args.next().ok_or("--database requires a path")?,
        ));
      }
      "--prepare" => prepare = true,
      "--seed-rows" => {
        seed_rows = args
          .next()
          .ok_or("--seed-rows requires an integer")?
          .parse()
          .map_err(|_| "--seed-rows must be an integer")?;
      }
      "--help" | "-h" => {
        println!(
          "Usage: RESOURCE_PROBE [--database PATH] [--prepare] [--seed-rows N]\n\
           Without --prepare, emits readiness JSON and waits for one stdin line at each stage.\n\
           --database is required for SQLite/DuckDB. --seed-rows describes the prepared fixture."
        );
        std::process::exit(0);
      }
      _ => return Err(format!("unknown argument: {arg}")),
    }
  }
  if prepare && database.is_none() {
    return Err("--prepare requires --database".into());
  }
  if seed_rows > 400_000 {
    return Err("--seed-rows must be at most 400000".into());
  }
  Ok(Args {
    database,
    prepare,
    seed_rows,
  })
}

pub fn emit_and_wait(
  engine: &str,
  stage: &str,
  seed_rows: u64,
  result_rows: u64,
  digest: Option<u64>,
  configuration: serde_json::Value,
) -> Result<(), String> {
  let event = serde_json::json!({
    "protocol_version": 1,
    "engine": engine,
    "stage": stage,
    "pid": std::process::id(),
    "seed_rows": seed_rows,
    "ambient_rows": ambient_rows(seed_rows),
    "result_rows": result_rows,
    "query_digest": digest.map(|value| format!("{value:016x}")),
    "configuration": configuration,
  });
  println!("{event}");
  io::stdout().flush().map_err(|error| error.to_string())?;
  let mut line = String::new();
  io::stdin()
    .lock()
    .read_line(&mut line)
    .map_err(|error| error.to_string())?;
  if line.is_empty() {
    return Err("stdin closed before stage acknowledgement".into());
  }
  Ok(())
}

pub fn ambient_rows(process_rows: u64) -> u64 {
  process_rows.div_ceil(10)
}

pub fn timestamp_for_minute(minute: u64) -> String {
  let day = minute / 1_440 + 1;
  let hour = minute % 1_440 / 60;
  let minute = minute % 60;
  format!("2026-01-{day:02}T{hour:02}:{minute:02}:00.000Z")
}

pub fn process_values(id: u64) -> (i64, String, f64, i64, i64, String) {
  let group = id % 45;
  (
    1_000 + group as i64,
    format!("process-{group:02}"),
    ((id * 37) % 400) as f64 / 4.0,
    64_000_000 + ((id * 7_919) % 2_000_000_000) as i64,
    (id / 15) as i64,
    timestamp_for_minute(id / 15),
  )
}

pub fn ambient_values(id: u64) -> (String, f64, Option<f64>, String, i64) {
  let minute = id;
  (
    format!("ambient-{}", id % 2),
    18.0 + ((id * 13) % 120) as f64 / 10.0,
    (id % 7 != 0).then_some(35.0 + ((id * 17) % 500) as f64 / 10.0),
    timestamp_for_minute(minute),
    (minute * 60_000) as i64,
  )
}

pub fn fnv_update(mut hash: u64, bytes: &[u8]) -> u64 {
  for byte in bytes {
    hash ^= u64::from(*byte);
    hash = hash.wrapping_mul(0x100000001b3);
  }
  hash
}

pub fn hash_field(hash: u64, value: impl AsRef<[u8]>) -> u64 {
  let hash = fnv_update(hash, value.as_ref());
  fnv_update(hash, &[0xff])
}

pub fn runtime_workers() -> usize {
  std::thread::available_parallelism()
    .map(std::num::NonZeroUsize::get)
    .unwrap_or(1)
}
