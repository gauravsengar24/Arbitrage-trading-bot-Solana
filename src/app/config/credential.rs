use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct NodeConfig {
    #[serde(rename = "signer_keypair_path", alias = "keypair_path", alias = "wallet_path")]
    pub keypair_path: String,
    #[serde(rename = "rpc_endpoint", alias = "rpc_url")]
    pub rpc_url: String,
    #[serde(rename = "submit_endpoint", alias = "submit_url")]
    pub submit_url: String,
    #[serde(default)]
    pub laser_token: String,
    #[serde(rename = "geyser_endpoint", alias = "geyser_url", alias = "yellowstone_grpc_endpoint", default)]
    pub geyser_url: Option<String>,
    #[serde(rename = "geyser_auth_token", alias = "geyser_token", alias = "yellowstone_grpc_token", default)]
    pub geyser_token: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SwapApiConfig {
    #[serde(rename = "endpoint", alias = "base_url", alias = "jupiter_endpoint")]
    pub base_url: String,
    #[serde(rename = "auth_token", alias = "api_key", alias = "jupiter_api_key", default)]
    pub api_key: String,
    #[serde(default)]
    pub confirm_service: String,
    #[serde(default)]
    pub jito_api_key: String,
    #[serde(default)]
    pub nozomi_api_key: String,
    #[serde(default)]
    pub zero_slot_key: String,
    #[serde(default)]
    pub liljit_endpoint: String,
    #[serde(default)]
    pub astralane_key: String,
    #[serde(default)]
    pub blockrazor_key: String,
    #[serde(default)]
    pub bloxroute_key: String,
    #[serde(default)]
    pub nextblock_key: String,
}
