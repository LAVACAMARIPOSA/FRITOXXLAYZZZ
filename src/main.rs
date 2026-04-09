mod bundle;
mod config;
mod flash_loan;
mod jupiter;
mod liquidation;
mod utils;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Signer;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Estructura para trackear estadísticas del bot
#[derive(Debug, Default)]
struct BotStats {
    cycles: u64,
    arbitrage_opportunities: u64,
    liquidation_opportunities: u64,
    bundles_sent: u64,
    total_profit_usd: f64,
}

impl BotStats {
    fn new() -> Self {
        Self::default()
    }
    
    fn log_summary(&self) {
        utils::log_info(&format!("\n📊 Estadísticas:"));
        utils::log_info(&format!("   - Ciclos ejecutados: {}", self.cycles));
        utils::log_info(&format!("   - Oportunidades arbitrage: {}", self.arbitrage_opportunities));
        utils::log_info(&format!("   - Oportunidades liquidación: {}", self.liquidation_opportunities));
        utils::log_info(&format!("   - Bundles enviados: {}", self.bundles_sent));
        utils::log_info(&format!("   - Profit total estimado: ${:.2}", self.total_profit_usd));
    }
}

/// Bot de Flash Loans + Arbitrage + Liquidaciones
/// Kamino Lending + Jupiter Swaps + Jito Bundles
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inicializar logging
    tracing_subscriber::fmt::init();
    
    utils::log_success("🚀 Solana Zero-Capital Beast v1.0 - Iniciando");
    utils::log_info("Flash Loan Kamino + Jupiter Arbitrage + Jito Bundles");
    
    if config::DRY_RUN {
        utils::log_warning("🔒 MODO DRY RUN ACTIVADO - No se enviarán transacciones reales");
        utils::log_info("   Para activar modo producción, cambia DRY_RUN a false en config.rs");
    }

    // Cargar keypair
    let keypair = utils::load_keypair();
    let user_pubkey = keypair.pubkey();
    utils::log_success(&format!("✅ Wallet cargada: {}", user_pubkey));
    
    // Crear cliente RPC
    let rpc_url = config::get_rpc_url();
    utils::log_info(&format!("🔗 Conectando a: {}", rpc_url));
    
    let client = Arc::new(RpcClient::new(rpc_url));
    
    // Verificar conexión
    match client.get_slot().await {
        Ok(slot) => utils::log_success(&format!("✅ Conectado. Slot actual: {}", slot)),
        Err(e) => {
            utils::log_error(&format!("❌ Error conectando al RPC: {}", e));
            return Err(e.into());
        }
    }
    
    // Obtener balance
    match client.get_balance(&user_pubkey).await {
        Ok(balance) => {
            let sol_balance = balance as f64 / 1_000_000_000.0;
            utils::log_info(&format!("💰 Balance: {:.4} SOL", sol_balance));
            if balance < 10_000_000 {
                utils::log_warning("⚠️ Balance muy bajo. Se necesitan al menos 0.01 SOL para fees");
            }
        }
        Err(e) => utils::log_error(&format!("❌ Error obteniendo balance: {}", e)),
    }

    // Estadísticas
    let mut stats = BotStats::new();
    
    // Loop principal del bot
    loop {
        stats.cycles += 1;
        let cycle = stats.cycles;
        
        utils::log_info(&format!("\n🔄 === Ciclo #{} ===", cycle));
        
        // ============================================================
        // ESTRATEGIA 1: Arbitrage con Flash Loans
        // ============================================================
        utils::log_info("📊 Estrategia 1: Arbitrage con Flash Loans");
        
        // Buscar oportunidades de arbitrage USDC <-> SOL
        let flash_amount = 500_000_000u64; // 500 USDC (6 decimales)
        let input_mint = flash_loan::USDC_MINT;
        let output_mint = flash_loan::SOL_MINT;
        
        // Obtener quote de Jupiter
        match jupiter::get_best_jupiter_quote(input_mint, output_mint, flash_amount).await {
            Some(profit) => {
                stats.arbitrage_opportunities += 1;
                stats.total_profit_usd += profit;
                utils::log_success(&format!("💰 Oportunidad de arbitrage detectada: +${:.2}", profit));
                
                // Construir transacción de flash loan con arbitrage
                match flash_loan::build_flash_loan_tx(&client, &keypair, flash_amount).await {
                    Ok(Some(tx)) => {
                        // Enviar bundle a Jito
                        match bundle::send_jito_bundle(tx, &keypair, profit).await {
                            Ok(bundle_id) => {
                                stats.bundles_sent += 1;
                                utils::log_success(&format!("✅ Bundle enviado: {}", bundle_id));
                            }
                            Err(e) => {
                                utils::log_error(&format!("❌ Error enviando bundle: {}", e));
                            }
                        }
                    }
                    Ok(None) => {
                        utils::log_warning("⚠️ No se pudo construir la transacción de flash loan");
                    }
                    Err(e) => {
                        utils::log_error(&format!("❌ Error construyendo flash loan: {}", e));
                    }
                }
            }
            None => {
                utils::log_info("ℹ️ No se encontraron oportunidades de arbitrage rentables");
            }
        }
        
        // ============================================================
        // ESTRATEGIA 2: Escaneo de Liquidaciones
        // ============================================================
        utils::log_info("📊 Estrategia 2: Escaneo de Liquidaciones");
        
        let opportunities = liquidation::scan_small_liquidations(&client).await;
        
        if !opportunities.is_empty() {
            stats.liquidation_opportunities += opportunities.len() as u64;
            utils::log_success(&format!("🎯 {} oportunidades de liquidación encontradas", opportunities.len()));
            
            // Procesar cada oportunidad
            for (idx, opp) in opportunities.iter().enumerate() {
                utils::log_info(&format!("\n  📌 Oportunidad #{}", idx + 1));
                utils::log_info(&format!("     Obligation: {}", opp.obligation_pubkey));
                utils::log_info(&format!("     Health Factor: {:.4}", opp.health_factor));
                utils::log_info(&format!("     Depósitos: ${:.2}", opp.deposited_value_usd));
                utils::log_info(&format!("     Deuda: ${:.2}", opp.borrow_factor_adjusted_debt_usd));
                utils::log_info(&format!("     Ganancia Estimada: ${:.2}", opp.estimated_profit_usd));
                
                // Verificar si es rentable
                if opp.estimated_profit_usd >= config::MIN_PROFIT_USD {
                    utils::log_success("     ✅ Rentable - Procesando...");
                    
                    if config::DRY_RUN {
                        utils::log_info("     🔒 DRY RUN - Simulando flujo de liquidación:");
                        utils::log_info("        1. Flash Borrow USDC del reserve Kamino");
                        utils::log_info(&format!("        2. Liquidar obligation {} (HF: {:.4})", opp.obligation_pubkey, opp.health_factor));
                        utils::log_info("        3. Recibir collateral + bonus (~7%)");
                        utils::log_info("        4. Swap collateral -> USDC via Jupiter");
                        utils::log_info("        5. Repay flash loan + fee (0.09%)");
                        utils::log_info("        6. Enviar bundle atómico a Jito");
                        utils::log_info(&format!("        💵 Profit neto estimado: ${:.2}", opp.estimated_profit_usd));
                        stats.total_profit_usd += opp.estimated_profit_usd;
                    } else {
                        // Producción: ejecutar liquidación real
                        let flash_amount = (opp.borrow_factor_adjusted_debt_usd * 0.5 * 1_000_000.0) as u64;
                        match flash_loan::build_flash_loan_tx(&client, &keypair, flash_amount).await {
                            Ok(Some(tx)) => {
                                match bundle::send_jito_bundle(tx, &keypair, opp.estimated_profit_usd).await {
                                    Ok(bundle_id) => {
                                        stats.bundles_sent += 1;
                                        stats.total_profit_usd += opp.estimated_profit_usd;
                                        utils::log_success(&format!("     ✅ Liquidación ejecutada! Bundle: {}", bundle_id));
                                    }
                                    Err(e) => utils::log_error(&format!("     ❌ Error en bundle: {}", e)),
                                }
                            }
                            Ok(None) => utils::log_warning("     ⚠️ No se pudo construir tx de liquidación"),
                            Err(e) => utils::log_error(&format!("     ❌ Error: {}", e)),
                        }
                    }
                } else {
                    utils::log_info(&format!("     ℹ️ No rentable (min: ${})", config::MIN_PROFIT_USD));
                }
            }
        } else {
            utils::log_info("ℹ️ No se encontraron oportunidades de liquidación");
        }
        
        // ============================================================
        // ESTRATEGIA 3: Flash Loan Simple (Testing)
        // ============================================================
        // Solo ejecutar en modo test con DRY_RUN cada 10 ciclos
        if config::DRY_RUN && cycle % 10 == 0 {
            utils::log_info("📊 Estrategia 3: Test de Flash Loan Simple");
            
            let test_amount = 1_000_000u64; // 1 USDC
            match flash_loan::build_simple_flash_loan_tx(
                &client,
                &keypair,
                test_amount,
                flash_loan::USDC_RESERVE,
                flash_loan::USDC_MINT,
            ).await {
                Ok(Some(_tx)) => {
                    utils::log_success("✅ Flash loan simple construido correctamente");
                }
                Ok(None) => {
                    utils::log_warning("⚠️ No se pudo construir flash loan simple");
                }
                Err(e) => {
                    utils::log_error(&format!("❌ Error en flash loan simple: {}", e));
                }
            }
        }
        
        // ============================================================
        // ESTADÍSTICAS Y ESPERA
        // ============================================================
        if cycle % 10 == 0 {
            stats.log_summary();
        }
        
        // Esperar antes del siguiente ciclo
        let sleep_secs = 5u64;
        utils::log_info(&format!("\n⏳ Esperando {} segundos...", sleep_secs));
        sleep(Duration::from_secs(sleep_secs)).await;
    }
}
