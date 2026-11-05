---
id: LRN-20261104-guide-optional-service-migration
status: promoted
cause_status: confirmed
scope: Windows sensor service migration and External Component Guidance
trigger: changing optional service setup or replacing an elevated in-process provider
failure_signature: users could miss required service setup guidance or be interrupted later by recurring automatic setup dialogs
root_cause: the selected opt-in and modal workflows were treated as fixed requirements; local cleanup retained unnecessary steps, timers, queuing, and a separate diagnostic/status query
guardrail: apply DP-12 to challenge workflow assumptions before simplifying implementation; preserve explicit user intent and concrete platform or security constraints
canonical_refs: docs/design-principles.md, AGENTS.md, .agents/skills/hardwarevisualizer-design-review/SKILL.md, docs/adr/0029-windows-sensor-service.md, docs/architecture/backend.md
verification: monitor-event and browser tests prove the nonmodal simplification; agent-guidance validation checks canonical references, while specification trade-offs require design review
evidence: maintainer corrections on PR 2509 requesting specification-level simplification and questioning service opt-in; 0dd341dd replaces startup timers and dedicated queries with the normal monitor stream
revalidate_when: sensor installation policy, provider fallback, guidance suppression, startup preference ownership, or the design-review workflow changes
---

# Guide Optional Service Migration

ADR 0029 now includes the app-owned service in protected MSI installation and
updates. PawnIO remains an optional external driver, and existing elevated
startup preferences remain explicit. Release notes and the existing startup
setting explain the privilege change. Successful upgrades need no migration
CTA; failed visible collections still have actionable repair guidance.

The maintainer also questioned whether this separate service opt-in was needed.
Do not elevate the current installation policy into a universal consent rule:
an app-owned service and an external driver have different ownership. Evaluate
whether normal application installation can include the service and remove a
later CTA/setup step, while preserving the external driver's consent and the
user's saved startup choice. The revised ADR makes that distinction, and the
existing startup toggle remains the single owner of that preference.

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
