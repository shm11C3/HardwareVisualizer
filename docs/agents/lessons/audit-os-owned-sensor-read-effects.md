---
id: LRN-20261108-audit-os-owned-sensor-read-effects
status: promoted
cause_status: confirmed
scope: readiness review of OS-owned hardware telemetry
trigger: a sensor proposal treats an OS input file as the read-only boundary
failure_signature: a fan input read refreshes other registers whose alarm status clears when read
root_cause: the review checked the file operation and the exposed value without tracing the driver's complete refresh path for the exact chip
guardrail: docs/specs/sensors/README.md
canonical_refs: docs/specs/sensors/README.md
verification: confirm the default driver read path and exact hardware semantics of every touched status group before enabling the profile
evidence: issues 2518 and 2519 and the IT8728F/EX Linux fan evidence
revalidate_when: the admitted driver, chip revision or refresh path changes
---

# Audit OS-Owned Sensor Read Effects

A Linux hwmon input can refresh a shared device cache before returning a
single value. On IT8728F/EX, the pinned default it87 refresh reads status
registers that the vendor identifies as read-to-clear. The proposed fan
profile remains disabled. Verify each touched status register for the
exact controller and driver during specification review.
