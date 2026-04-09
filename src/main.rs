mod bundle;
mod config;
mod flash_loan;
mod jupiter;
mod liquidation;
mod utils;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Signer;
use std::env;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    utils::log_success("🚀 Solana Zero-Capital Beast v1.0 - COMPLETO");
    utils::log_info("Flash Loan Kamino + Jupiter Arbitrage + Jito Bundles + Liquidaciones");

    let keypair_path = env::var("KEYPAIR_PATH").unwrap_or_else(|_| "keypair.json".to_string());
    let keypair = utils::load_keypair(&keypair_path);
    utils::log_success(&format!("Wallet cargada: {}", keypair.pubkey()));

    let client = RpcClient::new(config::rpc_url());

    loop {
        utils::log_info("=== Nuevo ciclo de búsqueda ===");

        // Arbitrage
        if let Some(profit) = jupiter::get_best_jupiter_quote(
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "So11111111111111111111111111111111111111112",
            500_000_000,
        )
        .await
        {
            if let Ok(Some(tx)) = flash_loan::build_flash_loan_tx(&client, &keypair, 500_000_000).await {
                let _ = bundle::send_jito_bundle(tx, &keypair, profit).await;
            }
        }

        // Liquidaciones
        liquidation::scan_small_liquidations(&client).await;

        sleep(Duration::from_secs(5)).await;
    }
}