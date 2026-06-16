//! Quote simulation across a range of input amounts.
//!
//! Uses a semaphore to cap concurrent Jupiter API calls (avoids rate-limiting),
//! and streams results with FuturesUnordered so profitable quotes surface
//! as soon as they complete rather than after the slowest request finishes.

use std::sync::Arc;
use std::time::Instant;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

use futures::stream::{FuturesUnordered, StreamExt};
use jupiter_swap_api_client::quote::QuoteResponse;
use tokio::sync::Semaphore;

use crate::*;

/// Maximum concurrent Jupiter API requests per simulation call.
const MAX_CONCURRENT_QUOTES: usize = 8;

static LOG_MUTEX: Mutex<()> = Mutex::new(());

fn write_log(message: &str) {
    let _guard = LOG_MUTEX.lock().unwrap();
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("logs.txt") {
        let _ = writeln!(file, "{}", message);
        let _ = file.flush();
    }
}

pub async fn simulate_amount_in(
    mother_token: String,
    mother_token_decimal: u8,
    mother_token_symbol: String,
    target_tokens: Vec<String>,
    from_f: f64,
    to_f: f64,
    steps: usize,
    min_profit_amount: f64,
    is_polling_mode: bool,
) -> Vec<(u64, u64, QuoteResponse, QuoteResponse, u128, String)> {
    let sim_start = Instant::now();

    // Build a geometric grid of input amounts: [from_f, …, to_f]
    let steps = steps.max(2);
    let ratio = (to_f / from_f).powf(1.0 / (steps as f64 - 1.0));
    let pow = 10_f64.powf(mother_token_decimal as f64);
    let amounts: Vec<u64> = (0..steps)
        .map(|i| (from_f * ratio.powi(i as i32) * pow) as u64)
        .collect();

    let min_profit_raw = (min_profit_amount * pow) as u64;

    // Semaphore keeps us under the Jupiter API rate limit.
    let sem = Arc::new(Semaphore::new(MAX_CONCURRENT_QUOTES));

    // Launch all (amount × target) combinations as concurrent futures.
    let mut futures: FuturesUnordered<_> = amounts
        .iter()
        .flat_map(|&in_amount| {
            target_tokens.iter().map(move |output_token| {
                let mother_token   = mother_token.clone();
                let output_token   = output_token.clone();
                let sem            = Arc::clone(&sem);
                async move {
                    let _permit = sem.acquire().await.unwrap();
                    let start = Instant::now();
                    let result = if is_polling_mode {
                        get_quote_polling(in_amount, &mother_token, &output_token).await
                    } else {
                        get_quote_big_trade(in_amount, &mother_token, &output_token).await
                    };
                    let elapsed = start.elapsed().as_micros();
                    result.ok().map(|(ia, oa, q1, q2)| (ia, oa, q1, q2, elapsed, output_token))
                }
            })
        })
        .collect();

    let sol_price   = crate::engine::runtime::sol_price::get_sol_price_usdc(FEES.sol_usd).await;
    let token_is_sol = mother_token == "So11111111111111111111111111111111111111112";

    let mut profitable: Vec<(u64, u64, QuoteResponse, QuoteResponse, u128, String)> = Vec::new();
    let mut total = 0usize;
    let mut ok    = 0usize;

    // Process results as they stream in — don't wait for the slowest quote.
    while let Some(maybe) = futures.next().await {
        total += 1;
        let Some((in_amount, out_amount, q1, q2, elapsed, target_token)) = maybe else {
            continue;
        };
        ok += 1;

        let gross_profit = out_amount as i64 - in_amount as i64;
        let (total_tx_cost, _tip) = crate::engine::runtime::calculate_tx_cost_for_trade_with_sol_price(
            &FEES,
            gross_profit,
            token_is_sol,
            mother_token_decimal,
            sol_price,
        );
        let net_profit = gross_profit - total_tx_cost;

        if net_profit > 0 && net_profit as u64 > min_profit_raw {
            let target_symbol = POPULAR_TOKEN_INFO
                .iter()
                .find(|t| t.mint == target_token)
                .map(|t| t.symbol)
                .unwrap_or("?");

            write_log(&format!(
                "[SIMULATE] {} -> {} -> {}: in={:.6} out={:.6} net={:.6} (quote {}µs)",
                mother_token_symbol,
                target_symbol,
                mother_token_symbol,
                in_amount  as f64 / pow,
                out_amount as f64 / pow,
                net_profit as f64 / pow,
                elapsed,
            ));

            profitable.push((in_amount, out_amount, q1, q2, elapsed, target_token));
        }
    }

    let sim_ms = sim_start.elapsed().as_millis();
    write_log(&format!(
        "[SIMULATE] done in {}ms — {}/{} quotes ok, {} profitable",
        sim_ms, ok, total, profitable.len()
    ));

    profitable
}
