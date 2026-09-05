use super::{
  Config, Error, Result,
  codec::{self, Compression, Layout, Record, Value},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{
  Connection, Row, SqliteConnection,
  sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteRow, SqliteSynchronous},
};
use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf},
  time::{Duration, Instant},
};

const PROCESS: &str = "process_stats";
const AMBIENT: &str = "ambient_archive";
const EPOCH_MS_SQL: &str = "(CAST(strftime('%s', timestamp) AS INTEGER) * 1000 + CAST(substr(strftime('%f', timestamp), 4, 3) AS INTEGER))";
const SEED_BATCH_MINUTES: u64 = 128;
const PROCESS_CHUNK_LOOP_SQL: &str =
  "SELECT id, row_count, payload, digest FROM ARCHIVE_CHUNKS
   WHERE family = ? AND id > ? AND max_timestamp >= ? AND min_timestamp <= ?
   ORDER BY id LIMIT 1";
const AMBIENT_CHUNK_LOOP_SQL: &str =
  "SELECT id, row_count, payload, digest FROM ARCHIVE_CHUNKS
   WHERE family = ? AND id > ? ORDER BY id LIMIT 1";

#[derive(Default)]
pub(crate) struct Times {
  pub fixture_seed_batch: Vec<Duration>,
  pub production_minute_append: Vec<Duration>,
  pub source_scan: Vec<Duration>,
  pub encode: Vec<Duration>,
  pub decode: Vec<Duration>,
  pub chunk_insert: Vec<Duration>,
  pub finalize: Vec<Duration>,
  pub process_baseline: Vec<Duration>,
  pub process_candidate: Vec<Duration>,
  pub ambient_baseline: Vec<Duration>,
  pub ambient_candidate: Vec<Duration>,
}

#[derive(Default)]
pub(crate) struct Finalized {
  pub chunks: i64,
  pub encoded_bytes: i64,
  pub decoded_value_bytes: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct CorrectnessReport {
  exact_decoder_check: bool,
  persisted_reopen_exact_records: bool,
  process_query_equivalent: bool,
  ambient_raw_range_equivalent: bool,
  process_float_tolerance: &'static str,
  precommit_changed_selection_rollback: bool,
  postcommit_retry_exact_multiplicity: bool,
  concurrent_snapshot_before_or_after: bool,
  process_groups: usize,
  ambient_range_rows: u64,
}

impl CorrectnessReport {
  pub(crate) fn all_pass(&self) -> bool {
    self.exact_decoder_check
      && self.persisted_reopen_exact_records
      && self.process_query_equivalent
      && self.ambient_raw_range_equivalent
      && self.precommit_changed_selection_rollback
      && self.postcommit_retry_exact_multiplicity
      && self.concurrent_snapshot_before_or_after
  }
}

#[derive(Serialize)]
pub(crate) struct FileFootprint {
  pub database_bytes: u64,
  pub wal_bytes: u64,
  pub shm_bytes: u64,
  page_size: i64,
  page_count: i64,
  freelist_pages: i64,
  allocated_page_bytes: i64,
  live_page_bytes: i64,
  index_bytes: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct SqliteReport {
  version: String,
  journal_mode: String,
  synchronous: i64,
  compile_options: Vec<String>,
}

pub(crate) struct Benchmark {
  config: Config,
  baseline_path: PathBuf,
  candidate_path: PathBuf,
  baseline: SqliteConnection,
  candidate: Option<SqliteConnection>,
  pub process_rows: i64,
  pub ambient_rows: i64,
  pub times: Times,
  pub finalized: Finalized,
  pub correctness: CorrectnessReport,
}

pub(crate) fn layout_name(layout: Layout) -> &'static str {
  match layout {
    Layout::Row => "row",
    Layout::Columnar => "columnar",
  }
}

pub(crate) fn compression_name(compression: Compression) -> &'static str {
  match compression {
    Compression::None => "none",
    Compression::Deflate => "deflate",
  }
}

impl Benchmark {
  pub async fn create(config: Config) -> Result<Self> {
    let baseline_path = config.output.join("relational.sqlite3");
    let candidate_path = config.output.join("chunked.sqlite3");
    let mut baseline = open(&baseline_path).await?;
    create_schema(&mut baseline).await?;
    Ok(Self {
      config,
      baseline_path,
      candidate_path,
      baseline,
      candidate: None,
      process_rows: 0,
      ambient_rows: 0,
      times: Times::default(),
      finalized: Finalized::default(),
      correctness: CorrectnessReport {
        process_float_tolerance: "abs(actual-reference) <= max(1e-9, 1e-12*abs(reference))",
        ..CorrectnessReport::default()
      },
    })
  }

  pub async fn seed(&mut self) -> Result<()> {
    let sampled_minutes = self.config.minutes.div_ceil(self.config.duty_cycle);
    let mut sampled = 0;
    while sampled < sampled_minutes {
      let batch_started = Instant::now();
      let mut tx = self.baseline.begin().await?;
      let batch_end = (sampled + SEED_BATCH_MINUTES).min(sampled_minutes);
      while sampled < batch_end {
        let minute = sampled * self.config.duty_cycle;
        insert_minute(&mut tx, &self.config, minute).await?;
        sampled += 1;
      }
      tx.commit().await?;
      self.times.fixture_seed_batch.push(batch_started.elapsed());
    }
    checkpoint(&mut self.baseline).await?;
    self.process_rows =
      scalar(&mut self.baseline, "SELECT COUNT(*) FROM PROCESS_STATS").await?;
    self.ambient_rows =
      scalar(&mut self.baseline, "SELECT COUNT(*) FROM AMBIENT_ARCHIVE").await?;
    measure_minute_append(&self.config, &mut self.times.production_minute_append).await?;
    fs::copy(&self.baseline_path, &self.candidate_path)?;
    let mut candidate = open(&self.candidate_path).await?;
    create_chunk_schema(&mut candidate).await?;
    self.candidate = Some(candidate);
    Ok(())
  }

  pub async fn finalize(&mut self) -> Result<()> {
    self.correctness.precommit_changed_selection_rollback =
      rollback_changed_selection(&self.candidate_path, &self.config).await?;
    let candidate = self.candidate.as_mut().ok_or("candidate is not open")?;
    for family in [PROCESS, AMBIENT] {
      loop {
        let finalize_started = Instant::now();
        let scan_started = Instant::now();
        let records = select_chunk(candidate, family, &self.config).await?;
        self.times.source_scan.push(scan_started.elapsed());
        if records.is_empty() {
          break;
        }
        let decoded_value_bytes = value_bytes(&records);
        let encode_started = Instant::now();
        let payload =
          codec::encode(&records, self.config.layout, self.config.compression)
            .map_err(codec_error)?;
        self.times.encode.push(encode_started.elapsed());
        let decode_started = Instant::now();
        let decoded = codec::decode(&payload).map_err(codec_error)?;
        self.times.decode.push(decode_started.elapsed());
        if decoded != records {
          return Err(format!("codec changed exact {family} records").into());
        }
        self.correctness.exact_decoder_check = true;
        let insert_started = Instant::now();
        persist_chunk(
          candidate,
          family,
          &records,
          &payload,
          decoded_value_bytes,
          &self.config,
        )
        .await?;
        self.times.chunk_insert.push(insert_started.elapsed());
        self.times.finalize.push(finalize_started.elapsed());
        self.finalized.chunks += 1;
        self.finalized.encoded_bytes += i64::try_from(payload.len())?;
        self.finalized.decoded_value_bytes += decoded_value_bytes;
      }
    }
    checkpoint(candidate).await?;
    let replacement = open(&self.candidate_path).await?;
    let old = self
      .candidate
      .replace(replacement)
      .ok_or("candidate is not open")?;
    old.close().await?;
    Ok(())
  }

  pub async fn verify_and_query(&mut self) -> Result<()> {
    let candidate = self.candidate.as_mut().ok_or("candidate is not open")?;
    self.correctness.persisted_reopen_exact_records =
      compare_persisted(&mut self.baseline, candidate, &mut self.times.decode).await?;

    let before_rows = total_logical_rows(candidate, &mut self.times.decode).await?;
    let retry = finalize_once(candidate, PROCESS, &self.config).await?;
    let after_rows = total_logical_rows(candidate, &mut self.times.decode).await?;
    self.correctness.postcommit_retry_exact_multiplicity =
      retry == 0 && before_rows == after_rows;
    self.correctness.concurrent_snapshot_before_or_after =
      concurrent_snapshot(&self.candidate_path, &self.config).await?;

    let start_minute = self.config.minutes / 2;
    let end_minute = self.config.minutes.saturating_sub(1);
    let start = timestamp(start_minute)?;
    let end = timestamp(end_minute)?;
    let start_ms = parse_timestamp(&start)?;
    let end_ms = parse_timestamp(&end)?;
    for _ in 0..self.config.repetitions {
      let started = Instant::now();
      let baseline = process_oracle(&mut self.baseline, &start, &end).await?;
      self.times.process_baseline.push(started.elapsed());
      let started = Instant::now();
      let chunked = process_chunked(
        candidate,
        &start,
        &end,
        self.config.group_cap,
        &mut self.times.decode,
      )
      .await?;
      self.times.process_candidate.push(started.elapsed());
      self.correctness.process_query_equivalent = compare_aggregates(&baseline, &chunked);
      self.correctness.process_groups = baseline.len();

      let started = Instant::now();
      let baseline_ambient = ambient_oracle(&mut self.baseline, start_ms, end_ms).await?;
      self.times.ambient_baseline.push(started.elapsed());
      let started = Instant::now();
      let chunked_ambient =
        ambient_chunked(candidate, start_ms, end_ms, &mut self.times.decode).await?;
      self.times.ambient_candidate.push(started.elapsed());
      self.correctness.ambient_raw_range_equivalent = baseline_ambient == chunked_ambient;
      self.correctness.ambient_range_rows = baseline_ambient.1;
      if !self.correctness.process_query_equivalent
        || !self.correctness.ambient_raw_range_equivalent
      {
        return Err("differential query comparison failed".into());
      }
    }
    Ok(())
  }

  pub async fn baseline_footprint(&mut self) -> Result<FileFootprint> {
    checkpoint(&mut self.baseline).await?;
    footprint(&mut self.baseline, &self.baseline_path).await
  }

  pub async fn candidate_footprint(&mut self) -> Result<FileFootprint> {
    let candidate = self.candidate.as_mut().ok_or("candidate is not open")?;
    checkpoint(candidate).await?;
    footprint(candidate, &self.candidate_path).await
  }

  pub async fn vacuum_candidate(&mut self) -> Result<()> {
    let candidate = self.candidate.as_mut().ok_or("candidate is not open")?;
    sqlx::query("VACUUM").execute(&mut *candidate).await?;
    checkpoint(candidate).await
  }

  pub async fn sqlite_report(&mut self) -> Result<SqliteReport> {
    let candidate = self.candidate.as_mut().ok_or("candidate is not open")?;
    Ok(SqliteReport {
      version: sqlx::query_scalar("SELECT sqlite_version()")
        .fetch_one(&mut *candidate)
        .await?,
      journal_mode: sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&mut *candidate)
        .await?,
      synchronous: sqlx::query_scalar("PRAGMA synchronous")
        .fetch_one(&mut *candidate)
        .await?,
      compile_options: sqlx::query_scalar("PRAGMA compile_options")
        .fetch_all(&mut *candidate)
        .await?,
    })
  }
}

fn codec_error(message: String) -> Error {
  message.into()
}

async fn open(path: &Path) -> Result<SqliteConnection> {
  let options = SqliteConnectOptions::new()
    .filename(path)
    .create_if_missing(true)
    .journal_mode(SqliteJournalMode::Wal)
    .synchronous(SqliteSynchronous::Normal)
    .busy_timeout(Duration::from_secs(30));
  Ok(SqliteConnection::connect_with(&options).await?)
}

async fn create_schema(db: &mut SqliteConnection) -> Result<()> {
  for ddl in [
    "CREATE TABLE PROCESS_STATS (id INTEGER PRIMARY KEY AUTOINCREMENT, pid INTEGER NOT NULL, process_name TEXT NOT NULL, cpu_usage REAL NOT NULL, memory_usage INTEGER NOT NULL, execution_sec INTEGER NOT NULL, timestamp DATETIME NOT NULL)",
    "CREATE INDEX idx_process_stats_timestamp ON PROCESS_STATS(timestamp)",
    "CREATE TABLE AMBIENT_ARCHIVE (id INTEGER PRIMARY KEY AUTOINCREMENT, source TEXT NOT NULL, temperature REAL NOT NULL, humidity REAL, timestamp DATETIME NOT NULL)",
    "CREATE INDEX idx_ambient_archive_timestamp ON AMBIENT_ARCHIVE(timestamp)",
  ] {
    sqlx::query(ddl).execute(&mut *db).await?;
  }
  Ok(())
}

async fn create_chunk_schema(db: &mut SqliteConnection) -> Result<()> {
  sqlx::query("CREATE TABLE ARCHIVE_CHUNKS (id INTEGER PRIMARY KEY AUTOINCREMENT, family TEXT NOT NULL, min_row_id INTEGER NOT NULL, max_row_id INTEGER NOT NULL, min_timestamp TEXT NOT NULL, max_timestamp TEXT NOT NULL, row_count INTEGER NOT NULL, layout TEXT NOT NULL, compression TEXT NOT NULL, decoded_value_bytes INTEGER NOT NULL, payload BLOB NOT NULL, digest BLOB NOT NULL)")
    .execute(&mut *db).await?;
  sqlx::query("CREATE INDEX idx_archive_chunks_family_id ON ARCHIVE_CHUNKS(family, id)")
    .execute(&mut *db)
    .await?;
  Ok(())
}

async fn measure_minute_append(
  config: &Config,
  timings: &mut Vec<Duration>,
) -> Result<()> {
  let path = config.output.join("append-probe.sqlite3");
  let mut db = open(&path).await?;
  create_schema(&mut db).await?;
  for minute in 1..=config.repetitions as u64 {
    let started = Instant::now();
    let mut tx = db.begin().await?;
    insert_minute(&mut tx, config, minute).await?;
    tx.commit().await?;
    timings.push(started.elapsed());
  }
  checkpoint(&mut db).await?;
  db.close().await?;
  fs::remove_file(path)?;
  Ok(())
}

async fn insert_minute(
  tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
  config: &Config,
  minute: u64,
) -> Result<()> {
  let at = timestamp(minute)?;
  for slot in 0..config.processes_per_minute {
    let identity = (u64::from(slot) * 17 + minute / 31 + config.seed)
      % (u64::from(config.processes_per_minute) * 3);
    let name = match identity % 23 {
      0 => format!("worker\0{identity}"),
      1 => format!("描画-{identity}"),
      _ => format!("process-{identity:05}"),
    };
    let random = mix(config.seed ^ minute.rotate_left(7) ^ u64::from(slot));
    // Normal rows keep the current producer's f32/i32-shaped range. Separate
    // sparse sentinel rows exercise exact i64 and binary64 storage classes.
    let (cpu, memory, execution) = if minute == 0 && slot == 0 {
      (
        f64::from_bits(0x4009_21fb_5444_2d18),
        i64::MAX - 2052,
        i64::MAX - 17,
      )
    } else {
      (
        ((random & 0xffff) as f32 / 1_021.0) as f64,
        i64::from(((random >> 9) % (8 * 1024 * 1024)) as i32),
        i64::try_from((minute % 43_200).saturating_mul(60) + u64::from(slot))?,
      )
    };
    sqlx::query("INSERT INTO PROCESS_STATS (pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp) VALUES (?, ?, ?, ?, ?, ?)")
      .bind(100_i64 + i64::try_from(identity)?)
      .bind(name)
      .bind(cpu)
      .bind(memory)
      .bind(execution)
      .bind(&at)
      .execute(&mut **tx)
      .await?;
  }
  if minute.is_multiple_of(97) {
    sqlx::query("INSERT INTO PROCESS_STATS (pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp) VALUES (?, ?, ?, ?, ?, ?)")
      .bind(777_i64).bind("duplicate-identity").bind(-0.0_f64)
      .bind(i64::MIN + 2052).bind(i64::try_from(minute)?).bind(&at)
      .execute(&mut **tx).await?;
  }
  for source_index in 0..2_u64 {
    if (minute + source_index + config.seed).is_multiple_of(11) {
      continue;
    }
    let source = if source_index == 0 {
      "Desk"
    } else {
      "Living Room\0BLE"
    };
    let temperature = 18.0 + ((minute + source_index * 7) % 190) as f64 / 10.0;
    let humidity = if (minute + source_index).is_multiple_of(5) {
      None
    } else {
      Some(35.0 + (minute % 500) as f64 / 10.0)
    };
    sqlx::query("INSERT INTO AMBIENT_ARCHIVE (source, temperature, humidity, timestamp) VALUES (?, ?, ?, ?)")
      .bind(source).bind(temperature).bind(humidity).bind(&at)
      .execute(&mut **tx).await?;
  }
  Ok(())
}

fn mix(mut value: u64) -> u64 {
  value ^= value >> 30;
  value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
  value ^= value >> 27;
  value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
  value ^ (value >> 31)
}

fn timestamp(minute: u64) -> Result<String> {
  let millis = 1_767_225_600_000_i64
    .checked_add(
      i64::try_from(minute)?
        .checked_mul(60_000)
        .ok_or("timestamp overflow")?,
    )
    .ok_or("timestamp overflow")?;
  Ok(
    chrono::DateTime::from_timestamp_millis(millis)
      .ok_or("timestamp out of range")?
      .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
  )
}

fn parse_timestamp(value: &str) -> Result<i64> {
  Ok(chrono::DateTime::parse_from_rfc3339(value)?.timestamp_millis())
}

async fn checkpoint(db: &mut SqliteConnection) -> Result<()> {
  sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
    .execute(db)
    .await?;
  Ok(())
}

async fn scalar(db: &mut SqliteConnection, sql: &str) -> Result<i64> {
  Ok(sqlx::query_scalar(sql).fetch_one(db).await?)
}

fn family_table(family: &str) -> (&'static str, usize, usize) {
  match family {
    PROCESS => ("PROCESS_STATS", 7, 6),
    AMBIENT => ("AMBIENT_ARCHIVE", 5, 4),
    _ => unreachable!("known benchmark family"),
  }
}

async fn select_chunk(
  db: &mut SqliteConnection,
  family: &str,
  config: &Config,
) -> Result<Vec<Record>> {
  let (table, _, timestamp_index) = family_table(family);
  let newest: Option<String> =
    sqlx::query_scalar(&format!("SELECT MAX(timestamp) FROM {table}"))
      .fetch_one(&mut *db)
      .await?;
  let Some(newest) = newest else {
    return Ok(Vec::new());
  };
  let newest_ms = parse_timestamp(&newest)?;
  let tail_minutes = (config.chunk_minutes / 2).max(1);
  let cutoff = chrono::DateTime::from_timestamp_millis(
    newest_ms - i64::try_from(tail_minutes)?.saturating_mul(60_000),
  )
  .ok_or("invalid tail cutoff")?
  .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
  let first: Option<String> = sqlx::query_scalar(&format!(
    "SELECT MIN(timestamp) FROM {table} WHERE timestamp < ?"
  ))
  .bind(&cutoff)
  .fetch_one(&mut *db)
  .await?;
  let Some(first) = first else {
    return Ok(Vec::new());
  };
  let upper = chrono::DateTime::from_timestamp_millis(
    parse_timestamp(&first)?
      + i64::try_from(config.chunk_minutes)?.saturating_mul(60_000),
  )
  .ok_or("invalid chunk upper bound")?
  .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
  .min(cutoff);
  let rows = sqlx::query(&format!(
    "SELECT * FROM {table} WHERE timestamp >= ? AND timestamp < ? ORDER BY id LIMIT ?"
  ))
  .bind(&first)
  .bind(&upper)
  .bind(i64::try_from(config.chunk_rows)?)
  .fetch_all(&mut *db)
  .await?;
  let records = rows
    .iter()
    .map(|row| row_record(family, row))
    .collect::<Result<Vec<_>>>()?;
  if records
    .iter()
    .any(|record| record.get(timestamp_index).is_none())
  {
    return Err("record width mismatch".into());
  }
  Ok(records)
}

fn row_record(family: &str, row: &SqliteRow) -> Result<Record> {
  Ok(match family {
    PROCESS => vec![
      Value::Integer(row.try_get("id")?),
      Value::Integer(row.try_get("pid")?),
      Value::Text(row.try_get::<Vec<u8>, _>("process_name")?),
      Value::Real(row.try_get::<f64, _>("cpu_usage")?.to_bits()),
      Value::Integer(row.try_get("memory_usage")?),
      Value::Integer(row.try_get("execution_sec")?),
      Value::Text(row.try_get::<Vec<u8>, _>("timestamp")?),
    ],
    AMBIENT => vec![
      Value::Integer(row.try_get("id")?),
      Value::Text(row.try_get::<Vec<u8>, _>("source")?),
      Value::Real(row.try_get::<f64, _>("temperature")?.to_bits()),
      match row.try_get::<Option<f64>, _>("humidity")? {
        Some(value) => Value::Real(value.to_bits()),
        None => Value::Null,
      },
      Value::Text(row.try_get::<Vec<u8>, _>("timestamp")?),
    ],
    _ => return Err(format!("unknown family {family}").into()),
  })
}

fn integer(record: &Record, index: usize) -> Result<i64> {
  match record.get(index) {
    Some(Value::Integer(value)) => Ok(*value),
    _ => Err(format!("expected integer at column {index}").into()),
  }
}

fn real(record: &Record, index: usize) -> Result<f64> {
  match record.get(index) {
    Some(Value::Real(bits)) => Ok(f64::from_bits(*bits)),
    _ => Err(format!("expected real at column {index}").into()),
  }
}

fn text(record: &Record, index: usize) -> Result<&[u8]> {
  match record.get(index) {
    Some(Value::Text(value)) => Ok(value),
    _ => Err(format!("expected text at column {index}").into()),
  }
}

fn value_bytes(records: &[Record]) -> u64 {
  records
    .iter()
    .flat_map(|record| record.iter())
    .map(|value| match value {
      Value::Null => 1,
      Value::Integer(_) | Value::Real(_) => 9,
      Value::Text(bytes) | Value::Blob(bytes) => 9 + bytes.len() as u64,
    })
    .sum()
}

fn update_digest(digest: &mut Sha256, record: &Record) {
  digest.update((record.len() as u64).to_le_bytes());
  for value in record {
    match value {
      Value::Null => digest.update([0]),
      Value::Integer(value) => {
        digest.update([1]);
        digest.update(value.to_le_bytes());
      }
      Value::Real(bits) => {
        digest.update([2]);
        digest.update(bits.to_le_bytes());
      }
      Value::Text(bytes) => {
        digest.update([3]);
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
      }
      Value::Blob(bytes) => {
        digest.update([4]);
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
      }
    }
  }
}

fn records_digest(records: &[Record]) -> Vec<u8> {
  let mut digest = Sha256::new();
  for record in records {
    update_digest(&mut digest, record);
  }
  digest.finalize().to_vec()
}

async fn persist_chunk(
  db: &mut SqliteConnection,
  family: &str,
  records: &[Record],
  payload: &[u8],
  decoded_value_bytes: u64,
  config: &Config,
) -> Result<()> {
  let (table, _, timestamp_index) = family_table(family);
  let min_id = records
    .iter()
    .map(|record| integer(record, 0))
    .try_fold(i64::MAX, |current, value| {
      Ok::<_, Error>(current.min(value?))
    })?;
  let max_id = records
    .iter()
    .map(|record| integer(record, 0))
    .try_fold(i64::MIN, |current, value| {
      Ok::<_, Error>(current.max(value?))
    })?;
  let min_timestamp = records
    .iter()
    .map(|record| text(record, timestamp_index))
    .collect::<Result<Vec<_>>>()?
    .into_iter()
    .min()
    .ok_or("empty chunk")?
    .to_vec();
  let max_timestamp = records
    .iter()
    .map(|record| text(record, timestamp_index))
    .collect::<Result<Vec<_>>>()?
    .into_iter()
    .max()
    .ok_or("empty chunk")?
    .to_vec();
  let mut tx = db.begin().await?;
  sqlx::query("INSERT INTO ARCHIVE_CHUNKS (family, min_row_id, max_row_id, min_timestamp, max_timestamp, row_count, layout, compression, decoded_value_bytes, payload, digest) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
    .bind(family)
    .bind(min_id)
    .bind(max_id)
    .bind(String::from_utf8(min_timestamp)?)
    .bind(String::from_utf8(max_timestamp)?)
    .bind(i64::try_from(records.len())?)
    .bind(layout_name(config.layout))
    .bind(compression_name(config.compression))
    .bind(i64::try_from(decoded_value_bytes)?)
    .bind(payload)
    .bind(records_digest(records))
    .execute(&mut *tx)
    .await?;
  let placeholders = std::iter::repeat_n("?", records.len())
    .collect::<Vec<_>>()
    .join(",");
  let sql = format!("DELETE FROM {table} WHERE id IN ({placeholders})");
  let mut delete = sqlx::query(&sql);
  for record in records {
    delete = delete.bind(integer(record, 0)?);
  }
  let deleted = delete.execute(&mut *tx).await?.rows_affected();
  if deleted != records.len() as u64 {
    return Err(
      format!(
        "selected {} {family} rows but deleted {deleted}",
        records.len()
      )
      .into(),
    );
  }
  tx.commit().await?;
  Ok(())
}

async fn finalize_once(
  db: &mut SqliteConnection,
  family: &str,
  config: &Config,
) -> Result<usize> {
  let records = select_chunk(db, family, config).await?;
  if records.is_empty() {
    return Ok(0);
  }
  let payload =
    codec::encode(&records, config.layout, config.compression).map_err(codec_error)?;
  persist_chunk(
    db,
    family,
    &records,
    &payload,
    value_bytes(&records),
    config,
  )
  .await?;
  Ok(records.len())
}

async fn rollback_changed_selection(path: &Path, config: &Config) -> Result<bool> {
  let probe = path.with_file_name("rollback-probe.sqlite3");
  let mut db = open(&probe).await?;
  create_schema(&mut db).await?;
  create_chunk_schema(&mut db).await?;
  let mut tx = db.begin().await?;
  insert_minute(&mut tx, config, 0).await?;
  insert_minute(&mut tx, config, config.chunk_minutes * 2).await?;
  tx.commit().await?;
  let selected = select_chunk(&mut db, PROCESS, config).await?;
  let Some(first) = selected.first() else {
    return Err("rollback probe did not produce an eligible selection".into());
  };
  sqlx::query("DELETE FROM PROCESS_STATS WHERE id = ?")
    .bind(integer(first, 0)?)
    .execute(&mut db)
    .await?;
  let remaining_before = scalar(&mut db, "SELECT COUNT(*) FROM PROCESS_STATS").await?;
  let payload =
    codec::encode(&selected, config.layout, config.compression).map_err(codec_error)?;
  let failed = persist_chunk(
    &mut db,
    PROCESS,
    &selected,
    &payload,
    value_bytes(&selected),
    config,
  )
  .await
  .is_err();
  let result = failed
    && scalar(&mut db, "SELECT COUNT(*) FROM PROCESS_STATS").await? == remaining_before
    && scalar(&mut db, "SELECT COUNT(*) FROM ARCHIVE_CHUNKS").await? == 0;
  db.close().await?;
  fs::remove_file(probe)?;
  Ok(result)
}

async fn decode_chunk(
  row: &SqliteRow,
  timings: &mut Vec<Duration>,
) -> Result<Vec<Record>> {
  let payload: Vec<u8> = row.try_get("payload")?;
  let expected_digest: Vec<u8> = row.try_get("digest")?;
  let expected_count: i64 = row.try_get("row_count")?;
  let started = Instant::now();
  let records = codec::decode(&payload).map_err(codec_error)?;
  timings.push(started.elapsed());
  if records.len() != usize::try_from(expected_count)?
    || records_digest(&records) != expected_digest
  {
    return Err("persisted chunk count or digest mismatch".into());
  }
  Ok(records)
}

async fn compare_persisted(
  baseline: &mut SqliteConnection,
  candidate: &mut SqliteConnection,
  decode_timings: &mut Vec<Duration>,
) -> Result<bool> {
  sqlx::query(
    "CREATE TEMP TABLE IF NOT EXISTS verification_ids (
       family TEXT NOT NULL, id INTEGER NOT NULL, PRIMARY KEY (family, id)
     ) WITHOUT ROWID",
  )
  .execute(&mut *candidate)
  .await?;
  for family in [PROCESS, AMBIENT] {
    let (table, _, _) = family_table(family);
    sqlx::query("DELETE FROM temp.verification_ids WHERE family = ?")
      .bind(family)
      .execute(&mut *candidate)
      .await?;
    let mut candidate_tx = candidate.begin().await?;
    let mut chunk_cursor = 0_i64;
    let mut covered = 0_i64;
    loop {
      let row = sqlx::query("SELECT id, min_row_id, max_row_id, row_count, payload, digest FROM ARCHIVE_CHUNKS WHERE family = ? AND id > ? ORDER BY id LIMIT 1")
        .bind(family).bind(chunk_cursor).fetch_optional(&mut *candidate_tx).await?;
      let Some(row) = row else {
        break;
      };
      chunk_cursor = row.try_get("id")?;
      let records = decode_chunk(&row, decode_timings).await?;
      let placeholders = std::iter::repeat_n("?", records.len())
        .collect::<Vec<_>>()
        .join(",");
      let sql = format!("SELECT * FROM {table} WHERE id IN ({placeholders}) ORDER BY id");
      let mut query = sqlx::query(&sql);
      for record in &records {
        query = query.bind(integer(record, 0)?);
      }
      let source_rows = query.fetch_all(&mut *baseline).await?;
      let source = source_rows
        .iter()
        .map(|row| row_record(family, row))
        .collect::<Result<Vec<_>>>()?;
      if source != records {
        return Ok(false);
      }
      let values = std::iter::repeat_n("(?, ?)", records.len())
        .collect::<Vec<_>>()
        .join(",");
      let manifest_sql =
        format!("INSERT INTO temp.verification_ids (family, id) VALUES {values}");
      let mut manifest = sqlx::query(&manifest_sql);
      for record in &records {
        manifest = manifest.bind(family).bind(integer(record, 0)?);
      }
      if manifest.execute(&mut *candidate_tx).await.is_err() {
        return Ok(false);
      }
      covered += i64::try_from(records.len())?;
    }
    let mut tail_cursor = 0_i64;
    loop {
      let candidate_rows = sqlx::query(&format!(
        "SELECT * FROM {table} WHERE id > ? ORDER BY id LIMIT 4096"
      ))
      .bind(tail_cursor)
      .fetch_all(&mut *candidate_tx)
      .await?;
      if candidate_rows.is_empty() {
        break;
      }
      let candidate_records = candidate_rows
        .iter()
        .map(|row| row_record(family, row))
        .collect::<Result<Vec<_>>>()?;
      let ids = (
        integer(candidate_records.first().unwrap(), 0)?,
        integer(candidate_records.last().unwrap(), 0)?,
      );
      let placeholders = std::iter::repeat_n("?", candidate_records.len())
        .collect::<Vec<_>>()
        .join(",");
      let sql = format!("SELECT * FROM {table} WHERE id IN ({placeholders}) ORDER BY id");
      let mut query = sqlx::query(&sql);
      for record in &candidate_records {
        query = query.bind(integer(record, 0)?);
      }
      let source_rows = query.fetch_all(&mut *baseline).await?;
      let source = source_rows
        .iter()
        .map(|row| row_record(family, row))
        .collect::<Result<Vec<_>>>()?;
      if source != candidate_records {
        return Ok(false);
      }
      let values = std::iter::repeat_n("(?, ?)", candidate_records.len())
        .collect::<Vec<_>>()
        .join(",");
      let manifest_sql =
        format!("INSERT INTO temp.verification_ids (family, id) VALUES {values}");
      let mut manifest = sqlx::query(&manifest_sql);
      for record in &candidate_records {
        manifest = manifest.bind(family).bind(integer(record, 0)?);
      }
      if manifest.execute(&mut *candidate_tx).await.is_err() {
        return Ok(false);
      }
      tail_cursor = ids.1;
      covered += i64::try_from(candidate_records.len())?;
    }
    let source_count = scalar(baseline, &format!("SELECT COUNT(*) FROM {table}")).await?;
    let manifest_count: i64 =
      sqlx::query_scalar("SELECT COUNT(*) FROM temp.verification_ids WHERE family = ?")
        .bind(family)
        .fetch_one(&mut *candidate_tx)
        .await?;
    if covered != source_count || manifest_count != source_count {
      return Ok(false);
    }
    candidate_tx.commit().await?;
  }
  Ok(true)
}

async fn total_logical_rows(
  candidate: &mut SqliteConnection,
  timings: &mut Vec<Duration>,
) -> Result<i64> {
  let tail: i64 = sqlx::query_scalar(
    "SELECT (SELECT COUNT(*) FROM PROCESS_STATS) + (SELECT COUNT(*) FROM AMBIENT_ARCHIVE)",
  )
  .fetch_one(&mut *candidate)
  .await?;
  let chunk_rows: i64 =
    sqlx::query_scalar("SELECT COALESCE(SUM(row_count), 0) FROM ARCHIVE_CHUNKS")
      .fetch_one(&mut *candidate)
      .await?;
  // Callers run this only after the bounded per-chunk digest validation.
  let _ = timings;
  Ok(tail + chunk_rows)
}

#[derive(Clone, Debug)]
struct Aggregate {
  pid: i64,
  name: Vec<u8>,
  cpu_sum: f64,
  memory_sum: f64,
  count: u64,
  max_execution: i64,
  latest: Vec<u8>,
}

async fn process_oracle(
  db: &mut SqliteConnection,
  start: &str,
  end: &str,
) -> Result<Vec<Aggregate>> {
  let rows = sqlx::query(
    "SELECT pid, process_name, AVG(cpu_usage) avg_cpu, AVG(memory_usage) avg_memory,
            COUNT(*) sample_count, MAX(execution_sec) max_execution,
            MAX(timestamp) latest
     FROM PROCESS_STATS
     WHERE timestamp BETWEEN ? AND ?
     GROUP BY pid, process_name
     ORDER BY pid, process_name",
  )
  .bind(start)
  .bind(end)
  .fetch_all(db)
  .await?;
  rows
    .iter()
    .map(|row| {
      let count: i64 = row.try_get("sample_count")?;
      Ok(Aggregate {
        pid: row.try_get("pid")?,
        name: row.try_get::<Vec<u8>, _>("process_name")?,
        cpu_sum: row.try_get::<f64, _>("avg_cpu")? * count as f64,
        memory_sum: row.try_get::<f64, _>("avg_memory")? * count as f64,
        count: u64::try_from(count)?,
        max_execution: row.try_get("max_execution")?,
        latest: row.try_get::<Vec<u8>, _>("latest")?,
      })
    })
    .collect()
}

async fn process_chunked(
  db: &mut SqliteConnection,
  start: &str,
  end: &str,
  group_cap: usize,
  decode_timings: &mut Vec<Duration>,
) -> Result<Vec<Aggregate>> {
  process_chunked_interleaved(db, start, end, group_cap, decode_timings, None).await
}

async fn process_chunked_interleaved(
  db: &mut SqliteConnection,
  start: &str,
  end: &str,
  group_cap: usize,
  decode_timings: &mut Vec<Duration>,
  mut interleave: Option<(&mut SqliteConnection, &Config)>,
) -> Result<Vec<Aggregate>> {
  let mut tx = db.begin().await?;
  let mut groups = HashMap::new();
  let mut cursor = 0_i64;
  loop {
    let row = sqlx::query(PROCESS_CHUNK_LOOP_SQL)
      .bind(PROCESS)
      .bind(cursor)
      .bind(start)
      .bind(end)
      .fetch_optional(&mut *tx)
      .await?;
    let Some(row) = row else {
      break;
    };
    cursor = row.try_get("id")?;
    for record in decode_chunk(&row, decode_timings).await? {
      aggregate_process(
        &mut groups,
        &record,
        start.as_bytes(),
        end.as_bytes(),
        group_cap,
      )?;
    }
  }
  if let Some((writer, config)) = interleave.as_mut() {
    finalize_once(writer, PROCESS, config).await?;
  }
  let mut tail_cursor = 0_i64;
  loop {
    let rows = sqlx::query(
      "SELECT * FROM PROCESS_STATS
       WHERE timestamp BETWEEN ? AND ? AND id > ?
       ORDER BY id LIMIT 4096",
    )
    .bind(start)
    .bind(end)
    .bind(tail_cursor)
    .fetch_all(&mut *tx)
    .await?;
    if rows.is_empty() {
      break;
    }
    for row in &rows {
      let record = row_record(PROCESS, row)?;
      tail_cursor = integer(&record, 0)?;
      aggregate_process(
        &mut groups,
        &record,
        start.as_bytes(),
        end.as_bytes(),
        group_cap,
      )?;
    }
  }
  let mut aggregates: Vec<_> = groups.into_values().collect();
  aggregates.sort_by(|left, right| (left.pid, &left.name).cmp(&(right.pid, &right.name)));
  tx.commit().await?;
  Ok(aggregates)
}

fn aggregate_process(
  groups: &mut HashMap<(i64, Vec<u8>), Aggregate>,
  record: &Record,
  start: &[u8],
  end: &[u8],
  group_cap: usize,
) -> Result<()> {
  let at = text(record, 6)?;
  if at < start || at > end {
    return Ok(());
  }
  let pid = integer(record, 1)?;
  let name = text(record, 2)?.to_vec();
  if !groups.contains_key(&(pid, name.clone())) && groups.len() == group_cap {
    return Err(format!(
      "Process Stats query exceeded explicit group cap {group_cap}; narrow the range or raise --group-cap"
    )
    .into());
  }
  let aggregate = groups
    .entry((pid, name.clone()))
    .or_insert_with(|| Aggregate {
      pid,
      name,
      cpu_sum: 0.0,
      memory_sum: 0.0,
      count: 0,
      max_execution: i64::MIN,
      latest: Vec::new(),
    });
  aggregate.cpu_sum += real(record, 3)?;
  aggregate.memory_sum += integer(record, 4)? as f64;
  aggregate.count += 1;
  aggregate.max_execution = aggregate.max_execution.max(integer(record, 5)?);
  if at > aggregate.latest.as_slice() {
    aggregate.latest = at.to_vec();
  }
  Ok(())
}

fn float_equal(reference: f64, actual: f64) -> bool {
  if !reference.is_finite() || !actual.is_finite() {
    return false;
  }
  if reference.to_bits() == actual.to_bits() {
    return true;
  }
  (actual - reference).abs() <= (1e-9_f64).max(1e-12 * reference.abs())
}

fn compare_aggregates(reference: &[Aggregate], actual: &[Aggregate]) -> bool {
  reference.len() == actual.len()
    && reference.iter().zip(actual).all(|(left, right)| {
      left.pid == right.pid
        && left.name == right.name
        && left.count == right.count
        && left.max_execution == right.max_execution
        && left.latest == right.latest
        && float_equal(
          left.cpu_sum / left.count as f64,
          right.cpu_sum / right.count as f64,
        )
        && float_equal(
          left.memory_sum / left.count as f64,
          right.memory_sum / right.count as f64,
        )
    })
}

async fn ambient_oracle(
  db: &mut SqliteConnection,
  start_ms: i64,
  end_ms: i64,
) -> Result<(Vec<u8>, u64)> {
  let mut digest = Sha256::new();
  let mut count = 0_u64;
  let mut cursor = 0_i64;
  loop {
    let rows = sqlx::query(&format!(
      "SELECT * FROM AMBIENT_ARCHIVE
       WHERE {EPOCH_MS_SQL} BETWEEN ? AND ? AND id > ?
       ORDER BY id LIMIT 4096"
    ))
    .bind(start_ms)
    .bind(end_ms)
    .bind(cursor)
    .fetch_all(&mut *db)
    .await?;
    if rows.is_empty() {
      break;
    }
    for row in &rows {
      let record = row_record(AMBIENT, row)?;
      cursor = integer(&record, 0)?;
      digest.update(record_digest(&record));
      count += 1;
    }
  }
  Ok((digest.finalize().to_vec(), count))
}

async fn ambient_chunked(
  db: &mut SqliteConnection,
  start_ms: i64,
  end_ms: i64,
  decode_timings: &mut Vec<Duration>,
) -> Result<(Vec<u8>, u64)> {
  // TEXT catalog bounds cannot prove membership for every valid timestamp
  // spelling, so the experimental raw endpoint conservatively scans chunks.
  sqlx::query(
    "CREATE TEMP TABLE IF NOT EXISTS ambient_query_records (
       id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL, record_digest BLOB NOT NULL
     )",
  )
  .execute(&mut *db)
  .await?;
  sqlx::query("DELETE FROM temp.ambient_query_records")
    .execute(&mut *db)
    .await?;
  let mut tx = db.begin().await?;
  let mut chunk_cursor = 0_i64;
  loop {
    let row = sqlx::query(AMBIENT_CHUNK_LOOP_SQL)
      .bind(AMBIENT)
      .bind(chunk_cursor)
      .fetch_optional(&mut *tx)
      .await?;
    let Some(row) = row else {
      break;
    };
    chunk_cursor = row.try_get("id")?;
    for record in decode_chunk(&row, decode_timings).await? {
      sqlx::query(
        "INSERT INTO temp.ambient_query_records (id, timestamp, record_digest)
         VALUES (?, ?, ?)",
      )
      .bind(integer(&record, 0)?)
      .bind(String::from_utf8(text(&record, 4)?.to_vec())?)
      .bind(record_digest(&record))
      .execute(&mut *tx)
      .await?;
    }
  }
  let mut tail_cursor = 0_i64;
  loop {
    let rows = sqlx::query(&format!(
      "SELECT * FROM AMBIENT_ARCHIVE
       WHERE {EPOCH_MS_SQL} BETWEEN ? AND ? AND id > ?
       ORDER BY id LIMIT 4096"
    ))
    .bind(start_ms)
    .bind(end_ms)
    .bind(tail_cursor)
    .fetch_all(&mut *tx)
    .await?;
    if rows.is_empty() {
      break;
    }
    for row in &rows {
      let record = row_record(AMBIENT, row)?;
      tail_cursor = integer(&record, 0)?;
      sqlx::query(
        "INSERT INTO temp.ambient_query_records (id, timestamp, record_digest)
         VALUES (?, ?, ?)",
      )
      .bind(integer(&record, 0)?)
      .bind(String::from_utf8(text(&record, 4)?.to_vec())?)
      .bind(record_digest(&record))
      .execute(&mut *tx)
      .await?;
    }
  }
  let mut digest = Sha256::new();
  let mut count = 0_u64;
  let mut cursor = 0_i64;
  loop {
    let rows = sqlx::query(&format!(
      "SELECT id, record_digest FROM temp.ambient_query_records
       WHERE {EPOCH_MS_SQL} BETWEEN ? AND ? AND id > ?
       ORDER BY id LIMIT 4096"
    ))
    .bind(start_ms)
    .bind(end_ms)
    .bind(cursor)
    .fetch_all(&mut *tx)
    .await?;
    if rows.is_empty() {
      break;
    }
    for row in rows {
      cursor = row.try_get("id")?;
      digest.update(row.try_get::<Vec<u8>, _>("record_digest")?);
      count += 1;
    }
  }
  tx.commit().await?;
  Ok((digest.finalize().to_vec(), count))
}

fn record_digest(record: &Record) -> Vec<u8> {
  let mut digest = Sha256::new();
  update_digest(&mut digest, record);
  digest.finalize().to_vec()
}

async fn concurrent_snapshot(path: &Path, config: &Config) -> Result<bool> {
  let source_path = path.with_file_name("snapshot-source.sqlite3");
  let candidate_path = path.with_file_name("snapshot-candidate.sqlite3");
  let mut source = open(&source_path).await?;
  create_schema(&mut source).await?;
  let mut sampled = 0;
  while sampled <= config.chunk_minutes * 2 {
    let mut tx = source.begin().await?;
    let end = (sampled + 64).min(config.chunk_minutes * 2 + 1);
    while sampled < end {
      insert_minute(&mut tx, config, sampled).await?;
      sampled += 1;
    }
    tx.commit().await?;
  }
  checkpoint(&mut source).await?;
  fs::copy(&source_path, &candidate_path)?;
  let mut writer = open(&candidate_path).await?;
  create_chunk_schema(&mut writer).await?;
  if finalize_once(&mut writer, PROCESS, config).await? == 0 {
    return Err("snapshot probe could not create its initial chunk".into());
  }
  let mut reader = open(&candidate_path).await?;
  let start = timestamp(0)?;
  let end = timestamp(config.chunk_minutes * 2)?;
  let reference = process_oracle(&mut source, &start, &end).await?;
  let mut timings = Vec::new();
  let actual = process_chunked_interleaved(
    &mut reader,
    &start,
    &end,
    config.group_cap,
    &mut timings,
    Some((&mut writer, config)),
  )
  .await?;
  let result = compare_aggregates(&reference, &actual);
  reader.close().await?;
  writer.close().await?;
  source.close().await?;
  fs::remove_file(source_path)?;
  fs::remove_file(candidate_path)?;
  Ok(result)
}

async fn footprint(db: &mut SqliteConnection, path: &Path) -> Result<FileFootprint> {
  let page_size = scalar(db, "PRAGMA page_size").await?;
  let page_count = scalar(db, "PRAGMA page_count").await?;
  let freelist_pages = scalar(db, "PRAGMA freelist_count").await?;
  let index_bytes = sqlx::query_scalar::<_, Option<i64>>(
    "SELECT SUM(pgsize) FROM dbstat
     WHERE name IN (SELECT name FROM sqlite_schema WHERE type = 'index')",
  )
  .fetch_one(&mut *db)
  .await
  .ok()
  .flatten();
  Ok(FileFootprint {
    database_bytes: file_size(path),
    wal_bytes: file_size(&PathBuf::from(format!("{}-wal", path.display()))),
    shm_bytes: file_size(&PathBuf::from(format!("{}-shm", path.display()))),
    page_size,
    page_count,
    freelist_pages,
    allocated_page_bytes: page_size.saturating_mul(page_count),
    live_page_bytes: page_size.saturating_mul(page_count - freelist_pages),
    index_bytes,
  })
}

fn file_size(path: &Path) -> u64 {
  fs::metadata(path)
    .map(|metadata| metadata.len())
    .unwrap_or(0)
}

#[cfg(test)]
#[path = "database_contract_tests.rs"]
mod contract_tests;

#[cfg(test)]
mod tests {
  use super::*;

  fn assert_family_id_keyset_plan(details: &[String]) {
    let normalized = details.join(" ").to_ascii_lowercase().replace(' ', "");
    assert!(
      normalized.contains("idx_archive_chunks_family_id")
        && normalized.contains("family=?")
        && normalized.contains("id>?"),
      "expected family/id keyset index, got {details:?}"
    );
    assert!(
      !normalized.contains("tempb-tree"),
      "keyset loop must not sort through a temporary B-tree: {details:?}"
    );
  }

  fn config(output: PathBuf) -> Config {
    Config {
      output,
      minutes: 180,
      processes_per_minute: 5,
      chunk_minutes: 30,
      chunk_rows: 101,
      repetitions: 2,
      layout: Layout::Columnar,
      compression: Compression::Deflate,
      seed: 9,
      duty_cycle: 1,
      group_cap: 10_000,
    }
  }

  #[tokio::test]
  async fn finalization_preserves_partial_chunks_tail_and_queries() {
    let temp = tempfile::tempdir().unwrap();
    let mut benchmark = Benchmark::create(config(temp.path().to_path_buf()))
      .await
      .unwrap();
    benchmark.seed().await.unwrap();
    benchmark.finalize().await.unwrap();
    benchmark.verify_and_query().await.unwrap();
    assert!(benchmark.finalized.chunks > 2);
    assert!(benchmark.correctness.exact_decoder_check);
    assert!(benchmark.correctness.persisted_reopen_exact_records);
    assert!(benchmark.correctness.precommit_changed_selection_rollback);
    assert!(benchmark.correctness.postcommit_retry_exact_multiplicity);
    assert!(benchmark.correctness.concurrent_snapshot_before_or_after);
    assert!(benchmark.correctness.process_query_equivalent);
    assert!(benchmark.correctness.ambient_raw_range_equivalent);
  }

  #[tokio::test]
  async fn process_and_ambient_chunk_loops_use_family_id_keyset_index() {
    let temp = tempfile::tempdir().unwrap();
    let mut db = open(&temp.path().join("plan.sqlite3")).await.unwrap();
    create_schema(&mut db).await.unwrap();
    create_chunk_schema(&mut db).await.unwrap();

    let process_plan: Vec<String> =
      sqlx::query(&format!("EXPLAIN QUERY PLAN {PROCESS_CHUNK_LOOP_SQL}"))
        .bind(PROCESS)
        .bind(0_i64)
        .bind("2026-01-01T00:00:00.000Z")
        .bind("2026-01-02T00:00:00.000Z")
        .fetch_all(&mut db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("detail"))
        .collect();
    assert_family_id_keyset_plan(&process_plan);

    let ambient_plan: Vec<String> =
      sqlx::query(&format!("EXPLAIN QUERY PLAN {AMBIENT_CHUNK_LOOP_SQL}"))
        .bind(AMBIENT)
        .bind(0_i64)
        .fetch_all(&mut db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("detail"))
        .collect();
    assert_family_id_keyset_plan(&ambient_plan);
  }

  #[test]
  fn derived_float_comparison_uses_the_ratified_tolerance() {
    assert!(float_equal(10_000.0, 10_000.0 + 5e-9));
    assert!(!float_equal(1.0, 1.0 + 1e-6));
    assert!(!float_equal(f64::NAN, f64::NAN));
  }
}
