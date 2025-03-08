use crate::{database::hardware_archive, structs};
use crate::{log_error, log_internal};
use std::{
  collections::VecDeque,
  sync::{Arc, Mutex},
  thread,
  time::Duration,
};

const HISTORY_CAPACITY: usize = 60;

pub async fn start_hardware_archive_service(
  cpu_history: Arc<Mutex<VecDeque<f32>>>,
  memory_history: Arc<Mutex<VecDeque<f32>>>,
) {
  tokio::spawn(async move {
    loop {
      // 1分待機
      thread::sleep(Duration::from_secs(HISTORY_CAPACITY as u64));

      // 現在時刻と平均値を用いてアーカイブデータを生成
      let cpu_data = get_cpu_data(cpu_history.clone());
      let memory_data = get_memory_data(memory_history.clone());

      let result = hardware_archive::insert(cpu_data, memory_data).await;
      if let Err(e) = result {
        log_error!(
          "Failed to insert hardware archive data",
          "start_hardware_archive_service",
          Some(e.to_string())
        );
      }
    }
  });
}

///
/// 指定された日数より古いデータを削除する
///
pub async fn batch_delete_old_data(refresh_interval_days: u32) {
  tokio::task::spawn_blocking(move || {
    let deletion_result = tokio::runtime::Handle::current()
      .block_on(hardware_archive::delete_old_data(refresh_interval_days));

    if let Err(e) = deletion_result {
      log_error!(
        "Failed to delete old hardware archive data",
        "batch_delete_old_data",
        Some(e.to_string())
      );
    }
  });
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
