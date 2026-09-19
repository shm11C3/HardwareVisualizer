const assert = require("node:assert/strict");
const {
  buildCacheReport,
} = require("../actions/cache-sccache/report/metrics.cjs");

const healthy = buildCacheReport(
  {
    cache_location: "s3, name: hardwarevisualizer-sccache, prefix: /",
    stats: {
      compile_requests: 340,
      requests_executed: 320,
      cache_hits: { counts: { Rust: 20, "C/C++": 294 } },
      cache_misses: { counts: { Rust: 3, "C/C++": 3 } },
      cache_errors: { counts: {} },
      cache_timeouts: 0,
      cache_read_errors: 0,
      cache_write_errors: 0,
    },
  },
  {
    GITHUB_JOB: "test-core",
    RUNNER_OS: "Windows",
    RUNNER_ARCH: "X64",
    RUST_CACHE_EXACT_HIT: "false",
    SCCACHE_EXPECT_CPP: "true",
    SCCACHE_BUCKET: "hardwarevisualizer-sccache",
    AWS_ACCESS_KEY_ID: "test",
    AWS_SECRET_ACCESS_KEY: "test",
  },
);

assert.equal(healthy.metrics.rust_hit_rate_pct, 86.96);
assert.equal(healthy.metrics.cpp_hits, 294);
assert.equal(healthy.metrics.cpp_hit_rate_pct, 98.99);
assert.equal(healthy.metrics.cache_errors, 0);
assert.deepEqual(healthy.warnings, []);

const missingCpp = buildCacheReport(
  {
    cache_location: "disk",
    stats: {
      compile_requests: 220,
      requests_executed: 205,
      cache_hits: { counts: { Rust: 180 } },
      cache_misses: { counts: { Rust: 25 } },
      cache_errors: { counts: { backend: 1 } },
      cache_timeouts: 1,
      cache_read_errors: 0,
      cache_write_errors: 2,
    },
  },
  {
    RUST_CACHE_EXACT_HIT: "false",
    SCCACHE_EXPECT_CPP: "true",
    SCCACHE_BUCKET: "hardwarevisualizer-sccache",
    AWS_ACCESS_KEY_ID: "test",
    AWS_SECRET_ACCESS_KEY: "test",
  },
);

assert.equal(missingCpp.metrics.cache_errors, 4);
assert.equal(missingCpp.metrics.cpp_hit_rate_pct, null);
assert.equal(missingCpp.warnings.length, 3);
assert.match(missingCpp.warnings[0], /active backend is disk/);
assert.match(missingCpp.warnings[1], /reported 4/);
assert.match(missingCpp.warnings[2], /no C\/C\+\+ requests/);

const exactTargetHit = buildCacheReport(
  {
    cache_location: "s3, name: hardwarevisualizer-sccache, prefix: /",
    stats: {
      cache_hits: { counts: { Rust: 1 } },
      cache_misses: { counts: {} },
      cache_errors: { counts: {} },
    },
  },
  {
    RUST_CACHE_EXACT_HIT: "true",
    SCCACHE_EXPECT_CPP: "true",
  },
);

assert.deepEqual(exactTargetHit.warnings, []);

console.log("cache metrics tests passed");
