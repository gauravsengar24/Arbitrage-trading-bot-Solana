//! Estimate Jupiter API latency: quote (2 calls) and build swap tx (swap_instructions).

use std::time::Instant;

use crate::*;

/// Estimated timings for one quote round-trip and one swap build (ms).
#[derive(Debug, Clone)]
pub struct JupiterTimingEstimate {
    /// Time for one full quote (mother -> target -> mother), i.e. 2 Jupiter quote API calls, in ms.
    pub quote_ms: u64,
    /// Time for Jupiter swap_instructions (build swap tx) in ms.
    pub swap_build_ms: u64,
    /// Total for quote + swap_build (sequential).
    pub total_ms: u64,
}

/// Run one quote (2 Jupiter API calls) and one swap_instructions call using config's first
/// base token and a single target, then return estimated timings in ms.
///
/// Uses: first base token from config, one target (USDC if base is SOL, else quote_mint),
/// amount = geometric midpoint of amount_range, and min_profit for swap build.
pub async fn estimate_jupiter_timing() -> Result<JupiterTimingEstimate, anyhow::Error> {
    let base_config = BASE_TOKENS
        .first()
        .ok_or_else(|| anyhow::anyhow!("No base token in config"))?;
    let mother_token = base_config.mint.as_str();

    let (decimal, symbol) = POPULAR_TOKEN_INFO
        .iter()
        .find(|t| t.mint == mother_token)
        .map(|t| (t.decimals, t.symbol))
        .unwrap_or_else(|| {
            if mother_token == "So11111111111111111111111111111111111111112" {
                (9, "SOL")
            } else {
                (6, "UNKNOWN")
            }
        });

    let target_token = if symbol == "SOL" || symbol == "WSOL" || mother_token == "So11111111111111111111111111111111111111112" {
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" // USDC
    } else {
        TARGET_TOKEN.as_str()
    };

    let [min_f, max_f] = base_config.amount_range;
    let mid_f = (min_f * max_f).sqrt();
    let amount = (mid_f * 10_f64.powf(decimal as f64)) as u64;

    // Time: one full quote (2 Jupiter quote API calls)
    let quote_start = Instant::now();
    let (_, _, in_res, out_res) = get_quote_polling(amount, mother_token, target_token).await?;
    let quote_ms = quote_start.elapsed().as_millis() as u64;

    // Time: build swap instructions (1 Jupiter swap_instructions API call)
    let min_profit_raw = (base_config.min_profit * 10_f64.powf(decimal as f64)) as u64;
    let swap_start = Instant::now();
    let _ = get_swap_ix(in_res, out_res, min_profit_raw).await?;
    let swap_build_ms = swap_start.elapsed().as_millis() as u64;

    Ok(JupiterTimingEstimate {
        quote_ms,
        swap_build_ms,
        total_ms: quote_ms + swap_build_ms,
    })
}
