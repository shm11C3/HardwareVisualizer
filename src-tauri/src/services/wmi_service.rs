use crate::{log_debug, log_error, log_info, log_internal};

use regex::Regex;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use wmi::{COMLibrary, WMIConnection};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct GpuEngineLoadInfo {
  name: String,
  utilization_percentage: Option<u16>,
}

///
/// 指定したGPUエンジンの使用率を取得する（WMIを使用）
///
pub async fn get_gpu_usage_by_device_and_engine(
  engine_type: &str,
) -> Result<f32, Box<dyn Error>> {
  // GPUエンジン情報を取得
  let results: Vec<GpuEngineLoadInfo>  = wmi_query_in_thread(
      "SELECT Name, UtilizationPercentage FROM Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine".to_string(),
  )?;

  log_info!(
    &format!("GPU engine usage data: {:?}", results),
    "get_gpu_usage_by_device_and_engine",
    None::<&str>
  );

  // 正規表現で `engtype_xxx` の部分を抽出
  let re = Regex::new(r"engtype_(\w+)").unwrap();

  for engine in results.iter() {
    if let Some(captures) = re.captures(&engine.name) {
      if let Some(engine_name) = captures.get(1) {
        if engine_name.as_str() == engine_type {
          if let Some(load) = engine.utilization_percentage {
            return Ok(load as f32 / 100.0);
          } else {
            return Err(Box::new(std::io::Error::new(
              std::io::ErrorKind::NotFound,
              format!("No usage data available for engine type: {}", engine_type),
            )));
          }
        }
      }
    }
  }

  Err(Box::new(std::io::Error::new(
    std::io::ErrorKind::NotFound,
    format!("No GPU engine found: engine_type: {}", engine_type,),
  )))
}

///
/// ## WMIから別スレッドで￥クエリ実行する（WMIを使用）
///
fn wmi_query_in_thread<T>(query: String) -> Result<Vec<T>, String>
where
  T: DeserializeOwned + std::fmt::Debug + Send + 'static,
{
  let (tx, rx): (
    Sender<Result<Vec<T>, String>>,
    Receiver<Result<Vec<T>, String>>,
  ) = channel();

  // 別スレッドを起動してWMIクエリを実行
  thread::spawn(move || {
    let result = (|| {
      let com_con = COMLibrary::new()
        .map_err(|e| format!("Failed to initialize COM Library: {:?}", e))?;
      let wmi_con = WMIConnection::new(com_con)
        .map_err(|e| format!("Failed to create WMI connection: {:?}", e))?;

      // WMIクエリを実行してメモリ情報を取得
      let results: Vec<T> = wmi_con
        .raw_query(query)
        .map_err(|e| format!("Failed to execute query: {:?}", e))?;

      Ok(results)
    })();

    // メインスレッドに結果を送信
    if let Err(err) = tx.send(result) {
      log_error!(
        "Failed to send data from thread",
        "get_wmi_data_in_thread",
        Some(err.to_string())
      );
    }
  });

  // メインスレッドで結果を受信
  rx.recv()
    .map_err(|_| "Failed to receive data from thread".to_string())?
}
