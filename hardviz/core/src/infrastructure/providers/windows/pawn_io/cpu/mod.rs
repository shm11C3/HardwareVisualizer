//! PawnIO-backed CPU temperature and power providers.

mod amd_family_15h_16h_tctl;
pub(crate) mod ccd_temperature;
pub(crate) mod core_temperature;
mod identity;
pub mod power;
mod power_decode;
pub mod temperature;
mod temperature_decode;
