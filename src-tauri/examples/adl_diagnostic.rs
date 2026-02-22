/// ADL Diagnostic Tool
///
/// Standalone diagnostic that loads atiadlxx.dll and checks each ADL step
/// to identify where GPU value retrieval fails.
///
/// Run with: cargo run --example adl_diagnostic
use std::ffi::c_void;

const ADL_OK: i32 = 0;
const ADL_TRUE: i32 = 1;
const ADL_MAX_PATH: usize = 256;
const ADL_PMLOG_MAX_SENSORS: usize = 256;

// OverdriveN temperature types
const ODNT_EDGE: i32 = 1;
const ODNT_MEM: i32 = 2;
const ODNT_VRVDDC: i32 = 3;
const ODNT_VRMVDD: i32 = 4;
const ODNT_LIQUID: i32 = 5;
const ODNT_PLX: i32 = 6;
const ODNT_HOTSPOT: i32 = 7;

// PMLog sensor IDs
const PMLOG_TEMPERATURE_EDGE: u32 = 8;
const PMLOG_TEMPERATURE_MEM: u32 = 9;
const PMLOG_TEMPERATURE_VRVDDC: u32 = 10;
const PMLOG_TEMPERATURE_VRMVDD: u32 = 11;
const PMLOG_TEMPERATURE_LIQUID: u32 = 12;
const PMLOG_TEMPERATURE_PLX: u32 = 13;
const PMLOG_TEMPERATURE_HOTSPOT: u32 = 27;
const PMLOG_TEMPERATURE_SOC: u32 = 29;
const PMLOG_INFO_ACTIVITY_GFX: u32 = 19;

// ---------------------------------------------------------------------------
// ADL C structures
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
  temperature: i32,
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

type AdlMainMallocCallback = unsafe extern "C" fn(i32) -> *mut c_void;
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

unsafe extern "C" fn adl_malloc(size: i32) -> *mut c_void {
  if size <= 0 {
    return std::ptr::null_mut();
  }
  let layout = match std::alloc::Layout::from_size_align(size as usize, 8) {
    Ok(layout) => layout,
    Err(_) => return std::ptr::null_mut(),
  };
  unsafe { std::alloc::alloc(layout) as *mut c_void }
}

fn main() {
  println!("=== ADL Diagnostic Tool ===\n");

  unsafe {
    // Step 1: Load DLL
    println!("[Step 1] Loading atiadlxx.dll...");
    let lib = match libloading::Library::new("atiadlxx.dll") {
      Ok(l) => {
        println!("  OK: DLL loaded successfully");
        l
      }
      Err(e) => {
        println!("  FAIL: Could not load atiadlxx.dll: {e}");
        println!("\n  -> AMD driver is not installed or atiadlxx.dll is missing.");
        return;
      }
    };

    // Step 2: Resolve mandatory symbols
    println!("\n[Step 2] Resolving mandatory symbols...");
    let adl2_main_control_create: FnAdl2MainControlCreate =
      match lib.get(b"ADL2_Main_Control_Create\0") {
        Ok(s) => {
          println!("  OK: ADL2_Main_Control_Create");
          *s
        }
        Err(e) => {
          println!("  FAIL: ADL2_Main_Control_Create: {e}");
          return;
        }
      };
    let adl2_main_control_destroy: FnAdl2MainControlDestroy =
      match lib.get(b"ADL2_Main_Control_Destroy\0") {
        Ok(s) => {
          println!("  OK: ADL2_Main_Control_Destroy");
          *s
        }
        Err(e) => {
          println!("  FAIL: ADL2_Main_Control_Destroy: {e}");
          return;
        }
      };
    let adl2_adapter_number: FnAdl2AdapterNumberOfAdapters =
      match lib.get(b"ADL2_Adapter_NumberOfAdapters_Get\0") {
        Ok(s) => {
          println!("  OK: ADL2_Adapter_NumberOfAdapters_Get");
          *s
        }
        Err(e) => {
          println!("  FAIL: ADL2_Adapter_NumberOfAdapters_Get: {e}");
          return;
        }
      };
    let adl2_adapter_info: FnAdl2AdapterAdapterInfoGet =
      match lib.get(b"ADL2_Adapter_AdapterInfo_Get\0") {
        Ok(s) => {
          println!("  OK: ADL2_Adapter_AdapterInfo_Get");
          *s
        }
        Err(e) => {
          println!("  FAIL: ADL2_Adapter_AdapterInfo_Get: {e}");
          return;
        }
      };
    let adl2_adapter_active: FnAdl2AdapterActiveGet =
      match lib.get(b"ADL2_Adapter_Active_Get\0") {
        Ok(s) => {
          println!("  OK: ADL2_Adapter_Active_Get");
          *s
        }
        Err(e) => {
          println!("  FAIL: ADL2_Adapter_Active_Get: {e}");
          return;
        }
      };

    // Step 3: Resolve optional symbols
    println!("\n[Step 3] Resolving optional symbols...");
    let adl2_overdrive_caps: Option<FnAdl2OverdriveCaps> =
      lib.get(b"ADL2_Overdrive_Caps\0").ok().map(|s| *s);
    println!(
      "  ADL2_Overdrive_Caps: {}",
      if adl2_overdrive_caps.is_some() {
        "OK"
      } else {
        "NOT FOUND"
      }
    );

    let adl2_od5_params: Option<FnAdl2Overdrive5OdParametersGet> = lib
      .get(b"ADL2_Overdrive5_ODParameters_Get\0")
      .ok()
      .map(|s| *s);
    println!(
      "  ADL2_Overdrive5_ODParameters_Get: {}",
      if adl2_od5_params.is_some() {
        "OK"
      } else {
        "NOT FOUND"
      }
    );

    let adl2_od5_temp: Option<FnAdl2Overdrive5TemperatureGet> = lib
      .get(b"ADL2_Overdrive5_Temperature_Get\0")
      .ok()
      .map(|s| *s);
    println!(
      "  ADL2_Overdrive5_Temperature_Get: {}",
      if adl2_od5_temp.is_some() {
        "OK"
      } else {
        "NOT FOUND"
      }
    );

    let adl2_od5_activity: Option<FnAdl2Overdrive5CurrentActivityGet> = lib
      .get(b"ADL2_Overdrive5_CurrentActivity_Get\0")
      .ok()
      .map(|s| *s);
    println!(
      "  ADL2_Overdrive5_CurrentActivity_Get: {}",
      if adl2_od5_activity.is_some() {
        "OK"
      } else {
        "NOT FOUND"
      }
    );

    let adl2_odn_temp: Option<FnAdl2OverdriveNTemperatureGet> = lib
      .get(b"ADL2_OverdriveN_Temperature_Get\0")
      .ok()
      .map(|s| *s);
    println!(
      "  ADL2_OverdriveN_Temperature_Get: {}",
      if adl2_odn_temp.is_some() {
        "OK"
      } else {
        "NOT FOUND"
      }
    );

    let adl2_pmlog: Option<FnAdl2NewQueryPmLogDataGet> =
      lib.get(b"ADL2_New_QueryPMLogData_Get\0").ok().map(|s| *s);
    println!(
      "  ADL2_New_QueryPMLogData_Get: {}",
      if adl2_pmlog.is_some() {
        "OK"
      } else {
        "NOT FOUND"
      }
    );

    // Step 4: Create ADL context
    println!("\n[Step 4] Creating ADL context (ADL2_Main_Control_Create)...");
    let mut context: *mut c_void = std::ptr::null_mut();
    let rc = adl2_main_control_create(adl_malloc, 1, &mut context);
    if rc != ADL_OK {
      println!("  FAIL: ADL2_Main_Control_Create returned {rc}");
      println!("  -> ADL initialization failed. Common causes:");
      println!("     - Driver incompatibility");
      println!("     - Insufficient permissions");
      return;
    }
    println!("  OK: Context created (rc={rc})");

    // Step 5: Enumerate adapters
    println!("\n[Step 5] Enumerating adapters...");
    let mut num_adapters: i32 = 0;
    let rc = adl2_adapter_number(context, &mut num_adapters);
    if rc != ADL_OK {
      println!("  FAIL: ADL2_Adapter_NumberOfAdapters_Get returned {rc}");
      adl2_main_control_destroy(context);
      return;
    }
    println!("  Total adapters reported: {num_adapters}");

    if num_adapters <= 0 {
      println!("  FAIL: No adapters found");
      adl2_main_control_destroy(context);
      return;
    }

    let count = num_adapters as usize;
    let mut infos = vec![AdlAdapterInfo::default(); count];
    let buf_size = (count * std::mem::size_of::<AdlAdapterInfo>()) as i32;
    let rc = adl2_adapter_info(context, infos.as_mut_ptr(), buf_size);
    if rc != ADL_OK {
      println!("  FAIL: ADL2_Adapter_AdapterInfo_Get returned {rc}");
      adl2_main_control_destroy(context);
      return;
    }
    println!("  OK: Adapter info retrieved");

    // Step 6: Check each adapter
    println!("\n[Step 6] Checking each adapter...");
    let mut seen_buses = std::collections::HashSet::new();
    let mut amd_adapter_found = false;

    for (i, info) in infos.iter().enumerate() {
      let name = String::from_utf8_lossy(&info.adapter_name)
        .trim_end_matches('\0')
        .to_string();
      let display = String::from_utf8_lossy(&info.display_name)
        .trim_end_matches('\0')
        .to_string();

      println!("\n  --- Adapter #{i} ---");
      println!("    Name: {name}");
      println!("    Display: {display}");
      println!("    Index: {}", info.adapter_index);
      println!("    Vendor ID: 0x{:04X}", info.vendor_id);
      println!(
        "    Bus/Dev/Func: {}/{}/{}",
        info.bus_number, info.device_number, info.function_number
      );
      println!("    Present: {}, Exist: {}", info.present, info.exist);

      // Check vendor
      let is_amd_vendor = info.vendor_id == 0x1002 || info.vendor_id == 0x03EA;
      let is_amd_name = name.contains("AMD") || name.contains("Radeon");
      if !is_amd_vendor && !is_amd_name {
        println!(
          "    -> Skipped: Not AMD (vendor=0x{:04X}, name doesn't match)",
          info.vendor_id
        );
        continue;
      }

      // Deduplicate
      let bus_key = (info.bus_number, info.device_number, info.function_number);
      if !seen_buses.insert(bus_key) {
        println!("    -> Skipped: Duplicate bus/dev/func");
        continue;
      }

      // Active check
      let mut active: i32 = 0;
      let rc = adl2_adapter_active(context, info.adapter_index, &mut active);
      println!("    Active check: rc={rc}, active={active}");
      if rc != ADL_OK || active != ADL_TRUE {
        println!("    -> Skipped: Adapter not active (rc={rc}, active={active})");
        continue;
      }

      amd_adapter_found = true;
      let idx = info.adapter_index;

      // Step 7: Overdrive capabilities
      println!("\n    [Overdrive Capabilities]");
      let mut od_level = -1i32;
      let mut od_supported = false;

      if let Some(caps_fn) = adl2_overdrive_caps {
        let mut supported: i32 = 0;
        let mut enabled: i32 = 0;
        let mut version: i32 = 0;
        let rc = caps_fn(context, idx, &mut supported, &mut enabled, &mut version);
        println!(
          "    ADL2_Overdrive_Caps: rc={rc}, supported={supported}, enabled={enabled}, version={version}"
        );
        if rc == ADL_OK {
          od_level = version;
          od_supported = supported == ADL_TRUE;
        }
      } else {
        println!("    ADL2_Overdrive_Caps: not available");
      }

      // Fallback: OD5 parameters
      if od_level == -1 {
        if let Some(od5_fn) = adl2_od5_params {
          let mut params = AdlOdParameters {
            size: std::mem::size_of::<AdlOdParameters>() as i32,
            ..Default::default()
          };
          let rc = od5_fn(context, idx, &mut params);
          println!(
            "    OD5 params fallback: rc={rc}, activity_reporting={}, perf_levels={}",
            params.activity_reporting_supported, params.number_of_performance_levels
          );
          if rc == ADL_OK && params.activity_reporting_supported > 0 {
            od_level = 5;
            od_supported = true;
          }
        }
      }

      println!("    => Overdrive level: {od_level}, Supported: {od_supported}");

      // Step 8: OD5 Temperature
      println!("\n    [OD5 Temperature]");
      if let Some(temp_fn) = adl2_od5_temp {
        let mut temp = AdlTemperature {
          size: std::mem::size_of::<AdlTemperature>() as i32,
          ..Default::default()
        };
        let rc = temp_fn(context, idx, 0, &mut temp);
        if rc == ADL_OK {
          let celsius = temp.temperature as f64 * 0.001;
          println!(
            "    OD5 Temperature: rc={rc}, raw={}, celsius={celsius:.1}°C",
            temp.temperature
          );
        } else {
          println!("    OD5 Temperature: rc={rc} (FAILED)");
        }
      } else {
        println!("    OD5 Temperature: function not available");
      }

      // Step 9: OD5 Activity (Usage)
      println!("\n    [OD5 Activity/Usage]");
      if let Some(act_fn) = adl2_od5_activity {
        let mut activity = AdlPmActivity {
          size: std::mem::size_of::<AdlPmActivity>() as i32,
          ..Default::default()
        };
        let rc = act_fn(context, idx, &mut activity);
        if rc == ADL_OK {
          println!(
            "    OD5 Activity: rc={rc}, usage={}%, engine_clk={}, mem_clk={}, vddc={}",
            activity.activity_percent,
            activity.engine_clock,
            activity.memory_clock,
            activity.vddc
          );
        } else {
          println!("    OD5 Activity: rc={rc} (FAILED)");
        }
      } else {
        println!("    OD5 Activity: function not available");
      }

      // Step 10: ODN Temperatures
      println!("\n    [ODN Temperatures (Overdrive N, OD >= 7)]");
      if od_level >= 7 {
        let odn_sensors = [
          (ODNT_EDGE, "Edge/Core"),
          (ODNT_MEM, "Memory"),
          (ODNT_VRVDDC, "VR VDDC"),
          (ODNT_VRMVDD, "VR MVDD"),
          (ODNT_LIQUID, "Liquid"),
          (ODNT_PLX, "PLX"),
          (ODNT_HOTSPOT, "Hot Spot"),
        ];
        if let Some(odn_fn) = adl2_odn_temp {
          for (temp_type, label) in &odn_sensors {
            let mut temp_raw: i32 = 0;
            let rc = odn_fn(context, idx, *temp_type, &mut temp_raw);
            if rc == ADL_OK {
              let scale = if *temp_type == ODNT_EDGE { 0.001 } else { 1.0 };
              let celsius = temp_raw as f64 * scale;
              println!(
                "    ODN {label}: rc={rc}, raw={temp_raw}, celsius={celsius:.1}°C"
              );
            } else {
              println!("    ODN {label}: rc={rc} (FAILED)");
            }
          }
        } else {
          println!("    ODN Temperature function not available");
        }
      } else {
        println!("    Skipped: Overdrive level ({od_level}) < 7");
      }

      // Step 11: PMLog / OD8 Sensors
      println!("\n    [PMLog / OD8 Sensors]");
      if let Some(pmlog_fn) = adl2_pmlog {
        let mut output = AdlPmLogDataOutput2 {
          size: std::mem::size_of::<AdlPmLogDataOutput2>() as i32,
          ..Default::default()
        };
        let rc = pmlog_fn(context, idx, &mut output);
        if rc == ADL_OK {
          println!("    PMLog query: rc={rc} (OK)");

          let sensors_to_check = [
            (PMLOG_TEMPERATURE_EDGE, "Temp Edge"),
            (PMLOG_TEMPERATURE_MEM, "Temp Memory"),
            (PMLOG_TEMPERATURE_VRVDDC, "Temp VR VDDC"),
            (PMLOG_TEMPERATURE_VRMVDD, "Temp VR MVDD"),
            (PMLOG_TEMPERATURE_LIQUID, "Temp Liquid"),
            (PMLOG_TEMPERATURE_PLX, "Temp PLX"),
            (PMLOG_TEMPERATURE_HOTSPOT, "Temp Hot Spot"),
            (PMLOG_TEMPERATURE_SOC, "Temp SoC"),
            (PMLOG_INFO_ACTIVITY_GFX, "GPU Usage (GFX)"),
          ];

          for (sensor_id, label) in &sensors_to_check {
            let i = *sensor_id as usize;
            if i < output.sensors.len() {
              let s = &output.sensors[i];
              if s.supported != 0 {
                println!(
                  "    PMLog [{sensor_id:>2}] {label}: supported={}, value={}",
                  s.supported, s.value
                );
              } else {
                println!(
                  "    PMLog [{sensor_id:>2}] {label}: NOT SUPPORTED (supported=0)"
                );
              }
            } else {
              println!("    PMLog [{sensor_id:>2}] {label}: index out of range");
            }
          }

          // Also print all supported sensors
          println!("\n    [All supported PMLog sensors]");
          let mut any_supported = false;
          for (i, s) in output.sensors.iter().enumerate() {
            if s.supported != 0 {
              println!(
                "    PMLog [{i:>3}]: supported={}, value={}",
                s.supported, s.value
              );
              any_supported = true;
            }
          }
          if !any_supported {
            println!("    -> No PMLog sensors are supported for this adapter!");
          }
        } else {
          println!("    PMLog query: rc={rc} (FAILED)");
          println!(
            "    -> ADL2_New_QueryPMLogData_Get is not supported for this adapter"
          );
        }
      } else {
        println!("    PMLog function not available in DLL");
      }
    }

    if !amd_adapter_found {
      println!("\n  => No active AMD adapters found!");
      println!("     The ADL DLL loaded but no AMD GPU adapter passed the active check.");
    }

    // Summary
    println!("\n=== Diagnostic Summary ===");
    println!("GPU: AMD Radeon detected");
    println!("DLL: atiadlxx.dll loaded");
    println!("Active AMD adapters found: {amd_adapter_found}");

    // Cleanup
    adl2_main_control_destroy(context);
    println!("\nDone.");
  }
}
