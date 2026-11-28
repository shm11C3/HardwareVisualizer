//! AMD pre-Zen control temperature (Tctl) through the signed `AMDFamily10`
//! module.
//!
//! Implemented clean-room from:
//!
//! - `docs/specs/sensors/cpu-amd-family-15h-16h-tctl.md` rev 2 and the
//!   `AMDFamily10` contract in `docs/specs/sensors/pawnio-interface.md` rev 8
//!   (Family 15h/16h);
//! - `docs/specs/sensors/cpu-amd-family-10h-14h-tctl.md` rev 1 and
//!   `docs/specs/sensors/pawnio-interface.md` rev 9 (Family 10h, 11h, 12h and
//!   14h models 00h-0Fh).
//!
//! Only `ioctl_read_miscctl` (node 0, offsets `0x00` and `0xA4`) and, for
//! Family 15h models 60h-7Fh only, `ioctl_read_smu` (address `0xD8200CA4`)
//! are ever invoked. The module's TSC-measurement ioctl has upstream freeze
//! reports and its C-state ioctl writes an I/O port without a documented
//! mutex, so neither has a caller here; adding one needs a specification
//! revision.
//!
//! - `access`: the module operations, the `Global\Access_PCI` lock and the
//!   D18F3x00 identity gate.
//! - `reported_temperature`: the pure Reported Temperature Control decode.
//! - `family_15h_16h`: the Family 15h/16h model groups, PkgType gate and
//!   per-map sample.
//! - `family_10h_14h`: the Family 10h-14h model groups, the Family 10h
//!   erratum 319, PkgType and DDR3 gates, and the D18F3xA4 sample.

mod access;
mod family_10h_14h;
mod family_15h_16h;
mod reported_temperature;

pub(crate) use access::admit_identity;
pub use family_10h_14h::Family10h14hRejection;
pub(crate) use family_10h_14h::{
  Ddr3Gate, Family10h14hGroup, is_family_10h_14h, query_ddr3_gate,
  sample_family_10h_14h_tctl, select_family_10h_14h_group,
};
pub(crate) use family_15h_16h::{
  TctlGroup, is_family_15h_16h, sample_tctl, select_tctl_group,
};
