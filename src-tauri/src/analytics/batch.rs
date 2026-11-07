//! Local aggregation and sealed batches.
//!
//! Counters accumulate until a flush seals them into a batch. A sealed batch
//! has a random id and never changes again, so a retry after a lost response
//! is a repeat the service can deduplicate; merging new counters into a
//! failed batch would need a new id and double count the stored one.

use std::collections::{BTreeMap, VecDeque};

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

use super::consent::GrantedConsent;
use super::period::{UtcMonth, serialize_day};
use super::vocabulary::{CpuArch, OsFamily, ProductEvent, StateFlag};

pub(crate) const SCHEMA_VERSION: u32 = 1;
/// Unsent batches kept in memory; the oldest is dropped beyond this.
pub(crate) const MAX_QUEUED_BATCHES: usize = 8;

/// One contract v1 batch, serialized exactly as it is sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SealedBatch {
  schema_version: u32,
  consent_version: u32,
  batch_id: uuid::Uuid,
  app_version: &'static str,
  os_family: OsFamily,
  cpu_arch: CpuArch,
  cohort_month: UtcMonth,
  installation: Installation,
  #[serde(skip_serializing_if = "Option::is_none")]
  flags: Option<Vec<StateFlag>>,
  counters: Vec<Counter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Installation {
  #[serde(serialize_with = "serialize_day")]
  day: NaiveDate,
  day_token: String,
  month: UtcMonth,
  #[serde(skip_serializing_if = "Option::is_none")]
  month_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Counter {
  #[serde(serialize_with = "serialize_day")]
  day: NaiveDate,
  name: &'static str,
  count: u32,
}

impl SealedBatch {
  pub(crate) fn id(&self) -> uuid::Uuid {
    self.batch_id
  }

  #[cfg(test)]
  pub(crate) fn flags(&self) -> Option<&[StateFlag]> {
    self.flags.as_deref()
  }

  #[cfg(test)]
  pub(crate) fn has_month_token(&self) -> bool {
    self.installation.month_token.is_some()
  }

  #[cfg(test)]
  pub(crate) fn counters(&self) -> Vec<(NaiveDate, &'static str, u32)> {
    self
      .counters
      .iter()
      .map(|counter| (counter.day, counter.name, counter.count))
      .collect()
  }
}

/// Periods a seal asks the settings service to record before its batch is
/// queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReportedPeriods {
  pub(crate) day: Option<NaiveDate>,
  pub(crate) month: Option<UtcMonth>,
}

/// Counters keyed by `(utc_day, event_name)` and the queue of sealed,
/// unsent batches. The owner holds it behind one lock.
#[derive(Debug, Default)]
pub(crate) struct LocalAggregate {
  counters: BTreeMap<(NaiveDate, &'static str), u32>,
  queue: VecDeque<SealedBatch>,
}

impl LocalAggregate {
  /// Counts one event on the UTC day of `now`.
  pub(crate) fn record(&mut self, event: ProductEvent, now: DateTime<Utc>) {
    let count = self
      .counters
      .entry((now.date_naive(), event.wire_name()))
      .or_default();
    *count = count.saturating_add(1);
  }

  /// Seals everything counted since the previous seal into a new batch and
  /// queues it.
  ///
  /// `flags` is the day's complete flag set, or `None` while a source is
  /// unsettled. The month token is sealed for a month later than
  /// `reported_month`, and flags for a day later than `reported_day`, so a
  /// restart or a clock set backwards never sends either twice. Both are
  /// recorded through `persist` before the batch is queued; when that write
  /// fails the batch is sealed without them, because an undercount is
  /// accepted and a repeat is not.
  pub(crate) fn seal<P>(
    &mut self,
    consent: &mut GrantedConsent,
    now: DateTime<Utc>,
    flags: Option<Vec<StateFlag>>,
    persist: P,
  ) where
    P: FnOnce(ReportedPeriods) -> Result<(), String>,
  {
    let day = now.date_naive();
    let month = UtcMonth::of(day);
    let mut reported = ReportedPeriods {
      day: flags
        .is_some()
        .then_some(day)
        .filter(|day| consent.reported_day.is_none_or(|reported| *day > reported)),
      month: Some(month).filter(|month| {
        consent
          .reported_month
          .is_none_or(|reported| *month > reported)
      }),
    };
    if (reported.day.is_some() || reported.month.is_some()) && persist(reported).is_err()
    {
      reported = ReportedPeriods {
        day: None,
        month: None,
      };
    }
    if reported.day.is_some() {
      consent.reported_day = reported.day;
    }
    if reported.month.is_some() {
      consent.reported_month = reported.month;
    }

    let counters = std::mem::take(&mut self.counters)
      .into_iter()
      .map(|((day, name), count)| Counter { day, name, count })
      .collect();
    let batch = SealedBatch {
      schema_version: SCHEMA_VERSION,
      consent_version: consent.consent_version,
      batch_id: uuid::Uuid::new_v4(),
      app_version: env!("CARGO_PKG_VERSION"),
      os_family: OsFamily::CURRENT,
      cpu_arch: CpuArch::CURRENT,
      cohort_month: consent.cohort_month,
      installation: Installation {
        day,
        day_token: consent.seed.day_token(day),
        month,
        month_token: reported.month.map(|month| consent.seed.month_token(month)),
      },
      flags: reported.day.and(flags).map(|mut flags| {
        flags.sort_unstable();
        flags.dedup();
        flags
      }),
      counters,
    };

    if self.queue.len() == MAX_QUEUED_BATCHES {
      self.queue.pop_front();
    }
    self.queue.push_back(batch);
  }

  /// The oldest unsent batch, which the next flush sends first.
  pub(crate) fn head(&self) -> Option<&SealedBatch> {
    self.queue.front()
  }

  /// Removes the head batch once it is accepted or terminally rejected, but
  /// only if it is still the batch the caller sent.
  pub(crate) fn remove_head(&mut self, id: uuid::Uuid) {
    if self.queue.front().is_some_and(|batch| batch.id() == id) {
      self.queue.pop_front();
    }
  }

  /// Discards every counter and queued batch.
  pub(crate) fn clear(&mut self) {
    self.counters.clear();
    self.queue.clear();
  }

  #[cfg(test)]
  pub(crate) fn queued(&self) -> Vec<SealedBatch> {
    self.queue.iter().cloned().collect()
  }

  #[cfg(test)]
  pub(crate) fn counter_count(&self) -> usize {
    self.counters.len()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::analytics::consent::test_support::granted_section;
  use crate::analytics::period::parse_day;
  use crate::analytics::vocabulary::Capability;
  use crate::analytics::vocabulary::{
    CpuVendor, Insight, Navigation, PerformanceView, Screen,
  };
  use chrono::TimeZone;

  fn at(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
      .unwrap()
      .with_timezone(&Utc)
  }

  fn consent() -> GrantedConsent {
    GrantedConsent::from_settings(&granted_section()).unwrap()
  }

  fn flags() -> Vec<StateFlag> {
    vec![
      StateFlag::PerformanceView(PerformanceView::Panels),
      StateFlag::Navigation(Navigation::Grouped),
      StateFlag::CpuVendor(CpuVendor::Amd),
    ]
  }

  fn ok(_: ReportedPeriods) -> Result<(), String> {
    Ok(())
  }

  fn failing(_: ReportedPeriods) -> Result<(), String> {
    Err("injected settings write failure".to_string())
  }

  const PERFORMANCE: ProductEvent = ProductEvent::ScreenView(Screen::Performance);

  #[test]
  fn counters_bucket_by_utc_day_across_midnight() {
    let mut aggregate = LocalAggregate::default();
    aggregate.record(PERFORMANCE, at("2026-11-03T23:59:59Z"));
    aggregate.record(PERFORMANCE, at("2026-11-03T23:59:59Z"));
    // 09:00 in UTC+9 is still the previous UTC day.
    aggregate.record(PERFORMANCE, at("2026-11-04T08:59:59+09:00"));
    aggregate.record(PERFORMANCE, at("2026-11-04T00:00:00Z"));
    aggregate.record(
      ProductEvent::Capability(Capability::InsightOpened(Insight::Gpu)),
      Utc.with_ymd_and_hms(2026, 11, 4, 0, 0, 1).unwrap(),
    );

    aggregate.seal(&mut consent(), at("2026-11-04T01:00:00Z"), None, ok);
    let batch = aggregate.head().unwrap();
    assert_eq!(
      batch.counters(),
      [
        (
          parse_day("2026-11-03").unwrap(),
          "screen_view.performance",
          3
        ),
        (
          parse_day("2026-11-04").unwrap(),
          "capability.insight_opened.gpu",
          1
        ),
        (
          parse_day("2026-11-04").unwrap(),
          "screen_view.performance",
          1
        ),
      ]
    );
  }

  #[test]
  fn a_batch_without_counters_is_still_a_heartbeat() {
    let mut aggregate = LocalAggregate::default();
    aggregate.seal(&mut consent(), at("2026-11-03T12:00:00Z"), None, ok);
    let json = serde_json::to_value(aggregate.head().unwrap()).unwrap();
    assert_eq!(json["counters"], serde_json::json!([]));
    assert_eq!(json["installation"]["day"], "2026-11-03");
    assert_eq!(json["cohortMonth"], "2026-11");
  }

  /// The payload shape of the design's contract v1 example.
  #[test]
  fn sealed_batch_serializes_to_the_contract_v1_shape() {
    let mut aggregate = LocalAggregate::default();
    aggregate.record(PERFORMANCE, at("2026-11-03T10:00:00Z"));
    aggregate.seal(
      &mut consent(),
      at("2026-11-03T12:00:00Z"),
      Some(flags()),
      ok,
    );
    let batch = aggregate.head().unwrap();
    let json = serde_json::to_value(batch).unwrap();

    assert_eq!(
      json,
      serde_json::json!({
        "schemaVersion": 1,
        "consentVersion": 1,
        "batchId": batch.id().to_string(),
        "appVersion": env!("CARGO_PKG_VERSION"),
        "osFamily": std::env::consts::OS,
        "cpuArch": std::env::consts::ARCH,
        "cohortMonth": "2026-11",
        "installation": {
          "day": "2026-11-03",
          "dayToken": "2ca1f2b7672934d3f30ff1f906d5ff4d",
          "month": "2026-11",
          "monthToken": "b6a3ae032d2ace6d4dbe487bba0ed1ab"
        },
        "flags": [
          "navigation.grouped",
          "view.performance.panels",
          "hardware.cpu_vendor.amd"
        ],
        "counters": [
          { "day": "2026-11-03", "name": "screen_view.performance", "count": 1 }
        ]
      })
    );
    assert_eq!(batch.id().get_version_num(), 4);
  }

  #[test]
  fn a_sealed_batch_keeps_its_id_and_counters_while_new_events_go_to_the_next() {
    let mut aggregate = LocalAggregate::default();
    aggregate.record(PERFORMANCE, at("2026-11-03T10:00:00Z"));
    aggregate.seal(&mut consent(), at("2026-11-03T12:00:00Z"), None, ok);
    let first = aggregate.head().unwrap().clone();

    aggregate.record(PERFORMANCE, at("2026-11-03T13:00:00Z"));
    aggregate.record(PERFORMANCE, at("2026-11-03T13:00:01Z"));
    aggregate.seal(&mut consent(), at("2026-11-03T18:00:00Z"), None, ok);

    let queued = aggregate.queued();
    assert_eq!(queued[0], first, "a failed head is retried unchanged");
    assert_ne!(queued[1].id(), first.id());
    assert_eq!(queued[1].counters()[0].2, 2);
  }

  #[test]
  fn the_queue_is_bounded_and_drops_the_oldest() {
    let mut aggregate = LocalAggregate::default();
    let mut consent = consent();
    let mut ids = Vec::new();
    for _ in 0..MAX_QUEUED_BATCHES + 2 {
      aggregate.seal(&mut consent, at("2026-11-03T12:00:00Z"), None, ok);
      ids.push(aggregate.queued().last().unwrap().id());
    }
    let queued: Vec<_> = aggregate.queued().iter().map(SealedBatch::id).collect();
    assert_eq!(queued, ids[2..]);
  }

  #[test]
  fn remove_head_ignores_a_batch_that_is_no_longer_the_head() {
    let mut aggregate = LocalAggregate::default();
    aggregate.seal(&mut consent(), at("2026-11-03T12:00:00Z"), None, ok);
    let id = aggregate.head().unwrap().id();
    aggregate.clear();
    aggregate.seal(&mut consent(), at("2026-11-03T13:00:00Z"), None, ok);
    aggregate.remove_head(id);
    assert_eq!(aggregate.queued().len(), 1);
    aggregate.remove_head(aggregate.head().unwrap().id());
    assert!(aggregate.head().is_none());
  }

  #[test]
  fn the_month_token_is_sealed_once_per_month() {
    let mut aggregate = LocalAggregate::default();
    let mut consent = consent();
    let mut writes = Vec::new();
    let mut seal = |aggregate: &mut LocalAggregate, now: &str| {
      aggregate.seal(&mut consent, at(now), None, |periods| {
        writes.push(periods);
        Ok(())
      });
      aggregate.queued().last().unwrap().has_month_token()
    };

    assert!(seal(&mut aggregate, "2026-11-03T12:00:00Z"));
    assert!(!seal(&mut aggregate, "2026-11-03T18:00:00Z"));
    assert!(!seal(&mut aggregate, "2026-11-30T23:00:00Z"));
    assert!(seal(&mut aggregate, "2026-12-01T00:00:00Z"));
    // A clock set back into an earlier month never re-sends it.
    assert!(!seal(&mut aggregate, "2026-11-15T00:00:00Z"));
    assert_eq!(
      writes,
      [
        ReportedPeriods {
          day: None,
          month: UtcMonth::parse("2026-11")
        },
        ReportedPeriods {
          day: None,
          month: UtcMonth::parse("2026-12")
        },
      ]
    );
  }

  #[test]
  fn a_persisted_reported_month_survives_a_restart() {
    let mut section = granted_section();
    section.reported_month = Some("2026-11".to_string());
    let mut restarted = GrantedConsent::from_settings(&section).unwrap();
    let mut aggregate = LocalAggregate::default();
    aggregate.seal(&mut restarted, at("2026-11-20T12:00:00Z"), None, |_| {
      panic!("nothing to record")
    });
    assert!(!aggregate.head().unwrap().has_month_token());
  }

  #[test]
  fn flags_are_sealed_once_per_utc_day_after_the_reported_day() {
    let mut section = granted_section();
    section.reported_day = Some("2026-11-03".to_string());
    section.reported_month = Some("2026-11".to_string());
    // A restart on the reported day reads the persisted day.
    let mut consent = GrantedConsent::from_settings(&section).unwrap();
    let mut aggregate = LocalAggregate::default();
    let mut writes = Vec::new();
    let mut seal = |aggregate: &mut LocalAggregate, now: &str| {
      aggregate.seal(&mut consent, at(now), Some(flags()), |periods| {
        writes.push(periods);
        Ok(())
      });
      aggregate.queued().last().unwrap().flags().is_some()
    };

    assert!(!seal(&mut aggregate, "2026-11-03T20:00:00Z"));
    assert!(seal(&mut aggregate, "2026-11-04T00:00:00Z"));
    assert!(!seal(&mut aggregate, "2026-11-04T06:00:00Z"));
    // A clock earlier than the reported day seals no flags.
    assert!(!seal(&mut aggregate, "2026-11-02T06:00:00Z"));
    assert_eq!(
      writes,
      [ReportedPeriods {
        day: parse_day("2026-11-04"),
        month: None
      }]
    );
  }

  #[test]
  fn an_unsettled_flag_set_leaves_the_reported_day_for_a_later_batch() {
    let mut consent = consent();
    let mut aggregate = LocalAggregate::default();
    aggregate.seal(&mut consent, at("2026-11-03T12:00:00Z"), None, ok);
    assert!(aggregate.head().unwrap().flags().is_none());
    assert_eq!(consent.reported_day, None);

    aggregate.seal(&mut consent, at("2026-11-03T18:00:00Z"), Some(flags()), ok);
    assert!(aggregate.queued()[1].flags().is_some());
    assert_eq!(consent.reported_day, parse_day("2026-11-03"));
  }

  #[test]
  fn a_failed_reported_period_write_seals_the_batch_without_flags_or_month_token() {
    let mut consent = consent();
    let mut aggregate = LocalAggregate::default();
    aggregate.seal(
      &mut consent,
      at("2026-11-03T12:00:00Z"),
      Some(flags()),
      failing,
    );

    let batch = aggregate.head().unwrap();
    assert!(batch.flags().is_none());
    assert!(!batch.has_month_token());
    assert_eq!(consent.reported_day, None);
    assert_eq!(consent.reported_month, None);

    // The next seal that day tries again.
    aggregate.seal(&mut consent, at("2026-11-03T18:00:00Z"), Some(flags()), ok);
    assert!(aggregate.queued()[1].flags().is_some());
    assert!(aggregate.queued()[1].has_month_token());
  }

  #[test]
  fn clear_discards_counters_and_the_queue() {
    let mut aggregate = LocalAggregate::default();
    aggregate.record(PERFORMANCE, at("2026-11-03T10:00:00Z"));
    aggregate.seal(&mut consent(), at("2026-11-03T12:00:00Z"), None, ok);
    aggregate.record(PERFORMANCE, at("2026-11-03T13:00:00Z"));
    aggregate.clear();
    assert!(aggregate.head().is_none());
    assert_eq!(aggregate.counter_count(), 0);
  }
}
