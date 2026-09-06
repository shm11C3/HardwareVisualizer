use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use duckdb::{Connection, InterruptHandle, params};
use serde_json::{Value, json};

const PROCESS_ROWS_PER_MINUTE: i64 = 15;
const AMBIENT_ROWS_PER_MINUTE: i64 = 2;
const WATCHDOG_TIMEOUT: Duration = Duration::from_secs(30);
const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
struct Args {
  output: PathBuf,
  database: PathBuf,
  seed_rows: i64,
  amplification: i64,
}

enum Command {
  LongQuery {
    amplification: i64,
    started: mpsc::SyncSender<()>,
    reply: mpsc::SyncSender<Result<(), String>>,
  },
  Count {
    reply: mpsc::SyncSender<Result<Counts, String>>,
  },
  Snapshot {
    reply: mpsc::SyncSender<Result<Snapshot, String>>,
  },
  AppendMinute {
    minute: i64,
    reply: mpsc::SyncSender<Result<MinuteCommit, String>>,
  },
  Shutdown,
}

struct Ready {
  interrupt: Arc<InterruptHandle>,
  reader: Connection,
  checkpoint: Connection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Counts {
  process: i64,
  ambient: i64,
}

#[derive(Clone, Debug)]
struct MinuteCommit {
  timestamp: String,
  process_ids: Vec<i64>,
  ambient_ids: Vec<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Digests {
  process: u64,
  ambient: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Snapshot {
  counts: Counts,
  digests: Digests,
}

fn parse_args() -> Result<Args, String> {
  let mut output = None;
  let mut database = None;
  let mut seed_rows = 3_000_i64;
  let mut amplification = 100_000_i64;
  let mut args = env::args().skip(1);
  while let Some(arg) = args.next() {
    match arg.as_str() {
      "--output" => {
        output = Some(PathBuf::from(args.next().ok_or("missing --output value")?));
      }
      "--database" => {
        database = Some(PathBuf::from(
          args.next().ok_or("missing --database value")?,
        ));
      }
      "--seed-rows" => {
        seed_rows = args
          .next()
          .ok_or("missing --seed-rows value")?
          .parse()
          .map_err(|_| "--seed-rows must be an integer")?;
      }
      "--amplification" => {
        amplification = args
          .next()
          .ok_or("missing --amplification value")?
          .parse()
          .map_err(|_| "--amplification must be an integer")?;
      }
      "--help" | "-h" => {
        println!(
          "Usage: duckdb-lifecycle-probe --output PATH --database PATH \
           [--seed-rows 3000] [--amplification 100000]"
        );
        std::process::exit(0);
      }
      _ => return Err(format!("unknown argument: {arg}")),
    }
  }
  let output = output.ok_or("--output is required")?;
  let database = database.ok_or("--database is required")?;
  if seed_rows < PROCESS_ROWS_PER_MINUTE {
    return Err(format!(
      "--seed-rows must be at least {PROCESS_ROWS_PER_MINUTE}"
    ));
  }
  if amplification < 1_000 {
    return Err("--amplification must be at least 1000".into());
  }
  Ok(Args {
    output,
    database,
    seed_rows,
    amplification,
  })
}

fn sql_literal(path: &Path) -> Result<String, String> {
  let value = path
    .to_str()
    .ok_or_else(|| format!("path is not valid UTF-8: {}", path.display()))?;
  Ok(value.replace('\'', "''"))
}

fn configure(connection: &Connection, temp_dir: &Path) -> Result<(), String> {
  fs::create_dir_all(temp_dir).map_err(|error| error.to_string())?;
  connection
    .execute_batch(&format!(
      "SET threads=2; SET memory_limit='128MB'; SET temp_directory='{}';",
      sql_literal(temp_dir)?
    ))
    .map_err(|error| error.to_string())
}

fn create_schema(connection: &Connection) -> Result<(), String> {
  connection
    .execute_batch(
      "CREATE TABLE PROCESS_STATS (
         id BIGINT PRIMARY KEY, pid BIGINT NOT NULL, process_name VARCHAR NOT NULL,
         cpu_usage DOUBLE NOT NULL, memory_usage BIGINT NOT NULL,
         execution_sec BIGINT NOT NULL, timestamp VARCHAR NOT NULL
       );
       CREATE INDEX idx_process_timestamp ON PROCESS_STATS(timestamp);
       CREATE TABLE AMBIENT_ARCHIVE (
         id BIGINT PRIMARY KEY, source VARCHAR NOT NULL, temperature DOUBLE NOT NULL,
         humidity DOUBLE, timestamp VARCHAR NOT NULL
       );
       CREATE INDEX idx_ambient_timestamp ON AMBIENT_ARCHIVE(timestamp);",
    )
    .map_err(|error| error.to_string())
}

fn timestamp(minute: i64) -> String {
  let hour = minute / 60;
  let minute = minute % 60;
  format!("2026-01-01T{hour:02}:{minute:02}:00.000Z")
}

fn process_values(id: i64) -> (i64, String, f64, i64, i64, String) {
  let group = id % 45;
  (
    1_000 + group,
    format!("process-{group:02}"),
    ((id * 37) % 400) as f64 / 4.0,
    64_000_000 + (id * 7_919) % 2_000_000_000,
    id / PROCESS_ROWS_PER_MINUTE,
    timestamp(id / PROCESS_ROWS_PER_MINUTE),
  )
}

fn ambient_values(id: i64, minute: i64) -> (String, f64, Option<f64>, String) {
  (
    format!("ambient-{}", id % 2),
    18.0 + ((id * 13) % 120) as f64 / 10.0,
    (id % 7 != 0).then_some(35.0 + ((id * 17) % 500) as f64 / 10.0),
    timestamp(minute),
  )
}

fn ambient_seed_rows(seed_rows: i64) -> i64 {
  (seed_rows + 9) / 10
}

fn seed(connection: &mut Connection, seed_rows: i64) -> Result<(), String> {
  let tx = connection
    .transaction()
    .map_err(|error| error.to_string())?;
  {
    let mut statement = tx
      .prepare(
        "INSERT INTO PROCESS_STATS
         (id, pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
      )
      .map_err(|error| error.to_string())?;
    for id in 1..=seed_rows {
      let (pid, name, cpu, memory, execution, recorded_at) = process_values(id);
      statement
        .execute(params![id, pid, name, cpu, memory, execution, recorded_at])
        .map_err(|error| error.to_string())?;
    }
  }
  {
    let mut statement = tx
      .prepare(
        "INSERT INTO AMBIENT_ARCHIVE
         (id, source, temperature, humidity, timestamp)
         VALUES (?, ?, ?, ?, ?)",
      )
      .map_err(|error| error.to_string())?;
    for id in 1..=ambient_seed_rows(seed_rows) {
      let (source, temperature, humidity, recorded_at) = ambient_values(id, id);
      statement
        .execute(params![id, source, temperature, humidity, recorded_at])
        .map_err(|error| error.to_string())?;
    }
  }
  tx.commit().map_err(|error| error.to_string())?;
  connection
    .execute_batch("CHECKPOINT")
    .map_err(|error| error.to_string())
}

fn counts(connection: &Connection) -> Result<Counts, String> {
  Ok(Counts {
    process: connection
      .query_row("SELECT count(*) FROM PROCESS_STATS", [], |row| row.get(0))
      .map_err(|error| error.to_string())?,
    ambient: connection
      .query_row("SELECT count(*) FROM AMBIENT_ARCHIVE", [], |row| row.get(0))
      .map_err(|error| error.to_string())?,
  })
}

fn run_long_query(connection: &Connection, amplification: i64) -> Result<(), String> {
  let mut statement = connection
    .prepare(
      "SELECT p.pid, p.process_name,
              avg(p.cpu_usage + sin(r.i::DOUBLE / 1000.0)),
              avg(p.memory_usage), max(p.execution_sec), max(p.timestamp)
       FROM PROCESS_STATS p
       CROSS JOIN range(?) r(i)
       GROUP BY p.pid, p.process_name",
    )
    .map_err(|error| error.to_string())?;
  let mut rows = statement
    .query([amplification])
    .map_err(|error| error.to_string())?;
  while rows.next().map_err(|error| error.to_string())?.is_some() {}
  Ok(())
}

fn append_minute(
  connection: &mut Connection,
  seed_rows: i64,
  minute: i64,
) -> Result<MinuteCommit, String> {
  let timestamp = timestamp(minute);
  let process_start = seed_rows + (minute - 1) * PROCESS_ROWS_PER_MINUTE + 1;
  let ambient_start =
    ambient_seed_rows(seed_rows) + (minute - 1) * AMBIENT_ROWS_PER_MINUTE + 1;
  let process_ids: Vec<_> =
    (process_start..process_start + PROCESS_ROWS_PER_MINUTE).collect();
  let ambient_ids: Vec<_> =
    (ambient_start..ambient_start + AMBIENT_ROWS_PER_MINUTE).collect();
  let tx = connection
    .transaction()
    .map_err(|error| error.to_string())?;
  {
    let mut statement = tx
      .prepare(
        "INSERT INTO PROCESS_STATS
         (id, pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
      )
      .map_err(|error| error.to_string())?;
    for id in &process_ids {
      let (pid, name, cpu, memory, execution, _) = process_values(*id);
      statement
        .execute(params![id, pid, name, cpu, memory, execution, &timestamp])
        .map_err(|error| error.to_string())?;
    }
  }
  {
    let mut statement = tx
      .prepare(
        "INSERT INTO AMBIENT_ARCHIVE
         (id, source, temperature, humidity, timestamp)
         VALUES (?, ?, ?, ?, ?)",
      )
      .map_err(|error| error.to_string())?;
    for id in &ambient_ids {
      let (source, temperature, humidity, _) = ambient_values(*id, minute);
      statement
        .execute(params![id, source, temperature, humidity, &timestamp])
        .map_err(|error| error.to_string())?;
    }
  }
  tx.commit().map_err(|error| error.to_string())?;
  Ok(MinuteCommit {
    timestamp,
    process_ids,
    ambient_ids,
  })
}

fn owner_thread(
  database: PathBuf,
  temp_dir: PathBuf,
  seed_rows: i64,
  commands: mpsc::Receiver<Command>,
  ready: mpsc::SyncSender<Result<Ready, String>>,
) {
  let setup = (|| {
    let mut connection =
      Connection::open(&database).map_err(|error| error.to_string())?;
    configure(&connection, &temp_dir)?;
    create_schema(&connection)?;
    seed(&mut connection, seed_rows)?;
    let reader = connection.try_clone().map_err(|error| error.to_string())?;
    let checkpoint = connection.try_clone().map_err(|error| error.to_string())?;
    let interrupt = connection.interrupt_handle();
    Ok::<_, String>((
      connection,
      Ready {
        interrupt,
        reader,
        checkpoint,
      },
    ))
  })();
  let (mut connection, handles) = match setup {
    Ok(value) => value,
    Err(error) => {
      let _ = ready.send(Err(error));
      return;
    }
  };
  if ready.send(Ok(handles)).is_err() {
    return;
  }
  while let Ok(command) = commands.recv() {
    match command {
      Command::LongQuery {
        amplification,
        started,
        reply,
      } => {
        let _ = started.send(());
        let _ = reply.send(run_long_query(&connection, amplification));
      }
      Command::Count { reply } => {
        let _ = reply.send(counts(&connection));
      }
      Command::Snapshot { reply } => {
        let _ = reply.send(snapshot(&connection));
      }
      Command::AppendMinute { minute, reply } => {
        let _ = reply.send(append_minute(&mut connection, seed_rows, minute));
      }
      Command::Shutdown => break,
    }
  }
}

fn fnv_update(mut hash: u64, bytes: &[u8]) -> u64 {
  for byte in bytes {
    hash ^= u64::from(*byte);
    hash = hash.wrapping_mul(0x100000001b3);
  }
  hash
}

fn hash_field(hash: u64, bytes: impl AsRef<[u8]>) -> u64 {
  let hash = fnv_update(hash, bytes.as_ref());
  fnv_update(hash, &[0xff])
}

fn digest(connection: &Connection) -> Result<Digests, String> {
  let mut process = 0xcbf29ce484222325_u64;
  {
    let mut statement = connection
      .prepare(
        "SELECT id, pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp
         FROM PROCESS_STATS ORDER BY id",
      )
      .map_err(|error| error.to_string())?;
    let rows = statement
      .query_map([], |row| {
        Ok((
          row.get::<_, i64>(0)?,
          row.get::<_, i64>(1)?,
          row.get::<_, String>(2)?,
          row.get::<_, f64>(3)?,
          row.get::<_, i64>(4)?,
          row.get::<_, i64>(5)?,
          row.get::<_, String>(6)?,
        ))
      })
      .map_err(|error| error.to_string())?;
    for row in rows {
      let (id, pid, name, cpu, memory, execution, recorded_at) =
        row.map_err(|error| error.to_string())?;
      process = hash_field(process, id.to_le_bytes());
      process = hash_field(process, pid.to_le_bytes());
      process = hash_field(process, name);
      process = hash_field(process, cpu.to_bits().to_le_bytes());
      process = hash_field(process, memory.to_le_bytes());
      process = hash_field(process, execution.to_le_bytes());
      process = hash_field(process, recorded_at);
    }
  }
  let mut ambient = 0xcbf29ce484222325_u64;
  {
    let mut statement = connection
      .prepare(
        "SELECT id, source, temperature, humidity, timestamp
         FROM AMBIENT_ARCHIVE ORDER BY id",
      )
      .map_err(|error| error.to_string())?;
    let rows = statement
      .query_map([], |row| {
        Ok((
          row.get::<_, i64>(0)?,
          row.get::<_, String>(1)?,
          row.get::<_, f64>(2)?,
          row.get::<_, Option<f64>>(3)?,
          row.get::<_, String>(4)?,
        ))
      })
      .map_err(|error| error.to_string())?;
    for row in rows {
      let (id, source, temperature, humidity, recorded_at) =
        row.map_err(|error| error.to_string())?;
      ambient = hash_field(ambient, id.to_le_bytes());
      ambient = hash_field(ambient, source);
      ambient = hash_field(ambient, temperature.to_bits().to_le_bytes());
      ambient = match humidity {
        Some(value) => hash_field(ambient, value.to_bits().to_le_bytes()),
        None => hash_field(ambient, [0]),
      };
      ambient = hash_field(ambient, recorded_at);
    }
  }
  Ok(Digests { process, ambient })
}

fn snapshot(connection: &Connection) -> Result<Snapshot, String> {
  Ok(Snapshot {
    counts: counts(connection)?,
    digests: digest(connection)?,
  })
}

fn wal_bytes(database: &Path) -> u64 {
  PathBuf::from(format!("{}.wal", database.display()))
    .metadata()
    .map(|metadata| metadata.len())
    .unwrap_or(0)
}

fn receive<T>(
  receiver: &mpsc::Receiver<Result<T, String>>,
  operation: &str,
) -> Result<T, String> {
  receiver.recv_timeout(REPLY_TIMEOUT).map_err(|error| {
    format!("{operation} did not complete within {REPLY_TIMEOUT:?}: {error}")
  })?
}

fn run(args: &Args) -> Result<Value, String> {
  if args.database.exists() {
    return Err(format!(
      "refusing existing database: {}",
      args.database.display()
    ));
  }
  if let Some(parent) = args.database.parent() {
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
  }
  let temp_dir = args.database.with_extension("temp");
  let (command_tx, command_rx) = mpsc::sync_channel(4);
  let (ready_tx, ready_rx) = mpsc::sync_channel(1);
  let owner_database = args.database.clone();
  let owner_temp = temp_dir.clone();
  let seed_rows = args.seed_rows;
  let owner = thread::spawn(move || {
    owner_thread(owner_database, owner_temp, seed_rows, command_rx, ready_tx);
  });
  let Ready {
    interrupt,
    reader,
    checkpoint,
  } = ready_rx
    .recv_timeout(REPLY_TIMEOUT)
    .map_err(|error| format!("owner setup timeout: {error}"))??;

  let version: String = reader
    .query_row("SELECT version()", [], |row| row.get(0))
    .map_err(|error| error.to_string())?;
  if version.trim_start_matches('v') != "1.5.5" {
    return Err(format!("expected DuckDB 1.5.5, got {version}"));
  }
  reader
    .execute_batch("BEGIN TRANSACTION")
    .map_err(|error| error.to_string())?;
  let pinned_before = counts(&reader)?;
  let pinned_digest_before = digest(&reader)?;

  let (started_tx, started_rx) = mpsc::sync_channel(1);
  let (long_tx, long_rx) = mpsc::sync_channel(1);
  command_tx
    .send(Command::LongQuery {
      amplification: args.amplification,
      started: started_tx,
      reply: long_tx,
    })
    .map_err(|error| error.to_string())?;
  started_rx
    .recv_timeout(REPLY_TIMEOUT)
    .map_err(|error| format!("long query did not enter owner: {error}"))?;
  let cancel_started = Instant::now();
  let long_result = loop {
    interrupt.interrupt();
    match long_rx.recv_timeout(Duration::from_millis(10)) {
      Ok(result) => break result,
      Err(mpsc::RecvTimeoutError::Timeout)
        if cancel_started.elapsed() < REPLY_TIMEOUT => {}
      Err(error) => return Err(format!("cancelled query did not stop: {error}")),
    }
  };
  let cancel_ms = cancel_started.elapsed().as_secs_f64() * 1_000.0;
  let cancel_error = long_result
    .err()
    .ok_or("long query completed before interruption; cancellation not demonstrated")?;
  if !cancel_error.to_ascii_uppercase().contains("INTERRUPT") {
    return Err(format!(
      "long query failed without DuckDB INTERRUPT evidence: {cancel_error}"
    ));
  }

  let (count_tx, count_rx) = mpsc::sync_channel(1);
  command_tx
    .send(Command::Count { reply: count_tx })
    .map_err(|error| error.to_string())?;
  let after_cancel_count = receive(&count_rx, "post-cancel count")?;

  let wal_before_write = wal_bytes(&args.database);
  let (append_tx, append_rx) = mpsc::sync_channel(1);
  command_tx
    .send(Command::AppendMinute {
      minute: 1,
      reply: append_tx,
    })
    .map_err(|error| error.to_string())?;
  let committed = receive(&append_rx, "minute append")?;
  let wal_after_write = wal_bytes(&args.database);
  let owner_after_write = {
    let (tx, rx) = mpsc::sync_channel(1);
    command_tx
      .send(Command::Snapshot { reply: tx })
      .map_err(|error| error.to_string())?;
    receive(&rx, "post-write snapshot")?
  };
  let pinned_after_write = counts(&reader)?;
  let pinned_digest_after_write = digest(&reader)?;

  let (checkpoint_started_tx, checkpoint_started_rx) = mpsc::sync_channel(1);
  let (checkpoint_tx, checkpoint_rx) = mpsc::sync_channel(1);
  let checkpoint_thread = thread::spawn(move || {
    let started = Instant::now();
    let _ = checkpoint_started_tx.send(());
    let result = checkpoint
      .execute_batch("CHECKPOINT")
      .map_err(|error| error.to_string());
    let _ = checkpoint_tx.send((result, started.elapsed()));
  });
  checkpoint_started_rx
    .recv_timeout(REPLY_TIMEOUT)
    .map_err(|error| format!("checkpoint did not start: {error}"))?;
  let while_pinned = checkpoint_rx.recv_timeout(Duration::from_millis(100));
  let checkpoint_while_pinned_state = match &while_pinned {
    Ok((Ok(()), _)) => "success",
    Ok((Err(_), _)) => "returned_error",
    Err(mpsc::RecvTimeoutError::Timeout) => "still_waiting",
    Err(mpsc::RecvTimeoutError::Disconnected) => "channel_disconnected",
  };
  let pinned_after_checkpoint = counts(&reader)?;
  let pinned_digest_after_checkpoint = digest(&reader)?;
  reader
    .execute_batch("COMMIT")
    .map_err(|error| error.to_string())?;
  let (first_checkpoint_result, first_checkpoint_duration) = match while_pinned {
    Ok(value) => value,
    Err(mpsc::RecvTimeoutError::Timeout) => {
      checkpoint_rx.recv_timeout(REPLY_TIMEOUT).map_err(|error| {
        format!("checkpoint did not finish after reader release: {error}")
      })?
    }
    Err(error) => return Err(format!("checkpoint channel failed: {error}")),
  };
  checkpoint_thread
    .join()
    .map_err(|_| "checkpoint thread panicked".to_string())?;
  let first_checkpoint_error = first_checkpoint_result.err();
  reader
    .execute_batch("CHECKPOINT")
    .map_err(|error| format!("checkpoint retry after reader release failed: {error}"))?;
  let wal_after_checkpoint = wal_bytes(&args.database);

  command_tx
    .send(Command::Shutdown)
    .map_err(|error| error.to_string())?;
  owner
    .join()
    .map_err(|_| "owner thread panicked".to_string())?;
  drop(interrupt);
  drop(reader);

  let reopened = Connection::open(&args.database).map_err(|error| error.to_string())?;
  let reopened_counts = counts(&reopened)?;
  let reopened_digest = digest(&reopened)?;
  let expected_counts = Counts {
    process: args.seed_rows + PROCESS_ROWS_PER_MINUTE,
    ambient: ambient_seed_rows(args.seed_rows) + AMBIENT_ROWS_PER_MINUTE,
  };
  let pass = pinned_before
    == Counts {
      process: args.seed_rows,
      ambient: ambient_seed_rows(args.seed_rows),
    }
    && after_cancel_count == pinned_before
    && pinned_after_write == pinned_before
    && pinned_digest_after_write == pinned_digest_before
    && pinned_after_checkpoint == pinned_before
    && pinned_digest_after_checkpoint == pinned_digest_before
    && owner_after_write.counts == expected_counts
    && reopened_counts == expected_counts
    && reopened_digest == owner_after_write.digests;
  if !pass {
    return Err("lifecycle invariants did not match expected counts/digests".into());
  }

  Ok(json!({
    "schema_version": 1,
    "pass": true,
    "binding": "duckdb-rs 1.10505.0 bundled",
    "engine_version": version,
    "configuration": {
      "threads": 2,
      "memory_limit": "128MB",
      "owner_channel_capacity": 4,
      "watchdog_seconds": WATCHDOG_TIMEOUT.as_secs(),
    },
    "database": args.database.display().to_string(),
    "connection_lifecycle": {
      "owner": "one dedicated blocking thread exclusively owns the writer connection",
      "clones": "reader and checkpoint use Connection::try_clone from the same database instance",
      "interrupt": "out-of-band Arc<InterruptHandle>; repeated only until command completion",
      "cancel_error": cancel_error,
      "cancel_ms": cancel_ms,
      "post_cancel_request_succeeded": after_cancel_count == pinned_before,
      "post_cancel_minute_commit_succeeded": owner_after_write.counts == expected_counts,
    },
    "minute_commit": {
      "timestamp": committed.timestamp,
      "process_ids": committed.process_ids,
      "ambient_ids": committed.ambient_ids,
      "one_transaction": true,
      "identity": "stored (pid, process_name) pair remains opaque",
    },
    "pinned_reader": {
      "before": {"process": pinned_before.process, "ambient": pinned_before.ambient},
      "after_writer_commit": {
        "process": pinned_after_write.process,
        "ambient": pinned_after_write.ambient
      },
      "digest_unchanged": pinned_digest_before == pinned_digest_after_write,
      "after_checkpoint_observation": {
        "process": pinned_after_checkpoint.process,
        "ambient": pinned_after_checkpoint.ambient,
        "counts_unchanged": pinned_after_checkpoint == pinned_before,
        "all_fields_digest_unchanged": pinned_digest_after_checkpoint == pinned_digest_before,
      },
    },
    "checkpoint": {
      "state_after_100ms_while_reader_pinned": checkpoint_while_pinned_state,
      "first_attempt_ms": first_checkpoint_duration.as_secs_f64() * 1_000.0,
      "first_attempt_error": first_checkpoint_error,
      "retry_after_reader_release_succeeded": true,
      "wal_bytes_before_write": wal_before_write,
      "wal_bytes_after_write": wal_after_write,
      "wal_bytes_after_checkpoint": wal_after_checkpoint,
    },
    "reopen": {
      "process_rows": reopened_counts.process,
      "ambient_rows": reopened_counts.ambient,
      "process_digest": format!("{:016x}", reopened_digest.process),
      "ambient_digest": format!("{:016x}", reopened_digest.ambient),
      "committed_rows_preserved": reopened_counts == expected_counts,
      "owner_before_close_process_digest": format!("{:016x}", owner_after_write.digests.process),
      "owner_before_close_ambient_digest": format!("{:016x}", owner_after_write.digests.ambient),
      "all_fields_digest_matches_owner_before_close": reopened_digest == owner_after_write.digests,
    },
    "limits": [
      "Synthetic standalone database; no production Cargo or lifecycle integration.",
      "The started signal marks owner dispatch, while DuckDB's explicit INTERRUPT error proves cancellation.",
      "The watchdog terminates the whole probe on a stuck FFI call; it cannot recover that process.",
      "Process SIGKILL evidence belongs to the earlier Python probe; OS and power-loss durability remain untested.",
      "Checkpoint behavior is observed, not assumed; only the post-release successful checkpoint is required.",
    ],
  }))
}

fn main() -> Result<(), String> {
  let args = parse_args()?;
  let finished = Arc::new(AtomicBool::new(false));
  let watchdog_finished = Arc::clone(&finished);
  let watchdog_output = args.output.clone();
  thread::spawn(move || {
    thread::sleep(WATCHDOG_TIMEOUT);
    if !watchdog_finished.load(Ordering::SeqCst) {
      let failure = json!({
        "pass": false,
        "error": "watchdog timeout",
        "watchdog_seconds": WATCHDOG_TIMEOUT.as_secs()
      });
      eprintln!("{failure}");
      if let Some(parent) = watchdog_output.parent() {
        let _ = fs::create_dir_all(parent);
      }
      if let Ok(mut encoded) = serde_json::to_vec_pretty(&failure) {
        encoded.push(b'\n');
        let _ = fs::write(&watchdog_output, encoded);
      }
      std::process::exit(124);
    }
  });
  let result = run(&args)?;
  if let Some(parent) = args.output.parent() {
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
  }
  let mut encoded =
    serde_json::to_vec_pretty(&result).map_err(|error| error.to_string())?;
  encoded.push(b'\n');
  fs::write(&args.output, encoded).map_err(|error| error.to_string())?;
  println!("{result}");
  finished.store(true, Ordering::SeqCst);
  Ok(())
}
