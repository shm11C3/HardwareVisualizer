//! Query-only Ambient chunk bounds experiment for issue #2052.
//!
//! The catalog is built after finalization and deliberately has no production
//! lifecycle or integrity claim. Missing, stale, or indeterminate metadata is
//! conservative: the corresponding readable chunk remains a query candidate.

use super::*;
use sqlx::QueryBuilder;

const BOUNDS_TABLE: &str = "AMBIENT_CHUNK_EPOCH_BOUNDS";
const INSERT_BATCH_ROWS: usize = 256;
const OUTPUT_BATCH_ROWS: i64 = 4_096;
const FIRST_PAGE_ROWS: i64 = 100;
const CANDIDATE_CHUNK_SQL: &str = "SELECT c.id, c.row_count, c.payload, c.digest
   FROM ARCHIVE_CHUNKS c
   LEFT JOIN AMBIENT_CHUNK_EPOCH_BOUNDS b ON b.chunk_id = c.id
   WHERE c.family = ? AND c.id > ?
     AND (b.chunk_id IS NULL OR b.chunk_digest != c.digest
          OR b.has_unknown = 1
          OR b.min_epoch_ms IS NULL OR b.max_epoch_ms IS NULL
          OR (b.max_epoch_ms >= ? AND b.min_epoch_ms <= ?))
   ORDER BY c.id LIMIT 1";

#[derive(Clone, Debug, Serialize)]
pub(super) struct BuildReport {
  pub(super) chunks: u64,
  pub(super) decoded_rows: u64,
  pub(super) unknown_bounds_chunks: u64,
  pub(super) build_ms: f64,
  pub(super) sql_ms: f64,
  pub(super) decode_ms: f64,
  pub(super) table_bytes: Option<i64>,
  pub(super) index_bytes: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct QueryResult {
  pub(super) digest: Vec<u8>,
  pub(super) count: u64,
  pub(super) total_chunks: u64,
  pub(super) candidate_chunks: u64,
  pub(super) candidate_chunk_rows: u64,
  pub(super) decoded_chunks: u64,
  pub(super) decoded_chunk_rows: u64,
  pub(super) tail_rows: u64,
  pub(super) materialized_rows: u64,
  pub(super) first_page_rows: u64,
  pub(super) sql_ms: f64,
  pub(super) decode_ms: f64,
  pub(super) materialization_ms: f64,
  pub(super) output_ms: f64,
  pub(super) first_page_ms: f64,
  pub(super) total_ms: f64,
  pub(super) temp_store: i64,
}

#[derive(Debug)]
struct ChunkBound {
  chunk_id: i64,
  chunk_digest: Vec<u8>,
  min_epoch_ms: Option<i64>,
  max_epoch_ms: Option<i64>,
  has_unknown: bool,
  row_count: i64,
}

enum TimestampValue {
  Null,
  Integer(i64),
  Real(f64),
  Text(String),
  Blob(Vec<u8>),
}

struct StagedRecord {
  id: i64,
  timestamp: TimestampValue,
  digest: Vec<u8>,
}

pub(super) async fn build(db: &mut SqliteConnection) -> Result<BuildReport> {
  let build_started = Instant::now();
  let mut sql_elapsed = Duration::ZERO;
  let mut decode_elapsed = Duration::ZERO;
  let mut chunks = 0_u64;
  let mut decoded_rows = 0_u64;
  let mut unknown_bounds_chunks = 0_u64;

  let sql_started = Instant::now();
  let mut tx = db.begin().await?;
  sqlx::query(&format!("DROP TABLE IF EXISTS {BOUNDS_TABLE}"))
    .execute(&mut *tx)
    .await?;
  sqlx::query(&format!(
    "CREATE TABLE {BOUNDS_TABLE} (
       chunk_id INTEGER PRIMARY KEY,
       chunk_digest BLOB NOT NULL,
       min_epoch_ms INTEGER,
       max_epoch_ms INTEGER,
       has_unknown INTEGER NOT NULL CHECK (has_unknown IN (0, 1)),
       row_count INTEGER NOT NULL
     )"
  ))
  .execute(&mut *tx)
  .await?;
  sqlx::query(
    "CREATE TEMP TABLE IF NOT EXISTS ambient_bound_timestamps (
       ordinal INTEGER PRIMARY KEY, timestamp DATETIME
     )",
  )
  .execute(&mut *tx)
  .await?;
  sql_elapsed += sql_started.elapsed();

  let mut chunk_cursor = 0_i64;
  let mut pending = Vec::with_capacity(INSERT_BATCH_ROWS);
  loop {
    let sql_started = Instant::now();
    let row = sqlx::query(
      "SELECT id, row_count, payload, digest FROM ARCHIVE_CHUNKS
       WHERE family = ? AND id > ? ORDER BY id LIMIT 1",
    )
    .bind(AMBIENT)
    .bind(chunk_cursor)
    .fetch_optional(&mut *tx)
    .await?;
    sql_elapsed += sql_started.elapsed();
    let Some(row) = row else {
      break;
    };
    chunk_cursor = row.try_get("id")?;
    let expected_digest: Vec<u8> = row.try_get("digest")?;

    let decode_started = Instant::now();
    let records = decode_chunk(&row, &mut Vec::new()).await?;
    decode_elapsed += decode_started.elapsed();

    let sql_started = Instant::now();
    sqlx::query("DELETE FROM temp.ambient_bound_timestamps")
      .execute(&mut *tx)
      .await?;
    insert_timestamps(&mut tx, &records).await?;
    let bounds = sqlx::query(&format!(
      "SELECT MIN({EPOCH_MS_SQL}) AS min_epoch_ms,
              MAX({EPOCH_MS_SQL}) AS max_epoch_ms,
              COALESCE(SUM(CASE WHEN {EPOCH_MS_SQL} IS NULL THEN 1 ELSE 0 END), 0)
                AS unknown_count
       FROM temp.ambient_bound_timestamps"
    ))
    .fetch_one(&mut *tx)
    .await?;
    sql_elapsed += sql_started.elapsed();

    let has_unknown = bounds.try_get::<i64, _>("unknown_count")? != 0;
    chunks = chunks.checked_add(1).ok_or("chunk count overflow")?;
    decoded_rows = decoded_rows
      .checked_add(u64::try_from(records.len())?)
      .ok_or("decoded row count overflow")?;
    if has_unknown {
      unknown_bounds_chunks = unknown_bounds_chunks
        .checked_add(1)
        .ok_or("unknown bounds count overflow")?;
    }
    pending.push(ChunkBound {
      chunk_id: chunk_cursor,
      chunk_digest: expected_digest,
      min_epoch_ms: bounds.try_get("min_epoch_ms")?,
      max_epoch_ms: bounds.try_get("max_epoch_ms")?,
      has_unknown,
      row_count: i64::try_from(records.len())?,
    });
    if pending.len() == INSERT_BATCH_ROWS {
      let sql_started = Instant::now();
      insert_bounds(&mut tx, &pending).await?;
      sql_elapsed += sql_started.elapsed();
      pending.clear();
    }
  }
  if !pending.is_empty() {
    let sql_started = Instant::now();
    insert_bounds(&mut tx, &pending).await?;
    sql_elapsed += sql_started.elapsed();
  }

  let sql_started = Instant::now();
  tx.commit().await?;
  sql_elapsed += sql_started.elapsed();

  Ok(BuildReport {
    chunks,
    decoded_rows,
    unknown_bounds_chunks,
    build_ms: milliseconds(build_started.elapsed()),
    sql_ms: milliseconds(sql_elapsed),
    decode_ms: milliseconds(decode_elapsed),
    table_bytes: dbstat_bytes(db, BOUNDS_TABLE).await,
    // The candidate loop is ordered by chunk id. A separate range index is
    // unused for that plan; the INTEGER PRIMARY KEY lookup is the only
    // metadata index cost.
    index_bytes: Some(0),
  })
}

pub(super) async fn query(
  db: &mut SqliteConnection,
  start_ms: i64,
  end_ms: i64,
) -> Result<QueryResult> {
  let total_started = Instant::now();
  let mut sql_elapsed = Duration::ZERO;
  let mut decode_elapsed = Duration::ZERO;
  let mut materialization_elapsed = Duration::ZERO;
  let mut output_elapsed = Duration::ZERO;
  let mut candidate_chunks = 0_u64;
  let mut candidate_chunk_rows = 0_u64;
  let mut decoded_chunks = 0_u64;
  let mut decoded_chunk_rows = 0_u64;
  let mut tail_rows = 0_u64;
  let mut materialized_rows = 0_u64;

  let sql_started = Instant::now();
  let temp_store: i64 = sqlx::query_scalar("PRAGMA temp_store")
    .fetch_one(&mut *db)
    .await?;
  let mut tx = db.begin().await?;
  let total_chunks: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM ARCHIVE_CHUNKS WHERE family = ?")
      .bind(AMBIENT)
      .fetch_one(&mut *tx)
      .await?;
  sql_elapsed += sql_started.elapsed();

  let materialization_started = Instant::now();
  sqlx::query(
    "CREATE TEMP TABLE IF NOT EXISTS ambient_bounds_stage (
       id INTEGER NOT NULL, timestamp DATETIME, record_digest BLOB NOT NULL
     )",
  )
  .execute(&mut *tx)
  .await?;
  sqlx::query(
    "CREATE TEMP TABLE IF NOT EXISTS ambient_bounds_query_records (
       id INTEGER PRIMARY KEY, record_digest BLOB NOT NULL
     )",
  )
  .execute(&mut *tx)
  .await?;
  sqlx::query("DELETE FROM temp.ambient_bounds_stage")
    .execute(&mut *tx)
    .await?;
  sqlx::query("DELETE FROM temp.ambient_bounds_query_records")
    .execute(&mut *tx)
    .await?;
  materialization_elapsed += materialization_started.elapsed();

  let mut chunk_cursor = 0_i64;
  loop {
    let sql_started = Instant::now();
    let row = sqlx::query(CANDIDATE_CHUNK_SQL)
      .bind(AMBIENT)
      .bind(chunk_cursor)
      .bind(start_ms)
      .bind(end_ms)
      .fetch_optional(&mut *tx)
      .await?;
    sql_elapsed += sql_started.elapsed();
    let Some(row) = row else {
      break;
    };
    chunk_cursor = row.try_get("id")?;
    let declared_rows: i64 = row.try_get("row_count")?;
    candidate_chunks += 1;
    candidate_chunk_rows = candidate_chunk_rows
      .checked_add(u64::try_from(declared_rows)?)
      .ok_or("candidate row count overflow")?;

    let decode_started = Instant::now();
    let records = decode_chunk(&row, &mut Vec::new()).await?;
    decode_elapsed += decode_started.elapsed();
    decoded_chunks += 1;
    decoded_chunk_rows = decoded_chunk_rows
      .checked_add(u64::try_from(records.len())?)
      .ok_or("decoded row count overflow")?;

    let materialization_started = Instant::now();
    sqlx::query("DELETE FROM temp.ambient_bounds_stage")
      .execute(&mut *tx)
      .await?;
    insert_stage_records(&mut tx, &records).await?;
    let inserted = sqlx::query(&format!(
      "INSERT INTO temp.ambient_bounds_query_records (id, record_digest)
       SELECT id, record_digest FROM temp.ambient_bounds_stage
       WHERE {EPOCH_MS_SQL} BETWEEN ? AND ?"
    ))
    .bind(start_ms)
    .bind(end_ms)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    materialized_rows = materialized_rows
      .checked_add(inserted)
      .ok_or("materialized row count overflow")?;
    materialization_elapsed += materialization_started.elapsed();
  }

  let mut tail_cursor: Option<i64> = None;
  loop {
    let sql_started = Instant::now();
    let rows = if let Some(cursor) = tail_cursor {
      sqlx::query(&format!(
        "SELECT * FROM AMBIENT_ARCHIVE
         WHERE {EPOCH_MS_SQL} BETWEEN ? AND ? AND id > ?
         ORDER BY id LIMIT {OUTPUT_BATCH_ROWS}"
      ))
      .bind(start_ms)
      .bind(end_ms)
      .bind(cursor)
      .fetch_all(&mut *tx)
      .await?
    } else {
      sqlx::query(&format!(
        "SELECT * FROM AMBIENT_ARCHIVE
         WHERE {EPOCH_MS_SQL} BETWEEN ? AND ?
         ORDER BY id LIMIT {OUTPUT_BATCH_ROWS}"
      ))
      .bind(start_ms)
      .bind(end_ms)
      .fetch_all(&mut *tx)
      .await?
    };
    sql_elapsed += sql_started.elapsed();
    if rows.is_empty() {
      break;
    }
    let records = rows
      .iter()
      .map(|row| row_record(AMBIENT, row))
      .collect::<Result<Vec<_>>>()?;
    tail_cursor = Some(integer(records.last().unwrap(), 0)?);
    let materialization_started = Instant::now();
    insert_result_records(&mut tx, &records).await?;
    let inserted = u64::try_from(records.len())?;
    tail_rows += inserted;
    materialized_rows += inserted;
    materialization_elapsed += materialization_started.elapsed();
  }

  let output_started = Instant::now();
  let first_page = sqlx::query(
    "SELECT id, record_digest FROM temp.ambient_bounds_query_records
     ORDER BY id LIMIT ?",
  )
  .bind(FIRST_PAGE_ROWS)
  .fetch_all(&mut *tx)
  .await?;
  let first_page_ms = milliseconds(total_started.elapsed());

  let mut digest = Sha256::new();
  let mut count = 0_u64;
  let mut output_cursor: Option<i64> = None;
  loop {
    let rows = if let Some(cursor) = output_cursor {
      sqlx::query(
        "SELECT id, record_digest FROM temp.ambient_bounds_query_records
         WHERE id > ? ORDER BY id LIMIT ?",
      )
      .bind(cursor)
      .bind(OUTPUT_BATCH_ROWS)
      .fetch_all(&mut *tx)
      .await?
    } else {
      sqlx::query(
        "SELECT id, record_digest FROM temp.ambient_bounds_query_records
         ORDER BY id LIMIT ?",
      )
      .bind(OUTPUT_BATCH_ROWS)
      .fetch_all(&mut *tx)
      .await?
    };
    if rows.is_empty() {
      break;
    }
    for row in rows {
      output_cursor = Some(row.try_get("id")?);
      digest.update(row.try_get::<Vec<u8>, _>("record_digest")?);
      count += 1;
    }
  }
  output_elapsed += output_started.elapsed();
  tx.commit().await?;
  if count != materialized_rows {
    return Err("Ambient materialized and output row counts differ".into());
  }

  Ok(QueryResult {
    digest: digest.finalize().to_vec(),
    count,
    total_chunks: u64::try_from(total_chunks)?,
    candidate_chunks,
    candidate_chunk_rows,
    decoded_chunks,
    decoded_chunk_rows,
    tail_rows,
    materialized_rows,
    first_page_rows: u64::try_from(first_page.len())?,
    sql_ms: milliseconds(sql_elapsed),
    decode_ms: milliseconds(decode_elapsed),
    materialization_ms: milliseconds(materialization_elapsed),
    output_ms: milliseconds(output_elapsed),
    first_page_ms,
    total_ms: milliseconds(total_started.elapsed()),
    temp_store,
  })
}

async fn insert_timestamps(
  tx: &mut Transaction<'_, Sqlite>,
  records: &[Record],
) -> Result<()> {
  let timestamps = records
    .iter()
    .map(timestamp_value)
    .collect::<Result<Vec<_>>>()?;
  for (batch_index, batch) in timestamps.chunks(INSERT_BATCH_ROWS).enumerate() {
    let mut builder: QueryBuilder<'_, Sqlite> = QueryBuilder::new(
      "INSERT INTO temp.ambient_bound_timestamps (ordinal, timestamp) ",
    );
    builder.push_values(
      batch.iter().enumerate(),
      |mut values, (index, timestamp)| {
        values.push_bind((batch_index * INSERT_BATCH_ROWS + index) as i64);
        push_timestamp(&mut values, timestamp);
      },
    );
    builder.build().execute(&mut **tx).await?;
  }
  Ok(())
}

async fn insert_stage_records(
  tx: &mut Transaction<'_, Sqlite>,
  records: &[Record],
) -> Result<()> {
  let staged = records
    .iter()
    .map(|record| {
      Ok::<_, Error>(StagedRecord {
        id: integer(record, 0)?,
        timestamp: timestamp_value(record)?,
        digest: record_digest(record),
      })
    })
    .collect::<Result<Vec<_>>>()?;
  for batch in staged.chunks(INSERT_BATCH_ROWS) {
    let mut builder: QueryBuilder<'_, Sqlite> = QueryBuilder::new(
      "INSERT INTO temp.ambient_bounds_stage (id, timestamp, record_digest) ",
    );
    builder.push_values(batch, |mut values, record| {
      values.push_bind(record.id);
      push_timestamp(&mut values, &record.timestamp);
      values.push_bind(&record.digest);
    });
    builder.build().execute(&mut **tx).await?;
  }
  Ok(())
}

fn push_timestamp<'args>(
  values: &mut sqlx::query_builder::Separated<'_, 'args, Sqlite, &'static str>,
  timestamp: &'args TimestampValue,
) {
  match timestamp {
    TimestampValue::Null => values.push_bind(Option::<i64>::None),
    TimestampValue::Integer(value) => values.push_bind(*value),
    TimestampValue::Real(value) => values.push_bind(*value),
    TimestampValue::Text(value) => values.push_bind(value),
    TimestampValue::Blob(value) => values.push_bind(value),
  };
}

fn timestamp_value(record: &Record) -> Result<TimestampValue> {
  Ok(
    match record.get(4).ok_or("Ambient record has no timestamp")? {
      Value::Null => TimestampValue::Null,
      Value::Integer(value) => TimestampValue::Integer(*value),
      Value::Real(bits) => TimestampValue::Real(f64::from_bits(*bits)),
      Value::Text(bytes) => TimestampValue::Text(String::from_utf8(bytes.clone())?),
      Value::Blob(bytes) => TimestampValue::Blob(bytes.clone()),
    },
  )
}

async fn insert_result_records(
  tx: &mut Transaction<'_, Sqlite>,
  records: &[Record],
) -> Result<()> {
  let staged = records
    .iter()
    .map(|record| Ok::<_, Error>((integer(record, 0)?, record_digest(record))))
    .collect::<Result<Vec<_>>>()?;
  for batch in staged.chunks(INSERT_BATCH_ROWS) {
    let mut builder: QueryBuilder<'_, Sqlite> = QueryBuilder::new(
      "INSERT INTO temp.ambient_bounds_query_records (id, record_digest) ",
    );
    builder.push_values(batch, |mut values, (id, digest)| {
      values.push_bind(*id);
      values.push_bind(digest);
    });
    builder.build().execute(&mut **tx).await?;
  }
  Ok(())
}

async fn insert_bounds(
  tx: &mut Transaction<'_, Sqlite>,
  bounds: &[ChunkBound],
) -> Result<()> {
  let mut builder: QueryBuilder<'_, Sqlite> = QueryBuilder::new(&format!(
    "INSERT INTO {BOUNDS_TABLE}
     (chunk_id, chunk_digest, min_epoch_ms, max_epoch_ms, has_unknown, row_count) "
  ));
  builder.push_values(bounds, |mut values, bound| {
    values
      .push_bind(bound.chunk_id)
      .push_bind(&bound.chunk_digest)
      .push_bind(bound.min_epoch_ms)
      .push_bind(bound.max_epoch_ms)
      .push_bind(i64::from(bound.has_unknown))
      .push_bind(bound.row_count);
  });
  builder.build().execute(&mut **tx).await?;
  Ok(())
}

async fn dbstat_bytes(db: &mut SqliteConnection, name: &str) -> Option<i64> {
  sqlx::query_scalar::<_, Option<i64>>("SELECT SUM(pgsize) FROM dbstat WHERE name = ?")
    .bind(name)
    .fetch_one(db)
    .await
    .ok()
    .flatten()
}

fn milliseconds(duration: Duration) -> f64 {
  duration.as_secs_f64() * 1_000.0
}

#[cfg(test)]
mod tests {
  use super::*;

  fn config(output: PathBuf) -> Config {
    Config {
      output,
      minutes: 1,
      processes_per_minute: 1,
      chunk_minutes: 1,
      chunk_rows: 64,
      repetitions: 1,
      layout: Layout::Columnar,
      compression: Compression::None,
      seed: 2052,
      duty_cycle: 1,
      group_cap: 64,
      query_experiment: false,
      process_workload: ProcessWorkload::Stable,
      process_lifetime_minutes: 30,
    }
  }

  async fn databases(
    temp: &tempfile::TempDir,
  ) -> (SqliteConnection, SqliteConnection, Config) {
    let config = config(temp.path().to_path_buf());
    let mut baseline = open(&temp.path().join("ambient-bounds-baseline.sqlite3"))
      .await
      .unwrap();
    let mut candidate = open(&temp.path().join("ambient-bounds-candidate.sqlite3"))
      .await
      .unwrap();
    create_schema(&mut baseline).await.unwrap();
    create_schema(&mut candidate).await.unwrap();
    create_chunk_schema(&mut candidate).await.unwrap();
    (baseline, candidate, config)
  }

  async fn insert_ambient(
    baseline: &mut SqliteConnection,
    candidate: &mut SqliteConnection,
    id: i64,
    timestamp: &str,
  ) {
    for db in [baseline, candidate] {
      sqlx::query(
        "INSERT INTO AMBIENT_ARCHIVE
         (id, source, temperature, humidity, timestamp) VALUES (?, ?, ?, ?, ?)",
      )
      .bind(id)
      .bind(format!("source-{id}"))
      .bind(20.0 + id as f64 / 100.0)
      .bind(if id % 2 == 0 { Some(45.0) } else { None })
      .bind(timestamp)
      .execute(&mut *db)
      .await
      .unwrap();
    }
  }

  async fn persist_ids(db: &mut SqliteConnection, config: &Config, ids: &[i64]) {
    let placeholders = std::iter::repeat_n("?", ids.len())
      .collect::<Vec<_>>()
      .join(",");
    let sql =
      format!("SELECT * FROM AMBIENT_ARCHIVE WHERE id IN ({placeholders}) ORDER BY id");
    let mut query = sqlx::query(&sql);
    for id in ids {
      query = query.bind(id);
    }
    let rows = query.fetch_all(&mut *db).await.unwrap();
    let records = rows
      .iter()
      .map(|row| row_record(AMBIENT, row).unwrap())
      .collect::<Vec<_>>();
    let payload = codec::encode(&records, config.layout, config.compression).unwrap();
    persist_chunk(
      db,
      AMBIENT,
      &records,
      &payload,
      value_bytes(&records).unwrap(),
      config,
    )
    .await
    .unwrap();
  }

  async fn assert_matches_oracle(
    baseline: &mut SqliteConnection,
    candidate: &mut SqliteConnection,
    start_ms: i64,
    end_ms: i64,
  ) -> QueryResult {
    let expected = ambient_oracle(baseline, start_ms, end_ms).await.unwrap();
    let actual = query(candidate, start_ms, end_ms).await.unwrap();
    assert_eq!((actual.digest.clone(), actual.count), expected);
    actual
  }

  #[tokio::test]
  async fn sqlite_bounds_preserve_offsets_submilliseconds_duplicates_and_negative_epochs()
  {
    let temp = tempfile::tempdir().unwrap();
    let (mut baseline, mut candidate, config) = databases(&temp).await;
    for (id, timestamp) in [
      (2, "1969-12-31T23:59:59.1239Z"),
      (7, "1970-01-01T08:59:59.1239+09:00"),
      (11, "1970-01-01T00:00:00.0004Z"),
      (20, "invalid-timestamp"),
    ] {
      insert_ambient(&mut baseline, &mut candidate, id, timestamp).await;
    }
    persist_ids(&mut candidate, &config, &[2, 11]).await;
    persist_ids(&mut candidate, &config, &[7, 20]).await;
    let report = build(&mut candidate).await.unwrap();
    assert_eq!(report.chunks, 2);
    assert_eq!(report.unknown_bounds_chunks, 1);

    let endpoint: i64 = sqlx::query_scalar(&format!(
      "SELECT {EPOCH_MS_SQL} FROM AMBIENT_ARCHIVE WHERE id = 2"
    ))
    .fetch_one(&mut baseline)
    .await
    .unwrap();
    assert!(endpoint < 0);
    let result =
      assert_matches_oracle(&mut baseline, &mut candidate, endpoint, endpoint).await;
    assert_eq!(result.count, 2);
    assert_eq!(result.candidate_chunks, 2);
  }

  #[tokio::test]
  async fn prunes_predicate_gaps_across_noncontiguous_overlapping_id_ranges() {
    let temp = tempfile::tempdir().unwrap();
    let (mut baseline, mut candidate, config) = databases(&temp).await;
    for (id, timestamp) in [
      (1, "2026-01-01T00:00:00.000Z"),
      (4, "2026-01-03T00:00:00.000Z"),
      (5, "2026-01-02T00:00:00.000Z"),
      (8, "2026-01-04T00:00:00.000Z"),
      (11, "2026-01-02T00:00:00.000Z"),
      (15, "2026-01-02T00:00:00.000Z"),
      (20, "2026-01-03T00:00:00.000Z"),
    ] {
      insert_ambient(&mut baseline, &mut candidate, id, timestamp).await;
    }
    persist_ids(&mut candidate, &config, &[1, 11]).await;
    persist_ids(&mut candidate, &config, &[5, 15]).await;
    persist_ids(&mut candidate, &config, &[4, 20]).await;
    build(&mut candidate).await.unwrap();

    let day = parse_timestamp("2026-01-03T00:00:00.000Z").unwrap();
    let result = assert_matches_oracle(&mut baseline, &mut candidate, day, day).await;
    assert_eq!(result.count, 2);
    assert_eq!(result.total_chunks, 3);
    assert_eq!(result.candidate_chunks, 1);
    assert_eq!(result.decoded_chunk_rows, 2);
  }

  #[tokio::test]
  async fn inclusive_boundaries_combine_chunks_and_tail_and_no_match_stays_empty() {
    let temp = tempfile::tempdir().unwrap();
    let (mut baseline, mut candidate, config) = databases(&temp).await;
    for (id, timestamp) in [
      (2, "2026-01-01T00:00:00.000Z"),
      (7, "2026-01-01T00:01:00.000Z"),
      (11, "2026-01-01T00:02:00.000Z"),
      (20, "2026-01-01T00:03:00.000Z"),
    ] {
      insert_ambient(&mut baseline, &mut candidate, id, timestamp).await;
    }
    persist_ids(&mut candidate, &config, &[2, 11]).await;
    build(&mut candidate).await.unwrap();

    let start = parse_timestamp("2026-01-01T00:02:00.000Z").unwrap();
    let end = parse_timestamp("2026-01-01T00:03:00.000Z").unwrap();
    let mixed = assert_matches_oracle(&mut baseline, &mut candidate, start, end).await;
    assert_eq!(mixed.count, 2);
    assert_eq!(mixed.candidate_chunks, 1);
    assert_eq!(mixed.tail_rows, 1);

    let gap = parse_timestamp("2026-01-01T00:01:30.000Z").unwrap();
    let empty = assert_matches_oracle(&mut baseline, &mut candidate, gap, gap).await;
    assert_eq!(empty.count, 0);
    assert_eq!(empty.candidate_chunks, 1);
  }

  #[tokio::test]
  async fn missing_and_stale_metadata_conservatively_select_chunks() {
    let temp = tempfile::tempdir().unwrap();
    let (mut baseline, mut candidate, config) = databases(&temp).await;
    for (id, timestamp) in [
      (1, "2026-01-01T00:00:00.000Z"),
      (2, "2026-01-02T00:00:00.000Z"),
    ] {
      insert_ambient(&mut baseline, &mut candidate, id, timestamp).await;
      persist_ids(&mut candidate, &config, &[id]).await;
    }
    build(&mut candidate).await.unwrap();
    sqlx::query(&format!(
      "DELETE FROM {BOUNDS_TABLE}
       WHERE chunk_id = (SELECT MIN(chunk_id) FROM {BOUNDS_TABLE})"
    ))
    .execute(&mut candidate)
    .await
    .unwrap();
    sqlx::query(&format!("UPDATE {BOUNDS_TABLE} SET chunk_digest = x'00'"))
      .execute(&mut candidate)
      .await
      .unwrap();

    let no_match = parse_timestamp("2030-01-01T00:00:00.000Z").unwrap();
    let result =
      assert_matches_oracle(&mut baseline, &mut candidate, no_match, no_match).await;
    assert_eq!(result.count, 0);
    assert_eq!(result.candidate_chunks, 2);
  }

  #[test]
  fn non_utf8_text_timestamp_is_rejected_instead_of_changed() {
    let record = vec![
      Value::Integer(1),
      Value::Text(b"source".to_vec()),
      Value::Real(20.0_f64.to_bits()),
      Value::Null,
      Value::Text(vec![0xff, 0x00]),
    ];
    assert!(timestamp_value(&record).is_err());
  }

  #[tokio::test]
  async fn candidate_loop_uses_chunk_keyset_and_metadata_primary_key() {
    let temp = tempfile::tempdir().unwrap();
    let (_, mut candidate, config) = databases(&temp).await;
    sqlx::query(
      "INSERT INTO AMBIENT_ARCHIVE
       (id, source, temperature, humidity, timestamp)
       VALUES (1, 'source', 20.0, NULL, '2026-01-01T00:00:00.000Z')",
    )
    .execute(&mut candidate)
    .await
    .unwrap();
    persist_ids(&mut candidate, &config, &[1]).await;
    build(&mut candidate).await.unwrap();

    let details = sqlx::query(&format!("EXPLAIN QUERY PLAN {CANDIDATE_CHUNK_SQL}"))
      .bind(AMBIENT)
      .bind(0_i64)
      .bind(i64::MIN)
      .bind(i64::MAX)
      .fetch_all(&mut candidate)
      .await
      .unwrap()
      .into_iter()
      .map(|row| row.get::<String, _>("detail"))
      .collect::<Vec<_>>()
      .join(" ")
      .to_ascii_lowercase()
      .replace(' ', "");
    assert!(details.contains("idx_archive_chunks_family_id"));
    assert!(details.contains("family=?") && details.contains("id>?"));
    assert!(
      details.contains("searchbusingintegerprimarykey")
        || details.contains("searchbusingindexsqlite_autoindex")
    );
    assert!(!details.contains("tempb-tree"));
  }
}
