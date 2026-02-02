use once_cell::sync::Lazy;
use serde_json::Value;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
};
use std::{fs, sync::Arc};

use crate::app::config;

fn load_keypair_from_str(key_str: &str, path: &str) -> Keypair {
    if let Ok(json_val) = serde_json::from_str::<Value>(key_str) {
        let bytes: Vec<u8> = serde_json::from_value(json_val).expect("Invalid JSON key format");
        Keypair::try_from(&bytes[..]).expect("Failed to create Keypair from JSON")
    } else {
        let key_bytes = bs58::decode(key_str)
            .into_vec()
            .unwrap_or_else(|e| panic!("Key file is not valid Base58: {}. Path: {}", e, path));
        Keypair::try_from(&key_bytes[..])
            .unwrap_or_else(|e| panic!("Failed to create Keypair from key file: {}. Path: {}", e, path))
    }
}

pub static PRIVATE_KEY: Lazy<Keypair> = Lazy::new(|| {
    let path = config::CONFIG.node.keypair_path.clone();
    let content = fs::read_to_string(&path).expect("Unable to load Key file");
    load_keypair_from_str(content.trim(), &path)
});

pub async fn keyfile_status() -> Keypair {
    let path = config::CONFIG.node.keypair_path.clone();
    let content = fs::read_to_string(&path).expect("Unable to load Key file");
    load_keypair_from_str(content.trim(), &path)
}

pub static PUBKEY: Lazy<Pubkey> = Lazy::new(|| PRIVATE_KEY.pubkey());

pub static RPC_ENDPOINT: Lazy<String> = Lazy::new(|| config::CONFIG.node.rpc_url.clone());
pub static SUBMIT_ENDPOINT: Lazy<String> = Lazy::new(|| config::CONFIG.node.submit_url.clone());

pub static RPC_CLIENT: Lazy<Arc<RpcClient>> = Lazy::new(|| {
    Arc::new(RpcClient::new_with_commitment(
        config::CONFIG.node.rpc_url.clone(),
        CommitmentConfig::processed(),
    ))
});

pub static SUBMIT_CLIENT: Lazy<Arc<RpcClient>> = Lazy::new(|| {
    Arc::new(RpcClient::new_with_commitment(
        config::CONFIG.node.submit_url.clone(),
        CommitmentConfig::processed(),
    ))
});

pub static YELLOWSTONE_GRPC_ENDPOINT: Lazy<Option<String>> =
    Lazy::new(|| config::CONFIG.node.geyser_url.clone());
pub static YELLOWSTONE_GRPC_TOKEN: Lazy<Option<String>> =
    Lazy::new(|| config::CONFIG.node.geyser_token.clone());

pub static SIGNERS: Lazy<Vec<&'static Keypair>> = Lazy::new(|| vec![&*PRIVATE_KEY]);
