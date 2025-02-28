#[tauri::command]
#[specta::specta]
pub fn restart_app(app_handle: tauri::AppHandle) {
  // 現在の実行ファイルのパスを取得
  let exe_path = std::env::current_exe().expect("Failed to obtain executable file path");
  let args: Vec<String> = std::env::args().collect();

  // 新たにプロセスを生成
  #[allow(clippy::zombie_processes)]
  std::process::Command::new(exe_path)
    .args(args)
    .spawn()
    .expect("Failed to restart process");

  app_handle.exit(0);
}
