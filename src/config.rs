use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub const RPC_URL: &str = "https://api.mainnet-beta.solana.com";
pub const KAMINO_PROGRAM_ID: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";
pub const JUPITER_QUOTE_API: &str = "https://quote-api.jup.ag/v6";
pub const JITO_BLOCK_ENGINE_URL: &str = "https://mainnet.block-engine.jito.wtf";

pub const DRY_RUN: bool = true;
pub const MIN_PROFIT_USD: f64 = 0.8;
pub const MAX_TIP_PERCENT: f64 = 0.80;

pub fn kamino_program_id() -> Pubkey {
    Pubkey::from_str(KAMINO_PROGRAM_ID).unwrap()
}