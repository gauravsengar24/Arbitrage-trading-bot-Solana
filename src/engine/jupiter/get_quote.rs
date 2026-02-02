use jupiter_swap_api_client::quote::{QuoteRequest, QuoteResponse};
use solana_sdk::pubkey::Pubkey;

use crate::JUPITER_CLIENT;

/// Get quote for big trade mode (with only_direct_routes and exclude_dexes)
pub async fn get_quote_big_trade(
    input_amount: u64,
    mother_token: &str,
    arb_token: &str,
) -> Result<(u64, u64, QuoteResponse, QuoteResponse), anyhow::Error> {
    let quote_request_1 = QuoteRequest {
        amount: input_amount,
        input_mint: Pubkey::from_str_const(mother_token),
        output_mint: Pubkey::from_str_const(arb_token),
        only_direct_routes: Some(true),
        restrict_intermediate_tokens : Some(true),
        slippage_bps: 0,
        ..QuoteRequest::default()
    };

    let quote_response_1 = JUPITER_CLIENT.quote(&quote_request_1).await?;

    let dexes_used_in_req_1 = quote_response_1
        .route_plan
        .iter()
        .map(|f| f.swap_info.label.replace(" ", " "))
        .collect::<Vec<_>>()
        .join(",");

    let quote_request_2 = QuoteRequest {
        amount: quote_response_1.out_amount,
        input_mint: Pubkey::from_str_const(arb_token),
        output_mint: Pubkey::from_str_const(mother_token),
        exclude_dexes: Some(dexes_used_in_req_1.clone().into()),
        only_direct_routes: Some(true),
        restrict_intermediate_tokens : Some(true),
        slippage_bps: 0,
        ..QuoteRequest::default()
    };

    let quote_response_2 = JUPITER_CLIENT.quote(&quote_request_2).await?;

    Ok((
        input_amount,
        quote_response_2.out_amount,
        quote_response_1,
        quote_response_2,
    ))
}

/// Get quote for polling mode
/// Based on Jupiter API documentation: https://dev.jup.ag/api-reference/swap/quote
/// - No only_direct_routes: Allows multi-hop routes for better opportunities (default: false)
/// - No exclude_dexes: Allows all DEXes to be considered
/// - Uses default restrict_intermediate_tokens (true): Reduces exposure to high slippage routes
/// - slippage_bps: 0 for precise arbitrage calculations
pub async fn get_quote_polling(
    input_amount: u64,
    mother_token: &str,
    arb_token: &str,
) -> Result<(u64, u64, QuoteResponse, QuoteResponse), anyhow::Error> {
    // Request 1: mother_token -> arb_token
    // No only_direct_routes: allows multi-hop routes (default: false)
    // No restrict_intermediate_tokens override: uses default (true) for stability
    // slippage_bps: 0 for exact quotes needed for arbitrage calculations
    let quote_request_1 = QuoteRequest {
        amount: input_amount,
        input_mint: Pubkey::from_str_const(mother_token),
        output_mint: Pubkey::from_str_const(arb_token),
        slippage_bps: 0,
        ..QuoteRequest::default()
    };

    let quote_response_1 = JUPITER_CLIENT.quote(&quote_request_1).await?;

    // Request 2: arb_token -> mother_token
    // No exclude_dexes: allows all DEXes (no restrictions)
    // No only_direct_routes: allows multi-hop routes (default: false)
    // No restrict_intermediate_tokens override: uses default (true) for stability
    // slippage_bps: 0 for exact quotes needed for arbitrage calculations
    let quote_request_2 = QuoteRequest {
        amount: quote_response_1.out_amount,
        input_mint: Pubkey::from_str_const(arb_token),
        output_mint: Pubkey::from_str_const(mother_token),
        slippage_bps: 0,
        ..QuoteRequest::default()
    };

    let quote_response_2 = JUPITER_CLIENT.quote(&quote_request_2).await?;

    Ok((
        input_amount,
        quote_response_2.out_amount,
        quote_response_1,
        quote_response_2,
    ))
}

// Keep the old function for backward compatibility, but it uses big_trade mode
pub async fn get_quote(
    input_amount: u64,
    mother_token: &str,
    arb_token: &str,
) -> Result<(u64, u64, QuoteResponse, QuoteResponse), anyhow::Error> {
    get_quote_big_trade(input_amount, mother_token, arb_token).await
}
