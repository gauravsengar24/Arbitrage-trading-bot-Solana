//! Jupiter arbitrage bot (offchain) library.
//!
//! Finds and executes profitable Jupiter swap opportunities. Submits transactions via RPC only.
//! Supports continuous quote polling and optional big-trades monitoring via Yellowstone gRPC.
//!
//! Configuration is loaded from `settings.toml` (or `Config.toml` as fallback) at runtime.
//! See PROJECT.md in the repo for layout and config reference.
//!
//! # Layout
//!
//! - **app** — Configuration and runtime settings (node, swap API, strategy, tx cost).
//! - **chain** — Chain data and constants (program maps, token info, transaction fee).
//! - **engine** — Arbitrage engine (Jupiter integration, discovery, execution, runtime utilities).

pub mod app;
pub mod chain;
pub mod engine;

// Re-export so binaries and tests using `jupiter_arbitrage_bot_offchain::*` work.
pub use app::*;
pub use chain::*;
pub use engine::*;
