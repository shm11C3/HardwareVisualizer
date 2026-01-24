use crate::infrastructure::providers::macos::{
  io_kit::iokit_info, system_profiler_displays,
};
use crate::models;
use crate::utils::formatter;
use crate::{log_error, log_internal};
use tauri::async_runtime;

pub async fn get_gpu_info() -> Result<Vec<models::hardware::GraphicInfo>, String> {
  log_internal!(debug, "start", "macos_get_gpu_info", None::<&str>);

  // Primary: IOKit (blocking)
  let iokit_res = async_runtime::spawn_blocking(iokit_info::get_gpus_from_iokit)
    .await
    .map_err(|e| format!("Failed to join IOKit GPU task: {e}"))?;

  if let Ok(list) = iokit_res {
    log_internal!(
      debug,
      "iokit_ok",
      "macos_get_gpu_info",
      Some(format!("count={}", list.len()))
    );

    let mut mapped: Vec<models::hardware::GraphicInfo> = list
      .into_iter()
      .map(|g| models::hardware::GraphicInfo {
        id: format!("0x{:016x}", g.registry_id),
        name: g.name.unwrap_or_else(|| "Unknown GPU".to_string()),
        vendor_name: vendor_name_from_vendor_id(g.vendor_id),
        clock: 0,
        memory_size: "N/A".to_string(),
        memory_size_dedicated: "N/A".to_string(),
        core_count: g.core_count.map(|v| v.to_string()),
      })
      .collect();

    // Best-effort enrichment (optional): use system_profiler to fill vendor/memory strings.
    // This still keeps IOKit as the primary source (IDs stay registry-id based).
    if !mapped.is_empty()
      && let Ok(Ok(sp_list)) = async_runtime::spawn_blocking(|| {
        let raw = system_profiler_displays::get_raw_spdisplays_json()?;
        system_profiler_displays::parse_spdisplays_json(&raw)
      })
      .await
    {
      for (i, sp) in sp_list.into_iter().enumerate() {
        let Some(info) = mapped.get_mut(i) else {
          break;
        };

        if info.vendor_name == "Unknown" {
          if let Some(v) = sp.vendor_name.as_deref() {
            info.vendor_name = vendor_name_from_string(v);
          } else {
            info.vendor_name = vendor_name_from_string(&info.name);
          }
        }

        if info.memory_size == "N/A"
          && let Some(v) = sp.vram_shared.clone().or_else(|| sp.vram.clone())
        {
          info.memory_size = v;
        }

        if info.memory_size_dedicated == "N/A"
          && let Some(v) = sp.vram
        {
          info.memory_size_dedicated = v;
        }
      }
    }

    for info in &mapped {
      log_internal!(
        debug,
        "gpu",
        "macos_get_gpu_info",
        Some(format!(
          "id={} name={} vendor={} mem={} mem_dedicated={}",
          info.id,
          info.name,
          info.vendor_name,
          info.memory_size,
          info.memory_size_dedicated
        ))
      );
    }

    if !mapped.is_empty() {
      log_internal!(
        debug,
        "end",
        "macos_get_gpu_info",
        Some(format!("source=iokit count={}", mapped.len()))
      );
      return Ok(mapped);
    }
  } else if let Err(e) = iokit_res {
    log_error!("iokit_failed", "macos_get_gpu_info", Some(e));
  }

  // Fallback: system_profiler SPDisplaysDataType (blocking)
  let sp_res = async_runtime::spawn_blocking(|| {
    let raw = system_profiler_displays::get_raw_spdisplays_json()?;
    system_profiler_displays::parse_spdisplays_json(&raw)
  })
  .await
  .map_err(|e| format!("Failed to join system_profiler GPU task: {e}"))?;

  let sp_list = match sp_res {
    Ok(v) => {
      log_internal!(
        debug,
        "system_profiler_ok",
        "macos_get_gpu_info",
        Some(format!("count={}", v.len()))
      );
      v
    }
    Err(e) => {
      log_error!(
        "system_profiler_failed",
        "macos_get_gpu_info",
        Some(e.clone())
      );
      Vec::new()
    }
  };
  let mapped: Vec<models::hardware::GraphicInfo> = sp_list
    .into_iter()
    .enumerate()
    .map(|(i, g)| {
      let name = g.name.unwrap_or_else(|| "Unknown GPU".to_string());
      let vendor_name = g
        .vendor_name
        .as_deref()
        .map(vendor_name_from_string)
        .unwrap_or_else(|| vendor_name_from_string(&name));

      let memory_size = g
        .vram_shared
        .or_else(|| g.vram.clone())
        .unwrap_or_else(|| "N/A".to_string());
      let memory_size_dedicated = g.vram.unwrap_or_else(|| "N/A".to_string());

      models::hardware::GraphicInfo {
        id: format!("spdisplay-{i}"),
        name,
        vendor_name,
        clock: 0,
        memory_size,
        memory_size_dedicated,
        core_count: None,
      }
    })
    .collect();

  if mapped.is_empty() {
    // Avoid returning `None`/empty list to the frontend.
    log_internal!(
      debug,
      "end",
      "macos_get_gpu_info",
      Some("source=placeholder count=1".to_string())
    );
    Ok(vec![unknown_gpu_placeholder()])
  } else {
    log_internal!(
      debug,
      "end",
      "macos_get_gpu_info",
      Some(format!("source=system_profiler count={}", mapped.len()))
    );
    Ok(mapped)
  }
}

pub async fn get_gpu_memory_usage()
-> Result<Option<models::hardware::GpuMemoryUsage>, String> {
  let res = async_runtime::spawn_blocking(iokit_info::get_gpu_memory_usage_from_iokit)
    .await
    .map_err(|e| format!("Failed to join IOKit GPU memory task: {e}"))?;

  Ok(res.map(|usage| {
    models::hardware::GpuMemoryUsage {
      in_use_bytes: usage
        .in_use_bytes
        .map(|bytes| formatter::format_size(bytes, 1)),
      alloc_bytes: usage
        .alloc_bytes
        .map(|bytes| formatter::format_size(bytes, 1)),
    }
  }))
}

fn unknown_gpu_placeholder() -> models::hardware::GraphicInfo {
  models::hardware::GraphicInfo {
    id: "macos-unknown-gpu".to_string(),
    name: "Unknown GPU".to_string(),
    vendor_name: "Unknown".to_string(),
    clock: 0,
    memory_size: "N/A".to_string(),
    memory_size_dedicated: "N/A".to_string(),
    core_count: None,
  }
}

fn vendor_name_from_vendor_id(vendor_id: Option<u32>) -> String {
  match vendor_id {
    Some(0x10de) => "NVIDIA".to_string(),
    Some(0x1002) => "AMD".to_string(),
    Some(0x8086) => "Intel".to_string(),
    Some(0x106b) => "Apple".to_string(),
    _ => "Unknown".to_string(),
  }
}

fn vendor_name_from_string(s: &str) -> String {
  let x = s.to_ascii_lowercase();
  if x.contains("nvidia") {
    "NVIDIA".to_string()
  } else if x.contains("amd") || x.contains("radeon") {
    "AMD".to_string()
  } else if x.contains("intel") {
    "Intel".to_string()
  } else if x.contains("apple")
    || x.contains("agx")
    || x.contains(" m1")
    || x.contains(" m2")
    || x.contains(" m3")
    || x.contains(" m4")
  {
    "Apple".to_string()
  } else {
    "Unknown".to_string()
  }
}
