#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
  #[cfg(target_os = "windows")]
  {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();

    if args.len() == 2 && args[0] == "--validate-install-dir" {
      if let Err(error) =
        hardviz_core::sensor_service::windows::validate_install_directory(
          std::path::Path::new(&args[1]),
        )
      {
        eprintln!("Unsafe sensor service installation: {error}");
        std::process::exit(1);
      }
      return;
    }

    if !args.is_empty() {
      std::process::exit(1);
    }
  }
  #[cfg(target_os = "windows")]
  if let Err(error) =
    hardviz_core::sensor_service::windows::run(env!("HARDVIZ_SENSOR_VERSION"))
  {
    eprintln!("Windows sensor service failed: {error}");
    std::process::exit(1);
  }
  #[cfg(not(target_os = "windows"))]
  {
    eprintln!("The sensor service is available only on Windows");
    std::process::exit(1);
  }
}
