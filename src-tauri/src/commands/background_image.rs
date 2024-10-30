use base64::encode;
use std::fs;
use tauri::command;

use crate::utils::file::get_app_data_dir;

#[command]
pub fn get_background_image() -> Result<String, String> {
  let file = get_app_data_dir("bg-img.jpg");

  // 画像を読み込んでBase64にエンコード
  match fs::read(&file) {
    Ok(image_data) => Ok(encode(image_data)),
    Err(e) => Err(format!("Failed to load image: {}", e)),
  }
}
