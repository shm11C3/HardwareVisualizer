---
id: LRN-20260720-prefer-experimental-sensor-coverage
status: promoted
cause_status: confirmed
scope: native sensor enablement, Core sensor models, App sensor event projection, and clean-room review
trigger: a native sensor path recognizes hardware that is not yet fully verified, or a new family or model allowlist is proposed
failure_signature: unverified but safely readable hardware was treated as unsupported solely because it was absent from a verified-family allowlist
root_cause: verification confidence and runtime availability were collapsed into one binary enablement gate
guardrail: docs/design-principles.md and docs/adr/0011-experimental-sensor-enablement.md
canonical_refs: docs/design-principles.md, docs/adr/0011-experimental-sensor-enablement.md
verification: confirm the path is recognized, read-only, uses an existing decode and plausibility gate, retains Experimental metadata, and does not guess an address, register map, or chip selection
evidence: "issue #1824; core/src/models/metrics.rs; core/src/infrastructure/providers/windows/cpu_temperature.rs; core/src/platform/windows/sensors.rs"
revalidate_when: sensor verification vocabulary, clean-room role boundaries, PawnIO module recognition, or native sensor wire models change
---

# Prefer Experimental Sensor Coverage

Incomplete verification alone is not an unsupported-hardware signal. When the
current access path recognizes a device and can safely reuse an existing
read-only, plausibility-gated decode, enable the path as Experimental and retain
that confidence through diagnostics, Core samples, and presentation data.

This preference does not authorize guessing hardware facts. Unknown addresses,
register maps, chip selection, writes, or behavior still require an
implementation-ready specification before runtime enablement.
