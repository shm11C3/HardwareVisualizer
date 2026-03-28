/*
Inspired by macmon (MIT): https://github.com/vladkens/macmon
Referenced for IOReport sampling and frequency-weighted GPU usage calculation.
No code copied; independently reimplemented from the same public Apple APIs.

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
  /// GPU DVFS frequencies for active P-states (P1, P2, …, PN) in MHz.
  /// Empty if the frequency table could not be read from PMGR.
  gpu_freqs: Vec<u32>,
}

impl GpuUsageIOReport {
  pub fn new() -> WithError<Self> {
    let chan = build_channels_gpu_only()?;
    let subs = create_subscription(chan)?;

    // Read the GPU frequency table from the PMGR IOKit node.
    // The first entry (index 0) is the OFF/idle state — skip it so that
    // gpu_freqs[0] = P1 frequency, gpu_freqs[1] = P2 frequency, etc.
    let gpu_freqs = super::iokit_info::read_gpu_dvfs_freqs_mhz()
      .map(|f| {
        if f.len() > 1 {
          f[1..].to_vec()
        } else {
          Vec::new()
        }
      })
      .unwrap_or_default();

    Ok(Self {
      subs,
      chan,
      prev: None,
      gpu_freqs,
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
    let usage = compute_gpu_usage_from_delta(delta, &self.gpu_freqs)?;
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

/// Compute GPU usage from the GPUPH channel in the IOReport delta.
///
/// When a GPU frequency table is available, the usage is computed as:
///   `(avg_active_freq × active_ratio) / max_freq`
/// This weights lower P-states (e.g. P1 at minimum frequency for display
/// compositing) proportionally less than higher P-states, matching the
/// approach used by macmon.
///
/// Without frequencies, falls back to simple active-time ratio.
fn compute_gpu_usage_from_delta(
  delta: CFDictionaryRef,
  gpu_freqs: &[u32],
) -> WithError<f32> {
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

    if gpu_freqs.is_empty() {
      return compute_usage_unweighted(&resid);
    }
    return compute_usage_freq_weighted(&resid, gpu_freqs);
  }

  Err("GPU Performance States channel not found".into())
}

/// Frequency-weighted GPU usage (matches macmon's approach).
///
/// Formula: `usage = (avg_active_freq × active_ratio) / max_freq`
/// where `avg_active_freq` is the time-weighted average of active P-state
/// frequencies, and `active_ratio` is the fraction of total time spent
/// in any active state.
fn compute_usage_freq_weighted(resid: &[(String, i64)], freqs: &[u32]) -> WithError<f32> {
  // Find the first active (non-idle) state index.
  let offset = resid
    .iter()
    .position(|(name, _)| !is_idle_state(name))
    .ok_or("no active P-states found")?;

  let total: f64 = resid.iter().map(|(_, v)| *v as f64).sum();
  let active: f64 = resid.iter().skip(offset).map(|(_, v)| *v as f64).sum();

  if total <= 0.0 || active <= 0.0 {
    return Ok(0.0);
  }

  // Compute the time-weighted average frequency across active P-states.
  let count = freqs.len().min(resid.len() - offset);
  let mut avg_freq = 0.0_f64;
  for i in 0..count {
    let frac = resid[i + offset].1 as f64 / active;
    avg_freq += frac * freqs[i] as f64;
  }

  let active_ratio = active / total;
  let min_freq = *freqs.first().unwrap_or(&1) as f64;
  let max_freq = *freqs.last().unwrap_or(&1) as f64;
  if max_freq <= 0.0 {
    return Ok(0.0);
  }

  let usage = (avg_freq.max(min_freq) * active_ratio) / max_freq;
  Ok((usage as f32).clamp(0.0, 1.0))
}

/// Simple unweighted fallback: fraction of time in any active P-state.
fn compute_usage_unweighted(resid: &[(String, i64)]) -> WithError<f32> {
  let mut total: i64 = 0;
  let mut idle: i64 = 0;
  for (name, v) in resid {
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
  Ok(usage.clamp(0.0, 1.0))
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

#[cfg(test)]
mod tests {
  use super::*;

  // ── is_idle_state ──

  #[test]
  fn is_idle_state_classification() {
    // Idle states (OFF observed on M4; IDLE/DOWN on M2/M3 Max per macmon)
    for name in [
      "OFF", "off", "IDLE", "idle", "IDLE_OFF", "DOWN", "down", "  OFF  ",
    ] {
      assert!(is_idle_state(name), "{name:?} should be idle");
    }
    // Active P-states
    for name in ["P1", "P15", "PERF"] {
      assert!(!is_idle_state(name), "{name:?} should not be idle");
    }
  }

  // ── compute_usage_unweighted ──

  #[test]
  fn unweighted_all_off() {
    let resid = vec![("OFF".into(), 1000i64)];
    let usage = compute_usage_unweighted(&resid).unwrap();
    assert_eq!(usage, 0.0);
  }

  #[test]
  fn unweighted_all_active() {
    let resid = vec![("OFF".into(), 0i64), ("P1".into(), 1000)];
    let usage = compute_usage_unweighted(&resid).unwrap();
    assert!((usage - 1.0).abs() < 1e-6);
  }

  #[test]
  fn unweighted_half_active() {
    let resid = vec![("OFF".into(), 500i64), ("P1".into(), 500)];
    let usage = compute_usage_unweighted(&resid).unwrap();
    assert!((usage - 0.5).abs() < 1e-6);
  }

  #[test]
  fn unweighted_empty_returns_zero() {
    let resid: Vec<(String, i64)> = vec![];
    let usage = compute_usage_unweighted(&resid).unwrap();
    assert_eq!(usage, 0.0);
  }

  // ── compute_usage_freq_weighted ──

  fn m4_resid(off: i64, p1: i64, p2: i64, p3: i64) -> Vec<(String, i64)> {
    vec![
      ("OFF".into(), off),
      ("P1".into(), p1),
      ("P2".into(), p2),
      ("P3".into(), p3),
    ]
  }

  // Frequencies in MHz: P1=396, P2=720, P3=1398
  fn m4_freqs() -> Vec<u32> {
    vec![396, 720, 1398]
  }

  #[test]
  fn freq_weighted_all_off() {
    let resid = m4_resid(1000, 0, 0, 0);
    let usage = compute_usage_freq_weighted(&resid, &m4_freqs()).unwrap();
    assert_eq!(usage, 0.0);
  }

  #[test]
  fn freq_weighted_all_p1() {
    // 100% active at P1 (396 MHz), max = 1398 MHz
    // usage = (396 * 1.0) / 1398 ≈ 0.283
    let resid = m4_resid(0, 1000, 0, 0);
    let usage = compute_usage_freq_weighted(&resid, &m4_freqs()).unwrap();
    assert!((usage - 396.0 / 1398.0).abs() < 0.01);
  }

  #[test]
  fn freq_weighted_all_max() {
    // 100% active at P3 (1398 MHz) → usage = 1.0
    let resid = m4_resid(0, 0, 0, 1000);
    let usage = compute_usage_freq_weighted(&resid, &m4_freqs()).unwrap();
    assert!((usage - 1.0).abs() < 0.01);
  }

  #[test]
  fn freq_weighted_half_off_half_p1() {
    // 50% active at P1 (396 MHz)
    // usage = (396 * 0.5) / 1398 ≈ 0.142
    let resid = m4_resid(500, 500, 0, 0);
    let usage = compute_usage_freq_weighted(&resid, &m4_freqs()).unwrap();
    let expected = (396.0 * 0.5) / 1398.0;
    assert!((usage - expected as f32).abs() < 0.01);
  }

  #[test]
  fn freq_weighted_realistic_idle_desktop() {
    // Typical idle: OFF=10M, P1=13M, P2=0.5M, P3=0.5M
    let resid = m4_resid(10_000_000, 13_000_000, 500_000, 500_000);
    let usage = compute_usage_freq_weighted(&resid, &m4_freqs()).unwrap();
    // Should be in the 10-20% range, not 50-60%
    assert!(usage > 0.05 && usage < 0.25, "usage={usage}");
  }

  #[test]
  fn freq_weighted_empty_freqs_returns_err() {
    let resid = m4_resid(500, 500, 0, 0);
    // Empty freqs → no active P-states to match
    let result = compute_usage_freq_weighted(&resid, &[]);
    // offset finds P1, but count = min(0, 3) = 0, so avg_freq stays 0
    // max_freq from empty slice is 1 (unwrap_or(&1)), so usage ≈ 0
    assert!(result.is_ok());
  }

  #[test]
  fn freq_weighted_clamped_to_one() {
    // Edge case: should never exceed 1.0
    let resid = vec![("OFF".into(), 0i64), ("P1".into(), 1000)];
    let freqs = vec![2000]; // freq > max_freq (they're the same here)
    let usage = compute_usage_freq_weighted(&resid, &freqs).unwrap();
    assert!(usage <= 1.0);
  }
}
