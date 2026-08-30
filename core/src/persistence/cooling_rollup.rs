//! Cooling daily rollup worker.
//!
//! Derives a long-lived per-local-day cooling summary from the one-minute
//! Hardware Archive rows (`DATA_ARCHIVE`) so Cooling Insight can show
//! 90-day and 1-year trends without loading a year of per-minute rows and
//! without extending the Hardware Archive Retention Period (see
//! `crate::persistence::archive`). The rollup keeps its own retention
//! contract ([`COOLING_DAILY_SUMMARY_RETENTION_DAYS`]), independent of
//! `hardwareArchive.retentionDays`.
//!
//! Each one-minute archive row is classified into a [`CpuLoadBand`] by its
//! average CPU usage for that minute, then folded into that band's
//! same-day temperature statistics. A minute with no CPU usage reading or
//! no CPU temperature reading contributes nothing; a day with zero
//! archived minutes stays absent from the table entirely rather than
//! becoming a zeroed-out row (see `summarize_day`).

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};

use tokio::runtime::Handle;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};

use crate::{log_error, log_info};

/// Retention for `cooling_daily_summary` rows. Deliberately independent of
/// `hardwareArchive.retentionDays` (default 30 days): the whole point of
/// the daily rollup is to outlive the one-minute archive rows it is
/// derived from, so Cooling Insight can show 90-day and 1-year trends.
/// ~400 days covers slightly more than a year of daily rows.
pub const COOLING_DAILY_SUMMARY_RETENTION_DAYS: u32 = 400;

/// How often the worker checks whether the local calendar day has
/// advanced. The rollup only ever acts on completed days (yesterday and
/// earlier), so sub-minute precision is unnecessary; hourly is frequent
/// enough that a newly completed day is caught up well within the same
/// day, while staying cheap (the check is a single date-string compare
/// unless the day actually changed).
pub const COOLING_ROLLUP_CHECK_INTERVAL_SECONDS: u64 = 60 * 60;

/// CPU-load bands used to bucket each archived one-minute row before
/// computing per-band cooling temperature summaries. Boundaries are
/// `[low, high)` in percent, except [`CpuLoadBand::Idle`] and
/// [`CpuLoadBand::High`], which are open-ended on the low/high end
/// respectively so no reported usage value (including a >100% or
/// negative measurement artifact) is ever left unclassified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuLoadBand {
  /// `< 10%`
  Idle,
  /// `10% ..< 30%`
  Low,
  /// `30% ..< 60%`
  Mid,
  /// `>= 60%`
  High,
}

impl CpuLoadBand {
  pub fn classify(cpu_usage_percent: f32) -> Self {
    if cpu_usage_percent < 10.0 {
      Self::Idle
    } else if cpu_usage_percent < 30.0 {
      Self::Low
    } else if cpu_usage_percent < 60.0 {
      Self::Mid
    } else {
      Self::High
    }
  }
}

/// One archived one-minute row's fields relevant to the cooling rollup.
/// `None` means the minute has no reading for that field, not zero.
///
/// `timestamp` is the archived row's own instant. The daily rollup does
/// not need it (its caller already fetched exactly one local day), but the
/// hourly rollup derived from the same fetch does - see
/// [`crate::persistence::cooling_hourly_rollup`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArchiveMinuteSample {
  pub timestamp: DateTime<Utc>,
  pub cpu_usage_avg: Option<f32>,
  pub cpu_temperature_avg: Option<f32>,
  pub cpu_temperature_max: Option<f32>,
  pub cpu_temperature_min: Option<f32>,
}

/// CPU temperature summary for one [`CpuLoadBand`] on one local day.
/// `sample_minutes == 0` implies `avg`/`max`/`min` are all `None`: a band
/// with no contributing minute is absent, never zero.
/// `sample_minutes` counts contributing archived rows; a shutdown-flush
/// partial-window row counts as one minute (see
/// [`DailyCoolingSummary::coverage_minutes`]).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BandSummary {
  pub avg: Option<f32>,
  pub max: Option<f32>,
  pub min: Option<f32>,
  pub sample_minutes: u32,
}

/// Minutes in a local calendar day; the cap for [`DailyCoolingSummary::coverage_minutes`].
const MINUTES_PER_DAY: u32 = 24 * 60;

/// One `cooling_daily_summary` row: the derived cooling profile for a
/// single completed local day.
#[derive(Debug, Clone, PartialEq)]
pub struct DailyCoolingSummary {
  pub date: NaiveDate,
  /// Count of archived rows found for this local day, regardless of
  /// whether they carried a usable CPU usage or temperature reading.
  /// Represents how much of the day was actually recorded (the app
  /// running), distinct from `sample_minutes` per band.
  ///
  /// Approximation: an archived row is normally a one-minute window,
  /// but `ArchiveController` also flushes a final partial window on
  /// shutdown, so repeated short launches can produce more rows than
  /// elapsed minutes. The count is capped at [`MINUTES_PER_DAY`] so a
  /// day never reports more than a full day of coverage.
  pub coverage_minutes: u32,
  pub idle: BandSummary,
  pub low: BandSummary,
  pub mid: BandSummary,
  pub high: BandSummary,
}

/// Fold one local day's archived one-minute rows into a
/// [`DailyCoolingSummary`]. Returns `None` when `minutes` is empty: a day
/// with zero archived rows stays absent rather than becoming a zeroed row.
pub fn summarize_day(
  date: NaiveDate,
  minutes: &[ArchiveMinuteSample],
) -> Option<DailyCoolingSummary> {
  if minutes.is_empty() {
    return None;
  }

  let mut idle = BandAccumulator::default();
  let mut low = BandAccumulator::default();
  let mut mid = BandAccumulator::default();
  let mut high = BandAccumulator::default();

  for minute in minutes {
    // A minute without a CPU usage reading cannot be classified into any
    // band; a minute without a temperature reading has nothing to
    // contribute even once classified. Either way it contributes nothing
    // - but it was still archived, so it still counts toward
    // `coverage_minutes` below.
    let (
      Some(cpu_usage_avg),
      Some(temperature_avg),
      Some(temperature_max),
      Some(temperature_min),
    ) = (
      minute.cpu_usage_avg,
      minute.cpu_temperature_avg,
      minute.cpu_temperature_max,
      minute.cpu_temperature_min,
    )
    else {
      continue;
    };

    let band = match CpuLoadBand::classify(cpu_usage_avg) {
      CpuLoadBand::Idle => &mut idle,
      CpuLoadBand::Low => &mut low,
      CpuLoadBand::Mid => &mut mid,
      CpuLoadBand::High => &mut high,
    };
    band.push(temperature_avg, temperature_max, temperature_min);
  }

  Some(DailyCoolingSummary {
    date,
    coverage_minutes: (minutes.len() as u32).min(MINUTES_PER_DAY),
    idle: idle.finish(),
    low: low.finish(),
    mid: mid.finish(),
    high: high.finish(),
  })
}

/// Accumulates one [`CpuLoadBand`]'s temperature readings for a single
/// day. `avg` is the average of the per-minute averages (consistent with
/// how `archive_queries` already aggregates `DATA_ARCHIVE` rows, since
/// each row is itself already a one-minute average); `max`/`min` are the
/// extremes across the per-minute extremes.
#[derive(Default)]
struct BandAccumulator {
  sum: f64,
  count: u32,
  max: Option<f32>,
  min: Option<f32>,
}

impl BandAccumulator {
  fn push(&mut self, avg: f32, max: f32, min: f32) {
    self.sum += avg as f64;
    self.count += 1;
    self.max = Some(self.max.map_or(max, |current| current.max(max)));
    self.min = Some(self.min.map_or(min, |current| current.min(min)));
  }

  fn finish(self) -> BandSummary {
    BandSummary {
      avg: (self.count > 0).then(|| (self.sum / self.count as f64) as f32),
      max: self.max,
      min: self.min,
      sample_minutes: self.count,
    }
  }
}

/// Determine which local days still need a rollup.
///
/// - `last_summarized_date`: the latest `date` already present in
///   `cooling_daily_summary` (`MAX(date)`), or `None` if the table is
///   empty (no rollup has ever run).
/// - `earliest_archived_local_date`: the local calendar day of the
///   oldest `DATA_ARCHIVE` row, or `None` if the archive itself is empty.
///   Only consulted when `last_summarized_date` is `None`, so the very
///   first catch-up backfills every already-archived day instead of
///   silently skipping straight to yesterday.
/// - `yesterday`: the most recent local day that has fully completed.
///   The rollup never summarizes the current (incomplete) day.
///
/// Returns an ordered, inclusive day range with no gaps; empty when there
/// is nothing archived yet or the rollup is already caught up through
/// yesterday.
pub fn days_to_roll_up(
  last_summarized_date: Option<NaiveDate>,
  earliest_archived_local_date: Option<NaiveDate>,
  yesterday: NaiveDate,
) -> Vec<NaiveDate> {
  let start = match last_summarized_date {
    Some(last) => last.succ_opt().unwrap_or(last),
    None => match earliest_archived_local_date {
      Some(earliest) => earliest,
      // Nothing has ever been archived: there is nothing to catch up.
      None => return Vec::new(),
    },
  };

  let mut days = Vec::new();
  let mut day = start;
  while day <= yesterday {
    days.push(day);
    match day.succ_opt() {
      Some(next) => day = next,
      // `NaiveDate` upper bound reached - practically unreachable.
      None => break,
    }
  }
  days
}

/// `[start, end)` UTC instants covering local calendar day `date` in
/// timezone `zone`. `end` is the following local midnight, so the range
/// is a half-open interval matching the `timestamp >= start AND
/// timestamp < end` query it is used for.
pub(crate) fn day_utc_bounds_for_offset<Tz: TimeZone>(
  date: NaiveDate,
  zone: &Tz,
) -> (DateTime<Utc>, DateTime<Utc>) {
  let start_naive = date.and_hms_opt(0, 0, 0).expect("midnight is always valid");
  let end_naive = (date + Duration::days(1))
    .and_hms_opt(0, 0, 0)
    .expect("midnight is always valid");
  (
    local_naive_to_utc(start_naive, zone),
    local_naive_to_utc(end_naive, zone),
  )
}

/// Local calendar day of `zone` in which UTC instant `instant` falls.
pub(crate) fn utc_to_date_for_offset<Tz: TimeZone>(
  instant: DateTime<Utc>,
  zone: &Tz,
) -> NaiveDate {
  instant.with_timezone(zone).date_naive()
}

/// Resolve a naive local wall-clock time to a UTC instant, handling the
/// two DST edge cases `chrono` can return for `from_local_datetime`. An
/// ambiguous result (a fall-back overlap) picks the earliest of the two,
/// which is deterministic and good enough for a once-a-day boundary. A
/// `None` result (a spring-forward gap, meaning the local time never
/// occurred) falls back to treating the naive value as if it were
/// already UTC rather than panicking a persistence worker over a
/// vanishingly rare midnight edge case.
fn local_naive_to_utc<Tz: TimeZone>(
  naive: chrono::NaiveDateTime,
  zone: &Tz,
) -> DateTime<Utc> {
  match zone.from_local_datetime(&naive) {
    chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc),
    chrono::LocalResult::Ambiguous(earliest, _latest) => earliest.with_timezone(&Utc),
    chrono::LocalResult::None => DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc),
  }
}

/// [`day_utc_bounds_for_offset`] using the OS's configured local
/// timezone, matching the "local date" used throughout Core (see
/// `crate::persistence::storage_health::local_storage_health_date_string`).
pub fn local_day_utc_bounds(date: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>) {
  day_utc_bounds_for_offset(date, &chrono::Local)
}

/// [`utc_to_date_for_offset`] using the OS's configured local timezone.
pub fn utc_to_local_date(instant: DateTime<Utc>) -> NaiveDate {
  utc_to_date_for_offset(instant, &chrono::Local)
}

/// Background controller for the cooling rollup worker. Unlike
/// [`crate::persistence::archive::ArchiveController`], this worker does
/// not subscribe to the `EventBus` — it only reads already-persisted
/// archive rows and writes daily summaries, so it needs no live snapshot
/// stream.
pub struct CoolingRollupController {
  handle: JoinHandle<()>,
  stop_tx: watch::Sender<bool>,
}

impl CoolingRollupController {
  /// Spawn the rollup worker on `runtime`. Runs an immediate catch-up
  /// pass (covering every completed local day missed since the last
  /// rollup, or since the earliest archived day on a first run), then
  /// checks once per [`COOLING_ROLLUP_CHECK_INTERVAL_SECONDS`] whether
  /// the local day has advanced far enough to catch up again.
  ///
  /// Also returns a receiver that resolves with the *outcome* of that
  /// first catch-up pass, so callers can gate work that deletes archive
  /// rows — the `scheduledDataDeletion` retention cleanup — on the
  /// backfill having actually read the rows it needs. Rows older than
  /// the archive Retention Period can still be present at startup
  /// (cleanup was disabled, or the app was simply not running past the
  /// cutoff); racing the cleanup, or running it after a failed pass,
  /// would silently lose those days from the rollup forever. `false`
  /// (or a closed channel, meaning the worker died) tells the caller to
  /// skip this boot's cleanup and let the next boot retry both.
  pub fn setup(runtime: Handle) -> (Self, oneshot::Receiver<bool>) {
    let (stop_tx, mut stop_rx) = watch::channel(false);
    let (first_catch_up_tx, first_catch_up_rx) = oneshot::channel();

    let handle = runtime.spawn(async move {
      let first_pass_succeeded = run_catch_up().await;
      let _ = first_catch_up_tx.send(first_pass_succeeded);
      let mut last_checked_date = Some(chrono::Local::now().date_naive());

      let mut ticker = interval(tokio::time::Duration::from_secs(
        COOLING_ROLLUP_CHECK_INTERVAL_SECONDS,
      ));
      ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
      ticker.tick().await;

      loop {
        tokio::select! {
          biased;
          changed = stop_rx.changed() => {
            if changed.is_err() || *stop_rx.borrow() {
              log_info!(
                "cooling rollup worker shutdown signal received",
                "persistence::cooling_rollup",
                None::<&str>
              );
              break;
            }
          }
          _ = ticker.tick() => {
            let today = chrono::Local::now().date_naive();
            if last_checked_date != Some(today) {
              let _ = run_catch_up().await;
              last_checked_date = Some(today);
            }
          }
        }
      }
    });

    (Self { handle, stop_tx }, first_catch_up_rx)
  }

  pub async fn terminate(self) {
    let _ = self.stop_tx.send(true);
    let _ = self.handle.await;
  }
}

/// Run one catch-up pass; `true` when every pending day was rolled up.
async fn run_catch_up() -> bool {
  match catch_up_cooling_rollup().await {
    Ok(()) => true,
    Err(e) => {
      log_error!(
        "Failed to catch up cooling daily rollup",
        "persistence::cooling_rollup::run_catch_up",
        Some(e.to_string())
      );
      false
    }
  }
}

/// Where the catch-up must resume from, given how far each rollup
/// projection has actually got.
///
/// The daily cursor alone is not enough. `cooling_hourly_summary` was
/// added by a later migration, so an existing install starts with a full
/// daily table and an empty hourly one; resuming from the daily cursor
/// would leave every past day's hourly rows missing forever. Resuming from
/// the *slower* projection regenerates them, and re-running a day is
/// harmless because both upserts are idempotent.
///
/// "Slower", though, cannot just be `min`: a machine with no CPU
/// temperature sensor produces daily rows and no hourly rows at all, and a
/// plain `min` would make it re-read its whole archive on every cycle
/// forever. `last_pairable_daily_date` is the latest day that actually had
/// a (load, temperature) pair to record, which is exactly the latest day
/// hourly is *expected* to cover - so "hourly is behind" is only claimed
/// when there was something for it to miss.
///
/// A `None` result means "resume from the earliest archived day", the same
/// first-run backfill [`days_to_roll_up`] already performs. Only days the
/// one-minute archive still holds can be regenerated; older days no longer
/// have source rows, and reporting them as absent is the honest outcome.
pub fn rollup_catch_up_cursor(
  last_daily_date: Option<NaiveDate>,
  last_pairable_daily_date: Option<NaiveDate>,
  last_hourly_date: Option<NaiveDate>,
) -> Option<NaiveDate> {
  match last_pairable_daily_date {
    // No summarized day ever carried a pair, so the hourly rollup has
    // nothing it could be missing.
    None => last_daily_date,
    Some(pairable) => match last_hourly_date {
      // Hourly has reached every day that had something to record.
      Some(hourly) if hourly >= pairable => last_daily_date,
      // Hourly is genuinely behind: resume from where it actually got to.
      behind => behind,
    },
  }
}

async fn catch_up_cooling_rollup() -> Result<(), sqlx::Error> {
  use crate::infrastructure::database;

  let yesterday = chrono::Local::now().date_naive() - Duration::days(1);
  let last_summarized_date = rollup_catch_up_cursor(
    database::cooling_daily_summary::max_summarized_date().await?,
    database::cooling_daily_summary::max_pairable_summarized_date().await?,
    database::cooling_hourly_summary::max_summarized_date().await?,
  );
  let earliest_archived_local_date = match last_summarized_date {
    Some(_) => None,
    None => database::cooling_daily_summary::earliest_archived_timestamp()
      .await?
      .map(utc_to_local_date),
  };

  for date in days_to_roll_up(
    last_summarized_date,
    earliest_archived_local_date,
    yesterday,
  ) {
    roll_up_day(date).await?;
  }

  // Resolve (and, once established, pin) the baseline right after the
  // rollup advances, so establishment happens in the background rather
  // than only when Cooling Insight is read — otherwise a user who never
  // opens the view before retention cleanup erases the
  // establishment-window rows would get a drifted baseline pinned on
  // their first read.
  crate::persistence::cooling_baseline::ensure_baseline_pinned().await;

  Ok(())
}

/// Roll one completed local day up into both projections the Cooling
/// Insight queries read: the per-band daily summary and the per-hour
/// load/temperature pairs (#2023).
///
/// Both are folded from the *same* fetch rather than by a second worker or
/// a second query - they answer different questions about identical rows,
/// and the day's archive range has already been read here.
async fn roll_up_day(date: NaiveDate) -> Result<(), sqlx::Error> {
  use crate::infrastructure::database;

  let (start, end) = local_day_utc_bounds(date);
  let minutes =
    database::cooling_daily_summary::select_archive_minutes_for_range(&start, &end)
      .await?;

  // A day without samples stays absent - never insert a zeroed row.
  let summary = summarize_day(date, &minutes);
  let hours =
    crate::persistence::cooling_hourly_rollup::summarize_hours(&minutes, &chrono::Local);

  let pool = database::db::get_pool().await?;
  persist_day_rollup_from_pool(&pool, summary.as_ref(), &hours).await
}

/// Write one day's two rollup projections in a single transaction.
///
/// Atomic on purpose: a committed daily row with its hourly rows missing
/// is exactly the half-written state
/// [`rollup_catch_up_cursor`] would have to repair, and the cursor cannot
/// tell that case apart from a day that legitimately had no pairs once the
/// archive rows behind it age out. Failing the day as a whole leaves the
/// cursor unmoved, so the next pass simply retries it.
pub(crate) async fn persist_day_rollup_from_pool(
  pool: &sqlx::SqlitePool,
  summary: Option<&DailyCoolingSummary>,
  hours: &[crate::persistence::cooling_hourly_rollup::HourlyCoolingSummary],
) -> Result<(), sqlx::Error> {
  use crate::infrastructure::database;

  let mut tx = pool.begin().await?;

  if let Some(summary) = summary {
    database::cooling_daily_summary::upsert_with(&mut *tx, summary).await?;
  }
  for hour in hours {
    database::cooling_hourly_summary::upsert_with(&mut *tx, hour).await?;
  }

  tx.commit().await
}

/// Delete `cooling_daily_summary` and `cooling_hourly_summary` rows older
/// than [`COOLING_DAILY_SUMMARY_RETENTION_DAYS`]. Called from
/// `crate::persistence::archive::cleanup_old_data` at the same
/// `scheduledDataDeletion`-gated startup site as the Hardware Archive
/// cleanup, but with its own fixed retention window rather than the
/// user-configurable `hardwareArchive.retentionDays`.
///
/// Both rollups share one retention constant on purpose: they are two
/// projections of the same archived minutes, so a window Cooling Insight
/// can show daily but not hourly (or the reverse) would only be a source
/// of inconsistent answers.
///
/// The pinned baseline's own window is exempt from both deletes. The
/// baseline is a fixed reference that never expires (that is the point of
/// pinning it), so once its window drifts past the retention cutoff,
/// deleting the rows inside it would permanently empty every
/// baseline-side comparison (the load-band comparison, and the Explorer's
/// baseline medians) while the baseline still names that period as the
/// reference. The exemption costs at most a week of rows.
pub async fn cleanup_old_data() {
  use crate::infrastructure::database;

  let preserved_window =
    match database::cooling_baseline::select_established_baseline().await {
      Ok(baseline) => baseline.map(|b| (b.window_start_date, b.window_end_date)),
      Err(e) => {
        // Deleting without knowing the protected window could erase the
        // baseline's evidence irrecoverably, so skip this cleanup pass and
        // let the next boot retry rather than risk it.
        log_error!(
          "Failed to read the pinned cooling baseline; skipping cooling rollup cleanup",
          "persistence::cooling_rollup::cleanup_old_data",
          Some(e.to_string())
        );
        return;
      }
    };

  if let Err(e) = database::cooling_daily_summary::delete_old_data(
    COOLING_DAILY_SUMMARY_RETENTION_DAYS,
    preserved_window,
  )
  .await
  {
    log_error!(
      "Failed to delete old cooling daily summary data",
      "persistence::cooling_rollup::cleanup_old_data",
      Some(e.to_string())
    );
  }

  if let Err(e) = database::cooling_hourly_summary::delete_old_data(
    COOLING_DAILY_SUMMARY_RETENTION_DAYS,
    preserved_window,
  )
  .await
  {
    log_error!(
      "Failed to delete old cooling hourly summary data",
      "persistence::cooling_rollup::cleanup_old_data",
      Some(e.to_string())
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // ── CpuLoadBand::classify ──

  #[test]
  fn classify_idle_band() {
    assert_eq!(CpuLoadBand::classify(0.0), CpuLoadBand::Idle);
    assert_eq!(CpuLoadBand::classify(5.0), CpuLoadBand::Idle);
    assert_eq!(CpuLoadBand::classify(9.999), CpuLoadBand::Idle);
  }

  #[test]
  fn classify_low_band_boundary_is_inclusive() {
    assert_eq!(CpuLoadBand::classify(10.0), CpuLoadBand::Low);
    assert_eq!(CpuLoadBand::classify(20.0), CpuLoadBand::Low);
    assert_eq!(CpuLoadBand::classify(29.999), CpuLoadBand::Low);
  }

  #[test]
  fn classify_mid_band_boundary_is_inclusive() {
    assert_eq!(CpuLoadBand::classify(30.0), CpuLoadBand::Mid);
    assert_eq!(CpuLoadBand::classify(45.0), CpuLoadBand::Mid);
    assert_eq!(CpuLoadBand::classify(59.999), CpuLoadBand::Mid);
  }

  #[test]
  fn classify_high_band_boundary_is_inclusive_and_open_ended() {
    assert_eq!(CpuLoadBand::classify(60.0), CpuLoadBand::High);
    assert_eq!(CpuLoadBand::classify(100.0), CpuLoadBand::High);
    // Measurement noise above 100% must still land in a band.
    assert_eq!(CpuLoadBand::classify(150.0), CpuLoadBand::High);
  }

  #[test]
  fn classify_negative_usage_falls_back_to_idle() {
    // Never expected in practice, but classify() must be total: no value
    // should be left unclassified.
    assert_eq!(CpuLoadBand::classify(-1.0), CpuLoadBand::Idle);
  }

  // ── summarize_day ──

  fn sample(
    cpu_usage_avg: Option<f32>,
    temp_avg: Option<f32>,
    temp_max: Option<f32>,
    temp_min: Option<f32>,
  ) -> ArchiveMinuteSample {
    ArchiveMinuteSample {
      // `summarize_day` folds a range its caller already narrowed to one
      // local day, so the instant is irrelevant here (only the hourly
      // rollup reads it).
      timestamp: utc("2026-08-20T12:00:00Z"),
      cpu_usage_avg,
      cpu_temperature_avg: temp_avg,
      cpu_temperature_max: temp_max,
      cpu_temperature_min: temp_min,
    }
  }

  fn full_sample(cpu_usage_avg: f32, temp: f32) -> ArchiveMinuteSample {
    sample(
      Some(cpu_usage_avg),
      Some(temp),
      Some(temp + 1.0),
      Some(temp - 1.0),
    )
  }

  fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
  }

  #[test]
  fn summarize_day_returns_none_for_a_day_without_samples() {
    assert_eq!(summarize_day(date(2026, 8, 20), &[]), None);
  }

  #[test]
  fn summarize_day_places_single_minute_in_its_band() {
    let summary = summarize_day(date(2026, 8, 20), &[full_sample(5.0, 40.0)]).unwrap();

    assert_eq!(summary.date, date(2026, 8, 20));
    assert_eq!(summary.coverage_minutes, 1);
    assert_eq!(summary.idle.sample_minutes, 1);
    assert_eq!(summary.idle.avg, Some(40.0));
    assert_eq!(summary.idle.max, Some(41.0));
    assert_eq!(summary.idle.min, Some(39.0));
    // Untouched bands stay fully absent, not zero.
    for band in [summary.low, summary.mid, summary.high] {
      assert_eq!(band.sample_minutes, 0);
      assert_eq!(band.avg, None);
      assert_eq!(band.max, None);
      assert_eq!(band.min, None);
    }
  }

  #[test]
  fn summarize_day_excludes_minutes_without_a_temperature_reading() {
    let minutes = [
      full_sample(5.0, 40.0),
      // CPU usage was recorded (idle-band), but no temperature sensor
      // reading that minute - this must contribute nothing.
      sample(Some(5.0), None, None, None),
    ];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    // Coverage counts every archived minute regardless of temperature
    // availability.
    assert_eq!(summary.coverage_minutes, 2);
    // But the band's own sample count/aggregate ignores the missing one.
    assert_eq!(summary.idle.sample_minutes, 1);
    assert_eq!(summary.idle.avg, Some(40.0));
  }

  #[test]
  fn summarize_day_excludes_minutes_without_a_cpu_usage_reading() {
    let minutes = [
      full_sample(5.0, 40.0),
      // No CPU usage reading at all - cannot be classified into any band.
      sample(None, Some(99.0), Some(99.0), Some(99.0)),
    ];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.coverage_minutes, 2);
    assert_eq!(summary.idle.sample_minutes, 1);
    let total_sample_minutes: u32 =
      [summary.idle, summary.low, summary.mid, summary.high]
        .iter()
        .map(|b| b.sample_minutes)
        .sum();
    assert_eq!(
      total_sample_minutes, 1,
      "the unclassifiable 99.0 reading must not silently land in a band"
    );
  }

  #[test]
  fn summarize_day_aggregates_avg_of_avgs_and_extremes_across_the_band() {
    let minutes = [
      full_sample(65.0, 60.0),
      full_sample(70.0, 80.0),
      full_sample(75.0, 70.0),
    ];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.high.sample_minutes, 3);
    assert_eq!(summary.high.avg, Some(70.0));
    // full_sample uses temp + 1.0 / temp - 1.0 for max/min.
    assert_eq!(summary.high.max, Some(81.0));
    assert_eq!(summary.high.min, Some(59.0));
  }

  #[test]
  fn summarize_day_reports_partial_day_coverage() {
    let minutes: Vec<_> = (0..500).map(|_| full_sample(2.0, 35.0)).collect();
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.coverage_minutes, 500);
    assert_eq!(summary.idle.sample_minutes, 500);
  }

  #[test]
  fn summarize_day_caps_coverage_at_a_full_day() {
    // Shutdown flushes can archive a partial window as an extra row, so
    // a day can hold more rows than minutes; coverage must not exceed a
    // full day.
    let minutes: Vec<_> = (0..1445).map(|_| full_sample(2.0, 35.0)).collect();
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.coverage_minutes, 1440);
  }

  // ── days_to_roll_up ──

  #[test]
  fn days_to_roll_up_is_empty_when_nothing_is_archived_yet() {
    assert_eq!(days_to_roll_up(None, None, date(2026, 8, 20)), Vec::new());
  }

  #[test]
  fn days_to_roll_up_backfills_from_earliest_archived_day_on_first_run() {
    let days = days_to_roll_up(None, Some(date(2026, 8, 1)), date(2026, 8, 3));
    assert_eq!(
      days,
      vec![date(2026, 8, 1), date(2026, 8, 2), date(2026, 8, 3)]
    );
  }

  #[test]
  fn days_to_roll_up_is_empty_when_already_caught_up_through_yesterday() {
    let days = days_to_roll_up(Some(date(2026, 8, 20)), None, date(2026, 8, 20));
    assert_eq!(days, Vec::new());
  }

  #[test]
  fn days_to_roll_up_covers_days_missed_while_the_app_was_not_running() {
    let days = days_to_roll_up(Some(date(2026, 8, 18)), None, date(2026, 8, 20));
    assert_eq!(days, vec![date(2026, 8, 19), date(2026, 8, 20)]);
  }

  #[test]
  fn days_to_roll_up_ignores_earliest_archived_once_a_rollup_exists() {
    let days = days_to_roll_up(
      Some(date(2026, 8, 19)),
      Some(date(2026, 1, 1)),
      date(2026, 8, 20),
    );
    assert_eq!(days, vec![date(2026, 8, 20)]);
  }

  #[test]
  fn days_to_roll_up_is_empty_when_last_summarized_is_not_before_yesterday() {
    let days = days_to_roll_up(Some(date(2026, 8, 25)), None, date(2026, 8, 20));
    assert_eq!(days, Vec::new());
  }

  // ── rollup_catch_up_cursor ──

  #[test]
  fn the_cursor_follows_the_daily_rollup_when_hourly_has_kept_up() {
    assert_eq!(
      rollup_catch_up_cursor(
        Some(date(2026, 8, 20)),
        Some(date(2026, 8, 20)),
        Some(date(2026, 8, 20)),
      ),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn an_empty_hourly_table_rewinds_the_cursor_for_a_full_backfill() {
    // The v13 upgrade path: a long-running install has a full daily table
    // and no hourly rows at all. Following the daily cursor would leave
    // every past day's hourly rows missing forever.
    assert_eq!(
      rollup_catch_up_cursor(Some(date(2026, 8, 20)), Some(date(2026, 8, 20)), None),
      None,
      "an empty hourly table must fall back to the earliest-archived-day backfill"
    );
  }

  #[test]
  fn a_lagging_hourly_table_resumes_from_the_slower_of_the_two_cursors() {
    assert_eq!(
      rollup_catch_up_cursor(
        Some(date(2026, 8, 20)),
        Some(date(2026, 8, 20)),
        Some(date(2026, 8, 15)),
      ),
      Some(date(2026, 8, 15)),
      "the days between the two cursors must be regenerated"
    );
  }

  #[test]
  fn a_machine_that_never_recorded_a_pair_does_not_rewind_every_cycle() {
    // No CPU temperature sensor: daily rows accrue coverage but no band
    // samples, so the hourly table is legitimately empty. Treating that
    // as "behind" would re-read the whole archive on every catch-up,
    // forever.
    assert_eq!(
      rollup_catch_up_cursor(Some(date(2026, 8, 20)), None, None),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn hourly_ahead_of_the_last_pairable_day_is_still_caught_up() {
    // Recent days recorded coverage but no pairs, so the latest pairable
    // day is older than the daily cursor. Hourly has nothing to add.
    assert_eq!(
      rollup_catch_up_cursor(
        Some(date(2026, 8, 20)),
        Some(date(2026, 8, 10)),
        Some(date(2026, 8, 10)),
      ),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn a_first_ever_run_still_backfills_from_the_earliest_archived_day() {
    assert_eq!(rollup_catch_up_cursor(None, None, None), None);
  }

  // ── persist_day_rollup_from_pool ──

  mod persist_day_rollup {
    use super::*;
    use crate::persistence::cooling_hourly_rollup::HourlyCoolingSummary;
    use sqlx::SqlitePool;

    async fn create_daily_table(pool: &SqlitePool) {
      sqlx::query(
        "CREATE TABLE cooling_daily_summary (
          date TEXT PRIMARY KEY,
          idle_cpu_temperature_avg REAL,
          idle_cpu_temperature_max REAL,
          idle_cpu_temperature_min REAL,
          idle_sample_minutes INTEGER NOT NULL DEFAULT 0,
          low_cpu_temperature_avg REAL,
          low_cpu_temperature_max REAL,
          low_cpu_temperature_min REAL,
          low_sample_minutes INTEGER NOT NULL DEFAULT 0,
          mid_cpu_temperature_avg REAL,
          mid_cpu_temperature_max REAL,
          mid_cpu_temperature_min REAL,
          mid_sample_minutes INTEGER NOT NULL DEFAULT 0,
          high_cpu_temperature_avg REAL,
          high_cpu_temperature_max REAL,
          high_cpu_temperature_min REAL,
          high_sample_minutes INTEGER NOT NULL DEFAULT 0,
          coverage_minutes INTEGER NOT NULL
        )",
      )
      .execute(pool)
      .await
      .unwrap();
    }

    async fn create_hourly_table(pool: &SqlitePool) {
      sqlx::query(
        "CREATE TABLE cooling_hourly_summary (
          hour_start TEXT PRIMARY KEY,
          cpu_usage_avg REAL,
          cpu_temperature_avg REAL,
          sample_minutes INTEGER NOT NULL
        )",
      )
      .execute(pool)
      .await
      .unwrap();
    }

    fn summary() -> DailyCoolingSummary {
      summarize_day(date(2026, 8, 20), &[full_sample(5.0, 40.0)]).unwrap()
    }

    fn hour() -> HourlyCoolingSummary {
      HourlyCoolingSummary {
        hour_start: date(2026, 8, 20).and_hms_opt(12, 0, 0).unwrap(),
        cpu_usage_avg: Some(5.0),
        cpu_temperature_avg: Some(40.0),
        sample_minutes: 60,
      }
    }

    async fn daily_row_count(pool: &SqlitePool) -> i64 {
      sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM cooling_daily_summary")
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_successful_day_commits_both_projections() {
      let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
      create_daily_table(&pool).await;
      create_hourly_table(&pool).await;

      persist_day_rollup_from_pool(&pool, Some(&summary()), &[hour()])
        .await
        .unwrap();

      assert_eq!(daily_row_count(&pool).await, 1);
      assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM cooling_hourly_summary")
          .fetch_one(&pool)
          .await
          .unwrap(),
        1
      );
    }

    #[tokio::test]
    async fn a_failed_hourly_write_rolls_the_daily_row_back() {
      // The two projections must land together: a committed daily row
      // with its hourly rows missing is a half-written day the catch-up
      // cursor cannot reliably distinguish from a day that had no pairs.
      let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
      create_daily_table(&pool).await;
      // `cooling_hourly_summary` deliberately absent, so the hourly
      // upsert fails after the daily one has already been issued.

      let result = persist_day_rollup_from_pool(&pool, Some(&summary()), &[hour()]).await;

      assert!(result.is_err(), "the day must fail as a whole");
      assert_eq!(
        daily_row_count(&pool).await,
        0,
        "the daily row must not survive a failed hourly write"
      );
    }
  }

  // ── day_utc_bounds_for_offset / utc_to_date_for_offset ──
  //
  // Use a fixed offset rather than `chrono::Local` so these are
  // deterministic regardless of the machine/CI runner's configured
  // timezone.

  fn jst() -> chrono::FixedOffset {
    chrono::FixedOffset::east_opt(9 * 3600).unwrap()
  }

  fn utc(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input)
      .unwrap()
      .with_timezone(&Utc)
  }

  #[test]
  fn day_bounds_for_utc_offset_match_calendar_midnight() {
    let (start, end) = day_utc_bounds_for_offset(date(2026, 8, 15), &Utc);
    assert_eq!(start, utc("2026-08-15T00:00:00Z"));
    assert_eq!(end, utc("2026-08-16T00:00:00Z"));
  }

  #[test]
  fn day_bounds_shift_by_a_positive_offset() {
    let (start, end) = day_utc_bounds_for_offset(date(2026, 8, 15), &jst());
    // JST local midnight is the previous UTC day at 15:00.
    assert_eq!(start, utc("2026-08-14T15:00:00Z"));
    assert_eq!(end, utc("2026-08-15T15:00:00Z"));
  }

  #[test]
  fn day_bounds_shift_by_a_negative_offset() {
    let pst = chrono::FixedOffset::west_opt(8 * 3600).unwrap();
    let (start, end) = day_utc_bounds_for_offset(date(2026, 8, 15), &pst);
    assert_eq!(start, utc("2026-08-15T08:00:00Z"));
    assert_eq!(end, utc("2026-08-16T08:00:00Z"));
  }

  #[test]
  fn utc_to_date_for_offset_maps_instant_to_local_calendar_day() {
    assert_eq!(
      utc_to_date_for_offset(utc("2026-08-14T15:00:00Z"), &jst()),
      date(2026, 8, 15),
      "exactly JST midnight must fall on the new day"
    );
    assert_eq!(
      utc_to_date_for_offset(utc("2026-08-14T14:59:59Z"), &jst()),
      date(2026, 8, 14),
      "one second before JST midnight must stay on the previous day"
    );
  }
}
