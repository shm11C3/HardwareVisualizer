---
id: LRN-20260830-keep-independent-cpu-power-sampler
status: promoted
cause_status: confirmed
scope: Windows PawnIO CPU temperature and RAPL power providers
trigger: adding a Windows CPU sensor that shares an existing PawnIO module or executor
failure_signature: design review identified that combining CPU temperature and RAPL power samplers would couple independent capability failures and obscure the one-IntelMSR-executor ownership rule
root_cause: the proposed combined architecture treated module-handle ownership and sensor sampling state as the same boundary even though Intel temperature and power need independent capability, probe, and baseline lifetimes; design review separated those boundaries before implementation
guardrail: docs/architecture/windows-sensor-external-components.md and the shared IntelMSR owner in core/src/infrastructure/providers/windows/pawn_io.rs
canonical_refs: docs/architecture/windows-sensor-external-components.md, core/src/infrastructure/providers/windows/pawn_io.rs, core/src/infrastructure/providers/windows/cpu_temperature.rs, core/src/infrastructure/providers/windows/cpu_power.rs
verification: confirm IntelMSR is opened only through the shared owner, temperature and power retain separate samplers, AMD temperature and power use separate module clients, and each provider can remain unavailable without suppressing the other
evidence: Sol implementation design review, authorized by the maintainer on 2026-08-30; the proposed combination was changed before landing; core/src/infrastructure/providers/windows/pawn_io.rs; core/src/infrastructure/providers/windows/cpu_temperature.rs; core/src/infrastructure/providers/windows/cpu_power.rs
revalidate_when: PawnIO module ownership, CPU sensor capability selection, or Windows sampler lifecycle changes
---

# Keep CPU Power Sampling Independent

Sharing a PawnIO executor does not require merging the sensors that use it.
Keep Intel temperature and RAPL power as independent providers with separate
capability gates, probe/setup failures, diagnostics, and sampling baselines;
share only the single `IntelMSR` client owner. AMD temperature and power use
their distinct module clients. This preserves partial results when one sensor
path is unavailable while preventing multiple IntelMSR executors.

This lesson comes from a design review of a proposed combined manager; the
coupling was identified and removed before the combined architecture landed.
