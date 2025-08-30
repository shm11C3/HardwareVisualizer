use serde::{Deserialize, Serialize};
use std;

#[derive(Serialize, Deserialize)]
pub struct CachedData<T> {
  pub timestamp: u64, // UNIX time millis
  pub data: T,
}

const MAX_AGE_SECS: u64 = 60 * 60 * 24;

/// キャッシュからデータを読み込む
pub fn read_cache<T>(cache_path: &std::path::PathBuf) -> std::io::Result<T>
where
  T: for<'de> Deserialize<'de>,
{
  let content = std::fs::read_to_string(cache_path)?;
  let wrapper: CachedData<T> = serde_json::from_str(&content)?;

  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_err(std::io::Error::other)?
    .as_secs();

  let cache_time = wrapper.timestamp / 1000;

  if now - cache_time <= MAX_AGE_SECS {
    Ok(wrapper.data)
  } else {
    Err(std::io::Error::other("Cache expired"))
  }
}

/// キャッシュにデータを保存
pub fn write_cache<T>(data: &T, cache_path: &std::path::PathBuf) -> std::io::Result<()>
where
  T: Serialize + Clone,
{
  if let Some(parent) = cache_path.parent() {
    std::fs::create_dir_all(parent)?;
  }

  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_err(std::io::Error::other)?
    .as_millis() as u64;

  let wrapper = CachedData {
    timestamp: now,
    data: data.clone(),
  };

  let json = serde_json::to_string_pretty(&wrapper)?;
  std::fs::write(cache_path, json)
}

/// メモリ情報のキャッシュパスを取得
pub fn get_memory_cache_path() -> std::path::PathBuf {
  dirs::cache_dir()
    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
    .join("hardware_visualizer")
    .join("memory_info.json")
}
