use tauri::command;

use crate::models::background_image::BackgroundImage;
use crate::services::background_image_service;

///
/// Get background image
///
/// - `file_id`: Image file ID
///
#[command]
#[specta::specta]
pub async fn get_background_image(file_id: String) -> Result<String, String> {
  background_image_service::get_background_image(&file_id).await
}

///
/// Get list of background images in BG_IMG_DIR_NAME directory
///
#[command]
#[specta::specta]
pub async fn get_background_images() -> Result<Vec<BackgroundImage>, String> {
  background_image_service::get_background_images().await
}

///
/// Save background image
///
/// - `image_data`: Base64 string of image data
/// - returns: `file_id`
///
/// ### TODO
/// - Use JsImage https://docs.rs/tauri/2.1.1/tauri/image/enum.JsImage.html
///   - Implemented with Base64 for now as type definition with specta was difficult
///
///
#[command]
#[specta::specta]
pub async fn save_background_image(image_data: String) -> Result<String, String> {
  background_image_service::save_background_image(&image_data).await
}

///
/// Delete background image
/// - `file_id`: Image file ID
///
#[tauri::command]
#[specta::specta]
pub async fn delete_background_image(file_id: String) -> Result<(), String> {
  background_image_service::delete_background_image(&file_id).await
}
