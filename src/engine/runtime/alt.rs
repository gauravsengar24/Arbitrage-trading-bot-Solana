use solana_sdk::{
    address_lookup_table::state::AddressLookupTable,
    message::AddressLookupTableAccount, pubkey::Pubkey,
};

use crate::{ALT_EXTERNAL, RPC_CLIENT};

pub async fn fetch_alt(lut_addrs: Vec<Pubkey>) -> Vec<AddressLookupTableAccount> {

    let mut new_addr = Vec::new();

    new_addr.extend(ALT_EXTERNAL.iter().copied());
    new_addr.extend(lut_addrs);

    let accounts = RPC_CLIENT
        .get_multiple_accounts(&new_addr)
        .await
        .unwrap();

    let look_up_table_addresses: Vec<AddressLookupTableAccount> = accounts
        .into_iter()
        .zip(new_addr.into_iter())
        .filter_map(|(maybe_account, lut_addr)| {
            if let Some(account) = maybe_account {
                let lut_data = AddressLookupTable::deserialize(&account.data).ok()?;
                Some(AddressLookupTableAccount {
                    key: lut_addr,
                    addresses: lut_data.addresses.to_vec(),
                })
            } else {
                None
            }
        })
        .collect();

    look_up_table_addresses
}