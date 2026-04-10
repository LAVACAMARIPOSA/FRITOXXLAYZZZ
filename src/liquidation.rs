use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
};
use borsh::BorshDeserialize;
use std::str::FromStr;

use crate::utils;

/// Program ID de Kamino Lending
pub const KAMINO_PROGRAM_ID: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";

/// Main Lending Market de Kamino
pub const KAMINO_MAIN_MARKET: &str = "7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF";

/// Discriminador para cuenta Obligation (sha256("account:Obligation")[:8])
pub const OBLIGATION_DISCRIMINATOR: [u8; 8] = [0xa8, 0xce, 0x8d, 0x6a, 0x58, 0x4c, 0xac, 0xa7];

/// Discriminador para cuenta Reserve (sha256("account:Reserve")[:8])
pub const RESERVE_DISCRIMINATOR: [u8; 8] = [0x2b, 0xf2, 0xcc, 0xca, 0x1a, 0xf7, 0x3b, 0x7f];

/// Tamaño fijo de una cuenta Obligation
pub const OBLIGATION_SIZE: usize = 1784;

/// Factor de escala para valores en SF (Scaled Fraction)
pub const SCALE_FACTOR: u128 = 1_000_000_000_000_000_000; // 10^18

/// Representa una oportunidad de liquidación identificada
#[derive(Debug, Clone)]
pub struct LiquidationOpportunity {
    /// Dirección de la cuenta Obligation
    pub obligation_pubkey: Pubkey,
    /// Dueño de la posición
    pub owner: Pubkey,
    /// Factor de salud (HF < 1.0 significa liquidable)
    pub health_factor: f64,
    /// Valor total depositado en USD
    pub deposited_value_usd: f64,
    /// Valor total prestado en USD
    pub borrowed_value_usd: f64,
    /// Valor ajustado por borrow factor (deuda efectiva)
    pub borrow_factor_adjusted_debt_usd: f64,
    /// Valor umbral de préstamo no saludable (liquidación)
    pub unhealthy_borrow_value_usd: f64,
    /// LTV actual (Loan-to-Value)
    pub current_ltv: f64,
    /// Lista de depósitos (collateral)
    pub deposits: Vec<CollateralDeposit>,
    /// Lista de préstamos (borrows)
    pub borrows: Vec<BorrowPosition>,
    /// Potencial ganancia estimada para el liquidador (bonus de liquidación)
    pub estimated_profit_usd: f64,
}

/// Representa un depósito de collateral en una obligación
#[derive(Debug, Clone)]
pub struct CollateralDeposit {
    /// Dirección del reserve donde está depositado
    pub reserve: Pubkey,
    /// Cantidad depositada (en tokens nativos)
    pub deposited_amount: u64,
    /// Valor de mercado en USD
    pub market_value_usd: f64,
    /// Símbolo del token (si está disponible)
    pub token_symbol: String,
    /// Decimales del token
    pub decimals: u8,
}

/// Representa una posición de préstamo en una obligación
#[derive(Debug, Clone)]
pub struct BorrowPosition {
    /// Dirección del reserve del que se prestó
    pub reserve: Pubkey,
    /// Cantidad prestada incluyendo intereses (en tokens nativos escalados)
    pub borrowed_amount_sf: u128,
    /// Valor de mercado en USD
    pub market_value_usd: f64,
    /// Símbolo del token (si está disponible)
    pub token_symbol: String,
    /// Decimales del token
    pub decimals: u8,
}

/// Estructura LastUpdate de Kamino
#[derive(Debug, Clone, Copy, Default, PartialEq, BorshDeserialize)]
pub struct LastUpdate {
    pub slot: u64,
    pub stale: u8,
    pub price_status: u8,
}

/// Estructura ObligationCollateral de Kamino
#[derive(Debug, Clone, Copy, Default, PartialEq, BorshDeserialize)]
pub struct ObligationCollateral {
    pub deposit_reserve: Pubkey,
    pub deposited_amount: u64,
    pub market_value_sf: u128,
    pub borrowed_amount_against_this_collateral_in_elevation_group: u64,
    pub _padding: [u64; 3],
}

/// Estructura BigFractionBytes para campos BSF
#[derive(Debug, Clone, Copy, Default, PartialEq, BorshDeserialize)]
pub struct BigFractionBytes {
    pub value: u128,
    pub padding: u64,
}

/// Estructura ObligationLiquidity de Kamino
#[derive(Debug, Clone, Copy, Default, PartialEq, BorshDeserialize)]
pub struct ObligationLiquidity {
    pub borrow_reserve: Pubkey,
    pub cumulative_borrow_rate_bsf: BigFractionBytes,
    pub first_borrowed_at_timestamp: u64,
    pub borrowed_amount_sf: u128,
    pub market_value_sf: u128,
    pub borrow_factor_adjusted_market_value_sf: u128,
    pub borrowed_amount_outside_elevation_groups: u64,
    pub _padding: [u64; 3],
}

/// Estructura ObligationOrder de Kamino
#[derive(Debug, Clone, Copy, Default, PartialEq, BorshDeserialize)]
pub struct ObligationOrder {
    pub threshold_value: u128,
    pub opportunity: u128,
    pub min_execution_bonus_bps: u16,
    pub max_execution_bonus_bps: u16,
    pub condition: u8,
    pub opportunity_type: u8,
    pub _padding: [u8; 4],
}

/// Estructura BorrowOrder de Kamino
#[derive(Debug, Clone, Copy, Default, PartialEq, BorshDeserialize)]
pub struct BorrowOrder {
    pub borrow_mint: Pubkey,
    pub remaining_amount: u64,
    pub destination_account: Pubkey,
    pub min_debt_term_seconds: u64,
    pub valid_until_timestamp: u64,
    pub placed_at_timestamp: u64,
    pub last_updated_at_timestamp: u64,
    pub original_borrow_amount: u64,
    pub max_borrow_rate_bps: u16,
    pub is_active: u8,
    pub _padding: [u8; 5],
}

/// Estructura principal Obligation de Kamino Lending
/// Basada en: https://docs.kamino.finance/klend/reference/accounts/obligation
#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct Obligation {
    pub tag: u64,
    pub last_update: LastUpdate,
    pub lending_market: Pubkey,
    pub owner: Pubkey,
    
    // Depósitos (hasta 8)
    pub deposits: [ObligationCollateral; 8],
    pub deposited_value_sf: u128,
    pub lowest_reserve_deposit_liquidation_ltv: u64,
    pub lowest_reserve_deposit_max_ltv_pct: u8,
    pub _padding0: [u8; 7],
    
    // Préstamos (hasta 5)
    pub borrows: [ObligationLiquidity; 5],
    pub borrowed_assets_market_value_sf: u128,
    pub borrow_factor_adjusted_debt_value_sf: u128,
    pub allowed_borrow_value_sf: u128,
    pub unhealthy_borrow_value_sf: u128,
    
    // Gestión de riesgo
    pub elevation_group: u8,
    pub highest_borrow_factor_pct: u64,
    pub has_debt: u8,
    pub borrowing_disabled: u8,
    pub _padding1: [u8; 6],
    
    // Campos adicionales
    pub referrer: Pubkey,
    pub obligation_orders: [ObligationOrder; 2],
    pub borrow_order: BorrowOrder,
    pub autodeleverage_target_ltv_pct: u8,
    pub num_of_obsolete_deposit_reserves: u8,
    pub num_of_obsolete_borrow_reserves: u8,
    pub _padding2: [u8; 5],
    pub autodeleverage_margin_call_started_timestamp: u64,
}

impl Obligation {
    /// Verifica si esta obligación tiene deuda
    pub fn has_debt(&self) -> bool {
        self.has_debt != 0
    }
    
    /// Calcula el factor de salud (Health Factor)
    /// HF = unhealthy_borrow_value / borrow_factor_adjusted_debt_value
    /// Si HF < 1.0, la posición es liquidable
    pub fn health_factor(&self) -> f64 {
        if self.borrow_factor_adjusted_debt_value_sf == 0 {
            return f64::MAX;
        }
        
        let unhealthy = self.unhealthy_borrow_value_sf as f64 / SCALE_FACTOR as f64;
        let debt = self.borrow_factor_adjusted_debt_value_sf as f64 / SCALE_FACTOR as f64;
        
        if debt == 0.0 {
            return f64::MAX;
        }
        
        unhealthy / debt
    }
    
    /// Calcula el LTV actual (Loan-to-Value)
    pub fn loan_to_value(&self) -> f64 {
        if self.deposited_value_sf == 0 {
            return 0.0;
        }
        
        let debt = self.borrow_factor_adjusted_debt_value_sf as f64 / SCALE_FACTOR as f64;
        let deposits = self.deposited_value_sf as f64 / SCALE_FACTOR as f64;
        
        if deposits == 0.0 {
            return 0.0;
        }
        
        debt / deposits
    }
    
    /// Verifica si la posición es liquidable
    pub fn is_liquidatable(&self) -> bool {
        self.health_factor() < 1.0 && self.has_debt()
    }
    
    /// Obtiene el valor depositado en USD
    pub fn deposited_value_usd(&self) -> f64 {
        self.deposited_value_sf as f64 / SCALE_FACTOR as f64
    }
    
    /// Obtiene el valor prestado en USD (sin ajustar por borrow factor)
    pub fn borrowed_value_usd(&self) -> f64 {
        self.borrowed_assets_market_value_sf as f64 / SCALE_FACTOR as f64
    }
    
    /// Obtiene el valor de deuda ajustado por borrow factor
    pub fn borrow_factor_adjusted_debt_usd(&self) -> f64 {
        self.borrow_factor_adjusted_debt_value_sf as f64 / SCALE_FACTOR as f64
    }
    
    /// Obtiene el valor umbral de préstamo no saludable
    pub fn unhealthy_borrow_value_usd(&self) -> f64 {
        self.unhealthy_borrow_value_sf as f64 / SCALE_FACTOR as f64
    }
    
    /// Obtiene los depósitos activos (no vacíos)
    pub fn active_deposits(&self) -> Vec<&ObligationCollateral> {
        self.deposits
            .iter()
            .filter(|d| d.deposited_amount > 0)
            .collect()
    }
    
    /// Obtiene los préstamos activos
    pub fn active_borrows(&self) -> Vec<&ObligationLiquidity> {
        self.borrows
            .iter()
            .filter(|b| b.borrowed_amount_sf > 0)
            .collect()
    }
}

/// Información de un reserve (para cálculos de precios)
#[derive(Debug, Clone)]
pub struct ReserveInfo {
    pub pubkey: Pubkey,
    pub liquidity_mint: Pubkey,
    pub market_price_sf: u128,
    pub decimals: u8,
    pub token_symbol: String,
}

/// Cache de reserves para evitar fetch repetidos
pub struct ReserveCache {
    reserves: std::collections::HashMap<Pubkey, ReserveInfo>,
}

impl ReserveCache {
    pub fn new() -> Self {
        Self {
            reserves: std::collections::HashMap::new(),
        }
    }
    
    pub fn get(&self, pubkey: &Pubkey) -> Option<&ReserveInfo> {
        self.reserves.get(pubkey)
    }
    
    pub fn insert(&mut self, pubkey: Pubkey, info: ReserveInfo) {
        self.reserves.insert(pubkey, info);
    }
}

impl Default for ReserveCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Obtiene todas las cuentas Obligation de Kamino Lending
async fn get_all_obligations(
    client: &RpcClient,
) -> Result<Vec<(Pubkey, Obligation)>, Box<dyn std::error::Error + Send + Sync>> {
    let program_id = Pubkey::from_str(KAMINO_PROGRAM_ID)?;
    
    utils::log_info("Obteniendo cuentas Obligation de Kamino...");
    
    // Configuración del filtro para obtener solo cuentas Obligation
    let config = solana_client::rpc_config::RpcProgramAccountsConfig {
        filters: Some(vec![
            solana_client::rpc_filter::RpcFilterType::DataSize(OBLIGATION_SIZE as u64),
            solana_client::rpc_filter::RpcFilterType::Memcmp(
                solana_client::rpc_filter::Memcmp::new(
                    0,
                    solana_client::rpc_filter::MemcmpEncodedBytes::Bytes(
                        OBLIGATION_DISCRIMINATOR.to_vec()
                    ),
                )
            ),
        ]),
        account_config: solana_client::rpc_config::RpcAccountInfoConfig {
            encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        },
        ..Default::default()
    };
    
    let accounts = client.get_program_accounts_with_config(&program_id, config).await?;
    
    utils::log_info(&format!("Total cuentas encontradas: {}", accounts.len()));
    
    let mut obligations = Vec::new();
    
    for (pubkey, account) in accounts {
        // Verificar discriminador
        if account.data.len() < 8 || &account.data[0..8] != &OBLIGATION_DISCRIMINATOR[..] {
            continue;
        }
        
        // Deserializar la cuenta
        match try_deserialize_obligation(&account.data) {
            Ok(obligation) => {
                obligations.push((pubkey, obligation));
            }
            Err(e) => {
                utils::log_warning(&format!("Error deserializando {}: {}", pubkey, e));
            }
        }
    }
    
    utils::log_success(&format!("Obligaciones deserializadas: {}", obligations.len()));
    
    Ok(obligations)
}

/// Intenta deserializar una cuenta Obligation
fn try_deserialize_obligation(data: &[u8]) -> Result<Obligation, Box<dyn std::error::Error + Send + Sync>> {
    if data.len() < OBLIGATION_SIZE {
        return Err("Datos de cuenta insuficientes".into());
    }

    // Skip 8-byte discriminator, then deserialize with borsh
    let data_after_discriminator = &data[8..];
    let obligation = Obligation::try_from_slice(data_after_discriminator)?;

    Ok(obligation)
}

/// Filtra posiciones liquidables en el rango de valor especificado
fn filter_liquidatable_positions(
    obligations: Vec<(Pubkey, Obligation)>,
    min_value_usd: f64,
    max_value_usd: f64,
) -> Vec<(Pubkey, Obligation)> {
    obligations
        .into_iter()
        .filter(|(_, obl)| {
            // Debe tener deuda
            if !obl.has_debt() {
                return false;
            }
            
            let deposited = obl.deposited_value_usd();
            let health_factor = obl.health_factor();
            
            // Debe estar en el rango de valor
            if deposited < min_value_usd || deposited > max_value_usd {
                return false;
            }
            
            // Debe ser liquidable (HF < 1.0)
            health_factor < 1.0
        })
        .collect()
}

/// Calcula la ganancia estimada de liquidación
fn calculate_estimated_profit(
    obligation: &Obligation,
    _reserve_cache: &ReserveCache,
) -> f64 {
    // El bonus de liquidación típico en Kamino es 5-20%
    // Usamos un estimado conservador del 7%
    const LIQUIDATION_BONUS_ESTIMATE: f64 = 0.07;
    
    let debt_value = obligation.borrow_factor_adjusted_debt_usd();
    
    // La ganancia es el bonus sobre la deuda liquidada
    // Normalmente se liquida hasta un 50% de la deuda (close factor)
    let max_liquidation_amount = debt_value * 0.5;
    let estimated_profit = max_liquidation_amount * LIQUIDATION_BONUS_ESTIMATE;
    
    estimated_profit
}

/// Convierte una Obligation en una LiquidationOpportunity
fn obligation_to_opportunity(
    pubkey: Pubkey,
    obligation: &Obligation,
    reserve_cache: &ReserveCache,
) -> LiquidationOpportunity {
    let deposits: Vec<CollateralDeposit> = obligation
        .active_deposits()
        .iter()
        .map(|d| {
            let reserve_info = reserve_cache.get(&d.deposit_reserve);
            CollateralDeposit {
                reserve: d.deposit_reserve,
                deposited_amount: d.deposited_amount,
                market_value_usd: d.market_value_sf as f64 / SCALE_FACTOR as f64,
                token_symbol: reserve_info
                    .map(|r| r.token_symbol.clone())
                    .unwrap_or_else(|| "UNKNOWN".to_string()),
                decimals: reserve_info.map(|r| r.decimals).unwrap_or(6),
            }
        })
        .collect();
    
    let borrows: Vec<BorrowPosition> = obligation
        .active_borrows()
        .iter()
        .map(|b| {
            let reserve_info = reserve_cache.get(&b.borrow_reserve);
            BorrowPosition {
                reserve: b.borrow_reserve,
                borrowed_amount_sf: b.borrowed_amount_sf,
                market_value_usd: b.market_value_sf as f64 / SCALE_FACTOR as f64,
                token_symbol: reserve_info
                    .map(|r| r.token_symbol.clone())
                    .unwrap_or_else(|| "UNKNOWN".to_string()),
                decimals: reserve_info.map(|r| r.decimals).unwrap_or(6),
            }
        })
        .collect();
    
    let estimated_profit = calculate_estimated_profit(obligation, reserve_cache);
    
    LiquidationOpportunity {
        obligation_pubkey: pubkey,
        owner: obligation.owner,
        health_factor: obligation.health_factor(),
        deposited_value_usd: obligation.deposited_value_usd(),
        borrowed_value_usd: obligation.borrowed_value_usd(),
        borrow_factor_adjusted_debt_usd: obligation.borrow_factor_adjusted_debt_usd(),
        unhealthy_borrow_value_usd: obligation.unhealthy_borrow_value_usd(),
        current_ltv: obligation.loan_to_value(),
        deposits,
        borrows,
        estimated_profit_usd: estimated_profit,
    }
}

/// Obtiene información de reserves necesarios para las obligaciones
async fn fetch_reserve_info(
    _client: &RpcClient,
    obligations: &[(Pubkey, Obligation)],
) -> Result<ReserveCache, Box<dyn std::error::Error + Send + Sync>> {
    let mut cache = ReserveCache::new();
    let mut reserve_pubkeys: std::collections::HashSet<Pubkey> = std::collections::HashSet::new();
    
    // Recolectar todos los reserves únicos
    for (_, obl) in obligations {
        for deposit in obl.active_deposits() {
            reserve_pubkeys.insert(deposit.deposit_reserve);
        }
        for borrow in obl.active_borrows() {
            reserve_pubkeys.insert(borrow.borrow_reserve);
        }
    }
    
    utils::log_info(&format!("Fetching info for {} unique reserves...", reserve_pubkeys.len()));
    
    // Por ahora, usamos información básica
    // En una implementación completa, se fetchearían las cuentas Reserve
    // y se extraería la información de precios y tokens
    for pubkey in reserve_pubkeys {
        // Información por defecto - en producción se fetchea de la cadena
        cache.insert(
            pubkey,
            ReserveInfo {
                pubkey,
                liquidity_mint: Pubkey::default(),
                market_price_sf: 0,
                decimals: 6,
                token_symbol: "UNKNOWN".to_string(),
            },
        );
    }
    
    Ok(cache)
}

/// Result of a liquidation scan with metrics
pub struct LiquidationScanResult {
    pub opportunities: Vec<LiquidationOpportunity>,
    pub total_obligations_fetched: usize,
    pub total_with_debt: usize,
    pub total_in_range: usize,
    pub scan_error: Option<String>,
}

/// Escanea liquidaciones pequeñas ($10-$500) en Kamino Lending
pub async fn scan_small_liquidations(client: &RpcClient) -> LiquidationScanResult {
    utils::log_info("Escaneando posiciones underwater $10-$500 en Kamino...");

    let min_value = 10.0;
    let max_value = 500.0;

    // Obtener todas las obligaciones
    let obligations = match get_all_obligations(client).await {
        Ok(obs) => obs,
        Err(e) => {
            let err_msg = format!("Error obteniendo obligaciones: {}", e);
            utils::log_error(&err_msg);
            return LiquidationScanResult {
                opportunities: Vec::new(),
                total_obligations_fetched: 0,
                total_with_debt: 0,
                total_in_range: 0,
                scan_error: Some(err_msg),
            };
        }
    };

    let total_fetched = obligations.len();
    if obligations.is_empty() {
        utils::log_warning("No se encontraron obligaciones");
        return LiquidationScanResult {
            opportunities: Vec::new(),
            total_obligations_fetched: 0,
            total_with_debt: 0,
            total_in_range: 0,
            scan_error: None,
        };
    }

    // Count obligations with debt for metrics
    let total_with_debt = obligations.iter().filter(|(_, o)| o.has_debt()).count();

    // Count those in our value range (even if not liquidatable)
    let total_in_range = obligations.iter().filter(|(_, o)| {
        let dep = o.deposited_value_usd();
        o.has_debt() && dep >= min_value && dep <= max_value
    }).count();

    // Filtrar posiciones liquidables en el rango
    let liquidatable = filter_liquidatable_positions(obligations, min_value, max_value);

    utils::log_info(&format!(
        "Obligaciones: {} total, {} con deuda, {} en rango ${}-${}, {} liquidables",
        total_fetched, total_with_debt, total_in_range,
        min_value, max_value, liquidatable.len()
    ));

    if liquidatable.is_empty() {
        return LiquidationScanResult {
            opportunities: Vec::new(),
            total_obligations_fetched: total_fetched,
            total_with_debt,
            total_in_range,
            scan_error: None,
        };
    }
    
    // Obtener información de reserves
    let reserve_cache = match fetch_reserve_info(client, &liquidatable).await {
        Ok(cache) => cache,
        Err(e) => {
            utils::log_error(&format!("Error obteniendo info de reserves: {}", e));
            ReserveCache::new()
        }
    };
    
    // Convertir a oportunidades
    let opportunities: Vec<LiquidationOpportunity> = liquidatable
        .iter()
        .map(|(pubkey, obl)| obligation_to_opportunity(*pubkey, obl, &reserve_cache))
        .collect();
    
    // Mostrar resultados
    for opp in &opportunities {
        println!("   🎯 Oportunidad encontrada:");
        println!("      📍 Obligation: {}", opp.obligation_pubkey);
        println!("      👤 Owner: {}", opp.owner);
        println!("      💰 Depósitos: ${:.2}", opp.deposited_value_usd);
        println!("      💸 Deuda: ${:.2}", opp.borrow_factor_adjusted_debt_usd);
        println!("      📊 Health Factor: {:.4}", opp.health_factor);
        println!("      📈 LTV: {:.2}%", opp.current_ltv * 100.0);
        println!("      💵 Ganancia estimada: ${:.2}", opp.estimated_profit_usd);
        println!();
    }
    
    utils::log_success(&format!(
        "Escaneo completado. {} oportunidades de liquidacion encontradas",
        opportunities.len()
    ));

    LiquidationScanResult {
        opportunities,
        total_obligations_fetched: total_fetched,
        total_with_debt,
        total_in_range,
        scan_error: None,
    }
}

/// Obtiene las obligaciones de un usuario específico
pub async fn get_user_obligations(
    client: &RpcClient,
    owner: &Pubkey,
) -> Result<Vec<(Pubkey, Obligation)>, Box<dyn std::error::Error + Send + Sync>> {
    let all_obligations = get_all_obligations(client).await?;
    
    let user_obligations: Vec<(Pubkey, Obligation)> = all_obligations
        .into_iter()
        .filter(|(_, obl)| &obl.owner == owner)
        .collect();
    
    Ok(user_obligations)
}

/// Verifica si una obligación específica es liquidable
pub fn is_obligation_liquidatable(obligation: &Obligation) -> bool {
    obligation.is_liquidatable()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_health_factor_calculation() {
        // Crear una obligación de prueba
        let mut obligation = Obligation {
            tag: 0,
            last_update: LastUpdate::default(),
            lending_market: Pubkey::default(),
            owner: Pubkey::default(),
            deposits: [ObligationCollateral::default(); 8],
            deposited_value_sf: 1000 * SCALE_FACTOR, // $1000 depositado
            lowest_reserve_deposit_liquidation_ltv: 0,
            lowest_reserve_deposit_max_ltv_pct: 0,
            _padding0: [0; 7],
            borrows: [ObligationLiquidity::default(); 5],
            borrowed_assets_market_value_sf: 0,
            borrow_factor_adjusted_debt_value_sf: 800 * SCALE_FACTOR, // $800 deuda
            allowed_borrow_value_sf: 750 * SCALE_FACTOR, // $750 permitido
            unhealthy_borrow_value_sf: 850 * SCALE_FACTOR, // $850 umbral
            elevation_group: 0,
            highest_borrow_factor_pct: 100,
            has_debt: 1,
            borrowing_disabled: 0,
            _padding1: [0; 6],
            referrer: Pubkey::default(),
            obligation_orders: [ObligationOrder::default(); 2],
            borrow_order: BorrowOrder::default(),
            autodeleverage_target_ltv_pct: 0,
            num_of_obsolete_deposit_reserves: 0,
            num_of_obsolete_borrow_reserves: 0,
            _padding2: [0; 5],
            autodeleverage_margin_call_started_timestamp: 0,
        };
        
        // HF = 850 / 800 = 1.0625 > 1.0 (saludable)
        assert!(!obligation.is_liquidatable());
        
        // Ahora hacerla no saludable
        obligation.borrow_factor_adjusted_debt_value_sf = 900 * SCALE_FACTOR; // $900 deuda
        // HF = 850 / 900 = 0.944 < 1.0 (liquidable)
        assert!(obligation.is_liquidatable());
    }
    
    #[test]
    fn test_ltv_calculation() {
        let obligation = Obligation {
            tag: 0,
            last_update: LastUpdate::default(),
            lending_market: Pubkey::default(),
            owner: Pubkey::default(),
            deposits: [ObligationCollateral::default(); 8],
            deposited_value_sf: 1000 * SCALE_FACTOR, // $1000
            lowest_reserve_deposit_liquidation_ltv: 0,
            lowest_reserve_deposit_max_ltv_pct: 0,
            _padding0: [0; 7],
            borrows: [ObligationLiquidity::default(); 5],
            borrowed_assets_market_value_sf: 0,
            borrow_factor_adjusted_debt_value_sf: 500 * SCALE_FACTOR, // $500
            allowed_borrow_value_sf: 0,
            unhealthy_borrow_value_sf: 0,
            elevation_group: 0,
            highest_borrow_factor_pct: 0,
            has_debt: 1,
            borrowing_disabled: 0,
            _padding1: [0; 6],
            referrer: Pubkey::default(),
            obligation_orders: [ObligationOrder::default(); 2],
            borrow_order: BorrowOrder::default(),
            autodeleverage_target_ltv_pct: 0,
            num_of_obsolete_deposit_reserves: 0,
            num_of_obsolete_borrow_reserves: 0,
            _padding2: [0; 5],
            autodeleverage_margin_call_started_timestamp: 0,
        };
        
        // LTV = 500 / 1000 = 0.5 = 50%
        assert!((obligation.loan_to_value() - 0.5).abs() < 0.001);
    }
}
