use std::collections::HashSet;

use borsh::BorshDeserialize;
use jupiter_swap_api_client::{quote::QuoteResponse, swap::SwapInstructionsResponse};
use solana_sdk::{
    instruction::AccountMeta,
    pubkey::Pubkey,
};

use crate::{RouteArgs, RoutePlanStep};

pub fn get_swap_infos(
    ix1: SwapInstructionsResponse,
    ix2: SwapInstructionsResponse,
    data1: QuoteResponse,
    data2: QuoteResponse,
) -> Result<(
    Vec<RoutePlanStep>,
    Vec<AccountMeta>,
    Vec<Pubkey>,
    Vec<Pubkey>,
), anyhow::Error> {
    let token_addresses: Vec<Pubkey> = data1
        .route_plan
        .iter()
        .chain(&data2.route_plan)
        .flat_map(|f| [f.swap_info.input_mint, f.swap_info.output_mint])
        .collect::<HashSet<_>>() // deduplicate
        .into_iter()
        .collect();

    println!("{:#?}", token_addresses);

    // Deserialize route args using Borsh deserialization
    // The instruction data starts with an 8-byte discriminator, followed by the RouteArgs
    let mut route_1 = RouteArgs::try_from_slice(&ix1.swap_instruction.data[8..])
        .map_err(|e| anyhow::anyhow!("Failed to deserialize route_1: {}", e))?;
    let mut route_2 = RouteArgs::try_from_slice(&ix2.swap_instruction.data[8..])
        .map_err(|e| anyhow::anyhow!("Failed to deserialize route_2: {}", e))?;

    // Combine routes
    let combined_route: Vec<_> = route_1
        .route_plan
        .drain(..)
        .chain(route_2.route_plan.drain(..))
        .collect();

    // Build refined plan
    let refined_plan: Vec<RoutePlanStep> = combined_route
        .iter()
        .enumerate()
        .map(|(idx, ele)| RoutePlanStep {
            swap: ele.swap.clone(),
            percent: 100, // adjust if needed
            input_index: idx as u8,
            output_index: if idx == combined_route.len() - 1 {
                0
            } else {
                (idx + 1) as u8
            },
        })
        .collect();

    // Merge remaining accounts
    let remaining_accounts: Vec<AccountMeta> = ix1.swap_instruction.accounts[9..]
        .iter()
        .chain(&ix2.swap_instruction.accounts[9..])
        .cloned()
        .collect();

    // Merge + deduplicate ALT addresses
    let combined_alt: Vec<Pubkey> = ix1
        .address_lookup_table_addresses
        .into_iter()
        .chain(ix2.address_lookup_table_addresses)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    Ok((
        refined_plan,
        remaining_accounts,
        token_addresses,
        combined_alt,
    ))
}
