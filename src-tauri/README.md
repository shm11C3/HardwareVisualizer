# hardware_visualizer (src-tauri)

Tauri-aware app crate for HardwareVisualizer.

This crate is the upper half of the Core / App split introduced by
[#1402][issue-1402]. It depends on [`hardviz-core`](../core) via a workspace
path dependency and is the only crate in the workspace that links against
`tauri`. Everything that requires an `AppHandle`, a window, or a Tauri plugin
lives here; everything else belongs in `hardviz-core`.

[issue-1402]: https://github.com/shm11C3/HardwareVisualizer/issues/1402

## Responsibilities

- Expose Tauri commands as the UI ↔ backend boundary; generate the matching
  TypeScript bindings via `tauri-specta`.
- Translate Core `MetricsSnapshot` events into Tauri events the frontend
  consumes (`HardwareMonitorUpdate`). This is the only place that calls
  `window.emit(...)` for sensor updates.
- Drive App startup: resolve OS-specific paths, hand the SQLite path to Core,
  surface the Core preflight result to the user (recovery dialog, restart),
  bring up Tauri plugins.
- Own UI-only settings (theme, language, line graph styling, burn-in shift,
  temperature unit, background image, …) that have no effect on Core.
- Own App-side services for background images, language, settings, system
  lifecycle, UI-local state, and App DTO conversion.
- Hold worker handles (`WorkersState`) so the App can terminate Core
  controllers and adapters cleanly on quit.

## Module layout

```text
src-tauri/src/
├── lib.rs                ← run() — Tauri builder, command registration, setup hook
├── main.rs               ← thin binary entry point
├── commands/             ← Tauri command handlers (UI boundary)
│   ├── background_image.rs
│   ├── hardware.rs
│   ├── settings.rs
│   ├── system.rs
│   ├── ui.rs
│   └── updater.rs
├── services/             ← App-side business logic
│   ├── background_image_service.rs
│   ├── gpu_service.rs / memory_service.rs / motherboard_service.rs / network_service.rs / …
│   ├── hardware_service.rs
│   ├── language_service.rs
│   ├── settings_service.rs
│   ├── system_service.rs
│   └── ui_service.rs
├── adapters/             ← Core EventBus → App outputs
│   ├── window.rs           subscribes to MetricsSnapshot, emits HardwareMonitorUpdate
│   └── tray.rs             subscribes to MetricsSnapshot, updates tray widget output
├── app/                  ← App lifecycle helpers
│   └── startup.rs          DB preflight error dialog + reset-and-restart flow
├── lifecycle.rs          ← close-to-tray, second instance, and run-event policy
├── workers/              ← WorkersState — holds Core controller / adapter handles
├── tray/                 ← tray widget windows, surface helpers, and UI policy
├── infrastructure/       ← App-only DB code
│   └── database/migration.rs   SQL migration definitions for tauri-plugin-sql
├── models/               ← App-side DTOs (HardwareMonitorUpdate, settings, SMART, …)
├── enums/                ← App-side enums (TemperatureUnit, hardware, settings, …)
├── utils/                ← App-side helpers (file paths, color, Tauri-aware logger)
└── _tests/               ← Unit and command-level tests
```

## Design rules

These rules are inherited from #1402 and apply across the App crate:

1. **`hardviz-core` is the source of truth for sensor state.** Commands read
   through Core APIs (`HistoryStore`, `MetricsSnapshot`) — never through a
   `Mutex` reached out of Core internals.
2. **All `MetricsSnapshot → window.emit(...)` translation lives under
   `adapters/`.** The collector loop in Core never touches `AppHandle`.
3. **App lifecycle is App-owned.** Window creation, plugin wiring, shutdown,
   and any process-restart logic stay under `src-tauri/`. Core lifecycle is
   driven by App-side controllers (`SystemMonitorController`,
   `ArchiveController`, `StorageSmartController`, `WindowAdapter`,
   `TrayAdapter`) held in `WorkersState`.
4. **UI-only settings stay App-side.** Theme, language, line graph styling,
   burn-in shift, `temperatureUnit`, and similar fields are owned here.
   Presentation conversions (e.g. °C → °F) happen in App, not in Core.
5. **No business logic in `commands/`.** Commands validate input, format
   output, and delegate to a service or to the Core API.

## Service categories

`src-tauri/src/services/` contains both thin Core wrappers and App-owned
services:

- `gpu_service`, `memory_service`, `motherboard_service`, and
  `network_service` wrap Core platform APIs and convert results into App DTOs or
  App error types.
- `hardware_service` aggregates Core history, Core platform data, and provider
  data into App-facing system information.
- `settings_service`, `background_image_service`, `language_service`,
  `system_service`, and `ui_service` own App-side behavior, Tauri-aware state,
  or UI-local persistence.

## Backend layering

Within the App crate, the layering follows the existing
[`docs/ARCHITECTURE/BACKEND_ARCHITECTURE.md`](../docs/ARCHITECTURE/BACKEND_ARCHITECTURE.md):

```text
Commands → Services → (Core API)         ─ App-owned
                    ↘ (Core: Platform → Infrastructure → OS APIs)
```

The `platform/` and `infrastructure/providers/` directories that the
architecture document references now live in [`core/src/`](../core); the
App crate retains only the App-specific `infrastructure/database/`
(migrations registered with `tauri-plugin-sql`).

## Frontend integration

- TypeScript bindings are generated by `tauri-specta` from the
  `collect_commands![ ... ]` macro in `lib.rs`. Output: `src/rspc/bindings.ts`
  (do not edit by hand — regenerate via `npm run tauri dev`).
- New backend commands must be added to `collect_commands![...]` in
  `src-tauri/src/lib.rs` to be exposed to the frontend.
- Backend errors are surfaced to the frontend via `error_event`; the
  frontend renders them through `useErrorModalListener`
  (`src/hooks/useTauriEventListener.ts`).

## Build & test

From the repository root:

```bash
# Desktop dev mode (regenerates TypeScript bindings)
npm run tauri dev

# Production build
npm run tauri build

# Cargo aliases (CI parity)
cargo tauri-fmt
cargo tauri-lint
cargo tauri-test
```

The `cargo tauri-*` aliases are defined in `.cargo/config.toml` at the
workspace root and operate on the full workspace, including `hardviz-core`.

## References

- Epic: [#1402 — split backend into Tauri-independent Core and thin App
  adapters][issue-1402]
- Backend architecture: [`docs/ARCHITECTURE/BACKEND_ARCHITECTURE.md`](../docs/ARCHITECTURE/BACKEND_ARCHITECTURE.md)
- Core crate: [`core/README.md`](../core/README.md)
- Top-level user-facing README: [`README.md`](../README.md)
