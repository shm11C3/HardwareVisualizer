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
//! - `docs/specs/sensors/superio-nuvoton-nct67xx.md` revision 9
//! - `docs/specs/sensors/superio-ite-it86xx-it87xx.md` revision 2
//!
//! No other external sensor source was used.

pub(super) mod access;
pub mod diagnostics;
pub(super) mod ite;
pub mod motherboard;
pub(super) mod nuvoton;

/// Inclusive plausibility bounds shared by every Super I/O byte temperature:
/// the ITE revision 2 policy and the Nuvoton revision 9 filter (S1 Table 8-1,
/// +125 down to -55 degrees C) use the same window.
const SUPER_IO_MIN_PLAUSIBLE_TEMPERATURE: f32 = -55.0;
const SUPER_IO_MAX_PLAUSIBLE_TEMPERATURE: f32 = 125.0;

/// Whether a decoded Super I/O byte temperature lies inside the shared
/// plausibility window (-55 to +125 degrees C inclusive). Zero is a valid
/// reading; an implausible value is omitted, never clamped or zeroed.
pub(super) const fn is_plausible_super_io_temperature(temperature: f32) -> bool {
  temperature >= SUPER_IO_MIN_PLAUSIBLE_TEMPERATURE
    && temperature <= SUPER_IO_MAX_PLAUSIBLE_TEMPERATURE
}
