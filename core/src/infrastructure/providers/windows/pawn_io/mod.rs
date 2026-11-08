//! Every provider whose readings come through PawnIO.
//!
//! Only the Windows sensor service opens PawnIO through this subtree
//! (ADR 0029); app-side code may use its types and CPUID support checks.
//! It is implemented clean-room from `docs/specs/sensors/**`; see
//! `.agents/rules/clean-room-sensors.md`.

pub(crate) mod client;
pub mod cpu;
pub mod super_io;
