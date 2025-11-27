use serde::{Deserialize, Serialize};
use specta::Type;

///
/// - `file_id` : Image file ID
/// - `image_data` : Base64 string of image data
///
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundImage {
  pub file_id: String,
  pub image_data: String,
}
