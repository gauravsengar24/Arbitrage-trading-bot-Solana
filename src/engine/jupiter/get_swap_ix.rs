use jupiter_swap_api_client::{
    quote::{QuoteResponse, SwapMode},
    swap::{SwapInstructionsResponse, SwapRequest, SwapResponse},
    transaction_config::{ComputeUnitPriceMicroLamports, TransactionConfig},
};

use crate::*;

fn rpc_transaction_config() -> TransactionConfig {
    TransactionConfig {
        use_shared_accounts: Some(false),
        wrap_and_unwrap_sol: true,
        compute_unit_price_micro_lamports: Some(ComputeUnitPriceMicroLamports::MicroLamports(
            FEES.priority_lamports,
        )),
        skip_user_accounts_rpc_calls: true,
        ..Default::default()
    }
}

pub async fn get_swap_tx(
    quote_response_1: QuoteResponse,
    quote_response_2: QuoteResponse,
    min_profit_amount: u64,
) -> Result<SwapResponse, anyhow::Error> {
    let mut combined_route_plan = Vec::new();
    combined_route_plan.extend(quote_response_1.clone().route_plan);
    combined_route_plan.extend(quote_response_2.clone().route_plan);

    let combined_request = SwapRequest {
        quote_response: QuoteResponse {
            input_mint: quote_response_1.input_mint,
            in_amount: quote_response_1.in_amount,
            output_mint: quote_response_2.output_mint,
            out_amount: quote_response_1.in_amount + min_profit_amount,
            other_amount_threshold: quote_response_2.other_amount_threshold,
            swap_mode: SwapMode::ExactIn,
            slippage_bps: quote_response_2.slippage_bps,
            computed_auto_slippage: quote_response_2.computed_auto_slippage,
            uses_quote_minimizing_slippage: quote_response_2.uses_quote_minimizing_slippage,
            platform_fee: None,
            price_impact_pct: quote_response_2.price_impact_pct,
            route_plan: combined_route_plan,
            context_slot: quote_response_2.context_slot,
            time_taken: quote_response_2.time_taken,
        },
        config: rpc_transaction_config(),
        user_public_key: PUBKEY.clone(),
    };

    let swap_tx = JUPITER_CLIENT.swap(&combined_request, None).await.unwrap();

    Ok(swap_tx)
}

pub async fn get_swap_ix(
    quote_response_1: QuoteResponse,
    quote_response_2: QuoteResponse,
    min_profit_amount: u64,
) -> Result<SwapInstructionsResponse, anyhow::Error> {
    let mut combined_route_plan = Vec::new();
    combined_route_plan.extend(quote_response_1.clone().route_plan);
    combined_route_plan.extend(quote_response_2.clone().route_plan);

    let combined_request = SwapRequest {
        quote_response: QuoteResponse {
            input_mint: quote_response_1.input_mint,
            in_amount: quote_response_1.in_amount,
            output_mint: quote_response_2.output_mint,
            out_amount: quote_response_1.in_amount + min_profit_amount,
            other_amount_threshold: quote_response_2.other_amount_threshold,
            swap_mode: SwapMode::ExactIn,
            slippage_bps: quote_response_2.slippage_bps, // <- keep slippage!
            computed_auto_slippage: quote_response_2.computed_auto_slippage,
            uses_quote_minimizing_slippage: quote_response_2.uses_quote_minimizing_slippage,
            platform_fee: None,
            price_impact_pct: quote_response_2.price_impact_pct,
            route_plan: combined_route_plan,
            context_slot: quote_response_2.context_slot,
            time_taken: quote_response_2.time_taken,
        },
        config: TransactionConfig {
            use_shared_accounts: Some(false),
            wrap_and_unwrap_sol: true,
            skip_user_accounts_rpc_calls: true,
            ..Default::default()
        },
        user_public_key: PUBKEY.clone(),
    };

    let swap_ix = JUPITER_CLIENT
        .swap_instructions(&combined_request)
        .await
        .unwrap();

    Ok(swap_ix)
}
