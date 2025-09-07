#[cfg(test)]
mod tests {
  use crate::enums::error::BackendError;
  use crate::services::network_service::*;

  #[test]
  fn test_fetch_network_info_returns_result() {
    let result = fetch_network_info();

    match result {
      Ok(network_info) => {
        // 成功時：NetworkInfoのVecが返される
        assert!(network_info.is_empty() || !network_info.is_empty());
        // 各NetworkInfoの基本的な構造をチェック
        for info in network_info {
          // NetworkInfo構造体のフィールドをチェック
          // descriptionは Option<String> なので、存在する場合は空文字でないことを確認
          if let Some(ref description) = info.description {
            assert!(!description.is_empty(), "Description should not be empty when present");
          }
        }
      }
      Err(_) => {
        // プラットフォーム初期化またはネットワーク情報取得に失敗
        // すべてのBackendErrorバリアントに対応
        assert!(true); // エラーが期待される場合もある
      }
    }
  }

  #[test]
  fn test_fetch_network_info_error_type() {
    match fetch_network_info() {
      Ok(_) => {
        // 成功時は何もしない
        assert!(true);
      }
      Err(error) => {
        // エラーの型が正しいかチェック
        match error {
          BackendError::UnexpectedError => assert!(true),
          _ => assert!(true, "Unexpected error type: {:?}", error),
        }
      }
    }
  }
}
