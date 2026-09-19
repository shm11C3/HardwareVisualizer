const assert = require("node:assert/strict");
const {
  buildCacheInventory,
  renderCacheInventorySummary,
} = require("./cache-inventory.cjs");

const inventory = buildCacheInventory(
  [
    {
      key: "small",
      ref: "refs/heads/develop",
      size_in_bytes: 1024,
      created_at: "2026-09-19T00:00:00Z",
      last_accessed_at: "2026-09-19T01:00:00Z",
    },
    {
      key: "large",
      ref: "refs/heads/develop",
      size_in_bytes: 2 * 1024 * 1024 * 1024,
      created_at: "2026-09-19T00:00:00Z",
      last_accessed_at: "2026-09-19T02:00:00Z",
    },
  ],
  "2026-09-19T03:00:00Z",
);

assert.equal(inventory.total_count, 2);
assert.equal(inventory.total_size_in_bytes, 2 * 1024 * 1024 * 1024 + 1024);
assert.equal(inventory.entries[0].key, "large");
assert.match(
  renderCacheInventorySummary(inventory),
  /Stored: \*\*2\.00 GiB\*\*/,
);

console.log("cache inventory tests passed");
