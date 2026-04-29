//! Tauri-independent core for HardwareVisualizer.
//!
//! Phase 1 establishes the workspace boundary only — module bodies are
//! populated in subsequent phases (see issue #1402). The Cargo dependency
//! graph enforces "no `tauri::*` under `core/src/`" at compile time.

pub mod collector {}

pub mod persistence {}

pub mod monitoring {}

pub mod event_bus {}

pub mod settings {}

pub mod platform {}

pub mod infrastructure {}

pub mod models {}
