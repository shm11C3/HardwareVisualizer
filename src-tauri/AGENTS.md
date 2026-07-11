# Tauri App Instructions

These instructions add to the repository root `AGENTS.md` for work under
`src-tauri/`.

## Ownership

- The App crate owns Tauri commands, App services, adapters, lifecycle, plugins,
  windows, wire DTOs, UI-owned settings, ordered migration definitions, and
  worker handles.
- Commands validate/normalize input and delegate. Do not put sensor collection,
  OS access, long-running workers, or schema policy in command handlers.
- Call Core APIs for hardware facts. Core snapshots become Tauri events only in
  App adapters.
- Presentation conversion, including temperature units and wire serialization,
  stays at the App boundary.
- Process shutdown and window close are different lifecycle events when Close
  to Tray is enabled. Final cleanup belongs to process/App shutdown.

Read [`src-tauri/README.md`](README.md),
[`docs/architecture/backend.md`](../docs/architecture/backend.md), and
[`docs/design-principles.md`](../docs/design-principles.md). Also read
[`rust.md`](../.agents/rules/rust.md) and, for persisted settings,
[`settings.md`](../.agents/rules/settings.md).

## Commands, DTOs, And Settings

- Register frontend-callable commands in `collect_commands![...]`.
- Do not manually edit `src/rspc/bindings.ts`. Regenerate through
  `npm run tauri:dev` after Rust command/type changes.
- App wire DTOs may mirror Core through the allowlisted generator, but
  presentation/event-specific DTOs remain App-owned. Follow ADR 0009.
- Persist Application Preferences through the owning Rust settings service and
  typed commands. Preserve Core-owned keys when writing App settings.
- Keep ordered migration definitions in App and execute them through Core's
  migrator. Core owns the pool and Tauri-independent DB work. Add a new
  migration; do not rewrite an applied migration.

## Validation

Use focused command/service tests where possible, then the workspace checks:

```bash
cargo fmt --all -- --check
cargo clippy -p hardware_visualizer --all-targets -- -D warnings
cargo test -p hardware_visualizer --lib
```

If the IPC surface changed, regenerate bindings and verify that the frontend
uses the generated command/result shape.
