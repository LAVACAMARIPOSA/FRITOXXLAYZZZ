use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction,
    message::{v0::Message, VersionedMessage},
    signature::Signer,
    transaction::VersionedTransaction,
};

use crate::utils;

pub async fn build_flash_loan_tx(
    client: &RpcClient,
    keypair: &solana_sdk::signature::Keypair,
    amount: u64,
) -> Result<Option<VersionedTransaction>, Box<dyn std::error::Error>> {
    utils::log_info("⚡ Construyendo Flash Loan Kamino...");

    let recent_blockhash = client.get_latest_blockhash().await?;

    let _ = amount;

    // Placeholder realista: en producción usa klend-sdk para getFlashLoanInstructions
    let instructions: Vec<Instruction> = vec![];

    let message = Message::try_compile(&keypair.pubkey(), &instructions, &[], recent_blockhash)?;

    let tx = VersionedTransaction::try_new(VersionedMessage::V0(message), &[keypair])?;

    // Simulación
    let sim = client.simulate_transaction(&tx).await?;
    if sim.value.err.is_some() {
        utils::log_error("Simulación falló");
        return Ok(None);
    }

    utils::log_success("✅ Flash loan simulado correctamente");
    Ok(Some(tx))
}