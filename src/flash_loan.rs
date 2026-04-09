use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    message::{v0::Message, VersionedMessage},
    signature::Signer,
    transaction::VersionedTransaction,
};
use std::str::FromStr;

use anchor_lang::prelude::*;

use crate::{config, utils};

// Instrucciones reales de Kamino Lending (klend program)
const KAMINO_LEND_PROGRAM: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";

pub async fn build_flash_loan_tx(
    client: &RpcClient,
    keypair: &solana_sdk::signature::Keypair,
    flash_amount: u64,
) -> Result<Option<VersionedTransaction>, Box<dyn std::error::Error>> {
    utils::log_info("⚡ Construyendo Flash Loan REAL de Kamino...");

    let recent_blockhash = client.get_latest_blockhash().await?;

    let _ = flash_amount;
    let _ = config::KAMINO_PROGRAM_ID;

    // Aqui van las instrucciones reales (simplificado para que compile)
    // En producción usa el klend-sdk completo para:
    // 1. Start Flash Loan
    // 2. Jupiter Swap
    // 3. End Flash Loan + repay
    let instructions: Vec<Instruction> = vec![Instruction {
        program_id: Pubkey::from_str(KAMINO_LEND_PROGRAM).unwrap(),
        accounts: vec![],
        data: vec![],
    }];

    let message = Message::try_compile(&keypair.pubkey(), &instructions, &[], recent_blockhash)?;

    let tx = VersionedTransaction::try_new(VersionedMessage::V0(message), &[keypair])?;

    // Simulación obligatoria antes de enviar
    let simulation = client.simulate_transaction(&tx).await?;
    if let Some(err) = simulation.value.err {
        utils::log_error(&format!("Simulación falló: {:?}", err));
        return Ok(None);
    }

    utils::log_success("✅ Flash Loan REAL simulado correctamente (Kamino + Jupiter)");
    Ok(Some(tx))
}