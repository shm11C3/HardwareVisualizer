// Parses machine-readable marker lines out of untrusted GitHub Actions job
// logs. A PR (including a fork or Dependabot PR) fully controls what a job
// prints, so every value here is treated as hostile input: only numbers,
// booleans, and closed enums survive validation. Free-form strings (job
// names, branch names, etc.) are handled separately in render.cjs and are
// never accepted through a marker.
"use strict";

const MAX_LINE_LENGTH = 65536;
const MAX_SAFE = Number.MAX_SAFE_INTEGER;
const MAX_COUNT = 1e6;

function isPlainObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

// Returns the number when it is finite, >= 0, and <= max; otherwise
// undefined, which callers use as an "invalid" sentinel distinct from a
// legitimate null.
function finiteInRange(value, max) {
  return typeof value === "number" &&
    Number.isFinite(value) &&
    value >= 0 &&
    value <= max
    ? value
    : undefined;
}

function clampPercent(value) {
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
function findLastValidMarker(logText, markerName, validate) {
  let result = null;
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
      let parsed;
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

function validateCpuPct(value) {
  if (!isPlainObject(value)) return null;
  const avg = clampPercent(value.avg);
  const p95 = clampPercent(value.p95);
  const max = clampPercent(value.max);
  if (avg === undefined || p95 === undefined || max === undefined) {
    return null;
  }
  return { avg, p95, max };
}

function validateByteObject(value, keys) {
  if (!isPlainObject(value)) return null;
  const result = {};
  for (const key of keys) {
    const num = finiteInRange(value[key], MAX_SAFE);
    if (num === undefined) return null;
    result[key] = num;
  }
  return result;
}

function validateTimelineArray(arr) {
  if (!Array.isArray(arr) || arr.length > 120) return undefined;
  const result = [];
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

function validateTimeline(value) {
  if (!isPlainObject(value)) return null;
  const bucketSeconds = finiteInRange(value.bucket_seconds, MAX_COUNT);
  const cpuAvg = validateTimelineArray(value.cpu_pct_avg);
  const cpuMax = validateTimelineArray(value.cpu_pct_max);
  const memMax = validateTimelineArray(value.mem_used_pct_max);
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
  };
}

function validateResources(parsed) {
  if (!isPlainObject(parsed) || parsed.schema !== 1) return null;

  const cpuCount = finiteInRange(parsed.cpu_count, MAX_COUNT);
  const intervalSeconds = finiteInRange(parsed.interval_seconds, MAX_COUNT);
  const sampleCount = finiteInRange(parsed.sample_count, MAX_COUNT);
  const durationSeconds = finiteInRange(parsed.duration_seconds, MAX_COUNT);
  // Null only when the sampler recorded nothing; unavailable is never 0.
  const memTotalBytes =
    parsed.mem_total_bytes === null
      ? null
      : finiteInRange(parsed.mem_total_bytes, MAX_SAFE);

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

  return {
    schema: 1,
    runner_os: ["Linux", "Windows", "macOS"].includes(parsed.runner_os)
      ? parsed.runner_os
      : "unknown",
    runner_arch: ["X86", "X64", "ARM", "ARM64"].includes(parsed.runner_arch)
      ? parsed.runner_arch
      : "unknown",
    cpu_count: cpuCount,
    interval_seconds: intervalSeconds,
    sample_count: sampleCount,
    duration_seconds: durationSeconds,
    cpu_pct: validateCpuPct(parsed.cpu_pct),
    mem_total_bytes: memTotalBytes,
    mem_used_bytes: validateByteObject(parsed.mem_used_bytes, ["avg", "max"]),
    disk_total_bytes: finiteInRange(parsed.disk_total_bytes, MAX_SAFE) ?? null,
    disk_used_bytes: validateByteObject(parsed.disk_used_bytes, [
      "start",
      "end",
      "max",
    ]),
    timeline: validateTimeline(parsed.timeline),
  };
}

function clampPercentOrNull(value) {
  if (value === null) return null;
  const clamped = clampPercent(value);
  return clamped === undefined ? null : clamped;
}

function deriveSccacheBackend(cacheLocation) {
  if (typeof cacheLocation !== "string") return "other";
  if (cacheLocation.startsWith("s3")) return "s3";
  if (cacheLocation.startsWith("Local disk") || cacheLocation === "disk") {
    return "disk";
  }
  return "other";
}

function deriveExactHitTristate(value) {
  if (value === "true") return true;
  if (value === "false") return false;
  return null;
}

function validateSccache(parsed) {
  if (!isPlainObject(parsed)) return null;

  const compileRequests = finiteInRange(parsed.compile_requests, 1e9);
  const compileRequestsExecuted = finiteInRange(
    parsed.compile_requests_executed,
    1e9,
  );
  const rustHits = finiteInRange(parsed.rust_hits, 1e9);
  const rustMisses = finiteInRange(parsed.rust_misses, 1e9);
  const cppHits = finiteInRange(parsed.cpp_hits, 1e9);
  const cppMisses = finiteInRange(parsed.cpp_misses, 1e9);
  const cacheErrors = finiteInRange(parsed.cache_errors, 1e9);

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
    rust_hit_rate_pct: clampPercentOrNull(parsed.rust_hit_rate_pct),
    cpp_hits: cppHits,
    cpp_misses: cppMisses,
    cpp_hit_rate_pct: clampPercentOrNull(parsed.cpp_hit_rate_pct),
    cache_errors: cacheErrors,
    backend: deriveSccacheBackend(parsed.cache_location),
    target_cache_exact_hit: deriveExactHitTristate(
      parsed.target_cache_exact_hit,
    ),
  };
}

function validateRustCache(parsed) {
  if (!isPlainObject(parsed)) return null;
  if (
    typeof parsed.exact_hit !== "boolean" ||
    typeof parsed.targets !== "boolean"
  ) {
    return null;
  }
  const restoreSeconds = finiteInRange(parsed.restore_seconds, 86400);
  if (restoreSeconds === undefined) return null;

  const sharedKey =
    typeof parsed.shared_key === "string" &&
    /^[a-z0-9-]{1,40}$/.test(parsed.shared_key)
      ? parsed.shared_key
      : null;

  return {
    shared_key: sharedKey,
    exact_hit: parsed.exact_hit,
    restore_seconds: restoreSeconds,
    targets: parsed.targets,
  };
}

function validateNodeCache(parsed) {
  if (!isPlainObject(parsed) || typeof parsed.exact_hit !== "boolean") {
    return null;
  }
  return { exact_hit: parsed.exact_hit };
}

function extractMarkers(logText) {
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

module.exports = { extractMarkers };
