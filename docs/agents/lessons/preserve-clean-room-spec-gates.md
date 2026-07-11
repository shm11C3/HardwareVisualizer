---
id: LRN-20260711-preserve-clean-room-spec-gates
status: promoted
cause_status: confirmed
scope: PawnIO CPU sensors, Super I/O sensors, sensor specifications, implementation, and review
trigger: work reads or changes clean-room sensor specifications or native CPU and Super I/O sensor implementation
failure_signature: stale handoff status or transport readiness could be mistaken for implementation-ready chip decode evidence
root_cause: snapshot coordination documents and separate specification gates were collapsed into one readiness decision
guardrail: .agents/rules/clean-room-sensors.md and docs/specs/sensors/README.md
canonical_refs: docs/specs/sensors/README.md, .agents/rules/clean-room-sensors.md
verification: confirm exact Implementation-ready status at the pinned revision, no TODO(provenance), read-only scope, and required attestations
evidence: current specification status, pinned revision, TODO(provenance) scan, evidence files, and required PR attestations
revalidate_when: clean-room roles, prohibited sources, specification status rules, or PR template requirements change
---

# Preserve Clean-room Specification Gates

The current specification and its evidence outrank a handoff snapshot. A ready
transport contract does not make a chip-family decode specification ready.
Confirm every consulted spec is implementation-ready and free of unresolved
provenance markers before implementation or review.

If required information is absent, return the question to the spec-author role.
Do not fill the gap from another monitoring implementation.
