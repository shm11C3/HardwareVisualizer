use std::sync::Mutex;

use super::libre_hardware_monitor_model::LibreHardwareMonitorNode;

#[derive(Default)]
pub struct LibreHardwareMonitorDataState {
  pub latest: Mutex<Option<LibreHardwareMonitorNode>>,
}

