//! The Process Stats family against a finalized native database.
//!
//! These run *beside* [`crate::infrastructure::database::process_stats`] and
//! [`crate::infrastructure::database::archive_queries::select_process_stats`],
//! not instead of them. SQLite stays authoritative and keeps serving the
//! application; choosing between the two backends is a separate change. Keeping
//! both callable is what lets a test put the same fixture through each path and
//! compare the results bit for bit.

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use duckdb::params;

use super::NativeDatabaseError;
use super::runtime::{NativeCancellation, NativeDatabase};
use crate::infrastructure::database::archive_queries::ProcessStatRecord;
use crate::persistence::archive_data::ProcessStatData;

const TABLE: &str = "PROCESS_STATS";

/// Render an instant the way the SQLite writers store it.
///
/// The writers bind a `chrono::DateTime<Utc>`, and sqlx's SQLite encoder writes
/// `to_rfc3339_opts(SecondsFormat::AutoSi, false)` - so `+00:00` rather than
/// `Z`, and sub-second digits only when the instant has them. Every timestamp
/// comparison in this family is a byte-wise string comparison, so the native
/// writer has to produce the same bytes rather than an equivalent instant.
pub fn sqlite_timestamp_text(timestamp: &DateTime<Utc>) -> String {
  timestamp.to_rfc3339_opts(SecondsFormat::AutoSi, false)
}

/// Insert one write cycle's process rows, all stamped with the cycle's own
/// `timestamp`, in a single transaction - the same boundary and the same shared
/// instant as the SQLite writer.
pub async fn insert(
  database: &NativeDatabase,
  cancellation: NativeCancellation,
  processes: Vec<ProcessStatData>,
  timestamp: DateTime<Utc>,
) -> Result<(), NativeDatabaseError> {
  if processes.is_empty() {
    return Ok(());
  }
  let stamp = sqlite_timestamp_text(&timestamp);
  database
    .request_write(cancellation, move |context| {
      context.with_transaction(|transaction| {
        let connection = transaction.connection();
        let mut statement = connection
          .prepare(
            "INSERT INTO PROCESS_STATS \
             (id, pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
          )
          .map_err(|error| {
            NativeDatabaseError::duckdb("prepare the process stats insert", error)
          })?;
        for process in &processes {
          transaction.check_cancelled()?;
          let id = transaction.next_id(TABLE)?;
          statement
            .execute(params![
              id,
              i64::from(process.pid),
              process.process_name.as_str(),
              f64::from(process.cpu_usage),
              i64::from(process.memory_usage),
              i64::from(process.execution_sec),
              stamp.as_str()
            ])
            .map_err(|error| {
              NativeDatabaseError::duckdb("insert a process stats row", error)
            })?;
        }
        Ok(())
      })
    })
    .await
}

/// Delete rows older than the Retention Period, returning how many went.
///
/// SQLite evaluates `timestamp < $1` between a `DATETIME` column - which has
/// NUMERIC affinity - and a bound parameter, which has none, so it applies
/// NUMERIC affinity to the parameter. The rendered ISO-8601 text is not a
/// well-formed number, so it stays TEXT and the comparison runs under the
/// BINARY collation: byte-wise, not chronological. DuckDB compares VARCHAR by
/// the same byte order, and the candidate admits only valid UTF-8, so binding
/// the identically rendered text reproduces the rule exactly instead of
/// approximating it with an instant comparison.
pub async fn delete_old_data(
  database: &NativeDatabase,
  cancellation: NativeCancellation,
  retention_days: u32,
) -> Result<u64, NativeDatabaseError> {
  let bound =
    sqlite_timestamp_text(&(Utc::now() - Duration::days(i64::from(retention_days))));
  database
    .request_write(cancellation, move |context| {
      context.with_transaction(|transaction| {
        let deleted = transaction
          .connection()
          .execute(
            "DELETE FROM PROCESS_STATS WHERE timestamp < ?",
            params![bound.as_str()],
          )
          .map_err(|error| {
            NativeDatabaseError::duckdb("delete expired process stats rows", error)
          })?;
        Ok(deleted as u64)
      })
    })
    .await
}

/// The native form of
/// [`crate::infrastructure::database::archive_queries::select_process_stats`].
///
/// `BETWEEN` keeps the SQLite endpoint semantics: both bounds are inclusive and
/// compared as text, so a caller's rendering of an instant selects exactly the
/// stored spellings it would have selected in SQLite. Grouping stays on the
/// recorded `(pid, process_name)` tuple - the Process identity ADR 0019 defines
/// - rather than on the row id.
pub async fn select_process_stats(
  database: &NativeDatabase,
  cancellation: NativeCancellation,
  start: String,
  end: String,
  order_by_cpu_desc: bool,
) -> Result<Vec<ProcessStatRecord>, NativeDatabaseError> {
  let order_by = if order_by_cpu_desc {
    " ORDER BY avg_cpu_usage DESC"
  } else {
    ""
  };
  let sql = format!(
    "SELECT
       pid,
       process_name,
       AVG(cpu_usage) AS avg_cpu_usage,
       AVG(memory_usage) AS avg_memory_usage,
       MAX(execution_sec) AS total_execution_sec,
       MAX(timestamp) AS latest_timestamp
     FROM PROCESS_STATS
     WHERE timestamp BETWEEN ? AND ?
     GROUP BY pid, process_name{order_by}"
  );
  database
    .request_read(cancellation, move |context| {
      context.check_cancelled()?;
      let mut statement = context.connection().prepare(&sql).map_err(|error| {
        NativeDatabaseError::duckdb("prepare the process stats query", error)
      })?;
      let rows = statement
        .query_map(params![start.as_str(), end.as_str()], |row| {
          Ok(ProcessStatRecord {
            pid: row.get(0)?,
            process_name: row.get(1)?,
            avg_cpu_usage: row.get(2)?,
            avg_memory_usage: row.get(3)?,
            total_execution_sec: row.get(4)?,
            latest_timestamp: row.get(5)?,
          })
        })
        .map_err(|error| {
          NativeDatabaseError::duckdb("run the process stats query", error)
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
          NativeDatabaseError::duckdb("decode a process stats row", error)
        })?;
      Ok(rows)
    })
    .await
}

#[cfg(test)]
mod tests {
  use super::*;

  /// The exact bytes sqlx stores are pinned by
  /// `the_native_process_stats_family_reproduces_the_sqlite_family` in
  /// `core/tests/duckdb_avg_compatibility.rs`, which reads them back from a
  /// real SQLite database. This only pins the shapes the renderer must produce.
  #[test]
  fn renders_the_sqlx_datetime_shapes() {
    let whole = "2026-09-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
    assert_eq!(sqlite_timestamp_text(&whole), "2026-09-01T00:00:00+00:00");
    let millis = "2026-09-01T00:00:00.125Z".parse::<DateTime<Utc>>().unwrap();
    assert_eq!(
      sqlite_timestamp_text(&millis),
      "2026-09-01T00:00:00.125+00:00"
    );
    let micros = "2026-09-01T00:00:00.000125Z"
      .parse::<DateTime<Utc>>()
      .unwrap();
    assert_eq!(
      sqlite_timestamp_text(&micros),
      "2026-09-01T00:00:00.000125+00:00"
    );
  }
}
