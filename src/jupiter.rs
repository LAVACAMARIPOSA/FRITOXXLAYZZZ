use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::VersionedTransaction,
};
use crate::{config, memory::AgentMemory, utils};

/// Base URL para la API de Jupiter v6
const JUPITER_API_BASE: &str = "https://quote-api.jup.ag/v6";

/// Número máximo de reintentos para requests fallidos
const MAX_RETRIES: u32 = 3;

/// Delay base entre reintentos (en ms)
const RETRY_DELAY_MS: u64 = 500;

// =============================================================================
// ESTRUCTURAS DE DATOS PARA LA API DE JUPITER
// =============================================================================

/// Respuesta de la API de quote de Jupiter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterQuoteResponse {
    /// Input mint
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    /// Output mint
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    /// Amount de input
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    /// Amount de output
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    /// Otras amounts (con slippage)
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    /// Slippage en bps
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    /// Swap mode
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    /// Platform fee
    #[serde(rename = "platformFee")]
    pub platform_fee: Option<PlatformFee>,
    /// Price impact
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    /// Route plan
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<RouteStep>,
    /// Context slot
    #[serde(rename = "contextSlot")]
    pub context_slot: Option<u64>,
    /// Time taken
    #[serde(rename = "timeTaken")]
    pub time_taken: Option<f64>,
}

/// Fee de plataforma
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformFee {
    #[serde(rename = "amount")]
    pub amount: String,
    #[serde(rename = "feeBps")]
    pub fee_bps: u16,
}

/// Paso en la ruta de swap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStep {
    #[serde(rename = "swapInfo")]
    pub swap_info: SwapInfo,
    pub percent: u8,
}

/// Información del swap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapInfo {
    #[serde(rename = "ammKey")]
    pub amm_key: String,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "feeAmount")]
    pub fee_amount: String,
    #[serde(rename = "feeMint")]
    pub fee_mint: String,
}

/// Request para el swap
#[derive(Debug, Clone, Serialize)]
pub struct JupiterSwapRequest {
    /// Respuesta del quote
    #[serde(rename = "quoteResponse")]
    pub quote_response: JupiterQuoteResponse,
    /// Public key del usuario
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    /// Wrap and unwrap SOL
    #[serde(rename = "wrapAndUnwrapSol")]
    pub wrap_and_unwrap_sol: bool,
    /// Fee de prioridad (opcional)
    #[serde(rename = "prioritizationFeeLamports", skip_serializing_if = "Option::is_none")]
    pub prioritization_fee_lamports: Option<u64>,
    /// Fee de computación (opcional)
    #[serde(rename = "computeUnitPriceMicroLamports", skip_serializing_if = "Option::is_none")]
    pub compute_unit_price_micro_lamports: Option<u64>,
    /// Slippage dinámico (opcional)
    #[serde(rename = "dynamicSlippage", skip_serializing_if = "Option::is_none")]
    pub dynamic_slippage: Option<DynamicSlippage>,
    /// As legacy transaction
    #[serde(rename = "asLegacyTransaction", skip_serializing_if = "Option::is_none")]
    pub as_legacy_transaction: Option<bool>,
    /// Use shared accounts
    #[serde(rename = "useSharedAccounts", skip_serializing_if = "Option::is_none")]
    pub use_shared_accounts: Option<bool>,
    /// Destination token account (opcional)
    #[serde(rename = "destinationTokenAccount", skip_serializing_if = "Option::is_none")]
    pub destination_token_account: Option<String>,
}

/// Configuración de slippage dinámico
#[derive(Debug, Clone, Serialize)]
pub struct DynamicSlippage {
    #[serde(rename = "minBps")]
    pub min_bps: u16,
    #[serde(rename = "maxBps")]
    pub max_bps: u16,
}

/// Respuesta del swap de Jupiter
#[derive(Debug, Clone, Deserialize)]
pub struct JupiterSwapResponse {
    /// Transacción en base64
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    /// Last valid block height
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: Option<u64>,
    /// Prioritization fee
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<u64>,
    /// Compute unit limit
    #[serde(rename = "computeUnitLimit")]
    pub compute_unit_limit: Option<u32>,
    /// Prioritization type
    #[serde(rename = "prioritizationType")]
    pub prioritization_type: Option<PrioritizationType>,
    /// Dynamic slippage report
    #[serde(rename = "dynamicSlippageReport")]
    pub dynamic_slippage_report: Option<DynamicSlippageReport>,
    /// Simulation error
    #[serde(rename = "simulationError")]
    pub simulation_error: Option<SimulationError>,
}

/// Tipo de priorización
#[derive(Debug, Clone, Deserialize)]
pub struct PrioritizationType {
    #[serde(rename = "computeBudget")]
    pub compute_budget: Option<ComputeBudget>,
}

/// Compute budget
#[derive(Debug, Clone, Deserialize)]
pub struct ComputeBudget {
    #[serde(rename = "microLamports")]
    pub micro_lamports: Option<u64>,
    #[serde(rename = "estimatedMicroLamports")]
    pub estimated_micro_lamports: Option<u64>,
}

/// Reporte de slippage dinámico
#[derive(Debug, Clone, Deserialize)]
pub struct DynamicSlippageReport {
    #[serde(rename = "slippageBps")]
    pub slippage_bps: Option<u16>,
    #[serde(rename = "otherAmount")]
    pub other_amount: Option<u64>,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: Option<u64>,
    #[serde(rename = "simulatedIncurredSlippageBps")]
    pub simulated_incurred_slippage_bps: Option<u16>,
}

/// Error de simulación
#[derive(Debug, Clone, Deserialize)]
pub struct SimulationError {
    #[serde(rename = "errorCode")]
    pub error_code: Option<String>,
    #[serde(rename = "error")]
    pub error: Option<String>,
}

/// Transacción firmada lista para enviar
#[derive(Debug, Clone)]
pub struct SignedSwapTransaction {
    /// La transacción firmada
    pub transaction: VersionedTransaction,
    /// Last valid block height
    pub last_valid_block_height: Option<u64>,
    /// Prioritization fee usada
    pub prioritization_fee_lamports: Option<u64>,
}

/// Instrucciones extraídas del swap para bundles
#[derive(Debug, Clone)]
pub struct JupiterSwapInstructions {
    /// Instrucciones del swap
    pub instructions: Vec<Instruction>,
    /// Account keys necesarios
    pub account_keys: Vec<Pubkey>,
    /// Blockhash usado
    pub blockhash: Hash,
    /// Last valid block height
    pub last_valid_block_height: Option<u64>,
}

/// Configuración de slippage basada en volatilidad
#[derive(Debug, Clone)]
pub struct SlippageConfig {
    /// Slippage base en bps
    pub base_bps: u16,
    /// Slippage máximo en bps
    pub max_bps: u16,
    /// Multiplicador de volatilidad
    pub volatility_multiplier: f64,
}

impl Default for SlippageConfig {
    fn default() -> Self {
        Self {
            base_bps: 50,      // 0.5%
            max_bps: 500,      // 5%
            volatility_multiplier: 1.5,
        }
    }
}

// =============================================================================
// FUNCIONES PÚBLICAS
// =============================================================================

/// Result of a round-trip quote scan: profit in USD and spread percentage.
pub struct QuoteScanResult {
    /// Profit in USD (negative means loss)
    pub profit_usd: f64,
    /// Spread as a percentage of the input amount
    pub spread_pct: f64,
    /// Number of successful quotes in this scan
    pub quotes_ok: u64,
    /// Number of failed quotes in this scan
    pub quotes_failed: u64,
}

/// Busca oportunidades de arbitrage haciendo un round-trip:
/// USDC -> intermediate_token -> USDC
///
/// Compara lo que sale al final vs lo que entró (ambos en USDC).
/// Always returns the spread info (even when unprofitable) for metrics.
/// Returns Ok(Some(profit)) if profitable, Ok(None) if not, Err if both quotes failed.
pub async fn get_best_jupiter_quote(
    input_mint: &str,
    output_mint: &str,
    amount: u64,
) -> Result<QuoteScanResult, String> {
    let slippage_config = SlippageConfig::default();

    // Leg 1: USDC -> intermediate (e.g. SOL)
    let quote_leg1 = match get_jupiter_quote_with_slippage(
        input_mint, output_mint, amount, &slippage_config,
    ).await {
        Ok(q) => q,
        Err(e) => {
            return Err(format!("leg1: {}", e));
        }
    };

    let intermediate_amount = quote_leg1.out_amount.parse::<u64>().unwrap_or(0);
    if intermediate_amount == 0 {
        return Err("leg1 returned 0".to_string());
    }

    // Small delay between legs to avoid rate limiting
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Leg 2: intermediate -> USDC (back to start)
    let quote_leg2 = match get_jupiter_quote_with_slippage(
        output_mint, input_mint, intermediate_amount, &slippage_config,
    ).await {
        Ok(q) => q,
        Err(e) => {
            return Err(format!("leg2: {}", e));
        }
    };

    let final_amount = quote_leg2.out_amount.parse::<u64>().unwrap_or(0);

    // Calculate spread including flash loan fee
    let flash_fee = amount * 9 / 10_000; // 0.09%
    let total_cost = amount + flash_fee;

    let profit_raw = final_amount as i64 - total_cost as i64;
    let profit_usd = profit_raw as f64 / 1_000_000.0;
    let spread_pct = profit_raw as f64 / amount as f64 * 100.0;

    if profit_usd > config::MIN_PROFIT_USD {
        let impact1 = &quote_leg1.price_impact_pct;
        let impact2 = &quote_leg2.price_impact_pct;
        utils::log_success(&format!(
            "Arbitrage round-trip: ${:.4} profit (impacts: {}% / {}%)",
            profit_usd, impact1, impact2
        ));
    }

    Ok(QuoteScanResult {
        profit_usd,
        spread_pct,
        quotes_ok: 2,
        quotes_failed: 0,
    })
}

// =============================================================================
// TOKEN REGISTRY
// =============================================================================

pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const MSOL_MINT: &str = "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So";
pub const JITOSOL_MINT: &str = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn";
pub const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
pub const BSOL_MINT: &str = "bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1";
pub const BONK_MINT: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const RAY_MINT: &str = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R";
pub const ORCA_MINT: &str = "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE";
pub const PYTH_MINT: &str = "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3";
pub const WIF_MINT: &str = "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm";
pub const JUP_MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

// =============================================================================
// STRATEGY: ROUND-TRIP ARBITRAGE (A -> B -> A)
// =============================================================================

/// Busca arbitrage en múltiples pares, priorizando rutas que el bot aprendió son mejores.
/// Salta rutas en backoff (por fallos anteriores). Adapta delays según experiencia.
pub async fn scan_arbitrage_opportunities(
    base_mint: &str,
    amount: u64,
    memory: &mut AgentMemory,
) -> Option<(f64, String)> {
    let intermediates: Vec<(&str, &str)> = vec![
        (SOL_MINT, "SOL"),
        (MSOL_MINT, "mSOL"),
        (JITOSOL_MINT, "jitoSOL"),
        (USDT_MINT, "USDT"),
        (BSOL_MINT, "bSOL"),
        (BONK_MINT, "BONK"),
        (RAY_MINT, "RAY"),
        (WIF_MINT, "WIF"),
        (JUP_MINT, "JUP"),
    ];

    // Build route names and prioritize using learned data
    let route_names: Vec<String> = intermediates.iter()
        .map(|(_, sym)| format!("USDC->{}->USDC", sym))
        .collect();
    let prioritized = memory.prioritized_routes(&route_names);

    let api_delay = memory.get_api_delay_ms();
    let mut best: Option<(f64, String)> = None;
    let mut scanned = 0u32;
    let mut skipped = 0u32;

    for route_name in &prioritized {
        // Find the corresponding mint
        let (mint, symbol) = match intermediates.iter()
            .find(|(_, sym)| route_name == &format!("USDC->{}->USDC", sym))
        {
            Some(pair) => pair,
            None => continue,
        };

        // Check if learning says to skip this route
        if !memory.should_scan_route(route_name) {
            skipped += 1;
            continue;
        }

        scanned += 1;

        match get_best_jupiter_quote(base_mint, mint, amount).await {
            Ok(result) => {
                memory.scan_quotes_ok += result.quotes_ok;
                memory.record_scan_spread(result.spread_pct, route_name);
                memory.learn_route_success(route_name, result.spread_pct);

                if result.profit_usd > config::MIN_PROFIT_USD {
                    utils::log_info(&format!("  {} = +${:.4} ({:+.3}%)", route_name, result.profit_usd, result.spread_pct));
                    match &best {
                        Some((b, _)) if result.profit_usd <= *b => {}
                        _ => { best = Some((result.profit_usd, route_name.clone())); }
                    }
                } else {
                    utils::log_info(&format!("  {} = ${:.4} ({:+.3}%)", route_name, result.profit_usd, result.spread_pct));
                }
            }
            Err(e) => {
                memory.scan_quotes_failed += 1;
                memory.learn_route_failure(route_name);
                utils::log_error(&format!("  {} FAIL (backoff activado): {}", route_name, e));
            }
        }

        // Use adaptive delay learned from experience
        tokio::time::sleep(tokio::time::Duration::from_millis(api_delay)).await;
    }

    if skipped > 0 {
        utils::log_info(&format!("  [{} rutas escaneadas, {} en backoff por fallos previos]", scanned, skipped));
    }

    best
}

// =============================================================================
// STRATEGY: TRIANGULAR ARBITRAGE (A -> B -> C -> A)
// =============================================================================

/// Triangular arbitrage: USDC -> token1 -> token2 -> USDC
/// Tries multiple 3-hop paths to find price inefficiencies.
pub async fn scan_triangular_arbitrage(amount: u64, memory: &mut AgentMemory) -> Option<(f64, String)> {
    let slippage = SlippageConfig::default();

    // Promising triangular routes
    let triangles: Vec<(&str, &str, &str, &str)> = vec![
        (SOL_MINT, MSOL_MINT, "SOL", "mSOL"),
        (SOL_MINT, JITOSOL_MINT, "SOL", "jitoSOL"),
        (SOL_MINT, BSOL_MINT, "SOL", "bSOL"),
        (MSOL_MINT, JITOSOL_MINT, "mSOL", "jitoSOL"),
        (SOL_MINT, BONK_MINT, "SOL", "BONK"),
        (SOL_MINT, WIF_MINT, "SOL", "WIF"),
        (SOL_MINT, JUP_MINT, "SOL", "JUP"),
    ];

    let mut best: Option<(f64, String)> = None;
    let flash_fee = amount * 9 / 10_000;

    let api_delay = memory.get_api_delay_ms();

    for (mid1, mid2, sym1, sym2) in &triangles {
        let route = format!("USDC->{}->{}->USDC", sym1, sym2);

        // Skip if learning says this route is in backoff
        if !memory.should_scan_route(&route) { continue; }

        // Leg 1: USDC -> mid1
        let q1 = match get_jupiter_quote_with_slippage(USDC_MINT, mid1, amount, &slippage).await {
            Ok(q) => { memory.record_quote_ok(); q }
            Err(_) => { memory.record_quote_failed(); memory.learn_route_failure(&route); continue; }
        };
        let amt1 = q1.out_amount.parse::<u64>().unwrap_or(0);
        if amt1 == 0 { continue; }

        tokio::time::sleep(tokio::time::Duration::from_millis(api_delay)).await;

        // Leg 2: mid1 -> mid2
        let q2 = match get_jupiter_quote_with_slippage(mid1, mid2, amt1, &slippage).await {
            Ok(q) => { memory.record_quote_ok(); q }
            Err(_) => { memory.record_quote_failed(); memory.learn_route_failure(&route); continue; }
        };
        let amt2 = q2.out_amount.parse::<u64>().unwrap_or(0);
        if amt2 == 0 { continue; }

        tokio::time::sleep(tokio::time::Duration::from_millis(api_delay)).await;

        // Leg 3: mid2 -> USDC
        let q3 = match get_jupiter_quote_with_slippage(mid2, USDC_MINT, amt2, &slippage).await {
            Ok(q) => { memory.record_quote_ok(); q }
            Err(_) => { memory.record_quote_failed(); memory.learn_route_failure(&route); continue; }
        };
        let final_amt = q3.out_amount.parse::<u64>().unwrap_or(0);

        let total_cost = amount + flash_fee;
        let profit_raw = final_amt as i64 - total_cost as i64;
        let spread_pct = profit_raw as f64 / amount as f64 * 100.0;
        let profit_usd = profit_raw as f64 / 1_000_000.0;

        memory.record_scan_spread(spread_pct, &route);
        memory.learn_route_success(&route, spread_pct);

        if final_amt > total_cost {
            utils::log_info(&format!("  triangular {} = +${:.4} ({:+.3}%)", route, profit_usd, spread_pct));
            match &best {
                Some((b, _)) if profit_usd <= *b => {}
                _ => { best = Some((profit_usd, route)); }
            }
        } else {
            utils::log_info(&format!("  triangular {} = ${:.4} ({:+.3}%)", route, profit_usd, spread_pct));
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(api_delay)).await;
    }

    best
}

// =============================================================================
// STRATEGY: STABLECOIN DEPEG ARBITRAGE
// =============================================================================

/// Detects USDC/USDT price deviations.
/// When stablecoins depeg even slightly, there's profit in the spread.
pub async fn scan_stablecoin_arb(amount: u64, memory: &mut AgentMemory) -> Option<(f64, String)> {
    let slippage = SlippageConfig::default();
    let flash_fee = amount * 9 / 10_000;
    let route = "USDC->USDT->USDC (depeg)";

    if !memory.should_scan_route(route) { return None; }

    let api_delay = memory.get_api_delay_ms();

    // USDC -> USDT -> USDC
    let q1 = match get_jupiter_quote_with_slippage(USDC_MINT, USDT_MINT, amount, &slippage).await {
        Ok(q) => { memory.record_quote_ok(); q }
        Err(_) => { memory.record_quote_failed(); memory.learn_route_failure(route); return None; }
    };
    let usdt_amount = q1.out_amount.parse::<u64>().unwrap_or(0);
    if usdt_amount == 0 { return None; }

    tokio::time::sleep(tokio::time::Duration::from_millis(api_delay)).await;

    let q2 = match get_jupiter_quote_with_slippage(USDT_MINT, USDC_MINT, usdt_amount, &slippage).await {
        Ok(q) => { memory.record_quote_ok(); q }
        Err(_) => { memory.record_quote_failed(); memory.learn_route_failure(route); return None; }
    };
    let final_usdc = q2.out_amount.parse::<u64>().unwrap_or(0);

    let total_cost = amount + flash_fee;
    let profit_raw = final_usdc as i64 - total_cost as i64;
    let spread_pct = profit_raw as f64 / amount as f64 * 100.0;
    let profit_usd = profit_raw as f64 / 1_000_000.0;

    memory.record_scan_spread(spread_pct, route);
    memory.learn_route_success(route, spread_pct);

    if final_usdc > total_cost && profit_usd > 0.01 {
        return Some((profit_usd, route.to_string()));
    }

    None
}

// =============================================================================
// STRATEGY: LST PREMIUM ARBITRAGE
// =============================================================================

/// Liquid Staking Token premium detection.
/// mSOL/jitoSOL/bSOL should trade at ~1:1 with SOL but sometimes have premiums.
/// Route: USDC -> SOL -> LST -> USDC (exploiting LST premium/discount)
pub async fn scan_lst_premium(amount: u64, memory: &mut AgentMemory) -> Option<(f64, String)> {
    let slippage = SlippageConfig::default();
    let flash_fee = amount * 9 / 10_000;

    let lst_tokens: Vec<(&str, &str)> = vec![
        (MSOL_MINT, "mSOL"),
        (JITOSOL_MINT, "jitoSOL"),
        (BSOL_MINT, "bSOL"),
    ];

    let mut best: Option<(f64, String)> = None;

    let api_delay = memory.get_api_delay_ms();

    for (lst_mint, symbol) in &lst_tokens {
        let route = format!("USDC->{}->SOL->USDC (LST)", symbol);

        if !memory.should_scan_route(&route) { continue; }

        // Direction 1: USDC -> LST -> SOL -> USDC
        let q1 = match get_jupiter_quote_with_slippage(USDC_MINT, lst_mint, amount, &slippage).await {
            Ok(q) => { memory.record_quote_ok(); q }
            Err(_) => { memory.record_quote_failed(); memory.learn_route_failure(&route); continue; }
        };
        let lst_amt = q1.out_amount.parse::<u64>().unwrap_or(0);
        if lst_amt == 0 { continue; }

        tokio::time::sleep(tokio::time::Duration::from_millis(api_delay)).await;

        let q2 = match get_jupiter_quote_with_slippage(lst_mint, SOL_MINT, lst_amt, &slippage).await {
            Ok(q) => { memory.record_quote_ok(); q }
            Err(_) => { memory.record_quote_failed(); memory.learn_route_failure(&route); continue; }
        };
        let sol_amt = q2.out_amount.parse::<u64>().unwrap_or(0);
        if sol_amt == 0 { continue; }

        tokio::time::sleep(tokio::time::Duration::from_millis(api_delay)).await;

        let q3 = match get_jupiter_quote_with_slippage(SOL_MINT, USDC_MINT, sol_amt, &slippage).await {
            Ok(q) => { memory.record_quote_ok(); q }
            Err(_) => { memory.record_quote_failed(); memory.learn_route_failure(&route); continue; }
        };
        let final_usdc = q3.out_amount.parse::<u64>().unwrap_or(0);

        let total_cost = amount + flash_fee;
        let profit_raw = final_usdc as i64 - total_cost as i64;
        let spread_pct = profit_raw as f64 / amount as f64 * 100.0;
        let profit_usd = profit_raw as f64 / 1_000_000.0;

        memory.record_scan_spread(spread_pct, &route);
        memory.learn_route_success(&route, spread_pct);

        if final_usdc > total_cost {
            utils::log_info(&format!("  LST {} = +${:.4} ({:+.3}%)", route, profit_usd, spread_pct));
            match &best {
                Some((b, _)) if profit_usd <= *b => {}
                _ => { best = Some((profit_usd, route)); }
            }
        } else {
            utils::log_info(&format!("  LST {} = ${:.4} ({:+.3}%)", route, profit_usd, spread_pct));
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(api_delay)).await;
    }

    best
}

// =============================================================================
// MASTER SCANNER: ALL STRATEGIES
// =============================================================================

/// Runs ALL arbitrage strategies and returns the single best opportunity.
/// Updates scan metrics in agent memory for Telegram reporting.
pub async fn scan_all_strategies(amount: u64, memory: &mut AgentMemory) -> Option<(f64, String)> {
    utils::log_info("Escaneando estrategias...");

    let mut best: Option<(f64, String)> = None;

    // Strategy 1: Simple round-trip (9 pairs)
    if let Some((p, r)) = scan_arbitrage_opportunities(USDC_MINT, amount, memory).await {
        utils::log_success(&format!("  [round-trip] {} = +${:.4}", r, p));
        best = Some((p, r));
    }

    // Strategy 2: Triangular (7 paths)
    if let Some((p, r)) = scan_triangular_arbitrage(amount, memory).await {
        utils::log_success(&format!("  [triangular] {} = +${:.4}", r, p));
        match &best {
            Some((b, _)) if p <= *b => {}
            _ => { best = Some((p, r)); }
        }
    }

    // Strategy 3: Stablecoin depeg
    if let Some((p, r)) = scan_stablecoin_arb(amount, memory).await {
        utils::log_success(&format!("  [stablecoin] {} = +${:.4}", r, p));
        match &best {
            Some((b, _)) if p <= *b => {}
            _ => { best = Some((p, r)); }
        }
    }

    // Strategy 4: LST premium
    if let Some((p, r)) = scan_lst_premium(amount, memory).await {
        utils::log_success(&format!("  [LST] {} = +${:.4}", r, p));
        match &best {
            Some((b, _)) if p <= *b => {}
            _ => { best = Some((p, r)); }
        }
    }

    if let Some((p, ref r)) = best {
        utils::log_success(&format!("MEJOR: {} = +${:.4}", r, p));
    } else {
        let scan_info = memory.scan_summary();
        utils::log_info(&format!("  Sin oportunidades rentables. {}", scan_info));
    }

    best
}

/// Obtiene un quote detallado de Jupiter con slippage configurable
/// 
/// # Arguments
/// * `input_mint` - Mint del token de entrada
/// * `output_mint` - Mint del token de salida
/// * `amount` - Cantidad de tokens de entrada
/// * `slippage_config` - Configuración de slippage
/// 
/// # Returns
/// * `Result<JupiterQuoteResponse, String>` - Quote detallado o error
pub async fn get_jupiter_quote_with_slippage(
    input_mint: &str,
    output_mint: &str,
    amount: u64,
    slippage_config: &SlippageConfig,
) -> Result<JupiterQuoteResponse, String> {
    let client = Client::new();
    
    // Calcular slippage basado en volatilidad (simplificado)
    let slippage_bps = calculate_dynamic_slippage(slippage_config);
    
    let url = format!(
        "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&onlyDirectRoutes=false",
        JUPITER_API_BASE,
        input_mint,
        output_mint,
        amount,
        slippage_bps
    );

    let mut last_error = String::new();
    
    for attempt in 0..MAX_RETRIES {
        match client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<JupiterQuoteResponse>().await {
                        Ok(quote) => {
                            utils::log_info(&format!(
                                "Quote obtenido: {} -> {} (slippage: {} bps)",
                                quote.in_amount, quote.out_amount, quote.slippage_bps
                            ));
                            return Ok(quote);
                        }
                        Err(e) => {
                            last_error = format!("Error parseando respuesta: {}", e);
                        }
                    }
                } else {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    last_error = format!("HTTP {}: {}", status, text);
                }
            }
            Err(e) => {
                last_error = format!("Error de red: {}", e);
            }
        }
        
        if attempt < MAX_RETRIES - 1 {
            let delay = RETRY_DELAY_MS * (attempt + 1) as u64;
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            utils::log_warning(&format!("Reintentando quote (intento {})", attempt + 2));
        }
    }
    
    Err(format!("Falló después de {} intentos: {}", MAX_RETRIES, last_error))
}

/// Ejecuta un swap real en Jupiter
/// 
/// # Arguments
/// * `quote_response` - Respuesta del quote de Jupiter
/// * `keypair` - Keypair para firmar la transacción
/// * `prioritization_fee` - Fee de prioridad opcional (en lamports)
/// 
/// # Returns
/// * `Result<SignedSwapTransaction, String>` - Transacción firmada lista para enviar
pub async fn execute_jupiter_swap(
    quote_response: &JupiterQuoteResponse,
    keypair: &Keypair,
    prioritization_fee: Option<u64>,
) -> Result<SignedSwapTransaction, String> {
    let client = Client::new();
    let user_pubkey = keypair.pubkey().to_string();
    
    let swap_request = JupiterSwapRequest {
        quote_response: quote_response.clone(),
        user_public_key: user_pubkey,
        wrap_and_unwrap_sol: true,
        prioritization_fee_lamports: prioritization_fee,
        compute_unit_price_micro_lamports: None,
        dynamic_slippage: None,
        as_legacy_transaction: Some(false),
        use_shared_accounts: None,
        destination_token_account: None,
    };

    let url = format!("{}/swap", JUPITER_API_BASE);
    let mut last_error = String::new();
    
    for attempt in 0..MAX_RETRIES {
        match client
            .post(&url)
            .json(&swap_request)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<JupiterSwapResponse>().await {
                        Ok(swap_response) => {
                            // Verificar si hay error de simulación
                            if let Some(ref error) = swap_response.simulation_error {
                                return Err(format!(
                                    "Error de simulación: {:?}",
                                    error
                                ));
                            }
                            
                            // Decodificar y firmar la transacción
                            match decode_and_sign_transaction(
                                &swap_response.swap_transaction,
                                keypair,
                            ) {
                                Ok(signed_tx) => {
                                    utils::log_success(&format!(
                                        "Swap firmado (fee: {:?} lamports)",
                                        swap_response.prioritization_fee_lamports
                                    ));
                                    return Ok(SignedSwapTransaction {
                                        transaction: signed_tx,
                                        last_valid_block_height: swap_response.last_valid_block_height,
                                        prioritization_fee_lamports: swap_response.prioritization_fee_lamports,
                                    });
                                }
                                Err(e) => {
                                    last_error = format!("Error firmando transacción: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            last_error = format!("Error parseando respuesta swap: {}", e);
                        }
                    }
                } else {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    last_error = format!("HTTP {}: {}", status, text);
                }
            }
            Err(e) => {
                last_error = format!("Error de red en swap: {}", e);
            }
        }
        
        if attempt < MAX_RETRIES - 1 {
            let delay = RETRY_DELAY_MS * (attempt + 1) as u64;
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            utils::log_warning(&format!("Reintentando swap (intento {})", attempt + 2));
        }
    }
    
    Err(format!("Falló swap después de {} intentos: {}", MAX_RETRIES, last_error))
}

/// Construye instrucciones de Jupiter para integrar en bundles Jito
/// 
/// # Arguments
/// * `quote_response` - Respuesta del quote de Jupiter
/// * `keypair` - Keypair del usuario
/// * `prioritization_fee` - Fee de prioridad opcional
/// 
/// # Returns
/// * `Result<JupiterSwapInstructions, String>` - Instrucciones extraídas
pub async fn build_jupiter_instructions(
    quote_response: &JupiterQuoteResponse,
    keypair: &Keypair,
    prioritization_fee: Option<u64>,
) -> Result<JupiterSwapInstructions, String> {
    let client = Client::new();
    let user_pubkey = keypair.pubkey().to_string();
    
    let swap_request = JupiterSwapRequest {
        quote_response: quote_response.clone(),
        user_public_key: user_pubkey,
        wrap_and_unwrap_sol: true,
        prioritization_fee_lamports: prioritization_fee,
        compute_unit_price_micro_lamports: None,
        dynamic_slippage: None,
        as_legacy_transaction: Some(false),
        use_shared_accounts: None,
        destination_token_account: None,
    };

    let url = format!("{}/swap", JUPITER_API_BASE);
    
    for attempt in 0..MAX_RETRIES {
        match client
            .post(&url)
            .json(&swap_request)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<JupiterSwapResponse>().await {
                        Ok(swap_response) => {
                            if let Some(ref error) = swap_response.simulation_error {
                                return Err(format!("Error de simulación: {:?}", error));
                            }
                            
                            // Decodificar la transacción
                            let tx_bytes = {
                                use base64::{Engine as _, engine::general_purpose};
                                general_purpose::STANDARD.decode(&swap_response.swap_transaction)
                                    .map_err(|e| format!("Error decodificando base64: {}", e))?
                            };
                            
                            let versioned_tx: VersionedTransaction = 
                                bincode::deserialize(&tx_bytes)
                                    .map_err(|e| format!("Error deserializando transacción: {}", e))?;
                            
                            // Extraer instrucciones
                            let instructions = extract_instructions_from_versioned_tx(&versioned_tx)?;
                            
                            utils::log_success(&format!(
                                "Extraídas {} instrucciones de Jupiter para bundle",
                                instructions.len()
                            ));
                            
                            return Ok(JupiterSwapInstructions {
                                instructions,
                                account_keys: versioned_tx.message.static_account_keys().to_vec(),
                                blockhash: *versioned_tx.message.recent_blockhash(),
                                last_valid_block_height: swap_response.last_valid_block_height,
                            });
                        }
                        Err(e) => {
                            utils::log_error(&format!("Error parseando swap: {}", e));
                        }
                    }
                } else {
                    utils::log_error(&format!("HTTP error: {}", resp.status()));
                }
            }
            Err(e) => {
                utils::log_error(&format!("Error de red: {}", e));
            }
        }
        
        if attempt < MAX_RETRIES - 1 {
            let delay = RETRY_DELAY_MS * (attempt + 1) as u64;
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
        }
    }
    
    Err("Falló al construir instrucciones después de múltiples intentos".to_string())
}

/// Ejecuta un swap completo: obtiene quote y ejecuta
/// 
/// # Arguments
/// * `input_mint` - Mint del token de entrada
/// * `output_mint` - Mint del token de salida  
/// * `amount` - Cantidad de entrada
/// * `keypair` - Keypair para firmar
/// * `prioritization_fee` - Fee de prioridad opcional
/// 
/// # Returns
/// * `Result<SignedSwapTransaction, String>` - Transacción firmada
pub async fn swap_tokens(
    input_mint: &str,
    output_mint: &str,
    amount: u64,
    keypair: &Keypair,
    prioritization_fee: Option<u64>,
) -> Result<SignedSwapTransaction, String> {
    // 1. Obtener quote
    let slippage_config = SlippageConfig::default();
    let quote = get_jupiter_quote_with_slippage(
        input_mint,
        output_mint,
        amount,
        &slippage_config,
    ).await?;
    
    // 2. Ejecutar swap
    execute_jupiter_swap(&quote, keypair, prioritization_fee).await
}

/// Calcula slippage dinámico basado en condiciones de mercado
/// 
/// # Arguments
/// * `config` - Configuración de slippage
/// 
/// # Returns
/// * `u16` - Slippage en bps
pub fn calculate_dynamic_slippage(config: &SlippageConfig) -> u16 {
    // Implementación simplificada - en producción se podría:
    // - Consultar volatilidad reciente del par
    // - Ajustar basado en liquidez disponible
    // - Considerar congestión de red
    
    let base = config.base_bps as f64;
    let adjusted = base * config.volatility_multiplier;
    
    (adjusted as u16).min(config.max_bps)
}

/// Obtiene múltiples quotes para comparar rutas
/// 
/// # Arguments
/// * `input_mint` - Mint de entrada
/// * `output_mint` - Mint de salida
/// * `amount` - Cantidad
/// 
/// # Returns
/// * `Result<Vec<JupiterQuoteResponse>, String>` - Lista de quotes
pub async fn get_multiple_quotes(
    input_mint: &str,
    output_mint: &str,
    amount: u64,
) -> Result<Vec<JupiterQuoteResponse>, String> {
    let client = Client::new();
    let url = format!(
        "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps=50&onlyDirectRoutes=false",
        JUPITER_API_BASE, input_mint, output_mint, amount
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Error de red: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    // Jupiter devuelve un solo quote, pero podríamos hacer múltiples requests
    // con diferentes parámetros para comparar
    let quote = resp
        .json::<JupiterQuoteResponse>()
        .await
        .map_err(|e| format!("Error parseando: {}", e))?;

    Ok(vec![quote])
}

// =============================================================================
// FUNCIONES AUXILIARES
// =============================================================================

/// Decodifica una transacción base64 y la firma
fn decode_and_sign_transaction(
    base64_tx: &str,
    keypair: &Keypair,
) -> Result<VersionedTransaction, String> {
    use base64::{Engine as _, engine::general_purpose};
    // Decodificar base64
    let tx_bytes = general_purpose::STANDARD.decode(base64_tx)
        .map_err(|e| format!("Error decodificando base64: {}", e))?;
    
    // Deserializar transacción
    let mut versioned_tx: VersionedTransaction = bincode::deserialize(&tx_bytes)
        .map_err(|e| format!("Error deserializando transacción: {}", e))?;
    
    // Firmar la transacción - necesitamos firmar los message bytes
    let message_bytes = versioned_tx.message.serialize();
    let signature = keypair.sign_message(&message_bytes);
    
    // Reemplazar la firma (la primera debe ser la del payer/fee payer)
    if !versioned_tx.signatures.is_empty() {
        versioned_tx.signatures[0] = signature;
    } else {
        versioned_tx.signatures.push(signature);
    }
    
    Ok(versioned_tx)
}

/// Extrae instrucciones de una transacción versionada
fn extract_instructions_from_versioned_tx(
    tx: &VersionedTransaction,
) -> Result<Vec<Instruction>, String> {
    let mut instructions = Vec::new();
    
    let account_keys = tx.message.static_account_keys();
    
    // Procesar instrucciones compiladas
    for compiled_ix in tx.message.instructions() {
        let program_id = *account_keys
            .get(compiled_ix.program_id_index as usize)
            .ok_or("Índice de programa inválido")?;
        
        let accounts: Vec<AccountMeta> = compiled_ix
            .accounts
            .iter()
            .filter_map(|&idx| {
                account_keys.get(idx as usize).map(|&pubkey| {
                    let is_signer = tx.message.is_signer(idx as usize);
                    let is_writable = tx.message.is_maybe_writable(idx as usize, None);
                    
                    if is_writable {
                        AccountMeta::new(pubkey, is_signer)
                    } else {
                        AccountMeta::new_readonly(pubkey, is_signer)
                    }
                })
            })
            .collect();
        
        instructions.push(Instruction {
            program_id,
            accounts,
            data: compiled_ix.data.clone(),
        });
    }
    
    Ok(instructions)
}

/// Valida que un quote tenga profit mínimo
/// 
/// # Arguments
/// * `quote` - Quote de Jupiter
/// * `min_profit_usd` - Profit mínimo requerido
/// 
/// # Returns
/// * `bool` - True si el quote es rentable
pub fn is_quote_profitable(quote: &JupiterQuoteResponse, min_profit_usd: f64) -> bool {
    let in_amount = quote.in_amount.parse::<u64>().unwrap_or(0) as f64 / 1_000_000.0;
    let out_amount = quote.out_amount.parse::<u64>().unwrap_or(0) as f64 / 1_000_000.0;
    
    let profit = out_amount - in_amount;
    profit > min_profit_usd
}

/// Obtiene el price impact de un quote
/// 
/// # Arguments
/// * `quote` - Quote de Jupiter
/// 
/// # Returns
/// * `f64` - Price impact como porcentaje
pub fn get_price_impact(quote: &JupiterQuoteResponse) -> f64 {
    quote.price_impact_pct.parse::<f64>().unwrap_or(0.0)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slippage_calculation() {
        let config = SlippageConfig {
            base_bps: 50,
            max_bps: 500,
            volatility_multiplier: 2.0,
        };
        
        let slippage = calculate_dynamic_slippage(&config);
        assert_eq!(slippage, 100); // 50 * 2.0 = 100
    }

    #[test]
    fn test_slippage_max_cap() {
        let config = SlippageConfig {
            base_bps: 400,
            max_bps: 500,
            volatility_multiplier: 2.0,
        };
        
        let slippage = calculate_dynamic_slippage(&config);
        assert_eq!(slippage, 500); // Capped at max
    }

    #[test]
    fn test_base64_decode() {
        use base64::{Engine as _, engine::general_purpose};
        let encoded = "SGVsbG8gV29ybGQ="; // "Hello World"
        let decoded = general_purpose::STANDARD.decode(encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "Hello World");
    }

    #[test]
    fn test_is_quote_profitable() {
        let quote = JupiterQuoteResponse {
            input_mint: "mint1".to_string(),
            output_mint: "mint2".to_string(),
            in_amount: "1000000".to_string(),
            out_amount: "2000000".to_string(),
            other_amount_threshold: "1900000".to_string(),
            slippage_bps: 50,
            swap_mode: "ExactIn".to_string(),
            platform_fee: None,
            price_impact_pct: "0.1".to_string(),
            route_plan: vec![],
            context_slot: None,
            time_taken: None,
        };
        
        assert!(is_quote_profitable(&quote, 0.5));
        assert!(!is_quote_profitable(&quote, 2.0));
    }
}
