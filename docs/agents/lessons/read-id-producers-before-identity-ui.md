---
id: LRN-20260819-read-id-producers-before-identity-ui
status: promoted
cause_status: confirmed
scope: frontend features that join, select, persist, or attribute entities keyed by backend-produced ids
trigger: a change adds a selector, attribution UI, cross-source join, persisted id, or a fixture for multi-source data
failure_signature: "PR #1944 needed eleven review rounds because the GPU adapter selector was built on an assumed shared id namespace; the inventory and the monitor stream key GPUs differently on every platform, and the e2e fixture used one id for both sources, so no test failed"
root_cause: the frontend id contract was inferred from fixtures and frontend types instead of read from the producing Rust code, and effective-entity resolution was duplicated per surface instead of owned in one place
guardrail: .agents/skills/verify-identity-contracts/SKILL.md
canonical_refs: .agents/skills/verify-identity-contracts/SKILL.md, docs/adr/0016-gpu-attribution-on-the-performance-screen.md
verification: before identity-touching UI work, the PR records a producer-sourced contract table and names the single resolution owner; src/e2e/fixtures/hardware.ts keeps distinct inventory and live ids
evidence: "PR #1944 review history; core/src/platform/{windows,macos,linux}/gpu.rs id producers vs core/src/infrastructure/providers GraphicInfo ids; src/e2e/fixtures/hardware.ts GPU_FIXTURES id/liveId split; src/features/hardware/gpuIdentity.ts"
revalidate_when: "backend id namespaces are unified (issue #1948 direction) or the monitor stream starts carrying inventory ids"
---

# Read Id Producers Before Identity UI

An identity-touching UI change is only as sound as its id contract, and the
contract lives in the producing backend code, not in fixtures or frontend
types. In PR #1944 thirty minutes of reading the Rust producers would have
surfaced up front what eleven review rounds surfaced incrementally: the
inventory and the live stream cannot be joined by id on any platform.

Two practices follow, both encoded in the
[`verify-identity-contracts`](../../../.agents/skills/verify-identity-contracts/SKILL.md)
skill:

- Record the producer-sourced contract table before writing UI code, and give
  effective-entity resolution exactly one owner that every surface consumes.
- Keep fixtures as split as production: a fixture sharing one id across two
  sources certifies joins that fail on real hardware.

Deterministic follow-ups (branded id types, a fixture-realism assertion, and a
consumer allowlist for the shared selection atom) are tracked in issue #1956.
