//! Tauri-aware database glue.
//!
//! App owns the ordered schema migration definitions ([`migration`]).
//! They are executed at startup by Core's migrator
//! ([`hardviz_core::infrastructure::database::migrate`]) against Core's
//! pool, before any persistence worker writes. Read/write workloads
//! (archive writers, scheduled deletion, schema preflight) moved to
//! [`hardviz_core::infrastructure::database`] and
//! [`hardviz_core::persistence`] in #1407.

pub mod migration;
