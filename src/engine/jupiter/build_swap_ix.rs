use crate::{
    JUPITER_EVENT_AUTH, JUPITER_PROGRAM_ADDR, PUBKEY, ROUTE_DISCRIMINATOR, RouteArgs,
    TOKEN_PROGRAM_ID,
};
use borsh::to_vec;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use spl_associated_token_account::get_associated_token_address;

pub fn build_swap_ix(
    route_args: RouteArgs,
    remaining_accounts: Vec<AccountMeta>,
    mother_token: Pubkey,
) -> Instruction {
    let user_source_token_account = get_associated_token_address(&PUBKEY.clone(), &mother_token);

    let mut data: Vec<u8> = Vec::new();

    data.extend(ROUTE_DISCRIMINATOR);
    data.extend(to_vec(&route_args).unwrap());

    let mut accounts = vec![
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new(PUBKEY.clone(), true),
        AccountMeta::new(user_source_token_account, false),
        AccountMeta::new(user_source_token_account, false),
        AccountMeta::new_readonly(JUPITER_PROGRAM_ADDR, false),
        AccountMeta::new_readonly(mother_token, false),
        AccountMeta::new_readonly(JUPITER_PROGRAM_ADDR, false),
        AccountMeta::new_readonly(JUPITER_EVENT_AUTH, false),
        AccountMeta::new_readonly(JUPITER_PROGRAM_ADDR, false),
    ];

    accounts.extend(remaining_accounts);

    Instruction {
        program_id: JUPITER_PROGRAM_ADDR,
        accounts,
        data,
    }
}
