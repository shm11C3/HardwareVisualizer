//! Core-owned persistence primitives.
//!
//! Currently exposes [`preflight`] — the schema-version compatibility
//! check that runs before the App brings up the SQLite plugin. The
//! recovery dialog and process restart flow live in App
//! (`src-tauri/src/app/startup.rs`); this module only owns the read-only,
//! Tauri-independent compatibility decision.

pub mod preflight;
