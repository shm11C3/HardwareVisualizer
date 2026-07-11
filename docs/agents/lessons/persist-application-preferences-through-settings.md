---
id: LRN-20260711-persist-application-preferences-through-settings
status: promoted
cause_status: confirmed
scope: frontend state, settings commands, Core settings, and App settings
trigger: a new value must survive restart or is exposed as a user-facing setting
failure_signature: AI guidance described Tauri Store as the default persisted UI store and could route application preferences around typed Rust settings ownership
root_cause: persistence mechanism was chosen from the frontend implementation convenience instead of the user-facing ownership contract
guardrail: docs/design-principles.md DP-06 and DP-08 plus scoped frontend and App instructions
canonical_refs: AGENTS.md, src/AGENTS.md, .agents/rules/settings.md, and docs/architecture/backend.md
verification: settings-service tests cover valid/invalid reads and unknown-key preservation; generated bindings match typed commands
evidence: CONTEXT.md Application Preference and UI-local State definitions, docs/architecture/backend.md settings ownership, and src/README.md
revalidate_when: settings storage, typed IPC generation, or Core/App settings ownership changes
---

# Persist Application Preferences Through Settings

User-facing application preferences belong in the shared `settings.json`
contract and are written through typed Rust commands and the appropriate
settings owner. Frontend Tauri Store is only for UI-local or transient values
that can be reset without losing an explicit configuration.

When adding a preference, decide whether Core or App consumes it, preserve keys
owned by the other side, and regenerate TypeScript bindings when the command
surface changes.
