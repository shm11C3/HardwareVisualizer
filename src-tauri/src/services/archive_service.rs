use crate::{database, log_error, log_internal, structs};
use std::{
  collections::{HashMap, HashSet, VecDeque},
  sync::{Arc, Mutex},
};

const PROCESS_RECORD_LIMIT: usize = 5;

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
}

/// Process statistics collector and ranker
struct ProcessStatsCollector<'a> {
  cpu_histories: &'a Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  memory_histories: &'a Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
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
    resources: &structs::hardware_archive::MonitorResources,
  ) {
    let hardware_data = Self::collect_hardware_metrics(resources);
    let gpu_data = GpuMetricsCollector::new(
      &resources.nv_gpu_usage_histories,
      &resources.nv_gpu_temperature_histories,
      &resources.nv_gpu_dedicated_memory_histories,
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
    resources: &structs::hardware_archive::MonitorResources,
  ) -> (
    structs::hardware_archive::HardwareData,
    structs::hardware_archive::HardwareData,
  ) {
    (
      StatsCalculator::calculate_hardware_stats(&resources.cpu_history),
      StatsCalculator::calculate_hardware_stats(&resources.memory_history),
    )
  }

  /// Persists all collected data to the database
  async fn persist_all_data(
    cpu_data: structs::hardware_archive::HardwareData,
    memory_data: structs::hardware_archive::HardwareData,
    gpu_data_list: Vec<structs::hardware_archive::GpuData>,
    process_stats: Vec<structs::hardware_archive::ProcessStatData>,
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
  ) -> structs::hardware_archive::HardwareData {
    let values = Self::extract_values(history);
    Self::compute_stats(&values)
  }

  fn extract_values(history: &Arc<Mutex<VecDeque<f32>>>) -> Vec<f32> {
    history.lock().unwrap().iter().cloned().collect()
  }

  fn compute_stats(values: &[f32]) -> structs::hardware_archive::HardwareData {
    if values.is_empty() {
      return structs::hardware_archive::HardwareData {
        avg: None,
        max: None,
        min: None,
      };
    }

    let avg = Some(values.iter().sum::<f32>() / values.len() as f32);
    let max = values.iter().cloned().max_by(f32::total_cmp);
    let min = values.iter().cloned().min_by(f32::total_cmp);

    structs::hardware_archive::HardwareData { avg, max, min }
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
  ) -> Self {
    Self {
      usage_histories,
      temperature_histories,
      memory_histories,
    }
  }

  fn collect_all(&self) -> Vec<structs::hardware_archive::GpuData> {
    self
      .get_gpu_names()
      .into_iter()
      .map(|name| self.collect_single_gpu_metrics(&name))
      .collect()
  }

  fn get_gpu_names(&self) -> Vec<String> {
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
    gpu_name: &str,
  ) -> structs::hardware_archive::GpuData {
    let usage_stats = self.calculate_usage_stats();
    let temperature_stats = self.calculate_temperature_stats();
    let memory_stats = self.calculate_memory_stats();

    structs::hardware_archive::GpuData {
      gpu_name: gpu_name.to_string(),
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

  fn calculate_usage_stats(&self) -> (Option<f32>, Option<f32>, Option<f32>) {
    let values = self.flatten_f32_histories(self.usage_histories);
    StatsCalculator::compute_f32_aggregates(&values)
  }

  fn calculate_temperature_stats(&self) -> (Option<f32>, Option<i32>, Option<i32>) {
    let values = self.flatten_i32_histories(self.temperature_histories);
    StatsCalculator::compute_i32_aggregates(&values)
  }

  fn calculate_memory_stats(&self) -> (Option<i32>, Option<i32>, Option<i32>) {
    let values = self.flatten_i32_histories(self.memory_histories);
    let (avg_f32, max, min) = StatsCalculator::compute_i32_aggregates(&values);
    (avg_f32.map(|v| v as i32), max, min)
  }

  fn flatten_f32_histories(
    &self,
    histories: &Arc<Mutex<HashMap<String, VecDeque<f32>>>>,
  ) -> Vec<f32> {
    histories
      .lock()
      .unwrap()
      .values()
      .flat_map(|v| v.iter())
      .cloned()
      .collect()
  }

  fn flatten_i32_histories(
    &self,
    histories: &Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
  ) -> Vec<i32> {
    histories
      .lock()
      .unwrap()
      .values()
      .flat_map(|v| v.iter())
      .cloned()
      .collect()
  }
}

impl<'a> ProcessStatsCollector<'a> {
  fn new(
    cpu_histories: &'a Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
    memory_histories: &'a Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  ) -> Self {
    Self {
      cpu_histories,
      memory_histories,
    }
  }

  fn collect_and_rank(&self) -> Vec<structs::hardware_archive::ProcessStatData> {
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
  ) -> Vec<structs::hardware_archive::ProcessStatData> {
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
  ) -> Option<structs::hardware_archive::ProcessStatData> {
    let (cpu_avg, mem_avg) = self.calculate_process_averages(cpu_history, mem_history)?;

    if cpu_avg == 0.0 && mem_avg == 0.0 {
      return None;
    }

    let process = sys.process(pid)?;
    let exec_time = process.run_time() as i32;

    if !self.is_valid_execution_time(exec_time) {
      return None;
    }

    Some(structs::hardware_archive::ProcessStatData {
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
    all_stats: Vec<structs::hardware_archive::ProcessStatData>,
  ) -> Vec<structs::hardware_archive::ProcessStatData> {
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
    mut stats: Vec<structs::hardware_archive::ProcessStatData>,
    metric: ProcessRankingMetric,
  ) -> Vec<structs::hardware_archive::ProcessStatData> {
    match metric {
      ProcessRankingMetric::Cpu => {
        stats.sort_by(|a, b| b.cpu_usage.total_cmp(&a.cpu_usage));
      }
      ProcessRankingMetric::Memory => {
        stats.sort_by(|a, b| b.memory_usage.cmp(&a.memory_usage));
      }
      ProcessRankingMetric::ExecutionTime => {
        stats.sort_by(|a, b| b.execution_sec.cmp(&a.execution_sec));
      }
    }
    stats
  }

  fn add_top_processes(
    &self,
    result: &mut Vec<structs::hardware_archive::ProcessStatData>,
    seen_pids: &mut HashSet<i32>,
    sorted_stats: &[structs::hardware_archive::ProcessStatData],
  ) {
    for stat in sorted_stats.iter().take(PROCESS_RECORD_LIMIT) {
      if seen_pids.insert(stat.pid) {
        result.push(stat.clone());
      }
    }
  }
}
