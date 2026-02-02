use solana_rpc_client_nonce_utils::data_from_account;
use solana_sdk::{commitment_config::CommitmentConfig, nonce::state::Data as NonceData};
use tokio::time::{Duration, sleep};
use once_cell::sync::Lazy;
use std::sync::Mutex;

use crate::{NONCE_ADDR, RPC_CLIENT};

pub static GLOBAL_CURRENT_NONCE: Lazy<Mutex<NonceData>> =
    Lazy::new(|| Mutex::new(NonceData::default()));

pub fn set_nonce(new_nonce: NonceData) {
    let mut nonce = GLOBAL_CURRENT_NONCE.lock().unwrap();
    *nonce = new_nonce;
}

pub fn get_nonce() -> NonceData {
    let nonce = GLOBAL_CURRENT_NONCE.lock().unwrap();
    nonce.clone()
}

pub async fn fetch_nonce() {
    loop {
        match RPC_CLIENT
            .get_account_with_commitment(&NONCE_ADDR, CommitmentConfig::processed())
            .await
        {
            Ok(response) => {
                if let Some(account) = response.value {
                    match data_from_account(&account) {
                        Ok(nonce_data) => {
                            set_nonce(nonce_data);
                        }
                        Err(err) => {
                            eprintln!("[NONCE ERROR] Failed to decode nonce: {}", err);
                        }
                    }
                } else {
                    eprintln!("[NONCE ERROR] Nonce account not found.");
                }
            }
            Err(e) => {
                eprintln!(
                    "[NONCE ERROR]\n\t* ERR MSG : {}\n\t* Retrying in 200ms...",
                    e
                );
            }
        }

        sleep(Duration::from_millis(200)).await;
    }
}
