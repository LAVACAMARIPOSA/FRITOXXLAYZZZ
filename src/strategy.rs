use crate::memory::{AgentMemory, RiskLevel};

/// Action the engine recommends.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Go,
    Skip,
}

/// Full decision returned by the strategy engine.
#[derive(Debug, Clone)]
pub struct StrategyDecision {
    pub action: Action,
    pub confidence: f64,
    pub adjusted_profit: f64,
    pub reason: String,
}

/// Stateless strategy engine that consults the agent memory to adapt decisions.
pub struct StrategyEngine;

impl StrategyEngine {
    pub fn new() -> Self {
        Self
    }

    // ── Arbitrage ────────────────────────────────────────────────────────

    /// Evaluate whether an arbitrage opportunity is worth executing.
    ///
    /// * `memory`           – current agent memory / state
    /// * `estimated_profit` – expected profit in USD
    /// * `congestion`       – network congestion estimate in `[0.0, 1.0]`
    pub fn evaluate_arbitrage(
        &self,
        memory: &AgentMemory,
        estimated_profit: f64,
        congestion: f64,
    ) -> StrategyDecision {
        let risk = memory.risk_level;

        // Minimum profit thresholds per risk level (USD).
        let min_profit = match risk {
            RiskLevel::Safe => 2.0,
            RiskLevel::Normal => 0.8,
            RiskLevel::Aggressive => 0.3,
        };

        // Congestion ceiling – skip when network is too busy for our risk
        // appetite.
        let max_congestion = match risk {
            RiskLevel::Safe => 0.5,
            RiskLevel::Normal => 0.75,
            RiskLevel::Aggressive => 0.95,
        };

        if congestion > max_congestion {
            return StrategyDecision {
                action: Action::Skip,
                confidence: 0.9,
                adjusted_profit: estimated_profit,
                reason: format!(
                    "Congestion {:.0}% exceeds {:.0}% ceiling for {:?} risk",
                    congestion * 100.0,
                    max_congestion * 100.0,
                    risk
                ),
            };
        }

        // Adjust profit downward when congestion is high (more likely to
        // fail or pay higher priority fees).
        let congestion_penalty = 1.0 - (congestion * 0.5);
        let adjusted_profit = estimated_profit * congestion_penalty;

        if adjusted_profit < min_profit {
            return StrategyDecision {
                action: Action::Skip,
                confidence: 0.8,
                adjusted_profit,
                reason: format!(
                    "Adjusted profit ${:.4} below ${:.2} minimum for {:?} risk",
                    adjusted_profit, min_profit, risk
                ),
            };
        }

        // Confidence scales with how far above the minimum we are.
        let confidence = ((adjusted_profit / min_profit).min(3.0) / 3.0)
            .clamp(0.0, 1.0);

        StrategyDecision {
            action: Action::Go,
            confidence,
            adjusted_profit,
            reason: format!(
                "Arbitrage viable: ${:.4} adjusted profit (min ${:.2}, {:?} risk, {:.0}% congestion)",
                adjusted_profit, min_profit, risk, congestion * 100.0
            ),
        }
    }

    // ── Liquidation ─────────────────────────────────────────────────────

    /// Evaluate whether a liquidation opportunity is worth pursuing.
    ///
    /// * `memory`        – current agent memory / state
    /// * `health_factor` – borrower health factor (< 1.0 means liquidatable)
    /// * `profit`        – estimated liquidation profit in USD
    /// * `debt_usd`      – total debt to repay in USD
    pub fn evaluate_liquidation(
        &self,
        memory: &AgentMemory,
        health_factor: f64,
        profit: f64,
        debt_usd: f64,
    ) -> StrategyDecision {
        let risk = memory.risk_level;

        // Health-factor ceiling – only liquidate positions that are clearly
        // under-water enough for our risk tolerance.
        let max_hf = match risk {
            RiskLevel::Safe => 0.85,
            RiskLevel::Normal => 0.95,
            RiskLevel::Aggressive => 1.0,
        };

        if health_factor > max_hf {
            return StrategyDecision {
                action: Action::Skip,
                confidence: 0.85,
                adjusted_profit: profit,
                reason: format!(
                    "Health factor {:.4} above {:.2} ceiling for {:?} risk",
                    health_factor, max_hf, risk
                ),
            };
        }

        // Maximum debt we are willing to repay in a single flash loan.
        let max_debt = match risk {
            RiskLevel::Safe => 5_000.0,
            RiskLevel::Normal => 25_000.0,
            RiskLevel::Aggressive => 100_000.0,
        };

        if debt_usd > max_debt {
            return StrategyDecision {
                action: Action::Skip,
                confidence: 0.75,
                adjusted_profit: profit,
                reason: format!(
                    "Debt ${:.0} exceeds ${:.0} cap for {:?} risk",
                    debt_usd, max_debt, risk
                ),
            };
        }

        // Minimum profit percentage relative to the debt.
        let min_profit_pct = match risk {
            RiskLevel::Safe => 0.02,
            RiskLevel::Normal => 0.008,
            RiskLevel::Aggressive => 0.003,
        };

        let profit_pct = if debt_usd > 0.0 { profit / debt_usd } else { 0.0 };

        if profit_pct < min_profit_pct {
            return StrategyDecision {
                action: Action::Skip,
                confidence: 0.7,
                adjusted_profit: profit,
                reason: format!(
                    "Profit ratio {:.4}% below {:.3}% minimum for {:?} risk",
                    profit_pct * 100.0,
                    min_profit_pct * 100.0,
                    risk
                ),
            };
        }

        let confidence = ((profit_pct / min_profit_pct).min(5.0) / 5.0)
            .clamp(0.0, 1.0);

        StrategyDecision {
            action: Action::Go,
            confidence,
            adjusted_profit: profit,
            reason: format!(
                "Liquidation viable: HF {:.4}, profit ${:.2} ({:.3}% of ${:.0} debt), {:?} risk",
                health_factor, profit, profit_pct * 100.0, debt_usd, risk
            ),
        }
    }

    // ── Parameter helpers ───────────────────────────────────────────────

    /// Flash-loan amount in USDC base units (6 decimals).
    pub fn get_flash_amount(&self, memory: &AgentMemory) -> u64 {
        match memory.risk_level {
            RiskLevel::Safe => 250_000_000,       // 250 USDC
            RiskLevel::Normal => 500_000_000,     // 500 USDC
            RiskLevel::Aggressive => 1_000_000_000, // 1 000 USDC
        }
    }

    /// Maximum tolerated slippage in basis points.
    pub fn get_slippage_bps(&self, memory: &AgentMemory) -> u16 {
        match memory.risk_level {
            RiskLevel::Safe => 30,
            RiskLevel::Normal => 50,
            RiskLevel::Aggressive => 100,
        }
    }

    /// Jito tip in lamports, scaled by estimated profit and congestion.
    ///
    /// * `profit_lamports` – expected profit denominated in lamports
    /// * `congestion`      – network congestion in `[0.0, 1.0]`
    pub fn get_tip_lamports(
        &self,
        memory: &AgentMemory,
        profit_lamports: u64,
        congestion: f64,
    ) -> u64 {
        let base_pct = match memory.risk_level {
            RiskLevel::Safe => 0.05,
            RiskLevel::Normal => 0.10,
            RiskLevel::Aggressive => 0.20,
        };

        // Increase the tip when congestion is high so bundles land faster.
        let congestion_multiplier = 1.0 + congestion;

        let tip = (profit_lamports as f64 * base_pct * congestion_multiplier) as u64;

        // Floor: at least 10_000 lamports (0.00001 SOL).
        tip.max(10_000)
    }

    /// Min / max USD range for scanning liquidation candidates.
    pub fn get_scan_range(&self, memory: &AgentMemory) -> (f64, f64) {
        match memory.risk_level {
            RiskLevel::Safe => (100.0, 5_000.0),
            RiskLevel::Normal => (50.0, 25_000.0),
            RiskLevel::Aggressive => (10.0, 100_000.0),
        }
    }

    /// Whether to scan for liquidation opportunities this cycle.
    pub fn should_scan_liquidations(&self, _memory: &AgentMemory) -> bool {
        true // always scan
    }

    /// Seconds to sleep between main-loop cycles.
    pub fn get_cycle_delay_secs(&self, memory: &AgentMemory) -> u64 {
        match memory.risk_level {
            RiskLevel::Safe => 10,
            RiskLevel::Normal => 5,
            RiskLevel::Aggressive => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal stub so unit tests compile without the full memory crate.
    // In production the real AgentMemory is used.
    #[test]
    fn action_enum_equality() {
        assert_eq!(Action::Go, Action::Go);
        assert_ne!(Action::Go, Action::Skip);
    }
}
