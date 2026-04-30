# hwviz-core

Tauri-independent core crate for HardwareVisualizer.

`hwviz-core` (library name `hwviz_core`) owns sensor collection, persistence,
the in-process event bus, and Core-consumed settings. It is the lower half of
the Core / App split introduced by [#1402][issue-1402]: everything that does
not need a Tauri context lives here, and the Tauri app crate
([`src-tauri/`](../src-tauri)) depends on it via a path dependency.

[issue-1402]: https://github.com/shm11C3/HardwareVisualizer/issues/1402

## Responsibilities

- Sample CPU / memory / GPU / process metrics on a periodic loop.
- Hold sensor history (ring buffers) behind a Core-owned read API.
- Publish each tick as a `MetricsSnapshot` on a single in-process broadcast bus
  that any number of subscribers (window, future tray / overlay / alerts) can
  fan out from.
- Run a SQLite schema-version preflight check before the App brings up the
  Tauri SQL plugin.
- Hold the subset of on-disk settings whose values change Core behavior.
- Expose the platform abstraction (Windows / Linux / macOS) and the
  infrastructure-level providers (sysinfo, NVAPI, WMI, procfs, …) used by it.

> **Planned (not yet in Core).** The hardware archive writer
> (`src-tauri/src/services/archive_service.rs` +
> `src-tauri/src/workers/hardware_archive.rs`) is still App-side today.
> Phase 4 of #1402 will move it into Core as an `EventBus` subscriber so
> that a slow DB write never stalls sensor polling.

## Design rules

These rules are enforced by structure, not by review:

1. **No `tauri` dependency.** `core/Cargo.toml` does not list `tauri`, so any
   `use tauri::*;` under `core/src/` simply fails to compile. The dependency
   graph is the boundary.
2. **No `window.emit(...)` in Core.** Core publishes `MetricsSnapshot` to
   `EventBus`. Translating a snapshot into a Tauri event is an App-side
   concern (`src-tauri/src/adapters/window.rs`).
3. **Persistence does not share state with the collector.** Once the archive
   writer moves into Core (Phase 4 of #1402), it will subscribe to `EventBus`
   rather than reaching into the collector's history bag, so the two run as
   independent tasks. Until then, the App-side worker reads through the
   `HistoryStore` public API only.
4. **Settings split by consumer.** `CoreSettings` deserializes only the keys
   that affect Core (currently `hardwareArchive`). UI-only keys (`theme`,
   `language`, `lineGraph*`, `temperatureUnit`, …) stay App-side. Both crates
   read and write the same JSON file; `CoreSettings::save_to_path` merges into
   the existing object so App-owned keys survive.
5. **Presentation lives in App.** Core stores temperatures in °C; conversion
   to °F is done in the App-side adapter, not in the collector.

## Module layout

```text
core/src/
├── lib.rs
├── event_bus.rs           ← tokio broadcast<MetricsSnapshot> fan-out
├── collector/             ← sampling loop + history ring buffers
│   ├── history.rs           HistoryStore (Arc<Mutex<...>> behind a read API)
│   ├── sampling.rs          sample_system / sample_gpu cycle
│   └── system_monitor.rs    SystemMonitorController (drives the tokio task)
├── persistence/           ← Core-owned storage primitives
│   └── preflight.rs         DB schema-version compatibility check
├── settings/              ← Core-consumed settings (subset of settings.json)
│   ├── mod.rs               CoreSettings (load / save with App-key merge)
│   └── hardware_archive.rs  HardwareArchiveSettings
├── platform/              ← Cross-platform hardware access
│   ├── traits.rs            MemoryPlatform / GpuPlatform / NetworkPlatform
│   ├── factory.rs           PlatformFactory (compile-time OS selection)
│   ├── windows/  linux/  macos/
├── infrastructure/        ← External I/O backing the platform layer
│   └── providers/           sysinfo / NVAPI / WMI / procfs / DRM / …
├── models/                ← Shared data types (MetricsSnapshot, GpuMetric, …)
├── enums/                 ← Cross-cutting enums (errors, hardware, settings)
└── utils/                 ← Logger macros, formatters, IP / rounding helpers
```

Phase 4 of #1402 will introduce `persistence/archive.rs` (the SQLite writer)
and `infrastructure/database/` (pool + writers); both are currently App-side.
The `monitoring` module reserved for the `Running` / `Paused` / `Stopped`
state machine (Phase 5) is also not yet present.

## Build & test

`hwviz-core` is a workspace member. From the repository root:

```bash
# Build only the core crate
cargo build -p hwviz-core

# Run Core tests without spinning up a Tauri runtime
cargo test -p hwviz-core
```

Core is also covered by the workspace-wide `cargo tauri-fmt` /
`cargo tauri-lint` / `cargo tauri-test` aliases defined in
`.cargo/config.toml` at the repository root.

## Relationship to the App crate

```text
hwviz-core (this crate)
    │  ├─ publishes MetricsSnapshot on EventBus
    │  ├─ exposes HistoryStore read API
    │  └─ exposes SystemMonitorController
    ▼
src-tauri/ (App)
    ├─ adapters::window   ─ subscribes to EventBus, emits HardwareMonitorUpdate
    ├─ commands::*        ─ thin delegation to Core API + Tauri input/output
    ├─ app::startup       ─ wires Core setup + DB preflight error dialog flow
    └─ workers::*         ─ owns the controller handles for graceful shutdown
```

## References

- Epic: [#1402 — split backend into Tauri-independent Core and thin App
  adapters][issue-1402]
- Architecture: [`docs/ARCHITECTURE/BACKEND_ARCHITECTURE.md`](../docs/ARCHITECTURE/BACKEND_ARCHITECTURE.md)
- App crate: [`src-tauri/README.md`](../src-tauri/README.md)
