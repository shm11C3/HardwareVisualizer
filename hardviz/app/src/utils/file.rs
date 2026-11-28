use std::path::PathBuf;

#[cfg(test)]
use mockall::automock;

// Trait to abstract environment variable access
#[cfg_attr(test, automock)]
pub trait EnvProvider {
  fn get_var(&self, key: &str) -> Result<String, std::env::VarError>;
}

// Actual environment variable access
pub struct RealEnvProvider;

impl EnvProvider for RealEnvProvider {
  fn get_var(&self, key: &str) -> Result<String, std::env::VarError> {
    std::env::var(key)
  }
}

///
/// Get the path of `sub_item` under the platform app data directory:
/// `AppData/Roaming/<identifier>` on Windows,
/// `~/Library/Application Support/<identifier>` on macOS, and
/// `~/.config/<identifier>` on Linux.
///
/// Unit tests resolve it under a temporary directory owned by the test's
/// thread instead, so no test reads or overwrites the developer's real
/// `HardwareVisualizerDev` data, and tests that run in parallel never share a
/// file. libtest runs each test on its own thread, and the directory is removed
/// when that thread exits.
///
pub fn get_app_data_dir(sub_item: &str) -> PathBuf {
  #[cfg(test)]
  return test_app_data_root().join(sub_item);

  #[cfg(not(test))]
  get_app_data_dir_with_env(&RealEnvProvider, sub_item)
}

#[cfg(test)]
fn test_app_data_root() -> PathBuf {
  thread_local! {
    static ROOT: tempfile::TempDir =
      tempfile::tempdir().expect("create a per-test app data directory");
  }

  ROOT.with(|dir| dir.path().join(crate::utils::tauri::get_identifier()))
}

#[cfg(target_os = "windows")]
pub fn get_app_data_dir_with_env<E: EnvProvider>(env: &E, sub_item: &str) -> PathBuf {
  // Create directory based on identifier from tauri.conf.json
  let identifier = crate::utils::tauri::get_identifier();

  let app_data = PathBuf::from(env.get_var("APPDATA").unwrap());
  app_data.join(identifier).join(sub_item)
}

#[cfg(target_os = "macos")]
pub fn get_app_data_dir_with_env<E: EnvProvider>(env: &E, sub_item: &str) -> PathBuf {
  use std::path::Path;

  let identifier = crate::utils::tauri::get_identifier();

  let home = env.get_var("HOME").unwrap_or_else(|_| ".".to_string());
  Path::new(&home)
    .join("Library")
    .join("Application Support")
    .join(identifier)
    .join(sub_item)
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
pub fn get_app_data_dir_with_env<E: EnvProvider>(env: &E, sub_item: &str) -> PathBuf {
  use std::path::Path;

  let identifier = crate::utils::tauri::get_identifier();

  let home = env.get_var("HOME").unwrap_or_else(|_| ".".to_string());
  Path::new(&home)
    .join(".config")
    .join(identifier)
    .join(sub_item)
}

#[cfg(test)]
mod tests {
  use super::*;
  use mockall::predicate::eq;
  use std::path::PathBuf;

  #[test]
  fn unit_tests_resolve_app_data_outside_the_real_app_data_directory() {
    let real = get_app_data_dir_with_env(&RealEnvProvider, "settings.json");
    let resolved = get_app_data_dir("settings.json");

    assert_ne!(resolved, real);
    assert!(resolved.starts_with(std::env::temp_dir()));
    assert!(resolved.ends_with("settings.json"));
  }

  #[test]
  fn each_test_thread_gets_its_own_app_data_directory() {
    let here = get_app_data_dir("settings.json");
    let other = std::thread::spawn(|| get_app_data_dir("settings.json"))
      .join()
      .unwrap();

    assert_ne!(here, other);
    assert_eq!(here, get_app_data_dir("settings.json"));
  }

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
        .return_const(Ok("/Users/test".to_string()));

      let result = get_app_data_dir_with_env(&mock_env, "test_subdirectory");
      let path_str = result.to_string_lossy();
      assert!(path_str.contains("test_subdirectory"));

      #[cfg(target_os = "macos")]
      assert!(path_str.contains("Library/Application Support"));

      #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
      assert!(path_str.contains(".config"));
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
        .return_const(Ok("/Users/test".to_string()));

      mock_env2
        .expect_get_var()
        .with(eq("HOME"))
        .return_const(Ok("/Users/test".to_string()));

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

    let result =
      std::panic::catch_unwind(|| get_app_data_dir_with_env(&mock_env, "test"));

    assert!(result.is_err());
  }

  #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
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

  #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
  #[test]
  fn test_get_app_data_dir_unix_without_home() {
    let mut mock_env = MockEnvProvider::new();

    mock_env
      .expect_get_var()
      .with(eq("HOME"))
      .return_const(Err(std::env::VarError::NotPresent));

    let result = get_app_data_dir_with_env(&mock_env, "test");
    let path_str = result.to_string_lossy();

    assert!(!path_str.is_empty());
    assert!(path_str.contains("test"));
    assert!(path_str.contains(".config"));
  }

  #[cfg(target_os = "macos")]
  #[test]
  fn test_get_app_data_dir_macos_with_home() {
    let mut mock_env = MockEnvProvider::new();

    mock_env
      .expect_get_var()
      .with(eq("HOME"))
      .return_const(Ok("/Users/testuser".to_string()));

    let result = get_app_data_dir_with_env(&mock_env, "test");
    let path_str = result.to_string_lossy();

    assert!(!path_str.is_empty());
    assert!(path_str.contains("test"));
    assert!(path_str.contains("Library/Application Support"));
    assert!(path_str.contains("testuser") || path_str.contains("/Users"));
  }

  #[cfg(target_os = "macos")]
  #[test]
  fn test_get_app_data_dir_macos_without_home() {
    let mut mock_env = MockEnvProvider::new();

    mock_env
      .expect_get_var()
      .with(eq("HOME"))
      .return_const(Err(std::env::VarError::NotPresent));

    let result = get_app_data_dir_with_env(&mock_env, "test");
    let path_str = result.to_string_lossy();

    assert!(!path_str.is_empty());
    assert!(path_str.contains("test"));
    assert!(path_str.contains("Library/Application Support"));
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

    let components: Vec<_> = result.components().collect();
    assert!(!components.is_empty());

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

    let result1 = get_app_data_dir_with_env(&mock_env1, "consistency_test");
    let result2 = get_app_data_dir_with_env(&mock_env2, "consistency_test");

    assert_eq!(result1, result2);
  }

  #[test]
  fn test_real_env_provider() {
    let env = RealEnvProvider;

    match env.get_var("PATH") {
      Ok(path) => assert!(!path.is_empty()),
      Err(_) => panic!("PATH environment variable should exist"),
    }

    match env.get_var("NONEXISTENT_VAR_12345") {
      Ok(_) => panic!("Nonexistent environment variable should not exist"),
      Err(std::env::VarError::NotPresent) => {}
      Err(_) => panic!("Unexpected error type"),
    }
  }
}
