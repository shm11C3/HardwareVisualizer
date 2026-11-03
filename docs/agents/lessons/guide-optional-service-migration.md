---
id: LRN-20261104-guide-optional-service-migration
status: promoted
cause_status: confirmed
scope: Windows sensor service migration and External Component Guidance
trigger: changing optional service setup or replacing an elevated in-process provider
failure_signature: users could miss required service setup guidance or be interrupted later by recurring automatic setup dialogs
root_cause: transport failures produced no guidance candidate, Performance filtered to unelevated permission failures, and live guidance polling had no startup-only display boundary
guardrail: preserve optional setup and startup intent; limit automatic sensor setup guidance to one startup opportunity and keep later checks in Settings
canonical_refs: docs/adr/0029-windows-sensor-service.md, core/src/platform/windows/sensors.rs, src/app/ExternalComponentGuidanceDialog.tsx, src/app/SensorServiceStartupNotice.tsx
verification: sensor adapter tests and e2e/startup-guidance.spec.ts cover failed attempts, bounded startup-only guidance, late failures, explicit migration, and user dismissals
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

Live failure detection does not authorize recurring modal setup guidance.
Restrict the automatic sensor dialog to one bounded startup opportunity. Later
service loss, screen changes or window restoration must not reopen it; Settings
owns user-requested status and repair. A startup dialog already being read is
not closed on a timer, while one still queued behind another flow expires.
