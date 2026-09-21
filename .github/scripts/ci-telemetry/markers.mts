// Parses machine-readable marker lines out of untrusted GitHub Actions job
// logs. A PR (including a fork or Dependabot PR) fully controls what a job
// prints, so every value here is treated as hostile input: only numbers,
// booleans, and closed enums survive validation. Free-form strings (job
// names, branch names, etc.) are handled separately in render.mts and are
// never accepted through a marker.

import type {
  ExtractedMarkers,
  JobResourcesMarker,
  NodeCacheMarker,
  RustCacheMarker,
  SccacheMarker,
} from "./schema.mts";

const MAX_LINE_LENGTH = 65536;
const MAX_SAFE = Number.MAX_SAFE_INTEGER;
const MAX_COUNT = 1e6;

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

// Returns the number when it is finite, >= 0, and <= max; otherwise
// undefined, which callers use as an "invalid" sentinel distinct from a
// legitimate null.
function finiteInRange(value: unknown, max: number): number | undefined {
  return typeof value === "number" &&
    Number.isFinite(value) &&
    value >= 0 &&
    value <= max
    ? value
    : undefined;
}

function clampPercent(value: unknown): number | undefined {
  if (typeof value !== "number" || !Number.isFinite(value)) return undefined;
  return Math.min(100, Math.max(0, value));
}

// Scans every line of `logText` for occurrences of `${markerName}=`, tries
// JSON.parse on the remainder of the line, and keeps the LAST occurrence
// that both parses and passes `validate`. This tolerates two things a PR (or
// the runner itself) can put in a log ahead of the real marker: a step's
// echoed script source containing the marker name followed by unparseable
// text, and timestamp-prefixed noise. Only a genuine last-valid occurrence
// wins, so an attacker cannot "override" a later, legitimate marker by
// printing an earlier one — but an attacker-controlled step run AFTER the
// real telemetry step could still print a forged, schema-valid marker later
// in the same job log; that risk is bounded by the validator only ever
// producing numbers/booleans/enums, never free text.
function findLastValidMarker<T>(
  logText: string,
  markerName: string,
  validate: (parsed: unknown) => T | null,
): T | null {
  let result: T | null = null;
  const needle = `${markerName}=`;
  const lines = logText.split("\n");
  for (const rawLine of lines) {
    if (rawLine.length > MAX_LINE_LENGTH) continue;
    let searchFrom = 0;
    for (;;) {
      const idx = rawLine.indexOf(needle, searchFrom);
      if (idx === -1) break;
      searchFrom = idx + needle.length;
      // Require a token boundary before the match so that, e.g., scanning
      // for "CACHE_METRICS_JSON=" does not also match the tail of
      // "RUST_CACHE_METRICS_JSON=" or "NODE_CACHE_METRICS_JSON=" on the same
      // line.
      const boundaryChar = idx > 0 ? rawLine[idx - 1] : undefined;
      if (boundaryChar !== undefined && /[A-Za-z0-9_]/.test(boundaryChar)) {
        continue;
      }
      const candidate = rawLine.slice(searchFrom).trim();
      let parsed: unknown;
      try {
        parsed = JSON.parse(candidate);
      } catch {
        continue;
      }
      const validated = validate(parsed);
      if (validated !== null) {
        result = validated;
      }
    }
  }
  return result;
}

function validateCpuPct(value: unknown): JobResourcesMarker["cpu_pct"] {
  if (!isPlainObject(value)) return null;
  const avg = clampPercent(value["avg"]);
  const p95 = clampPercent(value["p95"]);
  const max = clampPercent(value["max"]);
  if (avg === undefined || p95 === undefined || max === undefined) {
    return null;
  }
  return { avg, p95, max };
}

// Shared by mem_used_bytes ({avg,max}) and disk_used_bytes ({start,end,max}):
// every key must resolve to a finite non-negative number or the whole object
// is rejected, never partially filled in.
function validateByteObject<K extends string>(
  value: unknown,
  keys: readonly K[],
): Record<K, number> | null {
  if (!isPlainObject(value)) return null;
  const result = {} as Record<K, number>;
  for (const key of keys) {
    const num = finiteInRange(value[key], MAX_SAFE);
    if (num === undefined) return null;
    result[key] = num;
  }
  return result;
}

const MAX_CPU_THREADS = 64;
const MAX_TIMELINE_THREADS = 8;

function validateCpuThreadStat(
  value: unknown,
): { avg: number; max: number } | null {
  if (!isPlainObject(value)) return null;
  const avg = clampPercent(value["avg"]);
  const max = clampPercent(value["max"]);
  if (avg === undefined || max === undefined) return null;
  return { avg, max };
}

// A PR-controlled job can print anything here. Unlike most fields in this
// module, an invalid cpu_threads does not reject the whole marker — it just
// nulls this one field, since the rest of the marker (whole-VM CPU, memory,
// disk) is still trustworthy on its own. A per-entry `null` is legitimate
// data (that one thread was never measured) and is kept as-is; only a
// non-null entry that fails to validate nulls the WHOLE field, since there
// is no way to tell a genuinely malformed entry from a forged one.
function validateCpuThreads(value: unknown): JobResourcesMarker["cpu_threads"] {
  if (!Array.isArray(value) || value.length > MAX_CPU_THREADS) return null;
  const result: ({ avg: number; max: number } | null)[] = [];
  for (const item of value) {
    if (item === null) {
      result.push(null);
      continue;
    }
    const stat = validateCpuThreadStat(item);
    if (stat === null) return null;
    result.push(stat);
  }
  return result;
}

// Same shape rule as validateTimelineArray (length <= 120, items null or
// 0..100), applied to at most MAX_TIMELINE_THREADS arrays. Any invalid
// array, or too many of them, nulls only this field — the rest of the
// timeline (cpu_pct_avg/cpu_pct_max/mem_used_pct_max) is kept regardless.
function validateCpuThreadTimeline(value: unknown): (number | null)[][] | null {
  if (!Array.isArray(value) || value.length > MAX_TIMELINE_THREADS) {
    return null;
  }
  const result: (number | null)[][] = [];
  for (const arr of value) {
    const validated = validateTimelineArray(arr);
    if (validated === undefined) return null;
    result.push(validated);
  }
  return result;
}

function validateTimelineArray(arr: unknown): (number | null)[] | undefined {
  if (!Array.isArray(arr) || arr.length > 120) return undefined;
  const result: (number | null)[] = [];
  for (const item of arr) {
    if (item === null) {
      result.push(null);
      continue;
    }
    if (
      typeof item !== "number" ||
      !Number.isFinite(item) ||
      item < 0 ||
      item > 100
    ) {
      return undefined;
    }
    result.push(item);
  }
  return result;
}

function validateTimeline(value: unknown): JobResourcesMarker["timeline"] {
  if (!isPlainObject(value)) return null;
  const bucketSeconds = finiteInRange(value["bucket_seconds"], MAX_COUNT);
  const cpuAvg = validateTimelineArray(value["cpu_pct_avg"]);
  const cpuMax = validateTimelineArray(value["cpu_pct_max"]);
  const memMax = validateTimelineArray(value["mem_used_pct_max"]);
  if (
    bucketSeconds === undefined ||
    cpuAvg === undefined ||
    cpuMax === undefined ||
    memMax === undefined
  ) {
    return null;
  }
  return {
    bucket_seconds: bucketSeconds,
    cpu_pct_avg: cpuAvg,
    cpu_pct_max: cpuMax,
    mem_used_pct_max: memMax,
    cpu_thread_pct_avg: validateCpuThreadTimeline(value["cpu_thread_pct_avg"]),
  };
}

const KNOWN_RUNNER_OS = ["Linux", "Windows", "macOS"] as const;
const KNOWN_RUNNER_ARCH = ["X86", "X64", "ARM", "ARM64"] as const;

function isKnownMember<T extends string>(
  candidates: readonly T[],
  value: unknown,
): value is T {
  return (
    typeof value === "string" &&
    (candidates as readonly string[]).includes(value)
  );
}

function validateResources(parsed: unknown): JobResourcesMarker | null {
  if (!isPlainObject(parsed) || parsed["schema"] !== 1) return null;

  const cpuCount = finiteInRange(parsed["cpu_count"], MAX_COUNT);
  const intervalSeconds = finiteInRange(parsed["interval_seconds"], MAX_COUNT);
  const sampleCount = finiteInRange(parsed["sample_count"], MAX_COUNT);
  const durationSeconds = finiteInRange(parsed["duration_seconds"], MAX_COUNT);
  // Null only when the sampler recorded nothing; unavailable is never 0.
  const memTotalBytesRaw = parsed["mem_total_bytes"];
  const memTotalBytes =
    memTotalBytesRaw === null
      ? null
      : finiteInRange(memTotalBytesRaw, MAX_SAFE);

  // These five are required scalars in the schema. A missing or
  // out-of-range value here means the whole marker is untrustworthy, so
  // reject it rather than guess a fallback.
  if (
    cpuCount === undefined ||
    intervalSeconds === undefined ||
    sampleCount === undefined ||
    durationSeconds === undefined ||
    memTotalBytes === undefined
  ) {
    return null;
  }

  const runnerOsRaw = parsed["runner_os"];
  const runnerArchRaw = parsed["runner_arch"];

  return {
    schema: 1,
    runner_os: isKnownMember(KNOWN_RUNNER_OS, runnerOsRaw)
      ? runnerOsRaw
      : "unknown",
    runner_arch: isKnownMember(KNOWN_RUNNER_ARCH, runnerArchRaw)
      ? runnerArchRaw
      : "unknown",
    cpu_count: cpuCount,
    interval_seconds: intervalSeconds,
    sample_count: sampleCount,
    duration_seconds: durationSeconds,
    cpu_pct: validateCpuPct(parsed["cpu_pct"]),
    cpu_threads: validateCpuThreads(parsed["cpu_threads"]),
    mem_total_bytes: memTotalBytes,
    mem_used_bytes: validateByteObject(parsed["mem_used_bytes"], [
      "avg",
      "max",
    ] as const),
    disk_total_bytes:
      finiteInRange(parsed["disk_total_bytes"], MAX_SAFE) ?? null,
    disk_used_bytes: validateByteObject(parsed["disk_used_bytes"], [
      "start",
      "end",
      "max",
    ] as const),
    timeline: validateTimeline(parsed["timeline"]),
  };
}

function clampPercentOrNull(value: unknown): number | null {
  if (value === null) return null;
  const clamped = clampPercent(value);
  return clamped === undefined ? null : clamped;
}

function deriveSccacheBackend(
  cacheLocation: unknown,
): SccacheMarker["backend"] {
  if (typeof cacheLocation !== "string") return "other";
  if (cacheLocation.startsWith("s3")) return "s3";
  if (cacheLocation.startsWith("Local disk") || cacheLocation === "disk") {
    return "disk";
  }
  return "other";
}

function deriveExactHitTristate(value: unknown): boolean | null {
  if (value === "true") return true;
  if (value === "false") return false;
  return null;
}

function validateSccache(parsed: unknown): SccacheMarker | null {
  if (!isPlainObject(parsed)) return null;

  const compileRequests = finiteInRange(parsed["compile_requests"], 1e9);
  const compileRequestsExecuted = finiteInRange(
    parsed["compile_requests_executed"],
    1e9,
  );
  const rustHits = finiteInRange(parsed["rust_hits"], 1e9);
  const rustMisses = finiteInRange(parsed["rust_misses"], 1e9);
  const cppHits = finiteInRange(parsed["cpp_hits"], 1e9);
  const cppMisses = finiteInRange(parsed["cpp_misses"], 1e9);
  const cacheErrors = finiteInRange(parsed["cache_errors"], 1e9);

  if (
    compileRequests === undefined ||
    compileRequestsExecuted === undefined ||
    rustHits === undefined ||
    rustMisses === undefined ||
    cppHits === undefined ||
    cppMisses === undefined ||
    cacheErrors === undefined
  ) {
    return null;
  }

  return {
    compile_requests: compileRequests,
    compile_requests_executed: compileRequestsExecuted,
    rust_hits: rustHits,
    rust_misses: rustMisses,
    rust_hit_rate_pct: clampPercentOrNull(parsed["rust_hit_rate_pct"]),
    cpp_hits: cppHits,
    cpp_misses: cppMisses,
    cpp_hit_rate_pct: clampPercentOrNull(parsed["cpp_hit_rate_pct"]),
    cache_errors: cacheErrors,
    backend: deriveSccacheBackend(parsed["cache_location"]),
    target_cache_exact_hit: deriveExactHitTristate(
      parsed["target_cache_exact_hit"],
    ),
  };
}

function validateRustCache(parsed: unknown): RustCacheMarker | null {
  if (!isPlainObject(parsed)) return null;
  const exactHit = parsed["exact_hit"];
  const targets = parsed["targets"];
  if (typeof exactHit !== "boolean" || typeof targets !== "boolean") {
    return null;
  }
  const restoreSeconds = finiteInRange(parsed["restore_seconds"], 86400);
  if (restoreSeconds === undefined) return null;

  const sharedKeyRaw = parsed["shared_key"];
  const sharedKey =
    typeof sharedKeyRaw === "string" && /^[a-z0-9-]{1,40}$/.test(sharedKeyRaw)
      ? sharedKeyRaw
      : null;

  return {
    shared_key: sharedKey,
    exact_hit: exactHit,
    restore_seconds: restoreSeconds,
    targets,
  };
}

function validateNodeCache(parsed: unknown): NodeCacheMarker | null {
  if (!isPlainObject(parsed)) return null;
  const exactHit = parsed["exact_hit"];
  if (typeof exactHit !== "boolean") return null;
  return { exact_hit: exactHit };
}

export function extractMarkers(logText: string): ExtractedMarkers {
  const text = typeof logText === "string" ? logText : "";
  return {
    resources: findLastValidMarker(
      text,
      "CI_JOB_METRICS_JSON",
      validateResources,
    ),
    sccache: findLastValidMarker(text, "CACHE_METRICS_JSON", validateSccache),
    rust_cache: findLastValidMarker(
      text,
      "RUST_CACHE_METRICS_JSON",
      validateRustCache,
    ),
    node_cache: findLastValidMarker(
      text,
      "NODE_CACHE_METRICS_JSON",
      validateNodeCache,
    ),
  };
}
