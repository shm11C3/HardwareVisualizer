#[cfg(test)]
mod tests {
  use std::collections::{HashMap, VecDeque};
  use std::sync::{Arc, Mutex};
  use sysinfo::System;
  use tauri::Manager;

  use crate::commands::hardware::*;

  ///
  /// Test the get_process_list function
  ///
  #[test]
  fn test_get_process_list() {
    let app = tauri::test::mock_app();

    // Mock
    let mut mock_system = System::new_all();
    mock_system.refresh_all();

    let mock_pid = 12345;
    let mock_process_name = "TestProcess".to_string();
    let mock_cpu_usage = 50.0;
    let mock_memory_usage = 1024.0;

    let mut cpu_histories = HashMap::new();
    cpu_histories.insert(mock_pid.into(), VecDeque::from(vec![mock_cpu_usage; 5]));

    let mut memory_histories = HashMap::new();
    memory_histories.insert(mock_pid.into(), VecDeque::from(vec![mock_memory_usage; 5]));

    let app_state = AppState {
      system: Arc::new(Mutex::new(mock_system)),
      cpu_history: Arc::new(Mutex::new(VecDeque::new())),
      memory_history: Arc::new(Mutex::new(VecDeque::new())),
      gpu_history: Arc::new(Mutex::new(VecDeque::new())),
      gpu_usage: Arc::new(Mutex::new(0.0)),
      process_cpu_histories: Arc::new(Mutex::new(cpu_histories)),
      process_memory_histories: Arc::new(Mutex::new(memory_histories)),
    };

    app.manage(app_state);

    // Act
    let process_list = get_process_list(app.state());

    // Assert
    assert_eq!(process_list.len(), 1);
    let process = &process_list[0];
    assert_eq!(process.pid, mock_pid as i32);
    assert_eq!(process.name, mock_process_name);
    assert_eq!(process.cpu_usage, mock_cpu_usage);
    assert_eq!(process.memory_usage, mock_memory_usage / 1024.0);
  }
}
