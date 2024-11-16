use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
  pub size: String,
  pub clock: u64,
  pub clock_unit: String,
  pub memory_count: usize,
  pub total_slots: usize,
  pub memory_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicInfo {
  pub id: String,
  pub name: String,
  pub vendor_name: String,
  pub clock: u64,
  pub memory_size: String,
  pub memory_size_dedicated: String,
}
