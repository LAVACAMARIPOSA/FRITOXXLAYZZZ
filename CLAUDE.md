# ELCOQUI Bot — AI Control Protocol

## What This Is
ELCOQUI is an autonomous Solana DeFi trading bot (flash loan arbitrage + liquidations).
It runs 24/7 on HuggingFace Spaces and exposes a REST API for monitoring and control.

## Connect to the Bot

### Step 1: Find the bot
```bash
# HuggingFace Spaces (primary)
curl -s https://xxvelonxx-fritoxxlayzzz.hf.space/api/status

# If that fails, try health check
curl -s https://xxvelonxx-fritoxxlayzzz.hf.space/health
```

### Step 2: Check status
```bash
curl -s https://xxvelonxx-fritoxxlayzzz.hf.space/api/status | python3 -m json.tool
```

You'll see: cycle number, running state, risk level, profit, opportunities, build version, DRY_RUN mode.

---

## API Endpoints

Base URL: `https://xxvelonxx-fritoxxlayzzz.hf.space` (or whatever the deploy URL is)

### READ (GET) — Monitor

| Endpoint | What It Returns |
|----------|----------------|
| `GET /api/status` | Cycle, running, risk, profit, opportunities, build, dry_run |
| `GET /api/memory` | Full brain summary + learning report + scan metrics |
| `GET /api/scan` | Last arbitrage scan report + last liquidation scan report |
| `GET /api/logs` | Last 200 activity log lines with timestamps |
| `GET /api/routes` | All routes sorted by priority with stats (scans, fails, spreads) |
| `GET /api/config` | Current config: risk level, min_profit, api_delay, liq_interval |
| `GET /health` | Simple health check `{"status":"ok"}` |

### WRITE (POST) — Control

| Endpoint | Body | What It Does |
|----------|------|-------------|
| `POST /api/start` | (none) | Start the scanner |
| `POST /api/stop` | (none) | Pause the scanner |
| `POST /api/risk` | `{"level":"aggressive"}` | Change risk: safe/normal/aggressive |
| `POST /api/reset` | (none) | Reset all memory and learning |
| `POST /api/min_profit` | `{"value": 0.05}` | Set minimum profit threshold |

---

## Monitoring Protocol

When asked to "connect to ELCOQUI" or "monitor the bot", do this:

### 1. Check if bot is alive
```bash
curl -s $BOT_URL/api/status
```

### 2. Get full brain state
```bash
curl -s $BOT_URL/api/memory
```

### 3. See what the scanner found last cycle
```bash
curl -s $BOT_URL/api/scan
```

### 4. See route learning data (which routes work, which fail)
```bash
curl -s $BOT_URL/api/routes
```

### 5. Read activity logs
```bash
curl -s $BOT_URL/api/logs
```

### 6. Monitor loop (check every 60 seconds)
```bash
while true; do
  curl -s $BOT_URL/api/status | python3 -m json.tool
  sleep 60
done
```

---

## Diagnosing Problems

### Bot shows 0 opportunities
1. Check `/api/routes` — are routes getting scanned or all failing?
2. Check `/api/scan` — what spreads are being found?
3. If all routes show high `fails` count → Jupiter API issue
4. If spreads are all < -0.5% → market has no micro-opportunities right now

### Bot is not scanning
1. Check `/api/status` → is `running` true?
2. If false → `POST /api/start`
3. Check `/api/config` → is `api_delay_ms` too high? (should be 200-500)

### Bot finds opportunity but doesn't execute
1. Check `/api/logs` for "rejected" entries
2. The strategy engine may have rejected it (min_profit too high, wrong risk level)
3. Try: `POST /api/risk` with `{"level":"aggressive"}` to lower thresholds
4. Try: `POST /api/min_profit` with `{"value": 0.01}` to accept smaller profits

### Deploy seems broken
1. Check `/api/status` → look at `build` field
2. It should say `v2.1.0-nobackoff` or higher
3. If it shows old version → GitHub Action didn't deploy. Check Actions tab.

---

## Architecture

```
┌──────────────────────────────────────────────────┐
│           HUGGINGFACE SPACES (Docker)             │
│                                                   │
│  ┌─────────────────────────────────────────────┐ │
│  │              ELCOQUI BOT (Rust)              │ │
│  │                                              │ │
│  │  Main Loop ──► Jupiter API (quotes)          │ │
│  │      │     ──► Solana RPC (liquidations)     │ │
│  │      │     ──► Telegram (notifications)      │ │
│  │      │     ──► Groq AI (chat)                │ │
│  │      │                                       │ │
│  │      ▼                                       │ │
│  │  AgentMemory ◄──► memory.json (persistent)   │ │
│  │      │                                       │ │
│  │      ▼                                       │ │
│  │  API Server (:7860) ◄── Claude/AI connects   │ │
│  └─────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
         │                        ▲
         ▼                        │
    Telegram Bot              curl/WebFetch
    (user chat)              (AI monitoring)
```

---

## Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Main loop, command handling, strategy execution |
| `src/api.rs` | HTTP API server for AI control |
| `src/memory.rs` | Persistent brain: learning, metrics, adaptive params |
| `src/jupiter.rs` | Jupiter API scanner (4 strategies, 20 routes) |
| `src/liquidation.rs` | Kamino liquidation scanner |
| `src/strategy.rs` | Strategy engine (Go/Skip decisions) |
| `src/telegram.rs` | Telegram bot + Groq AI chat |
| `src/config.rs` | Constants and env var config |
| `data/memory.json` | Persisted brain state |

---

## Current Mode
- **DRY_RUN = true** — Bot scans real data but does NOT execute real transactions
- Learning is active — bot accumulates knowledge for when it goes live
- All spreads and opportunities are from real mainnet data
