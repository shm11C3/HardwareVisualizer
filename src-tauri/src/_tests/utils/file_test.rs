#[cfg(test)]
mod tests {
  use crate::utils::file::*;
  use std::path::PathBuf;
  use mockall::predicate::eq;

  #[test]
  fn test_get_app_data_dir_path_structure_with_env() {
    let mut mock_env = MockEnvProvider::new();
    
    #[cfg(target_os = "windows")]
    {
      mock_env
        .expect_get_var()
        .with(eq("APPDATA"))
        .return_const(Ok("C:\\Users\\Test\\AppData\\Roaming".to_string()));
      
      let result = get_app_data_dir_with_env(&mock_env, "test_subdirectory");
      let path_str = result.to_string_lossy();
      assert!(path_str.contains("test_subdirectory"));
    }
    
    #[cfg(not(target_os = "windows"))]
    {
      mock_env
        .expect_get_var()
        .with(eq("HOME"))
        .return_const(Ok("/home/test".to_string()));
      
      let result = get_app_data_dir_with_env(&mock_env, "test_subdirectory");
      let path_str = result.to_string_lossy();
      assert!(path_str.contains("test_subdirectory"));
    }
  }

  #[test]
  fn test_get_app_data_dir_different_subdirectories_with_env() {
    let mut mock_env1 = MockEnvProvider::new();
    let mut mock_env2 = MockEnvProvider::new();
    
    #[cfg(target_os = "windows")]
    {
      mock_env1
        .expect_get_var()
        .with(eq("APPDATA"))
        .return_const(Ok("C:\\Users\\Test\\AppData\\Roaming".to_string()));
      
      mock_env2
        .expect_get_var()
        .with(eq("APPDATA"))
        .return_const(Ok("C:\\Users\\Test\\AppData\\Roaming".to_string()));
      
      let result1 = get_app_data_dir_with_env(&mock_env1, "logs");
      let result2 = get_app_data_dir_with_env(&mock_env2, "cache");
      
      assert_ne!(result1, result2);
      assert!(result1.to_string_lossy().contains("logs"));
      assert!(result2.to_string_lossy().contains("cache"));
    }
    
    #[cfg(not(target_os = "windows"))]
    {
      mock_env1
        .expect_get_var()
        .with(eq("HOME"))
        .return_const(Ok("/home/test".to_string()));
      
      mock_env2
        .expect_get_var()
        .with(eq("HOME"))
        .return_const(Ok("/home/test".to_string()));
      
      let result1 = get_app_data_dir_with_env(&mock_env1, "logs");
      let result2 = get_app_data_dir_with_env(&mock_env2, "cache");
      
      assert_ne!(result1, result2);
      assert!(result1.to_string_lossy().contains("logs"));
      assert!(result2.to_string_lossy().contains("cache"));
    }
  }

  #[test]
  fn test_get_app_data_dir_empty_string_with_env() {
    let mut mock_env = MockEnvProvider::new();
    
    #[cfg(target_os = "windows")]
    {
      mock_env
        .expect_get_var()
        .with(eq("APPDATA"))
        .return_const(Ok("C:\\Users\\Test\\AppData\\Roaming".to_string()));
      
      let result = get_app_data_dir_with_env(&mock_env, "");
      let path_str = result.to_string_lossy();
      assert!(!path_str.is_empty());
    }
    
    #[cfg(not(target_os = "windows"))]
    {
      mock_env
        .expect_get_var()
        .with(eq("HOME"))
        .return_const(Ok("/home/test".to_string()));
      
      let result = get_app_data_dir_with_env(&mock_env, "");
      let path_str = result.to_string_lossy();
      assert!(!path_str.is_empty());
    }
  }

  #[test]
  fn test_get_app_data_dir_special_characters_with_env() {
    let mut mock_env = MockEnvProvider::new();
    
    #[cfg(target_os = "windows")]
    {
      mock_env
        .expect_get_var()
        .with(eq("APPDATA"))
        .return_const(Ok("C:\\Users\\Test\\AppData\\Roaming".to_string()));
      
      let result = get_app_data_dir_with_env(&mock_env, "test_dir-with.special_chars");
      let path_str = result.to_string_lossy();
      assert!(path_str.contains("test_dir-with.special_chars"));
    }
    
    #[cfg(not(target_os = "windows"))]
    {
      mock_env
        .expect_get_var()
        .with(eq("HOME"))
        .return_const(Ok("/home/test".to_string()));
      
      let result = get_app_data_dir_with_env(&mock_env, "test_dir-with.special_chars");
      let path_str = result.to_string_lossy();
      assert!(path_str.contains("test_dir-with.special_chars"));
    }
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn test_get_app_data_dir_windows_with_appdata() {
    let mut mock_env = MockEnvProvider::new();
    
    mock_env
      .expect_get_var()
      .with(eq("APPDATA"))
      .return_const(Ok("C:\\Users\\TestUser\\AppData\\Roaming".to_string()));
    
    let result = get_app_data_dir_with_env(&mock_env, "test");
    let path_str = result.to_string_lossy();
    
    assert!(!path_str.is_empty());
    assert!(path_str.contains("test"));
    assert!(path_str.contains("AppData\\Roaming") || path_str.contains("TestUser"));
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn test_get_app_data_dir_windows_without_appdata() {
    let mut mock_env = MockEnvProvider::new();
    
    mock_env
      .expect_get_var()
      .with(eq("APPDATA"))
      .return_const(Err(std::env::VarError::NotPresent));
    
    // パニックが発生することを期待
    let result = std::panic::catch_unwind(|| {
      get_app_data_dir_with_env(&mock_env, "test")
    });
    
    assert!(result.is_err());
  }

  #[cfg(not(target_os = "windows"))]
  #[test]
  fn test_get_app_data_dir_unix_with_home() {
    let mut mock_env = MockEnvProvider::new();
    
    mock_env
      .expect_get_var()
      .with(eq("HOME"))
      .return_const(Ok("/home/testuser".to_string()));
    
    let result = get_app_data_dir_with_env(&mock_env, "test");
    let path_str = result.to_string_lossy();
    
    assert!(!path_str.is_empty());
    assert!(path_str.contains("test"));
    assert!(path_str.contains(".config"));
    assert!(path_str.contains("testuser") || path_str.contains("/home"));
  }

  #[cfg(not(target_os = "windows"))]
  #[test]
  fn test_get_app_data_dir_unix_without_home() {
    let mut mock_env = MockEnvProvider::new();
    
    mock_env
      .expect_get_var()
      .with(eq("HOME"))
      .return_const(Err(std::env::VarError::NotPresent));
    
    let result = get_app_data_dir_with_env(&mock_env, "test");
    let path_str = result.to_string_lossy();
    
    // HOME が無い場合は "." が使用されることを確認
    assert!(!path_str.is_empty());
    assert!(path_str.contains("test"));
    assert!(path_str.contains(".config"));
  }

  #[test]
  fn test_get_app_data_dir_return_type() {
    let mut mock_env = MockEnvProvider::new();
    
    #[cfg(target_os = "windows")]
    {
      mock_env
        .expect_get_var()
        .with(eq("APPDATA"))
        .return_const(Ok("C:\\Users\\Test\\AppData\\Roaming".to_string()));
    }
    #[cfg(not(target_os = "windows"))]
    {
      mock_env
        .expect_get_var()
        .with(eq("HOME"))
        .return_const(Ok("/home/test".to_string()));
    }
    
    let result = get_app_data_dir_with_env(&mock_env, "type_test");
    
    // 戻り値が PathBuf 型であることを確認
    let _pathbuf: PathBuf = result;
    assert!(_pathbuf.to_string_lossy().contains("type_test"));
  }

  #[test]
  fn test_get_app_data_dir_path_components() {
    let mut mock_env = MockEnvProvider::new();
    
    #[cfg(target_os = "windows")]
    {
      mock_env
        .expect_get_var()
        .with(eq("APPDATA"))
        .return_const(Ok("C:\\Users\\Test\\AppData\\Roaming".to_string()));
    }
    #[cfg(not(target_os = "windows"))]
    {
      mock_env
        .expect_get_var()
        .with(eq("HOME"))
        .return_const(Ok("/home/test".to_string()));
    }
    
    let result = get_app_data_dir_with_env(&mock_env, "components_test");
    
    // パスのコンポーネントを確認
    let components: Vec<_> = result.components().collect();
    assert!(!components.is_empty());
    
    // 最後のコンポーネントがサブディレクトリ名であることを確認
    if let Some(last_component) = components.last() {
      let last_str = last_component.as_os_str().to_string_lossy();
      assert_eq!(last_str, "components_test");
    }
  }

  #[test]
  fn test_get_app_data_dir_consistency() {
    let mut mock_env1 = MockEnvProvider::new();
    let mut mock_env2 = MockEnvProvider::new();
    
    #[cfg(target_os = "windows")]
    {
      mock_env1
        .expect_get_var()
        .with(eq("APPDATA"))
        .return_const(Ok("C:\\Users\\Test\\AppData\\Roaming".to_string()));
      
      mock_env2
        .expect_get_var()
        .with(eq("APPDATA"))
        .return_const(Ok("C:\\Users\\Test\\AppData\\Roaming".to_string()));
    }
    #[cfg(not(target_os = "windows"))]
    {
      mock_env1
        .expect_get_var()
        .with(eq("HOME"))
        .return_const(Ok("/home/test".to_string()));
      
      mock_env2
        .expect_get_var()
        .with(eq("HOME"))
        .return_const(Ok("/home/test".to_string()));
    }
    
    // 同じ引数・同じ環境変数で複数回呼び出した場合、同じ結果が返されることを確認
    let result1 = get_app_data_dir_with_env(&mock_env1, "consistency_test");
    let result2 = get_app_data_dir_with_env(&mock_env2, "consistency_test");
    
    assert_eq!(result1, result2);
  }

  #[test]
  fn test_real_env_provider() {
    let env = RealEnvProvider;
    
    // PATH環境変数は通常存在するはず
    match env.get_var("PATH") {
      Ok(path) => assert!(!path.is_empty()),
      Err(_) => panic!("PATH environment variable should exist"),
    }
    
    // 存在しない環境変数はエラーになるはず
    match env.get_var("NONEXISTENT_VAR_12345") {
      Ok(_) => panic!("Nonexistent environment variable should not exist"),
      Err(std::env::VarError::NotPresent) => {}, // 期待される結果
      Err(_) => panic!("Unexpected error type"),
    }
  }
}