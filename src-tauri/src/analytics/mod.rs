//! Opt-in Product Analytics runtime (ADR 0028).
//!
//! `docs/design/product-analytics.md` is the specification. Core gains no
//! analytics concept: this App module classifies facts Core already
//! publishes, derives Installation Tokens, seals batches, and owns their
//! transport and lifecycle.

mod batch;
mod consent;
mod flags;
mod period;
mod runtime;
mod service;
mod tokens;
mod transport;
mod vocabulary;

pub(crate) use runtime::{AppConsentStore, AppFlagSources, spawn_flush_loop};
pub(crate) use service::{AnalyticsService, Ports, SystemClock};
pub(crate) use transport::{LoggingTransport, build_transport};
pub use vocabulary::ProductEvent;

/// Whether this OS family has the v1 optional sensor providers, whose
/// outcomes feed `provider_available.*`. Only Windows does.
pub(crate) const OS_FAMILY_HAS_OPTIONAL_PROVIDERS: bool =
  matches!(vocabulary::OsFamily::CURRENT, vocabulary::OsFamily::Windows);
