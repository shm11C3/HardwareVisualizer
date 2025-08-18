use serde::{Deserialize, Serialize};
use specta::Type;

///
/// - `file_id` : 画像ファイルID
/// - `image_data` : 画像データのBase64文字列
///
#[derive(Debug, Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundImage {
  pub file_id: String,
  pub image_data: String,
}
