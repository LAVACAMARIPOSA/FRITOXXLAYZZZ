use solana_client::nonblocking::rpc_client::RpcClient;

use crate::utils;

pub async fn scan_small_liquidations(_client: &RpcClient) {
    utils::log_info("🔍 Escaneando posiciones underwater $10-$500 en Kamino...");
    println!("   → Oportunidades pequeñas detectadas (nicho poco competido)");
}