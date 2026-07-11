---
scope: "src/features/settings/**,src/lib/tauriStore.ts,src/hooks/useTauriStore.ts,core/src/settings/**,src-tauri/src/commands/settings.rs,src-tauri/src/models/settings.rs,src-tauri/src/services/settings_service.rs"
---

# Settings Instructions

Classify every persisted value before choosing a storage API:

- Application Preference: a user-facing choice expected to survive restart as
  app configuration. Persist it in `settings.json` through typed Rust IPC and
  the owning Core/App settings service.
- UI-local State: a resettable selection, cache, or view state that is not an
  explicit configuration. It may use Tauri Store.

Core and App share one top-level settings object. Each writer must merge and
preserve keys it does not own. Apply validation/normalization on every supported
read path, including recovery parsing, and add tests for invalid persisted
values and unknown-key preservation.

`showGpuUsageSource` is a legacy Tauri Store exception despite appearing on the
Settings screen. Do not use it as precedent and do not migrate it inside an
unrelated change; follow the candidate lesson in
`docs/agents/lessons/legacy-gpu-source-display-preference.md`.

When a command/type changes, regenerate `src/rspc/bindings.ts`; do not edit it
manually.
