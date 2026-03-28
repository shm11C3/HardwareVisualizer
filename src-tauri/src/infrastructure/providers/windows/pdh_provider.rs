///
/// PDH-based GPU engine utilization provider.
///
/// Replaces the WMI-based `Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine`
/// query with a direct PDH counter query for lower overhead and no COM dependency.
///
/// The PDH query handle and counter handle are initialised once (on the first
/// call) and reused across subsequent polls so that only `CollectQueryData` +
/// `GetFormattedCounterArrayW` run on each tick.
///
use crate::{log_debug, log_error, log_internal};
use std::collections::HashMap;
use std::error::Error;
use std::mem::{self, MaybeUninit};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use tokio::task::spawn_blocking;
use windows::Win32::System::Performance::{
  PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W,
  PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PdhAddEnglishCounterW, PdhCloseQuery,
  PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};
use windows::core::PCWSTR;

/// `PdhGetFormattedCounterArrayW` returns this when the caller-supplied buffer
/// is too small and needs to be grown (normal sizing flow).
const PDH_MORE_DATA: u32 = 0x800007D2;

/// How long cached results remain valid.  When multiple engine types are
/// queried within the same monitoring tick, only the first call triggers an
/// actual PDH collect; the rest are served from cache.
const CACHE_TTL: std::time::Duration = std::time::Duration::from_millis(100);

// ---------------------------------------------------------------------------
// GPU engine type
// ---------------------------------------------------------------------------

/// WDDM GPU engine types exposed by the PDH `GPU Engine` counter set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuEngineType {
  Graphics3D,
  Copy,
  VideoDecode,
  VideoEncode,
  VideoProcessing,
}

impl GpuEngineType {
  /// The suffix that appears in PDH instance names (e.g. `engtype_3D`).
  fn as_pdh_suffix(&self) -> &'static str {
    match self {
      Self::Graphics3D => "3D",
      Self::Copy => "Copy",
      Self::VideoDecode => "VideoDecode",
      Self::VideoEncode => "VideoEncode",
      Self::VideoProcessing => "VideoProcessing",
    }
  }

  /// Parse a PDH engine-type suffix back into the enum.
  fn from_pdh_suffix(s: &str) -> Option<Self> {
    match s {
      "3D" => Some(Self::Graphics3D),
      "Copy" => Some(Self::Copy),
      "VideoDecode" => Some(Self::VideoDecode),
      "VideoEncode" => Some(Self::VideoEncode),
      "VideoProcessing" => Some(Self::VideoProcessing),
      _ => None,
    }
  }
}

// ---------------------------------------------------------------------------
// Persistent query state
// ---------------------------------------------------------------------------

/// Holds the PDH query and counter handles that survive across polls.
/// Created once, closed when the process exits (or on `Drop`).
struct PdhState {
  query: PDH_HQUERY,
  counter: PDH_HCOUNTER,
  /// Reusable buffer for `PdhGetFormattedCounterArrayW`.
  /// Typed as `MaybeUninit<PDH_FMT_COUNTERVALUE_ITEM_W>` to guarantee
  /// correct alignment regardless of the struct's actual `align_of`.
  buf: Vec<MaybeUninit<PDH_FMT_COUNTERVALUE_ITEM_W>>,
  /// Per-engine-type max utilisation (0.0–1.0) from the last collect.
  /// Populated for *all* known engine types in a single pass so that
  /// concurrent queries for different engine types don't each trigger a
  /// PDH collect.
  cache: HashMap<GpuEngineType, f32>,
  /// Per-LUID per-engine-type max utilisation (0.0–1.0) from the last
  /// collect.  Keyed by `(luid_high, luid_low, engine_type)` so that
  /// callers can query a specific GPU device (e.g. Intel iGPU).
  luid_cache: HashMap<(i32, u32, GpuEngineType), f32>,
  /// When `cache` was last populated.
  cache_time: Instant,
}

// SAFETY: PDH handles are *not* thread-safe, but we guarantee exclusive
// access through a Mutex and always call PDH from `spawn_blocking`.
unsafe impl Send for PdhState {}

impl Drop for PdhState {
  fn drop(&mut self) {
    unsafe {
      PdhCloseQuery(self.query);
    }
  }
}

/// Lazy-initialised global state.  Stores `Err(message)` if PDH
/// initialisation failed so subsequent calls return the error instead of
/// panicking.
static PDH_STATE: OnceLock<Result<Mutex<PdhState>, String>> = OnceLock::new();

/// Initialise the persistent query (called once under the lock).
fn init_pdh_state() -> Result<PdhState, Box<dyn Error + Send>> {
  unsafe {
    let mut query: PDH_HQUERY = PDH_HQUERY::default();
    let status = PdhOpenQueryW(PCWSTR::null(), 0, &mut query);
    if status != 0 {
      return Err(pdh_err(format!("PdhOpenQueryW failed: 0x{status:08X}")));
    }

    let counter_path = "\\GPU Engine(*)\\Utilization Percentage";
    let wide: Vec<u16> = counter_path
      .encode_utf16()
      .chain(std::iter::once(0))
      .collect();

    let mut counter: PDH_HCOUNTER = PDH_HCOUNTER::default();
    let status = PdhAddEnglishCounterW(query, PCWSTR(wide.as_ptr()), 0, &mut counter);
    if status != 0 {
      PdhCloseQuery(query);
      return Err(pdh_err(format!(
        "PdhAddEnglishCounterW failed: 0x{status:08X}"
      )));
    }

    // Prime: collect baseline → sleep → collect again so rate-based
    // counters and drivers that return 0 on the first sample are ready.
    let status = PdhCollectQueryData(query);
    if status != 0 {
      PdhCloseQuery(query);
      return Err(pdh_err(format!(
        "PdhCollectQueryData (prime 1) failed: 0x{status:08X}"
      )));
    }

    std::thread::sleep(std::time::Duration::from_millis(200));

    let status = PdhCollectQueryData(query);
    if status != 0 {
      PdhCloseQuery(query);
      return Err(pdh_err(format!(
        "PdhCollectQueryData (prime 2) failed: 0x{status:08X}"
      )));
    }

    Ok(PdhState {
      query,
      counter,
      buf: Vec::new(),
      cache: HashMap::new(),
      luid_cache: HashMap::new(),
      cache_time: Instant::now(),
    })
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Query GPU engine utilization for the specified engine type using PDH.
///
/// Returns the **maximum** utilisation percentage (as 0.0–1.0) across all
/// instances whose name contains `engtype_<engine_type>`.
pub async fn query_gpu_usage_by_device_and_engine(
  engine_type: GpuEngineType,
) -> Result<f32, Box<dyn Error + Send>> {
  spawn_blocking(move || collect_and_read(engine_type))
    .await
    .map_err(|e| {
      log_error!(
        "join_error",
        "pdh_provider::query_gpu_usage_by_device_and_engine",
        Some(e.to_string())
      );
      pdh_err("PDH query task panicked".to_string())
    })?
}

/// Query GPU engine utilization for a specific GPU identified by its LUID.
///
/// Returns the **maximum** utilisation percentage (as 0.0–1.0) across all
/// instances matching the given LUID and engine type.
pub async fn query_gpu_usage_by_luid_and_engine(
  luid_high: i32,
  luid_low: u32,
  engine_type: GpuEngineType,
) -> Result<f32, Box<dyn Error + Send>> {
  spawn_blocking(move || collect_and_read_by_luid(luid_high, luid_low, engine_type))
    .await
    .map_err(|e| {
      log_error!(
        "join_error",
        "pdh_provider::query_gpu_usage_by_luid_and_engine",
        Some(e.to_string())
      );
      pdh_err("PDH LUID query task panicked".to_string())
    })?
}

// ---------------------------------------------------------------------------
// Pure helper functions (testable without PDH handles)
// ---------------------------------------------------------------------------

/// Returns `true` when `raw` is finite and within the 0–100 range expected
/// from PDH utilisation counters.
fn is_valid_pdh_value(raw: f64) -> bool {
  raw.is_finite() && (0.0..=100.0).contains(&raw)
}

/// Extract the [`GpuEngineType`] from a PDH instance name such as
/// `pid_1234_luid_0x00_0x0000_phys_0_eng_1_engtype_3D`.
///
/// Uses `rfind` so that only the **last** `engtype_` tag is considered.
fn parse_engine_type_from_instance(name: &str) -> Option<GpuEngineType> {
  let engtype_tag = "engtype_";
  let pos = name.rfind(engtype_tag)?;
  let suffix = &name[pos + engtype_tag.len()..];
  let suffix = suffix.split('_').next().unwrap_or(suffix);
  GpuEngineType::from_pdh_suffix(suffix)
}

/// Extract the LUID `(HighPart, LowPart)` from a PDH instance name such as
/// `pid_1234_luid_0x00_0x0000C61F_phys_0_eng_0_engtype_3D`.
///
/// The LUID is encoded as two hex values: `luid_0x<high>_0x<low>`.
fn parse_luid_from_instance(name: &str) -> Option<(i32, u32)> {
  let luid_tag = "luid_";
  let pos = name.find(luid_tag)?;
  let after = &name[pos + luid_tag.len()..];

  // Split remaining by '_' and take the two hex parts: "0xHH" and "0xLL"
  let mut parts = after.split('_');
  let high_str = parts.next()?;
  let low_str = parts.next()?;

  let high = i32::from_str_radix(
    high_str
      .strip_prefix("0x")
      .or(high_str.strip_prefix("0X"))?,
    16,
  )
  .ok()?;
  let low = u32::from_str_radix(
    low_str.strip_prefix("0x").or(low_str.strip_prefix("0X"))?,
    16,
  )
  .ok()?;
  Some((high, low))
}

/// Aggregate a slice of `(engine_type, raw_percentage)` pairs into `cache`.
///
/// For each engine type the **maximum** raw value is kept, normalised from
/// the 0–100 PDH range to 0.0–1.0.  The cache is cleared before aggregation
/// so stale entries from a previous collection do not leak through.
fn aggregate_engine_usage(
  cache: &mut HashMap<GpuEngineType, f32>,
  entries: &[(GpuEngineType, f64)],
) {
  cache.clear();
  for &(etype, raw) in entries {
    let value = (raw as f32 / 100.0).clamp(0.0, 1.0);
    let entry = cache.entry(etype).or_insert(0.0f32);
    if value > *entry {
      *entry = value;
    }
  }
}

/// Aggregate a slice of `(luid, engine_type, raw_percentage)` triples into
/// `luid_cache`.  For each `(luid, engine_type)` pair the **maximum** raw
/// value is kept, normalised from the 0–100 PDH range to 0.0–1.0.
fn aggregate_engine_usage_per_luid(
  luid_cache: &mut HashMap<(i32, u32, GpuEngineType), f32>,
  entries: &[((i32, u32), GpuEngineType, f64)],
) {
  luid_cache.clear();
  for &((high, low), etype, raw) in entries {
    let value = (raw as f32 / 100.0).clamp(0.0, 1.0);
    let entry = luid_cache.entry((high, low, etype)).or_insert(0.0f32);
    if value > *entry {
      *entry = value;
    }
  }
}

// ---------------------------------------------------------------------------
// Core collection logic (runs inside `spawn_blocking`)
// ---------------------------------------------------------------------------

/// Acquire the global PDH state, returning a locked guard.
fn acquire_pdh_state()
-> Result<std::sync::MutexGuard<'static, PdhState>, Box<dyn Error + Send>> {
  let init_result = PDH_STATE.get_or_init(|| match init_pdh_state() {
    Ok(state) => Ok(Mutex::new(state)),
    Err(e) => {
      log_error!(
        "init_error",
        "pdh_provider::acquire_pdh_state",
        Some(e.to_string())
      );
      Err(e.to_string())
    }
  });

  let mtx = init_result
    .as_ref()
    .map_err(|e| pdh_err(format!("PDH init failed: {e}")))?;

  mtx
    .lock()
    .map_err(|e| pdh_err(format!("PDH_STATE lock poisoned: {e}")))
}

/// Collect fresh PDH data if the cache has expired, populating both the
/// global engine cache and the per-LUID cache.
fn ensure_fresh_data(state: &mut PdhState) -> Result<(), Box<dyn Error + Send>> {
  if state.cache_time.elapsed() < CACHE_TTL {
    return Ok(());
  }

  unsafe {
    // Collect fresh sample
    let status = PdhCollectQueryData(state.query);
    if status != 0 {
      return Err(pdh_err(format!(
        "PdhCollectQueryData failed: 0x{status:08X}"
      )));
    }

    // --- size the buffer ---
    let mut buf_size: u32 = 0;
    let mut item_count: u32 = 0;

    let status = PdhGetFormattedCounterArrayW(
      state.counter,
      PDH_FMT_DOUBLE,
      &mut buf_size,
      &mut item_count,
      None,
    );
    if status != PDH_MORE_DATA {
      return Err(pdh_err(format!(
        "PdhGetFormattedCounterArrayW (sizing) failed: 0x{status:08X}"
      )));
    }

    // Grow reusable buffer if necessary (convert byte count → item count, round up)
    let item_size = mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
    let needed = (buf_size as usize).div_ceil(item_size);
    if needed > state.buf.len() {
      state.buf.resize_with(needed, MaybeUninit::uninit);
    }

    // --- fetch data into aligned buffer ---
    let status = PdhGetFormattedCounterArrayW(
      state.counter,
      PDH_FMT_DOUBLE,
      &mut buf_size,
      &mut item_count,
      Some(state.buf.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>()),
    );
    if status != 0 {
      return Err(pdh_err(format!(
        "PdhGetFormattedCounterArrayW (data) failed: 0x{status:08X}"
      )));
    }

    // --- walk all instances, build caches ---
    let items = std::slice::from_raw_parts(
      state.buf.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
      item_count as usize,
    );

    let mut global_entries: Vec<(GpuEngineType, f64)> = Vec::new();
    let mut luid_entries: Vec<((i32, u32), GpuEngineType, f64)> = Vec::new();
    for item in items {
      let cstatus = item.FmtValue.CStatus;
      if cstatus != PDH_CSTATUS_VALID_DATA && cstatus != PDH_CSTATUS_NEW_DATA {
        continue;
      }
      let raw = item.FmtValue.Anonymous.doubleValue;
      if !is_valid_pdh_value(raw) {
        continue;
      }
      let name = pwstr_to_string(item.szName.0);
      if let Some(etype) = parse_engine_type_from_instance(&name) {
        global_entries.push((etype, raw));
        if let Some(luid) = parse_luid_from_instance(&name) {
          luid_entries.push((luid, etype, raw));
        }
      }
    }
    aggregate_engine_usage(&mut state.cache, &global_entries);
    aggregate_engine_usage_per_luid(&mut state.luid_cache, &luid_entries);

    state.cache_time = Instant::now();
  }

  Ok(())
}

fn collect_and_read(engine_type: GpuEngineType) -> Result<f32, Box<dyn Error + Send>> {
  let mut guard = acquire_pdh_state()?;
  let state = &mut *guard;

  ensure_fresh_data(state)?;

  let suffix = engine_type.as_pdh_suffix();
  match state.cache.get(&engine_type) {
    Some(&v) => {
      log_debug!(
        &format!("PDH GPU usage for engtype_{suffix}: {v}"),
        "pdh_provider::collect_and_read",
        None::<&str>
      );
      Ok(v)
    }
    None => Err(pdh_err(format!(
      "No PDH GPU engine data for engtype_{suffix}"
    ))),
  }
}

fn collect_and_read_by_luid(
  luid_high: i32,
  luid_low: u32,
  engine_type: GpuEngineType,
) -> Result<f32, Box<dyn Error + Send>> {
  let mut guard = acquire_pdh_state()?;
  let state = &mut *guard;

  ensure_fresh_data(state)?;

  let key = (luid_high, luid_low, engine_type);
  match state.luid_cache.get(&key) {
    Some(&v) => Ok(v),
    None => Err(pdh_err(format!(
      "No PDH GPU engine data for LUID 0x{luid_high:02X}_0x{luid_low:08X} engtype_{suffix}",
      suffix = engine_type.as_pdh_suffix()
    ))),
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read a null-terminated wide string from a raw `*const u16` pointer.
///
/// # Safety
/// The caller must guarantee that `ptr` points to a valid, null-terminated
/// UTF-16 string.
unsafe fn pwstr_to_string(ptr: *const u16) -> String {
  if ptr.is_null() {
    return String::new();
  }
  unsafe {
    let mut len = 0;
    while *ptr.add(len) != 0 {
      len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
  }
}

fn pdh_err(msg: String) -> Box<dyn Error + Send> {
  Box::new(std::io::Error::other(msg))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  // -----------------------------------------------------------------------
  // GpuEngineType::as_pdh_suffix
  // -----------------------------------------------------------------------

  #[test]
  fn as_pdh_suffix_graphics3d() {
    assert_eq!(GpuEngineType::Graphics3D.as_pdh_suffix(), "3D");
  }

  #[test]
  fn as_pdh_suffix_copy() {
    assert_eq!(GpuEngineType::Copy.as_pdh_suffix(), "Copy");
  }

  #[test]
  fn as_pdh_suffix_video_decode() {
    assert_eq!(GpuEngineType::VideoDecode.as_pdh_suffix(), "VideoDecode");
  }

  #[test]
  fn as_pdh_suffix_video_encode() {
    assert_eq!(GpuEngineType::VideoEncode.as_pdh_suffix(), "VideoEncode");
  }

  #[test]
  fn as_pdh_suffix_video_processing() {
    assert_eq!(
      GpuEngineType::VideoProcessing.as_pdh_suffix(),
      "VideoProcessing"
    );
  }

  // -----------------------------------------------------------------------
  // GpuEngineType::from_pdh_suffix
  // -----------------------------------------------------------------------

  #[test]
  fn from_pdh_suffix_3d() {
    assert_eq!(
      GpuEngineType::from_pdh_suffix("3D"),
      Some(GpuEngineType::Graphics3D)
    );
  }

  #[test]
  fn from_pdh_suffix_copy() {
    assert_eq!(
      GpuEngineType::from_pdh_suffix("Copy"),
      Some(GpuEngineType::Copy)
    );
  }

  #[test]
  fn from_pdh_suffix_video_decode() {
    assert_eq!(
      GpuEngineType::from_pdh_suffix("VideoDecode"),
      Some(GpuEngineType::VideoDecode)
    );
  }

  #[test]
  fn from_pdh_suffix_video_encode() {
    assert_eq!(
      GpuEngineType::from_pdh_suffix("VideoEncode"),
      Some(GpuEngineType::VideoEncode)
    );
  }

  #[test]
  fn from_pdh_suffix_video_processing() {
    assert_eq!(
      GpuEngineType::from_pdh_suffix("VideoProcessing"),
      Some(GpuEngineType::VideoProcessing)
    );
  }

  #[test]
  fn from_pdh_suffix_unknown_returns_none() {
    assert_eq!(GpuEngineType::from_pdh_suffix("Compute"), None);
  }

  #[test]
  fn from_pdh_suffix_empty_returns_none() {
    assert_eq!(GpuEngineType::from_pdh_suffix(""), None);
  }

  #[test]
  fn from_pdh_suffix_wrong_case_returns_none() {
    assert_eq!(GpuEngineType::from_pdh_suffix("3d"), None);
    assert_eq!(GpuEngineType::from_pdh_suffix("copy"), None);
  }

  // -----------------------------------------------------------------------
  // Roundtrip: as_pdh_suffix → from_pdh_suffix
  // -----------------------------------------------------------------------

  #[test]
  fn roundtrip_all_variants() {
    let variants = [
      GpuEngineType::Graphics3D,
      GpuEngineType::Copy,
      GpuEngineType::VideoDecode,
      GpuEngineType::VideoEncode,
      GpuEngineType::VideoProcessing,
    ];
    for variant in variants {
      assert_eq!(
        GpuEngineType::from_pdh_suffix(variant.as_pdh_suffix()),
        Some(variant),
        "roundtrip failed for {variant:?}"
      );
    }
  }

  // -----------------------------------------------------------------------
  // is_valid_pdh_value
  // -----------------------------------------------------------------------

  #[test]
  fn valid_pdh_value_zero() {
    assert!(is_valid_pdh_value(0.0));
  }

  #[test]
  fn valid_pdh_value_mid() {
    assert!(is_valid_pdh_value(50.0));
  }

  #[test]
  fn valid_pdh_value_hundred() {
    assert!(is_valid_pdh_value(100.0));
  }

  #[test]
  fn invalid_pdh_value_below_range() {
    assert!(!is_valid_pdh_value(-0.001));
  }

  #[test]
  fn invalid_pdh_value_above_range() {
    assert!(!is_valid_pdh_value(100.001));
  }

  #[test]
  fn invalid_pdh_value_nan() {
    assert!(!is_valid_pdh_value(f64::NAN));
  }

  #[test]
  fn invalid_pdh_value_infinity() {
    assert!(!is_valid_pdh_value(f64::INFINITY));
    assert!(!is_valid_pdh_value(f64::NEG_INFINITY));
  }

  // -----------------------------------------------------------------------
  // parse_engine_type_from_instance
  // -----------------------------------------------------------------------

  #[test]
  fn parse_instance_3d() {
    let name = "pid_1234_luid_0x00_0x0000_phys_0_eng_0_engtype_3D";
    assert_eq!(
      parse_engine_type_from_instance(name),
      Some(GpuEngineType::Graphics3D)
    );
  }

  #[test]
  fn parse_instance_copy() {
    let name = "pid_5678_luid_0x00_0x0001_phys_0_eng_1_engtype_Copy";
    assert_eq!(
      parse_engine_type_from_instance(name),
      Some(GpuEngineType::Copy)
    );
  }

  #[test]
  fn parse_instance_video_decode() {
    let name = "pid_9999_luid_0x00_0x0002_phys_0_eng_2_engtype_VideoDecode";
    assert_eq!(
      parse_engine_type_from_instance(name),
      Some(GpuEngineType::VideoDecode)
    );
  }

  #[test]
  fn parse_instance_video_encode() {
    let name = "pid_1111_luid_0x00_0x0003_phys_0_eng_3_engtype_VideoEncode";
    assert_eq!(
      parse_engine_type_from_instance(name),
      Some(GpuEngineType::VideoEncode)
    );
  }

  #[test]
  fn parse_instance_video_processing() {
    let name = "pid_2222_luid_0x00_0x0004_phys_0_eng_4_engtype_VideoProcessing";
    assert_eq!(
      parse_engine_type_from_instance(name),
      Some(GpuEngineType::VideoProcessing)
    );
  }

  #[test]
  fn parse_instance_no_engtype_tag() {
    assert_eq!(parse_engine_type_from_instance("pid_1234_luid_0x00"), None);
  }

  #[test]
  fn parse_instance_empty_string() {
    assert_eq!(parse_engine_type_from_instance(""), None);
  }

  #[test]
  fn parse_instance_unknown_engine_type() {
    let name = "pid_1234_luid_0x00_phys_0_eng_0_engtype_Compute";
    assert_eq!(parse_engine_type_from_instance(name), None);
  }

  #[test]
  fn parse_instance_with_trailing_suffix() {
    // engtype_ followed by additional underscore-separated parts
    let name = "pid_1234_luid_0x00_phys_0_eng_0_engtype_3D_extra";
    assert_eq!(
      parse_engine_type_from_instance(name),
      Some(GpuEngineType::Graphics3D)
    );
  }

  #[test]
  fn parse_instance_multiple_engtype_uses_last() {
    // Two engtype_ tags — rfind should pick the last one
    let name = "engtype_Copy_something_engtype_VideoDecode";
    assert_eq!(
      parse_engine_type_from_instance(name),
      Some(GpuEngineType::VideoDecode)
    );
  }

  // -----------------------------------------------------------------------
  // aggregate_engine_usage
  // -----------------------------------------------------------------------

  #[test]
  fn aggregate_empty_input() {
    let mut cache = HashMap::new();
    aggregate_engine_usage(&mut cache, &[]);
    assert!(cache.is_empty());
  }

  #[test]
  fn aggregate_single_entry() {
    let mut cache = HashMap::new();
    let entries = [(GpuEngineType::Graphics3D, 50.0)];
    aggregate_engine_usage(&mut cache, &entries);
    assert_eq!(cache.get(&GpuEngineType::Graphics3D), Some(&0.5));
  }

  #[test]
  fn aggregate_same_engine_keeps_max() {
    let mut cache = HashMap::new();
    let entries = [
      (GpuEngineType::Copy, 20.0),
      (GpuEngineType::Copy, 80.0),
      (GpuEngineType::Copy, 50.0),
    ];
    aggregate_engine_usage(&mut cache, &entries);
    assert_eq!(cache.get(&GpuEngineType::Copy), Some(&0.8));
  }

  #[test]
  fn aggregate_multiple_engine_types() {
    let mut cache = HashMap::new();
    let entries = [
      (GpuEngineType::Graphics3D, 60.0),
      (GpuEngineType::VideoDecode, 30.0),
    ];
    aggregate_engine_usage(&mut cache, &entries);
    assert_eq!(cache.len(), 2);
    assert_eq!(cache.get(&GpuEngineType::Graphics3D), Some(&0.6));
    assert_eq!(cache.get(&GpuEngineType::VideoDecode), Some(&0.3));
  }

  #[test]
  fn aggregate_normalizes_hundred_to_one() {
    let mut cache = HashMap::new();
    let entries = [(GpuEngineType::VideoEncode, 100.0)];
    aggregate_engine_usage(&mut cache, &entries);
    assert_eq!(cache.get(&GpuEngineType::VideoEncode), Some(&1.0));
  }

  #[test]
  fn aggregate_normalizes_zero_to_zero() {
    let mut cache = HashMap::new();
    let entries = [(GpuEngineType::VideoProcessing, 0.0)];
    aggregate_engine_usage(&mut cache, &entries);
    assert_eq!(cache.get(&GpuEngineType::VideoProcessing), Some(&0.0));
  }

  #[test]
  fn aggregate_clears_existing_cache() {
    let mut cache = HashMap::new();
    cache.insert(GpuEngineType::Graphics3D, 0.99);
    cache.insert(GpuEngineType::Copy, 0.5);

    let entries = [(GpuEngineType::VideoDecode, 40.0)];
    aggregate_engine_usage(&mut cache, &entries);

    assert_eq!(cache.len(), 1);
    assert!(!cache.contains_key(&GpuEngineType::Graphics3D));
    assert!(!cache.contains_key(&GpuEngineType::Copy));
    assert_eq!(cache.get(&GpuEngineType::VideoDecode), Some(&0.4));
  }

  // -----------------------------------------------------------------------
  // parse_luid_from_instance
  // -----------------------------------------------------------------------

  #[test]
  fn parse_luid_standard() {
    let name = "pid_1234_luid_0x00_0x0000C61F_phys_0_eng_0_engtype_3D";
    assert_eq!(parse_luid_from_instance(name), Some((0, 0x0000C61F)));
  }

  #[test]
  fn parse_luid_zero() {
    let name = "pid_1234_luid_0x00_0x0000_phys_0_eng_0_engtype_3D";
    assert_eq!(parse_luid_from_instance(name), Some((0, 0)));
  }

  #[test]
  fn parse_luid_nonzero_high() {
    let name = "pid_1234_luid_0x01_0x0000ABCD_phys_0_eng_0_engtype_3D";
    assert_eq!(parse_luid_from_instance(name), Some((1, 0xABCD)));
  }

  #[test]
  fn parse_luid_no_tag() {
    assert_eq!(parse_luid_from_instance("pid_1234_phys_0_eng_0"), None);
  }

  #[test]
  fn parse_luid_empty() {
    assert_eq!(parse_luid_from_instance(""), None);
  }

  #[test]
  fn parse_luid_truncated() {
    // Only one hex part after luid_
    assert_eq!(parse_luid_from_instance("luid_0x00"), None);
  }

  // -----------------------------------------------------------------------
  // aggregate_engine_usage_per_luid
  // -----------------------------------------------------------------------

  #[test]
  fn aggregate_per_luid_empty() {
    let mut cache = HashMap::new();
    aggregate_engine_usage_per_luid(&mut cache, &[]);
    assert!(cache.is_empty());
  }

  #[test]
  fn aggregate_per_luid_single() {
    let mut cache = HashMap::new();
    let entries = [((0i32, 0xC61Fu32), GpuEngineType::Graphics3D, 50.0)];
    aggregate_engine_usage_per_luid(&mut cache, &entries);
    assert_eq!(
      cache.get(&(0, 0xC61F, GpuEngineType::Graphics3D)),
      Some(&0.5)
    );
  }

  #[test]
  fn aggregate_per_luid_different_gpus() {
    let mut cache = HashMap::new();
    let entries = [
      ((0, 0xAAAA), GpuEngineType::Graphics3D, 60.0),
      ((0, 0xBBBB), GpuEngineType::Graphics3D, 30.0),
    ];
    aggregate_engine_usage_per_luid(&mut cache, &entries);
    assert_eq!(
      cache.get(&(0, 0xAAAA, GpuEngineType::Graphics3D)),
      Some(&0.6)
    );
    assert_eq!(
      cache.get(&(0, 0xBBBB, GpuEngineType::Graphics3D)),
      Some(&0.3)
    );
  }

  #[test]
  fn aggregate_per_luid_same_gpu_keeps_max() {
    let mut cache = HashMap::new();
    let entries = [
      ((0, 0xAAAA), GpuEngineType::Graphics3D, 20.0),
      ((0, 0xAAAA), GpuEngineType::Graphics3D, 80.0),
    ];
    aggregate_engine_usage_per_luid(&mut cache, &entries);
    assert_eq!(
      cache.get(&(0, 0xAAAA, GpuEngineType::Graphics3D)),
      Some(&0.8)
    );
  }
}
