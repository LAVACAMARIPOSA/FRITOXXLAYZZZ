use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Safe,
    Normal,
    Aggressive,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Safe => write!(f, "Safe"),
            RiskLevel::Normal => write!(f, "Normal"),
            RiskLevel::Aggressive => write!(f, "Aggressive"),
        }
    }
}

// ---------------------------------------------------------------------------
// Data structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunityRecord {
    pub timestamp: u64,
    pub strategy: String,
    pub route: String,
    pub estimated_profit_usd: f64,
    pub actual_profit_usd: f64,
    pub success: bool,
    pub congestion_level: f64,
    pub tip_lamports: u64,
    pub hour_utc: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteScore {
    pub total_seen: u64,
    pub profitable_count: u64,
    pub avg_profit: f64,
    pub last_seen: u64,
}

impl Default for RouteScore {
    fn default() -> Self {
        Self {
            total_seen: 0,
            profitable_count: 0,
            avg_profit: 0.0,
            last_seen: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Agent Memory
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemory {
    // History
    pub history: Vec<OpportunityRecord>,
    pub max_history: usize,

    // Route scoring
    pub route_scores: HashMap<String, RouteScore>,

    // Hourly stats: hour (0-23) -> (attempts, successes, total_profit)
    pub hourly_stats: HashMap<u8, (u64, u64, f64)>,

    // Congestion patterns: bucket (0-10) -> (attempts, successes)
    pub congestion_patterns: HashMap<u8, (u64, u64)>,

    // Streaks
    pub current_win_streak: u32,
    pub current_loss_streak: u32,
    pub best_win_streak: u32,
    pub worst_loss_streak: u32,

    // Cumulative stats
    pub total_opportunities: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub total_profit_usd: f64,
    pub total_loss_usd: f64,
    pub total_cycles: u64,

    // Adaptive parameters
    pub min_profit_usd: f64,
    pub flash_loan_amount_lamports: u64,
    pub tip_pct: f64,
    pub risk_level: RiskLevel,

    // Session info
    pub session_start: u64,
    pub last_updated: u64,
    pub version: u32,

    // Scan metrics (not persisted, reset each session)
    #[serde(skip)]
    pub scan_quotes_ok: u64,
    #[serde(skip)]
    pub scan_quotes_failed: u64,
    #[serde(skip)]
    pub scan_near_misses: u64,
    #[serde(skip)]
    pub scan_best_spread_pct: f64,
    #[serde(skip)]
    pub scan_best_spread_route: String,

    // Liquidation scan metrics
    #[serde(skip)]
    pub liq_scans_total: u64,
    #[serde(skip)]
    pub liq_scans_failed: u64,
    #[serde(skip)]
    pub liq_obligations_seen: u64,
    #[serde(skip)]
    pub liq_with_debt_seen: u64,
    #[serde(skip)]
    pub liq_in_range_seen: u64,
}

const MEMORY_PATH: &str = "data/memory.json";

impl AgentMemory {
    // -----------------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------------
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_history: 5000,

            route_scores: HashMap::new(),
            hourly_stats: HashMap::new(),
            congestion_patterns: HashMap::new(),

            current_win_streak: 0,
            current_loss_streak: 0,
            best_win_streak: 0,
            worst_loss_streak: 0,

            total_opportunities: 0,
            total_successes: 0,
            total_failures: 0,
            total_profit_usd: 0.0,
            total_loss_usd: 0.0,
            total_cycles: 0,

            min_profit_usd: 0.15,
            flash_loan_amount_lamports: 500_000_000, // 0.5 SOL
            tip_pct: 0.80,
            risk_level: RiskLevel::Normal,

            session_start: current_timestamp(),
            last_updated: current_timestamp(),
            version: 1,

            scan_quotes_ok: 0,
            scan_quotes_failed: 0,
            scan_near_misses: 0,
            scan_best_spread_pct: f64::NEG_INFINITY,
            scan_best_spread_route: String::new(),

            liq_scans_total: 0,
            liq_scans_failed: 0,
            liq_obligations_seen: 0,
            liq_with_debt_seen: 0,
            liq_in_range_seen: 0,
        }
    }

    /// Record a successful Jupiter quote
    pub fn record_quote_ok(&mut self) {
        self.scan_quotes_ok += 1;
    }

    /// Record a failed Jupiter quote
    pub fn record_quote_failed(&mut self) {
        self.scan_quotes_failed += 1;
    }

    /// Record a scan result (spread as percentage, e.g., -0.3 means 0.3% loss)
    pub fn record_scan_spread(&mut self, spread_pct: f64, route: &str) {
        if spread_pct > self.scan_best_spread_pct {
            self.scan_best_spread_pct = spread_pct;
            self.scan_best_spread_route = route.to_string();
        }
        // A "near miss" is a spread between -0.5% and 0% (close to profitable)
        if spread_pct > -0.5 && spread_pct <= 0.0 {
            self.scan_near_misses += 1;
        }
    }

    /// Record liquidation scan result
    pub fn record_liq_scan(&mut self, fetched: usize, with_debt: usize, in_range: usize, error: bool) {
        self.liq_scans_total += 1;
        if error {
            self.liq_scans_failed += 1;
        } else {
            self.liq_obligations_seen += fetched as u64;
            self.liq_with_debt_seen += with_debt as u64;
            self.liq_in_range_seen += in_range as u64;
        }
    }

    /// Get scan metrics summary for Telegram
    pub fn scan_summary(&self) -> String {
        let total_quotes = self.scan_quotes_ok + self.scan_quotes_failed;
        let success_rate = if total_quotes > 0 {
            self.scan_quotes_ok as f64 / total_quotes as f64 * 100.0
        } else {
            0.0
        };
        let best_spread = if self.scan_best_spread_pct > f64::NEG_INFINITY {
            format!("{:+.3}%", self.scan_best_spread_pct)
        } else {
            "N/A".to_string()
        };
        let best_route = if self.scan_best_spread_route.is_empty() {
            "N/A".to_string()
        } else {
            self.scan_best_spread_route.clone()
        };

        let liq_ok = self.liq_scans_total - self.liq_scans_failed;
        let liq_info = if self.liq_scans_total > 0 {
            format!(
                "\nLiq scans: {}/{} OK\nObligaciones: {} vistas, {} con deuda, {} en rango",
                liq_ok, self.liq_scans_total,
                self.liq_obligations_seen, self.liq_with_debt_seen, self.liq_in_range_seen
            )
        } else {
            "\nLiq scans: pendiente".to_string()
        };

        format!(
            "Arb quotes: {}/{} OK ({:.0}%)\nNear-misses: {}\nMejor spread: {} ({}){}",
            self.scan_quotes_ok, total_quotes, success_rate,
            self.scan_near_misses, best_spread, best_route,
            liq_info
        )
    }

    // -----------------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------------
    pub fn load() -> Self {
        match std::fs::read_to_string(MEMORY_PATH) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse memory file, starting fresh: {}", e);
                Self::new()
            }),
            Err(_) => {
                tracing::info!("No memory file found, starting fresh");
                Self::new()
            }
        }
    }

    pub fn save(&self) {
        if let Some(parent) = std::path::Path::new(MEMORY_PATH).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(MEMORY_PATH, json) {
                    tracing::error!("Failed to save memory: {}", e);
                }
            }
            Err(e) => tracing::error!("Failed to serialize memory: {}", e),
        }
    }

    // -----------------------------------------------------------------------
    // Recording
    // -----------------------------------------------------------------------
    pub fn record_opportunity(&mut self, record: OpportunityRecord) {
        // Update streaks
        if record.success {
            self.total_successes += 1;
            self.total_profit_usd += record.actual_profit_usd;
            self.current_win_streak += 1;
            self.current_loss_streak = 0;
            if self.current_win_streak > self.best_win_streak {
                self.best_win_streak = self.current_win_streak;
            }
        } else {
            self.total_failures += 1;
            self.total_loss_usd += record.actual_profit_usd.abs();
            self.current_loss_streak += 1;
            self.current_win_streak = 0;
            if self.current_loss_streak > self.worst_loss_streak {
                self.worst_loss_streak = self.current_loss_streak;
            }
        }

        self.total_opportunities += 1;
        self.last_updated = current_timestamp();

        // Push to history, trim if needed
        self.history.push(record);
        if self.history.len() > self.max_history {
            let drain_count = self.history.len() - self.max_history;
            self.history.drain(0..drain_count);
        }

        // Auto-save after each opportunity (important data)
        self.save();
    }

    pub fn record_cycle(&mut self) {
        self.total_cycles += 1;
        self.last_updated = current_timestamp();
    }

    // -----------------------------------------------------------------------
    // Learning
    // -----------------------------------------------------------------------
    pub fn learn(&mut self) {
        // Rebuild route_scores from history
        self.route_scores.clear();
        for rec in &self.history {
            let entry = self.route_scores.entry(rec.route.clone()).or_default();
            entry.total_seen += 1;
            if rec.success && rec.actual_profit_usd > 0.0 {
                entry.profitable_count += 1;
            }
            // Running average
            entry.avg_profit = entry.avg_profit
                + (rec.actual_profit_usd - entry.avg_profit) / entry.total_seen as f64;
            if rec.timestamp > entry.last_seen {
                entry.last_seen = rec.timestamp;
            }
        }

        // Rebuild hourly stats
        self.hourly_stats.clear();
        for rec in &self.history {
            let entry = self.hourly_stats.entry(rec.hour_utc).or_insert((0, 0, 0.0));
            entry.0 += 1;
            if rec.success {
                entry.1 += 1;
            }
            entry.2 += rec.actual_profit_usd;
        }

        // Rebuild congestion patterns
        self.congestion_patterns.clear();
        for rec in &self.history {
            let bucket = (rec.congestion_level * 10.0).min(10.0) as u8;
            let entry = self.congestion_patterns.entry(bucket).or_insert((0, 0));
            entry.0 += 1;
            if rec.success {
                entry.1 += 1;
            }
        }

        // Auto-adjust risk level based on recent performance
        let recent_count = 50.min(self.history.len());
        if recent_count >= 10 {
            let recent = &self.history[self.history.len() - recent_count..];
            let recent_successes = recent.iter().filter(|r| r.success).count() as f64;
            let recent_rate = recent_successes / recent_count as f64;

            match self.risk_level {
                RiskLevel::Safe => {
                    // Conservative: raise min_profit when losing
                    if recent_rate < 0.3 {
                        self.min_profit_usd = (self.min_profit_usd * 1.1).min(2.0);
                    } else if recent_rate > 0.7 {
                        self.min_profit_usd = (self.min_profit_usd * 0.95).max(0.15);
                    }
                }
                RiskLevel::Normal => {
                    if recent_rate < 0.3 {
                        self.min_profit_usd = (self.min_profit_usd * 1.05).min(1.5);
                    } else if recent_rate > 0.6 {
                        self.min_profit_usd = (self.min_profit_usd * 0.97).max(0.10);
                    }
                }
                RiskLevel::Aggressive => {
                    if recent_rate < 0.2 {
                        self.min_profit_usd = (self.min_profit_usd * 1.02).min(1.0);
                    } else if recent_rate > 0.5 {
                        self.min_profit_usd = (self.min_profit_usd * 0.93).max(0.05);
                    }
                }
            }

            // Auto-adjust tip percentage based on success rate
            if recent_rate < 0.4 {
                self.tip_pct = (self.tip_pct + 0.02).min(0.95);
            } else if recent_rate > 0.7 {
                self.tip_pct = (self.tip_pct - 0.01).max(0.50);
            }
        }

        self.last_updated = current_timestamp();
        self.save();
    }

    // -----------------------------------------------------------------------
    // Suggestions
    // -----------------------------------------------------------------------
    pub fn suggest_flash_amount(&self) -> u64 {
        let base = self.flash_loan_amount_lamports;
        match self.risk_level {
            RiskLevel::Safe => base / 2,
            RiskLevel::Normal => base,
            RiskLevel::Aggressive => base * 2,
        }
    }

    pub fn suggest_min_profit(&self) -> f64 {
        self.min_profit_usd
    }

    pub fn suggest_tip_pct(&self) -> f64 {
        self.tip_pct
    }

    // -----------------------------------------------------------------------
    // Analytics
    // -----------------------------------------------------------------------
    pub fn best_hours(&self) -> Vec<(u8, f64)> {
        let mut hours: Vec<(u8, f64)> = self
            .hourly_stats
            .iter()
            .filter(|(_, (attempts, _, _))| *attempts >= 5)
            .map(|(hour, (attempts, successes, _))| {
                (*hour, *successes as f64 / *attempts as f64)
            })
            .collect();
        hours.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        hours.truncate(5);
        hours
    }

    pub fn best_routes(&self) -> Vec<(String, f64, u64)> {
        let mut routes: Vec<(String, f64, u64)> = self
            .route_scores
            .iter()
            .filter(|(_, s)| s.total_seen >= 3)
            .map(|(name, s)| (name.clone(), s.avg_profit, s.profitable_count))
            .collect();
        routes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        routes.truncate(10);
        routes
    }

    // -----------------------------------------------------------------------
    // Summary (Telegram-friendly with emojis)
    // -----------------------------------------------------------------------
    pub fn summary(&self) -> String {
        let win_rate = if self.total_opportunities > 0 {
            self.total_successes as f64 / self.total_opportunities as f64 * 100.0
        } else {
            0.0
        };
        let net_pnl = self.total_profit_usd - self.total_loss_usd;
        let uptime_secs = current_timestamp().saturating_sub(self.session_start);
        let uptime_hours = uptime_secs as f64 / 3600.0;

        let best_hrs = self.best_hours();
        let best_hours_str = if best_hrs.is_empty() {
            "N/A".to_string()
        } else {
            best_hrs
                .iter()
                .map(|(h, r)| format!("{:02}h ({:.0}%)", h, r * 100.0))
                .collect::<Vec<_>>()
                .join(", ")
        };

        format!(
            "\u{1F9E0} *Agent Memory Summary*\n\
             \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\n\
             \u{1F4CA} *Stats*\n\
             \u{2022} Opportunities: {}\n\
             \u{2022} Win rate: {:.1}%\n\
             \u{2022} Net P&L: ${:.2}\n\
             \u{2022} Cycles: {}\n\n\
             \u{1F525} *Streaks*\n\
             \u{2022} Current W/L: {}W / {}L\n\
             \u{2022} Best win: {} | Worst loss: {}\n\n\
             \u{2699}\u{FE0F} *Adaptive Params*\n\
             \u{2022} Min profit: ${:.2}\n\
             \u{2022} Tip: {:.0}%\n\
             \u{2022} Risk: {}\n\
             \u{2022} Flash: {} lamports\n\n\
             \u{1F550} *Best hours*: {}\n\
             \u{23F1} Uptime: {:.1}h",
            self.total_opportunities,
            win_rate,
            net_pnl,
            self.total_cycles,
            self.current_win_streak,
            self.current_loss_streak,
            self.best_win_streak,
            self.worst_loss_streak,
            self.min_profit_usd,
            self.tip_pct * 100.0,
            self.risk_level,
            self.suggest_flash_amount(),
            best_hours_str,
            uptime_hours,
        )
    }

    // -----------------------------------------------------------------------
    // Detailed stats
    // -----------------------------------------------------------------------
    pub fn detailed_stats(&self) -> String {
        let mut out = String::new();

        out.push_str("=== DETAILED AGENT STATS ===\n\n");

        // Overall
        let win_rate = if self.total_opportunities > 0 {
            self.total_successes as f64 / self.total_opportunities as f64 * 100.0
        } else {
            0.0
        };
        out.push_str(&format!(
            "Total opportunities: {}\nSuccesses: {} | Failures: {}\nWin rate: {:.1}%\n\
             Total profit: ${:.2} | Total loss: ${:.2}\nNet P&L: ${:.2}\nCycles: {}\n\n",
            self.total_opportunities,
            self.total_successes,
            self.total_failures,
            win_rate,
            self.total_profit_usd,
            self.total_loss_usd,
            self.total_profit_usd - self.total_loss_usd,
            self.total_cycles,
        ));

        // Streaks
        out.push_str(&format!(
            "Streaks: current {}W/{}L | best win {} | worst loss {}\n\n",
            self.current_win_streak,
            self.current_loss_streak,
            self.best_win_streak,
            self.worst_loss_streak,
        ));

        // Top routes
        out.push_str("--- Top Routes ---\n");
        let routes = self.best_routes();
        if routes.is_empty() {
            out.push_str("  No route data yet\n");
        } else {
            for (name, avg, wins) in &routes {
                out.push_str(&format!(
                    "  {} -> avg ${:.4}, {} profitable\n",
                    name, avg, wins
                ));
            }
        }
        out.push('\n');

        // Hourly performance
        out.push_str("--- Hourly Performance ---\n");
        let mut hours: Vec<_> = self.hourly_stats.iter().collect();
        hours.sort_by_key(|(h, _)| *h);
        for (hour, (attempts, successes, profit)) in &hours {
            let rate = if *attempts > 0 {
                *successes as f64 / *attempts as f64 * 100.0
            } else {
                0.0
            };
            out.push_str(&format!(
                "  {:02}h: {} attempts, {:.0}% success, ${:.2} profit\n",
                hour, attempts, rate, profit
            ));
        }
        out.push('\n');

        // Congestion
        out.push_str("--- Congestion Patterns ---\n");
        let mut cong: Vec<_> = self.congestion_patterns.iter().collect();
        cong.sort_by_key(|(b, _)| *b);
        for (bucket, (attempts, successes)) in &cong {
            let rate = if *attempts > 0 {
                *successes as f64 / *attempts as f64 * 100.0
            } else {
                0.0
            };
            out.push_str(&format!(
                "  congestion {:.1}: {} attempts, {:.0}% success\n",
                **bucket as f64 / 10.0,
                attempts,
                rate
            ));
        }
        out.push('\n');

        // Adaptive params
        out.push_str(&format!(
            "--- Adaptive Parameters ---\n\
             Min profit: ${:.2}\nFlash loan: {} lamports\nTip: {:.1}%\nRisk: {}\n",
            self.min_profit_usd,
            self.flash_loan_amount_lamports,
            self.tip_pct * 100.0,
            self.risk_level,
        ));

        out
    }

    // -----------------------------------------------------------------------
    // Reset & config
    // -----------------------------------------------------------------------
    pub fn reset(&mut self) {
        *self = Self::new();
        self.save();
    }

    pub fn set_risk_level(&mut self, level: RiskLevel) {
        self.risk_level = level;
        self.last_updated = current_timestamp();
        self.save();
    }
}

impl Default for AgentMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_defaults() {
        let mem = AgentMemory::new();
        assert_eq!(mem.min_profit_usd, 0.15);
        assert_eq!(mem.flash_loan_amount_lamports, 500_000_000);
        assert!((mem.tip_pct - 0.80).abs() < f64::EPSILON);
        assert_eq!(mem.risk_level, RiskLevel::Normal);
        assert_eq!(mem.total_opportunities, 0);
    }

    #[test]
    fn test_record_opportunity_win() {
        let mut mem = AgentMemory::new();
        let rec = OpportunityRecord {
            timestamp: current_timestamp(),
            strategy: "arb".into(),
            route: "SOL->USDC->SOL".into(),
            estimated_profit_usd: 1.0,
            actual_profit_usd: 0.9,
            success: true,
            congestion_level: 0.5,
            tip_lamports: 10000,
            hour_utc: 14,
        };
        mem.record_opportunity(rec);
        assert_eq!(mem.total_successes, 1);
        assert_eq!(mem.current_win_streak, 1);
        assert_eq!(mem.total_opportunities, 1);
    }

    #[test]
    fn test_record_opportunity_loss() {
        let mut mem = AgentMemory::new();
        let rec = OpportunityRecord {
            timestamp: current_timestamp(),
            strategy: "arb".into(),
            route: "SOL->USDC->SOL".into(),
            estimated_profit_usd: 1.0,
            actual_profit_usd: -0.5,
            success: false,
            congestion_level: 0.8,
            tip_lamports: 10000,
            hour_utc: 3,
        };
        mem.record_opportunity(rec);
        assert_eq!(mem.total_failures, 1);
        assert_eq!(mem.current_loss_streak, 1);
    }

    #[test]
    fn test_suggest_flash_amount_by_risk() {
        let mut mem = AgentMemory::new();
        assert_eq!(mem.suggest_flash_amount(), 500_000_000);

        mem.set_risk_level(RiskLevel::Safe);
        assert_eq!(mem.suggest_flash_amount(), 250_000_000);

        mem.risk_level = RiskLevel::Aggressive;
        assert_eq!(mem.suggest_flash_amount(), 1_000_000_000);
    }

    #[test]
    fn test_learn_builds_route_scores() {
        let mut mem = AgentMemory::new();
        for i in 0..5 {
            mem.record_opportunity(OpportunityRecord {
                timestamp: current_timestamp() + i,
                strategy: "arb".into(),
                route: "A->B->A".into(),
                estimated_profit_usd: 1.0,
                actual_profit_usd: 0.5,
                success: true,
                congestion_level: 0.3,
                tip_lamports: 5000,
                hour_utc: 10,
            });
        }
        mem.learn();
        assert!(mem.route_scores.contains_key("A->B->A"));
        let score = &mem.route_scores["A->B->A"];
        assert_eq!(score.total_seen, 5);
        assert_eq!(score.profitable_count, 5);
    }

    #[test]
    fn test_summary_does_not_panic() {
        let mem = AgentMemory::new();
        let s = mem.summary();
        assert!(s.contains("Agent Memory Summary"));
    }

    #[test]
    fn test_detailed_stats_does_not_panic() {
        let mem = AgentMemory::new();
        let s = mem.detailed_stats();
        assert!(s.contains("DETAILED AGENT STATS"));
    }

    #[test]
    fn test_reset() {
        let mut mem = AgentMemory::new();
        mem.total_opportunities = 999;
        mem.reset();
        assert_eq!(mem.total_opportunities, 0);
    }
}
