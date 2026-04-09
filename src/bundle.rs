use solana_sdk::{
    instruction::Instruction,
    message::{v0::Message, VersionedMessage},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_instruction,
    transaction::VersionedTransaction,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use thiserror::Error;

use crate::{config, utils};

// =============================================================================
// JITO CONSTANTS
// =============================================================================

/// Jito Block Engine endpoints for mainnet
pub const JITO_MAINNET_BLOCK_ENGINE: &str = "https://mainnet.block-engine.jito.wtf";
pub const JITO_MAINNET_BLOCK_ENGINE_GRPC: &str = "mainnet.block-engine.jito.wtf:443";

/// Jito tip accounts (validators que reciben tips)
pub const JITO_TIP_ACCOUNTS: &[&str] = &[
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
    "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
    "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
    "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
    "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
    "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
];

/// Minimum tip in lamports to be competitive (0.001 SOL)
pub const MIN_TIP_LAMPORTS: u64 = 1_000_000;

/// Maximum tip percentage of estimated profit
pub const MAX_TIP_PERCENTAGE: f64 = 0.90;

/// Default tip percentage
pub const DEFAULT_TIP_PERCENTAGE: f64 = 0.80;

/// Bundle timeout in seconds
pub const BUNDLE_TIMEOUT_SECS: u64 = 30;

/// Maximum retries for bundle submission
pub const MAX_BUNDLE_RETRIES: u32 = 3;

/// Retry delay in milliseconds
pub const RETRY_DELAY_MS: u64 = 500;

// =============================================================================
// ERROR HANDLING
// =============================================================================

#[derive(Error, Debug)]
pub enum JitoBundleError {
    #[error("RPC error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),
    
    #[error("Transaction error: {0}")]
    TransactionError(String),
    
    #[error("Bundle submission failed: {0}")]
    SubmissionError(String),
    
    #[error("Bundle confirmation timeout")]
    ConfirmationTimeout,
    
    #[error("Bundle simulation failed: {0}")]
    SimulationError(String),
    
    #[error("gRPC error: {0}")]
    GrpcError(String),
    
    #[error("No tip accounts available")]
    NoTipAccounts,
    
    #[error("Invalid bundle: {0}")]
    InvalidBundle(String),
    
    #[error("Network congestion too high")]
    HighCongestion,
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Represents a bundle of transactions to be sent atomically
#[derive(Debug, Clone)]
pub struct JitoBundle {
    /// Transactions in the bundle (must be ordered)
    pub transactions: Vec<VersionedTransaction>,
    /// Estimated tip in lamports
    pub tip_lamports: u64,
    /// Bundle UUID for tracking
    pub bundle_uuid: String,
}

/// Result of a bundle submission
#[derive(Debug, Clone)]
pub struct BundleResult {
    /// Bundle UUID
    pub bundle_uuid: String,
    /// Transaction signatures in the bundle
    pub signatures: Vec<Signature>,
    /// Whether the bundle was accepted
    pub accepted: bool,
    /// Slot when bundle was processed
    pub processed_slot: Option<u64>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Network congestion level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CongestionLevel {
    Low,      // < 50% utilization
    Medium,   // 50-80% utilization
    High,     // 80-95% utilization
    Extreme,  // > 95% utilization
}

/// Bundle configuration
#[derive(Debug, Clone)]
pub struct BundleConfig {
    /// Block engine endpoint
    pub block_engine_url: String,
    /// gRPC endpoint
    pub grpc_endpoint: String,
    /// Tip account to use
    pub tip_account: Pubkey,
    /// Maximum tip percentage
    pub max_tip_percentage: f64,
    /// Minimum tip in lamports
    pub min_tip_lamports: u64,
    /// Whether to wait for confirmation
    pub wait_for_confirmation: bool,
    /// Bundle timeout
    pub timeout_secs: u64,
    /// Maximum retries
    pub max_retries: u32,
}

impl Default for BundleConfig {
    fn default() -> Self {
        // Parse tip account at runtime
        let tip_account = parse_pubkey(JITO_TIP_ACCOUNTS[0])
            .unwrap_or_else(|_| Pubkey::new_from_array([0u8; 32]));
        
        Self {
            block_engine_url: JITO_MAINNET_BLOCK_ENGINE.to_string(),
            grpc_endpoint: JITO_MAINNET_BLOCK_ENGINE_GRPC.to_string(),
            tip_account,
            max_tip_percentage: MAX_TIP_PERCENTAGE,
            min_tip_lamports: MIN_TIP_LAMPORTS,
            wait_for_confirmation: true,
            timeout_secs: BUNDLE_TIMEOUT_SECS,
            max_retries: MAX_BUNDLE_RETRIES,
        }
    }
}

/// Jito block engine client
#[derive(Clone)]
pub struct JitoClient {
    rpc_client: Arc<RpcClient>,
    config: BundleConfig,
    http_client: reqwest::Client,
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Parse a pubkey from string
fn parse_pubkey(s: &str) -> Result<Pubkey, JitoBundleError> {
    use std::str::FromStr;
    Pubkey::from_str(s).map_err(|e| {
        JitoBundleError::TransactionError(format!("Invalid pubkey {}: {}", s, e))
    })
}

/// Generate a new UUID for bundle tracking
fn generate_bundle_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Serialize transaction to base64
fn serialize_transaction_to_base64(tx: &VersionedTransaction) -> Result<String, JitoBundleError> {
    let serialized = bincode::serialize(tx)
        .map_err(|e| JitoBundleError::SerializationError(e.to_string()))?;
    Ok(base64::encode(serialized))
}

// =============================================================================
// JITO CLIENT IMPLEMENTATION
// =============================================================================

impl JitoClient {
    /// Create a new Jito client with the given RPC client
    pub fn new(rpc_client: Arc<RpcClient>) -> Result<Self, JitoBundleError> {
        let config = BundleConfig::default();
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| JitoBundleError::SubmissionError(e.to_string()))?;

        Ok(Self {
            rpc_client,
            config,
            http_client,
        })
    }

    /// Create a new Jito client with custom config
    pub fn with_config(
        rpc_client: Arc<RpcClient>,
        config: BundleConfig,
    ) -> Result<Self, JitoBundleError> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| JitoBundleError::SubmissionError(e.to_string()))?;

        Ok(Self {
            rpc_client,
            config,
            http_client,
        })
    }

    /// Get current network congestion level
    pub async fn get_congestion_level(&self) -> Result<CongestionLevel, JitoBundleError> {
        let slot = self.rpc_client.get_slot().await?;
        
        // Get recent prioritization fees to estimate congestion
        let recent_prioritization_fees = self.rpc_client
            .get_recent_prioritization_fees(&[])
            .await?;

        let avg_fee: u64 = if recent_prioritization_fees.is_empty() {
            0
        } else {
            recent_prioritization_fees.iter().map(|f| f.prioritization_fee).sum::<u64>() 
                / recent_prioritization_fees.len() as u64
        };

        // Determine congestion level based on prioritization fees
        let level = match avg_fee {
            0..=10_000 => CongestionLevel::Low,
            10_001..=50_000 => CongestionLevel::Medium,
            50_001..=100_000 => CongestionLevel::High,
            _ => CongestionLevel::Extreme,
        };

        utils::log_info(&format!(
            "📊 Network congestion: {:?} (avg fee: {} lamports, slot: {})",
            level, avg_fee, slot
        ));

        Ok(level)
    }

    /// Calculate optimal tip based on congestion and estimated profit
    pub fn calculate_optimal_tip(
        &self,
        estimated_profit_lamports: u64,
        congestion: CongestionLevel,
    ) -> u64 {
        let base_percentage = DEFAULT_TIP_PERCENTAGE;
        
        // Adjust percentage based on congestion
        let adjusted_percentage = match congestion {
            CongestionLevel::Low => base_percentage * 0.5,
            CongestionLevel::Medium => base_percentage * 0.8,
            CongestionLevel::High => base_percentage * 1.2,
            CongestionLevel::Extreme => base_percentage * 1.5,
        };

        // Cap at maximum percentage
        let capped_percentage = adjusted_percentage.min(self.config.max_tip_percentage);
        
        // Calculate tip
        let tip = (estimated_profit_lamports as f64 * capped_percentage) as u64;
        
        // Ensure minimum tip
        let final_tip = tip.max(self.config.min_tip_lamports);
        
        // Cap at 90% of profit to ensure profitability
        let max_tip = (estimated_profit_lamports as f64 * 0.90) as u64;
        let final_tip = final_tip.min(max_tip);

        utils::log_info(&format!(
            "💰 Tip calculation: {} lamports ({}% of profit, congestion: {:?})",
            final_tip,
            (final_tip as f64 / estimated_profit_lamports.max(1) as f64 * 100.0) as u64,
            congestion
        ));

        final_tip
    }

    /// Select a random tip account for load balancing
    pub fn select_tip_account(&self) -> Pubkey {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        
        let idx = (now % JITO_TIP_ACCOUNTS.len() as u128) as usize;
        let tip_pubkey_str = JITO_TIP_ACCOUNTS[idx];
        
        parse_pubkey(tip_pubkey_str).unwrap_or(self.config.tip_account)
    }

    /// Build a tip payment instruction
    pub fn build_tip_instruction(
        &self,
        payer: &Pubkey,
        tip_account: &Pubkey,
        amount_lamports: u64,
    ) -> Instruction {
        system_instruction::transfer(payer, tip_account, amount_lamports)
    }

    /// Simulate a bundle before submission
    pub async fn simulate_bundle(
        &self,
        bundle: &JitoBundle,
    ) -> Result<(), JitoBundleError> {
        for (idx, tx) in bundle.transactions.iter().enumerate() {
            let simulation = self.rpc_client.simulate_transaction(tx).await?;
            
            if let Some(err) = simulation.value.err {
                return Err(JitoBundleError::SimulationError(format!(
                    "Transaction {} failed simulation: {:?}",
                    idx, err
                )));
            }
            
            utils::log_info(&format!(
                "✅ Transaction {} simulated successfully (logs: {})",
                idx,
                simulation.value.logs.as_ref().map(|l| l.len()).unwrap_or(0)
            ));
        }
        
        Ok(())
    }

    /// Submit bundle to Jito block engine via HTTP API
    pub async fn submit_bundle_http(
        &self,
        bundle: &JitoBundle,
    ) -> Result<BundleResult, JitoBundleError> {
        let bundle_uuid = generate_bundle_uuid();
        
        // Serialize transactions to base64
        let transactions_b64: Vec<String> = bundle
            .transactions
            .iter()
            .map(serialize_transaction_to_base64)
            .collect::<Result<Vec<_>, _>>()?;

        // Build request payload
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendBundle",
            "params": [transactions_b64]
        });

        // Submit to block engine
        let url = format!("{}/api/v1/bundles", self.config.block_engine_url);
        
        let response = self.http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| JitoBundleError::SubmissionError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(JitoBundleError::SubmissionError(error_text));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| JitoBundleError::SubmissionError(e.to_string()))?;

        // Extract signatures
        let signatures: Vec<Signature> = bundle
            .transactions
            .iter()
            .map(|tx| {
                // Get the first signature from each transaction
                tx.signatures.get(0).copied()
                    .unwrap_or_else(Signature::default)
            })
            .collect();

        let accepted = response_json.get("result").is_some();

        utils::log_success(&format!(
            "📦 Bundle submitted: {} (accepted: {})",
            bundle_uuid, accepted
        ));

        Ok(BundleResult {
            bundle_uuid,
            signatures,
            accepted,
            processed_slot: None,
            error: None,
        })
    }

    /// Submit bundle with retries
    pub async fn submit_bundle_with_retry(
        &self,
        bundle: &JitoBundle,
    ) -> Result<BundleResult, JitoBundleError> {
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_retries {
            match self.submit_bundle_http(bundle).await {
                Ok(result) => {
                    if result.accepted {
                        return Ok(result);
                    }
                    last_error = Some("Bundle not accepted".to_string());
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    utils::log_warning(&format!(
                        "⚠️ Bundle submission attempt {} failed: {}",
                        attempt, e
                    ));
                }
            }
            
            if attempt < self.config.max_retries {
                sleep(Duration::from_millis(RETRY_DELAY_MS * attempt as u64)).await;
            }
        }

        Err(JitoBundleError::SubmissionError(
            last_error.unwrap_or_else(|| "All retry attempts failed".to_string())
        ))
    }

    /// Wait for bundle confirmation
    pub async fn wait_for_confirmation(
        &self,
        result: &BundleResult,
    ) -> Result<BundleResult, JitoBundleError> {
        if !self.config.wait_for_confirmation {
            return Ok(result.clone());
        }

        let start = Instant::now();
        let timeout = Duration::from_secs(self.config.timeout_secs);

        while start.elapsed() < timeout {
            // Check if any transaction in the bundle is confirmed
            for sig in &result.signatures {
                if sig == &Signature::default() {
                    continue;
                }
                
                match self.rpc_client.get_signature_status(sig).await {
                    Ok(Some(Ok(_))) => {
                        let slot = self.rpc_client.get_slot().await.ok();
                        utils::log_success(&format!(
                            "✅ Bundle confirmed! Signature: {}",
                            sig
                        ));
                        return Ok(BundleResult {
                            bundle_uuid: result.bundle_uuid.clone(),
                            signatures: result.signatures.clone(),
                            accepted: true,
                            processed_slot: slot,
                            error: None,
                        });
                    }
                    Ok(Some(Err(e))) => {
                        return Err(JitoBundleError::TransactionError(format!(
                            "Transaction failed: {:?}", e
                        )));
                    }
                    _ => {}
                }
            }

            sleep(Duration::from_millis(500)).await;
        }

        Err(JitoBundleError::ConfirmationTimeout)
    }
}

// =============================================================================
// BUNDLE BUILDING FUNCTIONS
// =============================================================================

/// Build a complete atomic bundle with flash loan + tip payment
pub async fn build_atomic_bundle(
    rpc_client: &RpcClient,
    keypair: &Keypair,
    flash_loan_tx: VersionedTransaction,
    tip_lamports: u64,
    tip_account: &Pubkey,
) -> Result<JitoBundle, JitoBundleError> {
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;
    
    // Build tip payment transaction
    let tip_ix = system_instruction::transfer(
        &keypair.pubkey(),
        tip_account,
        tip_lamports,
    );

    let tip_message = Message::try_compile(
        &keypair.pubkey(),
        &[tip_ix],
        &[],
        recent_blockhash,
    ).map_err(|e| JitoBundleError::TransactionError(e.to_string()))?;

    let tip_tx = VersionedTransaction::try_new(
        VersionedMessage::V0(tip_message),
        &[keypair],
    ).map_err(|e| JitoBundleError::TransactionError(e.to_string()))?;

    let bundle_uuid = generate_bundle_uuid();

    Ok(JitoBundle {
        transactions: vec![flash_loan_tx, tip_tx],
        tip_lamports,
        bundle_uuid,
    })
}

/// Build a bundle with multiple transactions
pub fn build_bundle_from_transactions(
    transactions: Vec<VersionedTransaction>,
    tip_lamports: u64,
) -> Result<JitoBundle, JitoBundleError> {
    if transactions.is_empty() {
        return Err(JitoBundleError::InvalidBundle(
            "Bundle must contain at least one transaction".to_string()
        ));
    }

    if transactions.len() > 5 {
        return Err(JitoBundleError::InvalidBundle(
            "Bundle cannot contain more than 5 transactions".to_string()
        ));
    }

    let bundle_uuid = generate_bundle_uuid();

    Ok(JitoBundle {
        transactions,
        tip_lamports,
        bundle_uuid,
    })
}

// =============================================================================
// MAIN PUBLIC API
// =============================================================================

/// Send a Jito bundle with the given transaction and estimated profit
/// This is the main entry point for bundle submission
pub async fn send_jito_bundle(
    tx: VersionedTransaction,
    keypair: &Keypair,
    estimated_profit: f64,
) -> Result<String, Box<dyn std::error::Error>> {
    // Convert profit to lamports
    let estimated_profit_lamports = (estimated_profit * 1_000_000_000.0) as u64;
    
    // Create RPC client
    let rpc_client = Arc::new(RpcClient::new(config::get_rpc_url()));
    
    // Create Jito client
    let jito_client = JitoClient::new(rpc_client.clone())?;
    
    // Check network congestion
    let congestion = jito_client.get_congestion_level().await?;
    
    // Calculate optimal tip
    let tip_lamports = jito_client.calculate_optimal_tip(
        estimated_profit_lamports,
        congestion,
    );

    // Handle dry run mode
    if config::DRY_RUN {
        utils::log_info(&format!(
            "🔒 DRY RUN - Bundle con tip {} lamports ({} SOL)",
            tip_lamports,
            tip_lamports as f64 / 1_000_000_000.0
        ));
        utils::log_info(&format!(
            "   Congestion: {:?}, Profit: {} SOL",
            congestion,
            estimated_profit
        ));
        return Ok("dry-run-bundle".to_string());
    }

    utils::log_info(&format!(
        "📦 Construyendo Jito Bundle con tip {} lamports...",
        tip_lamports
    ));

    // Select tip account
    let tip_account = jito_client.select_tip_account();
    utils::log_info(&format!("💸 Tip account: {}", tip_account));

    // Build atomic bundle
    let bundle = build_atomic_bundle(
        &rpc_client,
        keypair,
        tx,
        tip_lamports,
        &tip_account,
    ).await?;

    utils::log_info(&format!(
        "📦 Bundle construido: {} transacciones, UUID: {}",
        bundle.transactions.len(),
        bundle.bundle_uuid
    ));

    // Simulate bundle before submission
    match jito_client.simulate_bundle(&bundle).await {
        Ok(()) => {
            utils::log_success("✅ Bundle simulado correctamente");
        }
        Err(e) => {
            utils::log_error(&format!("❌ Bundle simulation failed: {}", e));
            return Err(Box::new(e));
        }
    }

    // Submit bundle with retries
    let result = match jito_client.submit_bundle_with_retry(&bundle).await {
        Ok(r) => r,
        Err(e) => {
            utils::log_error(&format!("❌ Bundle submission failed: {}", e));
            return Err(Box::new(e));
        }
    };

    utils::log_success(&format!(
        "✅ Bundle enviado a Jito! UUID: {}",
        result.bundle_uuid
    ));

    // Wait for confirmation if configured
    let confirmed_result = match jito_client.wait_for_confirmation(&result).await {
        Ok(r) => r,
        Err(e) => {
            utils::log_warning(&format!(
                "⚠️ Bundle confirmation issue: {}. Bundle may still land.",
                e
            ));
            result
        }
    };

    // Log signatures
    for (idx, sig) in confirmed_result.signatures.iter().enumerate() {
        if sig != &Signature::default() {
            utils::log_info(&format!(
                "   Tx {}: https://solscan.io/tx/{}",
                idx, sig
            ));
        }
    }

    Ok(confirmed_result.bundle_uuid)
}

/// Send a bundle with custom transactions (advanced usage)
pub async fn send_custom_bundle(
    transactions: Vec<VersionedTransaction>,
    keypair: &Keypair,
    estimated_profit_lamports: u64,
) -> Result<BundleResult, JitoBundleError> {
    let rpc_client = Arc::new(RpcClient::new(config::get_rpc_url()));
    let jito_client = JitoClient::new(rpc_client)?;
    
    let congestion = jito_client.get_congestion_level().await?;
    let tip_lamports = jito_client.calculate_optimal_tip(estimated_profit_lamports, congestion);
    
    let bundle = build_bundle_from_transactions(transactions, tip_lamports)?;
    
    jito_client.simulate_bundle(&bundle).await?;
    let result = jito_client.submit_bundle_with_retry(&bundle).await?;
    let confirmed = jito_client.wait_for_confirmation(&result).await?;
    
    Ok(confirmed)
}

/// Get bundle status from Jito
pub async fn get_bundle_status(
    bundle_uuid: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBundleStatuses",
        "params": [[bundle_uuid]]
    });

    let url = format!("{}/api/v1/bundles", JITO_MAINNET_BLOCK_ENGINE);
    
    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    
    Ok(result)
}

/// Get current tip recommendations from Jito
pub async fn get_tip_recommendations() -> Result<Vec<u64>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/api/v1/tips", JITO_MAINNET_BLOCK_ENGINE))
        .send()
        .await?;

    if response.status().is_success() {
        let tips: Vec<u64> = response.json().await?;
        Ok(tips)
    } else {
        // Return default recommendations if API fails
        Ok(vec![
            MIN_TIP_LAMPORTS,
            MIN_TIP_LAMPORTS * 2,
            MIN_TIP_LAMPORTS * 5,
            MIN_TIP_LAMPORTS * 10,
        ])
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pubkey() {
        let pubkey = parse_pubkey(JITO_TIP_ACCOUNTS[0]);
        assert!(pubkey.is_ok());
    }

    #[test]
    fn test_calculate_optimal_tip() {
        let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
        let client = JitoClient::new(rpc_client).unwrap();
        
        let profit = 100_000_000u64; // 0.1 SOL
        
        let tip_low = client.calculate_optimal_tip(profit, CongestionLevel::Low);
        let tip_high = client.calculate_optimal_tip(profit, CongestionLevel::High);
        
        assert!(tip_low >= MIN_TIP_LAMPORTS);
        assert!(tip_high >= tip_low);
        assert!(tip_high <= (profit as f64 * 0.90) as u64);
    }

    #[test]
    fn test_bundle_validation() {
        let empty_result = build_bundle_from_transactions(vec![], 1000);
        assert!(empty_result.is_err());
    }

    #[test]
    fn test_bundle_too_many_transactions() {
        // Create dummy transactions - we can't easily create real ones in tests
        // but we can test the validation logic
        let dummy_txs: Vec<VersionedTransaction> = vec![];
        let result = build_bundle_from_transactions(dummy_txs, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_bundle_uuid() {
        let uuid1 = generate_bundle_uuid();
        let uuid2 = generate_bundle_uuid();
        assert_ne!(uuid1, uuid2);
        assert_eq!(uuid1.len(), 36); // Standard UUID length
    }
}
