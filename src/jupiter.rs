use reqwest::Client;
use serde_json::Value;

use crate::{config, utils};

pub async fn get_best_jupiter_quote(
    input_mint: &str,
    output_mint: &str,
    amount: u64,
) -> Option<f64> {
    let client = Client::new();
    let url = format!(
        "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps=50&onlyDirectRoutes=false",
        config::JUPITER_QUOTE_API,
        input_mint,
        output_mint,
        amount
    );

    if let Ok(resp) = client.get(&url).send().await {
        if let Ok(json) = resp.json::<Value>().await {
            if let (Some(in_amt), Some(out_amt)) =
                (json["inAmount"].as_str(), json["outAmount"].as_str())
            {
                let profit = (out_amt.parse::<u64>().unwrap_or(0) as f64
                    - in_amt.parse::<u64>().unwrap_or(0) as f64)
                    / 1_000_000.0;
                if profit > config::MIN_PROFIT_USD {
                    utils::log_success(&format!("💰 Mejor ruta Jupiter: +${:.2}", profit));
                    return Some(profit);
                }
            }
        }
    }
    None
}