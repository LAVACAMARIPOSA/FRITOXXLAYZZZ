mod bundle;
mod config;
mod flash_loan;
mod jupiter;
mod liquidation;
mod memory;
mod strategy;
mod telegram;
mod utils;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Signer;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use memory::{AgentMemory, OpportunityRecord};
use strategy::{StrategyEngine, Action};
use telegram::{TelegramBot, BotCommand};

/// Agente autonomo 24/7 con memoria persistente
/// Flash Loan Kamino + Jupiter Arbitrage + Jito Bundles + Telegram
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    utils::log_success("Solana Zero-Capital Beast v2.0 - Agente Autonomo");
    utils::log_info("Memoria persistente + Estrategia adaptativa + Telegram");

    if config::DRY_RUN {
        utils::log_warning("MODO DRY RUN - No se enviaran transacciones reales");
    }

    // Load persistent memory
    let mut agent_memory = AgentMemory::load();
    utils::log_success(&format!(
        "Memoria: {} ciclos previos, ${:.2} profit acumulado, riesgo: {:?}",
        agent_memory.total_cycles, agent_memory.total_profit_usd, agent_memory.risk_level
    ));

    let strategy = StrategyEngine::new();

    let mut telegram = TelegramBot::new(
        config::get_telegram_bot_token(),
        config::get_telegram_chat_id(),
    );

    let keypair = utils::load_keypair();
    let user_pubkey = keypair.pubkey();
    utils::log_success(&format!("Wallet: {}", user_pubkey));

    let rpc_url = config::get_rpc_url();
    let client = Arc::new(RpcClient::new(rpc_url));

    match client.get_slot().await {
        Ok(slot) => utils::log_success(&format!("Conectado. Slot: {}", slot)),
        Err(e) => {
            telegram.notify_error(&format!("RPC error: {}", e)).await;
            return Err(e.into());
        }
    }

    let sol_balance = match client.get_balance(&user_pubkey).await {
        Ok(b) => { let s = b as f64 / 1e9; utils::log_info(&format!("Balance: {:.4} SOL", s)); s }
        Err(_) => 0.0,
    };

    // Health server for HuggingFace Spaces (port 7860)
    let health_port = config::get_health_port();
    tokio::spawn(async move {
        use tokio::net::TcpListener;
        use tokio::io::AsyncWriteExt;
        if let Ok(listener) = TcpListener::bind(format!("0.0.0.0:{}", health_port)).await {
            utils::log_info(&format!("Health server :{}", health_port));
            loop {
                if let Ok((mut s, _)) = listener.accept().await {
                    let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK").await;
                }
            }
        }
    });

    telegram.notify_started(&user_pubkey.to_string(), sol_balance).await;

    let mut running = true;
    let mut last_summary_cycle: u64 = 0;
    let mut last_liq_scan_cycle: u64 = 0;
    let mut last_liq_error: Option<String> = None;
    let mut last_adapt_cycle: u64 = 0;

    // ====================================================================
    // MAIN LOOP - 24/7 Autonomous Agent
    // ====================================================================
    loop {
        agent_memory.record_cycle();
        let cycle = agent_memory.total_cycles;

        // 1. Poll Telegram commands
        for cmd in telegram.poll_commands().await {
            match cmd {
                BotCommand::Start => {
                    running = true;
                    telegram.send_alert("Agente ACTIVADO").await;
                }
                BotCommand::Stop => {
                    running = false;
                    telegram.send_alert("Agente PAUSADO").await;
                }
                BotCommand::Status => {
                    let scan_info = agent_memory.scan_summary();
                    telegram.send_message(&format!(
                        "Ciclo: {}\nActivo: {}\nRiesgo: {:?}\nProfit: ${:.2}\nOportunidades: {}\n\n--- Scanner ---\n{}",
                        cycle, running, agent_memory.risk_level,
                        agent_memory.total_profit_usd, agent_memory.total_opportunities,
                        scan_info
                    )).await;
                }
                BotCommand::Stats => {
                    telegram.send_message(&agent_memory.detailed_stats()).await;
                }
                BotCommand::Balance => {
                    if let Ok(b) = client.get_balance(&user_pubkey).await {
                        telegram.send_message(&format!("Balance: {:.4} SOL", b as f64 / 1e9)).await;
                    }
                }
                BotCommand::Aggressive => {
                    agent_memory.set_risk_level(memory::RiskLevel::Aggressive);
                    telegram.send_alert("Modo AGRESIVO").await;
                }
                BotCommand::Safe => {
                    agent_memory.set_risk_level(memory::RiskLevel::Safe);
                    telegram.send_alert("Modo SEGURO").await;
                }
                BotCommand::Memory => {
                    let learning = agent_memory.learning_report();
                    telegram.send_message(&format!(
                        "{}\n\n--- Aprendizaje ---\n{}", agent_memory.summary(), learning
                    )).await;
                }
                BotCommand::Reset => {
                    agent_memory.reset();
                    telegram.send_alert("Memoria reseteada").await;
                }
                BotCommand::Help => {
                    telegram.send_message(&telegram::TelegramBot::help_text()).await;
                }
                BotCommand::Unknown(msg) => {
                    // Ask Groq AI for a conversational response
                    let learning_ctx = agent_memory.learning_report();
                    let scan_ctx = agent_memory.scan_summary();
                    let context = format!(
                        "Ciclos: {}\nActivo: {}\nRiesgo: {:?}\nProfit total: ${:.2}\nOportunidades: {}\nFallos: {}\nRacha: {}\nDRY_RUN: {}\n\nScanner:\n{}\n\nAprendizaje:\n{}",
                        cycle, running, agent_memory.risk_level,
                        agent_memory.total_profit_usd, agent_memory.total_opportunities,
                        agent_memory.total_failures, agent_memory.current_win_streak,
                        config::DRY_RUN, scan_ctx, learning_ctx
                    );
                    if let Some(ai_cmd) = telegram.handle_unknown_with_ai(&msg, &context).await {
                        // AI detected user intent - execute silently (AI already replied)
                        match ai_cmd {
                            BotCommand::Start => {
                                running = true;
                            }
                            BotCommand::Stop => {
                                running = false;
                            }
                            BotCommand::Aggressive => {
                                agent_memory.set_risk_level(memory::RiskLevel::Aggressive);
                            }
                            BotCommand::Safe => {
                                agent_memory.set_risk_level(memory::RiskLevel::Safe);
                            }
                            BotCommand::Reset => {
                                agent_memory.reset();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // 2. Execute strategies if running
        if running {
            if cycle % 20 == 1 {
                utils::log_info(&format!("=== Ciclo #{} | Riesgo: {:?} | Profit: ${:.2} ===",
                    cycle, agent_memory.risk_level, agent_memory.total_profit_usd));
            }

            // STRATEGY 1: ALL arbitrage strategies (round-trip, triangular, stablecoin, LST)
            let flash_amount = strategy.get_flash_amount(&agent_memory);

            let scan_report = jupiter::scan_all_strategies(flash_amount, &mut agent_memory).await;

            // Send scan report to Telegram every cycle so user sees everything
            let tg_msg = scan_report.to_telegram_message(cycle, agent_memory.get_api_delay_ms());
            telegram.send_message(&tg_msg).await;

            // If we found a profitable opportunity, evaluate and try to execute
            if let Some((profit, ref route)) = scan_report.best {
                let decision = strategy.evaluate_arbitrage(&agent_memory, profit, 0.0);
                if let Action::Go = decision.action {
                    agent_memory.total_opportunities += 1;
                    telegram.send_alert(&format!(
                        "OPORTUNIDAD #{}\n{}\nProfit: +${:.4}\nConfianza: {:.0}%\nEjecutando...",
                        agent_memory.total_opportunities, route, profit, decision.confidence * 100.0
                    )).await;

                    match flash_loan::build_flash_loan_tx(&client, &keypair, flash_amount).await {
                        Ok(Some(tx)) => {
                            let success = match bundle::send_jito_bundle(tx, &keypair, profit).await {
                                Ok(id) => {
                                    telegram.send_alert(&format!("EJECUTADO\nBundle: {}\nProfit: +${:.4}", id, profit)).await;
                                    true
                                }
                                Err(e) => {
                                    telegram.send_alert(&format!("FALLO ejecucion: {}", e)).await;
                                    false
                                }
                            };
                            agent_memory.record_opportunity(OpportunityRecord {
                                timestamp: memory::current_timestamp(),
                                strategy: "arbitrage".into(),
                                route: route.clone(),
                                estimated_profit_usd: profit,
                                actual_profit_usd: if success && !config::DRY_RUN { profit } else { 0.0 },
                                success,
                                congestion_level: 0.5,
                                tip_lamports: 0,
                                hour_utc: (memory::current_timestamp() / 3600 % 24) as u8,
                            });
                        }
                        Ok(None) => {
                            telegram.send_alert("FALLO: No se pudo construir flash loan TX").await;
                        }
                        Err(e) => {
                            telegram.send_alert(&format!("FALLO flash loan: {}", e)).await;
                        }
                    }
                } else {
                    // Strategy engine rejected it - tell user why
                    telegram.send_message(&format!(
                        "Oportunidad rechazada: {} +${:.4}\nRazon: {}", route, profit, decision.reason
                    )).await;
                }
            }

            // STRATEGY 2: Liquidations (throttled with adaptive interval)
            let liq_interval = agent_memory.get_liq_scan_interval();
            if strategy.should_scan_liquidations(&agent_memory) && (cycle - last_liq_scan_cycle >= liq_interval) {
                last_liq_scan_cycle = cycle;
                let scan_result = liquidation::scan_small_liquidations(&client).await;

                // Track metrics
                let has_error = scan_result.scan_error.is_some();
                agent_memory.record_liq_scan(
                    scan_result.total_obligations_fetched,
                    scan_result.total_with_debt,
                    scan_result.total_in_range,
                    has_error,
                );

                // Always report liquidation scan results
                if let Some(ref err) = scan_result.scan_error {
                    telegram.send_message(&format!(
                        "--- Liq Scan #{} ---\nERROR: {}", cycle, err
                    )).await;
                    last_liq_error = Some(err.clone());
                } else {
                    last_liq_error = None;
                    telegram.send_message(&format!(
                        "--- Liq Scan #{} ---\nObligaciones: {}\nCon deuda: {}\nEn rango $10-$500: {}\nLiquidables: {}",
                        cycle,
                        scan_result.total_obligations_fetched,
                        scan_result.total_with_debt,
                        scan_result.total_in_range,
                        scan_result.opportunities.len()
                    )).await;
                }

                for opp in &scan_result.opportunities {
                    let decision = strategy.evaluate_liquidation(
                        &agent_memory, opp.health_factor,
                        opp.estimated_profit_usd, opp.borrow_factor_adjusted_debt_usd,
                    );
                    if let Action::Go = decision.action {
                        agent_memory.total_opportunities += 1;
                        telegram.send_alert(&format!(
                            "LIQUIDACION #{}\nHF: {:.4}\nDeuda: ${:.2}\nProfit est: ${:.2}\nEjecutando...",
                            agent_memory.total_opportunities, opp.health_factor,
                            opp.borrow_factor_adjusted_debt_usd, opp.estimated_profit_usd
                        )).await;

                        if config::DRY_RUN {
                            telegram.send_message(&format!(
                                "DRY RUN: Liquidacion simulada OK (no se envio TX real)"
                            )).await;
                            agent_memory.record_opportunity(OpportunityRecord {
                                timestamp: memory::current_timestamp(),
                                strategy: "liquidation".into(),
                                route: format!("liq-{}", &opp.obligation_pubkey.to_string()[..8]),
                                estimated_profit_usd: opp.estimated_profit_usd,
                                actual_profit_usd: 0.0,
                                success: true,
                                congestion_level: 0.5,
                                tip_lamports: 0,
                                hour_utc: (memory::current_timestamp() / 3600 % 24) as u8,
                            });
                        } else {
                            let amt = (opp.borrow_factor_adjusted_debt_usd * 0.5 * 1e6) as u64;
                            if let Ok(Some(tx)) = flash_loan::build_flash_loan_tx(&client, &keypair, amt).await {
                                if let Ok(id) = bundle::send_jito_bundle(tx, &keypair, opp.estimated_profit_usd).await {
                                    telegram.notify_execution(&id, opp.estimated_profit_usd).await;
                                    agent_memory.record_opportunity(OpportunityRecord {
                                        timestamp: memory::current_timestamp(),
                                        strategy: "liquidation".into(),
                                        route: format!("liq-{}", &opp.obligation_pubkey.to_string()[..8]),
                                        estimated_profit_usd: opp.estimated_profit_usd,
                                        actual_profit_usd: opp.estimated_profit_usd,
                                        success: true,
                                        congestion_level: 0.5,
                                        tip_lamports: 0,
                                        hour_utc: (memory::current_timestamp() / 3600 % 24) as u8,
                                    });
                                }
                            }
                        }
                    } else {
                        // Strategy engine rejected - tell user why
                        telegram.send_message(&format!(
                            "Liq rechazada: HF:{:.4} Deuda:${:.2}\nRazon: {}",
                            opp.health_factor, opp.borrow_factor_adjusted_debt_usd, decision.reason
                        )).await;
                    }
                }
            }

            // STRATEGY 3: Flash loan test (DRY_RUN, every 10 cycles)
            if config::DRY_RUN && cycle % 10 == 0 {
                if let Ok(Some(_)) = flash_loan::build_simple_flash_loan_tx(
                    &client, &keypair, 1_000_000, flash_loan::USDC_RESERVE, flash_loan::USDC_MINT,
                ).await {
                    utils::log_success("Flash loan test OK");
                }
            }

            // LEARNING: Adapt after every 3 cycles based on accumulated experience
            if cycle - last_adapt_cycle >= 3 {
                last_adapt_cycle = cycle;
                agent_memory.adapt_after_scan();

                // Log what the bot learned every 60 cycles (~5 min)
                if cycle % 60 == 0 {
                    let report = agent_memory.learning_report();
                    utils::log_info(&format!("Learning:\n{}", report));
                }
            }
        }

        // 3. Persist memory (every 5 cycles, or after opportunities via record_opportunity)
        if cycle % 5 == 0 { agent_memory.save(); }

        // 4. Telegram summary every ~30 min (360 cycles * 5s)
        if cycle - last_summary_cycle >= 360 {
            last_summary_cycle = cycle;
            let scan_info = agent_memory.scan_summary();
            telegram.notify_summary(&format!(
                "{}\n\n--- Scanner ---\n{}", agent_memory.summary(), scan_info
            )).await;
        }

        // 5. Adaptive delay
        sleep(Duration::from_secs(strategy.get_cycle_delay_secs(&agent_memory))).await;
    }
}
