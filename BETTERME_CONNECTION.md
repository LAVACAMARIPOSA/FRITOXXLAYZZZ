# BETTERME — IA Connection Guide

## Quick Start: Connect AI to BETTERME

Any AI session (Claude, GPT, etc.) can monitor, control, and get real-time data from BETTERME.

### Step 1: Open BETTERME project
```bash
cd /Applications/BETTERME
claude
```

### Step 2: Just ask
```
"Show me the brain's predictions right now"
"How are the models performing?"
"Switch to REAL mode"
"Open the dashboard"
"How many spins has the brain seen?"
"What are the hot numbers?"
```

The AI reads the `CLAUDE.md` file automatically and knows how to:
- Fetch live predictions from the API
- Monitor model accuracy in real-time
- Control brain mode (PRACTICE/REAL)
- Open the dashboard in Chrome
- Check observer daemon status
- Force database snapshots

---

## Live API Endpoints

### Render (24/7 Cloud)
Base URL: `https://betterme-mydl.onrender.com`

### Local (when running)
Base URL: `http://localhost:5000`

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/brain` | GET | Full brain HUD data (predictions, models, hot/cold, dealer) |
| `/api/brain/predict` | GET | Current prediction (numbers, confidence, should_bet) |
| `/api/brain/models` | GET | Model performance stats (hit rates, weights) |
| `/api/brain/summary` | GET | Human-readable brain summary |
| `/api/brain/config` | GET/POST | Get or update brain configuration |
| `/api/brain/mode` | POST | Switch mode: `{"mode": "PRACTICE"}` or `{"mode": "REAL"}` |
| `/api/brain/persistence` | GET | PostgreSQL sync status |
| `/api/brain/snapshot` | POST | Force save brain data to PostgreSQL |
| `/api/brain/table` | POST | Switch roulette table: `{"table_id": "new_table"}` |
| `/api/observer/status` | GET | Observer daemon status (running, spins, strategy) |
| `/api/observer/start` | POST | Start observer daemon |
| `/api/observer/stop` | POST | Stop observer daemon |
| `/api/system_health` | GET | Overall system health check |
| `/api/hud_state` | GET | Lightweight HUD state |

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    YOUR CHROME BROWSER                     │
│  ┌─────────────┐    ┌──────────────────────────────────┐ │
│  │ Stake.com   │    │  BETTERME Chrome Extension       │ │
│  │ (Roulette)  │───►│  Intercepts WS + scrapes DOM     │ │
│  └─────────────┘    │  Sends results to server         │ │
│                     └──────────┬───────────────────────┘ │
└────────────────────────────────┼─────────────────────────┘
                                 │
                                 ▼
┌──────────────────────────────────────────────────────────┐
│              RENDER.COM (betterme-mydl)                   │
│                                                           │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐ │
│  │ Flask Server │  │ Agent        │  │ Observer Daemon  │ │
│  │ (Dashboard)  │  │ (Predictions)│  │ (24/7 learner)  │ │
│  └──────┬──────┘  └──────┬───────┘  └────────┬────────┘ │
│         │                │                    │           │
│         └────────────────┼────────────────────┘           │
│                          ▼                                │
│              ┌───────────────────────┐                    │
│              │    ROULETTE BRAIN     │                    │
│              │  7 ML Models Ensemble │                    │
│              │  + Wheel Bias Learner │                    │
│              │  + Session Profiler   │                    │
│              │  + Physics Predictor  │                    │
│              └───────────┬───────────┘                    │
│                          ▼                                │
│              ┌───────────────────────┐                    │
│              │     PostgreSQL        │                    │
│              │  (Permanent Memory)   │                    │
│              └───────────────────────┘                    │
└──────────────────────────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────┐
│                    CLAUDE CODE                            │
│  Monitors, controls, and analyzes via API                │
│  "Show me predictions" → WebFetch /api/brain/predict     │
│  "Open dashboard"      → Opens Chrome to Render URL      │
│  "Switch to REAL mode" → POST /api/brain/mode            │
└──────────────────────────────────────────────────────────┘
```

---

## Brain Models (7 ML Ensemble)

| # | Model | What It Learns | Weight |
|---|-------|----------------|--------|
| 1 | **Markov Chain** (order 1-4) | P(next number \| last N results) | Adaptive |
| 2 | **Bayesian Sector** | Sector transition probabilities | Adaptive |
| 3 | **Dealer Cluster** | Groups dealers by signature, predicts per cluster | Adaptive |
| 4 | **Bounce Physics** | Ball scatter distribution by velocity class | Adaptive |
| 5 | **Timing Patterns** | How spin interval affects dealer consistency | Adaptive |
| 6 | **Hot/Cold Streaks** | Momentum + mean reversion signals | Adaptive |
| 7 | **Conditional Engine** | P(result \| dealer + prev_color + timing) | Adaptive |

All models self-adjust their weights every 50 spins based on actual hit rate.

---

## Modes

| Mode | Behavior |
|------|----------|
| **PRACTICE** | Always predict from spin 1. Always learn. No real bets. |
| **REAL** | Only signal bets when confidence > threshold AND 2+ models agree. |

---

## Data Persistence

Brain data is saved to PostgreSQL every 25 spins. On restart:
1. `BrainPersistence.restore()` downloads all data from PostgreSQL
2. Brain reconstructs with full history
3. Continues learning exactly where it left off

No data is ever lost, even on Render free tier restarts.

---

## GitHub
Repository: https://github.com/LAVACAMARIPOSA/BETTERME
Push to `main` → Render auto-deploys in ~5 minutes
