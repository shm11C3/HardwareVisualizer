//! Super I/O sensor access through PawnIO's `LpcIO` module.
//!
//! The pure, spec-encoded facts live next to the provider that uses them:
//! [`access`] holds the configuration-access facts shared by every vendor,
//! [`nuvoton`] and [`ite`] hold the vendor hardware-monitor decode facts, and
//! [`diagnostics`] / [`motherboard`] perform the actual port I/O. This mirrors
//! the `cpu_*_decode.rs` placement for the PawnIO CPU paths.
//!
//! Implemented from:
//! - `docs/specs/sensors/superio-access.md` revision 3 (revision 4 for the
//!   slot-1 ITE exit withhold)
//! - `docs/specs/sensors/superio-nuvoton-nct67xx.md` revision 7
//! - `docs/specs/sensors/superio-ite-it86xx-it87xx.md` revision 2
//!
//! No other external sensor source was used.

pub(super) mod access;
pub mod diagnostics;
pub(super) mod ite;
pub mod motherboard;
pub(super) mod nuvoton;
