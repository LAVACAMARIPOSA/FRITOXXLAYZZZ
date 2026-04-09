use reqwest::Client;
use serde::{Deserialize, Serialize};

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
// Groq API structures
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct GroqRequest {
    model: String,
    messages: Vec<GroqMessage>,
    max_tokens: u32,
    temperature: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroqMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GroqResponse {
    choices: Option<Vec<GroqChoice>>,
    error: Option<GroqError>,
}

#[derive(Debug, Deserialize)]
struct GroqChoice {
    message: GroqMessage,
}

#[derive(Debug, Deserialize)]
struct GroqError {
    message: String,
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
    Help,
    Unknown(String),
}

impl BotCommand {
    pub fn parse(text: &str) -> Self {
        let lower = text.trim().to_lowercase();

        // Slash-commands
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
                "/help" | "/ayuda" => BotCommand::Help,
                _ => BotCommand::Unknown(text.to_string()),
            };
        }

        // Natural language matching (español + inglés)
        if contains_any(&lower, &[
            "dinero", "money", "activa", "activate", "go", "vamos",
            "a darle", "ponte a trabajar", "arranca", "enciende",
        ]) {
            BotCommand::Start
        } else if contains_any(&lower, &[
            "stop", "pausa", "descansa", "relax", "espera", "detente", "quieto",
        ]) {
            BotCommand::Stop
        } else if contains_any(&lower, &[
            "status", "estado", "como va", "cómo va", "todo bien",
            "funciona", "estas bien", "estás bien", "que tal", "qué tal",
        ]) {
            BotCommand::Status
        } else if contains_any(&lower, &[
            "stats", "estadística", "estadisticas", "estadísticas", "numeros", "números",
        ]) {
            BotCommand::Stats
        } else if contains_any(&lower, &["balance", "saldo", "cuanto tengo", "cuánto tengo"]) {
            BotCommand::Balance
        } else if contains_any(&lower, &[
            "agresivo", "aggressive", "más riesgo", "mas riesgo",
            "dale con todo", "full", "a tope",
        ]) {
            BotCommand::Aggressive
        } else if contains_any(&lower, &[
            "seguro", "safe", "conserva", "con calma", "tranquilo", "despacio",
            "menos riesgo",
        ]) {
            BotCommand::Safe
        } else if contains_any(&lower, &[
            "memoria", "memory", "aprendido", "sabes", "inteligencia",
            "que has aprendido", "qué has aprendido",
        ]) {
            BotCommand::Memory
        } else if contains_any(&lower, &["reset", "borrar", "reiniciar"]) {
            BotCommand::Reset
        } else if contains_any(&lower, &[
            "ayuda", "help", "comandos", "que puedes", "qué puedes",
        ]) {
            BotCommand::Help
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
    groq_api_key: Option<String>,
}

impl TelegramBot {
    pub fn new(token: Option<String>, chat_id: Option<String>) -> Self {
        let enabled = token.is_some() && chat_id.is_some();
        if !enabled {
            utils::log_warning("Telegram bot deshabilitado (token o chat_id no configurados)");
        }

        let groq_key = std::env::var("GROQ_API_KEY").ok();
        if groq_key.is_some() {
            utils::log_success("Groq AI habilitado para chat conversacional");
        }

        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            token: token.unwrap_or_default(),
            chat_id: chat_id.unwrap_or_default(),
            last_update_id: 0,
            enabled,
            message_count: 0,
            last_message_time: 0,
            groq_api_key: groq_key,
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

    fn is_rate_limited(&self) -> bool {
        let now = Self::now_secs();
        if now - self.last_message_time >= 60 {
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

    pub async fn send_message(&mut self, text: &str) {
        if !self.enabled { return; }
        self.reset_rate_window_if_needed();
        if self.is_rate_limited() { return; }
        self.do_send(text).await;
        self.message_count += 1;
        if self.last_message_time == 0 {
            self.last_message_time = Self::now_secs();
        }
    }

    pub async fn send_alert(&mut self, text: &str) {
        if !self.enabled { return; }
        self.do_send(text).await;
    }

    async fn do_send(&self, text: &str) {
        let url = format!("{}/sendMessage", self.base_url());
        let body = SendMessageBody {
            chat_id: &self.chat_id,
            text,
            parse_mode: "HTML",
        };
        match self.client.post(&url).json(&body)
            .timeout(std::time::Duration::from_secs(10))
            .send().await
        {
            Ok(resp) => {
                if !resp.status().is_success() {
                    // Try again without HTML parse mode (in case of formatting errors)
                    let plain_body = serde_json::json!({
                        "chat_id": &self.chat_id,
                        "text": text,
                    });
                    let _ = self.client.post(&url).json(&plain_body)
                        .timeout(std::time::Duration::from_secs(10))
                        .send().await;
                }
            }
            Err(e) => {
                utils::log_error(&format!("Telegram error: {}", e));
            }
        }
    }

    pub async fn poll_commands(&mut self) -> Vec<BotCommand> {
        if !self.enabled { return Vec::new(); }

        let url = format!(
            "{}/getUpdates?offset={}&timeout=1",
            self.base_url(), self.last_update_id + 1
        );

        let resp = match self.client.get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send().await
        {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        let body: TelegramResponse<Vec<TelegramUpdate>> = match resp.json().await {
            Ok(b) => b,
            Err(_) => return Vec::new(),
        };

        let updates = match body.result {
            Some(u) => u,
            None => return Vec::new(),
        };

        let mut commands = Vec::new();
        for update in &updates {
            if update.update_id > self.last_update_id {
                self.last_update_id = update.update_id;
            }
            if let Some(ref msg) = update.message {
                if msg.chat.id.to_string() != self.chat_id { continue; }
                if let Some(ref text) = msg.text {
                    let cmd = BotCommand::parse(text);
                    utils::log_info(&format!("Telegram: {:?}", cmd));
                    commands.push(cmd);
                }
            }
        }
        commands
    }

    // -- Groq AI chat -----------------------------------------------------

    /// Ask Groq AI to generate a conversational response.
    /// The agent_context string contains the current state of the bot.
    /// Returns the AI response, and optionally a BotCommand if the AI
    /// decided the user wants to execute an action.
    pub async fn ask_ai(&self, user_message: &str, agent_context: &str) -> (String, Option<BotCommand>) {
        let api_key = match &self.groq_api_key {
            Some(k) => k,
            None => {
                return (
                    "No entendi. Comandos: /status /stats /memory /balance /aggressive /safe /stop /start /help".to_string(),
                    None,
                );
            }
        };

        let system_prompt = format!(
            "Eres ELCOQUI, un agente autonomo de trading DeFi en Solana. \
             Respondes en espanol, breve y directo (max 3-4 lineas). \
             Usas un tono informal y amigable.\n\n\
             Estado actual del agente:\n{}\n\n\
             Si el usuario quiere que hagas algo, incluye EXACTAMENTE uno de estos \
             tags al final de tu respuesta (solo si aplica):\n\
             [CMD:START] - activar el agente\n\
             [CMD:STOP] - pausar el agente\n\
             [CMD:AGGRESSIVE] - modo agresivo\n\
             [CMD:SAFE] - modo seguro\n\
             [CMD:RESET] - borrar memoria\n\
             Solo incluye el tag si el usuario claramente pide esa accion.",
            agent_context
        );

        let request = GroqRequest {
            model: "llama-3.3-70b-versatile".to_string(),
            messages: vec![
                GroqMessage { role: "system".to_string(), content: system_prompt },
                GroqMessage { role: "user".to_string(), content: user_message.to_string() },
            ],
            max_tokens: 300,
            temperature: 0.7,
        };

        let resp = match self.client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                utils::log_error(&format!("Groq API error: {}", e));
                return ("Error conectando con IA. Intenta /status o /help".to_string(), None);
            }
        };

        let groq_resp: GroqResponse = match resp.json().await {
            Ok(r) => r,
            Err(e) => {
                utils::log_error(&format!("Groq parse error: {}", e));
                return ("Error procesando respuesta IA.".to_string(), None);
            }
        };

        if let Some(ref err) = groq_resp.error {
            utils::log_error(&format!("Groq error: {}", err.message));
            return ("IA no disponible ahora. Usa /status o /help".to_string(), None);
        }

        let ai_text = groq_resp.choices
            .and_then(|c| c.into_iter().next())
            .map(|c| c.message.content)
            .unwrap_or_else(|| "Sin respuesta de IA.".to_string());

        // Extract command tag if present
        let cmd = if ai_text.contains("[CMD:START]") {
            Some(BotCommand::Start)
        } else if ai_text.contains("[CMD:STOP]") {
            Some(BotCommand::Stop)
        } else if ai_text.contains("[CMD:AGGRESSIVE]") {
            Some(BotCommand::Aggressive)
        } else if ai_text.contains("[CMD:SAFE]") {
            Some(BotCommand::Safe)
        } else if ai_text.contains("[CMD:RESET]") {
            Some(BotCommand::Reset)
        } else {
            None
        };

        // Clean the response (remove command tags)
        let clean_text = ai_text
            .replace("[CMD:START]", "")
            .replace("[CMD:STOP]", "")
            .replace("[CMD:AGGRESSIVE]", "")
            .replace("[CMD:SAFE]", "")
            .replace("[CMD:RESET]", "")
            .trim()
            .to_string();

        (clean_text, cmd)
    }

    /// Handle an unknown message: ask Groq AI and reply.
    /// Returns an optional BotCommand if the AI detected user intent.
    pub async fn handle_unknown_with_ai(
        &mut self,
        user_message: &str,
        agent_context: &str,
    ) -> Option<BotCommand> {
        let (response, cmd) = self.ask_ai(user_message, agent_context).await;
        self.send_message(&response).await;
        cmd
    }

    // -- notification helpers ---------------------------------------------

    pub async fn notify_started(&mut self, wallet: &str, balance: f64) {
        self.send_alert(&format!(
            "<b>Agente iniciado</b>\nWallet: <code>{}</code>\nBalance: <b>{:.4} SOL</b>\nModo: 24/7 autonomo\n\nHablame! Entiendo espanol.",
            wallet, balance
        )).await;
    }

    pub async fn notify_opportunity(&mut self, strategy: &str, profit: f64, details: &str) {
        self.send_message(&format!(
            "<b>Oportunidad detectada</b>\nEstrategia: {}\nProfit: <b>${:.4}</b>\n{}",
            strategy, profit, details
        )).await;
    }

    pub async fn notify_execution(&mut self, bundle_id: &str, profit: f64) {
        self.send_alert(&format!(
            "<b>Bundle ejecutado</b>\nID: <code>{}</code>\nProfit: <b>${:.4}</b>",
            bundle_id, profit
        )).await;
    }

    pub async fn notify_error(&mut self, error: &str) {
        self.send_alert(&format!("<b>Error</b>\n<code>{}</code>", error)).await;
    }

    pub async fn notify_summary(&mut self, summary: &str) {
        self.send_message(&format!("<b>Resumen</b>\n{}", summary)).await;
    }

    pub async fn notify_learning(&mut self, what_learned: &str) {
        self.send_message(&format!("<b>Aprendizaje</b>\n{}", what_learned)).await;
    }

    pub fn help_text() -> String {
        "<b>Comandos ELCOQUI:</b>\n\
        /start - Activar agente\n\
        /stop - Pausar agente\n\
        /status - Estado actual\n\
        /stats - Estadisticas detalladas\n\
        /balance - Balance SOL\n\
        /memory - Que he aprendido\n\
        /aggressive - Modo agresivo\n\
        /safe - Modo seguro\n\
        /reset - Borrar memoria\n\
        /help - Esta ayuda\n\n\
        O simplemente hablame normal!".to_string()
    }
}
