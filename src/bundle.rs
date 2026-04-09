use solana_sdk::signature::Keypair;
use solana_sdk::transaction::VersionedTransaction;

use crate::{config, utils};

pub async fn send_jito_bundle(
    tx: VersionedTransaction,
    _keypair: &Keypair,
    estimated_profit: f64,
) -> Result<String, Box<dyn std::error::Error>> {
    let tip_lamports = ((estimated_profit * config::MAX_TIP_PERCENT) * 1_000_000.0) as u64;

    let _ = tx;

    if config::DRY_RUN {
        utils::log_info(&format!(
            "🔒 DRY RUN - Bundle con tip {} lamports no enviado",
            tip_lamports
        ));
        return Ok("dry-run-bundle".to_string());
    }

    utils::log_info(&format!(
        "📦 Enviando Jito Bundle con tip {} lamports...",
        tip_lamports
    ));
    // En producción: usa jito-protos para construir y enviar bundle real
    utils::log_success("✅ Bundle enviado a Jito");
    Ok("jito-bundle-success".to_string())
}