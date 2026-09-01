//! SwitchBot Meter as an ambient environment source (#2044).
//!
//! The first concrete [`EnvironmentalSensorProvider`] behind #2043's
//! abstraction. A SwitchBot meter continuously broadcasts its
//! temperature and humidity to anyone in radio range, so reading one
//! needs no account, no cloud API, no internet, and no pairing - the app
//! listens, decodes, and caches. Nothing leaves the machine.
//!
//! The module is split so that only the part that genuinely needs a
//! radio is platform-gated:
//!
//! - [`advertisement`] turns bytes into a reading. Pure, and tested
//!   from fixed byte strings on every platform.
//! - [`provider`] caches the newest reading and answers #2043's polling
//!   contract. Pure, and tested on every platform.
//! - [`scan`] drives a Windows BLE scan into the provider. Windows only,
//!   and the only part that cannot be tested without hardware.
//!
//! Windows is the verification platform, so it is the only one wired up.
//! Nothing above `scan` is Windows-specific, so another platform's
//! transport can be added later without touching the decode or the
//! cache.
//!
//! [`EnvironmentalSensorProvider`]: super::environmental::EnvironmentalSensorProvider

pub mod advertisement;
pub mod provider;

#[cfg(target_os = "windows")]
pub mod scan;

pub use advertisement::{
  MeterAdvertisement, SwitchBotMeterFrame, SwitchBotMeterModel, decode_service_data,
};
pub use provider::{SWITCHBOT_METER_SOURCE_LABEL, SwitchBotMeterProvider, source_label};

#[cfg(target_os = "windows")]
pub use scan::{OnBound, SwitchBotScanController};
