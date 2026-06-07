//! SQLite-backed persistence used by [`crate::persistence`].
//!
//! The schema is owned by App-side migrations (Tauri's `tauri-plugin-sql`
//! enforces them at startup), but every read / write that Core executes
//! goes through this module. Callers initialize the database location
//! once at startup via [`db::init`]; Core never resolves the path itself
//! because path resolution depends on Tauri's bundle identifier.

pub mod archive_queries;
pub mod db;
pub mod gpu_archive;
pub mod hardware_archive;
pub mod process_stats;
pub mod storage_health;
