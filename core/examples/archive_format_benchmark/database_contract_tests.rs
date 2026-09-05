use super::*;

fn test_config(output: PathBuf) -> Config {
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
  }
}

async fn databases(
  temp: &tempfile::TempDir,
) -> (SqliteConnection, SqliteConnection, Config) {
  let config = test_config(temp.path().to_path_buf());
  let mut baseline = open(&temp.path().join("baseline.sqlite3")).await.unwrap();
  let mut candidate = open(&temp.path().join("candidate.sqlite3")).await.unwrap();
  create_schema(&mut baseline).await.unwrap();
  create_schema(&mut candidate).await.unwrap();
  create_chunk_schema(&mut candidate).await.unwrap();
  (baseline, candidate, config)
}

async fn insert_process(db: &mut SqliteConnection, id: i64, timestamp: &str) {
  sqlx::query(
    "INSERT INTO PROCESS_STATS
     (id, pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp)
     VALUES (?, ?, ?, ?, ?, ?, ?)",
  )
  .bind(id)
  .bind(1000 + id)
  .bind(format!("process-{id}"))
  .bind(id as f64 / 10.0)
  .bind(4096 + id)
  .bind(60 + id)
  .bind(timestamp)
  .execute(db)
  .await
  .unwrap();
}

async fn insert_ambient(
  db: &mut SqliteConnection,
  id: i64,
  source: &str,
  timestamp: &str,
) {
  sqlx::query(
    "INSERT INTO AMBIENT_ARCHIVE
     (id, source, temperature, humidity, timestamp) VALUES (?, ?, ?, ?, ?)",
  )
  .bind(id)
  .bind(source)
  .bind(20.0 + id as f64 / 10.0)
  .bind(if id % 2 == 0 { Some(45.5) } else { None })
  .bind(timestamp)
  .execute(db)
  .await
  .unwrap();
}

async fn records_for_ids(
  db: &mut SqliteConnection,
  family: &str,
  ids: &[i64],
) -> Vec<Record> {
  let (table, _, _) = family_table(family);
  let placeholders = std::iter::repeat_n("?", ids.len())
    .collect::<Vec<_>>()
    .join(",");
  let sql = format!("SELECT * FROM {table} WHERE id IN ({placeholders}) ORDER BY id");
  let mut query = sqlx::query(&sql);
  for id in ids {
    query = query.bind(id);
  }
  query
    .fetch_all(db)
    .await
    .unwrap()
    .iter()
    .map(|row| row_record(family, row).unwrap())
    .collect()
}

async fn persist_records(
  db: &mut SqliteConnection,
  family: &str,
  records: &[Record],
  config: &Config,
) {
  let payload = codec::encode(records, config.layout, config.compression).unwrap();
  persist_chunk(db, family, records, &payload, value_bytes(records), config)
    .await
    .unwrap();
}

#[tokio::test]
async fn missing_persisted_chunk_fails_global_comparison() {
  let temp = tempfile::tempdir().unwrap();
  let (mut baseline, mut candidate, config) = databases(&temp).await;
  for id in [1, 4, 9] {
    let timestamp = format!("2026-01-01T00:0{id}:00.000Z");
    insert_process(&mut baseline, id, &timestamp).await;
    insert_process(&mut candidate, id, &timestamp).await;
  }

  let first = records_for_ids(&mut candidate, PROCESS, &[1, 4]).await;
  persist_records(&mut candidate, PROCESS, &first, &config).await;
  let second = records_for_ids(&mut candidate, PROCESS, &[9]).await;
  persist_records(&mut candidate, PROCESS, &second, &config).await;
  assert!(
    compare_persisted(&mut baseline, &mut candidate, &mut Vec::new())
      .await
      .unwrap()
  );

  sqlx::query("DELETE FROM ARCHIVE_CHUNKS WHERE family = ? AND min_row_id = 1")
    .bind(PROCESS)
    .execute(&mut candidate)
    .await
    .unwrap();

  assert!(
    !compare_persisted(&mut baseline, &mut candidate, &mut Vec::new())
      .await
      .unwrap()
  );
}

#[tokio::test]
async fn duplicated_persisted_chunk_fails_global_comparison() {
  let temp = tempfile::tempdir().unwrap();
  let (mut baseline, mut candidate, config) = databases(&temp).await;
  for id in [2, 8] {
    let timestamp = format!("2026-01-01T00:0{id}:00.000Z");
    insert_process(&mut baseline, id, &timestamp).await;
    insert_process(&mut candidate, id, &timestamp).await;
  }
  let records = records_for_ids(&mut candidate, PROCESS, &[2, 8]).await;
  persist_records(&mut candidate, PROCESS, &records, &config).await;

  sqlx::query(
    "INSERT INTO ARCHIVE_CHUNKS
     (family, min_row_id, max_row_id, min_timestamp, max_timestamp, row_count,
      layout, compression, decoded_value_bytes, payload, digest)
     SELECT family, min_row_id, max_row_id, min_timestamp, max_timestamp, row_count,
            layout, compression, decoded_value_bytes, payload, digest
     FROM ARCHIVE_CHUNKS WHERE family = ?",
  )
  .bind(PROCESS)
  .execute(&mut candidate)
  .await
  .unwrap();

  assert!(
    !compare_persisted(&mut baseline, &mut candidate, &mut Vec::new())
      .await
      .unwrap()
  );
}

#[tokio::test]
async fn changed_selection_rolls_back_chunk_and_other_deletes() {
  let temp = tempfile::tempdir().unwrap();
  let (_, mut candidate, config) = databases(&temp).await;
  for id in [1, 5, 12] {
    insert_process(&mut candidate, id, "2026-01-01T00:00:00.000Z").await;
  }
  let selected = records_for_ids(&mut candidate, PROCESS, &[1, 5, 12]).await;
  sqlx::query("DELETE FROM PROCESS_STATS WHERE id = 5")
    .execute(&mut candidate)
    .await
    .unwrap();
  let payload = codec::encode(&selected, config.layout, config.compression).unwrap();

  assert!(
    persist_chunk(
      &mut candidate,
      PROCESS,
      &selected,
      &payload,
      value_bytes(&selected),
      &config,
    )
    .await
    .is_err()
  );
  let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM PROCESS_STATS ORDER BY id")
    .fetch_all(&mut candidate)
    .await
    .unwrap();
  assert_eq!(ids, vec![1, 12]);
  assert_eq!(
    scalar(&mut candidate, "SELECT COUNT(*) FROM ARCHIVE_CHUNKS")
      .await
      .unwrap(),
    0
  );
}

#[tokio::test]
async fn ambient_endpoint_uses_sqlite_submillisecond_rounding_and_offsets() {
  let temp = tempfile::tempdir().unwrap();
  let (mut baseline, mut candidate, config) = databases(&temp).await;
  let fixtures = [
    (2, "utc", "2026-01-01T00:00:00.1239+00:00"),
    (7, "offset", "2026-01-01T09:00:00.1239+09:00"),
    (11, "outside", "2026-01-01T00:00:00.1229Z"),
  ];
  for (id, source, timestamp) in fixtures {
    insert_ambient(&mut baseline, id, source, timestamp).await;
    insert_ambient(&mut candidate, id, source, timestamp).await;
  }
  let records = records_for_ids(&mut candidate, AMBIENT, &[2, 7, 11]).await;
  persist_records(&mut candidate, AMBIENT, &records, &config).await;

  let endpoint: i64 = sqlx::query_scalar(&format!(
    "SELECT {EPOCH_MS_SQL} FROM AMBIENT_ARCHIVE WHERE id = 2"
  ))
  .fetch_one(&mut baseline)
  .await
  .unwrap();
  assert_eq!(endpoint, parse_timestamp(fixtures[0].2).unwrap() + 1);
  let oracle = ambient_oracle(&mut baseline, endpoint, endpoint)
    .await
    .unwrap();
  let chunked = ambient_chunked(&mut candidate, endpoint, endpoint, &mut Vec::new())
    .await
    .unwrap();
  assert_eq!(oracle.1, 2);
  assert_eq!(chunked, oracle);
}

#[tokio::test]
async fn noncontiguous_ids_backwards_timestamps_and_duplicate_instants_remain_exact() {
  let temp = tempfile::tempdir().unwrap();
  let (mut baseline, mut candidate, config) = databases(&temp).await;
  let fixtures = [
    (2, "later-first", "2026-01-01T00:02:00.000Z"),
    (7, "earlier-second", "2026-01-01T00:00:00.000Z"),
    (11, "same-instant", "2026-01-01T09:00:00.000+09:00"),
    (20, "middle-last", "2026-01-01T00:01:00.000Z"),
  ];
  for (id, source, timestamp) in fixtures {
    insert_ambient(&mut baseline, id, source, timestamp).await;
    insert_ambient(&mut candidate, id, source, timestamp).await;
  }

  let first = records_for_ids(&mut candidate, AMBIENT, &[2, 7]).await;
  persist_records(&mut candidate, AMBIENT, &first, &config).await;
  let second = records_for_ids(&mut candidate, AMBIENT, &[11, 20]).await;
  persist_records(&mut candidate, AMBIENT, &second, &config).await;

  assert!(
    compare_persisted(&mut baseline, &mut candidate, &mut Vec::new())
      .await
      .unwrap()
  );
  let start_ms = parse_timestamp("2026-01-01T00:00:00.000Z").unwrap();
  let end_ms = parse_timestamp("2026-01-01T00:01:00.000Z").unwrap();
  let oracle = ambient_oracle(&mut baseline, start_ms, end_ms)
    .await
    .unwrap();
  let chunked = ambient_chunked(&mut candidate, start_ms, end_ms, &mut Vec::new())
    .await
    .unwrap();
  assert_eq!(oracle.1, 3);
  assert_eq!(chunked, oracle);
}
