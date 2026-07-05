#[cfg(not(debug_assertions))]
fn main() {
  compile_error!("export_bindings is only available in debug builds");
}

#[cfg(debug_assertions)]
fn main() {
  hardware_monitor_lib::export_bindings();
}
