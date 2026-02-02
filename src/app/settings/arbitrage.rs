use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;

use crate::app::config;

pub static BASE_TOKENS: Lazy<Vec<config::BaseTokenConfig>> =
    Lazy::new(|| config::CONFIG.strategy.base_tokens.clone());

pub static NONCE_ADDR: Lazy<Pubkey> =
    Lazy::new(|| Pubkey::from_str_const(&config::CONFIG.strategy.nonce_account));

pub static TARGET_TOKEN: Lazy<String> = Lazy::new(|| {
    config::CONFIG
        .strategy
        .quote_mint
        .clone()
        .unwrap_or_else(|| "So11111111111111111111111111111111111111112".to_string())
});
