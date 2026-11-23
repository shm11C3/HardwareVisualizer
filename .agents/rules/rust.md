---
scope: "hardviz/core/**/*.rs,hardviz/app/**/*.rs,Cargo.toml,hardviz/core/Cargo.toml,hardviz/app/Cargo.toml"
---

# Rust coding instructions (HardwareVisualizer)

These instructions apply to Rust code in both crates of the workspace:
the Tauri-aware `hardviz/app/` and the Tauri-independent `hardviz-core` at
`hardviz/core/`.

## Logging macros

The logging macros (`log_debug!`, `log_info!`, `log_warn!`, `log_error!`,
`log_internal!`) live in `hardviz/core/src/utils/logger.rs` and are re-exported
from `hardviz/app/src/lib.rs` so existing `use crate::{log_warn, ...};`
sites in `hardviz/app` keep working unchanged.

The convenience macros expand to `$crate::log_internal!(...)`, so callers
that only use `log_debug!` / `log_info!` / `log_warn!` / `log_error!` no
longer need a parallel `log_internal` import.

```rust
// In either crate:
use crate::log_warn;

log_warn!("something happened", "my_function", None::<&str>);
```

If you call `log_internal!(...)` directly (rare — prefer the level-named
macros), import it alongside.

## Backend architecture (workspace split)

The backend is split across two crates:

- **`hardviz-core`** at `hardviz/core/` — Tauri-independent. No `tauri` dep is
  allowed (enforced at compile time by the Cargo dependency graph). Owns:
  the sensor collector and per-sensor history (`core::collector`), the
  in-process `EventBus` for `MetricsSnapshot` fan-out
  (`core::event_bus`), the platform abstraction (`core::platform`),
  OS-specific providers (`core::infrastructure::providers`), and POJO
  data types (`core::models`, `core::enums`).
- **`hardware_visualizer`** at `hardviz/app/` — Tauri-aware. Depends on
  `hardviz-core` via path. Owns: Tauri command handlers
  (`hardviz/app/src/commands/`), thin services (`hardviz/app/src/services/`)
  that call into Core, adapters that translate Core events into Tauri
  events (`hardviz/app/src/adapters/`), wire-format models with
  `specta::Type` derives (`hardviz/app/src/models/`), database-path resolution,
  startup compatibility wiring, and ordered SQL migration definitions. Core
  owns the pool, migration execution, Tauri-independent persistence workers,
  and database operations.

For hardware access, dependency direction is **Commands → Services →
`hardviz_core::platform` → `hardviz_core::infrastructure` / OS APIs**, with the
EventBus carrying real-time snapshots from Core to App-side adapters. App-owned
settings, lifecycle, and plugin services do not need to pass through the Core
platform abstraction.

Do not add a `tauri` dependency to `hardviz/core/Cargo.toml`, and do not write
`use tauri::*;` under `hardviz/core/src/`. `specta` and `tauri_specta` derives
stay in the `hardviz/app` crate. Core owns the platform-facing plain models; App
wire DTOs and their `From<core::...>` conversions are generated from the
allowlist or implemented at the boundary. Follow ADR 0009.

See `docs/architecture/backend.md` for the longer-form
architecture doc and `docs/design-principles.md` for the product/engineering
decision lens.

## Platform-conditional code

- Use `#[cfg(target_os = "windows")]`, `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]` for OS-specific code.
- Pure helper functions used only by platform-gated callers should use `#[cfg(any(target_os = "...", test))]` to keep them available for unit tests across all platforms.

## Testing

- Run Rust tests: `cargo tauri-test` (CI parity) or `cargo test --lib` (quick local check).
- Prefer inline `#[cfg(test)] mod tests { ... }` for pure module-local helpers.
  Shared App test setup may remain under `hardviz/app/src/_tests/`; integration
  tests should cover cross-module contracts rather than duplicating unit tests.
- Prefer testing pure functions. Extract platform-dependent logic into pure helpers where practical.
