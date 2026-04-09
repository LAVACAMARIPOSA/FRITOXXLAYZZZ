use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::env;

pub const KAMINO_PROGRAM_ID: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";
pub const JUPITER_QUOTE_API: &str = "https://quote-api.jup.ag/v6";

// Jito Block Engine Configuration
pub const JITO_BLOCK_ENGINE_URL: &str = "https://mainnet.block-engine.jito.wtf";
pub const JITO_BLOCK_ENGINE_GRPC: &str = "mainnet.block-engine.jito.wtf:443";

// Feature flags
pub const DRY_RUN: bool = true;
pub const MIN_PROFIT_USD: f64 = 0.8;

// Bundle configuration
pub const BUNDLE_TIMEOUT_SECS: u64 = 30;
pub const MAX_BUNDLE_RETRIES: u32 = 3;

pub fn get_rpc_url() -> String {
    env::var("RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string())
}

pub fn get_jito_block_engine_url() -> String {
    env::var("JITO_BLOCK_ENGINE_URL")
        .unwrap_or_else(|_| JITO_BLOCK_ENGINE_URL.to_string())
}

pub fn get_jito_auth_key() -> Option<String> {
    env::var("JITO_AUTH_KEY").ok()
}

pub fn kamino_program_id() -> Pubkey {
    Pubkey::from_str(KAMINO_PROGRAM_ID).unwrap()
}