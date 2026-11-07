//! Opt-in Product Analytics runtime (ADR 0028).
//!
//! `docs/design/product-analytics.md` is the specification. Core gains no
//! analytics concept: this App module classifies facts Core already
//! publishes, derives Installation Tokens, seals batches, and owns their
//! transport and lifecycle.

mod batch;
mod consent;
mod period;
mod tokens;
mod vocabulary;
