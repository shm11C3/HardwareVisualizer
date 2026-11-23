---
scope: "hardviz/frontend/src/features/settings/**,hardviz/frontend/src/lib/tauriStore.ts,hardviz/frontend/src/hooks/tauri/useTauriStore.ts,hardviz/core/src/settings/**,hardviz/app/src/commands/settings.rs,hardviz/app/src/models/settings.rs,hardviz/app/src/services/settings_service.rs"
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

When a command/type changes, regenerate `hardviz/frontend/src/rspc/bindings.ts`; do not edit it
manually.
