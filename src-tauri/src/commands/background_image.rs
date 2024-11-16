use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::load_from_memory;
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use tauri::command;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::utils::file::get_app_data_dir;

const FILE_NAME_FORMAT: &str = "bg-img-{}.png";
const BG_IMG_DIR_NAME: &str = "BgImages";

///
/// 背景画像を取得
///
/// - `file_id`: 画像ファイルID
///
#[command]
pub async fn get_background_image(file_id: String) -> Result<String, String> {
  let dir_path = get_app_data_dir(BG_IMG_DIR_NAME);

  // App/BgImages ディレクトリが存在しない場合新規作成
  if !dir_path.exists() {
    fs::create_dir_all(&dir_path)
      .await
      .map_err(|e| format!("Failed to create directory: {}", e))?;
  }

  let file_name = FILE_NAME_FORMAT.replace("{}", &file_id);
  let file_path = dir_path.join(file_name);

  // 画像を読み込んでBase64にエンコード
  match fs::read(&file_path).await {
    Ok(image_data) => Ok(STANDARD.encode(image_data)),
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
pub async fn get_background_images() -> Result<Vec<BackgroundImage>, String> {
  let dir_path = get_app_data_dir(BG_IMG_DIR_NAME);

  // App/BgImages ディレクトリが存在しない場合新規作成
  if !dir_path.exists() {
    fs::create_dir_all(&dir_path)
      .await
      .map_err(|e| format!("Failed to create directory: {}", e))?;
  }

  // ディレクトリ内のファイル一覧を取得
  match fs::read_dir(&dir_path).await {
    Ok(mut entries) => {
      let mut images: Vec<BackgroundImage> = Vec::new();

      while let Some(entry) = entries.next_entry().await.ok().flatten() {
        if let Some(file_name) = entry.file_name().to_str() {
          if let Some(file_id) = file_name
            .strip_prefix("bg-img-")
            .and_then(|s| s.strip_suffix(".png"))
          {
            let file_path = entry.path();
            if let Ok(image_data) = fs::read(&file_path).await {
              images.push(BackgroundImage {
                file_id: file_id.to_string(),
                image_data: STANDARD.encode(image_data),
              });
            }
          }
        }
      }

      images.sort_by(|a, b| b.file_id.cmp(&a.file_id));
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
pub async fn save_background_image(image_data: String) -> Result<String, String> {
  let dir_path = get_app_data_dir(BG_IMG_DIR_NAME);

  // App/BgImages ディレクトリが存在しない場合新規作成
  if !dir_path.exists() {
    fs::create_dir_all(&dir_path)
      .await
      .map_err(|e| format!("Failed to create directory: {}", e))?;
  }

  // Base64データのプレフィックスを除去
  let image_data = if let Some(index) = image_data.find(",") {
    &image_data[(index + 1)..]
  } else {
    &image_data
  };

  // 改行や余分な空白を除去
  let cleaned_data = image_data.replace("\n", "").replace("\r", "");

  let file_id = Uuid::now_v7().to_string();
  let file_name = FILE_NAME_FORMAT.replace("{}", &file_id);
  let file_path = dir_path.join(file_name);

  // Base64データをデコード
  match STANDARD.decode(&cleaned_data) {
    Ok(decoded_data) => {
      // 画像データをPNGとして保存
      match load_from_memory(&decoded_data) {
        Ok(image) => {
          let mut file = fs::File::create(&file_path)
            .await
            .map_err(|e| format!("Failed to create file: {}", e))?;

          // 非同期で画像データを書き込む
          let mut buffer = Vec::new();
          let mut cursor = std::io::Cursor::new(&mut buffer);
          image
            .write_to(&mut cursor, ImageFormat::Png)
            .map_err(|e| format!("Failed to convert image to PNG format: {}", e))?;

          file
            .write_all(&buffer)
            .await
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
/// - `file_id`: 画像ファイルID
///
#[tauri::command]
pub async fn delete_background_image(file_id: String) -> Result<(), String> {
  let dir_path = get_app_data_dir(BG_IMG_DIR_NAME);
  let file_name = FILE_NAME_FORMAT.replace("{}", &file_id);
  let file_path = dir_path.join(file_name);

  match fs::remove_file(&file_path).await {
    Ok(_) => Ok(()),
    Err(e) => Err(format!("Failed to delete image: {}", e)),
  }
}
