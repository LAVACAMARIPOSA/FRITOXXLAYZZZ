use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::env;

pub const KAMINO_PROGRAM_ID: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";
pub const JUPITER_QUOTE_API: &str = "https://quote-api.jup.ag/v6";

pub const DRY_RUN: bool = true;
pub const MIN_PROFIT_USD: f64 = 0.8;

pub fn get_rpc_url() -> String {
    env::var("RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string())
}

pub fn kamino_program_id() -> Pubkey {
    Pubkey::from_str(KAMINO_PROGRAM_ID).unwrap()
}