use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct StrategyConfig {
    #[serde(rename = "instruments", alias = "base_tokens", alias = "mother_token")]
    pub base_tokens: Vec<BaseTokenConfig>,
    #[serde(rename = "nonce_account_pubkey", alias = "nonce_account", alias = "nonce_addr")]
    pub nonce_account: String,
    #[serde(rename = "default_quote_mint", alias = "quote_mint", alias = "target_token", default)]
    pub quote_mint: Option<String>,
    #[serde(rename = "execution_enabled", alias = "live_trading", alias = "submit_transactions", default = "default_live_trading")]
    pub live_trading: bool,
    #[serde(rename = "geyser_watch_enabled", alias = "watch_flows", alias = "enable_big_trades_monitor", default = "default_watch_flows")]
    pub watch_flows: bool,
    #[serde(rename = "polling_enabled", alias = "poll_quotes", alias = "enable_continuous_polling", default = "default_poll_quotes")]
    pub poll_quotes: bool,
    #[serde(rename = "poll_interval_ms", alias = "polling_interval_ms", default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
}

fn default_live_trading() -> bool {
    true
}
fn default_watch_flows() -> bool {
    true
}
fn default_poll_quotes() -> bool {
    false
}
fn default_poll_interval_ms() -> u64 {
    500
}

#[derive(Debug, Deserialize, Clone)]
pub struct BaseTokenConfig {
    #[serde(rename = "token_mint", alias = "mint", alias = "token_addr")]
    pub mint: String,
    #[serde(rename = "min_delta_threshold", alias = "threshold")]
    pub threshold: f64,
    #[serde(rename = "min_profit_quote_units", alias = "min_profit", alias = "min_profit_amount")]
    pub min_profit: f64,
    #[serde(rename = "notional_range", alias = "amount_range", alias = "input_amount_range")]
    pub amount_range: [f64; 2],
    #[serde(rename = "grid_steps", alias = "steps", alias = "input_amount_steps")]
    pub steps: u64,
}
