use once_cell::sync::Lazy;

use crate::app::config;

pub static FEES: Lazy<config::TxCostConfig> = Lazy::new(|| config::CONFIG.tx_cost.clone());
