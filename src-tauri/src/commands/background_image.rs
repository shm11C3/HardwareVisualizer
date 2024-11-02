use base64::encode;
use image::load_from_memory;
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use tauri::command;

use crate::utils::file::get_app_data_dir;

const FILE_NAME_FORMAT: &str = "bg-img-{}.png";
const BG_IMG_DIR_NAME: &str = "BgImages";

///
/// 背景画像を取得
///
/// - `file_id`: 画像ファイルのインデックス
///
#[command]
pub fn get_background_image(file_id: String) -> Result<String, String> {
  let dir_path = get_app_data_dir(BG_IMG_DIR_NAME);
  let file_name = FILE_NAME_FORMAT.replace("{}", &file_id);
  let file_path = dir_path.join(file_name);

  // 画像を読み込んでBase64にエンコード
  match fs::read(&file_path) {
    Ok(image_data) => Ok(encode(image_data)),
    Err(e) => Err(format!("Failed to load image: {}", e)),
  }
}

///
/// - `file_id` : 画像ファイルID
/// - `image_data` : 画像データのBase64文字列
///
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundImage {
  pub file_id: String,
  pub image_data: String,
}

///
/// BG_IMG_DIR_NAME ディレクトリ内の背景画像一覧を取得
///
#[command]
pub fn get_background_images() -> Result<Vec<BackgroundImage>, String> {
  let dir_path = get_app_data_dir(BG_IMG_DIR_NAME);

  // ディレクトリ内のファイル一覧を取得
  match fs::read_dir(&dir_path) {
    Ok(entries) => {
      let images: Vec<BackgroundImage> = entries
        .filter_map(|entry| {
          let entry = entry.ok()?;
          let file_name = entry.file_name().into_string().ok()?;
          let file_id = file_name
            .strip_prefix("bg-img-")
            .and_then(|s| s.strip_suffix(".png"))?;
          let file_path = entry.path();
          let image_data = fs::read(&file_path).ok().map(|data| encode(data))?;
          Some(BackgroundImage {
            file_id: file_id.to_string(),
            image_data,
          })
        })
        .collect();
      Ok(images)
    }
    Err(e) => Err(format!("Failed to read directory: {}", e)),
  }
}

///
/// 背景画像を保存
///
/// - `image_data`: 画像データのBase64文字列
/// - returns: `file_id`
///
#[command]
pub fn save_background_image(image_data: String) -> Result<String, String> {
  let dir_path = get_app_data_dir(BG_IMG_DIR_NAME);

  // App/BgImages ディレクトリが存在しない場合新規作成
  if !dir_path.parent().unwrap().exists() {
    fs::create_dir_all(dir_path.parent().unwrap()).unwrap();
  }

  // Base64データのプレフィックスを除去
  let image_data = if let Some(index) = image_data.find(",") {
    &image_data[(index + 1)..]
  } else {
    &image_data
  };

  // 改行や余分な空白を除去
  let cleaned_data = image_data.replace("\n", "").replace("\r", "");

  // ディレクトリ内のファイル数を取得し、それをインデックスとして利用
  let file_id: String = match fs::read_dir(&dir_path) {
    Ok(entries) => entries.count(), // 現在のファイル数をインデックスとして利用
    Err(_) => 0,                    // 読み込み失敗の場合は最初のファイルとして 0
  }
  .to_string();

  let file_name = FILE_NAME_FORMAT.replace("{}", &file_id);
  let file_path = dir_path.join(file_name);

  // Base64データをデコード
  match base64::decode(&cleaned_data) {
    Ok(decoded_data) => {
      // 画像データをPNGとして保存
      match load_from_memory(&decoded_data) {
        Ok(image) => {
          let mut file = File::create(&file_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
          image
            .write_to(&mut file, ImageFormat::Png)
            .map_err(|e| format!("Failed to save image as PNG: {}", e))?;
          Ok(file_id)
        }
        Err(e) => Err(format!("Failed to load image from memory: {}", e)),
      }
    }
    Err(e) => Err(format!("Failed to decode image: {}", e)),
  }
}

///
/// 背景画像を削除
/// - `file_id`: 画像ファイルのインデックス
///
#[tauri::command]
pub fn delete_background_image(file_id: String) -> Result<(), String> {
  let dir_path = get_app_data_dir(BG_IMG_DIR_NAME);
  let file_name = FILE_NAME_FORMAT.replace("{}", &file_id);
  let file_path = dir_path.join(file_name);

  match fs::remove_file(&file_path) {
    Ok(_) => Ok(()),
    Err(e) => Err(format!("Failed to delete image: {}", e)),
  }
}
