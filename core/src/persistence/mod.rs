//! Core-owned persistence primitives.
//!
//! - [`preflight`] runs the schema-version compatibility check before
//!   the App brings up the SQLite plugin. The recovery dialog and
//!   process restart flow live in App (`src-tauri/src/app/startup.rs`);
//!   this module owns only the read-only, Tauri-independent decision.
//! - [`archive`] hosts the hardware-archive worker, which subscribes to
//!   [`crate::event_bus::EventBus`] and writes summaries to SQLite via
//!   [`crate::infrastructure::database`]. It does not share the
//!   collector's history bag; that decoupling is the point of #1407.

pub mod archive;
pub mod archive_data;
pub mod preflight;

pub use archive::{
  ArchiveController, HARDWARE_ARCHIVE_INTERVAL_SECONDS, cleanup_old_data,
};
