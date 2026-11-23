---
id: LRN-20261116-review-bundled-duckdb-upgrades
status: promoted
cause_status: confirmed
scope: DuckDB dependency updates
trigger: a change upgrades the duckdb crate or its bundled engine
failure_signature: Core and Tauri reject the bundled engine version and the license check reports stale coverage
root_cause: scheduled version updates changed the Cargo pins without updating application checks or bundled-library notices
guardrail: use the existing runtime, storage, and license checks to review Dependabot DuckDB PRs
canonical_refs: core/src/infrastructure/database/candidate_database/sink.rs, core/tests/duckdb/native.rs, scripts/licenses/check-duckdb-license-version.ts
verification: run the native Core and Tauri tests and the bundled-license check on the proposed dependency graph
evidence: "GitHub PRs #2490 and #2617 failed, and PR #2420 shows a coordinated upgrade"
revalidate_when: the bundled engine, storage compatibility, or notice checks change
---

# Review Bundled DuckDB Upgrades Together

[PR #2490](https://github.com/shm11C3/HardwareVisualizer/pull/2490) and
[PR #2617](https://github.com/shm11C3/HardwareVisualizer/pull/2617) both pulled
in DuckDB 2.0.0 through a crate minor bump. Native database creation still
expected DuckDB 1.5.6, and the license guard rejected the mismatch between
the lockfile and the manual notice.

Dependabot opens the update PR so new releases remain visible.
[PR #2420](https://github.com/shm11C3/HardwareVisualizer/pull/2420) shows the
changes to make together. Confirm the engine reported by the bundled build and
verify the pinned storage format with existing native archives. Review the
vendored sources before updating the manual and platform notices.
