---
id: LRN-20261107-reuse-existing-evidence-for-compatibility-reports
status: promoted
cause_status: confirmed
scope: Hardware Compatibility Report generation and related inventory or sensor reporting changes
trigger: Adding a report field or changing how a report obtains hardware evidence
failure_signature: The first report implementation queried Storage Health records again when opening the preview
root_cause: Report generation treated a read-only query as free even though the regular dashboard already acquired the evidence
guardrail: Use already acquired inventory and observations and represent a missing observation explicitly
canonical_refs: CONTEXT.md, src-tauri/src/services/hardware_compatibility_service.rs
verification: Trace the report command to cached inputs and run the hardware_compatibility_service Rust tests and the hardware compatibility Playwright test
evidence: GitHub issue 2500 and the existing Hardware Report, monitor snapshot, and Storage Health command paths
revalidate_when: The report gains fields or changes its inventory, monitoring, or Storage Health evidence sources
---

The Hardware Compatibility Report uses the allowlisted inventory fields from
the existing Hardware Report input and availability observations from normal
monitoring. The first implementation reused the Storage Health query itself,
which still incurred an extra read on each preview. Reuse the result that the
normal command has already returned. A field without evidence stays marked
`Not recorded`.

Super I/O identification follows the model or family already published by the
regular sensor samples. This avoids adding chip-identification metadata to
every sensor-service frame for an occasional export.
