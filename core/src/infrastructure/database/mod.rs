//! SQLite-backed persistence used by [`crate::persistence`].
//!
//! Core owns the database: callers initialize its location once at startup
//! via [`db::init`] (Core never resolves the path itself because that
//! depends on Tauri's bundle identifier), and Core applies the schema
//! migrations through [`migrate::run`] before any worker writes. App owns
//! the ordered migration definitions and hands them to the runner. Every
//! read / write that Core executes goes through this module.

pub mod archive_queries;
pub mod db;
pub mod gpu_archive;
pub mod hardware_archive;
pub mod migrate;
pub mod process_stats;
pub mod storage_health;
