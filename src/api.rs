use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::memory::{AgentMemory, RiskLevel};

// ---------------------------------------------------------------------------
// Shared Bot State — everything the API can read/write
// ---------------------------------------------------------------------------

pub struct BotState {
    pub memory: AgentMemory,
    pub running: bool,
    pub cycle: u64,
    pub build_id: String,
    /// Ring buffer of recent activity logs (last 100 lines)
    pub activity_log: VecDeque<String>,
    /// Last scan report text
    pub last_scan_report: String,
    /// Last liquidation report text
    pub last_liq_report: String,
    /// Pending commands from API (processed by main loop)
    pub pending_commands: Vec<ApiCommand>,
}

#[derive(Debug, Clone)]
pub enum ApiCommand {
    SetRisk(RiskLevel),
    ResetMemory,
    SetRunning(bool),
    SetMinProfit(f64),
    ForceScan,
}

pub type SharedState = Arc<RwLock<BotState>>;

impl BotState {
    pub fn new(memory: AgentMemory, build_id: String) -> Self {
        Self {
            memory,
            running: true,
            cycle: 0,
            build_id,
            activity_log: VecDeque::with_capacity(200),
            last_scan_report: String::new(),
            last_liq_report: String::new(),
            pending_commands: Vec::new(),
        }
    }

    pub fn log(&mut self, msg: &str) {
        let ts = crate::memory::current_timestamp();
        let line = format!("[{}] {}", ts, msg);
        self.activity_log.push_back(line);
        if self.activity_log.len() > 200 {
            self.activity_log.pop_front();
        }
    }
}

// ---------------------------------------------------------------------------
// HTTP API Server
// ---------------------------------------------------------------------------

pub async fn start_api_server(state: SharedState, port: u16) {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)).await {
        Ok(l) => {
            crate::utils::log_info(&format!("API server on :{}", port));
            l
        }
        Err(e) => {
            crate::utils::log_error(&format!("API server failed to bind: {}", e));
            return;
        }
    };

    loop {
        if let Ok((mut stream, _)) = listener.accept().await {
            let state = state.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let n = match stream.read(&mut buf).await {
                    Ok(n) if n > 0 => n,
                    _ => return,
                };
                let request = String::from_utf8_lossy(&buf[..n]).to_string();
                let response = handle_request(&request, &state).await;
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    }
}

async fn handle_request(request: &str, state: &SharedState) -> String {
    let first_line = request.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    let method = parts.first().copied().unwrap_or("GET");
    let path = parts.get(1).copied().unwrap_or("/");

    // Extract JSON body for POST requests
    let body = request.split("\r\n\r\n").nth(1).unwrap_or("");

    match (method, path) {
        ("GET", "/") | ("GET", "/health") => json_response(200, r#"{"status":"ok","bot":"ELCOQUI"}"#),

        ("GET", "/api/status") => {
            let s = state.read().await;
            let json = format!(
                r#"{{"cycle":{},"running":{},"risk":"{:?}","profit":{:.4},"opportunities":{},"build":"{}","api_delay_ms":{},"min_profit":{:.4},"dry_run":{}}}"#,
                s.cycle, s.running, s.memory.risk_level,
                s.memory.total_profit_usd, s.memory.total_opportunities,
                s.build_id, s.memory.get_api_delay_ms(),
                s.memory.min_profit_usd, crate::config::DRY_RUN
            );
            json_response(200, &json)
        }

        ("GET", "/api/memory") => {
            let s = state.read().await;
            let summary = s.memory.summary();
            let learning = s.memory.learning_report();
            let scan = s.memory.scan_summary();
            let json = format!(
                r#"{{"summary":{},"learning":{},"scan":{}}}"#,
                serde_json::to_string(&summary).unwrap_or_default(),
                serde_json::to_string(&learning).unwrap_or_default(),
                serde_json::to_string(&scan).unwrap_or_default(),
            );
            json_response(200, &json)
        }

        ("GET", "/api/scan") => {
            let s = state.read().await;
            let json = format!(
                r#"{{"last_scan":{},"last_liq":{}}}"#,
                serde_json::to_string(&s.last_scan_report).unwrap_or_default(),
                serde_json::to_string(&s.last_liq_report).unwrap_or_default(),
            );
            json_response(200, &json)
        }

        ("GET", "/api/logs") => {
            let s = state.read().await;
            let logs: Vec<&str> = s.activity_log.iter().map(|s| s.as_str()).collect();
            let json = serde_json::to_string(&logs).unwrap_or_else(|_| "[]".to_string());
            json_response(200, &json)
        }

        ("GET", "/api/routes") => {
            let s = state.read().await;
            let mut routes_json = Vec::new();
            let mut entries: Vec<_> = s.memory.route_learning.iter().collect();
            entries.sort_by(|a, b| b.1.priority_score.partial_cmp(&a.1.priority_score)
                .unwrap_or(std::cmp::Ordering::Equal));
            for (name, entry) in entries {
                routes_json.push(format!(
                    r#"{{"route":"{}","scans":{},"fails":{},"avg_spread":{:.4},"best_spread":{:.4},"profitable":{},"priority":{:.1}}}"#,
                    name, entry.times_scanned, entry.times_api_failed,
                    entry.avg_spread_pct,
                    if entry.best_spread_pct > f64::NEG_INFINITY { entry.best_spread_pct } else { 0.0 },
                    entry.times_profitable, entry.priority_score,
                ));
            }
            json_response(200, &format!("[{}]", routes_json.join(",")))
        }

        ("GET", "/api/config") => {
            let s = state.read().await;
            let json = format!(
                r#"{{"risk":"{:?}","min_profit":{:.4},"api_delay_ms":{},"liq_interval":{},"dry_run":{},"running":{}}}"#,
                s.memory.risk_level, s.memory.min_profit_usd,
                s.memory.get_api_delay_ms(), s.memory.get_liq_scan_interval(),
                crate::config::DRY_RUN, s.running,
            );
            json_response(200, &json)
        }

        ("POST", "/api/start") => {
            let mut s = state.write().await;
            s.pending_commands.push(ApiCommand::SetRunning(true));
            s.log("API: start command received");
            json_response(200, r#"{"ok":true,"action":"start"}"#)
        }

        ("POST", "/api/stop") => {
            let mut s = state.write().await;
            s.pending_commands.push(ApiCommand::SetRunning(false));
            s.log("API: stop command received");
            json_response(200, r#"{"ok":true,"action":"stop"}"#)
        }

        ("POST", "/api/risk") => {
            let level = if body.contains("aggressive") || body.contains("Aggressive") {
                RiskLevel::Aggressive
            } else if body.contains("safe") || body.contains("Safe") {
                RiskLevel::Safe
            } else {
                RiskLevel::Normal
            };
            let mut s = state.write().await;
            s.pending_commands.push(ApiCommand::SetRisk(level));
            s.log(&format!("API: risk set to {:?}", level));
            json_response(200, &format!(r#"{{"ok":true,"risk":"{:?}"}}"#, level))
        }

        ("POST", "/api/reset") => {
            let mut s = state.write().await;
            s.pending_commands.push(ApiCommand::ResetMemory);
            s.log("API: reset memory command");
            json_response(200, r#"{"ok":true,"action":"reset"}"#)
        }

        ("POST", "/api/min_profit") => {
            if let Some(val) = extract_f64(body, "value") {
                let mut s = state.write().await;
                s.pending_commands.push(ApiCommand::SetMinProfit(val));
                s.log(&format!("API: min_profit set to {:.4}", val));
                json_response(200, &format!(r#"{{"ok":true,"min_profit":{:.4}}}"#, val))
            } else {
                json_response(400, r#"{"error":"missing 'value' field"}"#)
            }
        }

        _ => json_response(404, r#"{"error":"not found","endpoints":["/api/status","/api/memory","/api/scan","/api/logs","/api/routes","/api/config","/api/start","/api/stop","/api/risk","/api/reset","/api/min_profit"]}"#),
    }
}

fn json_response(status: u16, body: &str) -> String {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
        status, status_text, body.len(), body
    )
}

fn extract_f64(body: &str, key: &str) -> Option<f64> {
    // Simple JSON value extraction: {"value": 0.15}
    let pattern = format!("\"{}\"", key);
    if let Some(pos) = body.find(&pattern) {
        let after_key = &body[pos + pattern.len()..];
        let after_colon = after_key.trim_start().strip_prefix(':')?;
        let num_str: String = after_colon.trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
            .collect();
        num_str.parse().ok()
    } else {
        None
    }
}
