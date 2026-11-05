---
id: LRN-20261104-guide-optional-service-migration
status: promoted
cause_status: confirmed
scope: Windows sensor service migration and External Component Guidance
trigger: changing optional service setup or replacing an elevated in-process provider
failure_signature: users could miss required service setup guidance or be interrupted later by recurring automatic setup dialogs
root_cause: missing diagnostics hid setup needs; a separate automatic guidance query and modal policy then required startup timers, queuing, and a second service-status poll
guardrail: preserve optional setup and startup intent; carry collection diagnostics on the existing monitor stream and use one nonmodal sensor notice with explicit actions in Settings
canonical_refs: docs/adr/0029-windows-sensor-service.md, docs/architecture/backend.md, core/src/platform/windows/sensors.rs, src/app/SensorServiceNotice.tsx
verification: sensor adapter tests and e2e/startup-guidance.spec.ts cover failed attempts, nonmodal late failures and recovery, explicit migration, and user dismissals
evidence: maintainer correction on PR 2509 and the transport, candidate filtering, and rendered setup flow
revalidate_when: sensor service consent, provider fallback, guidance suppression, or startup preference ownership changes
---

# Guide Optional Service Migration

Replacing direct PawnIO collection with an optional service does not justify
silently installing that service or disabling an existing elevated-startup
preference. The missing piece is an actionable route for both installation
cohorts: guide failed visible sensor attempts to setup, then make the explicit
normal-startup preference discoverable without claiming service readiness.
Settings verifies readiness before offering the explicit switch.

Initial connection wait is not a failed setup. A successful ACPI fallback
suppresses CPU setup guidance, and unsupported hardware keeps its own meaning.
Clear repaired sensor candidates without clearing a user's session deferral.

Live failure detection does not require a modal interruption. The maintainer's
request to simplify the specification exposed that a bounded startup dialog
still needed timers, queue arbitration and dedicated reads. Replace that policy
with a nonmodal notice driven by the normal hardware event, and keep explicit
status and repair in Settings. Closing the notice silences it for this launch;
existing per-key permanent suppression remains intact.

Do not simplify only the flags while retaining unnecessary requirements.
Removing the separate diagnostic query also removes backend knowledge of
screen names and the extra service-readiness poll. Share identical sensor
setup copy across CPU/motherboard and failure kinds. The stable Core-produced
guidance keys remain unchanged across the event and suppression boundaries.
