#[cfg(test)]
mod tests {
  use crate::utils::file::*;
  use std::path::PathBuf;
  use std::sync::Mutex;

  // テスト間での環境変数操作の競合を防ぐためのMutex
  static ENV_LOCK: Mutex<()> = Mutex::new(());

  #[test]
  fn test_get_app_data_dir_path_structure_with_env() {
    let _guard = ENV_LOCK.lock().unwrap();

    // 環境変数を設定してテスト
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::set_var("APPDATA", "C:\\Users\\Test\\AppData\\Roaming");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::set_var("HOME", "/home/test");
    }

    let result = get_app_data_dir("test_subdirectory");
    let path_str = result.to_string_lossy();

    // パスの文字列表現を確認
    assert!(path_str.contains("test_subdirectory"));

    // テスト後のクリーンアップ
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::remove_var("APPDATA");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::remove_var("HOME");
    }
  }

  #[test]
  fn test_get_app_data_dir_different_subdirectories_with_env() {
    let _guard = ENV_LOCK.lock().unwrap();

    // 環境変数を設定してテスト
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::set_var("APPDATA", "C:\\Users\\Test\\AppData\\Roaming");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::set_var("HOME", "/home/test");
    }

    let result1 = get_app_data_dir("logs");
    let result2 = get_app_data_dir("cache");

    // 異なるサブディレクトリで異なるパスが生成されることを確認
    assert_ne!(result1, result2);

    let path1_str = result1.to_string_lossy();
    let path2_str = result2.to_string_lossy();

    assert!(path1_str.contains("logs"));
    assert!(path2_str.contains("cache"));

    // テスト後のクリーンアップ
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::remove_var("APPDATA");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::remove_var("HOME");
    }
  }

  #[test]
  fn test_get_app_data_dir_empty_string_with_env() {
    let _guard = ENV_LOCK.lock().unwrap();

    // 環境変数を設定してテスト
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::set_var("APPDATA", "C:\\Users\\Test\\AppData\\Roaming");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::set_var("HOME", "/home/test");
    }

    let result = get_app_data_dir("");
    let path_str = result.to_string_lossy();

    // 空文字列でも有効なパスが生成されることを確認
    assert!(!path_str.is_empty());

    // テスト後のクリーンアップ
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::remove_var("APPDATA");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::remove_var("HOME");
    }
  }

  #[test]
  fn test_get_app_data_dir_special_characters_with_env() {
    let _guard = ENV_LOCK.lock().unwrap();

    // 環境変数を設定してテスト
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::set_var("APPDATA", "C:\\Users\\Test\\AppData\\Roaming");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::set_var("HOME", "/home/test");
    }

    let result = get_app_data_dir("test_dir-with.special_chars");
    let path_str = result.to_string_lossy();

    assert!(path_str.contains("test_dir-with.special_chars"));

    // テスト後のクリーンアップ
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::remove_var("APPDATA");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::remove_var("HOME");
    }
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn test_get_app_data_dir_windows_with_appdata() {
    let _guard = ENV_LOCK.lock().unwrap();

    // APPDATA 環境変数をモックして Windows 環境をシミュレート
    unsafe {
      std::env::set_var("APPDATA", "C:\\Users\\TestUser\\AppData\\Roaming");
    }

    let result = get_app_data_dir("test");
    let path_str = result.to_string_lossy();

    // Windows 形式のパスが生成されることを確認
    assert!(!path_str.is_empty());
    assert!(path_str.contains("test"));
    assert!(path_str.contains("AppData\\Roaming") || path_str.contains("TestUser"));

    // テスト後にクリーンアップ
    unsafe {
      std::env::remove_var("APPDATA");
    }
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn test_get_app_data_dir_windows_without_appdata() {
    let _guard = ENV_LOCK.lock().unwrap();

    // 元の値を保存
    let original_appdata = std::env::var("APPDATA").ok();

    // APPDATA 環境変数を削除
    unsafe {
      std::env::remove_var("APPDATA");
    }

    // パニックが発生することを期待
    let result = std::panic::catch_unwind(|| get_app_data_dir("test"));

    assert!(result.is_err());

    // テスト後に元の値を復元
    if let Some(value) = original_appdata {
      unsafe {
        std::env::set_var("APPDATA", value);
      }
    }
  }

  #[cfg(not(target_os = "windows"))]
  #[test]
  fn test_get_app_data_dir_unix_with_home() {
    let _guard = ENV_LOCK.lock().unwrap();

    // HOME 環境変数をモックして Unix 系環境をシミュレート
    unsafe {
      std::env::set_var("HOME", "/home/testuser");
    }

    let result = get_app_data_dir("test");
    let path_str = result.to_string_lossy();

    // Unix 形式のパスが生成されることを確認
    assert!(!path_str.is_empty());
    assert!(path_str.contains("test"));
    assert!(path_str.contains(".config"));
    assert!(path_str.contains("testuser") || path_str.contains("/home"));

    // テスト後にクリーンアップ
    unsafe {
      std::env::remove_var("HOME");
    }
  }

  #[cfg(not(target_os = "windows"))]
  #[test]
  fn test_get_app_data_dir_unix_without_home() {
    let _guard = ENV_LOCK.lock().unwrap();

    // 元の値を保存
    let original_home = std::env::var("HOME").ok();

    // HOME 環境変数を削除
    unsafe {
      std::env::remove_var("HOME");
    }

    let result = get_app_data_dir("test");
    let path_str = result.to_string_lossy();

    // HOME が無い場合は "." が使用されることを確認
    assert!(!path_str.is_empty());
    assert!(path_str.contains("test"));
    assert!(path_str.contains(".config"));

    // テスト後に元の値を復元
    if let Some(value) = original_home {
      unsafe {
        std::env::set_var("HOME", value);
      }
    }
  }

  #[test]
  fn test_get_app_data_dir_return_type() {
    let _guard = ENV_LOCK.lock().unwrap();

    // 環境変数を設定してテスト
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::set_var("APPDATA", "C:\\Users\\Test\\AppData\\Roaming");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::set_var("HOME", "/home/test");
    }

    let result = get_app_data_dir("type_test");

    // 戻り値が PathBuf 型であることを確認
    let _pathbuf: PathBuf = result;
    // パスの文字列表現を確認
    assert!(_pathbuf.to_string_lossy().contains("type_test"));

    // テスト後のクリーンアップ
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::remove_var("APPDATA");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::remove_var("HOME");
    }
  }

  #[test]
  fn test_get_app_data_dir_path_components() {
    let _guard = ENV_LOCK.lock().unwrap();

    // 環境変数を設定してテスト
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::set_var("APPDATA", "C:\\Users\\Test\\AppData\\Roaming");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::set_var("HOME", "/home/test");
    }

    let result = get_app_data_dir("components_test");

    // パスのコンポーネントを確認
    let components: Vec<_> = result.components().collect();
    assert!(!components.is_empty());

    // 最後のコンポーネントがサブディレクトリ名であることを確認
    if let Some(last_component) = components.last() {
      let last_str = last_component.as_os_str().to_string_lossy();
      assert_eq!(last_str, "components_test");
    }

    // テスト後のクリーンアップ
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::remove_var("APPDATA");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::remove_var("HOME");
    }
  }

  #[test]
  fn test_get_app_data_dir_consistency() {
    let _guard = ENV_LOCK.lock().unwrap();

    // 環境変数を設定してテスト
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::set_var("APPDATA", "C:\\Users\\Test\\AppData\\Roaming");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::set_var("HOME", "/home/test");
    }

    // 同じ引数で複数回呼び出した場合、同じ結果が返されることを確認
    let result1 = get_app_data_dir("consistency_test");
    let result2 = get_app_data_dir("consistency_test");

    assert_eq!(result1, result2);

    // テスト後のクリーンアップ
    #[cfg(target_os = "windows")]
    unsafe {
      std::env::remove_var("APPDATA");
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
      std::env::remove_var("HOME");
    }
  }
}
