use super::*;
use serde::Serialize;
use sqlx::{QueryBuilder, Sqlite, Transaction};
use std::{collections::HashMap, time::Instant};

const SUMMARY_TABLE: &str = "PROCESS_CHUNK_SUMMARIES";
const METADATA_TABLE: &str = "PROCESS_CHUNK_SUMMARY_METADATA";
const ACCUMULATOR_TABLE: &str = "temp.process_query_accumulator";
const RANKED_TABLE: &str = "temp.process_query_ranked";
const PAGE_SIZE: usize = 500;
const CATALOG_BATCH_SIZE: i64 = 256;
const MERGE_BATCH_SIZE: usize = 100;
const TEMP_CACHE_KIB: i64 = 16 * 1024;

#[derive(Clone, Debug, Serialize)]
pub(super) struct BuildMetrics {
  pub(super) total_ms: f64,
  pub(super) decode_ms: f64,
  pub(super) aggregation_ms: f64,
  pub(super) sqlite_write_ms: f64,
  pub(super) source_chunks: u64,
  pub(super) source_rows: u64,
  pub(super) summary_rows: u64,
  pub(super) metadata_rows: u64,
  pub(super) numerical_probe: NumericalProbe,
  pub(super) table_bytes: u64,
  pub(super) metadata_bytes: u64,
  pub(super) index_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct NumericalProbe {
  pub(super) input_memory_values: [&'static str; 4],
  pub(super) oracle_average: f64,
  pub(super) accelerated_average: f64,
  pub(super) absolute_error: f64,
  pub(super) allowed_error: f64,
  pub(super) contract_passed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct QueryMetrics {
  pub(super) total_ms: f64,
  pub(super) first_page_ms: f64,
  pub(super) catalog_fetch_ms: f64,
  pub(super) chunk_decode_ms: f64,
  pub(super) raw_fetch_ms: f64,
  pub(super) aggregation_ms: f64,
  pub(super) result_sort_ms: f64,
  pub(super) all_pages_ms: f64,
  pub(super) full_chunks_summarized: u64,
  pub(super) boundary_chunks_decoded: u64,
  pub(super) decoded_chunk_rows: u64,
  pub(super) raw_tail_rows: u64,
  pub(super) result_groups: u64,
  pub(super) page_count: u64,
  pub(super) page_size: usize,
  pub(super) temp_store: &'static str,
  pub(super) temp_cache_kib: i64,
}

#[derive(Debug)]
pub(super) struct QueryResult {
  pub(super) aggregates: Vec<Aggregate>,
  pub(super) first_page: Vec<Aggregate>,
  pub(super) metrics: QueryMetrics,
}

#[derive(Clone)]
struct PartialAggregate {
  pid: i64,
  name: Vec<u8>,
  cpu_sum: f64,
  cpu_count: i64,
  memory_sum: f64,
  memory_count: i64,
  sample_count: i64,
  max_execution: i64,
  latest: String,
}

struct CatalogRow {
  id: i64,
  min_timestamp: String,
  max_timestamp: String,
  row_count: i64,
  summary_row_count: Option<i64>,
  summary_min_timestamp: Option<String>,
  summary_max_timestamp: Option<String>,
}

pub(super) async fn build(db: &mut SqliteConnection) -> Result<BuildMetrics> {
  let numerical_probe = numerical_probe().await?;
  let total_started = Instant::now();
  create_summary_schema(db).await?;
  let mut decode_ms = 0.0;
  let mut aggregation_ms = 0.0;
  let mut sqlite_write_ms = 0.0;
  let mut source_chunks = 0_u64;
  let mut source_rows = 0_u64;
  let mut summary_rows = 0_u64;
  let mut cursor = 0_i64;
  let mut tx = db.begin().await?;
  sqlx::query(&format!("DELETE FROM {SUMMARY_TABLE}"))
    .execute(&mut *tx)
    .await?;
  sqlx::query(&format!("DELETE FROM {METADATA_TABLE}"))
    .execute(&mut *tx)
    .await?;
  loop {
    let row = sqlx::query(
      "SELECT id, min_timestamp, max_timestamp, row_count, payload, digest
       FROM ARCHIVE_CHUNKS WHERE family = ? AND id > ? ORDER BY id LIMIT 1",
    )
    .bind(PROCESS)
    .bind(cursor)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else { break };
    cursor = row.try_get("id")?;
    let min_timestamp: String = row.try_get("min_timestamp")?;
    let max_timestamp: String = row.try_get("max_timestamp")?;
    let row_count: i64 = row.try_get("row_count")?;
    let started = Instant::now();
    let records = decode_chunk(&row, &mut Vec::new()).await?;
    decode_ms += elapsed_ms(started);
    let started = Instant::now();
    let groups = aggregate_records(&records, None, None)?;
    aggregation_ms += elapsed_ms(started);
    let started = Instant::now();
    insert_summary_groups(&mut tx, cursor, groups.values()).await?;
    sqlx::query(&format!(
      "INSERT INTO {METADATA_TABLE}
       (chunk_id, source_row_count, min_timestamp, max_timestamp) VALUES (?, ?, ?, ?)"
    ))
    .bind(cursor)
    .bind(row_count)
    .bind(&min_timestamp)
    .bind(&max_timestamp)
    .execute(&mut *tx)
    .await?;
    sqlite_write_ms += elapsed_ms(started);
    source_chunks += 1;
    source_rows = source_rows
      .checked_add(u64::try_from(row_count)?)
      .ok_or("summary source row count overflow")?;
    summary_rows = summary_rows
      .checked_add(u64::try_from(groups.len())?)
      .ok_or("summary row count overflow")?;
  }
  tx.commit().await?;
  let metadata_rows = u64::try_from(
    sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {METADATA_TABLE}"))
      .fetch_one(&mut *db)
      .await?,
  )?;
  let table_bytes = dbstat_bytes(db, &[SUMMARY_TABLE]).await?;
  let metadata_bytes = dbstat_bytes(db, &[METADATA_TABLE]).await?;
  let index_bytes = dbstat_index_bytes(db, &[SUMMARY_TABLE, METADATA_TABLE]).await?;
  Ok(BuildMetrics {
    total_ms: elapsed_ms(total_started),
    decode_ms,
    aggregation_ms,
    sqlite_write_ms,
    source_chunks,
    source_rows,
    summary_rows,
    metadata_rows,
    numerical_probe,
    table_bytes,
    metadata_bytes,
    index_bytes,
  })
}

pub(super) async fn query(
  db: &mut SqliteConnection,
  start: &str,
  end: &str,
) -> Result<QueryResult> {
  let query_started = Instant::now();
  configure_temp_store(db).await?;
  create_query_tables(db).await?;
  let mut catalog_fetch_ms = 0.0;
  let mut chunk_decode_ms = 0.0;
  let mut raw_fetch_ms = 0.0;
  let mut aggregation_ms = 0.0;
  let mut full_chunks_summarized = 0_u64;
  let mut boundary_chunks_decoded = 0_u64;
  let mut decoded_chunk_rows = 0_u64;
  let mut cursor = 0_i64;
  let mut tx = db.begin().await?;
  loop {
    let started = Instant::now();
    let rows = sqlx::query(&format!(
      "SELECT c.id, c.min_timestamp, c.max_timestamp, c.row_count,
              m.source_row_count summary_row_count,
              m.min_timestamp summary_min_timestamp,
              m.max_timestamp summary_max_timestamp
       FROM ARCHIVE_CHUNKS c LEFT JOIN {METADATA_TABLE} m ON m.chunk_id = c.id
       WHERE c.family = ? AND c.id > ?
         AND c.max_timestamp >= ? AND c.min_timestamp <= ?
       ORDER BY c.id LIMIT ?"
    ))
    .bind(PROCESS)
    .bind(cursor)
    .bind(start)
    .bind(end)
    .bind(CATALOG_BATCH_SIZE)
    .fetch_all(&mut *tx)
    .await?;
    catalog_fetch_ms += elapsed_ms(started);
    if rows.is_empty() {
      break;
    }
    let catalog = rows
      .iter()
      .map(|row| {
        Ok(CatalogRow {
          id: row.try_get("id")?,
          min_timestamp: row.try_get("min_timestamp")?,
          max_timestamp: row.try_get("max_timestamp")?,
          row_count: row.try_get("row_count")?,
          summary_row_count: row.try_get("summary_row_count")?,
          summary_min_timestamp: row.try_get("summary_min_timestamp")?,
          summary_max_timestamp: row.try_get("summary_max_timestamp")?,
        })
      })
      .collect::<Result<Vec<_>>>()?;
    cursor = catalog.last().ok_or("catalog batch was empty")?.id;
    let mut summarized_ids = Vec::with_capacity(catalog.len());
    let mut boundary_ids = Vec::new();
    for chunk in catalog {
      let metadata_matches = chunk.summary_row_count == Some(chunk.row_count)
        && chunk.summary_min_timestamp.as_deref() == Some(chunk.min_timestamp.as_str())
        && chunk.summary_max_timestamp.as_deref() == Some(chunk.max_timestamp.as_str());
      if metadata_matches
        && chunk.min_timestamp.as_str() >= start
        && chunk.max_timestamp.as_str() <= end
      {
        summarized_ids.push(chunk.id);
      } else {
        boundary_chunks_decoded += 1;
        boundary_ids.push(chunk.id);
      }
    }
    if !summarized_ids.is_empty() {
      let started = Instant::now();
      merge_summaries(&mut tx, &summarized_ids).await?;
      aggregation_ms += elapsed_ms(started);
      full_chunks_summarized += u64::try_from(summarized_ids.len())?;
    }
    for chunk_id in boundary_ids {
      let started = Instant::now();
      let row = sqlx::query(
        "SELECT id, row_count, payload, digest FROM ARCHIVE_CHUNKS WHERE id = ?",
      )
      .bind(chunk_id)
      .fetch_one(&mut *tx)
      .await?;
      catalog_fetch_ms += elapsed_ms(started);
      let started = Instant::now();
      let records = decode_chunk(&row, &mut Vec::new()).await?;
      chunk_decode_ms += elapsed_ms(started);
      decoded_chunk_rows += u64::try_from(records.len())?;
      let started = Instant::now();
      let groups =
        aggregate_records(&records, Some(start.as_bytes()), Some(end.as_bytes()))?;
      merge_partial_groups(&mut tx, groups.values()).await?;
      aggregation_ms += elapsed_ms(started);
    }
  }
  let started = Instant::now();
  let raw_tail_rows = u64::try_from(
    sqlx::query_scalar::<_, i64>(
      "SELECT COUNT(*) FROM PROCESS_STATS WHERE timestamp BETWEEN ? AND ?",
    )
    .bind(start)
    .bind(end)
    .fetch_one(&mut *tx)
    .await?,
  )?;
  merge_tail(&mut tx, start, end).await?;
  raw_fetch_ms += elapsed_ms(started);
  let started = Instant::now();
  materialize_ranked_results(&mut tx).await?;
  let result_groups = u64::try_from(
    sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {RANKED_TABLE}"))
      .fetch_one(&mut *tx)
      .await?,
  )?;
  let result_sort_ms = elapsed_ms(started);
  let page_fetch_started = Instant::now();
  let first_page = fetch_page(&mut tx, 0).await?;
  let first_page_ms = elapsed_ms(query_started);
  let mut aggregates = first_page.clone();
  let mut rank_cursor = i64::try_from(first_page.len())?;
  while aggregates.len() < usize::try_from(result_groups)? {
    let page = fetch_page(&mut tx, rank_cursor).await?;
    if page.is_empty() {
      return Err("ranked result paging ended before the reported group count".into());
    }
    rank_cursor = rank_cursor
      .checked_add(i64::try_from(page.len())?)
      .ok_or("rank cursor overflow")?;
    aggregates.extend(page);
  }
  let all_pages_ms = elapsed_ms(page_fetch_started);
  let page_count = if result_groups == 0 {
    0
  } else {
    result_groups.div_ceil(PAGE_SIZE as u64)
  };
  tx.commit().await?;
  Ok(QueryResult {
    aggregates,
    first_page,
    metrics: QueryMetrics {
      total_ms: elapsed_ms(query_started),
      first_page_ms,
      catalog_fetch_ms,
      chunk_decode_ms,
      raw_fetch_ms,
      aggregation_ms,
      result_sort_ms,
      all_pages_ms,
      full_chunks_summarized,
      boundary_chunks_decoded,
      decoded_chunk_rows,
      raw_tail_rows,
      result_groups,
      page_count,
      page_size: PAGE_SIZE,
      temp_store: "FILE",
      temp_cache_kib: TEMP_CACHE_KIB,
    },
  })
}

async fn numerical_probe() -> Result<NumericalProbe> {
  let mut db = SqliteConnection::connect("sqlite::memory:").await?;
  sqlx::query(
    "CREATE TABLE process_summary_numerical_probe (
       chunk_id INTEGER NOT NULL, memory_usage INTEGER NOT NULL
     )",
  )
  .execute(&mut db)
  .await?;
  sqlx::query(
    "INSERT INTO process_summary_numerical_probe (chunk_id, memory_usage)
     VALUES (1, ?), (1, ?), (2, ?), (2, ?)",
  )
  .bind(i64::MAX - 2)
  .bind(i64::MAX - 3)
  .bind(i64::MIN + 2052)
  .bind(i64::MIN + 2053)
  .execute(&mut db)
  .await?;
  let oracle_average: f64 =
    sqlx::query_scalar("SELECT AVG(memory_usage) FROM process_summary_numerical_probe")
      .fetch_one(&mut db)
      .await?;
  let accelerated_average: f64 = sqlx::query_scalar(
    "WITH partials AS (
       SELECT chunk_id, TOTAL(memory_usage) partial_sum, COUNT(*) partial_count
       FROM process_summary_numerical_probe GROUP BY chunk_id
     )
     SELECT TOTAL(partial_sum) / SUM(partial_count) FROM partials",
  )
  .fetch_one(&mut db)
  .await?;
  db.close().await?;
  let absolute_error = (accelerated_average - oracle_average).abs();
  let allowed_error = 1e-9_f64.max(1e-12 * oracle_average.abs());
  Ok(NumericalProbe {
    input_memory_values: [
      "9223372036854775805",
      "9223372036854775804",
      "-9223372036854773756",
      "-9223372036854773755",
    ],
    oracle_average,
    accelerated_average,
    absolute_error,
    allowed_error,
    contract_passed: float_equal(oracle_average, accelerated_average),
  })
}

async fn create_summary_schema(db: &mut SqliteConnection) -> Result<()> {
  sqlx::query(&format!(
    "CREATE TABLE IF NOT EXISTS {SUMMARY_TABLE} (
       chunk_id INTEGER NOT NULL,
       pid INTEGER NOT NULL,
       process_name BLOB NOT NULL,
       cpu_sum REAL NOT NULL,
       cpu_count INTEGER NOT NULL,
       memory_sum REAL NOT NULL,
       memory_count INTEGER NOT NULL,
       sample_count INTEGER NOT NULL,
       max_execution INTEGER NOT NULL,
       latest_timestamp TEXT NOT NULL,
       PRIMARY KEY (chunk_id, pid, process_name)
     ) WITHOUT ROWID"
  ))
  .execute(&mut *db)
  .await?;
  sqlx::query(&format!(
    "CREATE TABLE IF NOT EXISTS {METADATA_TABLE} (
       chunk_id INTEGER PRIMARY KEY,
       source_row_count INTEGER NOT NULL,
       min_timestamp TEXT NOT NULL,
       max_timestamp TEXT NOT NULL
     )"
  ))
  .execute(&mut *db)
  .await?;
  Ok(())
}

async fn configure_temp_store(db: &mut SqliteConnection) -> Result<()> {
  sqlx::query("PRAGMA temp_store = FILE")
    .execute(&mut *db)
    .await?;
  sqlx::query(&format!("PRAGMA temp.cache_size = -{TEMP_CACHE_KIB}"))
    .execute(&mut *db)
    .await?;
  Ok(())
}

async fn create_query_tables(db: &mut SqliteConnection) -> Result<()> {
  sqlx::query(&format!(
    "CREATE TEMP TABLE IF NOT EXISTS {ACCUMULATOR_TABLE} (
       pid INTEGER NOT NULL,
       process_name BLOB NOT NULL,
       cpu_sum REAL NOT NULL,
       cpu_count INTEGER NOT NULL,
       memory_sum REAL NOT NULL,
       memory_count INTEGER NOT NULL,
       sample_count INTEGER NOT NULL,
       max_execution INTEGER NOT NULL,
       latest_timestamp TEXT NOT NULL,
       PRIMARY KEY (pid, process_name)
     ) WITHOUT ROWID"
  ))
  .execute(&mut *db)
  .await?;
  sqlx::query(&format!("DELETE FROM {ACCUMULATOR_TABLE}"))
    .execute(&mut *db)
    .await?;
  sqlx::query(&format!(
    "CREATE TEMP TABLE IF NOT EXISTS {RANKED_TABLE} (
       rank INTEGER PRIMARY KEY,
       pid INTEGER NOT NULL,
       process_name BLOB NOT NULL,
       cpu_sum REAL NOT NULL,
       memory_sum REAL NOT NULL,
       sample_count INTEGER NOT NULL,
       max_execution INTEGER NOT NULL,
       latest_timestamp TEXT NOT NULL
     )"
  ))
  .execute(&mut *db)
  .await?;
  sqlx::query(&format!("DELETE FROM {RANKED_TABLE}"))
    .execute(&mut *db)
    .await?;
  Ok(())
}

fn aggregate_records(
  records: &[Record],
  start: Option<&[u8]>,
  end: Option<&[u8]>,
) -> Result<HashMap<(i64, Vec<u8>), PartialAggregate>> {
  let mut groups = HashMap::new();
  for record in records {
    let timestamp = text(record, 6)?;
    if start.is_some_and(|start| timestamp < start)
      || end.is_some_and(|end| timestamp > end)
    {
      continue;
    }
    let pid = integer(record, 1)?;
    let name = text(record, 2)?.to_vec();
    let group = groups
      .entry((pid, name.clone()))
      .or_insert_with(|| PartialAggregate {
        pid,
        name,
        cpu_sum: 0.0,
        cpu_count: 0,
        memory_sum: 0.0,
        memory_count: 0,
        sample_count: 0,
        max_execution: i64::MIN,
        latest: String::new(),
      });
    group.cpu_sum += real(record, 3)?;
    group.cpu_count = group.cpu_count.checked_add(1).ok_or("CPU count overflow")?;
    group.memory_sum += integer(record, 4)? as f64;
    group.memory_count = group
      .memory_count
      .checked_add(1)
      .ok_or("memory count overflow")?;
    group.sample_count = group
      .sample_count
      .checked_add(1)
      .ok_or("sample count overflow")?;
    group.max_execution = group.max_execution.max(integer(record, 5)?);
    let timestamp = std::str::from_utf8(timestamp)?;
    if timestamp > group.latest.as_str() {
      group.latest = timestamp.to_owned();
    }
  }
  Ok(groups)
}

async fn insert_summary_groups<'a>(
  tx: &mut Transaction<'_, Sqlite>,
  chunk_id: i64,
  groups: impl Iterator<Item = &'a PartialAggregate>,
) -> Result<()> {
  let groups = groups.collect::<Vec<_>>();
  for batch in groups.chunks(MERGE_BATCH_SIZE) {
    let mut query = QueryBuilder::<Sqlite>::new(format!(
      "INSERT INTO {SUMMARY_TABLE}
       (chunk_id, pid, process_name, cpu_sum, cpu_count, memory_sum, memory_count,
        sample_count, max_execution, latest_timestamp) "
    ));
    query.push_values(batch, |mut row, group| {
      row
        .push_bind(chunk_id)
        .push_bind(group.pid)
        .push_bind(&group.name)
        .push_bind(group.cpu_sum)
        .push_bind(group.cpu_count)
        .push_bind(group.memory_sum)
        .push_bind(group.memory_count)
        .push_bind(group.sample_count)
        .push_bind(group.max_execution)
        .push_bind(&group.latest);
    });
    query.build().execute(&mut **tx).await?;
  }
  Ok(())
}

async fn merge_summaries(
  tx: &mut Transaction<'_, Sqlite>,
  chunk_ids: &[i64],
) -> Result<()> {
  for ids in chunk_ids.chunks(CATALOG_BATCH_SIZE as usize) {
    let placeholders = std::iter::repeat_n("?", ids.len())
      .collect::<Vec<_>>()
      .join(",");
    let sql = format!(
      "INSERT INTO {ACCUMULATOR_TABLE}
         (pid, process_name, cpu_sum, cpu_count, memory_sum, memory_count,
          sample_count, max_execution, latest_timestamp)
       SELECT pid, process_name, SUM(cpu_sum), SUM(cpu_count), SUM(memory_sum),
              SUM(memory_count), SUM(sample_count), MAX(max_execution),
              MAX(latest_timestamp)
       FROM {SUMMARY_TABLE}
       WHERE chunk_id IN ({placeholders})
       GROUP BY pid, process_name
       ON CONFLICT(pid, process_name) DO UPDATE SET
         cpu_sum = cpu_sum + excluded.cpu_sum,
         cpu_count = cpu_count + excluded.cpu_count,
         memory_sum = memory_sum + excluded.memory_sum,
         memory_count = memory_count + excluded.memory_count,
         sample_count = sample_count + excluded.sample_count,
         max_execution = MAX(max_execution, excluded.max_execution),
         latest_timestamp = MAX(latest_timestamp, excluded.latest_timestamp)"
    );
    let mut query = sqlx::query(&sql);
    for id in ids {
      query = query.bind(id);
    }
    query.execute(&mut **tx).await?;
  }
  Ok(())
}

async fn merge_partial_groups<'a>(
  tx: &mut Transaction<'_, Sqlite>,
  groups: impl Iterator<Item = &'a PartialAggregate>,
) -> Result<()> {
  let groups = groups.collect::<Vec<_>>();
  for batch in groups.chunks(MERGE_BATCH_SIZE) {
    let mut query = QueryBuilder::<Sqlite>::new(format!(
      "INSERT INTO {ACCUMULATOR_TABLE}
       (pid, process_name, cpu_sum, cpu_count, memory_sum, memory_count,
        sample_count, max_execution, latest_timestamp) "
    ));
    query.push_values(batch, |mut row, group| {
      row
        .push_bind(group.pid)
        .push_bind(&group.name)
        .push_bind(group.cpu_sum)
        .push_bind(group.cpu_count)
        .push_bind(group.memory_sum)
        .push_bind(group.memory_count)
        .push_bind(group.sample_count)
        .push_bind(group.max_execution)
        .push_bind(&group.latest);
    });
    query.push(
      " ON CONFLICT(pid, process_name) DO UPDATE SET
          cpu_sum = cpu_sum + excluded.cpu_sum,
          cpu_count = cpu_count + excluded.cpu_count,
          memory_sum = memory_sum + excluded.memory_sum,
          memory_count = memory_count + excluded.memory_count,
          sample_count = sample_count + excluded.sample_count,
          max_execution = MAX(max_execution, excluded.max_execution),
          latest_timestamp = MAX(latest_timestamp, excluded.latest_timestamp)",
    );
    query.build().execute(&mut **tx).await?;
  }
  Ok(())
}

async fn merge_tail(
  tx: &mut Transaction<'_, Sqlite>,
  start: &str,
  end: &str,
) -> Result<()> {
  // The retained source column is INTEGER, but SQLite SUM can overflow i64.
  // TOTAL follows the existing AVG query's floating accumulation semantics.
  sqlx::query(&format!(
    "INSERT INTO {ACCUMULATOR_TABLE}
       (pid, process_name, cpu_sum, cpu_count, memory_sum, memory_count,
        sample_count, max_execution, latest_timestamp)
     SELECT pid, CAST(process_name AS BLOB), TOTAL(cpu_usage), COUNT(cpu_usage),
            TOTAL(memory_usage), COUNT(memory_usage), COUNT(*), MAX(execution_sec),
            MAX(timestamp)
     FROM PROCESS_STATS
     WHERE timestamp BETWEEN ? AND ?
     GROUP BY pid, process_name
     ON CONFLICT(pid, process_name) DO UPDATE SET
       cpu_sum = cpu_sum + excluded.cpu_sum,
       cpu_count = cpu_count + excluded.cpu_count,
       memory_sum = memory_sum + excluded.memory_sum,
       memory_count = memory_count + excluded.memory_count,
       sample_count = sample_count + excluded.sample_count,
       max_execution = MAX(max_execution, excluded.max_execution),
       latest_timestamp = MAX(latest_timestamp, excluded.latest_timestamp)"
  ))
  .bind(start)
  .bind(end)
  .execute(&mut **tx)
  .await?;
  Ok(())
}

async fn materialize_ranked_results(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
  sqlx::query(&format!(
    "INSERT INTO {RANKED_TABLE}
       (rank, pid, process_name, cpu_sum, memory_sum, sample_count,
        max_execution, latest_timestamp)
     SELECT ROW_NUMBER() OVER (
              ORDER BY cpu_sum / cpu_count DESC, pid ASC, process_name ASC
            ),
            pid, process_name, cpu_sum, memory_sum, sample_count,
            max_execution, latest_timestamp
     FROM {ACCUMULATOR_TABLE}"
  ))
  .execute(&mut **tx)
  .await?;
  Ok(())
}

async fn fetch_page(
  tx: &mut Transaction<'_, Sqlite>,
  after_rank: i64,
) -> Result<Vec<Aggregate>> {
  let rows = sqlx::query(&format!(
    "SELECT pid, process_name, cpu_sum, memory_sum, sample_count,
            max_execution, latest_timestamp
     FROM {RANKED_TABLE}
     WHERE rank > ? ORDER BY rank LIMIT ?"
  ))
  .bind(after_rank)
  .bind(i64::try_from(PAGE_SIZE)?)
  .fetch_all(&mut **tx)
  .await?;
  rows
    .iter()
    .map(|row| {
      Ok(Aggregate {
        pid: row.try_get("pid")?,
        name: row.try_get("process_name")?,
        cpu_sum: row.try_get("cpu_sum")?,
        memory_sum: row.try_get("memory_sum")?,
        count: u64::try_from(row.try_get::<i64, _>("sample_count")?)?,
        max_execution: row.try_get("max_execution")?,
        latest: row.try_get("latest_timestamp")?,
      })
    })
    .collect()
}

async fn dbstat_bytes(db: &mut SqliteConnection, names: &[&str]) -> Result<u64> {
  let placeholders = std::iter::repeat_n("?", names.len())
    .collect::<Vec<_>>()
    .join(",");
  let sql =
    format!("SELECT COALESCE(SUM(pgsize), 0) FROM dbstat WHERE name IN ({placeholders})");
  let mut query = sqlx::query_scalar::<_, i64>(&sql);
  for name in names {
    query = query.bind(name);
  }
  Ok(u64::try_from(query.fetch_one(&mut *db).await?)?)
}

async fn dbstat_index_bytes(db: &mut SqliteConnection, tables: &[&str]) -> Result<u64> {
  let placeholders = std::iter::repeat_n("?", tables.len())
    .collect::<Vec<_>>()
    .join(",");
  let sql = format!(
    "SELECT COALESCE(SUM(pgsize), 0) FROM dbstat
     WHERE name IN (
       SELECT name FROM sqlite_schema
       WHERE type = 'index' AND tbl_name IN ({placeholders})
     )"
  );
  let mut query = sqlx::query_scalar::<_, i64>(&sql);
  for table in tables {
    query = query.bind(table);
  }
  Ok(u64::try_from(query.fetch_one(&mut *db).await?)?)
}

fn duration_ms(duration: &std::time::Duration) -> f64 {
  duration.as_secs_f64() * 1_000.0
}

fn elapsed_ms(started: Instant) -> f64 {
  duration_ms(&started.elapsed())
}

#[cfg(test)]
mod tests {
  use super::*;

  async fn insert_process(
    db: &mut SqliteConnection,
    pid: i64,
    name: &[u8],
    cpu: f64,
    memory: i64,
    execution: i64,
    timestamp: &str,
  ) {
    sqlx::query(
      "INSERT INTO PROCESS_STATS
       (pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp)
       VALUES (?, CAST(? AS TEXT), ?, ?, ?, ?)",
    )
    .bind(pid)
    .bind(name)
    .bind(cpu)
    .bind(memory)
    .bind(execution)
    .bind(timestamp)
    .execute(db)
    .await
    .unwrap();
  }

  fn test_config(path: &std::path::Path) -> Config {
    Config {
      output: path.to_path_buf(),
      minutes: 1,
      processes_per_minute: 1,
      chunk_minutes: 1,
      chunk_rows: 10_000,
      repetitions: 1,
      layout: Layout::Columnar,
      compression: Compression::Deflate,
      seed: 1,
      duty_cycle: 1,
      group_cap: 100_000,
      query_experiment: true,
      process_workload: ProcessWorkload::Stable,
      process_lifetime_minutes: 1,
    }
  }

  async fn finalize_ids(db: &mut SqliteConnection, ids: &[i64], config: &Config) {
    let placeholders = std::iter::repeat_n("?", ids.len())
      .collect::<Vec<_>>()
      .join(",");
    let sql =
      format!("SELECT * FROM PROCESS_STATS WHERE id IN ({placeholders}) ORDER BY id");
    let mut query = sqlx::query(&sql);
    for id in ids {
      query = query.bind(id);
    }
    let rows = query.fetch_all(&mut *db).await.unwrap();
    let records = rows
      .iter()
      .map(|row| row_record(PROCESS, row))
      .collect::<Result<Vec<_>>>()
      .unwrap();
    let payload = codec::encode(&records, config.layout, config.compression).unwrap();
    persist_chunk(
      db,
      PROCESS,
      &records,
      &payload,
      value_bytes(&records).unwrap(),
      config,
    )
    .await
    .unwrap();
  }

  fn canonical(mut aggregates: Vec<Aggregate>) -> Vec<Aggregate> {
    aggregates
      .sort_by(|left, right| (left.pid, &left.name).cmp(&(right.pid, &right.name)));
    aggregates
  }

  #[tokio::test]
  async fn combines_full_boundary_and_tail_without_losing_tuple_identity() {
    let temp = tempfile::tempdir().unwrap();
    let config = test_config(temp.path());
    let mut db = open(&temp.path().join("mixed.sqlite3")).await.unwrap();
    create_schema(&mut db).await.unwrap();
    create_chunk_schema(&mut db).await.unwrap();
    let timestamps = [
      "2026-01-01T00:00:00.000Z",
      "2026-01-01T00:00:01.007Z",
      "2026-01-01T00:00:02.000Z",
      "2026-01-01T00:00:03.000Z",
      "2026-01-01T00:00:04.123Z",
      "2026-01-01T00:00:05.000Z",
      "2026-01-01T00:00:06.000Z",
    ];
    for (index, timestamp) in timestamps.iter().enumerate() {
      let pid = if index == 4 { 9 } else { 7 };
      let name = if index == 5 {
        b"worker-b".as_slice()
      } else {
        b"worker-a".as_slice()
      };
      insert_process(
        &mut db,
        pid,
        name,
        10.0 + index as f64,
        100 + index as i64,
        index as i64,
        timestamp,
      )
      .await;
    }
    let oracle = process_oracle(&mut db, timestamps[1], timestamps[6])
      .await
      .unwrap();
    finalize_ids(&mut db, &[1, 2, 3], &config).await;
    finalize_ids(&mut db, &[4, 5, 6], &config).await;
    let built = build(&mut db).await.unwrap();
    assert_eq!(built.source_chunks, 2);
    assert_eq!(built.metadata_rows, 2);
    assert!(built.table_bytes > 0);

    let result = query(&mut db, timestamps[1], timestamps[6]).await.unwrap();
    let actual = canonical(result.aggregates);
    assert!(
      compare_aggregates(&oracle, &actual),
      "oracle={oracle:?}, actual={actual:?}"
    );
    assert_eq!(result.metrics.full_chunks_summarized, 1);
    assert_eq!(result.metrics.boundary_chunks_decoded, 1);
    assert_eq!(result.metrics.decoded_chunk_rows, 3);
    assert_eq!(result.metrics.raw_tail_rows, 1);
    assert_eq!(result.metrics.result_groups, 3);
  }

  #[tokio::test]
  async fn returns_no_groups_when_text_range_matches_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let config = test_config(temp.path());
    let mut db = open(&temp.path().join("empty.sqlite3")).await.unwrap();
    create_schema(&mut db).await.unwrap();
    create_chunk_schema(&mut db).await.unwrap();
    insert_process(&mut db, 1, b"only", 1.0, 2, 3, "2026-01-01T00:00:00.001Z").await;
    finalize_ids(&mut db, &[1], &config).await;
    build(&mut db).await.unwrap();
    let result = query(
      &mut db,
      "2026-02-01T00:00:00.000Z",
      "2026-02-01T00:00:00.999Z",
    )
    .await
    .unwrap();
    assert!(result.aggregates.is_empty());
    assert!(result.first_page.is_empty());
    assert_eq!(result.metrics.page_count, 0);
    assert_eq!(result.metrics.decoded_chunk_rows, 0);
  }

  #[tokio::test]
  async fn detects_chunk_summary_cancellation_counterexample() {
    let temp = tempfile::tempdir().unwrap();
    let config = test_config(temp.path());
    let mut db = open(&temp.path().join("extreme-memory.sqlite3"))
      .await
      .unwrap();
    create_schema(&mut db).await.unwrap();
    create_chunk_schema(&mut db).await.unwrap();
    let timestamps = [
      "2026-01-01T00:00:00.001Z",
      "2026-01-01T00:00:00.002Z",
      "2026-01-01T00:00:00.003Z",
      "2026-01-01T00:00:00.004Z",
    ];
    let memories = [i64::MAX - 2, i64::MAX - 3, i64::MIN + 2052, i64::MIN + 2053];
    for (timestamp, memory) in timestamps.iter().zip(memories) {
      insert_process(&mut db, 44, b"large", 1.0, memory, 1, timestamp).await;
    }
    let oracle = process_oracle(&mut db, timestamps[0], timestamps[3])
      .await
      .unwrap();
    finalize_ids(&mut db, &[1, 2], &config).await;
    finalize_ids(&mut db, &[3, 4], &config).await;
    let built = build(&mut db).await.unwrap();
    assert_eq!(built.numerical_probe.oracle_average, 1024.5);
    assert_eq!(built.numerical_probe.accelerated_average, 1024.0);
    assert_eq!(built.numerical_probe.absolute_error, 0.5);
    assert!(!built.numerical_probe.contract_passed);
    let result = query(&mut db, timestamps[0], timestamps[3]).await.unwrap();
    let actual = canonical(result.aggregates);
    assert_eq!(oracle[0].memory_sum / oracle[0].count as f64, 1024.5);
    assert_eq!(actual[0].memory_sum / actual[0].count as f64, 1024.0);
    assert!(!compare_aggregates(&oracle, &actual));
    assert_eq!(result.metrics.full_chunks_summarized, 2);
    assert_eq!(result.metrics.decoded_chunk_rows, 0);
  }

  #[tokio::test]
  async fn spills_many_groups_and_pages_with_deterministic_rank_ties() {
    let temp = tempfile::tempdir().unwrap();
    let config = test_config(temp.path());
    let mut db = open(&temp.path().join("many.sqlite3")).await.unwrap();
    create_schema(&mut db).await.unwrap();
    create_chunk_schema(&mut db).await.unwrap();
    let group_count = 1_205_i64;
    let mut tx = db.begin().await.unwrap();
    for first_pid in (0..group_count).step_by(MERGE_BATCH_SIZE) {
      let end_pid = (first_pid + MERGE_BATCH_SIZE as i64).min(group_count);
      let mut insert = QueryBuilder::<Sqlite>::new(
        "INSERT INTO PROCESS_STATS
         (pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp) ",
      );
      insert.push_values(first_pid..end_pid, |mut row, pid| {
        row
          .push_bind(pid)
          .push_bind(format!("process-{pid:04}"))
          .push_bind(42.0)
          .push_bind(1_000 + pid)
          .push_bind(pid)
          .push_bind("2026-01-01T00:00:00.009Z");
      });
      insert.build().execute(&mut *tx).await.unwrap();
    }
    tx.commit().await.unwrap();
    let ids = (1..=group_count).collect::<Vec<_>>();
    finalize_ids(&mut db, &ids, &config).await;
    let built = build(&mut db).await.unwrap();
    assert_eq!(built.summary_rows, group_count as u64);

    let result = query(
      &mut db,
      "2026-01-01T00:00:00.009Z",
      "2026-01-01T00:00:00.009Z",
    )
    .await
    .unwrap();
    assert_eq!(result.aggregates.len(), group_count as usize);
    assert_eq!(result.first_page.len(), PAGE_SIZE);
    assert_eq!(result.metrics.page_count, 3);
    assert_eq!(result.metrics.full_chunks_summarized, 1);
    assert_eq!(result.metrics.boundary_chunks_decoded, 0);
    assert_eq!(result.metrics.decoded_chunk_rows, 0);
    assert_eq!(result.aggregates[0].pid, 0);
    assert_eq!(result.aggregates[1].pid, 1);
    assert_eq!(result.aggregates.last().unwrap().pid, group_count - 1);
  }
}
