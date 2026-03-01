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

// ---------------------------------------------------------------------------
// Core collection logic (runs inside `spawn_blocking`)
// ---------------------------------------------------------------------------

fn collect_and_read(engine_type: GpuEngineType) -> Result<f32, Box<dyn Error + Send>> {
  let init_result = PDH_STATE.get_or_init(|| match init_pdh_state() {
    Ok(state) => Ok(Mutex::new(state)),
    Err(e) => {
      log_error!(
        "init_error",
        "pdh_provider::collect_and_read",
        Some(e.to_string())
      );
      Err(e.to_string())
    }
  });

  let mtx = init_result
    .as_ref()
    .map_err(|e| pdh_err(format!("PDH init failed: {e}")))?;

  let mut guard = mtx
    .lock()
    .map_err(|e| pdh_err(format!("PDH_STATE lock poisoned: {e}")))?;

  let state = &mut *guard;

  // Return cached value if the last collect is recent enough.
  if state.cache_time.elapsed() < CACHE_TTL
    && let Some(&v) = state.cache.get(&engine_type)
  {
    return Ok(v);
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

    // --- walk all instances, build per-engine-type cache ---
    let items = std::slice::from_raw_parts(
      state.buf.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
      item_count as usize,
    );

    state.cache.clear();
    let engtype_tag = "engtype_";

    for item in items {
      // Skip items with invalid CStatus.
      let cstatus = item.FmtValue.CStatus;
      if cstatus != PDH_CSTATUS_VALID_DATA && cstatus != PDH_CSTATUS_NEW_DATA {
        continue;
      }

      let raw = item.FmtValue.Anonymous.doubleValue;

      // Skip non-finite or out-of-range values.
      if !raw.is_finite() || !(0.0..=100.0).contains(&raw) {
        continue;
      }

      let name = pwstr_to_string(item.szName.0);
      if let Some(pos) = name.rfind(engtype_tag) {
        let mut suffix = &name[pos + engtype_tag.len()..];
        suffix = suffix.split('_').next().unwrap_or(suffix);
        if let Some(etype) = GpuEngineType::from_pdh_suffix(suffix) {
          let value = (raw as f32 / 100.0).clamp(0.0, 1.0);
          let entry = state.cache.entry(etype).or_insert(0.0f32);
          if value > *entry {
            *entry = value;
          }
        }
      }
    }

    state.cache_time = Instant::now();
  }

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
