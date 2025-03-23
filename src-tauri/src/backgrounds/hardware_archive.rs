use crate::{database, structs};
use crate::{log_error, log_internal};
use std::{
  collections::{HashMap, VecDeque},
  sync::{Arc, Mutex},
  time::Duration,
};

const HISTORY_CAPACITY: u64 = 60;

pub async fn setup(
  cpu_history: Arc<Mutex<VecDeque<f32>>>,
  memory_history: Arc<Mutex<VecDeque<f32>>>,
  nv_gpu_usage_histories: Arc<Mutex<HashMap<String, VecDeque<f32>>>>,
  nv_gpu_temperature_histories: Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
) {
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
}

fn get_cpu_data(
  cpu_history: Arc<Mutex<VecDeque<f32>>>,
) -> structs::hardware_archive::HardwareData {
  let cpu_avg = {
    let cpu_hist = cpu_history.lock().unwrap();
    if cpu_hist.is_empty() {
      0.0
    } else {
      cpu_hist.iter().sum::<f32>() / cpu_hist.len() as f32
    }
  };

  let cpu_max = cpu_history
    .lock()
    .unwrap()
    .iter()
    .cloned()
    .fold(0.0, f32::max);

  let cpu_min = cpu_history
    .lock()
    .unwrap()
    .iter()
    .cloned()
    .fold(100.0, f32::min);

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
      0.0
    } else {
      mem_hist.iter().sum::<f32>() / mem_hist.len() as f32
    }
  };

  let memory_max = memory_history
    .lock()
    .unwrap()
    .iter()
    .cloned()
    .fold(0.0, f32::max);

  let memory_min = memory_history
    .lock()
    .unwrap()
    .iter()
    .cloned()
    .fold(100.0, f32::min);

  structs::hardware_archive::HardwareData {
    avg: memory_avg,
    max: memory_max,
    min: memory_min,
  }
}

fn get_gpu_data(
  nv_gpu_usage_histories: Arc<Mutex<HashMap<String, VecDeque<f32>>>>,
  nv_gpu_temperature_histories: Arc<Mutex<HashMap<String, VecDeque<i32>>>>,
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
        0.0
      } else {
        usage_hist.values().flat_map(|v| v.iter()).sum::<f32>()
          / usage_hist.values().map(|v| v.len()).sum::<usize>() as f32
      }
    };

    let usage_max = nv_gpu_usage_histories
      .lock()
      .unwrap()
      .values()
      .flat_map(|v| v.iter())
      .cloned()
      .fold(0.0, f32::max);

    let usage_min = nv_gpu_usage_histories
      .lock()
      .unwrap()
      .values()
      .flat_map(|v| v.iter())
      .cloned()
      .fold(100.0, f32::min);

    let temperature_avg = {
      let temperature_hist = nv_gpu_temperature_histories.lock().unwrap();
      if temperature_hist.is_empty() {
        0.0
      } else {
        temperature_hist
          .values()
          .flat_map(|v| v.iter())
          .sum::<i32>() as f32
          / temperature_hist.values().map(|v| v.len()).sum::<usize>() as f32
      }
    };

    let temperature_max = nv_gpu_temperature_histories
      .lock()
      .unwrap()
      .values()
      .flat_map(|v| v.iter())
      .cloned()
      .fold(i32::MIN, i32::max);

    let temperature_min = nv_gpu_temperature_histories
      .lock()
      .unwrap()
      .values()
      .flat_map(|v| v.iter())
      .cloned()
      .fold(100, i32::min);

    gpu_data.push(structs::hardware_archive::GpuData {
      gpu_name,
      usage_avg,
      usage_max,
      usage_min,
      temperature_avg,
      temperature_max,
      temperature_min,
    });
  }

  gpu_data
}
