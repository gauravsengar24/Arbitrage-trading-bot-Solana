//! Chain data: program maps, token info, and constants.

pub mod alt_constants;
pub mod keys;

pub use alt_constants::*;
pub use keys::*;

/// Base transaction fee in lamports (Solana network fee per signature).
pub const TRANSACTION_FEE: u64 = 5000;
