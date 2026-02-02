use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct TxCostConfig {
    #[serde(rename = "compute_unit_limit", alias = "compute_units", alias = "cu")]
    pub compute_units: u64,
    #[serde(rename = "priority_fee_lamports", alias = "priority_lamports", alias = "priority_fee_micro_lamport")]
    pub priority_lamports: u64,
    #[serde(rename = "relay_tip_sol", alias = "tip_sol", alias = "third_party_fee")]
    pub tip_sol: f64,
    #[serde(rename = "sol_price_usd", alias = "sol_usd", alias = "sol_price_usdc", default = "default_sol_usd")]
    pub sol_usd: f64,
}

fn default_sol_usd() -> f64 {
    150.0
}
