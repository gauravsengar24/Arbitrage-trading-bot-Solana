//! Transaction cost calculation using SOL price and tx cost config.

use crate::app::config::TxCostConfig;
use crate::chain::TRANSACTION_FEE;

use super::sol_price::SOL_PRICE;

/// Base transaction fee in lamports = 0.000005 SOL.
/// Formula: (base_tx_fee + tip_sol) * sol_usd.
pub async fn calculate_tx_cost_usdc(fee: &TxCostConfig) -> f64 {
    let sol_price = {
        let guard = SOL_PRICE.lock().await;
        guard.unwrap_or(fee.sol_usd)
    };
    let base_tx_fee_sol = TRANSACTION_FEE as f64 / 1_000_000_000.0;
    let total_sol_cost = base_tx_fee_sol + fee.tip_sol;
    total_sol_cost * sol_price
}
