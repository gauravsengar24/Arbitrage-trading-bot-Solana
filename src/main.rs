use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Instant;

use chrono::Utc;
use futures::stream::{FuturesUnordered, StreamExt};
use jupiter_arbitrage_bot_offchain::*;
use jupiter_arbitrage_bot_offchain::{Tips, submit_with_services};
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction::advance_nonce_account;
use tokio::time::Duration;
use tracing::{debug, error, info, warn};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::{
    SubscribeRequest, SubscribeRequestFilterTransactions,
};

// Most liquid intermediate tokens. All are tried as the arb "hop" for every base token
// (excluding the base token itself). More intermediates = more paths checked per round.
const LIQUID_INTERMEDIATES: &[&str] = &[
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", // USDT
    "So11111111111111111111111111111111111111112",   // WSOL
    "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So", // mSOL
    "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn", // JitoSOL
    "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN", // JUP
    "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs", // ETH (Wormhole)
    "3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh", // WBTC
];

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    info!("Starting arbitrage bot");

    // Background: keep nonce fresh.
    tokio::spawn(async move {
        loop {
            fetch_nonce().await;
        }
    });

    // Background: keep SOL price fresh (5-min updates).
    tokio::spawn({
        let fallback_price = CONFIG.tx_cost.sol_usd;
        async move {
            start_sol_price_fetcher(fallback_price).await;
        }
    });

    let _hash = get_slot();

    let pubkey = keyfile_status().await;
    info!(pubkey = %pubkey.pubkey(), "Wallet loaded");

    info!(
        watch_flows       = CONFIG.strategy.watch_flows,
        poll_quotes       = CONFIG.strategy.poll_quotes,
        poll_interval_ms  = CONFIG.strategy.poll_interval_ms,
        live_trading      = CONFIG.strategy.live_trading,
        base_tokens       = BASE_TOKENS.len(),
        "Arbitrage configuration"
    );

    match estimate_jupiter_timing().await {
        Ok(t) => info!(
            quote_ms      = t.quote_ms,
            swap_build_ms = t.swap_build_ms,
            total_ms      = t.quote_ms + t.swap_build_ms,
            "Jupiter timing"
        ),
        Err(e) => warn!(error = %e, "Jupiter timing estimate skipped"),
    }

    if CONFIG.strategy.poll_quotes {
        let interval_ms = CONFIG.strategy.poll_interval_ms;
        tokio::spawn(async move {
            continuous_polling_loop(interval_ms).await;
        });
    }

    if CONFIG.strategy.watch_flows {
        run_big_trades_monitor().await?;
    } else if CONFIG.strategy.poll_quotes {
        info!("Big-trades monitor disabled; running continuous polling only");
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    } else {
        warn!("Both modes disabled — enable poll_quotes or watch_flows in config");
    }

    Ok(())
}

// =============================================================================
// CONTINUOUS POLLING MODE
// =============================================================================

/// Build the list of arb intermediates to try for a given mother token.
/// Excludes the mother token itself to avoid trivial no-op routes.
fn build_arb_targets(mother_mint: &str) -> Vec<String> {
    LIQUID_INTERMEDIATES
        .iter()
        .filter(|&&t| t != mother_mint)
        .map(|&t| t.to_string())
        .collect()
}

/// Poll all base tokens concurrently for one round. Returns true if any token
/// had a profitable opportunity this round.
async fn poll_round(circuit_open: bool) -> bool {
    let mut tasks: FuturesUnordered<_> = BASE_TOKENS
        .iter()
        .map(|cfg| poll_single_token(cfg, circuit_open))
        .collect();

    let mut found_any = false;
    while let Some(had_opportunity) = tasks.next().await {
        if had_opportunity {
            found_any = true;
        }
    }
    found_any
}

/// Main polling loop. Adapts its interval based on market activity:
/// - Halves the interval (min 100 ms) when an opportunity is found.
/// - Gradually restores the base interval (50 ms per quiet round) when none is found.
///
/// Stats are printed every 50 rounds.
async fn continuous_polling_loop(base_interval_ms: u64) {
    info!(base_interval_ms, targets_per_token = LIQUID_INTERMEDIATES.len() - 1,
          "Starting concurrent multi-token polling");

    let mut current_interval_ms = base_interval_ms;
    let mut poll_round_count: u64 = 0;

    loop {
        let circuit_open = STATS.is_circuit_open();

        let round_start = Instant::now();
        let found_any   = poll_round(circuit_open).await;
        poll_round_count += 1;

        // Adaptive interval
        if found_any {
            current_interval_ms = (current_interval_ms / 2).max(100);
        } else {
            current_interval_ms = current_interval_ms.saturating_add(50).min(base_interval_ms);
        }

        // Periodic stats log
        if poll_round_count % 50 == 0 {
            info!(
                round           = poll_round_count,
                interval_ms     = current_interval_ms,
                trades_attempted = STATS.trades_attempted.load(Ordering::Relaxed),
                trades_won      = STATS.trades_successful.load(Ordering::Relaxed),
                win_rate        = format!("{:.1}%", STATS.win_rate()),
                total_profit    = format!("{:.6} SOL", STATS.total_profit_sol()),
                circuit         = if circuit_open { "OPEN" } else { "closed" },
                "Polling stats"
            );
        }

        // Sleep only what's left of the interval.
        let elapsed_ms = round_start.elapsed().as_millis() as u64;
        if let Some(wait_ms) = current_interval_ms.checked_sub(elapsed_ms) {
            if wait_ms > 0 {
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
            }
        }
    }
}

/// Poll one base token for arbitrage. Returns true when a profitable trade was found
/// (regardless of whether it was submitted — circuit breaker may skip submission).
async fn poll_single_token(cfg: &app::config::BaseTokenConfig, circuit_open: bool) -> bool {
    let mother_token = cfg.mint.clone();

    let (decimal, symbol) = POPULAR_TOKEN_INFO
        .iter()
        .find(|t| t.mint == mother_token.as_str())
        .map(|t| (t.decimals, t.symbol))
        .unwrap_or_else(|| {
            if mother_token == "So11111111111111111111111111111111111111112" {
                (9, "SOL")
            } else {
                (6, "UNKNOWN")
            }
        });

    let target_tokens = build_arb_targets(&mother_token);
    let sim_start = Instant::now();

    let quote_data = simulate_amount_in(
        mother_token.clone(),
        decimal,
        symbol.to_string(),
        target_tokens,
        cfg.amount_range[0],
        cfg.amount_range[1],
        cfg.steps as usize,
        cfg.min_profit,
        true,
    )
    .await;

    let sim_ms = sim_start.elapsed().as_millis();
    if sim_ms > 300 {
        debug!(elapsed_ms = sim_ms, %symbol, "simulate_amount_in slow");
    }

    if quote_data.is_empty() {
        return false;
    }

    info!(count = quote_data.len(), %symbol, sim_ms, "Profitable opportunities found");

    if !CONFIG.strategy.live_trading || circuit_open {
        if circuit_open {
            warn!(%symbol, "Circuit breaker open — skipping submission");
        }
        return true;
    }

    // Pick the opportunity with the highest net profit.
    let best = quote_data
        .into_iter()
        .max_by_key(|(in_a, out_a, _, _, _, _)| *out_a as i64 - *in_a as i64);

    if let Some((in_amount, out_amount, in_res, out_res, _, _target_token)) = best {
        let token_is_sol = symbol == "SOL" || symbol == "WSOL"
            || mother_token == "So11111111111111111111111111111111111111112";

        let (total_tx_cost_raw, tip_sol) =
            engine::runtime::calculate_tx_cost_for_trade(
                &FEES,
                out_amount as i64 - in_amount as i64,
                token_is_sol,
                decimal,
            )
            .await;

        let net_profit_raw = out_amount as i64 - in_amount as i64 - total_tx_cost_raw;
        let min_profit_raw = (cfg.min_profit * 10_f64.powf(decimal as f64)) as i64;

        if net_profit_raw < min_profit_raw {
            return false;
        }

        let pow          = 10_f64.powf(decimal as f64);
        let in_human     = in_amount  as f64 / pow;
        let out_human    = out_amount as f64 / pow;
        let profit_human = net_profit_raw as f64 / pow;

        info!(
            %symbol,
            in_amount  = format!("{:.6}", in_human),
            out_amount = format!("{:.6}", out_human),
            net_profit = format!("{:.6}", profit_human),
            "Submitting trade"
        );

        // Convert net profit to lamports for the P&L tracker.
        let net_profit_lamports = if token_is_sol {
            net_profit_raw
        } else {
            let sol_price = engine::runtime::sol_price::get_sol_price_usdc(FEES.sol_usd).await;
            ((net_profit_raw as f64 / pow) / sol_price * 1_000_000_000.0) as i64
        };

        STATS.record_attempt();
        tokio::spawn(async move {
            let accepted = submit_polling_trade(in_res, out_res, cfg.min_profit, decimal, tip_sol).await;
            if accepted {
                STATS.record_success(net_profit_lamports);
            } else {
                STATS.record_failure();
            }
        });

        return true;
    }

    false
}

/// Build and submit the swap transaction from two Jupiter quote responses.
/// Returns true if the RPC accepted the transaction.
async fn submit_polling_trade(
    in_res:             jupiter_swap_api_client::quote::QuoteResponse,
    out_res:            jupiter_swap_api_client::quote::QuoteResponse,
    min_profit_amount:  f64,
    decimal:            u8,
    tip_sol:            f64,
) -> bool {
    let nonce_ix = advance_nonce_account(&NONCE_ADDR, &PUBKEY);

    let ix = match get_swap_ix(
        in_res,
        out_res,
        (min_profit_amount * 10_f64.powf(decimal as f64)) as u64,
    )
    .await
    {
        Ok(ix) => ix,
        Err(e) => {
            error!(error = %e, "Failed to build swap instruction");
            return false;
        }
    };

    let mut swap_ixs = Vec::new();
    swap_ixs.extend(ix.setup_instructions);
    swap_ixs.push(ix.swap_instruction);

    let recent_blockhash = get_nonce().blockhash();
    let alts             = fetch_alt(ix.address_lookup_table_addresses).await;

    info!(
        time    = %Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        "Submitting via RPC"
    );

    submit_with_services(
        Tips {
            tip_sol_amount:              tip_sol,
            tip_addr_idx:                0,
            cu:                          Some(FEES.compute_units),
            priority_fee_micro_lamport:  Some(FEES.priority_lamports),
            payer:                       *PUBKEY,
            pure_ix:                     swap_ixs,
        },
        &*SIGNERS,
        recent_blockhash,
        nonce_ix,
        alts,
        1,
    )
    .await
}

// =============================================================================
// BIG-TRADES MONITOR MODE  (Yellowstone gRPC)
// =============================================================================

async fn run_big_trades_monitor() -> Result<(), anyhow::Error> {
    let yellowstone_endpoint = YELLOWSTONE_GRPC_ENDPOINT
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("yellowstone_grpc_endpoint not configured"))?;
    let yellowstone_token = YELLOWSTONE_GRPC_TOKEN
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("yellowstone_grpc_token not configured"))?;

    let endpoint_url = yellowstone_endpoint
        .strip_prefix("http://")
        .or_else(|| yellowstone_endpoint.strip_prefix("https://"))
        .unwrap_or(yellowstone_endpoint);

    let (host, port) = if let Some((h, p)) = endpoint_url.split_once(':') {
        (h, p.parse::<u16>().unwrap_or(10001))
    } else {
        (endpoint_url, 10001)
    };

    info!(%host, %port, "Connecting to Yellowstone gRPC");

    loop {
        info!("Connecting to Yellowstone");

        let endpoint = format!("http://{}:{}", host, port);
        let builder  = match GeyserGrpcClient::build_from_shared(endpoint) {
            Ok(b)  => b,
            Err(e) => {
                error!(error = ?e, "Yellowstone builder error");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        let builder = match builder.x_token(Some(yellowstone_token.clone())) {
            Ok(b)  => b,
            Err(e) => {
                error!(error = ?e, "Yellowstone x-token error");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        let mut client = match builder.connect().await {
            Ok(c)  => { info!("Yellowstone connected"); c }
            Err(e) => {
                error!(error = ?e, "Yellowstone connection error");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        let mut tx_map = HashMap::new();
        for (idx, base_token) in BASE_TOKENS.iter().enumerate() {
            tx_map.insert(
                format!("tx_{}", idx),
                SubscribeRequestFilterTransactions {
                    vote:             Some(false),
                    failed:           Some(false),
                    account_include:  vec![base_token.mint.clone()],
                    account_exclude:  vec![],
                    account_required: vec![],
                    signature:        None,
                },
            );
        }

        let request = SubscribeRequest {
            slots:               HashMap::new(),
            accounts:            HashMap::new(),
            transactions:        tx_map,
            transactions_status: HashMap::new(),
            blocks:              HashMap::new(),
            blocks_meta:         HashMap::new(),
            accounts_data_slice: vec![],
            commitment:          Some(0),
            ping:                None,
            entry:               HashMap::new(),
            from_slot:           None,
        };

        let (_sink, mut stream) = match client.subscribe_with_request(Some(request)).await {
            Ok(p)  => { info!("Yellowstone subscribed"); p }
            Err(e) => {
                error!(error = ?e, "Yellowstone subscribe error");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        loop {
            match stream.next().await {
                Some(Ok(update)) => {
                    tokio::spawn(async move {
                        process_single_trade_yellowstone(update).await;
                    });
                }
                Some(Err(e)) => {
                    error!(error = ?e, "Yellowstone stream error");
                    break;
                }
                None => {
                    info!("Yellowstone stream ended");
                    break;
                }
            }
        }
    }
}
