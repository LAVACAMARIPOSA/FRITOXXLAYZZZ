use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::utils;

// ---------------------------------------------------------------------------
// Telegram Bot API data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct TelegramResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramMessage {
    pub text: Option<String>,
    pub chat: TelegramChat,
}

#[derive(Debug, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

#[derive(Debug, Serialize)]
struct SendMessageBody<'a> {
    chat_id: &'a str,
    text: &'a str,
    parse_mode: &'a str,
}

// ---------------------------------------------------------------------------
// Bot commands
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum BotCommand {
    Start,
    Stop,
    Status,
    Stats,
    Balance,
    Aggressive,
    Safe,
    Memory,
    Reset,
    Unknown(String),
}

impl BotCommand {
    /// Parse a raw text message into a `BotCommand`.
    pub fn parse(text: &str) -> Self {
        let lower = text.trim().to_lowercase();

        // Slash-commands take priority
        if lower.starts_with('/') {
            return match lower.as_str() {
                "/start" => BotCommand::Start,
                "/stop" => BotCommand::Stop,
                "/status" => BotCommand::Status,
                "/stats" => BotCommand::Stats,
                "/balance" => BotCommand::Balance,
                "/aggressive" => BotCommand::Aggressive,
                "/safe" => BotCommand::Safe,
                "/memory" => BotCommand::Memory,
                "/reset" => BotCommand::Reset,
                _ => BotCommand::Unknown(text.to_string()),
            };
        }

        // Natural-language matching (order matters: first match wins)
        if contains_any(&lower, &["dinero", "money", "activa", "start", "go", "activate"]) {
            BotCommand::Start
        } else if contains_any(&lower, &["para", "stop", "pausa"]) {
            BotCommand::Stop
        } else if contains_any(&lower, &["status", "estado"]) {
            BotCommand::Status
        } else if contains_any(&lower, &["stats", "estadística", "estadisticas"]) {
            BotCommand::Stats
        } else if contains_any(&lower, &["balance", "saldo"]) {
            BotCommand::Balance
        } else if contains_any(&lower, &["agresivo", "aggressive", "riesgo"]) {
            BotCommand::Aggressive
        } else if contains_any(&lower, &["seguro", "safe", "conserva"]) {
            BotCommand::Safe
        } else if contains_any(&lower, &["memoria", "memory", "aprendido", "sabes"]) {
            BotCommand::Memory
        } else if contains_any(&lower, &["reset", "borrar"]) {
            BotCommand::Reset
        } else {
            BotCommand::Unknown(text.to_string())
        }
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

// ---------------------------------------------------------------------------
// TelegramBot
// ---------------------------------------------------------------------------

pub struct TelegramBot {
    client: Client,
    token: String,
    chat_id: String,
    last_update_id: i64,
    enabled: bool,
    message_count: u64,
    last_message_time: u64,
}

impl TelegramBot {
    /// Create a new `TelegramBot`.
    ///
    /// If `token` or `chat_id` is `None` the bot is created in disabled mode
    /// and all outgoing calls become no-ops.
    pub fn new(token: Option<String>, chat_id: Option<String>) -> Self {
        let enabled = token.is_some() && chat_id.is_some();
        if !enabled {
            utils::log_warning("Telegram bot deshabilitado (token o chat_id no configurados)");
        }
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            token: token.unwrap_or_default(),
            chat_id: chat_id.unwrap_or_default(),
            last_update_id: 0,
            enabled,
            message_count: 0,
            last_message_time: 0,
        }
    }

    // -- helpers ----------------------------------------------------------

    fn base_url(&self) -> String {
        format!("https://api.telegram.org/bot{}", self.token)
    }

    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Returns `true` if we have already sent too many messages this minute.
    fn is_rate_limited(&self) -> bool {
        let now = Self::now_secs();
        if now - self.last_message_time >= 60 {
            // New minute window – not limited.
            return false;
        }
        self.message_count >= 20
    }

    fn reset_rate_window_if_needed(&mut self) {
        let now = Self::now_secs();
        if now - self.last_message_time >= 60 {
            self.message_count = 0;
            self.last_message_time = now;
        }
    }

    // -- public API -------------------------------------------------------

    /// Send a regular message.  Silently drops the message when the bot is
    /// disabled or rate-limited (> 20 msgs / min).
    pub async fn send_message(&mut self, text: &str) {
        if !self.enabled {
            return;
        }

        self.reset_rate_window_if_needed();

        if self.is_rate_limited() {
            utils::log_warning("Telegram rate-limit: mensaje descartado");
            return;
        }

        self.do_send(text).await;
        self.message_count += 1;
        if self.last_message_time == 0 {
            self.last_message_time = Self::now_secs();
        }
    }

    /// Send an alert that bypasses the rate limiter (for critical messages).
    pub async fn send_alert(&mut self, text: &str) {
        if !self.enabled {
            return;
        }
        self.do_send(text).await;
    }

    /// Perform the actual HTTP POST to the Telegram `sendMessage` endpoint.
    async fn do_send(&self, text: &str) {
        let url = format!("{}/sendMessage", self.base_url());
        let body = SendMessageBody {
            chat_id: &self.chat_id,
            text,
            parse_mode: "HTML",
        };

        match self
            .client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) => {
                if !resp.status().is_success() {
                    utils::log_error(&format!(
                        "Telegram sendMessage HTTP {}: {}",
                        resp.status(),
                        resp.text().await.unwrap_or_default()
                    ));
                }
            }
            Err(e) => {
                utils::log_error(&format!("Telegram sendMessage error: {}", e));
            }
        }
    }

    /// Poll for new commands from the Telegram chat.
    ///
    /// Uses long-polling with `timeout=1` so it returns almost immediately
    /// and never blocks the main trading loop.
    pub async fn poll_commands(&mut self) -> Vec<BotCommand> {
        if !self.enabled {
            return Vec::new();
        }

        let url = format!(
            "{}/getUpdates?offset={}&timeout=1",
            self.base_url(),
            self.last_update_id + 1
        );

        let resp = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                utils::log_error(&format!("Telegram getUpdates error: {}", e));
                return Vec::new();
            }
        };

        let body: TelegramResponse<Vec<TelegramUpdate>> = match resp.json().await {
            Ok(b) => b,
            Err(e) => {
                utils::log_error(&format!("Telegram getUpdates parse error: {}", e));
                return Vec::new();
            }
        };

        let updates = match body.result {
            Some(u) => u,
            None => return Vec::new(),
        };

        let mut commands = Vec::new();

        for update in &updates {
            // Track offset so we don't re-process this update.
            if update.update_id > self.last_update_id {
                self.last_update_id = update.update_id;
            }

            // Only process text messages from the configured chat.
            if let Some(ref msg) = update.message {
                if msg.chat.id.to_string() != self.chat_id {
                    continue;
                }
                if let Some(ref text) = msg.text {
                    let cmd = BotCommand::parse(text);
                    utils::log_info(&format!("Telegram comando recibido: {:?}", cmd));
                    commands.push(cmd);
                }
            }
        }

        commands
    }

    // -- notification helpers ---------------------------------------------

    pub async fn notify_started(&mut self, wallet: &str, balance: f64) {
        let text = format!(
            "<b>Agente iniciado</b>\n\
             Wallet: <code>{}</code>\n\
             Balance: <b>{:.4} SOL</b>\n\
             Modo: 24/7 autonomo",
            wallet, balance
        );
        self.send_alert(&text).await;
    }

    pub async fn notify_opportunity(&mut self, strategy: &str, profit: f64, details: &str) {
        let text = format!(
            "<b>Oportunidad detectada</b>\n\
             Estrategia: {}\n\
             Profit estimado: <b>${:.4}</b>\n\
             {}",
            strategy, profit, details
        );
        self.send_message(&text).await;
    }

    pub async fn notify_execution(&mut self, bundle_id: &str, profit: f64) {
        let text = format!(
            "<b>Bundle ejecutado</b>\n\
             ID: <code>{}</code>\n\
             Profit: <b>${:.4}</b>",
            bundle_id, profit
        );
        self.send_alert(&text).await;
    }

    pub async fn notify_error(&mut self, error: &str) {
        let text = format!("<b>Error critico</b>\n<code>{}</code>", error);
        self.send_alert(&text).await;
    }

    pub async fn notify_summary(&mut self, summary: &str) {
        let text = format!("<b>Resumen (30 min)</b>\n{}", summary);
        self.send_message(&text).await;
    }

    pub async fn notify_learning(&mut self, what_learned: &str) {
        let text = format!("<b>Aprendizaje</b>\n{}", what_learned);
        self.send_message(&text).await;
    }
}
