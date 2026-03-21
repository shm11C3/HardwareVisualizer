# Rust coding instructions (HardwareVisualizer)

These instructions apply to all Rust code under `src-tauri/`.

## Macro imports — `log_internal` is required

The logging macros (`log_debug!`, `log_info!`, `log_warn!`, `log_error!`) are defined in `src-tauri/src/utils/logger.rs` and internally expand to `log_internal!`.

Because `log_internal!` is a `#[macro_export]` macro, any file that uses one of the convenience macros **must** import it:

```rust
use crate::{log_debug, log_internal};
```

`log_internal` will appear unused to the compiler and linters (including `clippy`), but removing it causes a compilation error. **Do not remove `log_internal` imports.**

## Backend architecture

Follow the one-way dependency chain: **Commands → Services → Platform (Factory) → Infrastructure / OS APIs**.

- `src-tauri/src/commands/` — Tauri command handlers (UI boundary)
- `src-tauri/src/services/` — Business logic
- `src-tauri/src/platform/` — OS-specific trait implementations
- `src-tauri/src/infrastructure/` — Providers (GPU APIs, DB, WMI, etc.)

See `docs/ARCHITECTURE/BACKEND_ARCHITECTURE.md` for details.

## Platform-conditional code

- Use `#[cfg(target_os = "windows")]`, `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]` for OS-specific code.
- Pure helper functions used only by platform-gated callers should use `#[cfg(any(target_os = "...", test))]` to keep them available for unit tests across all platforms.

## Testing

- Run Rust tests: `cargo tauri-test` (CI parity) or `cargo test --lib` (quick local check).
- Place tests in inline `#[cfg(test)] mod tests { ... }` at the bottom of each file.
- Prefer testing pure functions. Extract platform-dependent logic into pure helpers where practical.
