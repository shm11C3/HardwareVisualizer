//! The bucketed archive series query against a finalized native database.
//!
//! Shared by the DATA_ARCHIVE and GPU_DATA_ARCHIVE families because the two
//! SQLite queries they reproduce differ only in the table, the filtered column
//! and how one measurement column is spelled - everything that decides what a
//! point *means* (the bucket grid, the endpoints, the gap convention and the
//! point limit) is the same rule, and a second copy of it would be a second
//! place for it to drift.
//!
//! The bucket grid is not recomputed here either: the bounds, the gap filling
//! and the `MAX_ARCHIVE_SERIES_POINTS` refusal come from
//! [`crate::infrastructure::database::archive_queries`], so the native series
//! can only differ from the SQLite one in the aggregate it reads out of the
//! engine.

use chrono::{DateTime, Utc};
use duckdb::types::Value;

use super::NativeDatabaseError;
use super::cell::quote_identifier;
use super::runtime::{NativeCancellation, NativeDatabase};
use crate::infrastructure::database::archive_queries::{
  AggregatedArchiveBucket, ArchiveAggregation, ArchiveBucketTimestamp,
  ArchiveSeriesBounds, ArchiveSeriesPoint, fill_archive_series,
};

/// The range a caller asked for and the grid it asked for it on - the four
/// arguments every archive series query takes, whichever family it belongs to.
///
/// They travel together because they are only meaningful together: the bucket
/// grid is what turns a range into points, and validating one without the
/// others is what produced the refusals the families share.
#[derive(Debug, Clone, Copy)]
pub struct NativeSeriesWindow<'a> {
  pub start: &'a DateTime<Utc>,
  pub end: &'a DateTime<Utc>,
  pub bucket_width_ms: i64,
  pub bucket_timestamp: ArchiveBucketTimestamp,
}

impl NativeSeriesWindow<'_> {
  /// The grid, built by `archive_queries` so an invalid range, a non-positive
  /// bucket width and an over-long series are refused before any engine work,
  /// for exactly the SQLite path's reason - the native families return their
  /// own error type, so that reason travels wrapped rather than restated.
  pub(super) fn bounds(&self) -> Result<ArchiveSeriesBounds, NativeDatabaseError> {
    ArchiveSeriesBounds::new(
      self.start,
      self.end,
      self.bucket_width_ms,
      self.bucket_timestamp,
    )
    .map_err(|source| NativeDatabaseError::ArchiveSeries { source })
  }
}

/// The derived query key finalization fills from the stored timestamp text.
pub(super) const EPOCH_COLUMN: &str = "__hv_timestamp_epoch_ms";

pub(super) struct NativeSeriesQuery {
  pub(super) table: &'static str,
  /// The native equivalent of the SQLite query's `CAST(<column> AS REAL)`.
  pub(super) value_expression: String,
  pub(super) aggregation: ArchiveAggregation,
  /// The `WHERE` body, consuming `parameters` in order.
  pub(super) predicate: String,
  pub(super) parameters: Vec<Value>,
  pub(super) bucket_timestamp: ArchiveBucketTimestamp,
  pub(super) bucket_width_ms: i64,
}

/// Run one bucketed series query and fill its gaps.
///
/// `bounds` is built by the caller before any engine work, so an invalid range,
/// a non-positive bucket width and an over-long series are refused without
/// opening a request, carrying the SQLite path's own `ArchiveSeriesError`
/// inside `NativeDatabaseError::ArchiveSeries`.
pub(super) async fn select_series(
  database: &NativeDatabase,
  cancellation: NativeCancellation,
  query: NativeSeriesQuery,
  bounds: ArchiveSeriesBounds,
) -> Result<Vec<ArchiveSeriesPoint>, NativeDatabaseError> {
  let rows = database
    .request_read(cancellation, move |context| {
      context.check_cancelled()?;
      query.run(context.connection())
    })
    .await?;
  Ok(fill_archive_series(rows, bounds))
}

impl NativeSeriesQuery {
  fn run(
    &self,
    connection: &duckdb::Connection,
  ) -> Result<Vec<AggregatedArchiveBucket>, NativeDatabaseError> {
    let sql = format!(
      "SELECT {bucket} AS bucket,
              {aggregation}({value}) AS value,
              COUNT({value}) AS value_count
       FROM {table}
       WHERE {predicate}
       GROUP BY 1
       ORDER BY 1 ASC",
      bucket = self.bucket_expression(),
      aggregation = self.aggregation.sql(),
      value = self.value_expression,
      table = quote_identifier(self.table),
      predicate = self.predicate,
    );
    let mut statement = connection.prepare(&sql).map_err(|error| {
      NativeDatabaseError::duckdb("prepare the archive series query", error)
    })?;
    let rows = statement
      .query_map(duckdb::params_from_iter(self.parameters.iter()), |row| {
        Ok((
          row.get::<_, Option<i64>>(0)?,
          row.get::<_, Option<f64>>(1)?,
          row.get::<_, i64>(2)?,
        ))
      })
      .map_err(|error| {
        NativeDatabaseError::duckdb("run the archive series query", error)
      })?
      .collect::<Result<Vec<_>, _>>()
      .map_err(|error| {
        NativeDatabaseError::duckdb("decode an archive series bucket", error)
      })?;

    let mut buckets = Vec::with_capacity(rows.len());
    for (bucket, value, value_count) in rows {
      // A row whose stored text SQLite cannot read as an instant has no derived
      // key and therefore no bucket. This is the one place the native family
      // deliberately does not reproduce the SQLite one: SQLite groups such a
      // row under a NULL bucket, sqlx decodes that NULL as 0, and bucket 0 is
      // outside every range a user can ask for - so the row is inside the
      // requested range and silently absent from the answer (measured in
      // `duckdb_data_archive`). Refusing names the row a maintainer has to fix
      // instead of under-reporting the range without saying so.
      let Some(timestamp) = bucket else {
        return Err(NativeDatabaseError::UnreadableTimestamp {
          table: self.table,
          timestamp: self.unreadable_timestamp(connection)?,
        });
      };
      buckets.push(AggregatedArchiveBucket {
        timestamp,
        value,
        value_count,
      });
    }
    Ok(buckets)
  }

  /// One stored timestamp text from the requested range that has no derived
  /// key, so the refusal can name the value a maintainer has to look at.
  fn unreadable_timestamp(
    &self,
    connection: &duckdb::Connection,
  ) -> Result<String, NativeDatabaseError> {
    let sql = format!(
      "SELECT \"timestamp\" FROM {table}
       WHERE ({predicate}) AND {epoch} IS NULL
       ORDER BY \"timestamp\" ASC
       LIMIT 1",
      table = quote_identifier(self.table),
      predicate = self.predicate,
      epoch = quote_identifier(EPOCH_COLUMN),
    );
    connection
      .query_row(
        &sql,
        duckdb::params_from_iter(self.parameters.iter()),
        |row| row.get::<_, String>(0),
      )
      .map_err(|error| {
        NativeDatabaseError::duckdb("read an unreadable archive timestamp", error)
      })
  }

  /// The bucket grid, computed from the derived epoch key exactly as
  /// `archive_queries::bucket_of_epoch_sql` computes it from the adapter
  /// expression.
  ///
  /// SQLite's `/` between integers truncates toward zero, while DuckDB's `//`
  /// is only specified to be integer division; pre-epoch rows are where the
  /// two conventions would part. The operands are made non-negative first, so
  /// the grid is the source's regardless of which convention `//` follows.
  fn bucket_expression(&self) -> String {
    let epoch = quote_identifier(EPOCH_COLUMN);
    let width = self.bucket_width_ms;
    let numerator = match self.bucket_timestamp {
      ArchiveBucketTimestamp::Start => epoch,
      ArchiveBucketTimestamp::End => format!("({epoch} + {width} - 1)"),
    };
    format!(
      "(CASE WHEN {numerator} < 0 \
       THEN -((-({numerator})) // {width}) \
       ELSE {numerator} // {width} END) * {width}"
    )
  }
}
