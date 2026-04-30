//! Monitoring lifecycle state machine.
//!
//! [`MonitoringState`] tracks whether sensor collection is running,
//! paused, or stopped. Transitions are explicit; same-state transitions
//! and any transition out of `Stopped` are rejected with
//! [`InvalidTransition`]. App-side callers (`src-tauri/src/lifecycle.rs`)
//! own the state instance and combine it with worker termination, while
//! `Stopped` is terminal — once reached, the process is on its way out.
//!
//! Unit-tested without a Tauri runtime: this module has no I/O.

pub mod state;

pub use state::{InvalidTransition, MonitoringState};
