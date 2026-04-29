use crate::{infrastructure::database, log_error, models};
use std::{
  collections::{HashMap, HashSet, VecDeque},
  sync::{Arc, Mutex},
};

const PROCESS_RECORD_LIMIT: usize = 5;

type ProcessHistory = Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>;

#[derive(Debug, Clone, Copy)]
enum ProcessRankingMetric {
  Cpu,
  Memory,
  ExecutionTime,
}

impl ProcessRankingMetric {
  const ALL: [Self; 3] = [Self::Cpu, Self::Memory, Self::ExecutionTime];
}

/// Hardware archive service that manages periodic data archiving to database.
pub struct ArchiveService;

/// Statistics calculator for hardware metrics
struct StatsCalculator;

/// GPU metrics collector
struct GpuMetricsCollector<'a> {
  usage_histories: &'a Arc<Mutex<HashMap<String, VecDeque<f32>>>>,
  temperature_histories: &'a Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
  memory_histories: &'a Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
  name_map: &'a Arc<Mutex<HashMap<String, String>>>,
}

/// Process statistics collector and ranker
struct ProcessStatsCollector<'a> {
  cpu_histories: &'a ProcessHistory,
  memory_histories: &'a ProcessHistory,
}

impl ArchiveService {
  /// Deletes old archived data beyond the specified retention period.
  pub async fn cleanup_old_data(retention_days: u32) {
    if let Err(e) = database::hardware_archive::delete_old_data(retention_days).await {
      log_error!(
        "Failed to delete old hardware archive data",
        "cleanup_old_data",
        Some(e.to_string())
      );
    }

    if let Err(e) = database::gpu_archive::delete_old_data(retention_days).await {
      log_error!(
        "Failed to delete old GPU hardware archive data",
        "cleanup_old_data",
        Some(e.to_string())
      );
    }

    if let Err(e) = database::process_stats::delete_old_data(retention_days).await {
      log_error!(
        "Failed to delete old process stats data",
        "cleanup_old_data",
        Some(e.to_string())
      );
    }
  }

  /// Archives a single snapshot of current hardware data.
  pub async fn archive_current_snapshot(
    resources: &models::hardware_archive::MonitorResources,
  ) {
    let hardware_data = Self::collect_hardware_metrics(resources);
    let gpu_data = GpuMetricsCollector::new(
      &resources.gpu_usage_histories,
      &resources.gpu_temperature_histories,
      &resources.gpu_dedicated_memory_histories,
      &resources.gpu_name_map,
    )
    .collect_all();
    let process_stats = ProcessStatsCollector::new(
      &resources.process_cpu_histories,
      &resources.process_memory_histories,
    )
    .collect_and_rank();

    Self::persist_all_data(hardware_data.0, hardware_data.1, gpu_data, process_stats)
      .await;
  }

  /// Collects CPU and memory metrics
  fn collect_hardware_metrics(
    resources: &models::hardware_archive::MonitorResources,
  ) -> (
    models::hardware_archive::HardwareData,
    models::hardware_archive::HardwareData,
  ) {
    (
      StatsCalculator::calculate_hardware_stats(&resources.cpu_history),
      StatsCalculator::calculate_hardware_stats(&resources.memory_history),
    )
  }

  /// Persists all collected data to the database
  async fn persist_all_data(
    cpu_data: models::hardware_archive::HardwareData,
    memory_data: models::hardware_archive::HardwareData,
    gpu_data_list: Vec<models::hardware_archive::GpuData>,
    process_stats: Vec<models::hardware_archive::ProcessStatData>,
  ) {
    Self::persist_with_error_handling(
      database::hardware_archive::insert(cpu_data, memory_data),
      "hardware archive data",
    )
    .await;

    for gpu_data in gpu_data_list {
      Self::persist_with_error_handling(
        database::gpu_archive::insert(gpu_data),
        "GPU hardware archive data",
      )
      .await;
    }

    Self::persist_with_error_handling(
      database::process_stats::insert(process_stats),
      "process stats data",
    )
    .await;
  }

  /// Helper for database persistence with consistent error handling
  async fn persist_with_error_handling<T>(
    operation: impl std::future::Future<Output = Result<T, impl std::fmt::Display>>,
    data_type: &str,
  ) {
    if let Err(e) = operation.await {
      log_error!(
        format!("Failed to insert {}", data_type),
        "persist_with_error_handling",
        Some(e.to_string())
      );
    }
  }
}

impl StatsCalculator {
  /// Calculates average, min, and max values from a history buffer
  fn calculate_hardware_stats(
    history: &Arc<Mutex<VecDeque<f32>>>,
  ) -> models::hardware_archive::HardwareData {
    let values = Self::extract_values(history);
    Self::compute_stats(&values)
  }

  fn extract_values(history: &Arc<Mutex<VecDeque<f32>>>) -> Vec<f32> {
    history.lock().unwrap().iter().cloned().collect()
  }

  fn compute_stats(values: &[f32]) -> models::hardware_archive::HardwareData {
    if values.is_empty() {
      return models::hardware_archive::HardwareData {
        avg: None,
        max: None,
        min: None,
      };
    }

    let avg = Some(values.iter().sum::<f32>() / values.len() as f32);
    let max = values.iter().cloned().max_by(f32::total_cmp);
    let min = values.iter().cloned().min_by(f32::total_cmp);

    models::hardware_archive::HardwareData { avg, max, min }
  }

  fn compute_f32_aggregates(values: &[f32]) -> (Option<f32>, Option<f32>, Option<f32>) {
    if values.is_empty() {
      return (None, None, None);
    }

    let avg = Some(values.iter().sum::<f32>() / values.len() as f32);
    let max = values.iter().cloned().max_by(f32::total_cmp);
    let min = values.iter().cloned().min_by(f32::total_cmp);

    (avg, max, min)
  }

  fn compute_i32_aggregates(values: &[i32]) -> (Option<f32>, Option<i32>, Option<i32>) {
    if values.is_empty() {
      return (None, None, None);
    }

    let avg = Some(values.iter().sum::<i32>() as f32 / values.len() as f32);
    let max = values.iter().cloned().max();
    let min = values.iter().cloned().min();

    (avg, max, min)
  }
}

impl<'a> GpuMetricsCollector<'a> {
  fn new(
    usage_histories: &'a Arc<Mutex<HashMap<String, VecDeque<f32>>>>,
    temperature_histories: &'a Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
    memory_histories: &'a Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
    name_map: &'a Arc<Mutex<HashMap<String, String>>>,
  ) -> Self {
    Self {
      usage_histories,
      temperature_histories,
      memory_histories,
      name_map,
    }
  }

  fn collect_all(&self) -> Vec<models::hardware_archive::GpuData> {
    self
      .get_gpu_ids()
      .into_iter()
      .map(|gpu_id| self.collect_single_gpu_metrics(&gpu_id))
      .collect()
  }

  fn get_gpu_ids(&self) -> Vec<String> {
    self
      .usage_histories
      .lock()
      .unwrap()
      .keys()
      .cloned()
      .collect()
  }

  fn collect_single_gpu_metrics(
    &self,
    gpu_id: &str,
  ) -> models::hardware_archive::GpuData {
    let usage_stats = self.calculate_usage_stats(gpu_id);
    let temperature_stats = self.calculate_temperature_stats(gpu_id);
    let memory_stats = self.calculate_memory_stats(gpu_id);

    let gpu_name = self
      .name_map
      .lock()
      .unwrap()
      .get(gpu_id)
      .cloned()
      .unwrap_or_else(|| gpu_id.to_string());

    models::hardware_archive::GpuData {
      gpu_id: Some(gpu_id.to_string()),
      gpu_name,
      usage_avg: usage_stats.0,
      usage_max: usage_stats.1,
      usage_min: usage_stats.2,
      temperature_avg: temperature_stats.0,
      temperature_max: temperature_stats.1,
      temperature_min: temperature_stats.2,
      dedicated_memory_avg: memory_stats.0,
      dedicated_memory_max: memory_stats.1,
      dedicated_memory_min: memory_stats.2,
    }
  }

  fn calculate_usage_stats(
    &self,
    gpu_id: &str,
  ) -> (Option<f32>, Option<f32>, Option<f32>) {
    let values = self.get_f32_history_for_gpu(self.usage_histories, gpu_id);
    StatsCalculator::compute_f32_aggregates(&values)
  }

  fn calculate_temperature_stats(
    &self,
    gpu_id: &str,
  ) -> (Option<f32>, Option<i32>, Option<i32>) {
    let values = self.get_i32_history_for_gpu(self.temperature_histories, gpu_id);
    StatsCalculator::compute_i32_aggregates(&values)
  }

  fn calculate_memory_stats(
    &self,
    gpu_id: &str,
  ) -> (Option<i32>, Option<i32>, Option<i32>) {
    let values = self.get_i32_history_for_gpu(self.memory_histories, gpu_id);
    let (avg_f32, max, min) = StatsCalculator::compute_i32_aggregates(&values);
    (avg_f32.map(|v| v as i32), max, min)
  }

  fn get_f32_history_for_gpu(
    &self,
    histories: &Arc<Mutex<HashMap<String, VecDeque<f32>>>>,
    gpu_id: &str,
  ) -> Vec<f32> {
    histories
      .lock()
      .unwrap()
      .get(gpu_id)
      .map(|v| v.iter().cloned().collect())
      .unwrap_or_default()
  }

  fn get_i32_history_for_gpu(
    &self,
    histories: &Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
    gpu_id: &str,
  ) -> Vec<i32> {
    histories
      .lock()
      .unwrap()
      .get(gpu_id)
      .map(|v| v.iter().cloned().collect())
      .unwrap_or_default()
  }
}

impl<'a> ProcessStatsCollector<'a> {
  fn new(
    cpu_histories: &'a ProcessHistory,
    memory_histories: &'a ProcessHistory,
  ) -> Self {
    Self {
      cpu_histories,
      memory_histories,
    }
  }

  fn collect_and_rank(&self) -> Vec<models::hardware_archive::ProcessStatData> {
    let system_info = self.get_system_info();
    let all_stats = self.collect_all_process_stats(&system_info);
    self.rank_and_filter_processes(all_stats)
  }

  fn get_system_info(&self) -> (sysinfo::System, f32) {
    let mut sys = sysinfo::System::new();
    sys.refresh_all();
    let num_cores = sys.cpus().len() as f32;
    (sys, num_cores)
  }

  fn collect_all_process_stats(
    &self,
    (sys, num_cores): &(sysinfo::System, f32),
  ) -> Vec<models::hardware_archive::ProcessStatData> {
    let cpu_histories = self.cpu_histories.lock().unwrap();
    let mem_histories = self.memory_histories.lock().unwrap();

    cpu_histories
      .iter()
      .filter_map(|(pid, cpu_history)| {
        mem_histories.get(pid).and_then(|mem_history| {
          self.create_process_stat(*pid, cpu_history, mem_history, *num_cores, sys)
        })
      })
      .collect()
  }

  fn create_process_stat(
    &self,
    pid: sysinfo::Pid,
    cpu_history: &VecDeque<f32>,
    mem_history: &VecDeque<f32>,
    num_cores: f32,
    sys: &sysinfo::System,
  ) -> Option<models::hardware_archive::ProcessStatData> {
    let (cpu_avg, mem_avg) = self.calculate_process_averages(cpu_history, mem_history)?;

    if cpu_avg == 0.0 && mem_avg == 0.0 {
      return None;
    }

    let process = sys.process(pid)?;
    let exec_time = process.run_time() as i32;

    if !self.is_valid_execution_time(exec_time) {
      return None;
    }

    Some(models::hardware_archive::ProcessStatData {
      pid: pid.as_u32() as i32,
      process_name: process.name().to_string_lossy().into_owned(),
      cpu_usage: cpu_avg / num_cores,
      memory_usage: mem_avg.round() as i32,
      execution_sec: exec_time,
    })
  }

  fn calculate_process_averages(
    &self,
    cpu_history: &VecDeque<f32>,
    mem_history: &VecDeque<f32>,
  ) -> Option<(f32, f32)> {
    if cpu_history.is_empty() || mem_history.is_empty() {
      return None;
    }

    let cpu_avg = cpu_history.iter().sum::<f32>() / cpu_history.len() as f32;
    let mem_avg = mem_history.iter().sum::<f32>() / mem_history.len() as f32;

    Some((cpu_avg, mem_avg))
  }

  fn is_valid_execution_time(&self, exec_time: i32) -> bool {
    (0..=60 * 60 * 24 * 30).contains(&exec_time)
  }

  fn rank_and_filter_processes(
    &self,
    all_stats: Vec<models::hardware_archive::ProcessStatData>,
  ) -> Vec<models::hardware_archive::ProcessStatData> {
    let mut result = Vec::new();
    let mut seen_pids = HashSet::new();

    for &metric in &ProcessRankingMetric::ALL {
      let sorted_stats = self.sort_by_metric(all_stats.clone(), metric);
      self.add_top_processes(&mut result, &mut seen_pids, &sorted_stats);

      if result.len() >= PROCESS_RECORD_LIMIT * ProcessRankingMetric::ALL.len() {
        break;
      }
    }

    result
  }

  fn sort_by_metric(
    &self,
    mut stats: Vec<models::hardware_archive::ProcessStatData>,
    metric: ProcessRankingMetric,
  ) -> Vec<models::hardware_archive::ProcessStatData> {
    match metric {
      ProcessRankingMetric::Cpu => {
        stats.sort_by(|a, b| b.cpu_usage.total_cmp(&a.cpu_usage));
      }
      ProcessRankingMetric::Memory => {
        stats.sort_by_key(|s| std::cmp::Reverse(s.memory_usage));
      }
      ProcessRankingMetric::ExecutionTime => {
        stats.sort_by_key(|s| std::cmp::Reverse(s.execution_sec));
      }
    }
    stats
  }

  fn add_top_processes(
    &self,
    result: &mut Vec<models::hardware_archive::ProcessStatData>,
    seen_pids: &mut HashSet<i32>,
    sorted_stats: &[models::hardware_archive::ProcessStatData],
  ) {
    for stat in sorted_stats.iter().take(PROCESS_RECORD_LIMIT) {
      if seen_pids.insert(stat.pid) {
        result.push(stat.clone());
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use models::hardware_archive::ProcessStatData;

  // ── Helpers ──

  fn make_process_stat(pid: i32, cpu: f32, mem: i32, exec: i32) -> ProcessStatData {
    ProcessStatData {
      pid,
      process_name: format!("process_{pid}"),
      cpu_usage: cpu,
      memory_usage: mem,
      execution_sec: exec,
    }
  }

  fn dummy_process_collector() -> (ProcessHistory, ProcessHistory) {
    (
      Arc::new(Mutex::new(HashMap::new())),
      Arc::new(Mutex::new(HashMap::new())),
    )
  }

  // ── StatsCalculator::compute_stats ──

  #[test]
  fn compute_stats_empty_slice_returns_all_none() {
    let result = StatsCalculator::compute_stats(&[]);
    assert!(result.avg.is_none());
    assert!(result.max.is_none());
    assert!(result.min.is_none());
  }

  #[test]
  fn compute_stats_single_element() {
    let result = StatsCalculator::compute_stats(&[42.0]);
    assert_eq!(result.avg, Some(42.0));
    assert_eq!(result.max, Some(42.0));
    assert_eq!(result.min, Some(42.0));
  }

  #[test]
  fn compute_stats_multiple_elements() {
    let result = StatsCalculator::compute_stats(&[10.0, 20.0, 30.0]);
    assert_eq!(result.avg, Some(20.0));
    assert_eq!(result.max, Some(30.0));
    assert_eq!(result.min, Some(10.0));
  }

  #[test]
  fn compute_stats_identical_values() {
    let result = StatsCalculator::compute_stats(&[5.0, 5.0, 5.0]);
    assert_eq!(result.avg, Some(5.0));
    assert_eq!(result.max, Some(5.0));
    assert_eq!(result.min, Some(5.0));
  }

  #[test]
  fn compute_stats_with_negative_values() {
    let result = StatsCalculator::compute_stats(&[-10.0, 0.0, 10.0]);
    assert_eq!(result.avg, Some(0.0));
    assert_eq!(result.max, Some(10.0));
    assert_eq!(result.min, Some(-10.0));
  }

  #[test]
  fn compute_stats_large_dataset() {
    let values: Vec<f32> = (0..1000).map(|i| i as f32).collect();
    let result = StatsCalculator::compute_stats(&values);
    assert_eq!(result.avg, Some(499.5));
    assert_eq!(result.max, Some(999.0));
    assert_eq!(result.min, Some(0.0));
  }

  // ── StatsCalculator::compute_f32_aggregates ──

  #[test]
  fn compute_f32_aggregates_empty_returns_none_tuple() {
    assert_eq!(
      StatsCalculator::compute_f32_aggregates(&[]),
      (None, None, None)
    );
  }

  #[test]
  fn compute_f32_aggregates_single_value() {
    assert_eq!(
      StatsCalculator::compute_f32_aggregates(&[7.5]),
      (Some(7.5), Some(7.5), Some(7.5))
    );
  }

  #[test]
  fn compute_f32_aggregates_typical() {
    assert_eq!(
      StatsCalculator::compute_f32_aggregates(&[1.0, 2.0, 3.0, 4.0, 5.0]),
      (Some(3.0), Some(5.0), Some(1.0))
    );
  }

  // ── StatsCalculator::compute_i32_aggregates ──

  #[test]
  fn compute_i32_aggregates_empty_returns_none_tuple() {
    assert_eq!(
      StatsCalculator::compute_i32_aggregates(&[]),
      (None, None, None)
    );
  }

  #[test]
  fn compute_i32_aggregates_single_value() {
    assert_eq!(
      StatsCalculator::compute_i32_aggregates(&[50]),
      (Some(50.0), Some(50), Some(50))
    );
  }

  #[test]
  fn compute_i32_aggregates_typical() {
    assert_eq!(
      StatsCalculator::compute_i32_aggregates(&[10, 20, 30]),
      (Some(20.0), Some(30), Some(10))
    );
  }

  #[test]
  fn compute_i32_aggregates_avg_fractional() {
    let (avg, max, min) = StatsCalculator::compute_i32_aggregates(&[1, 2]);
    assert_eq!(avg, Some(1.5));
    assert_eq!(max, Some(2));
    assert_eq!(min, Some(1));
  }

  // ── ProcessStatsCollector::calculate_process_averages ──

  #[test]
  fn calculate_process_averages_both_empty_returns_none() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let result = collector.calculate_process_averages(&VecDeque::new(), &VecDeque::new());
    assert!(result.is_none());
  }

  #[test]
  fn calculate_process_averages_cpu_empty_returns_none() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let result =
      collector.calculate_process_averages(&VecDeque::new(), &VecDeque::from([1.0]));
    assert!(result.is_none());
  }

  #[test]
  fn calculate_process_averages_mem_empty_returns_none() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let result =
      collector.calculate_process_averages(&VecDeque::from([1.0]), &VecDeque::new());
    assert!(result.is_none());
  }

  #[test]
  fn calculate_process_averages_single_values() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let result = collector
      .calculate_process_averages(&VecDeque::from([50.0]), &VecDeque::from([1024.0]));
    assert_eq!(result, Some((50.0, 1024.0)));
  }

  #[test]
  fn calculate_process_averages_multiple_values() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let result = collector.calculate_process_averages(
      &VecDeque::from([10.0, 20.0, 30.0]),
      &VecDeque::from([100.0, 200.0, 300.0]),
    );
    assert_eq!(result, Some((20.0, 200.0)));
  }

  // ── ProcessStatsCollector::is_valid_execution_time ──

  #[test]
  fn is_valid_execution_time_zero() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    assert!(collector.is_valid_execution_time(0));
  }

  #[test]
  fn is_valid_execution_time_max_boundary() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    assert!(collector.is_valid_execution_time(60 * 60 * 24 * 30));
  }

  #[test]
  fn is_valid_execution_time_over_max() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    assert!(!collector.is_valid_execution_time(60 * 60 * 24 * 30 + 1));
  }

  #[test]
  fn is_valid_execution_time_negative() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    assert!(!collector.is_valid_execution_time(-1));
  }

  #[test]
  fn is_valid_execution_time_typical() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    assert!(collector.is_valid_execution_time(3600));
  }

  // ── ProcessStatsCollector::sort_by_metric ──

  #[test]
  fn sort_by_metric_cpu_descending() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let stats = vec![
      make_process_stat(1, 10.0, 100, 60),
      make_process_stat(2, 30.0, 200, 120),
      make_process_stat(3, 20.0, 300, 180),
    ];
    let sorted = collector.sort_by_metric(stats, ProcessRankingMetric::Cpu);
    assert_eq!(sorted[0].cpu_usage, 30.0);
    assert_eq!(sorted[1].cpu_usage, 20.0);
    assert_eq!(sorted[2].cpu_usage, 10.0);
  }

  #[test]
  fn sort_by_metric_memory_descending() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let stats = vec![
      make_process_stat(1, 10.0, 100, 60),
      make_process_stat(2, 20.0, 300, 120),
      make_process_stat(3, 30.0, 200, 180),
    ];
    let sorted = collector.sort_by_metric(stats, ProcessRankingMetric::Memory);
    assert_eq!(sorted[0].memory_usage, 300);
    assert_eq!(sorted[1].memory_usage, 200);
    assert_eq!(sorted[2].memory_usage, 100);
  }

  #[test]
  fn sort_by_metric_execution_time_descending() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let stats = vec![
      make_process_stat(1, 10.0, 100, 60),
      make_process_stat(2, 20.0, 200, 180),
      make_process_stat(3, 30.0, 300, 120),
    ];
    let sorted = collector.sort_by_metric(stats, ProcessRankingMetric::ExecutionTime);
    assert_eq!(sorted[0].execution_sec, 180);
    assert_eq!(sorted[1].execution_sec, 120);
    assert_eq!(sorted[2].execution_sec, 60);
  }

  #[test]
  fn sort_by_metric_empty_vec() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let sorted = collector.sort_by_metric(vec![], ProcessRankingMetric::Cpu);
    assert!(sorted.is_empty());
  }

  // ── ProcessStatsCollector::rank_and_filter_processes ──

  #[test]
  fn rank_and_filter_empty_input() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let result = collector.rank_and_filter_processes(vec![]);
    assert!(result.is_empty());
  }

  #[test]
  fn rank_and_filter_fewer_than_limit() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let stats = vec![
      make_process_stat(1, 10.0, 100, 60),
      make_process_stat(2, 20.0, 200, 120),
      make_process_stat(3, 30.0, 300, 180),
    ];
    let result = collector.rank_and_filter_processes(stats);
    // All 3 unique pids should appear (deduped across metric sorts)
    let pids: HashSet<i32> = result.iter().map(|s| s.pid).collect();
    assert_eq!(pids.len(), 3);
    assert!(pids.contains(&1));
    assert!(pids.contains(&2));
    assert!(pids.contains(&3));
  }

  #[test]
  fn rank_and_filter_deduplication() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    // Process 1 is top in all metrics — should appear only once
    let stats = vec![
      make_process_stat(1, 99.0, 999, 9999),
      make_process_stat(2, 1.0, 1, 1),
    ];
    let result = collector.rank_and_filter_processes(stats);
    let pid_1_count = result.iter().filter(|s| s.pid == 1).count();
    assert_eq!(pid_1_count, 1);
  }

  #[test]
  fn rank_and_filter_max_limit() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    // Create 20 unique processes
    let stats: Vec<ProcessStatData> = (0..20)
      .map(|i| make_process_stat(i, i as f32, i * 10, i * 60))
      .collect();
    let result = collector.rank_and_filter_processes(stats);
    // Should be capped at PROCESS_RECORD_LIMIT * 3 = 15
    assert!(result.len() <= PROCESS_RECORD_LIMIT * ProcessRankingMetric::ALL.len());
  }

  // ── ProcessStatsCollector::add_top_processes ──

  #[test]
  fn add_top_processes_respects_limit() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let sorted: Vec<ProcessStatData> =
      (0..10).map(|i| make_process_stat(i, 0.0, 0, 0)).collect();
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    collector.add_top_processes(&mut result, &mut seen, &sorted);
    assert_eq!(result.len(), PROCESS_RECORD_LIMIT);
  }

  #[test]
  fn add_top_processes_skips_seen_pids() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let sorted: Vec<ProcessStatData> =
      (0..5).map(|i| make_process_stat(i, 0.0, 0, 0)).collect();
    let mut result = Vec::new();
    let mut seen = HashSet::from([0, 1]);
    collector.add_top_processes(&mut result, &mut seen, &sorted);
    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|s| s.pid != 0 && s.pid != 1));
  }

  #[test]
  fn add_top_processes_empty_input() {
    let (cpu_h, mem_h) = dummy_process_collector();
    let collector = ProcessStatsCollector::new(&cpu_h, &mem_h);
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    collector.add_top_processes(&mut result, &mut seen, &[]);
    assert!(result.is_empty());
  }

  // ── GpuMetricsCollector ──

  #[test]
  fn gpu_metrics_collector_empty_histories() {
    let usage = Arc::new(Mutex::new(HashMap::new()));
    let temp = Arc::new(Mutex::new(HashMap::new()));
    let mem = Arc::new(Mutex::new(HashMap::new()));
    let names = Arc::new(Mutex::new(HashMap::new()));
    let collector = GpuMetricsCollector::new(&usage, &temp, &mem, &names);
    assert!(collector.collect_all().is_empty());
  }

  #[test]
  fn gpu_metrics_collector_single_gpu() {
    let mut usage_map: HashMap<String, VecDeque<f32>> = HashMap::new();
    usage_map.insert("gpu:0".to_string(), VecDeque::from([50.0, 60.0]));
    let mut temp_map: HashMap<String, VecDeque<i32>> = HashMap::new();
    temp_map.insert("gpu:0".to_string(), VecDeque::from([70, 80]));
    let mut mem_map: HashMap<String, VecDeque<i32>> = HashMap::new();
    mem_map.insert("gpu:0".to_string(), VecDeque::from([1000, 2000]));
    let mut name_map: HashMap<String, String> = HashMap::new();
    name_map.insert("gpu:0".to_string(), "TestGPU".to_string());

    let usage = Arc::new(Mutex::new(usage_map));
    let temp = Arc::new(Mutex::new(temp_map));
    let mem = Arc::new(Mutex::new(mem_map));
    let names = Arc::new(Mutex::new(name_map));
    let collector = GpuMetricsCollector::new(&usage, &temp, &mem, &names);
    let result = collector.collect_all();

    assert_eq!(result.len(), 1);
    let gpu = &result[0];
    assert_eq!(gpu.gpu_id, Some("gpu:0".to_string()));
    assert_eq!(gpu.gpu_name, "TestGPU");
    assert_eq!(gpu.usage_avg, Some(55.0));
    assert_eq!(gpu.usage_max, Some(60.0));
    assert_eq!(gpu.usage_min, Some(50.0));
    assert_eq!(gpu.temperature_avg, Some(75.0));
    assert_eq!(gpu.temperature_max, Some(80));
    assert_eq!(gpu.temperature_min, Some(70));
    assert_eq!(gpu.dedicated_memory_avg, Some(1500));
    assert_eq!(gpu.dedicated_memory_max, Some(2000));
    assert_eq!(gpu.dedicated_memory_min, Some(1000));
  }

  #[test]
  fn gpu_metrics_collector_multiple_gpus() {
    let mut usage_map: HashMap<String, VecDeque<f32>> = HashMap::new();
    usage_map.insert("pci:0:2.0".to_string(), VecDeque::from([10.0]));
    usage_map.insert("pci:0:3.0".to_string(), VecDeque::from([90.0]));
    let temp_map: HashMap<String, VecDeque<i32>> = HashMap::new();
    let mem_map: HashMap<String, VecDeque<i32>> = HashMap::new();
    let mut name_map: HashMap<String, String> = HashMap::new();
    name_map.insert("pci:0:2.0".to_string(), "GPU_A".to_string());
    name_map.insert("pci:0:3.0".to_string(), "GPU_B".to_string());

    let usage = Arc::new(Mutex::new(usage_map));
    let temp = Arc::new(Mutex::new(temp_map));
    let mem = Arc::new(Mutex::new(mem_map));
    let names = Arc::new(Mutex::new(name_map));
    let collector = GpuMetricsCollector::new(&usage, &temp, &mem, &names);
    let result = collector.collect_all();

    assert_eq!(result.len(), 2);
    let gpu_names: HashSet<&str> = result.iter().map(|g| g.gpu_name.as_str()).collect();
    assert!(gpu_names.contains("GPU_A"));
    assert!(gpu_names.contains("GPU_B"));
    let gpu_ids: HashSet<&str> = result
      .iter()
      .map(|g| g.gpu_id.as_deref().unwrap())
      .collect();
    assert!(gpu_ids.contains("pci:0:2.0"));
    assert!(gpu_ids.contains("pci:0:3.0"));
  }

  #[test]
  fn gpu_metrics_collector_name_map_fallback() {
    let mut usage_map: HashMap<String, VecDeque<f32>> = HashMap::new();
    usage_map.insert("unknown:0".to_string(), VecDeque::from([50.0]));

    let usage = Arc::new(Mutex::new(usage_map));
    let temp = Arc::new(Mutex::new(HashMap::new()));
    let mem = Arc::new(Mutex::new(HashMap::new()));
    let names = Arc::new(Mutex::new(HashMap::new())); // empty name map
    let collector = GpuMetricsCollector::new(&usage, &temp, &mem, &names);
    let result = collector.collect_all();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].gpu_name, "unknown:0"); // falls back to gpu_id
  }

  // ── StatsCalculator::calculate_hardware_stats (integration with Arc<Mutex<VecDeque>>) ──

  #[test]
  fn calculate_hardware_stats_empty_history() {
    let history = Arc::new(Mutex::new(VecDeque::new()));
    let result = StatsCalculator::calculate_hardware_stats(&history);
    assert!(result.avg.is_none());
    assert!(result.max.is_none());
    assert!(result.min.is_none());
  }

  #[test]
  fn calculate_hardware_stats_with_data() {
    let history = Arc::new(Mutex::new(VecDeque::from([25.0, 50.0, 75.0])));
    let result = StatsCalculator::calculate_hardware_stats(&history);
    assert_eq!(result.avg, Some(50.0));
    assert_eq!(result.max, Some(75.0));
    assert_eq!(result.min, Some(25.0));
  }
}
