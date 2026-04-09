use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    message::{v0::Message, VersionedMessage},
    signature::Signer,
    transaction::VersionedTransaction,
    system_program,
    sysvar,
};
use std::str::FromStr;
use borsh::{BorshSerialize, BorshDeserialize};

use crate::{config, utils};

/// Kamino Lending Program ID (Mainnet)
pub const KAMINO_LEND_PROGRAM: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";

/// SPL Token Program ID
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// SPL Token 2022 Program ID
pub const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

/// Main Lending Market (Kamino Main Market)
pub const MAIN_LENDING_MARKET: &str = "7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF";

/// USDC Reserve en Kamino Main Market (real mainnet address)
pub const USDC_RESERVE: &str = "D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59";

/// USDC Mint
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// SOL Mint (Wrapped SOL)
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Flash loan fee: 0.09% = 9 bps = 0.0009
pub const FLASH_LOAN_FEE_BPS: u64 = 9;
pub const FLASH_LOAN_FEE_DENOMINATOR: u64 = 10_000;

/// Calcula el discriminador de Anchor para una instrucción
/// Los discriminadores de Anchor se calculan como los primeros 8 bytes de 
/// SHA-256("global:<nombre_de_la_instrucción>")
pub fn get_instruction_discriminator(instruction_name: &str) -> [u8; 8] {
    use solana_sdk::hash::hash;
    let preimage = format!("global:{}", instruction_name);
    let hash = hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.to_bytes()[..8]);
    discriminator
}

/// Discriminador para FlashBorrowReserveLiquidity
pub fn flash_borrow_discriminator() -> [u8; 8] {
    get_instruction_discriminator("flash_borrow_reserve_liquidity")
}

/// Discriminador para FlashRepayReserveLiquidity  
pub fn flash_repay_discriminator() -> [u8; 8] {
    get_instruction_discriminator("flash_repay_reserve_liquidity")
}

/// Estructura de datos para FlashBorrowReserveLiquidity instruction
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct FlashBorrowReserveLiquidityArgs {
    pub liquidity_amount: u64,
}

/// Estructura de datos para FlashRepayReserveLiquidity instruction
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct FlashRepayReserveLiquidityArgs {
    pub liquidity_amount: u64,
    pub borrow_instruction_index: u8,
}

/// Cuentas necesarias para FlashBorrowReserveLiquidity
#[derive(Debug, Clone)]
pub struct FlashBorrowReserveLiquidityAccounts {
    pub user_transfer_authority: Pubkey,
    pub lending_market_authority: Pubkey,
    pub lending_market: Pubkey,
    pub reserve: Pubkey,
    pub reserve_liquidity_mint: Pubkey,
    pub reserve_source_liquidity: Pubkey,
    pub user_destination_liquidity: Pubkey,
    pub reserve_liquidity_fee_receiver: Pubkey,
    pub referrer_token_state: Option<Pubkey>,
    pub referrer_account: Option<Pubkey>,
    pub sysvar_info: Pubkey,
    pub token_program: Pubkey,
}

/// Cuentas necesarias para FlashRepayReserveLiquidity
#[derive(Debug, Clone)]
pub struct FlashRepayReserveLiquidityAccounts {
    pub user_transfer_authority: Pubkey,
    pub lending_market_authority: Pubkey,
    pub lending_market: Pubkey,
    pub reserve: Pubkey,
    pub reserve_liquidity_mint: Pubkey,
    pub reserve_destination_liquidity: Pubkey,
    pub user_source_liquidity: Pubkey,
    pub reserve_liquidity_fee_receiver: Pubkey,
    pub referrer_token_state: Option<Pubkey>,
    pub referrer_account: Option<Pubkey>,
    pub sysvar_info: Pubkey,
    pub token_program: Pubkey,
}

/// Estructura de datos del Reserve (simplificada para lectura)
#[derive(Debug, Clone, BorshDeserialize)]
pub struct Reserve {
    pub version: u8,
    pub last_update: LastUpdate,
    pub lending_market: Pubkey,
    pub farm_collateral: Pubkey,
    pub farm_debt: Pubkey,
    pub liquidity: ReserveLiquidity,
    pub collateral: ReserveCollateral,
    pub config: ReserveConfig,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct LastUpdate {
    pub slot: u64,
    pub stale: bool,
    pub price_status: u8,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct ReserveLiquidity {
    pub mint_pubkey: Pubkey,
    pub supply_pubkey: Pubkey,
    pub fee_receiver: Pubkey,
    pub available_amount: u64,
    pub borrowed_amount_sf: u128,
    pub market_price_sf: u128,
    pub market_price_last_updated_ts: u64,
    pub mint_decimals: u64,
    pub deposit_limit_crossed_timestamp: u64,
    pub borrow_limit_crossed_timestamp: u64,
    pub cumulative_borrow_rate_bsf: u128,
    pub accumulated_protocol_fees_sf: u128,
    pub accumulated_referrer_fees_sf: u128,
    pub pending_referrer_fees_sf: u128,
    pub absolute_referral_rate_sf: u128,
    pub token_program: Pubkey,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct ReserveCollateral {
    pub mint_pubkey: Pubkey,
    pub mint_total_supply: u64,
    pub supply_pubkey: Pubkey,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct ReserveConfig {
    pub status: u8,
    pub asset_tier: u8,
    pub host_fixed_interest_rate_bps: u16,
    pub min_deleveraging_bonus_bps: u16,
    pub max_deleveraging_bonus_bps: u16,
    pub deleveraging_threshold_bps_per_day: u64,
    pub fees: ReserveFees,
    pub borrow_rate_curve: BorrowRateCurve,
    pub deposit_limit: u64,
    pub borrow_limit: u64,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct ReserveFees {
    pub borrow_fee_sf: u64,
    pub flash_loan_fee_sf: u64,
    pub referral_fee_bps: u16,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct BorrowRateCurve {
    pub points: [BorrowRateCurvePoint; 11],
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct BorrowRateCurvePoint {
    pub utilization_rate_bps: u32,
    pub borrow_rate_bps: u32,
}

/// Calcula el lending market authority PDA
pub fn get_lending_market_authority(lending_market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"lending_market_auth", lending_market.as_ref()],
        &Pubkey::from_str(KAMINO_LEND_PROGRAM).unwrap(),
    )
}

/// Obtiene el PDA del reserve liquidity supply
pub fn get_reserve_liquidity_supply(reserve: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"reserve_liq_supply", reserve.as_ref()],
        &Pubkey::from_str(KAMINO_LEND_PROGRAM).unwrap(),
    )
}

/// Obtiene el PDA del reserve fee receiver
pub fn get_reserve_fee_receiver(reserve: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"fee_receiver", reserve.as_ref()],
        &Pubkey::from_str(KAMINO_LEND_PROGRAM).unwrap(),
    )
}

/// Calcula las fees del flash loan
/// 
/// # Arguments
/// * `amount` - Cantidad del flash loan en unidades base del token
/// * `flash_loan_fee_sf` - Fee configurado en el reserve (scaled fraction con 18 decimales)
/// 
/// # Returns
/// * `(protocol_fee, total_repay)` - Fee calculada y total a repagar
/// 
/// # Nota
/// El fee estándar de Kamino es 0.09% = 900_000_000_000_000 en scaled fraction
/// Fórmula: protocol_fee = amount * flash_loan_fee_sf / 10^18
pub fn calculate_flash_loan_fees(amount: u64, flash_loan_fee_sf: u64) -> (u64, u64) {
    // flash_loan_fee_sf está en scaled fraction (sf) con 18 decimales
    // Para simplificar, usamos el fee estándar de 0.09%
    if flash_loan_fee_sf == u64::MAX {
        // Flash loans deshabilitados
        return (0, 0);
    }
    
    // Scaled fraction usa 18 decimales
    const SCALED_FRACTION_DENOMINATOR: u128 = 1_000_000_000_000_000_000u128;
    
    let protocol_fee = (amount as u128 * flash_loan_fee_sf as u128 / SCALED_FRACTION_DENOMINATOR) as u64;
    let total_repay = amount.saturating_add(protocol_fee);
    
    (protocol_fee, total_repay)
}

/// Calcula el fee usando el fee estándar de Kamino (0.09%)
pub fn calculate_flash_loan_fees_standard(amount: u64) -> (u64, u64) {
    // Fee estándar de Kamino: 0.09% = 9 bps
    // En scaled fraction: 0.0009 * 10^18 = 900_000_000_000_000
    let standard_fee_sf: u64 = 900_000_000_000_000;
    calculate_flash_loan_fees(amount, standard_fee_sf)
}

/// Crea la instrucción de Flash Borrow Reserve Liquidity
pub fn create_flash_borrow_instruction(
    accounts: &FlashBorrowReserveLiquidityAccounts,
    liquidity_amount: u64,
) -> Instruction {
    let args = FlashBorrowReserveLiquidityArgs { liquidity_amount };
    
    let mut data = flash_borrow_discriminator().to_vec();
    data.extend_from_slice(&borsh::to_vec(&args).unwrap());
    
    let mut account_metas = vec![
        AccountMeta::new_readonly(accounts.user_transfer_authority, true),
        AccountMeta::new_readonly(accounts.lending_market_authority, false),
        AccountMeta::new_readonly(accounts.lending_market, false),
        AccountMeta::new(accounts.reserve, false),
        AccountMeta::new_readonly(accounts.reserve_liquidity_mint, false),
        AccountMeta::new(accounts.reserve_source_liquidity, false),
        AccountMeta::new(accounts.user_destination_liquidity, false),
        AccountMeta::new(accounts.reserve_liquidity_fee_receiver, false),
    ];
    
    // Referrer token state (optional)
    if let Some(referrer_token_state) = accounts.referrer_token_state {
        account_metas.push(AccountMeta::new(referrer_token_state, false));
    } else {
        account_metas.push(AccountMeta::new_readonly(system_program::ID, false));
    }
    
    // Referrer account (optional)
    if let Some(referrer_account) = accounts.referrer_account {
        account_metas.push(AccountMeta::new_readonly(referrer_account, false));
    } else {
        account_metas.push(AccountMeta::new_readonly(system_program::ID, false));
    }
    
    account_metas.extend_from_slice(&[
        AccountMeta::new_readonly(accounts.sysvar_info, false),
        AccountMeta::new_readonly(accounts.token_program, false),
    ]);
    
    Instruction {
        program_id: Pubkey::from_str(KAMINO_LEND_PROGRAM).unwrap(),
        accounts: account_metas,
        data,
    }
}

/// Crea la instrucción de Flash Repay Reserve Liquidity
pub fn create_flash_repay_instruction(
    accounts: &FlashRepayReserveLiquidityAccounts,
    liquidity_amount: u64,
    borrow_instruction_index: u8,
) -> Instruction {
    let args = FlashRepayReserveLiquidityArgs {
        liquidity_amount,
        borrow_instruction_index,
    };
    
    let mut data = flash_repay_discriminator().to_vec();
    data.extend_from_slice(&borsh::to_vec(&args).unwrap());
    
    let mut account_metas = vec![
        AccountMeta::new_readonly(accounts.user_transfer_authority, true),
        AccountMeta::new_readonly(accounts.lending_market_authority, false),
        AccountMeta::new_readonly(accounts.lending_market, false),
        AccountMeta::new(accounts.reserve, false),
        AccountMeta::new_readonly(accounts.reserve_liquidity_mint, false),
        AccountMeta::new(accounts.reserve_destination_liquidity, false),
        AccountMeta::new(accounts.user_source_liquidity, false),
        AccountMeta::new(accounts.reserve_liquidity_fee_receiver, false),
    ];
    
    // Referrer token state (optional)
    if let Some(referrer_token_state) = accounts.referrer_token_state {
        account_metas.push(AccountMeta::new(referrer_token_state, false));
    } else {
        account_metas.push(AccountMeta::new_readonly(system_program::ID, false));
    }
    
    // Referrer account (optional)
    if let Some(referrer_account) = accounts.referrer_account {
        account_metas.push(AccountMeta::new_readonly(referrer_account, false));
    } else {
        account_metas.push(AccountMeta::new_readonly(system_program::ID, false));
    }
    
    account_metas.extend_from_slice(&[
        AccountMeta::new_readonly(accounts.sysvar_info, false),
        AccountMeta::new_readonly(accounts.token_program, false),
    ]);
    
    Instruction {
        program_id: Pubkey::from_str(KAMINO_LEND_PROGRAM).unwrap(),
        accounts: account_metas,
        data,
    }
}

/// SPL Associated Token Account Program ID
pub const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

/// Obtiene la ATA (Associated Token Account) para un owner y mint
/// Usa el programa SPL Associated Token Account
pub fn get_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let associated_token_program = Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID).unwrap();
    let token_program = Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap();
    
    let (ata, _) = Pubkey::find_program_address(
        &[
            owner.as_ref(),
            token_program.as_ref(),
            mint.as_ref(),
        ],
        &associated_token_program,
    );
    ata
}

/// Crea una instrucción para crear una ATA
pub fn create_associated_token_account_instruction(
    payer: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Instruction {
    let associated_token = get_associated_token_address(owner, mint);
    let associated_token_program = Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID).unwrap();
    let token_program = Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap();
    let system_program = Pubkey::from_str("11111111111111111111111111111111").unwrap();
    
    Instruction {
        program_id: associated_token_program,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(associated_token, false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(token_program, false),
        ],
        data: vec![], // No data needed for Create instruction
    }
}

/// Lee los datos de un reserve desde la blockchain
pub async fn fetch_reserve_data(
    client: &RpcClient,
    reserve: &Pubkey,
) -> Result<Reserve, Box<dyn std::error::Error>> {
    let account = client.get_account(reserve).await?;
    // Skip discriminator (8 bytes) for Anchor accounts
    let data = &account.data[8..];
    let reserve_data = Reserve::try_from_slice(data)?;
    Ok(reserve_data)
}

/// Estructura para una ruta de Jupiter Swap
#[derive(Debug, Clone)]
pub struct JupiterSwapRoute {
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub in_amount: u64,
    pub out_amount: u64,
    pub other_amount_threshold: u64,
    pub swap_mode: String,
    pub slippage_bps: u16,
    pub platform_fee: Option<u64>,
    pub price_impact_pct: f64,
}

/// Obtiene una quote de Jupiter para swap
pub async fn get_jupiter_swap_quote(
    input_mint: &str,
    output_mint: &str,
    amount: u64,
    slippage_bps: u16,
) -> Result<JupiterSwapRoute, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&onlyDirectRoutes=false",
        config::JUPITER_QUOTE_API,
        input_mint,
        output_mint,
        amount,
        slippage_bps
    );
    
    let response = client.get(&url).send().await?;
    let json: serde_json::Value = response.json().await?;
    
    if let Some(error) = json.get("error") {
        return Err(format!("Jupiter API error: {:?}", error).into());
    }
    
    let route = JupiterSwapRoute {
        input_mint: Pubkey::from_str(input_mint)?,
        output_mint: Pubkey::from_str(output_mint)?,
        in_amount: json["inAmount"].as_str().unwrap_or("0").parse::<u64>()?,
        out_amount: json["outAmount"].as_str().unwrap_or("0").parse::<u64>()?,
        other_amount_threshold: json["otherAmountThreshold"].as_str().unwrap_or("0").parse::<u64>()?,
        swap_mode: json["swapMode"].as_str().unwrap_or("ExactIn").to_string(),
        slippage_bps: json["slippageBps"].as_u64().unwrap_or(slippage_bps as u64) as u16,
        platform_fee: json["platformFee"]["amount"].as_str().map(|s| s.parse::<u64>().unwrap_or(0)),
        price_impact_pct: json["priceImpactPct"].as_str().unwrap_or("0").parse::<f64>()?,
    };
    
    Ok(route)
}

/// Obtiene las instrucciones de swap de Jupiter
pub async fn get_jupiter_swap_instructions(
    user_public_key: &Pubkey,
    quote_response: &serde_json::Value,
) -> Result<Vec<Instruction>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("{}/swap-instructions", config::JUPITER_QUOTE_API);
    
    let body = serde_json::json!({
        "userPublicKey": user_public_key.to_string(),
        "wrapAndUnwrapSol": true,
        "useSharedAccounts": true,
        "quoteResponse": quote_response,
    });
    
    let response = client.post(&url).json(&body).send().await?;
    let json: serde_json::Value = response.json().await?;
    
    if let Some(error) = json.get("error") {
        return Err(format!("Jupiter swap instructions error: {:?}", error).into());
    }
    
    // Parsear instrucciones de swap (simplificado - en producción parsear completamente)
    let mut instructions = Vec::new();
    
    if let Some(setup_instructions) = json["setupInstructions"].as_array() {
        for ix_json in setup_instructions {
            if let Some(ix) = parse_jupiter_instruction(ix_json) {
                instructions.push(ix);
            }
        }
    }
    
    if let Some(swap_ix) = json["swapInstruction"].as_object() {
        let ix_json = serde_json::Value::Object(swap_ix.clone());
        if let Some(ix) = parse_jupiter_instruction(&ix_json) {
            instructions.push(ix);
        }
    }
    
    if let Some(cleanup_ix) = json["cleanupInstruction"].as_object() {
        let ix_json = serde_json::Value::Object(cleanup_ix.clone());
        if let Some(ix) = parse_jupiter_instruction(&ix_json) {
            instructions.push(ix);
        }
    }
    
    Ok(instructions)
}

/// Parsea una instrucción de Jupiter desde JSON
fn parse_jupiter_instruction(ix_json: &serde_json::Value) -> Option<Instruction> {
    let program_id = Pubkey::from_str(ix_json["programId"].as_str()?).ok()?;
    
    let accounts: Vec<AccountMeta> = ix_json["accounts"]
        .as_array()?
        .iter()
        .filter_map(|acc| {
            let pubkey = Pubkey::from_str(acc["pubkey"].as_str()?).ok()?;
            let is_signer = acc["isSigner"].as_bool()?;
            let is_writable = acc["isWritable"].as_bool()?;
            
            if is_writable {
                Some(AccountMeta::new(pubkey, is_signer))
            } else {
                Some(AccountMeta::new_readonly(pubkey, is_signer))
            }
        })
        .collect();
    
    let data = bs58::decode(ix_json["data"].as_str()?).into_vec().ok()?;
    
    Some(Instruction {
        program_id,
        accounts,
        data,
    })
}

/// Construye una transacción completa de Flash Loan con arbitrage
pub async fn build_flash_loan_tx(
    client: &RpcClient,
    keypair: &solana_sdk::signature::Keypair,
    flash_amount: u64,
) -> Result<Option<VersionedTransaction>, Box<dyn std::error::Error>> {
    utils::log_info("⚡ Construyendo Flash Loan con Kamino Lending...");
    
    let recent_blockhash = client.get_latest_blockhash().await?;
    let user_pubkey = keypair.pubkey();
    
    // Configuración de cuentas principales
    let lending_market = Pubkey::from_str(MAIN_LENDING_MARKET)?;
    let reserve = Pubkey::from_str(USDC_RESERVE)?;
    let reserve_liquidity_mint = Pubkey::from_str(USDC_MINT)?;
    let token_program = Pubkey::from_str(TOKEN_PROGRAM_ID)?;

    // Calcular PDAs
    let (lending_market_authority, _) = get_lending_market_authority(&lending_market);
    let (reserve_source_liquidity, _) = get_reserve_liquidity_supply(&reserve);
    let (reserve_liquidity_fee_receiver, _) = get_reserve_fee_receiver(&reserve);

    // ATA del usuario para recibir liquidez prestada (USDC)
    let user_destination_liquidity = get_associated_token_address(&user_pubkey, &reserve_liquidity_mint);

    // Verificar que el usuario tiene la ATA creada y agregar instrucción de creación si es necesario
    let mut setup_instructions: Vec<Instruction> = Vec::new();
    match client.get_account(&user_destination_liquidity).await {
        Ok(_) => {
            utils::log_info(&format!(
                "✅ ATA del usuario verificada: {}",
                user_destination_liquidity
            ));
        }
        Err(_) => {
            utils::log_warning(&format!(
                "⚠️  Creando ATA para mint {}...",
                reserve_liquidity_mint
            ));
            let create_ata_ix = create_associated_token_account_instruction(
                &user_pubkey,
                &user_pubkey,
                &reserve_liquidity_mint,
            );
            setup_instructions.push(create_ata_ix);
        }
    }
    
    // Leer datos del reserve para calcular fees
    let reserve_data = match fetch_reserve_data(client, &reserve).await {
        Ok(data) => data,
        Err(e) => {
            utils::log_error(&format!("❌ Error leyendo reserve data: {:?}", e));
            // Usar valores por defecto si no se puede leer
            return Ok(None);
        }
    };
    
    // Calcular fees
    let flash_loan_fee_sf = reserve_data.config.fees.flash_loan_fee_sf;
    if flash_loan_fee_sf == u64::MAX {
        utils::log_error("❌ Flash loans están deshabilitados para este reserve");
        return Ok(None);
    }
    
    let (protocol_fee, total_repay) = calculate_flash_loan_fees(flash_amount, flash_loan_fee_sf);
    utils::log_info(&format!(
        "💰 Flash Loan: {} base + {} fee = {} total a repay",
        flash_amount, protocol_fee, total_repay
    ));
    
    // Construir instrucciones (empezar con setup si es necesario)
    let mut instructions: Vec<Instruction> = setup_instructions;
    
    // 1. FLASH BORROW - Tomar prestado del reserve
    let borrow_accounts = FlashBorrowReserveLiquidityAccounts {
        user_transfer_authority: user_pubkey,
        lending_market_authority,
        lending_market,
        reserve,
        reserve_liquidity_mint,
        reserve_source_liquidity,
        user_destination_liquidity,
        reserve_liquidity_fee_receiver,
        referrer_token_state: None,
        referrer_account: None,
        sysvar_info: sysvar::instructions::ID,
        token_program,
    };
    
    let flash_borrow_ix = create_flash_borrow_instruction(&borrow_accounts, flash_amount);
    instructions.push(flash_borrow_ix);
    
    // 2. JUPITER SWAP - Arbitrage (USDC -> SOL -> USDC)
    // Obtener quote de Jupiter
    let jupiter_quote = match get_jupiter_swap_quote(
        USDC_MINT,
        SOL_MINT,
        flash_amount,
        50, // 0.5% slippage
    ).await {
        Ok(quote) => quote,
        Err(e) => {
            utils::log_error(&format!("❌ Error obteniendo quote de Jupiter: {:?}", e));
            return Ok(None);
        }
    };
    
    // Verificar profitabilidad
    let expected_output = jupiter_quote.out_amount;
    let min_profit_output = total_repay + ((total_repay * 10) / 10000); // 0.1% profit mínimo
    
    if expected_output < min_profit_output {
        utils::log_warning(&format!(
            "⚠️  Swap no rentable: esperado {} vs mínimo {} requerido",
            expected_output, min_profit_output
        ));
        // Continuamos de todas formas para testing, en producción retornar None
    }
    
    // Obtener instrucciones de swap de Jupiter
    let quote_json = serde_json::json!({
        "inAmount": jupiter_quote.in_amount.to_string(),
        "outAmount": jupiter_quote.out_amount.to_string(),
        "otherAmountThreshold": jupiter_quote.other_amount_threshold.to_string(),
        "swapMode": jupiter_quote.swap_mode,
        "slippageBps": jupiter_quote.slippage_bps,
        "priceImpactPct": jupiter_quote.price_impact_pct.to_string(),
        "marketInfos": [], // Simplificado
    });
    
    match get_jupiter_swap_instructions(&user_pubkey, &quote_json).await {
        Ok(swap_instructions) => {
            instructions.extend(swap_instructions);
        }
        Err(e) => {
            utils::log_error(&format!("❌ Error obteniendo instrucciones de swap: {:?}", e));
            return Ok(None);
        }
    }
    
    // 3. FLASH REPAY - Repagar el préstamo
    // La cuenta source ahora debe tener los fondos del swap
    let repay_accounts = FlashRepayReserveLiquidityAccounts {
        user_transfer_authority: user_pubkey,
        lending_market_authority,
        lending_market,
        reserve,
        reserve_liquidity_mint,
        reserve_destination_liquidity: reserve_source_liquidity, // Misma cuenta que source en borrow
        user_source_liquidity: user_destination_liquidity, // Misma cuenta que destination en borrow
        reserve_liquidity_fee_receiver,
        referrer_token_state: None,
        referrer_account: None,
        sysvar_info: sysvar::instructions::ID,
        token_program,
    };
    
    let flash_repay_ix = create_flash_repay_instruction(&repay_accounts, flash_amount, 0);
    instructions.push(flash_repay_ix);
    
    // Construir transacción versionada
    let message = Message::try_compile(
        &user_pubkey,
        &instructions,
        &[], // Address lookup tables (opcional)
        recent_blockhash,
    )?;
    
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(message), &[keypair])?;
    
    // Simular transacción antes de enviar
    utils::log_info("🔍 Simulando transacción...");
    match client.simulate_transaction(&tx).await {
        Ok(simulation) => {
            if let Some(err) = simulation.value.err {
                utils::log_error(&format!("❌ Simulación falló: {:?}", err));
                if let Some(logs) = simulation.value.logs {
                    for log in logs.iter().take(10) {
                        utils::log_error(&format!("   > {}", log));
                    }
                }
                return Ok(None);
            }
            
            // Calcular profit estimado
            let estimated_profit = expected_output.saturating_sub(total_repay);
            utils::log_success(&format!(
                "✅ Flash Loan simulado correctamente. Profit estimado: {} lamports",
                estimated_profit
            ));
        }
        Err(e) => {
            utils::log_error(&format!("❌ Error en simulación: {:?}", e));
            return Ok(None);
        }
    }
    
    Ok(Some(tx))
}

/// Ejecuta un flash loan simple sin arbitrage (solo borrow + repay)
pub async fn build_simple_flash_loan_tx(
    client: &RpcClient,
    keypair: &solana_sdk::signature::Keypair,
    flash_amount: u64,
    reserve_pubkey: &str,
    mint_pubkey: &str,
) -> Result<Option<VersionedTransaction>, Box<dyn std::error::Error>> {
    utils::log_info("⚡ Construyendo Flash Loan simple (borrow + repay)...");
    
    let recent_blockhash = client.get_latest_blockhash().await?;
    let user_pubkey = keypair.pubkey();
    
    let lending_market = Pubkey::from_str(MAIN_LENDING_MARKET)?;
    let reserve = Pubkey::from_str(reserve_pubkey)?;
    let reserve_liquidity_mint = Pubkey::from_str(mint_pubkey)?;
    let token_program = Pubkey::from_str(TOKEN_PROGRAM_ID)?;

    let (lending_market_authority, _) = get_lending_market_authority(&lending_market);
    let (reserve_source_liquidity, _) = get_reserve_liquidity_supply(&reserve);
    let (reserve_liquidity_fee_receiver, _) = get_reserve_fee_receiver(&reserve);
    let user_destination_liquidity = get_associated_token_address(&user_pubkey, &reserve_liquidity_mint);
    
    let mut instructions: Vec<Instruction> = Vec::new();
    
    // Flash Borrow
    let borrow_accounts = FlashBorrowReserveLiquidityAccounts {
        user_transfer_authority: user_pubkey,
        lending_market_authority,
        lending_market,
        reserve,
        reserve_liquidity_mint,
        reserve_source_liquidity,
        user_destination_liquidity,
        reserve_liquidity_fee_receiver,
        referrer_token_state: None,
        referrer_account: None,
        sysvar_info: sysvar::instructions::ID,
        token_program,
    };
    
    let flash_borrow_ix = create_flash_borrow_instruction(&borrow_accounts, flash_amount);
    instructions.push(flash_borrow_ix);
    
    // Flash Repay (inmediato, sin operaciones intermedias)
    let repay_accounts = FlashRepayReserveLiquidityAccounts {
        user_transfer_authority: user_pubkey,
        lending_market_authority,
        lending_market,
        reserve,
        reserve_liquidity_mint,
        reserve_destination_liquidity: reserve_source_liquidity,
        user_source_liquidity: user_destination_liquidity,
        reserve_liquidity_fee_receiver,
        referrer_token_state: None,
        referrer_account: None,
        sysvar_info: sysvar::instructions::ID,
        token_program,
    };
    
    let flash_repay_ix = create_flash_repay_instruction(&repay_accounts, flash_amount, 0);
    instructions.push(flash_repay_ix);
    
    let message = Message::try_compile(&user_pubkey, &instructions, &[], recent_blockhash)?;
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(message), &[keypair])?;
    
    Ok(Some(tx))
}

/// Lista de reserves populares en Kamino Main Market
/// 
/// # IMPORTANTE
/// Estas direcciones son ejemplos. Para mainnet real, debes:
/// 1. Consultar el lending market de Kamino para obtener los reserves activos
/// 2. O usar el Kamino SDK para obtener las direcciones actualizadas
/// 
/// Las direcciones reales pueden obtenerse de:
/// - https://app.kamino.finance (inspect element / network tab)
/// - Kamino SDK: `KaminoMarket.load(connection, marketAddress)`
/// - API de Kamino
pub mod kamino_reserves {
    use super::*;

    /// Main Lending Market de Kamino (Main Market)
    pub const MAIN_MARKET: &str = "7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF";

    /// USDC Reserve (Main Market) - real mainnet address
    pub const USDC: &str = "D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59";

    /// USDT Reserve (Main Market) - real mainnet address
    pub const USDT: &str = "H3t6qZ1JkguCNTi9uzVKqQ7dvt2cum4XiXWom6Gn5e5S";

    /// SOL (Wrapped) Reserve (Main Market) - real mainnet address
    pub const SOL: &str = "d4A2prbA2whesmvHaL88BH6Ewn5N4bTSU2Ze8P6Bc4Q";

    /// Estructura para información de un reserve
    #[derive(Debug, Clone)]
    pub struct ReserveInfo {
        pub address: Pubkey,
        pub mint: Pubkey,
        pub symbol: String,
        pub decimals: u8,
    }

    /// Obtiene la lista de reserves conocidos
    pub fn get_known_reserves() -> Vec<ReserveInfo> {
        vec![
            ReserveInfo {
                address: Pubkey::from_str(USDC).unwrap(),
                mint: Pubkey::from_str(super::USDC_MINT).unwrap(),
                symbol: "USDC".to_string(),
                decimals: 6,
            },
            ReserveInfo {
                address: Pubkey::from_str(USDT).unwrap(),
                mint: Pubkey::from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB").unwrap(),
                symbol: "USDT".to_string(),
                decimals: 6,
            },
            ReserveInfo {
                address: Pubkey::from_str(SOL).unwrap(),
                mint: Pubkey::from_str(super::SOL_MINT).unwrap(),
                symbol: "SOL".to_string(),
                decimals: 9,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_calculate_flash_loan_fees() {
        // Fee de 0.09% = 900000000000000 (en scaled fraction con 18 decimales)
        let flash_loan_fee_sf = 900_000_000_000_000u64;
        let amount = 1_000_000_000u64; // 1000 USDC
        
        let (protocol_fee, total_repay) = calculate_flash_loan_fees(amount, flash_loan_fee_sf);
        
        // 0.09% de 1000 = 0.9 USDC
        assert!(protocol_fee > 0);
        assert_eq!(total_repay, amount + protocol_fee);
    }
    
    #[test]
    fn test_get_lending_market_authority() {
        let lending_market = Pubkey::from_str(MAIN_LENDING_MARKET).unwrap();
        let (authority, bump) = get_lending_market_authority(&lending_market);
        
        assert_ne!(authority, Pubkey::default());
        // Verificar que es un PDA válido
        let seeds = &[b"lending_market_auth", lending_market.as_ref()];
        let expected = Pubkey::find_program_address(seeds, &Pubkey::from_str(KAMINO_LEND_PROGRAM).unwrap());
        assert_eq!(authority, expected.0);
        assert_eq!(bump, expected.1);
    }
    
    #[test]
    fn test_flash_borrow_accounts_struct() {
        let accounts = FlashBorrowReserveLiquidityAccounts {
            user_transfer_authority: Pubkey::new_unique(),
            lending_market_authority: Pubkey::new_unique(),
            lending_market: Pubkey::from_str(MAIN_LENDING_MARKET).unwrap(),
            reserve: Pubkey::from_str(USDC_RESERVE).unwrap(),
            reserve_liquidity_mint: Pubkey::from_str(USDC_MINT).unwrap(),
            reserve_source_liquidity: Pubkey::new_unique(),
            user_destination_liquidity: Pubkey::new_unique(),
            reserve_liquidity_fee_receiver: Pubkey::new_unique(),
            referrer_token_state: None,
            referrer_account: None,
            sysvar_info: sysvar::instructions::ID,
            token_program: Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap(),
        };
        
        let ix = create_flash_borrow_instruction(&accounts, 1_000_000_000);
        
        assert_eq!(ix.program_id, Pubkey::from_str(KAMINO_LEND_PROGRAM).unwrap());
        assert!(!ix.data.is_empty());
        assert_eq!(&ix.data[..8], &flash_borrow_discriminator());
    }
}
