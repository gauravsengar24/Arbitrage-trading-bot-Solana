//! Jupiter API client and settings. Transactions are submitted via RPC only.

use crate::app::config;
use jupiter_swap_api_client::JupiterSwapApiClient;
use once_cell::sync::Lazy;

pub static JUPITER_ENDPOINT: Lazy<String> =
    Lazy::new(|| config::CONFIG.swap_api.base_url.clone());

pub static JUPITER_API_KEY: Lazy<Option<String>> = Lazy::new(|| {
    if config::CONFIG.swap_api.api_key.is_empty() {
        None
    } else {
        Some(config::CONFIG.swap_api.api_key.clone())
    }
});

pub static JUPITER_CLIENT: Lazy<JupiterSwapApiClient> =
    Lazy::new(|| JupiterSwapApiClient::new(JUPITER_ENDPOINT.clone(), JUPITER_API_KEY.clone()));
