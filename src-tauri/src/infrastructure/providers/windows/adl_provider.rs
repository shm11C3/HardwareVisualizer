///
/// AMD Display Library (ADL) provider
///
/// Dynamically loads `atiadlxx.dll` (shipped with AMD GPU drivers) and exposes
/// GPU temperature and usage sensors via the Overdrive 5 / OverdriveN / PMLog
/// API hierarchy.
///
///   OD5 → ODN (OD ≥ 7) → PMLog / OD8 (OD ≥ 8 or !supported)
///
use crate::models::hardware::NameValue;
use crate::{log_debug, log_error, log_internal};
use std::ffi::c_void;
use std::sync::OnceLock;
use tokio::task::spawn_blocking;

// ---------------------------------------------------------------------------
// ADL constants
// ---------------------------------------------------------------------------
const ADL_OK: i32 = 0;
const ADL_TRUE: i32 = 1;
const ADL_MAX_PATH: usize = 256;
const ADL_PMLOG_MAX_SENSORS: usize = 256;
#[allow(dead_code)]
const ADL_SENSOR_MAXTYPES: u32 = 46; // Sentinel in PMLog sensor array

// OverdriveN temperature types
const ODNT_EDGE: i32 = 1;
const ODNT_MEM: i32 = 2;
const ODNT_VRVDDC: i32 = 3;
const ODNT_VRMVDD: i32 = 4;
const ODNT_LIQUID: i32 = 5;
const ODNT_PLX: i32 = 6;
const ODNT_HOTSPOT: i32 = 7;

// PMLog sensor IDs (temperature)
const PMLOG_TEMPERATURE_EDGE: u32 = 8;
const PMLOG_TEMPERATURE_MEM: u32 = 9;
const PMLOG_TEMPERATURE_VRVDDC: u32 = 10;
const PMLOG_TEMPERATURE_VRMVDD: u32 = 11;
const PMLOG_TEMPERATURE_LIQUID: u32 = 12;
const PMLOG_TEMPERATURE_PLX: u32 = 13;
const PMLOG_TEMPERATURE_HOTSPOT: u32 = 27;
const PMLOG_TEMPERATURE_SOC: u32 = 29;

// PMLog sensor IDs (usage)
const PMLOG_INFO_ACTIVITY_GFX: u32 = 19;
#[allow(dead_code)]
const PMLOG_INFO_ACTIVITY_MEM: u32 = 20;

// ---------------------------------------------------------------------------
// ADL C structures (repr(C), matching the AMD SDK headers)
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone)]
struct AdlAdapterInfo {
  size: i32,
  adapter_index: i32,
  udid: [u8; ADL_MAX_PATH],
  bus_number: i32,
  device_number: i32,
  function_number: i32,
  vendor_id: i32,
  adapter_name: [u8; ADL_MAX_PATH],
  display_name: [u8; ADL_MAX_PATH],
  present: i32,
  exist: i32,
  driver_path: [u8; ADL_MAX_PATH],
  driver_path_ext: [u8; ADL_MAX_PATH],
  pnp_string: [u8; ADL_MAX_PATH],
  os_display_index: i32,
}

impl Default for AdlAdapterInfo {
  fn default() -> Self {
    Self {
      size: std::mem::size_of::<AdlAdapterInfo>() as i32,
      adapter_index: 0,
      udid: [0u8; ADL_MAX_PATH],
      bus_number: 0,
      device_number: 0,
      function_number: 0,
      vendor_id: 0,
      adapter_name: [0u8; ADL_MAX_PATH],
      display_name: [0u8; ADL_MAX_PATH],
      present: 0,
      exist: 0,
      driver_path: [0u8; ADL_MAX_PATH],
      driver_path_ext: [0u8; ADL_MAX_PATH],
      pnp_string: [0u8; ADL_MAX_PATH],
      os_display_index: 0,
    }
  }
}

#[repr(C)]
#[derive(Clone, Default)]
struct AdlTemperature {
  size: i32,
  temperature: i32, // millidegrees Celsius
}

#[repr(C)]
#[derive(Clone, Default)]
struct AdlPmActivity {
  size: i32,
  engine_clock: i32,
  memory_clock: i32,
  vddc: i32,
  activity_percent: i32,
  current_performance_level: i32,
  current_bus_speed: i32,
  current_bus_lanes: i32,
  max_bus_lanes: i32,
  reserved: i32,
}

#[repr(C)]
#[derive(Clone, Default)]
struct AdlOdParameterRange {
  min: i32,
  max: i32,
  step: i32,
}

#[repr(C)]
#[derive(Clone, Default)]
struct AdlOdParameters {
  size: i32,
  number_of_performance_levels: i32,
  activity_reporting_supported: i32,
  discrete_performance_levels: i32,
  reserved: i32,
  engine_clock: AdlOdParameterRange,
  memory_clock: AdlOdParameterRange,
  vddc: AdlOdParameterRange,
}

#[repr(C)]
#[derive(Clone)]
#[allow(dead_code)]
struct AdlPmLogDataOutput {
  size: i32,
  logging_address: *mut AdlPmLogData,
}

#[repr(C)]
#[derive(Clone)]
#[allow(dead_code)]
struct AdlPmLogData {
  version: u32,
  active_sample_rate: u32,
  last_updated: u64,
  values: [u32; ADL_PMLOG_MAX_SENSORS * 2],
  reserved: [u32; 256],
}

impl Default for AdlPmLogData {
  fn default() -> Self {
    Self {
      version: 0,
      active_sample_rate: 0,
      last_updated: 0,
      values: [0u32; ADL_PMLOG_MAX_SENSORS * 2],
      reserved: [0u32; 256],
    }
  }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AdlSingleSensorData {
  supported: i32,
  value: i32,
}

#[repr(C)]
#[derive(Clone)]
struct AdlPmLogDataOutput2 {
  size: i32,
  sensors: [AdlSingleSensorData; ADL_PMLOG_MAX_SENSORS],
}

impl Default for AdlPmLogDataOutput2 {
  fn default() -> Self {
    Self {
      size: 0,
      sensors: [AdlSingleSensorData::default(); ADL_PMLOG_MAX_SENSORS],
    }
  }
}

// ---------------------------------------------------------------------------
// C callback type for ADL memory allocation
// ---------------------------------------------------------------------------
type AdlMainMallocCallback = unsafe extern "C" fn(i32) -> *mut c_void;

/// ADL requires a malloc callback; we use Rust's global allocator.
///
/// This function must not panic or unwind across the `extern "C"` boundary.
/// It returns a null pointer for invalid or zero sizes, or if a valid
/// allocation layout cannot be constructed.
unsafe extern "C" fn adl_malloc(size: i32) -> *mut c_void {
  // Reject non-positive sizes to avoid absurd allocations and zero-sized layouts.
  if size <= 0 {
    return std::ptr::null_mut();
  }

  // Construct a layout; return null if the layout is invalid instead of panicking.
  let layout = match std::alloc::Layout::from_size_align(size as usize, 8) {
    Ok(layout) => layout,
    Err(_) => return std::ptr::null_mut(),
  };

  // `alloc` may return null on OOM; propagate that as-is.
  unsafe { std::alloc::alloc(layout) as *mut c_void }
}

// ---------------------------------------------------------------------------
// ADL function pointer types
// ---------------------------------------------------------------------------
type FnAdl2MainControlCreate =
  unsafe extern "C" fn(AdlMainMallocCallback, i32, *mut *mut c_void) -> i32;
type FnAdl2MainControlDestroy = unsafe extern "C" fn(*mut c_void) -> i32;
type FnAdl2AdapterNumberOfAdapters = unsafe extern "C" fn(*mut c_void, *mut i32) -> i32;
type FnAdl2AdapterAdapterInfoGet =
  unsafe extern "C" fn(*mut c_void, *mut AdlAdapterInfo, i32) -> i32;
type FnAdl2AdapterActiveGet = unsafe extern "C" fn(*mut c_void, i32, *mut i32) -> i32;
type FnAdl2OverdriveCaps =
  unsafe extern "C" fn(*mut c_void, i32, *mut i32, *mut i32, *mut i32) -> i32;
type FnAdl2Overdrive5OdParametersGet =
  unsafe extern "C" fn(*mut c_void, i32, *mut AdlOdParameters) -> i32;
type FnAdl2Overdrive5TemperatureGet =
  unsafe extern "C" fn(*mut c_void, i32, i32, *mut AdlTemperature) -> i32;
type FnAdl2Overdrive5CurrentActivityGet =
  unsafe extern "C" fn(*mut c_void, i32, *mut AdlPmActivity) -> i32;
type FnAdl2OverdriveNTemperatureGet =
  unsafe extern "C" fn(*mut c_void, i32, i32, *mut i32) -> i32;
type FnAdl2NewQueryPmLogDataGet =
  unsafe extern "C" fn(*mut c_void, i32, *mut AdlPmLogDataOutput2) -> i32;

// ---------------------------------------------------------------------------
// Loaded ADL instance (lazy singleton)
// ---------------------------------------------------------------------------
struct AdlLibrary {
  // Keep the library alive so function pointers remain valid.
  _lib: libloading::Library,
  context: *mut c_void,

  // Resolved function pointers
  adl2_main_control_destroy: FnAdl2MainControlDestroy,
  adl2_adapter_number_of_adapters: FnAdl2AdapterNumberOfAdapters,
  adl2_adapter_adapter_info_get: FnAdl2AdapterAdapterInfoGet,
  adl2_adapter_active_get: FnAdl2AdapterActiveGet,
  adl2_overdrive_caps: Option<FnAdl2OverdriveCaps>,
  adl2_overdrive5_od_parameters_get: Option<FnAdl2Overdrive5OdParametersGet>,
  adl2_overdrive5_temperature_get: Option<FnAdl2Overdrive5TemperatureGet>,
  adl2_overdrive5_current_activity_get: Option<FnAdl2Overdrive5CurrentActivityGet>,
  adl2_overdriven_temperature_get: Option<FnAdl2OverdriveNTemperatureGet>,
  adl2_new_query_pm_log_data_get: Option<FnAdl2NewQueryPmLogDataGet>,
}

// Pointers inside AdlLibrary are only used from spawn_blocking (same thread).
unsafe impl Send for AdlLibrary {}
unsafe impl Sync for AdlLibrary {}

impl Drop for AdlLibrary {
  fn drop(&mut self) {
    unsafe {
      (self.adl2_main_control_destroy)(self.context);
    }
  }
}

static ADL_INSTANCE: OnceLock<Option<AdlLibrary>> = OnceLock::new();

/// Try to load the ADL library and initialise the context.
/// Returns `None` when the DLL is absent (no AMD driver installed).
fn try_load_adl() -> Option<AdlLibrary> {
  unsafe {
    let lib = match libloading::Library::new("atiadlxx.dll") {
      Ok(l) => l,
      Err(_) => {
        log_debug!(
          "dll_not_found",
          "adl_provider::try_load_adl",
          Some("atiadlxx.dll not found – AMD ADL not available")
        );
        return None;
      }
    };

    // Resolve mandatory symbols
    let adl2_main_control_create: FnAdl2MainControlCreate =
      *lib.get(b"ADL2_Main_Control_Create\0").ok()?;
    let adl2_main_control_destroy: FnAdl2MainControlDestroy =
      *lib.get(b"ADL2_Main_Control_Destroy\0").ok()?;
    let adl2_adapter_number_of_adapters: FnAdl2AdapterNumberOfAdapters =
      *lib.get(b"ADL2_Adapter_NumberOfAdapters_Get\0").ok()?;
    let adl2_adapter_adapter_info_get: FnAdl2AdapterAdapterInfoGet =
      *lib.get(b"ADL2_Adapter_AdapterInfo_Get\0").ok()?;
    let adl2_adapter_active_get: FnAdl2AdapterActiveGet =
      *lib.get(b"ADL2_Adapter_Active_Get\0").ok()?;

    // Create ADL context (enumerate all adapters, including inactive)
    let mut context: *mut c_void = std::ptr::null_mut();
    let rc =
      adl2_main_control_create(adl_malloc, 1 /* enumerate all */, &mut context);
    if rc != ADL_OK {
      log_error!(
        "init_failed",
        "adl_provider::try_load_adl",
        Some(format!("ADL2_Main_Control_Create returned {rc}"))
      );
      return None;
    }

    // Resolve optional symbols (may not be present in older drivers)
    let adl2_overdrive_caps: Option<FnAdl2OverdriveCaps> =
      lib.get(b"ADL2_Overdrive_Caps\0").ok().map(|s| *s);
    let adl2_overdrive5_od_parameters_get: Option<FnAdl2Overdrive5OdParametersGet> = lib
      .get(b"ADL2_Overdrive5_ODParameters_Get\0")
      .ok()
      .map(|s| *s);
    let adl2_overdrive5_temperature_get: Option<FnAdl2Overdrive5TemperatureGet> = lib
      .get(b"ADL2_Overdrive5_Temperature_Get\0")
      .ok()
      .map(|s| *s);
    let adl2_overdrive5_current_activity_get: Option<FnAdl2Overdrive5CurrentActivityGet> =
      lib
        .get(b"ADL2_Overdrive5_CurrentActivity_Get\0")
        .ok()
        .map(|s| *s);
    let adl2_overdriven_temperature_get: Option<FnAdl2OverdriveNTemperatureGet> = lib
      .get(b"ADL2_OverdriveN_Temperature_Get\0")
      .ok()
      .map(|s| *s);
    let adl2_new_query_pm_log_data_get: Option<FnAdl2NewQueryPmLogDataGet> =
      lib.get(b"ADL2_New_QueryPMLogData_Get\0").ok().map(|s| *s);

    log_debug!(
      "loaded",
      "adl_provider::try_load_adl",
      Some("ADL library loaded successfully")
    );

    Some(AdlLibrary {
      _lib: lib,
      context,
      adl2_main_control_destroy,
      adl2_adapter_number_of_adapters,
      adl2_adapter_adapter_info_get,
      adl2_adapter_active_get,
      adl2_overdrive_caps,
      adl2_overdrive5_od_parameters_get,
      adl2_overdrive5_temperature_get,
      adl2_overdrive5_current_activity_get,
      adl2_overdriven_temperature_get,
      adl2_new_query_pm_log_data_get,
    })
  }
}

fn get_adl() -> Option<&'static AdlLibrary> {
  ADL_INSTANCE.get_or_init(try_load_adl).as_ref()
}

// ---------------------------------------------------------------------------
// Per-adapter state computed during enumerate
// ---------------------------------------------------------------------------
struct AdapterState {
  adapter_index: i32,
  adapter_name: String,
  overdrive_level: i32, // 5, 6, 7, 8, or -1
  overdrive_supported: bool,
}

/// Enumerate active AMD adapters and determine their Overdrive capabilities.
fn enumerate_adapters(adl: &AdlLibrary) -> Vec<AdapterState> {
  unsafe {
    let mut num_adapters: i32 = 0;
    if (adl.adl2_adapter_number_of_adapters)(adl.context, &mut num_adapters) != ADL_OK {
      return Vec::new();
    }
    if num_adapters <= 0 {
      return Vec::new();
    }

    let count = num_adapters as usize;
    let mut infos = vec![AdlAdapterInfo::default(); count];
    let buf_size = (count * std::mem::size_of::<AdlAdapterInfo>()) as i32;

    if (adl.adl2_adapter_adapter_info_get)(adl.context, infos.as_mut_ptr(), buf_size)
      != ADL_OK
    {
      return Vec::new();
    }

    let mut seen_buses = std::collections::HashSet::new();
    let mut result = Vec::new();

    for info in &infos {
      // Skip non-AMD adapters (vendor 1002h)
      if info.vendor_id != 0x1002 {
        continue;
      }

      // Deduplicate by bus/device/function
      let bus_key = (info.bus_number, info.device_number, info.function_number);
      if !seen_buses.insert(bus_key) {
        continue;
      }

      // Check if adapter is active
      let mut active: i32 = 0;
      if (adl.adl2_adapter_active_get)(adl.context, info.adapter_index, &mut active)
        != ADL_OK
        || active != ADL_TRUE
      {
        continue;
      }

      let adapter_name = String::from_utf8_lossy(&info.adapter_name)
        .trim_end_matches('\0')
        .to_string();

      // Determine Overdrive level
      let (od_level, od_supported) = determine_overdrive_level(adl, info.adapter_index);

      result.push(AdapterState {
        adapter_index: info.adapter_index,
        adapter_name,
        overdrive_level: od_level,
        overdrive_supported: od_supported,
      });
    }

    result
  }
}

/// Determine the Overdrive API level for an adapter (mirrors AmdGpu.cs §3.1).
fn determine_overdrive_level(adl: &AdlLibrary, adapter_index: i32) -> (i32, bool) {
  unsafe {
    // Try ADL2_Overdrive_Caps first
    if let Some(caps_fn) = adl.adl2_overdrive_caps {
      let mut supported: i32 = 0;
      let mut enabled: i32 = 0;
      let mut version: i32 = 0;
      let rc = caps_fn(
        adl.context,
        adapter_index,
        &mut supported,
        &mut enabled,
        &mut version,
      );
      if rc == ADL_OK {
        return (version, supported == ADL_TRUE);
      }
    }

    // Fallback: try OD5 parameters
    if let Some(od5_fn) = adl.adl2_overdrive5_od_parameters_get {
      let mut params = AdlOdParameters {
        size: std::mem::size_of::<AdlOdParameters>() as i32,
        ..Default::default()
      };
      let rc = od5_fn(adl.context, adapter_index, &mut params);
      if rc == ADL_OK && params.activity_reporting_supported > 0 {
        return (5, true);
      }
    }

    (-1, false)
  }
}

// ---------------------------------------------------------------------------
// Temperature reading helpers
// ---------------------------------------------------------------------------

/// OD5 temperature (GPU Core only, millidegrees → degrees).
fn read_od5_temperature(adl: &AdlLibrary, adapter_index: i32) -> Option<i32> {
  let func = adl.adl2_overdrive5_temperature_get?;
  let mut temp = AdlTemperature {
    size: std::mem::size_of::<AdlTemperature>() as i32,
    ..Default::default()
  };
  let rc = unsafe { func(adl.context, adapter_index, 0, &mut temp) };
  if rc == ADL_OK {
    Some((temp.temperature as f32 * 0.001) as i32) // millidegrees → °C
  } else {
    None
  }
}

/// ODN temperature for a given sensor type.
fn read_odn_temperature(
  adl: &AdlLibrary,
  adapter_index: i32,
  temp_type: i32,
) -> Option<i32> {
  let func = adl.adl2_overdriven_temperature_get?;
  let mut temp_raw: i32 = 0;
  let rc = unsafe { func(adl.context, adapter_index, temp_type, &mut temp_raw) };
  if rc != ADL_OK {
    return None;
  }

  // EDGE returns millidegrees; others return degrees directly
  let (scale, min_temp, max_temp) = if temp_type == ODNT_EDGE {
    (0.001_f32, -256_000, 256_000)
  } else {
    (1.0_f32, -256, 256)
  };

  if temp_raw < min_temp || temp_raw > max_temp {
    return None; // Anomalous value guard (e.g. 54000°C)
  }

  Some((temp_raw as f32 * scale) as i32)
}

/// Read a temperature sensor from OD8 polling API.
fn read_od8_sensor_value(
  adl: &AdlLibrary,
  adapter_index: i32,
  sensor_id: u32,
) -> Option<i32> {
  let func = adl.adl2_new_query_pm_log_data_get?;
  let mut output = AdlPmLogDataOutput2 {
    size: std::mem::size_of::<AdlPmLogDataOutput2>() as i32,
    ..Default::default()
  };
  let rc = unsafe { func(adl.context, adapter_index, &mut output) };
  if rc != ADL_OK {
    return None;
  }

  let idx = sensor_id as usize;
  if idx < output.sensors.len() && output.sensors[idx].supported != 0 {
    Some(output.sensors[idx].value)
  } else {
    None
  }
}

// ---------------------------------------------------------------------------
// Usage reading helpers
// ---------------------------------------------------------------------------

/// OD5 GPU core usage (%).
fn read_od5_activity(adl: &AdlLibrary, adapter_index: i32) -> Option<f32> {
  let func = adl.adl2_overdrive5_current_activity_get?;
  let mut activity = AdlPmActivity {
    size: std::mem::size_of::<AdlPmActivity>() as i32,
    ..Default::default()
  };
  let rc = unsafe { func(adl.context, adapter_index, &mut activity) };
  if rc == ADL_OK {
    let usage = activity.activity_percent.min(100); // clamp to 100
    Some(usage as f32)
  } else {
    None
  }
}

/// Read GPU core usage from OD8 polling API.
fn read_od8_usage(adl: &AdlLibrary, adapter_index: i32) -> Option<f32> {
  read_od8_sensor_value(adl, adapter_index, PMLOG_INFO_ACTIVITY_GFX)
    .map(|v| (v as f32).min(100.0))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Collect temperatures from all active AMD GPU adapters.
///
/// Returns a list of `NameValue` where `name` is
/// `"<AdapterName> <SensorLabel>"` and `value` is in °C.
pub async fn get_amd_gpu_temperatures() -> Result<Vec<NameValue>, String> {
  spawn_blocking(|| {
    let adl = get_adl().ok_or("AMD ADL library not available")?;
    let adapters = enumerate_adapters(adl);

    if adapters.is_empty() {
      return Err("No active AMD GPU adapters found".to_string());
    }

    let mut temps: Vec<NameValue> = Vec::new();

    for adapter in &adapters {
      let idx = adapter.adapter_index;
      let name = &adapter.adapter_name;

      // ── Phase 1: OD5 baseline (GPU Core) ──
      let mut core_temp: Option<i32> = None;
      if adapter.overdrive_supported {
        core_temp = read_od5_temperature(adl, idx);
      }

      // ── Phase 2: ODN override (OD ≥ 7) ──
      struct SensorSpec {
        odn_type: i32,
        label: &'static str,
        reset: bool,
      }

      let odn_sensors = [
        SensorSpec {
          odn_type: ODNT_EDGE,
          label: "Core",
          reset: false,
        },
        SensorSpec {
          odn_type: ODNT_MEM,
          label: "Memory",
          reset: true,
        },
        SensorSpec {
          odn_type: ODNT_VRVDDC,
          label: "VR VDDC",
          reset: true,
        },
        SensorSpec {
          odn_type: ODNT_VRMVDD,
          label: "VR MVDD",
          reset: true,
        },
        SensorSpec {
          odn_type: ODNT_LIQUID,
          label: "Liquid",
          reset: true,
        },
        SensorSpec {
          odn_type: ODNT_PLX,
          label: "PLX",
          reset: true,
        },
        SensorSpec {
          odn_type: ODNT_HOTSPOT,
          label: "Hot Spot",
          reset: true,
        },
      ];

      // temp_values[i] corresponds to odn_sensors[i]
      let mut temp_values: Vec<Option<i32>> = vec![core_temp]; // start with Core
      for _ in 1..odn_sensors.len() {
        temp_values.push(None);
      }

      if adapter.overdrive_supported && adapter.overdrive_level >= 7 {
        for (i, spec) in odn_sensors.iter().enumerate() {
          if let Some(v) = read_odn_temperature(adl, idx, spec.odn_type) {
            temp_values[i] = Some(v);
          } else if spec.reset {
            temp_values[i] = None;
          }
          // reset=false → keep previous value
        }
      }

      // ── Phase 3: PMLog / OD8 override (OD ≥ 8 or !supported) ──
      struct PmlogSensorSpec {
        pmlog_id: u32,
        odn_index: Option<usize>, // index into odn_sensors / temp_values
        reset: bool,
      }

      let pmlog_sensors = [
        PmlogSensorSpec {
          pmlog_id: PMLOG_TEMPERATURE_EDGE,
          odn_index: Some(0),
          reset: false,
        },
        PmlogSensorSpec {
          pmlog_id: PMLOG_TEMPERATURE_MEM,
          odn_index: Some(1),
          reset: false,
        },
        PmlogSensorSpec {
          pmlog_id: PMLOG_TEMPERATURE_VRVDDC,
          odn_index: Some(2),
          reset: false,
        },
        PmlogSensorSpec {
          pmlog_id: PMLOG_TEMPERATURE_VRMVDD,
          odn_index: Some(3),
          reset: false,
        },
        PmlogSensorSpec {
          pmlog_id: PMLOG_TEMPERATURE_LIQUID,
          odn_index: Some(4),
          reset: false,
        },
        PmlogSensorSpec {
          pmlog_id: PMLOG_TEMPERATURE_PLX,
          odn_index: Some(5),
          reset: false,
        },
        PmlogSensorSpec {
          pmlog_id: PMLOG_TEMPERATURE_HOTSPOT,
          odn_index: Some(6),
          reset: false,
        },
        PmlogSensorSpec {
          pmlog_id: PMLOG_TEMPERATURE_SOC,
          odn_index: None,
          reset: true,
        },
      ];

      // SoC temperature (PMLog-only, not in odn_sensors)
      let mut soc_temp: Option<i32> = None;

      if adapter.overdrive_level >= 8 || !adapter.overdrive_supported {
        for spec in &pmlog_sensors {
          let value = read_od8_sensor_value(adl, idx, spec.pmlog_id);
          match spec.odn_index {
            Some(odn_i) => {
              if let Some(v) = value {
                temp_values[odn_i] = Some(v);
              }
              // reset=false for all shared sensors → keep previous value
            }
            None => {
              // SoC
              if let Some(v) = value {
                soc_temp = Some(v);
              } else if spec.reset {
                soc_temp = None;
              }
            }
          }
        }
      }

      // ── Collect results ──
      for (i, spec) in odn_sensors.iter().enumerate() {
        if let Some(v) = temp_values[i] {
          temps.push(NameValue {
            name: format!("{name} {}", spec.label),
            value: v,
          });
        }
      }
      if let Some(v) = soc_temp {
        temps.push(NameValue {
          name: format!("{name} SoC"),
          value: v,
        });
      }
    }

    if temps.is_empty() {
      Err("AMD ADL: no temperature sensors returned".to_string())
    } else {
      Ok(temps)
    }
  })
  .await
  .map_err(|e| {
    log_error!(
      "join_error",
      "adl_provider::get_amd_gpu_temperatures",
      Some(e.to_string())
    );
    "AMD GPU temperature retrieval failed".to_string()
  })?
}

/// Get the average GPU core usage across all active AMD adapters.
///
/// Fallback chain: OD5 Activity → OD8 PMLog usage → error.
pub async fn get_amd_gpu_usage() -> Result<f32, String> {
  spawn_blocking(|| {
    let adl = get_adl().ok_or("AMD ADL library not available")?;
    let adapters = enumerate_adapters(adl);

    if adapters.is_empty() {
      return Err("No active AMD GPU adapters found".to_string());
    }

    let mut total: f32 = 0.0;
    let mut count = 0u32;

    for adapter in &adapters {
      let idx = adapter.adapter_index;
      let mut usage: Option<f32> = None;

      // Phase 1: OD5 baseline
      if adapter.overdrive_supported {
        usage = read_od5_activity(adl, idx);
      }

      // Phase 2: OD8/PMLog override (OD ≥ 8 or !supported)
      // reset=false → keep OD5 value if OD8 fails
      if (adapter.overdrive_level >= 8 || !adapter.overdrive_supported)
        && let Some(v) = read_od8_usage(adl, idx)
      {
        usage = Some(v);
      }

      if let Some(u) = usage {
        total += u;
        count += 1;
      }
    }

    if count == 0 {
      Err("AMD ADL: no usage data collected".to_string())
    } else {
      Ok(total / count as f32)
    }
  })
  .await
  .map_err(|e| {
    log_error!(
      "join_error",
      "adl_provider::get_amd_gpu_usage",
      Some(e.to_string())
    );
    "AMD GPU usage retrieval failed".to_string()
  })?
}

/// Check whether the ADL library is available (AMD driver installed).
pub fn is_available() -> bool {
  get_adl().is_some()
}
