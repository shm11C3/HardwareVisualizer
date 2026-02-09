use crate::infrastructure::providers::windows::wmi_provider;
use crate::models::hardware::MotherboardInfo;
use std::future::Future;
use std::pin::Pin;

pub fn get_motherboard_info()
-> Pin<Box<dyn Future<Output = Result<MotherboardInfo, String>> + Send>> {
  Box::pin(async {
    tokio::task::spawn_blocking(wmi_provider::query_motherboard_info)
      .await
      .map_err(|e| format!("Join error: {e}"))?
  })
}
