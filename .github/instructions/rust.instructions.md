# Rust coding instructions (HardwareVisualizer)

These instructions apply to Rust code in both crates of the workspace:
the Tauri-aware `src-tauri/` and the Tauri-independent `hardviz-core` at
`core/`.

## Logging macros

The logging macros (`log_debug!`, `log_info!`, `log_warn!`, `log_error!`,
`log_internal!`) live in `core/src/utils/logger.rs` and are re-exported
from `src-tauri/src/lib.rs` so existing `use crate::{log_warn, ...};`
sites in `src-tauri` keep working unchanged.

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

- **`hardviz-core`** at `core/` — Tauri-independent. No `tauri` dep is
  allowed (enforced at compile time by the Cargo dependency graph). Owns:
  the sensor collector and per-sensor history (`core::collector`), the
  in-process `EventBus` for `MetricsSnapshot` fan-out
  (`core::event_bus`), the platform abstraction (`core::platform`),
  OS-specific providers (`core::infrastructure::providers`), and POJO
  data types (`core::models`, `core::enums`).
- **`hardware_visualizer`** at `src-tauri/` — Tauri-aware. Depends on
  `hardviz-core` via path. Owns: Tauri command handlers
  (`src-tauri/src/commands/`), thin services (`src-tauri/src/services/`)
  that call into Core, adapters that translate Core events into Tauri
  events (`src-tauri/src/adapters/`), wire-format models with
  `specta::Type` derives (`src-tauri/src/models/`), and persistence /
  database access (`src-tauri/src/infrastructure/database/` — moves to
  Core in a future phase).

Dependency direction: **Commands → Services → `hardviz_core::platform` →
`hardviz_core::infrastructure` / OS APIs**, with the EventBus carrying
real-time snapshots from Core to App-side adapters.

Do not add a `tauri` dependency to `core/Cargo.toml`, and do not write
`use tauri::*;` under `core/src/`. `specta` and `tauri_specta` derives
stay in the `src-tauri` crate; Core uses POJO mirrors and `From`
conversions handle the boundary.

See `docs/ARCHITECTURE/BACKEND_ARCHITECTURE.md` for the longer-form
architecture doc.

## Platform-conditional code

- Use `#[cfg(target_os = "windows")]`, `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]` for OS-specific code.
- Pure helper functions used only by platform-gated callers should use `#[cfg(any(target_os = "...", test))]` to keep them available for unit tests across all platforms.

## Testing

- Run Rust tests: `cargo tauri-test` (CI parity) or `cargo test --lib` (quick local check).
- Place tests in inline `#[cfg(test)] mod tests { ... }` at the bottom of each file.
- Prefer testing pure functions. Extract platform-dependent logic into pure helpers where practical.
