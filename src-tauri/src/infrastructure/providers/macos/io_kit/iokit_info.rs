use std::ffi::CString;

use core_foundation::{
  base::{CFRelease, CFTypeID, CFTypeRef, TCFType, kCFAllocatorDefault},
  string::{CFString, CFStringRef},
};

type KernReturn = i32;
type MachPort = u32;
type IoObject = u32;
type IoIterator = IoObject;
type IoRegistryEntry = IoObject;

const KERN_SUCCESS: KernReturn = 0;
const K_IOMASTER_PORT_DEFAULT: MachPort = 0;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
  fn IOServiceMatching(name: *const i8) -> CFTypeRef;

  fn IOServiceGetMatchingServices(
    master_port: MachPort,
    matching: CFTypeRef,
    existing: *mut IoIterator,
  ) -> KernReturn;

  fn IOIteratorNext(iterator: IoIterator) -> IoObject;

  fn IOObjectRelease(object: IoObject) -> KernReturn;

  fn IORegistryEntryCreateCFProperty(
    entry: IoRegistryEntry,
    key: CFStringRef,
    allocator: core_foundation::base::CFAllocatorRef,
    options: u32,
  ) -> CFTypeRef;

  fn IORegistryEntryGetRegistryEntryID(
    entry: IoRegistryEntry,
    entry_id: *mut u64,
  ) -> KernReturn;

  fn IORegistryEntryGetParentEntry(
    entry: IoRegistryEntry,
    plane: CFStringRef,
    parent: *mut IoRegistryEntry,
  ) -> KernReturn;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
  fn CFGetTypeID(cf: CFTypeRef) -> core_foundation::base::CFTypeID;

  fn CFStringGetTypeID() -> core_foundation::base::CFTypeID;

  fn CFDataGetTypeID() -> core_foundation::base::CFTypeID;
  fn CFDataGetLength(data: CFTypeRef) -> isize;
  fn CFDataGetBytePtr(data: CFTypeRef) -> *const u8;

  fn CFNumberGetTypeID() -> core_foundation::base::CFTypeID;
  fn CFNumberGetValue(
    number: CFTypeRef,
    the_type: i32,
    value_ptr: *mut std::ffi::c_void,
  ) -> u8;

  fn CFDictionaryGetTypeID() -> CFTypeID;
  fn CFDictionaryGetValue(
    dict: CFTypeRef,
    key: *const std::ffi::c_void,
  ) -> *const std::ffi::c_void;
}

// kCFNumberSInt64Type (from CFNumber.h)
const K_CFNUMBER_SINT64_TYPE: i32 = 4;
// kCFNumberSInt32Type (from CFNumber.h)
const K_CFNUMBER_SINT32_TYPE: i32 = 3;

#[derive(Debug, Clone)]
pub struct IokitGpuInfo {
  pub registry_id: u64,
  pub name: Option<String>,
  pub vendor_id: Option<u32>,
  pub core_count: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct IokitGpuMemoryUsage {
  pub in_use_bytes: Option<u64>,
  pub alloc_bytes: Option<u64>,
}

pub fn get_gpus_from_iokit() -> Result<Vec<IokitGpuInfo>, String> {
  // Prefer IOGraphicsAccelerator per user preference.
  // Fallback to IOAccelerator on some machines.
  let class_candidates = ["IOGraphicsAccelerator", "IOAccelerator"];

  for class_name in class_candidates {
    let list = enumerate_by_class(class_name)?;
    if !list.is_empty() {
      return Ok(list);
    }
  }

  Ok(Vec::new())
}

pub fn get_gpu_memory_usage_from_iokit() -> Option<IokitGpuMemoryUsage> {
  // Prefer IOGraphicsAccelerator per user preference.
  let class_candidates = ["IOGraphicsAccelerator", "IOAccelerator"];

  for class_name in class_candidates {
    if let Ok(mut list) = enumerate_memory_usage_by_class(class_name)
      && let Some(usage) = list.pop()
    {
      return Some(usage);
    }
  }

  None
}

fn enumerate_by_class(class_name: &str) -> Result<Vec<IokitGpuInfo>, String> {
  let class_cstr =
    CString::new(class_name).map_err(|_| format!("Invalid class name: {class_name}"))?;

  // Note: IOServiceGetMatchingServices consumes the matching dictionary.
  let matching = unsafe { IOServiceMatching(class_cstr.as_ptr()) };
  if matching.is_null() {
    return Err(format!("IOServiceMatching returned null for {class_name}"));
  }

  let mut iter: IoIterator = 0;
  let kr = unsafe {
    IOServiceGetMatchingServices(K_IOMASTER_PORT_DEFAULT, matching, &mut iter as *mut _)
  };

  if kr != KERN_SUCCESS {
    return Err(format!(
      "IOServiceGetMatchingServices failed for {class_name} (kern_return={kr})"
    ));
  }

  let mut out: Vec<IokitGpuInfo> = Vec::new();

  loop {
    let service = unsafe { IOIteratorNext(iter) };
    if service == 0 {
      break;
    }

    // `registry-id` is not a reliable property across machines.
    // Use the dedicated API to get a stable IORegistry entry id.
    let mut entry_id: u64 = 0;
    let kr_id =
      unsafe { IORegistryEntryGetRegistryEntryID(service, &mut entry_id as *mut _) };
    let registry_id = if kr_id == KERN_SUCCESS {
      Some(entry_id)
    } else {
      // Fallback (best-effort)
      get_u64_property(service, "registry-id")
    };
    // `IOName` can be a display name (e.g. "Color LCD"); avoid using it.
    // Prefer `model` on this entry; if missing, walk parents for `model`/`vendor-id`.
    let mut name = get_string_property(service, "model");
    let mut vendor_id = get_u32_property(service, "vendor-id");
    let core_count = get_u32_property(service, "gpu-core-count");

    if name.is_none() || vendor_id.is_none() {
      let (p_name, p_vendor) = find_parent_model_and_vendor(service, 8);
      if name.is_none() {
        name = p_name;
      }
      if vendor_id.is_none() {
        vendor_id = p_vendor;
      }
    }

    let _ = unsafe { IOObjectRelease(service) };

    let Some(registry_id) = registry_id else {
      continue;
    };

    out.push(IokitGpuInfo {
      registry_id,
      name,
      vendor_id,
      core_count,
    });
  }

  let _ = unsafe { IOObjectRelease(iter) };
  Ok(out)
}

fn enumerate_memory_usage_by_class(
  class_name: &str,
) -> Result<Vec<IokitGpuMemoryUsage>, String> {
  let class_cstr =
    CString::new(class_name).map_err(|_| format!("Invalid class name: {class_name}"))?;

  let matching = unsafe { IOServiceMatching(class_cstr.as_ptr()) };
  if matching.is_null() {
    return Err(format!("IOServiceMatching returned null for {class_name}"));
  }

  let mut iter: IoIterator = 0;
  let kr = unsafe {
    IOServiceGetMatchingServices(K_IOMASTER_PORT_DEFAULT, matching, &mut iter as *mut _)
  };

  if kr != KERN_SUCCESS {
    return Err(format!(
      "IOServiceGetMatchingServices failed for {class_name} (kern_return={kr})"
    ));
  }

  let mut out: Vec<IokitGpuMemoryUsage> = Vec::new();

  loop {
    let service = unsafe { IOIteratorNext(iter) };
    if service == 0 {
      break;
    }

    let stats = get_performance_statistics(service);
    let _ = unsafe { IOObjectRelease(service) };

    if stats.in_use_bytes.is_some() || stats.alloc_bytes.is_some() {
      out.push(stats);
    }
  }

  let _ = unsafe { IOObjectRelease(iter) };
  Ok(out)
}

fn find_parent_model_and_vendor(
  entry: IoRegistryEntry,
  max_depth: u8,
) -> (Option<String>, Option<u32>) {
  let plane = CFString::new("IOService");

  let mut current: IoRegistryEntry = entry;
  let mut depth: u8 = 0;

  let mut best_name: Option<String> = None;
  let mut best_vendor: Option<u32> = None;

  while depth < max_depth {
    let mut parent: IoRegistryEntry = 0;
    let kr = unsafe {
      IORegistryEntryGetParentEntry(
        current,
        plane.as_concrete_TypeRef(),
        &mut parent as *mut _,
      )
    };

    if kr != KERN_SUCCESS || parent == 0 {
      break;
    }

    if best_name.is_none() {
      best_name = get_string_property(parent, "model");
    }
    if best_vendor.is_none() {
      best_vendor = get_u32_property(parent, "vendor-id");
    }

    // Move up.
    if current != entry {
      let _ = unsafe { IOObjectRelease(current) };
    }
    current = parent;
    depth = depth.saturating_add(1);

    if best_name.is_some() && best_vendor.is_some() {
      break;
    }
  }

  if current != entry {
    let _ = unsafe { IOObjectRelease(current) };
  }

  (best_name, best_vendor)
}

fn get_cf_property(entry: IoRegistryEntry, key: &str) -> Option<CFTypeRef> {
  let k = CFString::new(key);
  let v = unsafe {
    IORegistryEntryCreateCFProperty(
      entry,
      k.as_concrete_TypeRef(),
      kCFAllocatorDefault,
      0,
    )
  };
  if v.is_null() { None } else { Some(v) }
}

fn get_u64_property(entry: IoRegistryEntry, key: &str) -> Option<u64> {
  let v = get_cf_property(entry, key)?;
  let out = cf_to_u64(v);
  unsafe { CFRelease(v as _) };
  out
}

fn get_u32_property(entry: IoRegistryEntry, key: &str) -> Option<u32> {
  let v = get_cf_property(entry, key)?;
  let out = cf_to_u32(v);
  unsafe { CFRelease(v as _) };
  out
}

fn get_string_property(entry: IoRegistryEntry, key: &str) -> Option<String> {
  let v = get_cf_property(entry, key)?;
  let out = cf_to_string(v);
  unsafe { CFRelease(v as _) };
  out
}

fn get_performance_statistics(entry: IoRegistryEntry) -> IokitGpuMemoryUsage {
  let v = get_cf_property(entry, "PerformanceStatistics");
  let Some(v) = v else {
    return IokitGpuMemoryUsage::default();
  };

  let is_dict = unsafe { CFGetTypeID(v) == CFDictionaryGetTypeID() };
  if !is_dict {
    unsafe { CFRelease(v as _) };
    return IokitGpuMemoryUsage::default();
  }

  let in_use = get_u64_from_dict(v, "In use system memory");
  let alloc = get_u64_from_dict(v, "Alloc system memory");

  unsafe { CFRelease(v as _) };

  IokitGpuMemoryUsage {
    in_use_bytes: in_use,
    alloc_bytes: alloc,
  }
}

fn get_u64_from_dict(dict: CFTypeRef, key: &str) -> Option<u64> {
  let k = CFString::new(key);
  let raw = unsafe { CFDictionaryGetValue(dict, k.as_concrete_TypeRef() as *const _) };
  if raw.is_null() {
    return None;
  }

  cf_to_u64(raw as CFTypeRef)
}

fn cf_to_string(v: CFTypeRef) -> Option<String> {
  if v.is_null() {
    return None;
  }

  let tid = unsafe { CFGetTypeID(v) };
  let string_tid = unsafe { CFStringGetTypeID() };
  let data_tid = unsafe { CFDataGetTypeID() };

  let s = if tid == string_tid {
    unsafe {
      core_foundation::string::CFString::wrap_under_get_rule(v as CFStringRef).to_string()
    }
  } else if tid == data_tid {
    cfdata_to_string(v)
  } else {
    String::new()
  };

  let s = s.trim().to_string();
  if s.is_empty() { None } else { Some(s) }
}

fn cf_to_u64(v: CFTypeRef) -> Option<u64> {
  if v.is_null() {
    return None;
  }

  let tid = unsafe { CFGetTypeID(v) };
  let number_tid = unsafe { CFNumberGetTypeID() };
  let data_tid = unsafe { CFDataGetTypeID() };

  if tid == number_tid {
    let mut out: i64 = 0;
    let ok = unsafe {
      CFNumberGetValue(v, K_CFNUMBER_SINT64_TYPE, (&mut out as *mut i64).cast())
    };
    if ok == 0 { None } else { Some(out as u64) }
  } else if tid == data_tid {
    cfdata_to_u64(v)
  } else {
    None
  }
}

fn cf_to_u32(v: CFTypeRef) -> Option<u32> {
  if v.is_null() {
    return None;
  }

  let tid = unsafe { CFGetTypeID(v) };
  let number_tid = unsafe { CFNumberGetTypeID() };
  let data_tid = unsafe { CFDataGetTypeID() };

  if tid == number_tid {
    let mut out: i32 = 0;
    let ok = unsafe {
      CFNumberGetValue(v, K_CFNUMBER_SINT32_TYPE, (&mut out as *mut i32).cast())
    };
    if ok == 0 { None } else { Some(out as u32) }
  } else if tid == data_tid {
    cfdata_to_u32(v)
  } else {
    None
  }
}

fn cfdata_to_u64(d: CFTypeRef) -> Option<u64> {
  let len = unsafe { CFDataGetLength(d) };
  if len < 8 {
    return None;
  }

  let ptr = unsafe { CFDataGetBytePtr(d) };
  if ptr.is_null() {
    return None;
  }

  let bytes = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
  let mut arr = [0u8; 8];
  arr.copy_from_slice(&bytes[..8]);
  Some(u64::from_le_bytes(arr))
}

fn cfdata_to_u32(d: CFTypeRef) -> Option<u32> {
  let len = unsafe { CFDataGetLength(d) };
  if len < 4 {
    return None;
  }

  let ptr = unsafe { CFDataGetBytePtr(d) };
  if ptr.is_null() {
    return None;
  }

  let bytes = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
  let mut arr = [0u8; 4];
  arr.copy_from_slice(&bytes[..4]);
  Some(u32::from_le_bytes(arr))
}

fn cfdata_to_string(d: CFTypeRef) -> String {
  let len = unsafe { CFDataGetLength(d) };
  if len <= 0 {
    return String::new();
  }

  let ptr = unsafe { CFDataGetBytePtr(d) };
  if ptr.is_null() {
    return String::new();
  }

  let bytes = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
  let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
  String::from_utf8_lossy(&bytes[..end]).to_string()
}
