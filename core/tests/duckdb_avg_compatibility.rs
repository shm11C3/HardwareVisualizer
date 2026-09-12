#![cfg(feature = "duckdb-archive")]
//! Whether the native Process Stats family answers what the SQLite family
//! answers.
//!
//! Two levels. The engine-level tests compare SQLite's own `avg()` with the
//! arithmetic the native query uses on the same rows - DuckDB's `AVG` for the
//! binary64 column, the exact sum and count for the integer column - which is
//! what says where the two are interchangeable at all. The family-level test
//! then puts one fixture through the real SQLite writer and query and through
//! candidate -> finalize -> native, and compares the answers bit for bit.

mod native_support;

use std::collections::BTreeMap;

use duckdb::{Config, Connection, params};
use hardviz_core::infrastructure::database::native_database::NativeCancellation;
use hardviz_core::infrastructure::database::native_database::process_stats as native_process_stats;
use hardviz_core::infrastructure::database::{
  archive_queries, db, migrate, process_stats,
};
use hardviz_core::persistence::archive_data::ProcessStatData;
use native_support::{NativeFixture, app_migrations};
use sqlx::{Executor, QueryBuilder, Row, Sqlite, SqlitePool};

const VECTOR_CROSSING_ROWS: usize = 8_193;

#[derive(Clone)]
struct ProcessInput {
  pid: i64,
  name: &'static str,
  cpu: f32,
  memory: i64,
  ordinal: i64,
  timestamp: String,
}

#[derive(Debug, Eq, PartialEq)]
struct AggregateBits {
  by_identity: BTreeMap<(i64, String), (u64, u64)>,
  cpu_rank_bands: Vec<(u64, Vec<(i64, String)>)>,
}

/// The engine-level claim the native Process query rests on: for the values the
/// production writers can produce, DuckDB's `AVG(DOUBLE)` returns SQLite's
/// exact binary64 result, independently of row order and of how many threads
/// DuckDB aggregates with, and the integer average rebuilt from DuckDB's exact
/// `SUM`/`COUNT` matches SQLite's integer `avg()` bit for bit.
///
/// The integer column deliberately does not go through DuckDB's `AVG(BIGINT)`:
/// that divides the exact sum as a `long double`, so its last bit depends on
/// the platform's `long double` width (this fixture's `i64-memory` group came
/// back one ulp apart on x86_64 Linux and aarch64 macOS).
#[tokio::test]
async fn duckdb_avg_matches_sqlite_for_archive_magnitude_process_values() {
  let mut mismatches = Vec::new();
  for order in [Order::Forward, Order::Reverse, Order::Interleaved] {
    let input = process_input(order);
    let sqlite = sqlite_aggregates(&input).await;

    for threads in [1, 2, 4] {
      let duckdb = duckdb_aggregates(&input, threads);
      if duckdb != sqlite {
        mismatches.push(format!(
          "{order:?}/threads={threads}: {}",
          describe_mismatches(&sqlite, &duckdb)
        ));
      }
    }
  }
  assert!(
    mismatches.is_empty(),
    "DuckDB AVG diverged from SQLite:\n{}",
    mismatches.join("\n")
  );
}

/// Where that stops being true, measured rather than assumed.
///
/// SQLite's `avg()` over REAL has used Kahan-Babuska-Neumaier compensated
/// summation since 3.43, so it recovers a sum that plain binary64 accumulation
/// loses. DuckDB has no aggregate that reproduces it: `avg`, `fsum`/`favg`
/// (classic Kahan) and `sum` all collapse `[x, 1.0, -x]` to 0 once `x` exceeds
/// about 2^53 - while SQLite returns 1/3.
///
/// This test pins that boundary instead of hiding it. It is inside binary64,
/// not inside the archive: `cpu_usage` is a CPU-usage percentage written from
/// `f32`, so a group can only reach this if something other than the collector
/// wrote it. See the finalization report and #2089 for the open decision on
/// whether that residue needs an exact Rust-side aggregation.
#[tokio::test]
async fn duckdb_avg_diverges_from_sqlite_only_beyond_binary64_cancellation() {
  let scales: [f32; 4] = [1e6, 1e15, 1e16, 1e17];
  let mut input = Vec::new();
  for index in 0..VECTOR_CROSSING_ROWS {
    let triplet = index % 3;
    for (group, scale) in scales.iter().enumerate() {
      input.push(ProcessInput {
        pid: group as i64 + 1,
        name: "cancellation",
        cpu: [*scale, 1.0, -*scale][triplet],
        memory: 1,
        ordinal: (input.len() + 1) as i64,
        timestamp: "2026-01-01T00:00:00.000Z".to_owned(),
      });
    }
  }
  let sqlite = sqlite_aggregates(&input).await;
  let duckdb = duckdb_aggregates(&input, 1);

  for (group, scale) in scales.iter().enumerate() {
    let identity = (group as i64 + 1, "cancellation".to_owned());
    let expected = sqlite.by_identity[&identity];
    let actual = duckdb.by_identity[&identity];
    if *scale <= 1e15 {
      assert_eq!(actual, expected, "scale {scale:e} must still agree");
    } else {
      assert_ne!(
        actual, expected,
        "scale {scale:e} is the measured divergence; update this test if an \
         engine changes"
      );
      assert_eq!(actual.0, 0.0_f64.to_bits(), "scale {scale:e}");
    }
  }
}

/// The whole family, end to end: the production SQLite writer and query against
/// candidate -> finalize -> native.
#[tokio::test]
async fn the_native_process_stats_family_reproduces_the_sqlite_family() {
  let fixture = NativeFixture::new();
  // The only test in this binary that initializes Core's process-wide database
  // path, so the SQLite side runs through the real writer and the real query.
  assert!(db::init(fixture.source.clone()));
  migrate::run(app_migrations::get_migrations())
    .await
    .unwrap();

  // 1. Written by the production writer, so the stored timestamp text is
  //    whatever sqlx actually produces rather than whatever a test spells.
  //
  //    Every fixed-spelling row is anchored to the clock rather than to a
  //    calendar date, so the Retention Period in step 4 keeps bisecting this
  //    fixture however long after the test was written it runs.
  let anchor = anchor_day();
  let whole = anchor;
  let fractional = anchor + chrono::Duration::milliseconds(60_125);
  process_stats::insert(
    vec![
      stat(4000, "renderer", 12.5, 4_096, 60),
      stat(4001, "helper", 0.0, 0, 0),
      stat(4002, "idle", 100.0, 8_192, 1),
    ],
    whole,
  )
  .await
  .unwrap();
  process_stats::insert(
    vec![
      // The same identity tuple again: Process identity is (pid,
      // process_name), so these must merge into one group.
      stat(4000, "renderer", 37.5, 4_096, 120),
      // A tie with `renderer`'s average, to pin the rank bands.
      stat(4003, "tied", 25.0, 4_096, 5),
    ],
    fractional,
  )
  .await
  .unwrap();

  let pool = native_support::open_pool(&fixture.source, false).await;
  let stored: Vec<(String, i64)> = sqlx::query(
    "SELECT CAST(timestamp AS TEXT), COUNT(*) FROM PROCESS_STATS GROUP BY 1 ORDER BY 1",
  )
  .fetch_all(&pool)
  .await
  .unwrap()
  .into_iter()
  .map(|row| (row.get(0), row.get(1)))
  .collect();
  assert_eq!(
    stored,
    vec![
      (native_process_stats::sqlite_timestamp_text(&whole), 3),
      (native_process_stats::sqlite_timestamp_text(&fractional), 2),
    ],
    "the native writer must render the bytes sqlx stores"
  );

  // 2. Rows an older build or a wider integer could have left behind: other
  //    accepted timestamp spellings, an i64 memory value no `i32` writer can
  //    produce, and an embedded NUL in a process name.
  pool
    .execute(
      format!(
        r#"
      INSERT INTO PROCESS_STATS(pid,process_name,cpu_usage,memory_usage,execution_sec,timestamp)
      VALUES
        (4004,'legacy-z',7.25,9223372036854775807,10,'{zulu}'),
        (4005,'legacy-offset',3.5,4096,11,'{east_nine}'),
        (4006,'nul'||char(0)||'name',1.25,4096,12,'{utc_offset}');
      "#,
        zulu = zulu(anchor + chrono::Duration::minutes(2)),
        east_nine = east_nine(anchor + chrono::Duration::minutes(3)),
        utc_offset = utc_offset(anchor + chrono::Duration::minutes(4)),
      )
      .as_str(),
    )
    .await
    .unwrap();

  // Rows placed relative to the clock, so the Retention Period actually
  // bisects them below.
  let now = chrono::Utc::now();
  for (days, name) in [(10_i64, "expired"), (5, "kept-old"), (1, "kept-recent")] {
    process_stats::insert(
      vec![stat(5000 + days as i32, name, 2.0, 1_024, 30)],
      now - chrono::Duration::days(days),
    )
    .await
    .unwrap();
  }
  pool.close().await;

  fixture.finalize().await;
  let database = fixture.open().await;

  // 3. The same ranges through both query paths.
  let ranges: [(String, String, bool); 8] = [
    (String::new(), "zzzz".to_owned(), false),
    (String::new(), "zzzz".to_owned(), true),
    // Inclusive endpoints, exactly on stored spellings.
    (utc_offset(whole), utc_offset_millis(fractional), false),
    (utc_offset(whole), utc_offset_millis(fractional), true),
    // One stamp only.
    (utc_offset(whole), utc_offset(whole), false),
    // Just inside the fractional stamp, which text comparison excludes.
    (
      utc_offset(whole),
      utc_offset_millis(fractional - chrono::Duration::milliseconds(1)),
      false,
    ),
    // Spellings that sort outside the ISO-8601 UTC block.
    (
      zulu(anchor + chrono::Duration::minutes(2)),
      east_nine(anchor + chrono::Duration::minutes(3)),
      false,
    ),
    // Empty.
    (
      utc_offset(anchor + chrono::Duration::days(120)),
      utc_offset(anchor + chrono::Duration::days(121)),
      false,
    ),
  ];
  for (start, end, order_by_cpu_desc) in ranges {
    let expected = archive_queries::select_process_stats(&start, &end, order_by_cpu_desc)
      .await
      .unwrap();
    let actual = native_process_stats::select_process_stats(
      &database,
      NativeCancellation::new(),
      start.clone(),
      end.clone(),
      order_by_cpu_desc,
    )
    .await
    .unwrap();
    assert_records_match(&expected, &actual, &start, &end, order_by_cpu_desc);
  }
  assert!(
    archive_queries::select_process_stats("", "zzzz", false)
      .await
      .unwrap()
      .iter()
      .any(|record| record.avg_memory_usage > 4.0e18),
    "the large-memory row must be inside the compared range"
  );

  // 4. The same Retention Period through both delete paths.
  let survivors_before = surviving_identities(&database).await;
  assert!(survivors_before.iter().any(|(_, name)| name == "expired"));
  let pool = native_support::open_pool(&fixture.source, false).await;
  let rows_before = process_stats_row_count(&pool).await;
  pool.close().await;
  process_stats::delete_old_data(7).await.unwrap();
  let pool = native_support::open_pool(&fixture.source, false).await;
  let sqlite_deleted = u64::try_from(rows_before - process_stats_row_count(&pool).await)
    .expect("a delete cannot add rows");
  pool.close().await;
  let deleted =
    native_process_stats::delete_old_data(&database, NativeCancellation::new(), 7)
      .await
      .unwrap();
  // The Retention Period must bisect the fixture, not clear it: only the row
  // written outside the window goes.
  assert_eq!(sqlite_deleted, 1, "the fixture must straddle the boundary");
  assert_eq!(deleted, sqlite_deleted);

  let pool = native_support::open_pool(&fixture.source, false).await;
  let mut expected: Vec<(i64, String)> =
    sqlx::query("SELECT pid, process_name FROM PROCESS_STATS")
      .fetch_all(&pool)
      .await
      .unwrap()
      .into_iter()
      .map(|row| (row.get(0), row.get(1)))
      .collect();
  pool.close().await;
  expected.sort();
  let mut actual = surviving_identities(&database).await;
  actual.sort();
  assert_eq!(actual, expected);
  assert!(!actual.iter().any(|(_, name)| name == "expired"));
  assert!(actual.iter().any(|(_, name)| name == "kept-old"));
  database.close().await.unwrap();
}

fn stat(
  pid: i32,
  name: &str,
  cpu_usage: f32,
  memory_usage: i32,
  execution_sec: i32,
) -> ProcessStatData {
  ProcessStatData {
    pid,
    process_name: name.to_owned(),
    cpu_usage,
    memory_usage,
    execution_sec,
  }
}

/// Midnight UTC two days ago: late enough that a seven-day Retention Period
/// keeps it, early enough that the relative rows below still land on both
/// sides of that boundary.
fn anchor_day() -> chrono::DateTime<chrono::Utc> {
  (chrono::Utc::now() - chrono::Duration::days(2))
    .date_naive()
    .and_hms_opt(0, 0, 0)
    .expect("midnight is a valid time of day")
    .and_utc()
}

/// `2026-09-01T00:02:00Z`.
fn zulu(at: chrono::DateTime<chrono::Utc>) -> String {
  at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// `2026-09-01T00:00:00+00:00`.
fn utc_offset(at: chrono::DateTime<chrono::Utc>) -> String {
  at.format("%Y-%m-%dT%H:%M:%S+00:00").to_string()
}

/// `2026-09-01T00:01:00.125+00:00`.
fn utc_offset_millis(at: chrono::DateTime<chrono::Utc>) -> String {
  at.format("%Y-%m-%dT%H:%M:%S%.3f+00:00").to_string()
}

/// The same instant spelled in a non-UTC offset: `2026-09-01T09:03:00+09:00`.
fn east_nine(at: chrono::DateTime<chrono::Utc>) -> String {
  at.with_timezone(
    &chrono::FixedOffset::east_opt(9 * 3_600).expect("+09:00 is a valid offset"),
  )
  .format("%Y-%m-%dT%H:%M:%S%:z")
  .to_string()
}

async fn process_stats_row_count(pool: &SqlitePool) -> i64 {
  sqlx::query("SELECT COUNT(*) FROM PROCESS_STATS")
    .fetch_one(pool)
    .await
    .unwrap()
    .get(0)
}

async fn surviving_identities(
  database: &hardviz_core::infrastructure::database::native_database::NativeDatabase,
) -> Vec<(i64, String)> {
  database
    .request_read(NativeCancellation::new(), |context| {
      let mut statement = context
        .connection()
        .prepare("SELECT pid, process_name FROM PROCESS_STATS")
        .unwrap();
      let rows = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
      Ok(rows)
    })
    .await
    .unwrap()
}

/// Compare every field exactly, floats by their binary64 bits, and - when the
/// query was asked to rank - the sequence of equal-average bands, so a tie can
/// keep its members in either engine's order without hiding a real difference.
fn assert_records_match(
  expected: &[archive_queries::ProcessStatRecord],
  actual: &[archive_queries::ProcessStatRecord],
  start: &str,
  end: &str,
  order_by_cpu_desc: bool,
) {
  let context = format!("range {start}..{end} ordered={order_by_cpu_desc}");
  let key = |record: &archive_queries::ProcessStatRecord| {
    (record.pid, record.process_name.clone())
  };
  let fields = |record: &archive_queries::ProcessStatRecord| {
    (
      record.avg_cpu_usage.to_bits(),
      record.avg_memory_usage.to_bits(),
      record.total_execution_sec,
      record.latest_timestamp.clone(),
    )
  };
  let expected_by_identity: BTreeMap<_, _> = expected
    .iter()
    .map(|record| (key(record), fields(record)))
    .collect();
  let actual_by_identity: BTreeMap<_, _> = actual
    .iter()
    .map(|record| (key(record), fields(record)))
    .collect();
  assert_eq!(expected.len(), expected_by_identity.len(), "{context}");
  assert_eq!(actual.len(), actual_by_identity.len(), "{context}");
  assert_eq!(actual_by_identity, expected_by_identity, "{context}");

  if order_by_cpu_desc {
    assert_eq!(rank_bands(expected), rank_bands(actual), "{context}");
  }
}

fn rank_bands(
  records: &[archive_queries::ProcessStatRecord],
) -> Vec<(u64, Vec<(i64, String)>)> {
  let mut bands: Vec<(u64, Vec<(i64, String)>)> = Vec::new();
  for record in records {
    let bits = record.avg_cpu_usage.to_bits();
    if bands.last().is_none_or(|band| band.0 != bits) {
      bands.push((bits, Vec::new()));
    }
    bands
      .last_mut()
      .expect("a band was just pushed")
      .1
      .push((record.pid, record.process_name.clone()));
  }
  for (_, identities) in &mut bands {
    identities.sort();
  }
  bands
}

#[derive(Clone, Copy, Debug)]
enum Order {
  Forward,
  Reverse,
  Interleaved,
}

fn process_input(order: Order) -> Vec<ProcessInput> {
  let mut rows = Vec::with_capacity(VECTOR_CROSSING_ROWS * 4);
  for index in 0..VECTOR_CROSSING_ROWS {
    let triplet = index % 3;
    let varied = ((index.wrapping_mul(2_654_435_761) % 1_000_003) as f32) / 10_000.03_f32;
    let timestamp = format!(
      "2026-01-01T00:{:02}:{:02}.{:03}Z",
      (index / 60) % 60,
      index % 60,
      index % 1_000
    );
    rows.push(ProcessInput {
      pid: 1,
      name: "ordinary-telemetry",
      cpu: match index % 257 {
        // A denormal and the smallest positive normal: still archive-plausible
        // magnitudes, unlike the cancellation fixture above.
        0 => f32::from_bits(1),
        1 => f32::MIN_POSITIVE,
        _ => varied,
      },
      memory: ((index % 2_001) * 4_096) as i64,
      ordinal: (rows.len() + 1) as i64,
      timestamp: timestamp.clone(),
    });
    rows.push(ProcessInput {
      pid: 2,
      name: "i64-memory",
      cpu: [100.0, 0.0, 0.5][triplet],
      // Memory far beyond i32 with a group sum beyond 2^53, so converting
      // the exact sum to binary64 has to round the same way on both sides.
      // The sum still fits i64: past that SQLite switches to approximate
      // summation and the native query refuses, so that region is outside
      // the claim (and outside what an `i32` writer can reach).
      memory: [
        i64::MAX / 10_000,
        i64::MAX / 10_000 - 3,
        i64::MIN / 10_000 + 2_052,
      ][triplet],
      ordinal: (rows.len() + 1) as i64,
      timestamp: timestamp.clone(),
    });
    rows.push(ProcessInput {
      pid: 3,
      name: "alternating",
      cpu: [1.0, -1.0, 0.5][triplet],
      memory: [1, -1, 0][triplet],
      ordinal: (rows.len() + 1) as i64,
      timestamp: timestamp.clone(),
    });
    rows.push(ProcessInput {
      pid: 4,
      name: "positive-rank-neighbor",
      cpu: 50.0,
      memory: 4_096,
      ordinal: (rows.len() + 1) as i64,
      timestamp,
    });
  }

  match order {
    Order::Forward => {}
    Order::Reverse => rows.reverse(),
    Order::Interleaved => rows.sort_by_key(|row| (row.ordinal % 257, row.ordinal)),
  }
  for (index, row) in rows.iter_mut().enumerate() {
    row.ordinal = (index + 1) as i64;
  }
  rows
}

async fn sqlite_aggregates(input: &[ProcessInput]) -> AggregateBits {
  let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
  sqlx::query(
    "CREATE TABLE PROCESS_STATS (
       id INTEGER PRIMARY KEY,
       pid INTEGER NOT NULL,
       process_name TEXT NOT NULL,
       cpu_usage REAL NOT NULL,
       memory_usage INTEGER NOT NULL
       ,timestamp TEXT NOT NULL
     )",
  )
  .execute(&pool)
  .await
  .unwrap();
  sqlx::query("CREATE INDEX idx_process_stats_timestamp ON PROCESS_STATS(timestamp)")
    .execute(&pool)
    .await
    .unwrap();

  for chunk in input.chunks(1_000) {
    let mut query = QueryBuilder::<Sqlite>::new(
      "INSERT INTO PROCESS_STATS (id, pid, process_name, cpu_usage, memory_usage, timestamp) ",
    );
    query.push_values(chunk, |mut values, row| {
      values
        .push_bind(row.ordinal)
        .push_bind(row.pid)
        .push_bind(row.name)
        .push_bind(row.cpu)
        .push_bind(row.memory)
        .push_bind(&row.timestamp);
    });
    query.build().execute(&pool).await.unwrap();
  }

  let rows = sqlx::query(
    "SELECT pid, process_name, AVG(cpu_usage), AVG(memory_usage)
     FROM PROCESS_STATS INDEXED BY idx_process_stats_timestamp
     WHERE timestamp BETWEEN '2026-01-01T00:00:00.000Z' AND '2026-01-01T00:59:59.999Z'
     GROUP BY pid, process_name
     ORDER BY AVG(cpu_usage) DESC",
  )
  .fetch_all(&pool)
  .await
  .unwrap()
  .into_iter()
  .map(|row| {
    (
      row.get::<i64, _>(0),
      row.get::<String, _>(1),
      row.get::<f64, _>(2),
      row.get::<f64, _>(3),
    )
  })
  .collect::<Vec<_>>();
  aggregate_bits(rows)
}

fn duckdb_aggregates(input: &[ProcessInput], threads: i64) -> AggregateBits {
  let config = Config::default().threads(threads).unwrap();
  let connection = Connection::open_in_memory_with_flags(config).unwrap();
  connection
    .execute_batch(
      "CREATE TABLE PROCESS_STATS (
         id BIGINT PRIMARY KEY,
         pid BIGINT NOT NULL,
         process_name VARCHAR NOT NULL,
         cpu_usage DOUBLE NOT NULL,
         memory_usage BIGINT NOT NULL,
         timestamp VARCHAR NOT NULL
       )",
    )
    .unwrap();
  connection
    .execute_batch("CREATE INDEX idx_process_stats_timestamp ON PROCESS_STATS(timestamp)")
    .unwrap();
  {
    let mut appender = connection.appender("PROCESS_STATS").unwrap();
    for row in input {
      appender
        .append_row(params![
          row.ordinal,
          row.pid,
          row.name,
          f64::from(row.cpu),
          row.memory,
          &row.timestamp
        ])
        .unwrap();
    }
    appender.flush().unwrap();
  }

  // The same shape as the production native query: DuckDB's `AVG` for the
  // binary64 column, the exact sum and count for the integer column.
  let mut statement = connection
    .prepare(
      "SELECT pid, process_name, AVG(cpu_usage),
              SUM(memory_usage), COUNT(memory_usage)
       FROM PROCESS_STATS
       WHERE timestamp BETWEEN '2026-01-01T00:00:00.000Z' AND '2026-01-01T00:59:59.999Z'
       GROUP BY pid, process_name
       ORDER BY AVG(cpu_usage) DESC",
    )
    .unwrap();
  let rows = statement
    .query_map([], |row| {
      Ok((
        row.get::<_, i64>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, f64>(2)?,
        native_process_stats::sqlite_integer_average(
          i64::try_from(row.get::<_, i128>(3)?).expect("fixture sums fit i64"),
          row.get::<_, i64>(4)?,
        ),
      ))
    })
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();
  aggregate_bits(rows)
}

fn aggregate_bits(rows: Vec<(i64, String, f64, f64)>) -> AggregateBits {
  let by_identity = rows
    .iter()
    .map(|(pid, name, cpu, memory)| {
      ((*pid, name.clone()), (cpu.to_bits(), memory.to_bits()))
    })
    .collect();
  let mut cpu_rank_bands: Vec<(u64, Vec<(i64, String)>)> = Vec::new();
  for (pid, name, cpu, _) in rows {
    let bits = cpu.to_bits();
    if cpu_rank_bands.last().is_none_or(|band| band.0 != bits) {
      cpu_rank_bands.push((bits, Vec::new()));
    }
    cpu_rank_bands
      .last_mut()
      .expect("a band was just pushed")
      .1
      .push((pid, name));
  }
  for (_, identities) in &mut cpu_rank_bands {
    identities.sort();
  }
  AggregateBits {
    by_identity,
    cpu_rank_bands,
  }
}

fn describe_mismatches(expected: &AggregateBits, actual: &AggregateBits) -> String {
  let mut differences = expected
    .by_identity
    .iter()
    .filter_map(|(identity, expected_bits)| {
      let actual_bits = actual.by_identity.get(identity);
      (actual_bits != Some(expected_bits)).then(|| {
        format!(
          "{identity:?}: SQLite(cpu={:016x},memory={:016x}) DuckDB={actual_bits:?}",
          expected_bits.0, expected_bits.1
        )
      })
    })
    .collect::<Vec<_>>();
  if expected.cpu_rank_bands != actual.cpu_rank_bands {
    differences.push(format!(
      "rank bands: SQLite={:?} DuckDB={:?}",
      expected.cpu_rank_bands, actual.cpu_rank_bands
    ));
  }
  differences.join("; ")
}
