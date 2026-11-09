---
id: LRN-20261108-audit-os-owned-sensor-read-effects
status: promoted
cause_status: confirmed
scope: readiness review of OS-owned hardware telemetry
trigger: a sensor proposal treats an OS input file or a driver binding as the full safety boundary
failure_signature: a fan input clears alarms through a shared refresh or a sensor read can race a surviving boot handler
root_cause: the review relied on a visible OS endpoint without tracing all refresh effects or the lifetime of an independent port owner
guardrail: docs/specs/sensors/README.md
canonical_refs: docs/specs/sensors/README.md
verification: trace the complete default read path and verify the lifetime of independent handlers wherever exclusive access is required
evidence: issues 2518 and 2519, IT8728F/EX Linux fan evidence, and the LattePanda Sigma boot EC arbitration in issue 2517
revalidate_when: the admitted driver, chip revision, refresh path or ownership evidence changes
---

# Audit OS-Owned Sensor Read Effects

A Linux hwmon input can refresh a shared device cache before returning a
single value. On IT8728F/EX, the pinned default it87 refresh reads status
registers that the vendor identifies as read-to-clear. The proposed fan
profile remains disabled. Verify each touched status register for the
exact controller and driver during specification review.

On LattePanda Sigma, Linux v7.2 can keep an early ACPI EC handler alive
when its platform driver releases the port reservations at unbind.
It can also install the handler before the platform device exists. A
native hwmon binding and empty competing-driver directory therefore leave
port arbitration unresolved. Recheck the lifetime of a competing handler
when the sensor scope depends on exclusive ownership. See the
[LattePanda Sigma draft](../../specs/sensors/superio-ite-lattepanda-sigma-linux-ec.md#unresolved-boot-ec-arbitration)
for the pinned evidence.
