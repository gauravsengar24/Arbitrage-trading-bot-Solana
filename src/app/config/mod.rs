use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fs;

pub mod arbitrage;
pub mod credential;
pub mod fee;

pub use arbitrage::*;
pub use credential::*;
pub use fee::*;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(rename = "connection", alias = "node", alias = "credential")]
    pub node: NodeConfig,
    #[serde(rename = "dex_api", alias = "swap_api", alias = "services")]
    pub swap_api: SwapApiConfig,
    #[serde(rename = "strategy", alias = "arbitrage")]
    pub strategy: StrategyConfig,
    #[serde(rename = "fees", alias = "tx_cost", alias = "fee")]
    pub tx_cost: TxCostConfig,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let path = "settings.toml";
    let content = fs::read_to_string(path).unwrap_or_else(|_| {
        fs::read_to_string("Config.toml").unwrap_or_else(|e| {
            panic!("Failed to read settings.toml or Config.toml: {}", e)
        })
    });
    toml::from_str(&content).expect("Failed to parse config")
});
