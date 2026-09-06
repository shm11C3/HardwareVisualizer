//! Opt-in orchestration for the issue #2052 long-range query experiment.
//!
//! This module measures query-only structures built offline on the already
//! verified synthetic candidate. It makes no production maintenance or
//! atomicity claim.

use super::*;

const PROCESS_PAGE_SIZE: usize = 500;

#[derive(Serialize)]
pub(crate) struct QueryExperimentReport {
  production_candidate_accepted: bool,
  numerical_contract_passed: bool,
  cache_state: &'static str,
  temp_store: &'static str,
  temp_cache_kib: i64,
  execution_policy: &'static str,
  metadata_footprint: MetadataFootprint,
  build: BuildReport,
  ranges: Vec<RangeReport>,
  correctness: ExperimentCorrectness,
  limitations: [&'static str; 7],
}

#[derive(Serialize)]
struct MetadataFootprint {
  after_vacuum_before_build: FileFootprint,
  after_build: FileFootprint,
  database_and_wal_growth_bytes: i128,
}

#[derive(Serialize)]
struct BuildReport {
  process_summary: process_summary::BuildMetrics,
  ambient_epoch_bounds: ambient_bounds::BuildReport,
}

#[derive(Serialize)]
struct RangeReport {
  name: &'static str,
  start: String,
  end: String,
  represented_minutes: u64,
  repetitions: Vec<RepetitionReport>,
}

#[derive(Serialize)]
struct RepetitionReport {
  repetition: usize,
  execution_order: [&'static str; 3],
  process: ProcessRunReport,
  ambient: AmbientRunReport,
  exact_validation_passed: bool,
}

#[derive(Serialize)]
struct ProcessRunReport {
  relational_oracle: RelationalProcessMetrics,
  current_full_decode: CurrentProcessRunMetrics,
  accelerated_summary: process_summary::QueryMetrics,
  full_result_equivalent: bool,
  first_page_equivalent: bool,
}

#[derive(Serialize)]
struct RelationalProcessMetrics {
  total_ms: f64,
  first_page_ms: f64,
  result_groups: usize,
  first_page_groups: usize,
  full_result_materialized_for_validation: bool,
  original_canonical_order: &'static str,
  first_page_order: &'static str,
}

#[derive(Serialize)]
struct CurrentProcessRunMetrics {
  total_with_ranked_page_ms: f64,
  first_page_ms: f64,
  first_page_groups: usize,
  full_result_materialized_for_validation: bool,
  phases: CurrentProcessQueryMetrics,
}

#[derive(Serialize)]
struct AmbientRunReport {
  relational_oracle: RelationalAmbientMetrics,
  current_full_scan: CurrentAmbientQueryMetrics,
  accelerated_bounds: ambient_bounds::QueryResult,
  exact_digest_and_count_equivalent: bool,
}

#[derive(Serialize)]
struct RelationalAmbientMetrics {
  total_ms: f64,
  rows: u64,
}

#[derive(Default, Serialize)]
struct ExperimentCorrectness {
  generated_workload_every_repetition_passed: bool,
  process_full_result_equivalent: bool,
  process_first_page_equivalent: bool,
  ambient_digest_and_count_equivalent: bool,
  process_float_tolerance: &'static str,
}

#[derive(Clone, Copy)]
enum Strategy {
  Relational,
  Current,
  Accelerated,
}

impl Strategy {
  fn name(self) -> &'static str {
    match self {
      Self::Relational => "relational_oracle",
      Self::Current => "current_chunk_decode",
      Self::Accelerated => "experimental_accelerator",
    }
  }
}

struct ProcessOutputs {
  relational: Option<(Vec<Aggregate>, Vec<Aggregate>, RelationalProcessMetrics)>,
  current: Option<(Vec<Aggregate>, Vec<Aggregate>, CurrentProcessRunMetrics)>,
  accelerated: Option<process_summary::QueryResult>,
}

struct AmbientOutputs {
  relational: Option<((Vec<u8>, u64), RelationalAmbientMetrics)>,
  current: Option<((Vec<u8>, u64), CurrentAmbientQueryMetrics)>,
  accelerated: Option<ambient_bounds::QueryResult>,
}

pub(super) async fn run(
  config: &Config,
  baseline: &mut SqliteConnection,
  candidate: &mut SqliteConnection,
  candidate_path: &Path,
) -> Result<QueryExperimentReport> {
  configure_shared_temp_policy(baseline).await?;
  configure_shared_temp_policy(candidate).await?;
  let before = footprint(candidate, candidate_path).await?;
  let process_build = process_summary::build(candidate).await?;
  let numerical_contract_passed = process_build.numerical_probe.contract_passed;
  let ambient_build = ambient_bounds::build(candidate).await?;
  checkpoint(candidate).await?;
  let after = footprint(candidate, candidate_path).await?;
  let footprint_growth = total_file_bytes(&after) - total_file_bytes(&before);

  let mut correctness = ExperimentCorrectness {
    generated_workload_every_repetition_passed: true,
    process_full_result_equivalent: true,
    process_first_page_equivalent: true,
    ambient_digest_and_count_equivalent: true,
    process_float_tolerance: "abs(actual-reference) <= max(1e-9, 1e-12*abs(reference))",
  };
  let mut range_reports = Vec::new();
  for range in query_ranges(config.minutes)? {
    let mut repetitions = Vec::with_capacity(config.repetitions);
    for repetition in 0..config.repetitions {
      let order = rotated_order(repetition);
      let mut process = ProcessOutputs {
        relational: None,
        current: None,
        accelerated: None,
      };
      let mut ambient = AmbientOutputs {
        relational: None,
        current: None,
        accelerated: None,
      };
      for strategy in order {
        run_strategy(
          strategy,
          config,
          baseline,
          candidate,
          &range,
          &mut process,
          &mut ambient,
        )
        .await?;
      }
      let repetition_report = validate_repetition(repetition, order, process, ambient)?;
      correctness.generated_workload_every_repetition_passed &=
        repetition_report.exact_validation_passed;
      correctness.process_full_result_equivalent &=
        repetition_report.process.full_result_equivalent;
      correctness.process_first_page_equivalent &=
        repetition_report.process.first_page_equivalent;
      correctness.ambient_digest_and_count_equivalent &=
        repetition_report.ambient.exact_digest_and_count_equivalent;
      if !repetition_report.exact_validation_passed {
        return Err(
          format!(
            "query experiment differential validation failed for {} repetition {}",
            range.name,
            repetition + 1
          )
          .into(),
        );
      }
      repetitions.push(repetition_report);
    }
    range_reports.push(RangeReport {
      name: range.name,
      start: range.start,
      end: range.end,
      represented_minutes: range.represented_minutes,
      repetitions,
    });
  }

  let production_candidate_accepted = numerical_contract_passed
    && correctness.generated_workload_every_repetition_passed
    && correctness.process_full_result_equivalent
    && correctness.process_first_page_equivalent
    && correctness.ambient_digest_and_count_equivalent;
  Ok(QueryExperimentReport {
    production_candidate_accepted,
    numerical_contract_passed,
    cache_state: "cache-primed repeated queries; controlled cold-cache behavior was not measured",
    temp_store: "FILE",
    temp_cache_kib: 16 * 1024,
    execution_policy: "A/B/C order rotates by repetition; each path uses its own SQLite snapshot",
    metadata_footprint: MetadataFootprint {
      after_vacuum_before_build: before,
      after_build: after,
      database_and_wal_growth_bytes: footprint_growth,
    },
    build: BuildReport {
      process_summary: process_build,
      ambient_epoch_bounds: ambient_build,
    },
    ranges: range_reports,
    correctness,
    limitations: [
      "The accelerators are built once offline after source and finalized-data verification; production atomic maintenance is not implemented.",
      "Process validation materializes every aggregate; Ambient first-page timing follows materialization of all matching rows and is not a bounded streaming page.",
      "First-page latency and total full-result materialization are reported separately; the relational and current paths require the full group set before CPU ranking.",
      "Repeated queries are cache-primed and do not establish controlled cold-cache behavior.",
      "The Process summary fails the retained float tolerance on the reported cancellation counterexample; its performance does not make it an acceptable production candidate.",
      "The experiment covers synthetic 24-hour, 30-day, and one-year inputs when selected by the runner, not ten-year history or a production consumer.",
      "The churn selector is synthetic cardinality stress with bounded PID reuse and generated names, not representative executable-name churn.",
    ],
  })
}

async fn run_strategy(
  strategy: Strategy,
  config: &Config,
  baseline: &mut SqliteConnection,
  candidate: &mut SqliteConnection,
  range: &QueryRange,
  process: &mut ProcessOutputs,
  ambient: &mut AmbientOutputs,
) -> Result<()> {
  match strategy {
    Strategy::Relational => {
      let process_started = Instant::now();
      let (aggregates, _) =
        process_oracle_measured(baseline, &range.start, &range.end).await?;
      let first_page = aggregates
        .iter()
        .take(PROCESS_PAGE_SIZE)
        .cloned()
        .collect::<Vec<_>>();
      let result_groups = aggregates.len();
      let process_total_ms = elapsed_ms(process_started);
      process.relational = Some((
        aggregates,
        first_page.clone(),
        RelationalProcessMetrics {
          total_ms: process_total_ms,
          first_page_ms: process_total_ms,
          result_groups,
          first_page_groups: first_page.len(),
          full_result_materialized_for_validation: true,
          original_canonical_order: "the standard benchmark gate separately uses pid ASC, process_name bytes ASC",
          first_page_order: "average CPU DESC, pid ASC, process_name bytes ASC",
        },
      ));
      let (result, total_ms) =
        ambient_oracle_measured(baseline, range.start_ms, range.end_ms).await?;
      ambient.relational = Some((
        result.clone(),
        RelationalAmbientMetrics {
          total_ms,
          rows: result.1,
        },
      ));
    }
    Strategy::Current => {
      let process_started = Instant::now();
      let (aggregates, phases) =
        process_chunked_measured(candidate, &range.start, &range.end, config.group_cap)
          .await?;
      let first_page = aggregates
        .iter()
        .take(PROCESS_PAGE_SIZE)
        .cloned()
        .collect::<Vec<_>>();
      let total_with_ranked_page_ms = elapsed_ms(process_started);
      process.current = Some((
        aggregates,
        first_page.clone(),
        CurrentProcessRunMetrics {
          total_with_ranked_page_ms,
          first_page_ms: total_with_ranked_page_ms,
          first_page_groups: first_page.len(),
          full_result_materialized_for_validation: true,
          phases,
        },
      ));
      ambient.current =
        Some(ambient_chunked_measured(candidate, range.start_ms, range.end_ms).await?);
    }
    Strategy::Accelerated => {
      process.accelerated =
        Some(process_summary::query(candidate, &range.start, &range.end).await?);
      ambient.accelerated =
        Some(ambient_bounds::query(candidate, range.start_ms, range.end_ms).await?);
    }
  }
  Ok(())
}

fn validate_repetition(
  repetition: usize,
  order: [Strategy; 3],
  process: ProcessOutputs,
  ambient: AmbientOutputs,
) -> Result<RepetitionReport> {
  let (reference, reference_page, reference_metrics) = process
    .relational
    .ok_or("missing relational process result")?;
  let (current, current_page, current_metrics) =
    process.current.ok_or("missing current process result")?;
  let accelerated = process
    .accelerated
    .ok_or("missing accelerated process result")?;
  let reference_canonical = canonical_process_results(&reference);
  let current_canonical = canonical_process_results(&current);
  let accelerated_canonical = canonical_process_results(&accelerated.aggregates);
  let full_result_equivalent =
    compare_aggregates(&reference_canonical, &current_canonical)
      && compare_aggregates(&reference_canonical, &accelerated_canonical);
  let first_page_equivalent = compare_aggregates(&reference_page, &current_page)
    && compare_aggregates(&reference_page, &accelerated.first_page);

  let (ambient_reference, ambient_reference_metrics) = ambient
    .relational
    .ok_or("missing relational ambient result")?;
  let (ambient_current, ambient_current_metrics) =
    ambient.current.ok_or("missing current ambient result")?;
  let ambient_accelerated = ambient
    .accelerated
    .ok_or("missing accelerated ambient result")?;
  let ambient_equivalent = ambient_reference == ambient_current
    && ambient_reference.0 == ambient_accelerated.digest
    && ambient_reference.1 == ambient_accelerated.count;
  let exact_validation_passed =
    full_result_equivalent && first_page_equivalent && ambient_equivalent;

  Ok(RepetitionReport {
    repetition: repetition + 1,
    execution_order: order.map(Strategy::name),
    process: ProcessRunReport {
      relational_oracle: reference_metrics,
      current_full_decode: current_metrics,
      accelerated_summary: accelerated.metrics,
      full_result_equivalent,
      first_page_equivalent,
    },
    ambient: AmbientRunReport {
      relational_oracle: ambient_reference_metrics,
      current_full_scan: ambient_current_metrics,
      accelerated_bounds: ambient_accelerated,
      exact_digest_and_count_equivalent: ambient_equivalent,
    },
    exact_validation_passed,
  })
}

fn canonical_process_results(aggregates: &[Aggregate]) -> Vec<Aggregate> {
  let mut canonical = aggregates.to_vec();
  canonical.sort_by(|left, right| (left.pid, &left.name).cmp(&(right.pid, &right.name)));
  canonical
}

fn rotated_order(repetition: usize) -> [Strategy; 3] {
  match repetition % 3 {
    0 => [
      Strategy::Relational,
      Strategy::Current,
      Strategy::Accelerated,
    ],
    1 => [
      Strategy::Current,
      Strategy::Accelerated,
      Strategy::Relational,
    ],
    _ => [
      Strategy::Accelerated,
      Strategy::Relational,
      Strategy::Current,
    ],
  }
}

struct QueryRange {
  name: &'static str,
  start: String,
  end: String,
  start_ms: i64,
  end_ms: i64,
  represented_minutes: u64,
}

fn query_ranges(minutes: u64) -> Result<[QueryRange; 2]> {
  if minutes < 30 {
    return Err("--query-experiment requires at least 30 represented minutes".into());
  }
  let last = minutes - 1;
  let half_start = (minutes / 2 + 7).min(last.saturating_sub(1));
  let half_end = last.saturating_sub(3).max(half_start);
  let middle_duration = (minutes / 3).clamp(1, 1_440);
  let middle_base = (minutes - middle_duration) / 2;
  let middle_start = (middle_base + 11).min(minutes - middle_duration);
  let middle_end = middle_start + middle_duration - 1;
  Ok([
    make_range("half_history_unaligned", half_start, half_end)?,
    make_range("middle_24h_or_middle_third", middle_start, middle_end)?,
  ])
}

fn make_range(
  name: &'static str,
  start_minute: u64,
  end_minute: u64,
) -> Result<QueryRange> {
  let start = timestamp(start_minute)?;
  let end = timestamp(end_minute)?;
  Ok(QueryRange {
    name,
    start_ms: parse_timestamp(&start)?,
    end_ms: parse_timestamp(&end)?,
    start,
    end,
    represented_minutes: end_minute - start_minute + 1,
  })
}

fn total_file_bytes(footprint: &FileFootprint) -> i128 {
  i128::from(footprint.database_bytes) + i128::from(footprint.wal_bytes)
}

async fn configure_shared_temp_policy(db: &mut SqliteConnection) -> Result<()> {
  sqlx::query("PRAGMA temp_store = FILE")
    .execute(&mut *db)
    .await?;
  sqlx::query("PRAGMA temp.cache_size = -16384")
    .execute(&mut *db)
    .await?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ranges_are_narrow_and_avoid_chunk_boundaries() {
    for minutes in [1_440, 43_200, 525_600] {
      let ranges = query_ranges(minutes).unwrap();
      assert!(ranges[0].represented_minutes < minutes);
      assert!(ranges[1].represented_minutes <= 1_440);
      for range in ranges {
        let start_minute =
          u64::try_from((range.start_ms - 1_767_225_600_000_i64) / 60_000).unwrap();
        let end_minute =
          u64::try_from((range.end_ms - 1_767_225_600_000_i64) / 60_000).unwrap();
        assert_ne!(start_minute % 60, 0);
        assert_ne!(end_minute % 60, 0);
      }
    }
  }
}
