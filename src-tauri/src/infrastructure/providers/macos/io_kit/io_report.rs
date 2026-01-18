/*
Inspired by macmon (MIT): https://github.com/vladkens/macmon
Referenced for IOReport sampling. No code copied.

Copyright (c) 2024 vladkens
Licensed under the MIT License. See THIRD_PARTY_NOTICES.md for the full text.
*/

use std::{mem::MaybeUninit, ptr::null, time::Instant};

use core_foundation::{
  array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef},
  base::{CFRelease, CFTypeRef, TCFType},
  dictionary::{
    CFDictionaryCreateMutableCopy, CFDictionaryGetCount, CFDictionaryGetValue,
    CFDictionaryRef, CFMutableDictionaryRef,
  },
  string::CFString,
};

pub type WithError<T> = Result<T, Box<dyn std::error::Error>>;
pub type CVoidRef = *const std::ffi::c_void;

#[repr(C)]
pub struct IOReportSubscription {
  _priv: [u8; 0],
}
pub type IOReportSubscriptionRef = *const IOReportSubscription;

#[allow(dead_code)]
#[link(name = "IOReport", kind = "dylib")]
unsafe extern "C" {
  fn IOReportCopyChannelsInGroup(
    group: core_foundation::string::CFStringRef,
    subgroup: core_foundation::string::CFStringRef,
    a: u64,
    b: u64,
    c: u64,
  ) -> CFDictionaryRef;

  fn IOReportMergeChannels(a: CFDictionaryRef, b: CFDictionaryRef, nil: CFTypeRef);

  fn IOReportCreateSubscription(
    a: CVoidRef,
    chan: CFMutableDictionaryRef,
    out: *mut CFMutableDictionaryRef,
    d: u64,
    e: CFTypeRef,
  ) -> IOReportSubscriptionRef;

  fn IOReportCreateSamples(
    subs: IOReportSubscriptionRef,
    chan: CFMutableDictionaryRef,
    nil: CFTypeRef,
  ) -> CFDictionaryRef;

  fn IOReportCreateSamplesDelta(
    a: CFDictionaryRef,
    b: CFDictionaryRef,
    nil: CFTypeRef,
  ) -> CFDictionaryRef;

  fn IOReportChannelGetGroup(
    item: CFDictionaryRef,
  ) -> core_foundation::string::CFStringRef;
  fn IOReportChannelGetSubGroup(
    item: CFDictionaryRef,
  ) -> core_foundation::string::CFStringRef;
  fn IOReportChannelGetChannelName(
    item: CFDictionaryRef,
  ) -> core_foundation::string::CFStringRef;

  fn IOReportStateGetCount(item: CFDictionaryRef) -> i32;
  fn IOReportStateGetNameForIndex(
    item: CFDictionaryRef,
    idx: i32,
  ) -> core_foundation::string::CFStringRef;
  fn IOReportStateGetResidency(item: CFDictionaryRef, idx: i32) -> i64;
}

fn dict_get(dict: CFDictionaryRef, key: &str) -> Option<CFTypeRef> {
  let k = CFString::new(key);
  unsafe {
    let v = CFDictionaryGetValue(dict, k.as_concrete_TypeRef() as _);
    if v.is_null() { None } else { Some(v) }
  }
}

fn cfstr_to_string(s: core_foundation::string::CFStringRef) -> String {
  if s.is_null() {
    return String::new();
  }
  unsafe { CFString::wrap_under_get_rule(s).to_string() }
}

fn get_group(item: CFDictionaryRef) -> String {
  cfstr_to_string(unsafe { IOReportChannelGetGroup(item) })
}
fn get_subgroup(item: CFDictionaryRef) -> String {
  cfstr_to_string(unsafe { IOReportChannelGetSubGroup(item) })
}
fn get_channel(item: CFDictionaryRef) -> String {
  cfstr_to_string(unsafe { IOReportChannelGetChannelName(item) })
}

fn get_residencies(item: CFDictionaryRef) -> Vec<(String, i64)> {
  let count = unsafe { IOReportStateGetCount(item) };
  let mut out = Vec::with_capacity(count.max(0) as usize);
  for i in 0..count {
    let name = cfstr_to_string(unsafe { IOReportStateGetNameForIndex(item, i) });
    let val = unsafe { IOReportStateGetResidency(item, i) };
    out.push((name, val));
  }
  out
}

fn build_channels_gpu_only() -> WithError<CFMutableDictionaryRef> {
  let g = CFString::new("GPU Stats");
  let sg = CFString::new("GPU Performance States");

  unsafe {
    let raw = IOReportCopyChannelsInGroup(
      g.as_concrete_TypeRef(),
      sg.as_concrete_TypeRef(),
      0,
      0,
      0,
    );
    if raw.is_null() {
      return Err("IOReportCopyChannelsInGroup returned null".into());
    }

    let size = CFDictionaryGetCount(raw);
    let chan = CFDictionaryCreateMutableCopy(
      core_foundation::base::kCFAllocatorDefault,
      size,
      raw,
    );
    CFRelease(raw as _);

    // Treat as failure if IOReportChannels is missing.
    if dict_get(chan as _, "IOReportChannels").is_none() {
      CFRelease(chan as _);
      return Err("channel dict has no IOReportChannels".into());
    }

    Ok(chan)
  }
}

fn create_subscription(
  chan: CFMutableDictionaryRef,
) -> WithError<IOReportSubscriptionRef> {
  unsafe {
    let mut out: MaybeUninit<CFMutableDictionaryRef> = MaybeUninit::uninit();
    let subs = IOReportCreateSubscription(null(), chan, out.as_mut_ptr(), 0, null());
    if subs.is_null() {
      return Err("IOReportCreateSubscription failed".into());
    }

    let _ = out.assume_init();
    Ok(subs)
  }
}

pub struct GpuUsageIOReport {
  subs: IOReportSubscriptionRef,
  chan: CFMutableDictionaryRef,
  prev: Option<(CFDictionaryRef, Instant)>,
}

impl GpuUsageIOReport {
  pub fn new() -> WithError<Self> {
    let chan = build_channels_gpu_only()?;
    let subs = create_subscription(chan)?;
    Ok(Self {
      subs,
      chan,
      prev: None,
    })
  }

  fn raw_sample(&self) -> (CFDictionaryRef, Instant) {
    (
      unsafe { IOReportCreateSamples(self.subs, self.chan, null()) },
      Instant::now(),
    )
  }

  /// Returns usage for the interval since the previous call (keeps prev, delta-based).
  pub fn sample_usage(&mut self) -> WithError<f32> {
    // Initialize prev if needed.
    let prev = match self.prev.take() {
      Some(p) => p,
      None => self.raw_sample(),
    };

    // Next sample.
    let next = self.raw_sample();

    // Build delta (difference).
    let delta = unsafe { IOReportCreateSamplesDelta(prev.0, next.0, null()) };
    unsafe { CFRelease(prev.0 as _) };

    // Keep for the next call.
    self.prev = Some(next);

    // Find GPU Perf States in the delta dictionary and compute usage.
    let usage = compute_gpu_usage_from_delta(delta)?;
    unsafe { CFRelease(delta as _) };
    Ok(usage)
  }
}

impl Drop for GpuUsageIOReport {
  fn drop(&mut self) {
    unsafe {
      if let Some((s, _)) = self.prev.take() {
        CFRelease(s as _);
      }
      CFRelease(self.chan as _);
      CFRelease(self.subs as _);
    }
  }
}

fn compute_gpu_usage_from_delta(delta: CFDictionaryRef) -> WithError<f32> {
  let arr = dict_get(delta, "IOReportChannels").ok_or("delta has no IOReportChannels")?
    as CFArrayRef;

  let n = unsafe { CFArrayGetCount(arr) };
  for i in 0..n {
    let item = unsafe { CFArrayGetValueAtIndex(arr, i) } as CFDictionaryRef;

    if get_group(item) != "GPU Stats" {
      continue;
    }
    if get_subgroup(item) != "GPU Performance States" {
      continue;
    }
    if get_channel(item) != "GPUPH" {
      continue;
    }

    let resid = get_residencies(item);
    if resid.is_empty() {
      return Ok(0.0);
    }

    let mut total: i64 = 0;
    let mut idle: i64 = 0;
    for (name, v) in &resid {
      if *v <= 0 {
        continue;
      }
      total += *v;
      if is_idle_state(name) {
        idle += *v;
      }
    }
    if total <= 0 {
      return Ok(0.0);
    }
    let usage = ((total - idle) as f32) / (total as f32);
    return Ok(usage.clamp(0.0, 1.0));
  }

  Err("GPU Performance States channel not found".into())
}

fn is_idle_state(name: &str) -> bool {
  let n = name.trim().to_ascii_lowercase();

  if n == "off" {
    return true;
  }

  if n == "idle" || n.contains("idle") {
    return true;
  }

  if n == "down" {
    return true;
  }

  false
}
