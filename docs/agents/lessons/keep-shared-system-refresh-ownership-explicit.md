---
id: LRN-20260815-keep-shared-system-refresh-ownership-explicit
status: promoted
cause_status: confirmed
scope: core/src/collector, core/src/infrastructure/providers/sysinfo_provider.rs, and src-tauri/src/services/hardware_service.rs
trigger: narrowing sysinfo refreshes or adding a consumer of HistoryStore::system
failure_signature: replacing refresh_all with usage-only refreshes leaves the visible CPU clock at its previously cached value on platforms that update frequency dynamically
root_cause: HistoryStore shares one sysinfo System between the periodic collector and command-backed CPU hardware information, so refresh ownership crosses those call paths
guardrail: keep periodic refreshes limited to sampled data and refresh CPU frequency on demand in the Core provider that reads it
canonical_refs: core/README.md, core/src/infrastructure/providers/sysinfo_provider.rs
verification: cargo test -p hardviz-core; cargo clippy -p hardviz-core --all-targets -- -D warnings; inspect the CPU information command on platforms with dynamic frequency reporting
evidence: "core/src/collector/history.rs, core/src/collector/sampling.rs, core/src/infrastructure/providers/sysinfo_provider.rs, src-tauri/src/services/hardware_service.rs, and GitHub Issue #1927"
revalidate_when: sysinfo refresh semantics change, HistoryStore stops sharing System, or CPU hardware information moves to another provider
---

# Keep Shared System Refresh Ownership Explicit

`HistoryStore::system()` is not private state of the periodic collector. The
App's hardware information service passes the same `sysinfo::System` to the
Core CPU provider, which reads cached CPU frequency in addition to identity
facts.

When narrowing the collector's refresh set, audit every consumer of the shared
system and assign refresh cost to the path that uses each fact. CPU frequency
belongs to the command-backed CPU information path, while usage, memory, and
process data belong to the periodic sampler. Cross-platform tests can verify
that required collections remain available, but dynamic frequency freshness
still requires platform runtime evidence because CI runner hardware may expose
a constant or unavailable value.
