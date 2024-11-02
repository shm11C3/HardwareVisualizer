use base64::encode;
use std::fs;
use std::io::Write;
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

#[command]
pub fn save_background_image(image_data: String) -> Result<String, String> {
  let dir_path = get_app_data_dir("BgImages");

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

  // 拡張子を設定 (pngまたはjpegを仮定)
  let extension = if cleaned_data.starts_with("/9j/") {
    "jpeg"
  } else {
    "png"
  };

  // ディレクトリ内のファイル数を取得し、それをインデックスとして利用
  let file_index = match fs::read_dir(&dir_path) {
    Ok(entries) => entries.count(), // 現在のファイル数をインデックスとして利用
    Err(_) => 0,                    // 読み込み失敗の場合は最初のファイルとして 0
  };

  let file_path = dir_path.join(format!("bg-img-{}.{}", file_index, extension));

  // Base64データをデコード
  match base64::decode(&cleaned_data) {
    Ok(decoded_data) => {
      // ファイルを新規に作成
      match fs::File::create(file_path).and_then(|mut file| file.write_all(&decoded_data))
      {
        Ok(_) => Ok(file_index.to_string()),
        Err(e) => Err(format!("Failed to save image: {}", e)),
      }
    }
    Err(e) => Err(format!("Failed to decode image: {}", e)),
  }
}
