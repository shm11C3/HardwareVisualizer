use crate::{database, structs};
use crate::{log_error, log_internal};
use std::{
  collections::{HashMap, VecDeque},
  sync::{Arc, Mutex},
  time::Duration,
};

const HISTORY_CAPACITY: u64 = 60;

///
/// プロセスアーカイブデータの保存する上限数
///
/// `PROCESS_RECORD_LIMIT`` * `PROCESS_RECORD_GROUP` の数だけ保存する
///
const PROCESS_RECORD_LIMIT: usize = 5;

const PROCESS_RECORD_GROUP: [&str; 3] = ["cpu", "memory", "exec"];

pub async fn setup(resources: structs::hardware_archive::MonitorResources) {
  let structs::hardware_archive::MonitorResources {
    system: _system,
    cpu_history,
    memory_history,
    process_cpu_histories,
    process_memory_histories,
    nv_gpu_usage_histories,
    nv_gpu_temperature_histories,
    nv_gpu_dedicated_memory_histories,
  } = resources;

  let mut interval = tokio::time::interval(Duration::from_secs(HISTORY_CAPACITY));

  loop {
    interval.tick().await;

    // 現在時刻と平均値を用いてアーカイブデータを生成
    let cpu_data = get_cpu_data(cpu_history.clone());
    let memory_data = get_memory_data(memory_history.clone());

    let result = database::hardware_archive::insert(cpu_data, memory_data).await;
    if let Err(e) = result {
      log_error!(
        "Failed to insert hardware archive data",
        "start_hardware_archive_service",
        Some(e.to_string())
      );
    }

    let gpu_data_list = get_gpu_data(
      nv_gpu_usage_histories.clone(),
      nv_gpu_temperature_histories.clone(),
      nv_gpu_dedicated_memory_histories.clone(),
    );

    for gpu_data in gpu_data_list {
      let result = database::gpu_archive::insert(gpu_data).await;
      if let Err(e) = result {
        log_error!(
          "Failed to insert GPU hardware archive data",
          "start_hardware_archive_service",
          Some(e.to_string())
        );
      }
    }

    let process_stats = get_process_stats(
      process_cpu_histories.clone(),
      process_memory_histories.clone(),
    );
    let result = database::process_stats::insert(process_stats).await;
    if let Err(e) = result {
      log_error!(
        "Failed to insert process stats data",
        "start_hardware_archive_service",
        Some(e.to_string())
      );
    }
  }
}

///
/// 指定された日数より古いデータを削除する
///
pub async fn batch_delete_old_data(refresh_interval_days: u32) {
  let deletion_result = tokio::runtime::Handle::current().block_on(
    database::hardware_archive::delete_old_data(refresh_interval_days),
  );

  if let Err(e) = deletion_result {
    log_error!(
      "Failed to delete old hardware archive data",
      "batch_delete_old_data",
      Some(e.to_string())
    );
  }

  let deletion_result = tokio::runtime::Handle::current().block_on(
    database::gpu_archive::delete_old_data(refresh_interval_days),
  );

  if let Err(e) = deletion_result {
    log_error!(
      "Failed to delete old GPU hardware archive data",
      "batch_delete_old_data",
      Some(e.to_string())
    );
  }

  let deletion_result =
    database::process_stats::delete_old_data(refresh_interval_days).await;
  if let Err(e) = deletion_result {
    log_error!(
      "Failed to delete old process stats data",
      "batch_delete_old_data",
      Some(e.to_string())
    );
  }
}

fn get_cpu_data(
  cpu_history: Arc<Mutex<VecDeque<f32>>>,
) -> structs::hardware_archive::HardwareData {
  let cpu_avg = {
    let cpu_hist = cpu_history.lock().unwrap();
    if cpu_hist.is_empty() {
      None
    } else {
      Some(cpu_hist.iter().sum::<f32>() / cpu_hist.len() as f32)
    }
  };

  let cpu_max = cpu_history
    .lock()
    .unwrap()
    .iter()
    .cloned()
    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

  let cpu_min = cpu_history
    .lock()
    .unwrap()
    .iter()
    .cloned()
    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

  structs::hardware_archive::HardwareData {
    avg: cpu_avg,
    max: cpu_max,
    min: cpu_min,
  }
}

fn get_memory_data(
  memory_history: Arc<Mutex<VecDeque<f32>>>,
) -> structs::hardware_archive::HardwareData {
  let memory_avg = {
    let mem_hist = memory_history.lock().unwrap();
    if mem_hist.is_empty() {
      None
    } else {
      Some(mem_hist.iter().sum::<f32>() / mem_hist.len() as f32)
    }
  };

  let memory_max = memory_history
    .lock()
    .unwrap()
    .iter()
    .cloned()
    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

  let memory_min = memory_history
    .lock()
    .unwrap()
    .iter()
    .cloned()
    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

  structs::hardware_archive::HardwareData {
    avg: memory_avg,
    max: memory_max,
    min: memory_min,
  }
}

fn get_gpu_data(
  nv_gpu_usage_histories: Arc<Mutex<HashMap<String, VecDeque<f32>>>>,
  nv_gpu_temperature_histories: Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
  nv_gpu_dedicated_memory_histories: Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
) -> Vec<structs::hardware_archive::GpuData> {
  let gpu_names = nv_gpu_usage_histories
    .lock()
    .unwrap()
    .keys()
    .cloned()
    .collect::<Vec<String>>();

  let mut gpu_data: Vec<structs::hardware_archive::GpuData> = Vec::new();

  for gpu_name in gpu_names {
    let usage_avg = {
      let usage_hist = nv_gpu_usage_histories.lock().unwrap();
      if usage_hist.is_empty() {
        None
      } else {
        Some(
          usage_hist.values().flat_map(|v| v.iter()).sum::<f32>()
            / usage_hist.values().map(|v| v.len()).sum::<usize>() as f32,
        )
      }
    };

    let usage_max = nv_gpu_usage_histories
      .lock()
      .unwrap()
      .values()
      .flat_map(|v| v.iter())
      .cloned()
      .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let usage_min = nv_gpu_usage_histories
      .lock()
      .unwrap()
      .values()
      .flat_map(|v| v.iter())
      .cloned()
      .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let temperature_avg = {
      let temperature_hist = nv_gpu_temperature_histories.lock().unwrap();
      if temperature_hist.is_empty() {
        None
      } else {
        Some(
          temperature_hist
            .values()
            .flat_map(|v| v.iter())
            .sum::<i32>() as f32
            / temperature_hist.values().map(|v| v.len()).sum::<usize>() as f32,
        )
      }
    };

    let temperature_max = nv_gpu_temperature_histories
      .lock()
      .unwrap()
      .values()
      .flat_map(|v| v.iter())
      .cloned()
      .max();

    let temperature_min = nv_gpu_temperature_histories
      .lock()
      .unwrap()
      .values()
      .flat_map(|v| v.iter())
      .cloned()
      .min();

    let dedicated_memory_avg = {
      let memory_hist = nv_gpu_dedicated_memory_histories.lock().unwrap();
      if memory_hist.is_empty() {
        None
      } else {
        Some(
          memory_hist.values().flat_map(|v| v.iter()).sum::<i32>()
            / memory_hist.values().map(|v| v.len()).sum::<usize>() as i32,
        )
      }
    };

    let dedicated_memory_max = nv_gpu_dedicated_memory_histories
      .lock()
      .unwrap()
      .values()
      .flat_map(|v| v.iter())
      .cloned()
      .max();

    let dedicated_memory_min = nv_gpu_dedicated_memory_histories
      .lock()
      .unwrap()
      .values()
      .flat_map(|v| v.iter())
      .cloned()
      .min();

    gpu_data.push(structs::hardware_archive::GpuData {
      gpu_name,
      usage_avg,
      usage_max,
      usage_min,
      temperature_avg,
      temperature_max,
      temperature_min,
      dedicated_memory_avg,
      dedicated_memory_max,
      dedicated_memory_min,
    });
  }

  gpu_data
}

fn get_process_stats(
  process_cpu_histories: Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
  process_memory_histories: Arc<Mutex<HashMap<sysinfo::Pid, VecDeque<f32>>>>,
) -> Vec<structs::hardware_archive::ProcessStatData> {
  let mut sys = sysinfo::System::new();
  sys.refresh_all();

  let num_cores = sys.cpus().len() as f32;

  let cpu_histories = process_cpu_histories.lock().unwrap();
  let mem_histories = process_memory_histories.lock().unwrap();

  let mut all_stats: Vec<structs::hardware_archive::ProcessStatData> = Vec::new();

  for (pid, cpu_history) in cpu_histories.iter() {
    if let Some(mem_history) = mem_histories.get(pid) {
      if !cpu_history.is_empty() && !mem_history.is_empty() {
        let cpu_avg = cpu_history.iter().copied().sum::<f32>() / cpu_history.len() as f32;
        let cpu_normalized_avg = cpu_avg / num_cores;

        let mem_avg = mem_history.iter().copied().sum::<f32>() / mem_history.len() as f32;

        // CPU とメモリの使用率が両方とも 0 の場合はスキップ
        if cpu_avg == 0.0 && mem_avg == 0.0 {
          continue;
        }

        if let Some(process) = sys.process(*pid) {
          let exec_time = process.run_time() as i32;

          // 異常な稼働時間のプロセスは無視
          if !(0..=60 * 60 * 24 * 30).contains(&exec_time) {
            continue;
          }

          all_stats.push(structs::hardware_archive::ProcessStatData {
            pid: (*pid).as_u32() as i32,
            process_name: process.name().to_string_lossy().into_owned(),
            cpu_usage: cpu_normalized_avg,
            memory_usage: mem_avg.round() as i32,
            execution_sec: exec_time,
          });
        }
      }
    }
  }

  // 上位25件 × 3種（重複排除）
  let mut top_stats = Vec::new();
  let mut seen_pids = std::collections::HashSet::new();

  for key in ["cpu", "memory", "exec"] {
    let mut sorted = all_stats.clone();
    match key {
      "cpu" => sorted.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap()),
      "memory" => sorted.sort_by(|a, b| b.memory_usage.cmp(&a.memory_usage)),
      "exec" => sorted.sort_by(|a, b| b.execution_sec.cmp(&a.execution_sec)),
      _ => {}
    }
    for stat in sorted.into_iter().take(PROCESS_RECORD_LIMIT) {
      if seen_pids.insert(stat.pid) {
        top_stats.push(stat);
      }
      if top_stats.len() >= PROCESS_RECORD_LIMIT * PROCESS_RECORD_GROUP.len() {
        break;
      }
    }
    if top_stats.len() >= PROCESS_RECORD_LIMIT * PROCESS_RECORD_GROUP.len() {
      break;
    }
  }

  top_stats
}
