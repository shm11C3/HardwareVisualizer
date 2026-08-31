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
//!
//! The same pass also folds the day's CPU package power ([`PowerSummary`],
//! #2021), which the Cooling Insight timeline's power lane reads for the
//! 90d/1y windows. Power is a separate hardware capability from CPU
//! temperature, so it is folded outside the band gate: neither reading's
//! absence suppresses the other, and a missing reading stays absent rather
//! than becoming 0 W.
//!
//! Finally the same pass folds the ambient-normalized thermal delta
//! ([`AmbientDeltaSummary`], #2045): `ΔT = CPU package temperature −
//! ambient temperature`, per band. The pairing that makes ΔT meaningful
//! happens *before* this fold, at the read boundary - each
//! [`ArchiveMinuteSample`] already carries the ambient temperature of its
//! own minute (see
//! [`crate::infrastructure::database::cooling_daily_summary`]) - so the
//! fold can only ever subtract two readings describing the same minute.
//! That is #2045's normative rule expressed in the type rather than by
//! discipline: independently aggregated CPU and ambient summaries must
//! never be subtracted, because the two archives do not share a sample
//! set - ambient readings go missing independently of hardware minutes,
//! and subtracting summaries built over different sample sets produces a
//! number that corresponds to no real pairing.

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
///
/// `cpu_power_*` is the CPU package-domain power draw the Hardware Archive
/// stores in its `cpu_power_*` columns - Apple Silicon's CPU domain and,
/// since #2035, the Windows RAPL package domain. Both publish through
/// `PowerDraw::cpu_watts`, so one archive column backs both.
///
/// `ambient_temperature_avg` is this minute's ambient air temperature
/// (#2045), already paired with the hardware reading by the read boundary.
/// `None` means the minute has no valid ambient pairing, which yields no
/// ΔT at all rather than an interpolated one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArchiveMinuteSample {
  pub timestamp: DateTime<Utc>,
  pub cpu_usage_avg: Option<f32>,
  pub cpu_temperature_avg: Option<f32>,
  pub cpu_temperature_max: Option<f32>,
  pub cpu_temperature_min: Option<f32>,
  pub cpu_power_avg: Option<f32>,
  pub cpu_power_max: Option<f32>,
  pub cpu_power_min: Option<f32>,
  pub ambient_temperature_avg: Option<f32>,
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

/// One local day's CPU package power draw, in watts (#2021).
///
/// Deliberately *not* a [`BandSummary`]: power is folded over the whole day
/// rather than per CPU-load band, because the timeline's power lane reads
/// one series per period and the load split is already carried by the
/// temperature bands.
///
/// `sample_minutes == 0` implies `avg`/`max`/`min` are all `None`. A
/// machine whose platform provider publishes no CPU power reports absent
/// power for every day, never 0 W (DP-02).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PowerSummary {
  pub avg: Option<f32>,
  pub max: Option<f32>,
  pub min: Option<f32>,
  pub sample_minutes: u32,
}

/// One local day's ambient-normalized thermal delta, per CPU-load band
/// (#2045).
///
/// Each band's [`BandSummary`] holds `ΔT = CPU package temperature −
/// ambient temperature` in kelvin-equivalent degrees, folded only over
/// minutes that carried *both* readings. `sample_minutes` there is
/// therefore a strict subset of the matching temperature band's: a minute
/// contributes to a ΔT band only when it already contributed to that
/// temperature band *and* had an ambient pair, so the two gates are
/// nested rather than independent.
///
/// `coverage_minutes` is counted outside that nesting: it is every
/// archived minute of the day that had an ambient pair at all, whether or
/// not the CPU side could be classified into a band. That makes it an
/// honest measure of ambient availability on a machine with no CPU
/// temperature sensor, which is exactly what the backfill cursor needs
/// (see [`RollupProgress::last_ambient_daily_date`]).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AmbientDeltaSummary {
  pub coverage_minutes: u32,
  pub idle: BandSummary,
  pub low: BandSummary,
  pub mid: BandSummary,
  pub high: BandSummary,
}

impl AmbientDeltaSummary {
  /// This day's ΔT summary for `band`.
  pub fn band(&self, band: CpuLoadBand) -> &BandSummary {
    match band {
      CpuLoadBand::Idle => &self.idle,
      CpuLoadBand::Low => &self.low,
      CpuLoadBand::Mid => &self.mid,
      CpuLoadBand::High => &self.high,
    }
  }
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
  /// The day's CPU package power draw, folded independently of the bands
  /// above (#2021). A machine with a temperature sensor and no power
  /// sampler keeps full band summaries with absent power, and one with a
  /// power sampler and no temperature sensor keeps power with empty
  /// bands: neither capability gates the other.
  pub power: PowerSummary,
  /// The day's ambient-normalized thermal delta per band (#2045). Fully
  /// default on a machine with no ambient sensor, which is what keeps
  /// every ambient-unaware query answering exactly as it did before.
  pub ambient: AmbientDeltaSummary,
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

  let mut idle = ReadingAccumulator::default();
  let mut low = ReadingAccumulator::default();
  let mut mid = ReadingAccumulator::default();
  let mut high = ReadingAccumulator::default();
  let mut power = ReadingAccumulator::default();
  let mut delta_idle = ReadingAccumulator::default();
  let mut delta_low = ReadingAccumulator::default();
  let mut delta_mid = ReadingAccumulator::default();
  let mut delta_high = ReadingAccumulator::default();
  let mut ambient_coverage_minutes: u32 = 0;

  for minute in minutes {
    // Power is folded before the band gate below and never `continue`s
    // past it: a minute can carry power without a usable CPU usage or
    // temperature reading, and dropping it would make the power lane
    // depend on sensors it has nothing to do with.
    //
    // All three of avg/max/min are required, matching the temperature
    // gate. `hardware_archive::insert` writes the triple together, so a
    // partial triple means a hand-edited row rather than real data.
    if let (Some(power_avg), Some(power_max), Some(power_min)) = (
      minute.cpu_power_avg,
      minute.cpu_power_max,
      minute.cpu_power_min,
    ) {
      power.push(power_avg, power_max, power_min);
    }

    // Ambient coverage is counted here, outside the band gate below, for
    // the same reason power is: whether the room's air temperature was
    // readable that minute has nothing to do with whether the CPU's own
    // sensors were. A machine with an ambient sensor and no CPU
    // temperature sensor still reports honest ambient coverage - and the
    // backfill cursor depends on that, or it would rewind forever
    // chasing coverage such a machine can never record.
    if minute.ambient_temperature_avg.is_some() {
      ambient_coverage_minutes += 1;
    }

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

    let classified = CpuLoadBand::classify(cpu_usage_avg);
    let band = match classified {
      CpuLoadBand::Idle => &mut idle,
      CpuLoadBand::Low => &mut low,
      CpuLoadBand::Mid => &mut mid,
      CpuLoadBand::High => &mut high,
    };
    band.push(temperature_avg, temperature_max, temperature_min);

    // ΔT is gated *inside* the band gate, not beside it: a minute
    // contributes to a ΔT band only when it already contributed to that
    // temperature band and carried an ambient pair. A minute with an
    // ambient reading but no usable CPU temperature or usage has nothing
    // to subtract from and nowhere to put the result, so it contributes
    // no ΔT - only ambient coverage, counted above.
    //
    // The minute's own ambient value shifts all three of avg/max/min
    // together: within one archived minute there is a single ambient
    // reading, so the ΔT extremes are the CPU extremes offset by it.
    // Nothing here is interpolated across minutes (DP-02).
    let Some(ambient_temperature) = minute.ambient_temperature_avg else {
      continue;
    };
    let delta_band = match classified {
      CpuLoadBand::Idle => &mut delta_idle,
      CpuLoadBand::Low => &mut delta_low,
      CpuLoadBand::Mid => &mut delta_mid,
      CpuLoadBand::High => &mut delta_high,
    };
    delta_band.push(
      temperature_avg - ambient_temperature,
      temperature_max - ambient_temperature,
      temperature_min - ambient_temperature,
    );
  }

  Some(DailyCoolingSummary {
    date,
    coverage_minutes: (minutes.len() as u32).min(MINUTES_PER_DAY),
    idle: idle.finish_band(),
    low: low.finish_band(),
    mid: mid.finish_band(),
    high: high.finish_band(),
    power: power.finish_power(),
    ambient: AmbientDeltaSummary {
      coverage_minutes: ambient_coverage_minutes.min(MINUTES_PER_DAY),
      idle: delta_idle.finish_band(),
      low: delta_low.finish_band(),
      mid: delta_mid.finish_band(),
      high: delta_high.finish_band(),
    },
  })
}

/// Accumulates one series of per-minute avg/max/min readings for a single
/// day - one [`CpuLoadBand`]'s temperatures, or the day's CPU package
/// power. `avg` is the average of the per-minute averages (consistent with
/// how `archive_queries` already aggregates `DATA_ARCHIVE` rows, since
/// each row is itself already a one-minute average); `max`/`min` are the
/// extremes across the per-minute extremes.
#[derive(Default)]
struct ReadingAccumulator {
  sum: f64,
  count: u32,
  max: Option<f32>,
  min: Option<f32>,
}

impl ReadingAccumulator {
  fn push(&mut self, avg: f32, max: f32, min: f32) {
    self.sum += avg as f64;
    self.count += 1;
    self.max = Some(self.max.map_or(max, |current| current.max(max)));
    self.min = Some(self.min.map_or(min, |current| current.min(min)));
  }

  fn avg(&self) -> Option<f32> {
    (self.count > 0).then(|| (self.sum / self.count as f64) as f32)
  }

  fn finish_band(self) -> BandSummary {
    BandSummary {
      avg: self.avg(),
      max: self.max,
      min: self.min,
      sample_minutes: self.count,
    }
  }

  fn finish_power(self) -> PowerSummary {
    PowerSummary {
      avg: self.avg(),
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

/// How far each rollup projection has actually got, plus what the
/// one-minute archive still holds - the facts
/// [`rollup_catch_up_cursor`] decides from.
///
/// A struct rather than positional arguments: every field is an
/// `Option<NaiveDate>` with a different meaning, and a transposed pair
/// would compile silently while quietly disabling a backfill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RollupProgress {
  /// `MAX(date)` in `cooling_daily_summary`.
  pub last_daily_date: Option<NaiveDate>,
  /// The latest summarized day that recorded a (load, temperature) pair.
  pub last_pairable_daily_date: Option<NaiveDate>,
  /// The local day of `MAX(hour_start)` in `cooling_hourly_summary`.
  pub last_hourly_date: Option<NaiveDate>,
  /// The latest summarized day that recorded any CPU package power.
  pub last_powered_daily_date: Option<NaiveDate>,
  /// The latest *completed* local day whose archive rows carry a full CPU
  /// package power triple, or `None` when the archive holds none.
  /// Completed days only - see [`power_rollup_is_behind`].
  pub last_powered_archive_date: Option<NaiveDate>,
  /// The local day of `MAX(date)` in `cooling_fan_daily_summary`.
  pub last_fanned_daily_date: Option<NaiveDate>,
  /// The latest *completed* local day the fan archive holds a reading for,
  /// or `None` when it holds none. Completed days only - see
  /// [`fan_rollup_is_behind`].
  pub last_fanned_archive_date: Option<NaiveDate>,
  /// The latest summarized day that recorded any ambient coverage (#2045).
  pub last_ambient_daily_date: Option<NaiveDate>,
  /// The latest *completed* local day whose ambient archive rows pair with
  /// a hardware archive minute, or `None` when none do. Completed days
  /// only, and pairable only - see [`ambient_rollup_is_behind`].
  pub last_ambient_archive_date: Option<NaiveDate>,
}

/// Where the catch-up must resume from, given how far each rollup
/// projection has actually got.
///
/// The daily cursor alone is not enough. Each later migration added a
/// projection to a table an existing install has already summarized
/// through yesterday, so following the daily cursor would leave the new
/// projection empty for the whole retention window. Resuming from the
/// *slowest* projection regenerates it, and re-running a day is harmless
/// because every upsert is idempotent.
///
/// "Slowest", though, cannot just be `min` over the projections: a machine
/// missing the sensor a projection needs legitimately has nothing there,
/// and a plain `min` would make it re-read its whole archive on every
/// cycle forever. Each projection therefore claims to be behind only on
/// evidence that there was something for it to miss - see
/// [`hourly_rollup_resume`] and [`power_rollup_is_behind`].
///
/// A `None` result means "resume from the earliest archived day", the same
/// first-run backfill [`days_to_roll_up`] already performs. Only days the
/// one-minute archive still holds can be regenerated; older days no longer
/// have source rows, and reporting them as absent is the honest outcome.
pub fn rollup_catch_up_cursor(progress: RollupProgress) -> Option<NaiveDate> {
  // Folded over a list rather than nested `earlier_resume` calls: there
  // are four projections now and adding the fifth should not mean
  // re-reading a pile of parentheses to check the nesting is right.
  [
    hourly_rollup_resume(progress),
    power_rollup_resume(progress),
    fan_rollup_resume(progress),
    ambient_rollup_resume(progress),
  ]
  .into_iter()
  .reduce(earlier_resume)
  .expect("the projection list is a non-empty literal")
}

/// The earlier of two resume points. `None` means "from the earliest
/// archived day", so it is earlier than any date rather than absent.
fn earlier_resume(a: Option<NaiveDate>, b: Option<NaiveDate>) -> Option<NaiveDate> {
  match (a, b) {
    (Some(a), Some(b)) => Some(a.min(b)),
    _ => None,
  }
}

/// Where the hourly `(load, temperature)` projection needs the catch-up to
/// resume from, or `last_daily_date` when it has kept up (#2023).
///
/// `last_pairable_daily_date` is the latest day that actually had a pair
/// to record, which is exactly the latest day hourly is *expected* to
/// cover - so "hourly is behind" is only claimed when there was something
/// for it to miss. A machine with no CPU temperature sensor produces daily
/// rows and no hourly rows at all, and must not rewind forever.
fn hourly_rollup_resume(progress: RollupProgress) -> Option<NaiveDate> {
  match progress.last_pairable_daily_date {
    // No summarized day ever carried a pair, so the hourly rollup has
    // nothing it could be missing.
    None => progress.last_daily_date,
    Some(pairable) => match progress.last_hourly_date {
      // Hourly has reached every day that had something to record.
      Some(hourly) if hourly >= pairable => progress.last_daily_date,
      // Hourly is genuinely behind: resume from where it actually got to.
      behind => behind,
    },
  }
}

/// Where the daily rollup's CPU package power columns need the catch-up to
/// resume from, or `last_daily_date` when they have kept up (#2021).
fn power_rollup_resume(progress: RollupProgress) -> Option<NaiveDate> {
  if power_rollup_is_behind(progress) {
    progress.last_powered_daily_date
  } else {
    progress.last_daily_date
  }
}

/// Whether the archive holds a completed day's CPU package power that no
/// summarized day recorded.
///
/// Migration 14 added the power columns to a table an existing install has
/// already summarized through yesterday, so every one of those rows
/// carries NULL power. Without this check the daily cursor would never
/// revisit them and the timeline's power lane would stay blank for the
/// whole retention window, even though the one-minute archive still holds
/// the readings.
///
/// Being behind is claimed only when the *archive* actually holds that
/// power - never merely because the daily table has none. A machine with
/// no CPU power source has neither side, so it never rewinds: the same
/// trap `last_pairable_daily_date` avoids for the hourly rollup.
///
/// `last_powered_archive_date` must cover completed days only. The rollup
/// never summarizes today, so today's archived power is not evidence that
/// anything was missed - counting it would make a machine that is
/// recording power right now rewind on every single cycle, forever. That
/// bound is applied in SQL, where the day boundary already lives (see
/// `cooling_daily_summary::max_powered_archive_timestamp_before`), so no
/// clamp here can disagree with it.
fn power_rollup_is_behind(progress: RollupProgress) -> bool {
  let Some(archived) = progress.last_powered_archive_date else {
    // The archive holds no CPU power at all, so there is nothing the
    // rollup could have missed.
    return false;
  };

  match progress.last_powered_daily_date {
    // The archive has power and no summarized day carries any: exactly
    // the post-migration state.
    None => true,
    Some(recorded) => recorded < archived,
  }
}

/// Where the fan daily rollup needs the catch-up to resume from, or
/// `last_daily_date` when it has kept up (#2022).
fn fan_rollup_resume(progress: RollupProgress) -> Option<NaiveDate> {
  if fan_rollup_is_behind(progress) {
    progress.last_fanned_daily_date
  } else {
    progress.last_daily_date
  }
}

/// Where the daily rollup's ambient delta columns need the catch-up to
/// resume from, or `last_daily_date` when they have kept up (#2045).
fn ambient_rollup_resume(progress: RollupProgress) -> Option<NaiveDate> {
  if ambient_rollup_is_behind(progress) {
    progress.last_ambient_daily_date
  } else {
    progress.last_daily_date
  }
}

/// Whether the fan archive holds a completed day that no
/// `cooling_fan_daily_summary` row covers.
///
/// The same shape as [`power_rollup_is_behind`], and for the same reason:
/// the migration that adds `cooling_fan_daily_summary` lands on installs
/// that have already summarized through yesterday, so following the daily
/// cursor alone would leave the fan lane blank for the whole retention
/// window even though `FAN_ARCHIVE` still holds the readings.
///
/// Being behind is claimed only when the *archive* actually holds fan
/// readings. A machine with no fan source has neither side, so it never
/// rewinds - without that guard it would re-read its entire archive on
/// every cycle, forever.
fn fan_rollup_is_behind(progress: RollupProgress) -> bool {
  let Some(archived) = progress.last_fanned_archive_date else {
    return false;
  };

  match progress.last_fanned_daily_date {
    None => true,
    Some(recorded) => recorded < archived,
  }
}

/// Whether the ambient archive holds a completed day's *pairable* ambient
/// reading that no summarized day recorded coverage for (#2045).
///
/// Migration 16 added the delta columns to a table an existing install has
/// already summarized through yesterday, so an install that was already
/// collecting ambient readings before this shipped carries NULL deltas for
/// every one of those days. Without this check the daily cursor would
/// never revisit them and the ambient-adjusted readings would stay absent
/// for the whole retention window while the archives still hold both
/// sides.
///
/// Two bounds keep this from becoming a permanent rewind, and they are the
/// same two the power backfill needed:
///
/// - Being behind is claimed only when the *archive* actually holds a
///   pairable ambient reading, never merely because the daily table has no
///   coverage. A machine with no ambient sensor has neither side, so it
///   never rewinds.
/// - `last_ambient_archive_date` counts only ambient rows that join a
///   hardware archive minute, matching `summarize_day`'s own coverage
///   gate. An ambient row whose minute has no `DATA_ARCHIVE` row can never
///   become coverage no matter how often the day is re-rolled, so counting
///   it would make the catch-up chase a day it can never fill.
///
/// The completed-days bound is applied in SQL, where the day boundary
/// already lives (see
/// `cooling_daily_summary::max_pairable_ambient_archive_timestamp_before`),
/// so no clamp here can disagree with it.
fn ambient_rollup_is_behind(progress: RollupProgress) -> bool {
  let Some(archived) = progress.last_ambient_archive_date else {
    return false;
  };

  match progress.last_ambient_daily_date {
    None => true,
    Some(recorded) => recorded < archived,
  }
}

async fn catch_up_cooling_rollup() -> Result<(), sqlx::Error> {
  use crate::infrastructure::database;

  let yesterday = chrono::Local::now().date_naive() - Duration::days(1);
  // The end of yesterday is the start of today, which is exactly the
  // "completed days only" bound the power backfill check needs.
  let (_, today_start) = local_day_utc_bounds(yesterday);
  let last_summarized_date = rollup_catch_up_cursor(RollupProgress {
    last_daily_date: database::cooling_daily_summary::max_summarized_date().await?,
    last_pairable_daily_date:
      database::cooling_daily_summary::max_pairable_summarized_date().await?,
    last_hourly_date: database::cooling_hourly_summary::max_summarized_date().await?,
    last_powered_daily_date:
      database::cooling_daily_summary::max_powered_summarized_date().await?,
    last_powered_archive_date:
      database::cooling_daily_summary::max_powered_archive_timestamp_before(&today_start)
        .await?
        .map(utc_to_local_date),
    last_fanned_daily_date: database::cooling_fan_daily_summary::max_summarized_date()
      .await?,
    last_fanned_archive_date: database::fan_archive::max_fan_archive_timestamp_before(
      &today_start,
    )
    .await?
    .map(utc_to_local_date),
    last_ambient_daily_date:
      database::cooling_daily_summary::max_ambient_summarized_date().await?,
    last_ambient_archive_date:
      database::cooling_daily_summary::max_pairable_ambient_archive_timestamp_before(
        &today_start,
      )
      .await?
      .map(utc_to_local_date),
  });
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

  // Resolve (and, once established, pin) both baselines right after the
  // rollup advances, so establishment happens in the background rather
  // than only when Cooling Insight is read — otherwise a user who never
  // opens the view before retention cleanup erases the
  // establishment-window rows would get a drifted baseline pinned on
  // their first read. The ΔT baseline (#2045) needs this at least as
  // much: it establishes later than the absolute one, so its window is
  // closer to the retention cutoff by the time anyone looks.
  crate::persistence::cooling_baseline::ensure_baseline_pinned().await;
  crate::persistence::cooling_delta_baseline::ensure_delta_baseline_pinned().await;

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
  // The fan archive is a separate table from `DATA_ARCHIVE` (fan count is
  // configuration-dependent, so fans are rows rather than columns), so it
  // needs its own read - but it is folded into the same day's transaction
  // below rather than by a second worker.
  let fan_minutes =
    database::fan_archive::select_fan_minutes_for_range(&start, &end).await?;
  let fans =
    crate::persistence::cooling_fan_rollup::summarize_fan_day(date, &fan_minutes);

  let pool = database::db::get_pool().await?;
  persist_day_rollup_from_pool(&pool, summary.as_ref(), &hours, &fans).await
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
  fans: &[crate::persistence::cooling_fan_rollup::FanDailySummary],
) -> Result<(), sqlx::Error> {
  use crate::infrastructure::database;

  let mut tx = pool.begin().await?;

  if let Some(summary) = summary {
    database::cooling_daily_summary::upsert_with(&mut *tx, summary).await?;
  }
  for hour in hours {
    database::cooling_hourly_summary::upsert_with(&mut *tx, hour).await?;
  }
  for fan in fans {
    database::cooling_fan_daily_summary::upsert_with(&mut *tx, fan).await?;
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
/// Every pinned baseline's own window is exempt from both deletes. A
/// baseline is a fixed reference that never expires (that is the point of
/// pinning it), so once its window drifts past the retention cutoff,
/// deleting the rows inside it would permanently empty every
/// baseline-side comparison (the load-band comparison, and the Explorer's
/// baseline medians) while the baseline still names that period as the
/// reference.
///
/// There are two such windows since #2045 - the absolute idle baseline's
/// and the ΔT baseline's - and they are generally *different* date
/// ranges, because ambient collection tends to begin long after the
/// machine did. The exemption costs at most two weeks of rows.
pub async fn cleanup_old_data() {
  use crate::infrastructure::database;

  // Deleting without knowing a protected window could erase a baseline's
  // evidence irrecoverably, so either read failing skips this cleanup
  // pass entirely and lets the next boot retry rather than risk it.
  let mut preserved_windows = Vec::new();
  match database::cooling_baseline::select_established_baseline().await {
    Ok(baseline) => {
      preserved_windows.extend(baseline.map(|b| (b.window_start_date, b.window_end_date)))
    }
    Err(e) => {
      log_error!(
        "Failed to read the pinned cooling baseline; skipping cooling rollup cleanup",
        "persistence::cooling_rollup::cleanup_old_data",
        Some(e.to_string())
      );
      return;
    }
  }
  match database::cooling_delta_baseline::select_established_delta_baseline().await {
    Ok(baseline) => {
      preserved_windows.extend(baseline.map(|b| (b.window_start_date, b.window_end_date)))
    }
    Err(e) => {
      log_error!(
        "Failed to read the pinned ΔT cooling baseline; skipping cooling rollup cleanup",
        "persistence::cooling_rollup::cleanup_old_data",
        Some(e.to_string())
      );
      return;
    }
  }

  if let Err(e) = database::cooling_daily_summary::delete_old_data(
    COOLING_DAILY_SUMMARY_RETENTION_DAYS,
    &preserved_windows,
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
    &preserved_windows,
  )
  .await
  {
    log_error!(
      "Failed to delete old cooling hourly summary data",
      "persistence::cooling_rollup::cleanup_old_data",
      Some(e.to_string())
    );
  }

  // The fan rollup shares the same retention constant - it is another
  // projection of the same days - but not the baseline exemption: the
  // pinned baseline is an idle CPU *temperature* reference, so preserving
  // fan rows inside its window would keep data nothing reads.
  if let Err(e) = database::cooling_fan_daily_summary::delete_old_data(
    COOLING_DAILY_SUMMARY_RETENTION_DAYS,
  )
  .await
  {
    log_error!(
      "Failed to delete old cooling fan daily summary data",
      "persistence::cooling_rollup::cleanup_old_data",
      Some(e.to_string())
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::infrastructure::database::test_schema::{
    COOLING_DAILY_SUMMARY_DDL, COOLING_FAN_DAILY_SUMMARY_DDL, COOLING_HOURLY_SUMMARY_DDL,
    create_tables,
  };

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
      cpu_power_avg: None,
      cpu_power_max: None,
      cpu_power_min: None,
      // No ambient pairing by default: the absolute-temperature cases
      // below must keep reading exactly as they did before #2045.
      ambient_temperature_avg: None,
    }
  }

  /// A minute carrying an ambient pairing on top of `sample`'s
  /// temperature fields (#2045). `ambient: None` is a minute the hardware
  /// archive recorded with no usable ambient reading for that minute.
  fn paired_sample(
    cpu_usage_avg: Option<f32>,
    temp: Option<f32>,
    ambient: Option<f32>,
  ) -> ArchiveMinuteSample {
    ArchiveMinuteSample {
      ambient_temperature_avg: ambient,
      ..sample(
        cpu_usage_avg,
        temp,
        temp.map(|t| t + 1.0),
        temp.map(|t| t - 1.0),
      )
    }
  }

  /// A minute carrying a CPU package power reading on top of `sample`'s
  /// temperature fields.
  fn powered_sample(
    cpu_usage_avg: Option<f32>,
    temp: Option<f32>,
    power: Option<f32>,
  ) -> ArchiveMinuteSample {
    ArchiveMinuteSample {
      cpu_power_avg: power,
      cpu_power_max: power.map(|w| w + 2.0),
      cpu_power_min: power.map(|w| w - 2.0),
      ..sample(
        cpu_usage_avg,
        temp,
        temp.map(|t| t + 1.0),
        temp.map(|t| t - 1.0),
      )
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

  // ── summarize_day: CPU package power (#2021) ──

  #[test]
  fn summarize_day_leaves_power_absent_when_no_minute_carried_a_reading() {
    let summary = summarize_day(date(2026, 8, 20), &[full_sample(5.0, 40.0)]).unwrap();

    assert_eq!(
      summary.power,
      PowerSummary::default(),
      "a machine with no power sensor must report absent power, never 0 W"
    );
  }

  #[test]
  fn summarize_day_aggregates_power_avg_of_avgs_and_extremes() {
    let minutes = [
      powered_sample(Some(5.0), Some(40.0), Some(10.0)),
      powered_sample(Some(65.0), Some(80.0), Some(30.0)),
      powered_sample(Some(35.0), Some(60.0), Some(20.0)),
    ];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.power.avg, Some(20.0));
    // `powered_sample` uses power + 2.0 / power - 2.0 for max/min.
    assert_eq!(summary.power.max, Some(32.0));
    assert_eq!(summary.power.min, Some(8.0));
    assert_eq!(summary.power.sample_minutes, 3);
  }

  #[test]
  fn summarize_day_folds_power_independently_of_the_load_band_gate() {
    // The minute has a power reading but no CPU usage, so it contributes
    // to no band - power collection and temperature/load pairing are
    // separate capabilities and must not gate each other.
    let minutes = [powered_sample(None, Some(40.0), Some(25.0))];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    let band_minutes: u32 = [summary.idle, summary.low, summary.mid, summary.high]
      .iter()
      .map(|band| band.sample_minutes)
      .sum();
    assert_eq!(band_minutes, 0);
    assert_eq!(summary.power.avg, Some(25.0));
    assert_eq!(summary.power.sample_minutes, 1);
  }

  #[test]
  fn summarize_day_folds_power_on_a_machine_without_a_temperature_sensor() {
    let minutes = [powered_sample(Some(5.0), None, Some(12.0))];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.idle.sample_minutes, 0);
    assert_eq!(summary.power.avg, Some(12.0));
  }

  #[test]
  fn summarize_day_skips_a_minute_whose_power_reading_is_incomplete() {
    let minutes = [
      powered_sample(Some(5.0), Some(40.0), Some(10.0)),
      // The collector writes avg/max/min together, so a partial triple
      // means a hand-edited or half-migrated row: drop it rather than
      // averaging an extreme-less reading into the day.
      ArchiveMinuteSample {
        cpu_power_avg: Some(999.0),
        cpu_power_max: None,
        cpu_power_min: None,
        ..sample(Some(5.0), Some(40.0), Some(41.0), Some(39.0))
      },
    ];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.power.avg, Some(10.0));
    assert_eq!(summary.power.sample_minutes, 1);
  }

  #[test]
  fn summarize_day_counts_only_powered_minutes_toward_power_sample_minutes() {
    let minutes = [
      powered_sample(Some(5.0), Some(40.0), Some(10.0)),
      // Archived, temperature-paired, but the power sampler was
      // unavailable that minute.
      powered_sample(Some(5.0), Some(40.0), None),
    ];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.coverage_minutes, 2);
    assert_eq!(summary.idle.sample_minutes, 2);
    assert_eq!(summary.power.sample_minutes, 1);
    assert_eq!(summary.power.avg, Some(10.0));
  }

  // ── summarize_day: ambient-normalized thermal delta (#2045) ──

  #[test]
  fn the_normative_pairing_example_from_the_issue() {
    // #2045's worked example, kept verbatim as the executable statement
    // of the rule. Two minutes: one with both readings, one where the
    // ambient sensor had nothing to say.
    //
    //   12:00  CPU 80 degC / Ambient 30 degC  -> delta 50
    //   18:00  CPU 50 degC / Ambient absent   -> no delta
    //
    // Aggregating the two archives independently and subtracting gives
    // avg(CPU) 65 - avg(Ambient) 30 = "35", a number that corresponds to
    // no minute that was ever observed. Pairing first gives 50.
    let minutes = [
      paired_sample(Some(65.0), Some(80.0), Some(30.0)),
      paired_sample(Some(65.0), Some(50.0), None),
    ];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(
      summary.ambient.high.avg,
      Some(50.0),
      "delta must come from the paired minute alone, not from subtracting \
       an ambient average out of a CPU average over a different sample set"
    );
    assert_eq!(
      summary.ambient.high.sample_minutes, 1,
      "only the paired minute may contribute"
    );
    // The absolute temperature band still sees both minutes: the
    // unpaired minute is real data, it just has no delta.
    assert_eq!(summary.high.sample_minutes, 2);
    assert_eq!(summary.high.avg, Some(65.0));
    assert_eq!(summary.ambient.coverage_minutes, 1);
  }

  #[test]
  fn a_delta_band_is_nested_inside_its_temperature_band() {
    // Two minutes in different load bands, each with its own ambient
    // pairing: the delta lands in the same band the temperature did,
    // never pooled across bands.
    let minutes = [
      paired_sample(Some(5.0), Some(40.0), Some(25.0)),
      paired_sample(Some(80.0), Some(70.0), Some(25.0)),
    ];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.ambient.idle.avg, Some(15.0));
    assert_eq!(summary.ambient.idle.sample_minutes, 1);
    assert_eq!(summary.ambient.high.avg, Some(45.0));
    assert_eq!(summary.ambient.high.sample_minutes, 1);
    assert_eq!(summary.ambient.low, BandSummary::default());
    assert_eq!(summary.ambient.mid, BandSummary::default());
  }

  #[test]
  fn a_minute_the_band_gate_rejects_contributes_no_delta_however_good_its_ambient() {
    // The outer gate is the band gate. An ambient reading cannot let a
    // minute with no usable CPU temperature into a delta band - there is
    // nothing to subtract it from.
    let minutes = [
      paired_sample(Some(5.0), Some(40.0), Some(25.0)),
      // Ambient present, CPU temperature absent.
      paired_sample(Some(5.0), None, Some(25.0)),
      // Ambient present, CPU usage absent so no band can be chosen.
      paired_sample(None, Some(40.0), Some(25.0)),
    ];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.ambient.idle.sample_minutes, 1);
    assert_eq!(summary.ambient.idle.avg, Some(15.0));
    // Coverage is counted outside the nesting, so all three minutes had
    // ambient available even though only one produced a delta.
    assert_eq!(
      summary.ambient.coverage_minutes, 3,
      "ambient availability is a separate capability from CPU sensing"
    );
  }

  #[test]
  fn ambient_coverage_is_recorded_without_a_cpu_temperature_sensor() {
    // The machine the backfill cursor must not rewind on forever: an
    // ambient sensor, no CPU temperature sensor. No delta is possible,
    // but the coverage it records is what tells the cursor the day was
    // already processed.
    let minutes = [paired_sample(Some(5.0), None, Some(25.0))];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.ambient.coverage_minutes, 1);
    assert_eq!(
      summary.ambient,
      AmbientDeltaSummary {
        coverage_minutes: 1,
        ..AmbientDeltaSummary::default()
      }
    );
  }

  #[test]
  fn a_delta_extreme_is_the_cpu_extreme_offset_by_that_minutes_ambient() {
    // Within one archived minute there is a single ambient reading, so
    // all three of avg/max/min shift together. Nothing is interpolated
    // between minutes.
    let minutes = [
      // `paired_sample` uses temp + 1.0 / temp - 1.0 for max/min.
      paired_sample(Some(5.0), Some(40.0), Some(25.0)),
      paired_sample(Some(5.0), Some(50.0), Some(20.0)),
    ];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    // Deltas are 15 and 30, so the average is 22.5.
    assert_eq!(summary.ambient.idle.avg, Some(22.5));
    assert_eq!(summary.ambient.idle.max, Some(31.0));
    assert_eq!(summary.ambient.idle.min, Some(14.0));
    assert_eq!(summary.ambient.idle.sample_minutes, 2);
  }

  #[test]
  fn a_machine_with_no_ambient_sensor_reports_a_fully_default_delta_summary() {
    // The invariant every ambient-unaware query depends on: with no
    // ambient rows anywhere, the day's ambient facts are absent, never
    // 0 K and never zero-coverage-with-a-value.
    let minutes = [full_sample(5.0, 40.0), full_sample(65.0, 80.0)];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.ambient, AmbientDeltaSummary::default());
    for band in [
      CpuLoadBand::Idle,
      CpuLoadBand::Low,
      CpuLoadBand::Mid,
      CpuLoadBand::High,
    ] {
      assert_eq!(summary.ambient.band(band).avg, None);
    }
    // ...while the absolute bands are untouched.
    assert_eq!(summary.idle.avg, Some(40.0));
    assert_eq!(summary.high.avg, Some(80.0));
  }

  #[test]
  fn a_negative_delta_is_kept_rather_than_clamped() {
    // A cold-boot minute, or a machine in a room warmer than its own
    // package sensor reads. The number is what it is; clamping would
    // fabricate a reading.
    let minutes = [paired_sample(Some(5.0), Some(20.0), Some(25.0))];
    let summary = summarize_day(date(2026, 8, 20), &minutes).unwrap();

    assert_eq!(summary.ambient.idle.avg, Some(-5.0));
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

  /// A fully caught-up install on a machine with every sensor: the
  /// baseline each case below perturbs one fact of.
  fn caught_up(day: NaiveDate) -> RollupProgress {
    RollupProgress {
      last_daily_date: Some(day),
      last_pairable_daily_date: Some(day),
      last_hourly_date: Some(day),
      last_powered_daily_date: Some(day),
      last_powered_archive_date: Some(day),
      last_fanned_daily_date: Some(day),
      last_fanned_archive_date: Some(day),
      last_ambient_daily_date: Some(day),
      last_ambient_archive_date: Some(day),
    }
  }

  #[test]
  fn the_cursor_follows_the_daily_rollup_when_every_projection_has_kept_up() {
    assert_eq!(
      rollup_catch_up_cursor(caught_up(date(2026, 8, 20))),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn an_empty_hourly_table_rewinds_the_cursor_for_a_full_backfill() {
    // The v13 upgrade path: a long-running install has a full daily table
    // and no hourly rows at all. Following the daily cursor would leave
    // every past day's hourly rows missing forever.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_hourly_date: None,
        ..caught_up(date(2026, 8, 20))
      }),
      None,
      "an empty hourly table must fall back to the earliest-archived-day backfill"
    );
  }

  #[test]
  fn a_lagging_hourly_table_resumes_from_the_slower_of_the_two_cursors() {
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_hourly_date: Some(date(2026, 8, 15)),
        ..caught_up(date(2026, 8, 20))
      }),
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
      rollup_catch_up_cursor(RollupProgress {
        last_pairable_daily_date: None,
        last_hourly_date: None,
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn hourly_ahead_of_the_last_pairable_day_is_still_caught_up() {
    // Recent days recorded coverage but no pairs, so the latest pairable
    // day is older than the daily cursor. Hourly has nothing to add.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_pairable_daily_date: Some(date(2026, 8, 10)),
        last_hourly_date: Some(date(2026, 8, 10)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn a_first_ever_run_still_backfills_from_the_earliest_archived_day() {
    assert_eq!(rollup_catch_up_cursor(RollupProgress::default()), None);
  }

  // ── rollup_catch_up_cursor: CPU package power backfill (#2021) ──

  #[test]
  fn a_daily_table_with_no_power_rewinds_while_the_archive_still_holds_power() {
    // The v14 upgrade path: the daily table is summarized through
    // yesterday but every row's power is NULL, and the one-minute archive
    // still holds readings for the days inside its retention window.
    // Following the daily cursor would leave the power lane blank for the
    // whole window even though the source rows are right there.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_powered_daily_date: None,
        last_powered_archive_date: Some(date(2026, 8, 20)),
        ..caught_up(date(2026, 8, 20))
      }),
      None,
      "the days the archive can still back must be re-rolled so their power fills in"
    );
  }

  #[test]
  fn a_machine_without_a_power_source_does_not_rewind_every_cycle() {
    // The regression this whole check has to avoid: no CPU power source
    // means neither the daily table nor the archive has power, which is
    // not evidence of a missed day. Claiming "behind" here would re-read
    // the entire archive on every catch-up, forever - the same trap the
    // hourly rollup's pairable cursor was built to dodge.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_powered_daily_date: None,
        last_powered_archive_date: None,
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn a_partially_backfilled_power_column_resumes_from_the_last_powered_day() {
    // A previous pass filled power up to 8-15 before failing. Only the
    // remaining days need re-reading, not the whole archive.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_powered_daily_date: Some(date(2026, 8, 15)),
        last_powered_archive_date: Some(date(2026, 8, 20)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 15))
    );
  }

  #[test]
  fn a_power_sampler_that_stopped_days_ago_is_not_treated_as_behind() {
    // The sampler became unavailable after 8-15 (a driver or firmware
    // change). Both sides agree it stopped there, so there is nothing to
    // regenerate and the cursor must stay put.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_powered_daily_date: Some(date(2026, 8, 15)),
        last_powered_archive_date: Some(date(2026, 8, 15)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn a_daily_power_column_ahead_of_the_archive_is_not_treated_as_behind() {
    // Retention has since deleted the archive rows the rollup summarized,
    // so the archive's latest powered day is older than the rollup's.
    // Nothing to regenerate - and nothing that could be.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_powered_daily_date: Some(date(2026, 8, 20)),
        last_powered_archive_date: Some(date(2026, 8, 1)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn the_power_and_hourly_backfills_resume_from_whichever_is_further_behind() {
    // Both projections lag by different amounts; one pass has to satisfy
    // the slower of the two.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_hourly_date: Some(date(2026, 8, 18)),
        last_powered_daily_date: Some(date(2026, 8, 12)),
        last_powered_archive_date: Some(date(2026, 8, 20)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 12))
    );
  }

  #[test]
  fn a_power_backfill_does_not_stall_the_hourly_backfill() {
    // Power is caught up but hourly is not: the power branch must not
    // pull the cursor forward past what hourly still needs.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_hourly_date: Some(date(2026, 8, 12)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 12))
    );
  }

  // ── rollup_catch_up_cursor: fan backfill (#2022) ──

  #[test]
  fn an_empty_fan_table_rewinds_while_the_fan_archive_still_holds_readings() {
    // The migration that adds `cooling_fan_daily_summary` lands on an
    // install already summarized through yesterday. Following the daily
    // cursor would leave the fan lane blank for the whole window even
    // though `FAN_ARCHIVE` still holds the readings.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_fanned_daily_date: None,
        last_fanned_archive_date: Some(date(2026, 8, 20)),
        ..caught_up(date(2026, 8, 20))
      }),
      None
    );
  }

  #[test]
  fn a_machine_without_any_fan_source_does_not_rewind_every_cycle() {
    // Neither side has fan data, which is not evidence of a missed day.
    // The same trap the power and hourly cursors were built to dodge.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_fanned_daily_date: None,
        last_fanned_archive_date: None,
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn a_partially_backfilled_fan_table_resumes_from_the_last_summarized_fan_day() {
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_fanned_daily_date: Some(date(2026, 8, 15)),
        last_fanned_archive_date: Some(date(2026, 8, 20)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 15))
    );
  }

  #[test]
  fn a_fan_sensor_that_stopped_days_ago_is_not_treated_as_behind() {
    // Both sides agree the readings stopped on 8-15, so there is nothing
    // to regenerate and the cursor must stay put.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_fanned_daily_date: Some(date(2026, 8, 15)),
        last_fanned_archive_date: Some(date(2026, 8, 15)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn the_fan_backfill_joins_the_others_at_whichever_is_further_behind() {
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_hourly_date: Some(date(2026, 8, 18)),
        last_powered_daily_date: Some(date(2026, 8, 16)),
        last_fanned_daily_date: Some(date(2026, 8, 11)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 11))
    );
  }

  #[test]
  fn a_caught_up_fan_projection_does_not_stall_the_other_backfills() {
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_hourly_date: Some(date(2026, 8, 12)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 12))
    );
  }

  // ── rollup_catch_up_cursor: ambient delta backfill (#2045) ──
  //
  // The four quadrants of (archive has pairable ambient?) x (daily table
  // recorded coverage?). Only one of them may rewind the cursor; a
  // mistake in any of the other three either strands the delta columns
  // empty forever or re-reads the whole archive on every cycle, forever.

  #[test]
  fn a_daily_table_with_no_ambient_rewinds_while_the_archive_still_pairs() {
    // Quadrant 1 - the migration 16 upgrade path, and the only one that
    // rewinds. An install that was already collecting ambient readings
    // has them in the archive while every summarized row's delta columns
    // are NULL.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_ambient_daily_date: None,
        last_ambient_archive_date: Some(date(2026, 8, 20)),
        ..caught_up(date(2026, 8, 20))
      }),
      None,
      "the days the archives can still back must be re-rolled so their deltas fill in"
    );
  }

  #[test]
  fn a_machine_without_an_ambient_sensor_does_not_rewind_every_cycle() {
    // Quadrant 2 - the regression this whole check exists to avoid.
    // Neither side has ambient, which is not evidence of a missed day.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_ambient_daily_date: None,
        last_ambient_archive_date: None,
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn an_ambient_sensor_that_went_offline_days_ago_is_not_treated_as_behind() {
    // Quadrant 3 - both sides agree ambient stopped after 8-15 (the
    // sensor was unplugged, or its integration was removed). There is
    // nothing to regenerate, so the cursor must stay put.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_ambient_daily_date: Some(date(2026, 8, 15)),
        last_ambient_archive_date: Some(date(2026, 8, 15)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn a_daily_ambient_column_ahead_of_the_archive_is_not_treated_as_behind() {
    // Quadrant 4 - retention has since deleted the ambient rows the
    // rollup already summarized, so the archive's latest pairable day is
    // older than the rollup's. Nothing to regenerate, and nothing that
    // could be.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_ambient_daily_date: Some(date(2026, 8, 20)),
        last_ambient_archive_date: Some(date(2026, 8, 1)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 20))
    );
  }

  #[test]
  fn a_partially_backfilled_ambient_column_resumes_from_the_last_covered_day() {
    // A previous pass filled ambient up to 8-15 before failing. Only the
    // remaining days need re-reading, not the whole archive.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_ambient_daily_date: Some(date(2026, 8, 15)),
        last_ambient_archive_date: Some(date(2026, 8, 20)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 15))
    );
  }

  #[test]
  fn the_ambient_backfill_joins_the_others_at_whichever_is_furthest_behind() {
    // One pass has to satisfy the slowest projection, and ambient is now
    // one of the four candidates rather than a separate cursor.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_hourly_date: Some(date(2026, 8, 18)),
        last_powered_daily_date: Some(date(2026, 8, 16)),
        last_powered_archive_date: Some(date(2026, 8, 20)),
        last_fanned_daily_date: Some(date(2026, 8, 14)),
        last_ambient_daily_date: Some(date(2026, 8, 11)),
        last_ambient_archive_date: Some(date(2026, 8, 20)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 11))
    );
  }

  #[test]
  fn an_ambient_backfill_does_not_stall_the_other_backfills() {
    // Ambient is caught up but hourly is not: the ambient branch must
    // not pull the cursor forward past what the others still need.
    assert_eq!(
      rollup_catch_up_cursor(RollupProgress {
        last_hourly_date: Some(date(2026, 8, 12)),
        ..caught_up(date(2026, 8, 20))
      }),
      Some(date(2026, 8, 12))
    );
  }

  // ── persist_day_rollup_from_pool ──

  mod persist_day_rollup {
    use super::*;
    use crate::persistence::cooling_hourly_rollup::HourlyCoolingSummary;
    use sqlx::SqlitePool;

    async fn create_daily_table(pool: &SqlitePool) {
      create_tables(pool, &[COOLING_DAILY_SUMMARY_DDL]).await;
    }

    async fn create_hourly_table(pool: &SqlitePool) {
      create_tables(pool, &[COOLING_HOURLY_SUMMARY_DDL]).await;
    }

    fn summary() -> DailyCoolingSummary {
      summarize_day(date(2026, 8, 20), &[full_sample(5.0, 40.0)]).unwrap()
    }

    async fn create_fan_table(pool: &SqlitePool) {
      create_tables(pool, &[COOLING_FAN_DAILY_SUMMARY_DDL]).await;
    }

    fn fan() -> crate::persistence::cooling_fan_rollup::FanDailySummary {
      crate::persistence::cooling_fan_rollup::FanDailySummary {
        date: date(2026, 8, 20),
        source: "Fan 1".to_string(),
        rpm_avg: 950.0,
        rpm_max: 1100,
        rpm_min: 800,
        sample_minutes: 60,
      }
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
    async fn a_successful_day_commits_every_projection() {
      let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
      create_daily_table(&pool).await;
      create_hourly_table(&pool).await;
      create_fan_table(&pool).await;

      persist_day_rollup_from_pool(&pool, Some(&summary()), &[hour()], &[fan()])
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
      assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM cooling_fan_daily_summary")
          .fetch_one(&pool)
          .await
          .unwrap(),
        1
      );
    }

    #[tokio::test]
    async fn a_day_without_fan_readings_still_commits_the_other_projections() {
      // Fans are a separate hardware capability: a machine with no fan
      // source must keep its temperature and load rollups.
      let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
      create_daily_table(&pool).await;
      create_hourly_table(&pool).await;
      create_fan_table(&pool).await;

      persist_day_rollup_from_pool(&pool, Some(&summary()), &[hour()], &[])
        .await
        .unwrap();

      assert_eq!(daily_row_count(&pool).await, 1);
      assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM cooling_fan_daily_summary")
          .fetch_one(&pool)
          .await
          .unwrap(),
        0
      );
    }

    #[tokio::test]
    async fn a_failed_hourly_write_rolls_the_daily_row_back() {
      // The projections must land together: a committed daily row with
      // its hourly rows missing is a half-written day the catch-up
      // cursor cannot reliably distinguish from a day that had no pairs.
      let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
      create_daily_table(&pool).await;
      // `cooling_hourly_summary` deliberately absent, so the hourly
      // upsert fails after the daily one has already been issued.

      let result =
        persist_day_rollup_from_pool(&pool, Some(&summary()), &[hour()], &[fan()]).await;

      assert!(result.is_err(), "the day must fail as a whole");
      assert_eq!(
        daily_row_count(&pool).await,
        0,
        "the daily row must not survive a failed hourly write"
      );
    }

    #[tokio::test]
    async fn a_failed_fan_write_rolls_the_whole_day_back() {
      // Same contract for the fan projection: a day the fan cursor would
      // report as summarized while its rows are missing is exactly the
      // half-written state the transaction exists to prevent.
      let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
      create_daily_table(&pool).await;
      create_hourly_table(&pool).await;
      // `cooling_fan_daily_summary` deliberately absent.

      let result =
        persist_day_rollup_from_pool(&pool, Some(&summary()), &[hour()], &[fan()]).await;

      assert!(result.is_err(), "the day must fail as a whole");
      assert_eq!(daily_row_count(&pool).await, 0);
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
