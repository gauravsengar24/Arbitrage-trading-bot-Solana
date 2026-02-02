use yellowstone_grpc_proto::prelude::SubscribeUpdate;
use yellowstone_grpc_proto::geyser::subscribe_update;
use solana_sdk::{bs58, pubkey::Pubkey};
use std::collections::{HashMap, HashSet};

use crate::*;

#[derive(Debug, Clone)]
pub struct TokenChange {
    pub mint: String,
    pub owner: String,
    pub delta: f64,
    pub pre_balance: f64,
    pub post_balance: f64,
}

pub fn extract_big_trade(
    update: &SubscribeUpdate,
) -> Option<(
    (String, u8, [f64; 2], u64, f64, String),
    Vec<TokenChange>,
    Vec<String>,
    Vec<String>,
    String,
)> {
    // Extract transaction update from Yellowstone SubscribeUpdate
    let tx_update = match &update.update_oneof {
        Some(subscribe_update::UpdateOneof::Transaction(tx)) => tx,
        _ => {
            // Not a transaction update - this is expected for account subscriptions
            return None;
        },
    };

    let tx_info = tx_update.transaction.as_ref()?;
    let transaction = tx_info.transaction.as_ref()?;
    let meta = tx_info.meta.as_ref()?;
    let tx_msg = transaction.message.as_ref()?;

    // --- Collect account keys
    let mut account_keys: Vec<Pubkey> = tx_msg
        .account_keys
        .iter()
        .filter_map(|k| Pubkey::try_from(k.as_slice()).ok())
        .collect();

    account_keys.extend(
        meta.loaded_writable_addresses
            .iter()
            .filter_map(|raw| Pubkey::try_from(raw.as_slice()).ok()),
    );
    account_keys.extend(
        meta.loaded_readonly_addresses
            .iter()
            .filter_map(|raw| Pubkey::try_from(raw.as_slice()).ok()),
    );

    // --- Collect program IDs used
    let mut program_keys: HashSet<String> = HashSet::new();
    for ix in &tx_msg.instructions {
        if let Some(pk) = account_keys.get(ix.program_id_index as usize) {
            program_keys.insert(pk.to_string());
        }
    }
    for inner in &meta.inner_instructions {
        for ix in &inner.instructions {
            if let Some(pk) = account_keys.get(ix.program_id_index as usize) {
                program_keys.insert(pk.to_string());
            }
        }
    }

    // --- Lookup program names
    let program_names: Vec<String> = program_keys
        .iter()
        .filter_map(|pk| PROGRAM_MAP.get(pk).map(|name| format!("{} ({})", name, pk)))
        .collect();

    let tx_id = bs58::encode(&tx_info.signature).into_string();

    // --- Build pre/post maps
    let pre_map: HashMap<u32, (&str, &str, f64)> = meta
        .pre_token_balances
        .iter()
        .map(|tb| {
            (
                tb.account_index,
                (
                    tb.owner.as_str(),
                    tb.mint.as_str(),
                    tb.ui_token_amount
                        .as_ref()
                        .map(|u| u.ui_amount)
                        .unwrap_or(0.0),
                ),
            )
        })
        .collect();

    let post_map: HashMap<u32, (&str, &str, f64)> = meta
        .post_token_balances
        .iter()
        .map(|tb| {
            (
                tb.account_index,
                (
                    tb.owner.as_str(),
                    tb.mint.as_str(),
                    tb.ui_token_amount
                        .as_ref()
                        .map(|u| u.ui_amount)
                        .unwrap_or(0.0),
                ),
            )
        })
        .collect();

    // --- Union of all account indexes
    let mut all_indexes: Vec<u32> = pre_map.keys().chain(post_map.keys()).cloned().collect();
    all_indexes.sort_unstable();
    all_indexes.dedup();

    // --- Collect changes
    let changes: Vec<TokenChange> = all_indexes
        .into_iter()
        .filter_map(|i| {
            let (pre_owner, pre_mint, pre_amt) = pre_map.get(&i).cloned().unwrap_or(("", "", 0.0));
            let (post_owner, post_mint, post_amt) =
                post_map.get(&i).cloned().unwrap_or(("", "", 0.0));

            let delta = post_amt - pre_amt;
            if delta.abs() < f64::EPSILON {
                return None;
            }

            let owner = if !pre_owner.is_empty() {
                pre_owner.to_string()
            } else if !post_owner.is_empty() {
                post_owner.to_string()
            } else {
                account_keys
                    .get(i as usize)
                    .map(|p| p.to_string())
                    .unwrap_or_default()
            };

            let mint = if !pre_mint.is_empty() {
                pre_mint.to_string()
            } else if !post_mint.is_empty() {
                post_mint.to_string()
            } else {
                "-".to_string()
            };

            Some(TokenChange {
                mint,
                owner,
                delta,
                pre_balance: pre_amt,
                post_balance: post_amt,
            })
        })
        .collect();

    // --- Collect unique tokens excluding all mother tokens
    let mother_addrs: HashSet<_> = BASE_TOKENS.iter().map(|f| f.mint.clone()).collect();

    let mut owner_changes: Vec<TokenChange> = changes
        .iter()
        .filter(|f| f.owner == account_keys.first().unwrap().to_string())
        .cloned() // turn &TokenChange into TokenChange
        .collect();

    let mut unique_tokens: Vec<String> = owner_changes
        .iter()
        .filter(|c| !mother_addrs.contains(&c.mint))
        .map(|c| c.mint.clone())
        .collect::<HashSet<_>>() // remove duplicates
        .into_iter()
        .collect();
    
    // Replace WSOL with TARGET_TOKEN if WSOL is present
    let wsol_addr = "So11111111111111111111111111111111111111112";
    if unique_tokens.contains(&wsol_addr.to_string()) {
        unique_tokens.retain(|x| x != wsol_addr);
        if !unique_tokens.contains(&TARGET_TOKEN) {
            unique_tokens.push(TARGET_TOKEN.clone());
        }
    }

    if owner_changes.len() <= 2 && !program_names.is_empty() && unique_tokens.len() > 0 {
        if owner_changes.len() == 1 {
            let pre_balance = (*meta.pre_balances.first().unwrap() as f64) / 1_000_000_000_f64;
            let post_balance = (*meta.post_balances.first().unwrap() as f64) / 1_000_000_000_f64;

            owner_changes.push(TokenChange {
                mint: TARGET_TOKEN.clone(),
                delta: post_balance - pre_balance,
                owner: account_keys.first().unwrap().to_string(),
                post_balance,
                pre_balance,
            });
        };

        let mother_token_addr: Option<(String, u8, [f64; 2], u64, f64, String)> =
            owner_changes.iter().find_map(|c| {
                BASE_TOKENS
                    .iter()
                    .find(|f| c.mint == f.mint && c.delta.abs() > f.threshold)
                    .and_then(|f| {
                        POPULAR_TOKEN_INFO
                            .iter()
                            .find(|t| t.mint == f.mint)
                            .map(|token_info| {
                                (
                                    token_info.mint.to_string(),
                                    token_info.decimals,
                                    f.amount_range.clone(),
                                    f.steps,
                                    f.min_profit,
                                    token_info.symbol.to_string(),
                                )
                            })
                    })
            });

        if mother_token_addr.is_some() {
            Some((
                mother_token_addr.unwrap(),
                owner_changes,
                program_names,
                unique_tokens,
                tx_id,
            ))
        } else {
            None
        }
    } else {
        None
    }
}
