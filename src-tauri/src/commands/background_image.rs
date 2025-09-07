use tauri::command;

use crate::models::background_image::BackgroundImage;
use crate::services::background_image_service;

///
/// 背景画像を取得
///
/// - `file_id`: 画像ファイルID
///
#[command]
#[specta::specta]
pub async fn get_background_image(file_id: String) -> Result<String, String> {
  background_image_service::get_background_image(&file_id).await
}

///
/// BG_IMG_DIR_NAME ディレクトリ内の背景画像一覧を取得
///
#[command]
#[specta::specta]
pub async fn get_background_images() -> Result<Vec<BackgroundImage>, String> {
  background_image_service::get_background_images().await
}

///
/// 背景画像を保存
///
/// - `image_data`: 画像データのBase64文字列
/// - returns: `file_id`
///
/// ### TODO
/// - JsImage https://docs.rs/tauri/2.1.1/tauri/image/enum.JsImage.html を使用する
///   - specta での型定義が難しかったため一旦 Base64 で実装
///
///
#[command]
#[specta::specta]
pub async fn save_background_image(image_data: String) -> Result<String, String> {
  background_image_service::save_background_image(&image_data).await
}

///
/// 背景画像を削除
/// - `file_id`: 画像ファイルID
///
#[tauri::command]
#[specta::specta]
pub async fn delete_background_image(file_id: String) -> Result<(), String> {
  background_image_service::delete_background_image(&file_id).await
}
