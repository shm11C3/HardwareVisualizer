//! Narrow read-only IPC; never exposes PawnIO handles or register operations.

pub mod protocol;
#[cfg(target_os = "windows")]
pub mod windows;
