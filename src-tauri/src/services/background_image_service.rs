use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use image::{ImageFormat, load_from_memory};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::models::background_image::BackgroundImage;
use crate::utils::file::get_app_data_dir;

const FILE_NAME_FORMAT: &str = "bg-img-{}.png";
const BG_IMG_DIR_NAME: &str = "BgImages";

/// Get a single background image
pub async fn get_background_image(file_id: &str) -> Result<String, String> {
  let dir_path = get_app_data_dir(BG_IMG_DIR_NAME);

  // Create App/BgImages directory if it doesn't exist
  if !dir_path.exists() {
    fs::create_dir_all(&dir_path)
      .await
      .map_err(|e| format!("Failed to create directory: {e}"))?;
  }

  let file_name = FILE_NAME_FORMAT.replace("{}", file_id);
  let file_path = dir_path.join(file_name);

  // Load image and encode to Base64
  match fs::read(&file_path).await {
    Ok(image_data) => Ok(STANDARD.encode(image_data)),
    Err(e) => Err(format!("Failed to load image: {e}")),
  }
}

/// Get list of background images
pub async fn get_background_images() -> Result<Vec<BackgroundImage>, String> {
  let dir_path = get_app_data_dir(BG_IMG_DIR_NAME);

  // Create App/BgImages directory if it doesn't exist
  if !dir_path.exists() {
    fs::create_dir_all(&dir_path)
      .await
      .map_err(|e| format!("Failed to create directory: {e}"))?;
  }

  // Get list of files in directory
  match fs::read_dir(&dir_path).await {
    Ok(mut entries) => {
      let mut images: Vec<BackgroundImage> = Vec::new();

      while let Some(entry) = entries.next_entry().await.ok().flatten() {
        if let Some(file_name) = entry.file_name().to_str()
          && let Some(file_id) = file_name
            .strip_prefix("bg-img-")
            .and_then(|s| s.strip_suffix(".png"))
          && let Ok(image_data) = fs::read(&entry.path()).await
        {
          images.push(BackgroundImage {
            file_id: file_id.to_string(),
            image_data: STANDARD.encode(image_data),
          });
        }
      }

      images.sort_by(|a, b| b.file_id.cmp(&a.file_id));
      Ok(images)
    }
    Err(e) => Err(format!("Failed to read directory: {e}")),
  }
}

/// Save background image and return file_id
pub async fn save_background_image(image_data: &str) -> Result<String, String> {
  let dir_path = get_app_data_dir(BG_IMG_DIR_NAME);

  // Create App/BgImages directory if it doesn't exist
  if !dir_path.exists() {
    fs::create_dir_all(&dir_path)
      .await
      .map_err(|e| format!("Failed to create directory: {e}"))?;
  }

  // Remove Base64 data prefix
  let image_data = if let Some(index) = image_data.find(",") {
    &image_data[(index + 1)..]
  } else {
    image_data
  };

  // Remove newlines and extra whitespace
  let cleaned_data = image_data.replace("\n", "").replace("\r", "");

  let file_id = Uuid::now_v7().to_string();
  let file_name = FILE_NAME_FORMAT.replace("{}", &file_id);
  let file_path = dir_path.join(file_name);

  // Decode Base64 data
  match STANDARD.decode(&cleaned_data) {
    Ok(decoded_data) => {
      // Save image data as PNG
      match load_from_memory(&decoded_data) {
        Ok(image) => {
          let mut file = fs::File::create(&file_path)
            .await
            .map_err(|e| format!("Failed to create file: {e}"))?;

          // Write image data asynchronously
          let mut buffer = Vec::new();
          let mut cursor = std::io::Cursor::new(&mut buffer);
          image
            .write_to(&mut cursor, ImageFormat::Png)
            .map_err(|e| format!("Failed to convert image to PNG format: {e}"))?;

          file
            .write_all(&buffer)
            .await
            .map_err(|e| format!("Failed to save image as PNG: {e}"))?;
          Ok(file_id)
        }
        Err(e) => Err(format!("Failed to load image from memory: {e}")),
      }
    }
    Err(e) => Err(format!("Failed to decode image: {e}")),
  }
}

/// Delete background image
pub async fn delete_background_image(file_id: &str) -> Result<(), String> {
  let dir_path = get_app_data_dir(BG_IMG_DIR_NAME);
  let file_name = FILE_NAME_FORMAT.replace("{}", file_id);
  let file_path = dir_path.join(file_name);

  match fs::remove_file(&file_path).await {
    Ok(_) => Ok(()),
    Err(e) => Err(format!("Failed to delete image: {e}")),
  }
}
