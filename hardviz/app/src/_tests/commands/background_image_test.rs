#[cfg(test)]
mod tests {
  use crate::commands::background_image::*;

  #[tokio::test]
  async fn test_save_background_image_invalid_data() {
    let invalid_base64 = "invalid_base64_data";

    let result = save_background_image(invalid_base64.to_string()).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_get_background_image_nonexistent_file() {
    let file_id = "nonexistent_file_id";

    let result = get_background_image(file_id.to_string()).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_delete_background_image_nonexistent_file() {
    let file_id = "nonexistent_file_id";

    let result = delete_background_image(file_id.to_string()).await;
    assert!(result.is_err());
  }
}
