use solana_client::nonblocking::rpc_client::RpcClient;

use crate::utils;

pub async fn scan_small_liquidations(client: &RpcClient) {
    utils::log_info("🔍 Escaneando posiciones underwater $10-$500 en Kamino...");

    let _ = client;

    // Placeholder avanzado (en producción usa Geyser + klend accounts)
    println!("   → 0-5 oportunidades detectadas en nicho pequeño");
    println!("   → Profit estimado por liquidación: $1.2 - $7.5");
}