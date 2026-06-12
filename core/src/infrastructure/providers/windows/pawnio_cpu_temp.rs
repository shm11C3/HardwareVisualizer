//! Native CPU package-temperature provider via PawnIO (Windows).
//!
//! Phase 1 of issue #1635. Reads the package temperature directly from
//! the CPU instead of ACPI thermal zones:
//!
//! - **Intel**: digital thermal sensor MSRs through the PawnIO
//!   `IntelMSR` module (`docs/specs/sensors/cpu-intel-dts-msr.md`, rev 2)
//! - **AMD Zen**: `THM_TCON_CUR_TMP` over SMN through the PawnIO
//!   `RyzenSMU` module (`docs/specs/sensors/cpu-amd-zen-smn.md`, rev 3)
//! - PawnIO client contract: `docs/specs/sensors/pawnio-interface.md`
//!   (rev 2)
//!
//! PawnIO IOCTLs are blocking, so all driver work is confined to one
//! dedicated sampler thread (the `thermal_zone` pattern): the collector
//! reads the latest value from a process-wide cache and never blocks.
//! The cache enforces a freshness window so a stalled sampler stops
//! feeding the headline value and the ACPI fallback takes over.
//!
//! Graceful degradation: when PawnIO is not installed (or the module
//! blob is missing) the sampler logs once, reports unavailable, and
//! re-probes at a slow idle cadence, leaving the ACPI thermal-zone
//! source (#1633) in charge. A CPU that can never be supported (wrong
//! vendor/family/feature bits, zero Temperature Target) ends the
//! sampler outright.
//!
//! Register access is read-only: the only module functions ever invoked
//! are `ioctl_read_msr` and `ioctl_read_smu_register`. Per the interface
//! spec, Intel MSR reads need no ecosystem mutex, while every `RyzenSMU`
//! call is wrapped in the caller-held `Global\Access_PCI` mutex with a
//! bounded timeout — a timeout skips the sample, never reads unlocked.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::named_mutex::NamedMutex;
use super::pawnio::{self, PawnIoError, PawnIoModule};
use crate::utils::cpu_thermal::{self, CpuSensorPlan};
use crate::{log_debug, log_error, log_info};

/// Cadence of the sampler thread; matches the thermal-zone sampler so
/// the two temperature sources age at the same rate.
const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);

/// After this many consecutive failed reads the session is torn down
/// (handles dropped) and detection restarts after [`IDLE_INTERVAL`].
const FAILURES_BEFORE_TEARDOWN: u32 = 5;

/// Re-probe cadence while PawnIO is unavailable. Keeps a PawnIO
/// install during the session discoverable without polling noise.
const IDLE_INTERVAL: Duration = Duration::from_secs(60);

/// Readings older than this are not served from the cache; the caller
/// then falls back to ACPI zones. Covers a few skipped samples without
/// letting a dead sampler pin a stale headline value.
const STALE_AFTER: Duration = Duration::from_secs(10);

/// Bounded wait for `Global\Access_PCI` (spec: bounded timeout, and a
/// timeout means a skipped sample). The exact bound is project policy.
const PCI_MUTEX_TIMEOUT: Duration = Duration::from_millis(500);

/// Ecosystem mutex the caller must hold around each `RyzenSMU` ioctl.
const PCI_MUTEX_NAME: &str = r"Global\Access_PCI";

/// Sensor-list display name of the PawnIO-derived package reading.
pub const CPU_PACKAGE_SENSOR_NAME: &str = "CPU Package";

/// Latest successful package reading with its timestamp.
struct PackageReading {
  celsius: f32,
  read_at: Instant,
}

/// Latest reading (`None` while unavailable). Guarded by a `Mutex` for
/// the non-blocking collector accessor.
static LATEST_READING: OnceLock<Mutex<Option<PackageReading>>> = OnceLock::new();

/// Indicates whether the sampler thread is running. Reset to `false`
/// when the spawn fails so a later collector tick can retry instead of
/// latching the sampler off for the rest of the process.
static SAMPLER_STARTED: AtomicBool = AtomicBool::new(false);

/// Start the background sampler thread. No-op if already running.
pub fn init_pawnio_cpu_temp_sampler() {
  let _ = LATEST_READING.get_or_init(|| Mutex::new(None));

  // Only the caller that flips the flag spawns; concurrent callers bail.
  if SAMPLER_STARTED
    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
    .is_err()
  {
    return;
  }

  if let Err(e) = std::thread::Builder::new()
    .name("pawnio-cpu-temp-sampler".to_string())
    .spawn(sampler_loop)
  {
    SAMPLER_STARTED.store(false, Ordering::Release);
    log_error!(
      "failed to spawn pawnio-cpu-temp-sampler thread",
      "pawnio_cpu_temp::init_pawnio_cpu_temp_sampler",
      Some(e.to_string())
    );
  }
}

/// Latest CPU package temperature in raw °C, or `None` when no fresh
/// PawnIO reading exists (driver absent, unsupported CPU, stalled
/// sampler). Never blocks.
pub fn read_cpu_package_temperature_cached() -> Option<f32> {
  let reading = LATEST_READING.get()?.lock().ok()?;
  reading
    .as_ref()
    .and_then(|r| (r.read_at.elapsed() <= STALE_AFTER).then_some(r.celsius))
}

fn store_reading(reading: Option<f32>) {
  if let Some(latest) = LATEST_READING.get()
    && let Ok(mut guard) = latest.lock()
  {
    *guard = reading.map(|celsius| PackageReading {
      celsius,
      read_at: Instant::now(),
    });
  }
}

/// How a session ended, deciding what the sampler does next.
enum SessionEnd {
  /// Recoverable (PawnIO missing, repeated read failures): drop all
  /// handles, idle, then re-detect.
  Retry,
  /// The CPU can never produce a reading (e.g. zero Temperature
  /// Target): the sampler thread ends for the process lifetime.
  Unsupported,
}

/// Dedicated thread: classifies the CPU once, then owns the PawnIO
/// handles and refreshes [`LATEST_READING`] until process exit.
fn sampler_loop() {
  let plan = detect_cpu();
  match plan {
    CpuSensorPlan::Unsupported => {
      log_debug!(
        "CPU is not a supported PawnIO temperature target; ACPI thermal zones remain the source",
        "pawnio_cpu_temp::sampler_loop",
        None::<&str>
      );
      return;
    }
    CpuSensorPlan::AmdFamilyDisabled { family } => {
      // Recognized by the RyzenSMU module but not yet verified by the
      // spec (scoped-enablement table) — disabled by default.
      log_info!(
        "AMD family recognized but disabled pending spec verification; using ACPI thermal zones",
        "pawnio_cpu_temp::sampler_loop",
        Some(format!("family {family:#x}"))
      );
      return;
    }
    CpuSensorPlan::IntelDts | CpuSensorPlan::AmdZenSmn { .. } => {}
  }

  let mut establish_failure_logged = false;
  loop {
    match run_session(plan, &mut establish_failure_logged) {
      SessionEnd::Unsupported => {
        store_reading(None);
        return;
      }
      SessionEnd::Retry => {
        store_reading(None);
        std::thread::sleep(IDLE_INTERVAL);
      }
    }
  }
}

/// Log the first establishment failure (then stay quiet until a session
/// succeeds again) and signal a retry.
fn note_establish_failure(detail: String, logged: &mut bool) -> SessionEnd {
  if !*logged {
    log_info!(
      "PawnIO unavailable; CPU package temperature stays on ACPI thermal zones",
      "pawnio_cpu_temp::run_session",
      Some(detail)
    );
    *logged = true;
  }
  SessionEnd::Retry
}

/// Establish one PawnIO session (library + module blob + executor
/// handle, all owned by this stack frame) and run its read loop.
fn run_session(plan: CpuSensorPlan, establish_failure_logged: &mut bool) -> SessionEnd {
  let (module_name, source_label) = match plan {
    CpuSensorPlan::IntelDts => ("IntelMSR", "Intel DTS"),
    CpuSensorPlan::AmdZenSmn { .. } => ("RyzenSMU", "AMD Zen SMN"),
    // Guarded by sampler_loop; kept total for safety.
    _ => return SessionEnd::Unsupported,
  };

  let lib = match pawnio::PawnIo::load_installed() {
    Ok(lib) => lib,
    Err(e) => return note_establish_failure(e.to_string(), establish_failure_logged),
  };
  let blob = match pawnio::read_module_blob(module_name) {
    Ok(blob) => blob,
    Err(e) => return note_establish_failure(e.to_string(), establish_failure_logged),
  };
  // One executor handle per module; lives until the session ends.
  let module = match lib.open_module(&blob) {
    Ok(module) => module,
    Err(e) => return note_establish_failure(e.to_string(), establish_failure_logged),
  };

  log_info!(
    &format!(
      "PawnIO CPU package temperature sampler active ({source_label} via {module_name}, PawnIOLib {})",
      lib
        .version_string()
        .unwrap_or_else(|| "unknown".to_string())
    ),
    "pawnio_cpu_temp::run_session",
    None::<&str>
  );
  *establish_failure_logged = false;

  match plan {
    CpuSensorPlan::IntelDts => run_intel_session(&module),
    CpuSensorPlan::AmdZenSmn { .. } => run_amd_session(&module),
    _ => SessionEnd::Unsupported,
  }
}

/// Intel DTS session: resolve the Temperature Target once, then decode
/// `IA32_PACKAGE_THERM_STATUS` every tick.
///
/// `0x1B1` is package-scope (identical from any logical CPU of the
/// package) and PawnIO executes `RDMSR` on the calling thread's current
/// processor; Phase 1 targets single-package machines, so no thread
/// affinity pinning is needed.
fn run_intel_session(module: &PawnIoModule<'_>) -> SessionEnd {
  // Spec read procedure step 2: read MSR_TEMPERATURE_TARGET once per
  // session. A faulted read means "unsupported" for this session — the
  // idle retry keeps a transient driver hiccup recoverable.
  let msr_1a2 = match read_msr(module, cpu_thermal::MSR_TEMPERATURE_TARGET) {
    Ok(value) => value,
    Err(e) => {
      log_debug!(
        "MSR_TEMPERATURE_TARGET read faulted; treating Intel DTS as unsupported this session",
        "pawnio_cpu_temp::run_intel_session",
        Some(e.to_string())
      );
      return SessionEnd::Retry;
    }
  };
  let Some(t_target) = cpu_thermal::intel_temperature_target(msr_1a2) else {
    log_info!(
      "Temperature Target field is zero; Intel DTS unsupported on this part",
      "pawnio_cpu_temp::run_intel_session",
      None::<&str>
    );
    return SessionEnd::Unsupported;
  };
  if !cpu_thermal::intel_t_target_plausible(t_target) {
    log_info!(
      "Temperature Target outside the 50-120 °C plausibility bounds; Intel DTS unsupported",
      "pawnio_cpu_temp::run_intel_session",
      Some(format!("t_target = {t_target} °C"))
    );
    return SessionEnd::Unsupported;
  }

  run_read_loop(|| {
    read_msr(module, cpu_thermal::MSR_IA32_PACKAGE_THERM_STATUS)
      .ok()
      .and_then(|value| cpu_thermal::intel_package_temperature(t_target, value))
  })
}

/// AMD Zen session: read `THM_TCON_CUR_TMP` over SMN every tick, under
/// the caller-held `Global\Access_PCI` mutex, and publish Tdie.
fn run_amd_session(module: &PawnIoModule<'_>) -> SessionEnd {
  // The Tctl − Tdie offset is keyed by product name (OPN) and constant
  // for the session; unlisted products use the documented default of 0.
  let tctl_offset = cpu_thermal::amd_tctl_offset(&read_brand_string());

  let Some(pci_mutex) = NamedMutex::create(PCI_MUTEX_NAME) else {
    log_debug!(
      "failed to create/open the Access_PCI mutex; skipping this session",
      "pawnio_cpu_temp::run_amd_session",
      None::<&str>
    );
    return SessionEnd::Retry;
  };

  run_read_loop(|| {
    // Hold the mutex for exactly one self-contained register read; a
    // wait timeout skips the sample (never proceed unlocked).
    let raw = {
      let _guard = pci_mutex.acquire(PCI_MUTEX_TIMEOUT)?;
      module
        .execute(
          c"ioctl_read_smu_register",
          &[cpu_thermal::AMD_SMN_THM_TCON_CUR_TMP],
          1,
        )
        .ok()?[0]
    };
    // out[0] carries the 32-bit register value in a 64-bit cell.
    cpu_thermal::amd_tdie_celsius(raw as u32, tctl_offset)
  })
}

/// Shared per-sample loop: publish successes, count failures, and tear
/// the session down after [`FAILURES_BEFORE_TEARDOWN`] misses in a row.
/// Single failures keep the last good value — the cache freshness
/// window ages it out if the gap persists.
fn run_read_loop(mut read_once: impl FnMut() -> Option<f32>) -> SessionEnd {
  let mut consecutive_failures: u32 = 0;
  loop {
    match read_once() {
      Some(celsius) => {
        consecutive_failures = 0;
        store_reading(Some(celsius));
      }
      None => {
        consecutive_failures = consecutive_failures.saturating_add(1);
        if consecutive_failures >= FAILURES_BEFORE_TEARDOWN {
          log_debug!(
            "repeated PawnIO read failures; tearing the session down for idle re-detection",
            "pawnio_cpu_temp::run_read_loop",
            None::<&str>
          );
          return SessionEnd::Retry;
        }
      }
    }
    std::thread::sleep(SAMPLE_INTERVAL);
  }
}

/// Read one allow-listed MSR through the `IntelMSR` module
/// (`ioctl_read_msr`: 1 input cell = MSR index, 1 output cell = value).
fn read_msr(module: &PawnIoModule<'_>, msr: u64) -> Result<u64, PawnIoError> {
  let output = module.execute(c"ioctl_read_msr", &[msr], 1)?;
  Ok(output[0])
}

/// Classify the host CPU from CPUID (vendor, family, thermal feature
/// bits). Leaves beyond the maximum supported leaf are passed as 0,
/// which the pure classifier treats as "feature absent".
#[cfg(target_arch = "x86_64")]
fn detect_cpu() -> CpuSensorPlan {
  use std::arch::x86_64::__cpuid;
  let leaf0 = __cpuid(0);
  let vendor = cpu_thermal::cpuid_vendor_string(leaf0.ebx, leaf0.ecx, leaf0.edx);
  let max_standard_leaf = leaf0.eax;
  let leaf1_eax = if max_standard_leaf >= 1 {
    __cpuid(1).eax
  } else {
    0
  };
  let leaf06_eax = if max_standard_leaf >= 6 {
    __cpuid(6).eax
  } else {
    0
  };
  cpu_thermal::plan_cpu_sensor(&vendor, leaf1_eax, leaf06_eax)
}

/// PawnIO targets x86-64 CPUs; other architectures report unsupported.
#[cfg(not(target_arch = "x86_64"))]
fn detect_cpu() -> CpuSensorPlan {
  CpuSensorPlan::Unsupported
}

/// CPU brand string from CPUID leaves `0x80000002..=0x80000004`; empty
/// when the extended leaves are unavailable (then the Tctl offset
/// lookup falls through to the default of 0).
#[cfg(target_arch = "x86_64")]
fn read_brand_string() -> String {
  use std::arch::x86_64::__cpuid;
  if __cpuid(0x8000_0000).eax < 0x8000_0004 {
    return String::new();
  }
  let mut regs = [0u32; 12];
  for (i, leaf) in (0x8000_0002u32..=0x8000_0004).enumerate() {
    let result = __cpuid(leaf);
    regs[i * 4] = result.eax;
    regs[i * 4 + 1] = result.ebx;
    regs[i * 4 + 2] = result.ecx;
    regs[i * 4 + 3] = result.edx;
  }
  cpu_thermal::cpuid_brand_string(&regs)
}

#[cfg(not(target_arch = "x86_64"))]
fn read_brand_string() -> String {
  String::new()
}
