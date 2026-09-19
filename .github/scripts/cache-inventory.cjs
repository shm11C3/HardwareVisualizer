function gib(bytes) {
  return bytes / 1024 / 1024 / 1024;
}

function buildCacheInventory(caches, generatedAt) {
  const entries = caches
    .map((cache) => ({
      key: cache.key,
      ref: cache.ref,
      size_in_bytes: Number(cache.size_in_bytes || 0),
      size_gib: gib(Number(cache.size_in_bytes || 0)),
      created_at: cache.created_at,
      last_accessed_at: cache.last_accessed_at,
    }))
    .sort((left, right) => right.size_in_bytes - left.size_in_bytes);
  const totalBytes = entries.reduce(
    (sum, entry) => sum + entry.size_in_bytes,
    0,
  );

  return {
    generated_at: generatedAt,
    total_count: entries.length,
    total_size_in_bytes: totalBytes,
    total_size_gib: gib(totalBytes),
    entries,
  };
}

function renderCacheInventorySummary(inventory) {
  const largest = inventory.entries.slice(0, 15);
  return [
    "## GitHub Actions cache inventory",
    "",
    `- Entries: **${inventory.total_count}**`,
    `- Stored: **${inventory.total_size_gib.toFixed(2)} GiB**`,
    `- Captured: \`${inventory.generated_at}\``,
    "",
    "| Key | Size | Last accessed |",
    "| --- | ---: | --- |",
    ...largest.map(
      (entry) =>
        `| \`${entry.key.replaceAll("|", "\\|")}\` | ${entry.size_gib.toFixed(2)} GiB | ${entry.last_accessed_at} |`,
    ),
    "",
  ].join("\n");
}

module.exports = { buildCacheInventory, renderCacheInventorySummary };
