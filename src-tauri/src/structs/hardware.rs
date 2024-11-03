use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicInfo {
  pub name: String,
  pub vendor_name: String,
  pub clock: u64,
  pub memory_size: String,
  pub memory_size_dedicated: String,
}
