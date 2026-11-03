---
id: LRN-20261104-guide-optional-service-migration
status: promoted
cause_status: confirmed
scope: Windows sensor service migration and External Component Guidance
trigger: changing optional service setup or replacing an elevated in-process provider
failure_signature: existing elevated users and fresh installs could miss required service setup guidance
root_cause: transport failures produced no guidance candidate and Performance filtered candidates to unelevated permission failures
guardrail: preserve optional installation and startup intent while guiding failed visible attempts to setup and service-ready elevated users to the explicit startup switch
canonical_refs: docs/adr/0029-windows-sensor-service.md, core/src/platform/windows/sensors.rs, src/app/SensorServiceStartupNotice.tsx
verification: service_guidance_waits_for_a_failed_attempt_and_respects_acpi_fallback and e2e/startup-guidance.spec.ts cover missing-service setup, explicit migration, and user dismissals
evidence: maintainer correction on PR 2509 and the transport, candidate filtering, and rendered setup flow
revalidate_when: sensor service consent, provider fallback, guidance suppression, or startup preference ownership changes
---

# Guide Optional Service Migration

Replacing direct PawnIO collection with an optional service does not justify
silently installing that service or disabling an existing elevated-startup
preference. The missing piece is an actionable route for both installation
cohorts: guide failed visible sensor attempts to setup, then make the explicit
normal-startup switch discoverable after service readiness is observed.

Initial connection wait is not a failed setup. A successful ACPI fallback
suppresses CPU setup guidance, and unsupported hardware keeps its own meaning.
Clear repaired sensor candidates without clearing a user's session deferral.
