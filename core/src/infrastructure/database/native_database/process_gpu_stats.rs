//! Per-process GPU usage, the first native-only family (ADR 0030, #2720).
//!
//! `PROCESS_GPU_STATS` has no SQLite counterpart: while SQLite is
//! authoritative the dispatch boundary answers
//! [`crate::infrastructure::database::dispatch::NativeOnly::NeedsNativeDatabase`]
//! instead of reaching these functions.
//!
//! Each row is a companion of the `PROCESS_STATS` row written for the same
//! process in the same write cycle. Both are stamped with the cycle's
//! [`sqlite_timestamp_text`], so ADR 0019's Process identity, the stamp with
//! `(pid, process_name)`, pairs them, and both families expire under the same
//! byte-wise text rule.

use chrono::{DateTime, Duration, Utc};
use duckdb::params;

use super::NativeDatabaseError;
use super::binding::required_real;
use super::process_stats::sqlite_timestamp_text;
use super::runtime::{NativeCancellation, NativeDatabase};
use crate::infrastructure::database::archive_queries::{
  ProcessGpuStatRecord, ProcessGpuStats,
};
use crate::persistence::archive_data::ProcessGpuStatData;

const TABLE: &str = "PROCESS_GPU_STATS";

/// Insert one write cycle's GPU rows, all stamped with the cycle's own
/// `timestamp`, in a single transaction.
pub async fn insert(
  database: &NativeDatabase,
  cancellation: NativeCancellation,
  rows: Vec<ProcessGpuStatData>,
  timestamp: DateTime<Utc>,
) -> Result<(), NativeDatabaseError> {
  if rows.is_empty() {
    return Ok(());
  }
  let stamp = sqlite_timestamp_text(&timestamp);
  database
    .request_write(cancellation, move |context| {
      context.with_transaction(|transaction| {
        let connection = transaction.connection();
        let mut statement = connection
          .prepare(
            "INSERT INTO PROCESS_GPU_STATS \
             (id, pid, process_name, gpu_usage, covered_seconds, timestamp) \
             VALUES (?, ?, ?, ?, ?, ?)",
          )
          .map_err(|error| {
            NativeDatabaseError::duckdb("prepare the process GPU stats insert", error)
          })?;
        for row in &rows {
          transaction.check_cancelled()?;
          let id = transaction.next_id(TABLE)?;
          statement
            .execute(params![
              id,
              i64::from(row.pid),
              row.process_name.as_str(),
              required_real(TABLE, "gpu_usage", row.gpu_usage)?,
              required_real(TABLE, "covered_seconds", row.covered_seconds)?,
              stamp.as_str()
            ])
            .map_err(|error| {
              NativeDatabaseError::duckdb("insert a process GPU stats row", error)
            })?;
        }
        Ok(())
      })
    })
    .await
}

/// Delete rows older than the Retention Period, returning how many went.
///
/// The bound is rendered and compared exactly as
/// [`super::process_stats::delete_old_data`] does - byte-wise on the stored
/// text - so a minute's GPU rows expire together with its `PROCESS_STATS`
/// rows.
pub async fn delete_old_data(
  database: &NativeDatabase,
  cancellation: NativeCancellation,
  retention_days: u32,
) -> Result<u64, NativeDatabaseError> {
  let bound =
    sqlite_timestamp_text(&(Utc::now() - Duration::days(i64::from(retention_days))));
  delete_before(database, cancellation, bound).await
}

async fn delete_before(
  database: &NativeDatabase,
  cancellation: NativeCancellation,
  bound: String,
) -> Result<u64, NativeDatabaseError> {
  database
    .request_write(cancellation, move |context| {
      context.with_transaction(|transaction| {
        let deleted = transaction
          .connection()
          .execute(
            "DELETE FROM PROCESS_GPU_STATS WHERE timestamp < ?",
            params![bound.as_str()],
          )
          .map_err(|error| {
            NativeDatabaseError::duckdb("delete expired process GPU stats rows", error)
          })?;
        Ok(deleted as u64)
      })
    })
    .await
}

/// GPU usage per Process between `start` and `end`, and when GPU usage was
/// first recorded at all.
///
/// `BETWEEN` compares the stored text with both bounds inclusive, the same
/// endpoint rule as
/// [`super::process_stats::select_process_stats`], so one range selects the
/// same minutes from both families. Each minute is weighted by how much of it
/// the readings covered, so a minute a process ran for ten seconds counts a
/// sixth of a fully covered one. `recorded_since` ignores the range: it is the
/// earliest row in the table, which tells a range before it apart from a range
/// in which nothing used the GPU.
pub async fn select_process_gpu_stats(
  database: &NativeDatabase,
  cancellation: NativeCancellation,
  start: String,
  end: String,
) -> Result<ProcessGpuStats, NativeDatabaseError> {
  database
    .request_read(cancellation, move |context| {
      context.check_cancelled()?;
      let connection = context.connection();
      let mut statement = connection
        .prepare(
          "SELECT
             pid,
             process_name,
             SUM(gpu_usage * covered_seconds) / SUM(covered_seconds) AS avg_gpu_usage,
             SUM(covered_seconds) AS covered_seconds
           FROM PROCESS_GPU_STATS
           WHERE timestamp BETWEEN ? AND ?
           GROUP BY pid, process_name
           ORDER BY pid, process_name",
        )
        .map_err(|error| {
          NativeDatabaseError::duckdb("prepare the process GPU stats query", error)
        })?;
      let records = statement
        .query_map(params![start.as_str(), end.as_str()], |row| {
          Ok(ProcessGpuStatRecord {
            pid: row.get(0)?,
            process_name: row.get(1)?,
            avg_gpu_usage: row.get(2)?,
            covered_seconds: row.get(3)?,
          })
        })
        .map_err(|error| {
          NativeDatabaseError::duckdb("run the process GPU stats query", error)
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
          NativeDatabaseError::duckdb("decode a process GPU stats row", error)
        })?;
      // An aggregate always yields one row; it is NULL for an empty table.
      let recorded_since = connection
        .query_row("SELECT MIN(timestamp) FROM PROCESS_GPU_STATS", [], |row| {
          row.get::<_, Option<String>>(0)
        })
        .map_err(|error| {
          NativeDatabaseError::duckdb(
            "read when process GPU usage was first recorded",
            error,
          )
        })?;
      Ok(ProcessGpuStats {
        records,
        recorded_since,
      })
    })
    .await
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::infrastructure::database::native_database::{
    AuthorityPaths, NativeDatabaseOptions, NativeIdentity, NativeIdentityMode,
    NativeSchemaDefinition, create_empty_native_database,
  };

  static IDENTITIES: &[NativeIdentity] = &[NativeIdentity {
    table: TABLE,
    column: "id",
    mode: NativeIdentityMode::NativeOnly,
  }];

  /// The family's table alone, with the production column shape.
  fn schema() -> NativeSchemaDefinition {
    NativeSchemaDefinition {
      version: 1,
      sql: "CREATE TABLE PROCESS_GPU_STATS (id BIGINT PRIMARY KEY, pid BIGINT NOT NULL, \
            process_name VARCHAR NOT NULL, gpu_usage DOUBLE NOT NULL, \
            covered_seconds DOUBLE NOT NULL, timestamp VARCHAR NOT NULL)",
      tables: &[],
      native_only_tables: &[TABLE],
      timestamp_columns: &[],
      identities: IDENTITIES,
      upgrades: &[],
    }
  }

  async fn database(directory: &std::path::Path) -> NativeDatabase {
    let paths =
      AuthorityPaths::in_directory(directory, "hv-database.db", "hv-database.duckdb");
    create_empty_native_database(paths.clone(), schema())
      .await
      .unwrap();
    NativeDatabase::open(&paths.native_database, NativeDatabaseOptions::new(1))
      .await
      .unwrap()
  }

  fn row(
    pid: i32,
    process_name: &str,
    gpu_usage: f64,
    covered_seconds: f64,
  ) -> ProcessGpuStatData {
    ProcessGpuStatData {
      pid,
      process_name: process_name.to_owned(),
      gpu_usage,
      covered_seconds,
    }
  }

  fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
  }

  async fn select(database: &NativeDatabase, start: &str, end: &str) -> ProcessGpuStats {
    select_process_gpu_stats(
      database,
      NativeCancellation::new(),
      start.to_owned(),
      end.to_owned(),
    )
    .await
    .unwrap()
  }

  async fn count(database: &NativeDatabase) -> i64 {
    database
      .request_read(NativeCancellation::new(), |context| {
        Ok(
          context
            .connection()
            .query_row("SELECT count(*) FROM PROCESS_GPU_STATS", [], |row| {
              row.get(0)
            })
            .unwrap(),
        )
      })
      .await
      .unwrap()
  }

  #[tokio::test]
  async fn an_empty_table_has_no_records_and_no_recorded_since() {
    let directory = tempfile::tempdir().unwrap();
    let database = database(directory.path()).await;

    let stats = select(
      &database,
      "2026-09-01T00:00:00+00:00",
      "2026-09-02T00:00:00+00:00",
    )
    .await;

    assert_eq!(
      stats,
      ProcessGpuStats {
        records: Vec::new(),
        recorded_since: None,
      }
    );
    database.close().await.unwrap();
  }

  #[tokio::test]
  async fn usage_is_weighted_by_coverage_per_process_identity() {
    let directory = tempfile::tempdir().unwrap();
    let database = database(directory.path()).await;
    for (stamp, rows) in [
      (
        "2026-09-01T00:00:00Z",
        vec![row(42, "renderer", 80.0, 60.0), row(7, "idle", 0.0, 60.0)],
      ),
      (
        "2026-09-01T00:01:00Z",
        // A partially covered minute counts for its coverage only, and a
        // reading above 100 (several engines) is kept as it is.
        vec![
          row(42, "renderer", 140.0, 30.0),
          row(42, "reused-pid", 50.0, 15.0),
        ],
      ),
      (
        "2026-09-01T00:02:00Z",
        vec![row(42, "renderer", 10.0, 60.0)],
      ),
    ] {
      insert(&database, NativeCancellation::new(), rows, at(stamp))
        .await
        .unwrap();
    }

    // Both bounds inclusive: the first and second minutes, not the third.
    let stats = select(
      &database,
      "2026-09-01T00:00:00+00:00",
      "2026-09-01T00:01:00+00:00",
    )
    .await;

    assert_eq!(
      stats.recorded_since.as_deref(),
      Some("2026-09-01T00:00:00+00:00")
    );
    assert_eq!(
      stats.records,
      vec![
        ProcessGpuStatRecord {
          pid: 7,
          process_name: "idle".to_owned(),
          avg_gpu_usage: 0.0,
          covered_seconds: 60.0,
        },
        ProcessGpuStatRecord {
          pid: 42,
          process_name: "renderer".to_owned(),
          avg_gpu_usage: (80.0 * 60.0 + 140.0 * 30.0) / 90.0,
          covered_seconds: 90.0,
        },
        ProcessGpuStatRecord {
          pid: 42,
          process_name: "reused-pid".to_owned(),
          avg_gpu_usage: 50.0,
          covered_seconds: 15.0,
        },
      ]
    );

    // `recorded_since` is the table's earliest row whatever the range.
    let later = select(
      &database,
      "2026-09-01T00:02:00+00:00",
      "2026-09-01T00:02:00+00:00",
    )
    .await;
    assert_eq!(later.records.len(), 1);
    assert_eq!(
      later.recorded_since.as_deref(),
      Some("2026-09-01T00:00:00+00:00")
    );
    database.close().await.unwrap();
  }

  #[tokio::test]
  async fn rows_strictly_before_the_bound_expire() {
    let directory = tempfile::tempdir().unwrap();
    let database = database(directory.path()).await;
    for stamp in [
      "2026-09-01T00:00:00Z",
      "2026-09-01T00:01:00Z",
      "2026-09-01T00:02:00Z",
    ] {
      insert(
        &database,
        NativeCancellation::new(),
        vec![row(1, "process", 5.0, 60.0)],
        at(stamp),
      )
      .await
      .unwrap();
    }

    let deleted = delete_before(
      &database,
      NativeCancellation::new(),
      sqlite_timestamp_text(&at("2026-09-01T00:01:00Z")),
    )
    .await
    .unwrap();

    assert_eq!(deleted, 1);
    assert_eq!(count(&database).await, 2);
    let stats = select(
      &database,
      "2026-09-01T00:00:00+00:00",
      "2026-09-02T00:00:00+00:00",
    )
    .await;
    assert_eq!(
      stats.recorded_since.as_deref(),
      Some("2026-09-01T00:01:00+00:00")
    );

    // The public form renders "now minus the Retention Period": everything
    // written in 2026-09 is long past a one-day window.
    assert_eq!(
      delete_old_data(&database, NativeCancellation::new(), 1)
        .await
        .unwrap(),
      2
    );
    assert_eq!(count(&database).await, 0);
    database.close().await.unwrap();
  }

  #[tokio::test]
  async fn ids_are_allocated_from_the_native_only_identity() {
    let directory = tempfile::tempdir().unwrap();
    let database = database(directory.path()).await;
    insert(
      &database,
      NativeCancellation::new(),
      vec![row(1, "a", 1.0, 60.0), row(2, "b", 2.0, 60.0)],
      at("2026-09-01T00:00:00Z"),
    )
    .await
    .unwrap();

    let ids = database
      .request_read(NativeCancellation::new(), |context| {
        let mut statement = context
          .connection()
          .prepare("SELECT id FROM PROCESS_GPU_STATS ORDER BY id")
          .unwrap();
        Ok(
          statement
            .query_map([], |row| row.get::<_, i64>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
        )
      })
      .await
      .unwrap();
    assert_eq!(ids, vec![1, 2]);
    database.close().await.unwrap();
  }

  #[tokio::test]
  async fn a_nan_reading_is_refused_and_nothing_is_written() {
    let directory = tempfile::tempdir().unwrap();
    let database = database(directory.path()).await;

    let error = insert(
      &database,
      NativeCancellation::new(),
      vec![row(1, "a", 1.0, 60.0), row(2, "b", f64::NAN, 60.0)],
      at("2026-09-01T00:00:00Z"),
    )
    .await
    .unwrap_err();

    assert!(matches!(
      error,
      NativeDatabaseError::NotANumberInRequiredColumn {
        column: "gpu_usage",
        ..
      }
    ));
    assert_eq!(count(&database).await, 0);
    database.close().await.unwrap();
  }
}
