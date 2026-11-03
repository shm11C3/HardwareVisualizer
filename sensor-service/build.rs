fn main() {
  println!("cargo:rerun-if-env-changed=HARDVIZ_SENSOR_VERSION");

  let version = std::env::var("HARDVIZ_SENSOR_VERSION")
    .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
  println!("cargo:rustc-env=HARDVIZ_SENSOR_VERSION={version}");

  if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
    println!("cargo:rustc-link-arg-bins=/DEPENDENTLOADFLAG:0x800");

    let mut parts = version
      .split(['.', '-'])
      .take(3)
      .map(|v| v.parse::<u64>().unwrap_or(0));

    let numeric = (parts.next().unwrap_or(0) << 48)
      | (parts.next().unwrap_or(0) << 32)
      | (parts.next().unwrap_or(0) << 16);
    tauri_winres::WindowsResource::new()
      .set(
        "FileDescription",
        "HardwareVisualizer Windows sensor service",
      )
      .set("ProductName", "HardwareVisualizer")
      .set("FileVersion", &version)
      .set("ProductVersion", &version)
      .set_version_info(tauri_winres::VersionInfo::FILEVERSION, numeric)
      .set_version_info(tauri_winres::VersionInfo::PRODUCTVERSION, numeric)
      .compile()
      .expect("compile Windows service version resource");
  }
}
