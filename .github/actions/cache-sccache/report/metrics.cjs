function count(counts, key) {
  return Number(counts?.[key] || 0);
}

function sumCounts(counts) {
  return Object.values(counts || {}).reduce(
    (sum, value) => sum + Number(value || 0),
    0,
  );
}

function hitRate(hits, misses) {
  const total = hits + misses;
  return total === 0 ? null : Number(((hits / total) * 100).toFixed(2));
}

function buildCacheReport(result, environment) {
  const stats = result.stats || {};
  const hits = stats.cache_hits?.counts || {};
  const misses = stats.cache_misses?.counts || {};
  const rustHits = count(hits, "Rust");
  const rustMisses = count(misses, "Rust");
  const cppHits = count(hits, "C/C++");
  const cppMisses = count(misses, "C/C++");
  const cacheErrors = sumCounts(stats.cache_errors?.counts);
  const readErrors = Number(stats.cache_read_errors || 0);
  const writeErrors = Number(stats.cache_write_errors || 0);
  const timeouts = Number(stats.cache_timeouts || 0);
  const totalErrors = cacheErrors + readErrors + writeErrors + timeouts;
  const rustRequests = rustHits + rustMisses;
  const cppRequests = cppHits + cppMisses;
  const cacheLocation = result.cache_location || "unknown";
  const targetCacheExactHit = environment.RUST_CACHE_EXACT_HIT || "unknown";
  const expectCpp = environment.SCCACHE_EXPECT_CPP === "true";

  const metrics = {
    job: environment.GITHUB_JOB || "unknown",
    runner_os: environment.RUNNER_OS || "unknown",
    runner_arch: environment.RUNNER_ARCH || "unknown",
    target_cache_exact_hit: targetCacheExactHit,
    cache_location: cacheLocation,
    compile_requests: Number(stats.compile_requests || 0),
    compile_requests_executed: Number(stats.requests_executed || 0),
    rust_hits: rustHits,
    rust_misses: rustMisses,
    rust_hit_rate_pct: hitRate(rustHits, rustMisses),
    cpp_hits: cppHits,
    cpp_misses: cppMisses,
    cpp_hit_rate_pct: hitRate(cppHits, cppMisses),
    cache_errors: totalErrors,
  };

  const warnings = [];
  const expectsR2 = Boolean(
    environment.SCCACHE_BUCKET &&
      environment.AWS_ACCESS_KEY_ID &&
      environment.AWS_SECRET_ACCESS_KEY,
  );
  if (expectsR2 && !cacheLocation.startsWith("s3,")) {
    warnings.push(
      `R2 credentials were provided but the active backend is ${cacheLocation}`,
    );
  }
  if (totalErrors > 0) {
    warnings.push(
      `sccache reported ${totalErrors} read, write, timeout, or backend errors`,
    );
  }
  if (
    expectCpp &&
    targetCacheExactHit !== "true" &&
    rustRequests >= 100 &&
    cppRequests === 0
  ) {
    warnings.push(
      `DuckDB job compiled ${rustRequests} Rust units after a non-exact target restore but recorded no C/C++ requests`,
    );
  }

  return { metrics, warnings };
}

module.exports = { buildCacheReport };
