# BETTERME — Autonomous Roulette Intelligence

## Absolute Rules — READ BEFORE ANYTHING ELSE

1. **NEVER disconnect from BETTERME until the user explicitly says "stop" or "disconnect".** If the user says "connect and monitor" you must stay monitoring indefinitely, checking status every few minutes via tools.

2. **NEVER ask the user to do something you can do.** Claude has full control of Chrome via CDP (Chrome DevTools Protocol) at endpoints `/api/control/*`. Use those endpoints for: screenshots, clicks, typing, scrolling, JS injection (even in cross-origin iframes). If you also have `mcp__Claude_in_Chrome__*` available, use it as a complement. The user does NOTHING — you do everything. Only exceptions are typing passwords (security) and updating macOS.

3. **WHEN STAKE SESSION EXPIRES — act without asking:**
   - Use CDP evaluate to detect popup: `curl -X POST http://localhost:5000/api/control/evaluate -d '{"expression":"document.body.innerText.includes(\"SESIÓN FINALIZADA\")"}'`
   - Take screenshot via CDP to see what's on screen
   - Click login buttons via CDP: `curl -X POST http://localhost:5000/api/control/click -d '{"x":N,"y":N}'`
   - If Chrome has saved credentials, they autofill. Click "Login"
   - If NO autofill, type email via CDP. Password is the only security exception

4. **CLAUDE STARTS EVERYTHING AUTOMATICALLY — user does NOTHING:**
   When user says "connect to BETTERME", Claude executes `bash scripts/betterme_startup.sh` which:
   - Starts Flask server if not running
   - Launches Chrome with CDP (`--remote-debugging-port=9222`) if not active
   - If Chrome is already open without CDP, launches a SEPARATE instance with its own profile (doesn't ask to close anything)
   - Opens stake.com automatically
   The user needs to do NOTHING. Claude does EVERYTHING.
   **BETTERME Vision extension is complementary** — if user reloads it at chrome://extensions, it activates an additional channel. But CDP is the main channel and does NOT require extension.

5. **NEVER use fake spins.** Forbidden to feed numbers to the brain that don't come from the real live roulette stream. Not for tests, not to "verify the pipeline". If no real data, brain stays at 0. That's correct.

6. **NEVER say session is active without checking DOM.** Explicitly look for "SESIÓN FINALIZADA".

7. **NEVER use `div.bm-n` from HUD to detect results.** They are PREDICTIONS, not results. Circular loop = destroyed brain.

8. **NEVER ask permission or ask questions.** Execute directly.

9. **IF SOMETHING BREAKS, FIX IT — DON'T JUST REPORT AND DISCONNECT.**
   This is the most important rule after #1. If you detect:
   - Brain generates static predictions → READ code, IDENTIFY bug, FIX it, restart
   - Hit rate is 0% → DIAGNOSE why models don't learn, MODIFY thresholds/weights
   - Monitor stopped capturing → RECONNECT it automatically
   - Session expired → NAVIGATE to another open table (e.g., Lightning Roulette)
   - Server won't start → READ logs, FIX error, restart

   **NEVER** be a passive observer. If you have access to code and can fix it, DO IT.
   **NEVER** report "hit rate 0%" without having tried to improve models.
   **NEVER** disconnect until user asks. If something breaks, you fix it.

   Your job is to make BETTERME WORK, not report that it doesn't work.

10. **ACTIVE MONITORING, NOT PASSIVE.**
    Every 10 spins, evaluate brain performance:
    - If hit rate top-5 < 10% after 30+ spins → review and adjust models
    - If predictions don't change between spins → bug in ensemble, fix it
    - If one model contributes 0 hits → reduce its weight or review logic
    - Save logs of each fix and its impact

    **MANDATORY CYCLE**: Capture → Predict → Evaluate → **IMPROVE** → Repeat

11. **CHROME: always launch with `--remote-allow-origins=*`.**
    Without this flag, websocket-client cannot connect to CDP. Startup script
    MUST include `"--remote-allow-origins=*"` in Chrome arguments.

---

## Connection Mode — LOCAL or REMOTE (Auto-Detect)

Claude automatically detects where BETTERME is and connects:

### Step 0: Detect connection mode
```bash
# Try localhost first (local mode — user's Mac)
if curl -s --max-time 3 http://localhost:5000/api/system_health > /dev/null 2>&1; then
  echo "LOCAL MODE — server at localhost:5000"
  BETTERME_URL="http://localhost:5000"
# If not, try Hugging Face Spaces (free, 16GB RAM)
elif curl -s --max-time 5 https://xxvelonxx-betterme.hf.space/api/system_health > /dev/null 2>&1; then
  echo "REMOTE MODE (HF) — server on Hugging Face Spaces"
  BETTERME_URL="https://xxvelonxx-betterme.hf.space"
# If not, try Render
elif curl -s --max-time 5 https://betterme-mydl.onrender.com/api/system_health > /dev/null 2>&1; then
  echo "REMOTE MODE (Render) — server at betterme-mydl.onrender.com"
  BETTERME_URL="https://betterme-mydl.onrender.com"
else
  # None available — try to start local
  echo "No server detected — trying to start local..."
  bash scripts/betterme_startup.sh
  BETTERME_URL="http://localhost:5000"
fi
```

### LOCAL Mode (User's Mac is on)
- Flask server runs at `localhost:5000`
- Chrome with CDP at `localhost:9222`
- Claude executes `bash scripts/betterme_startup.sh` to start everything
- Screenshots via local CDP → real image of user's Chrome
- Full control: click, type, scroll, evaluate JS

### REMOTE Mode (from phone or other computer)
- Flask server runs on VPS or Render: `https://betterme-mydl.onrender.com`
- Chrome headless with CDP runs on same VPS (Docker)
- Claude uses same `/api/control/*` endpoints but pointing to remote URL
- Everything works the same — screenshots, clicks, brain, monitoring
- **User only needs VPS active — can control from phone**

### How to use remote mode
To deploy BETTERME on a VPS ($5-10/month):
```bash
# On VPS:
curl -sSL https://raw.githubusercontent.com/LAVACAMARIPOSA/BETTERME/main/deploy/deploy-vps.sh | bash
cd /opt/betterme
nano .env  # put ANTHROPIC_API_KEY
docker compose -f deploy/docker-compose.yml up -d
```

Then, from anywhere:
```
You (from phone): "Connect to BETTERME"
Claude: detects remote server → connects → monitors → fully autonomous
```

### Important: use BETTERME_URL in ALL commands
Instead of `http://localhost:5000`, always use detected variable:
- Local: `curl -s http://localhost:5000/api/brain`
- Remote: `curl -s https://betterme-mydl.onrender.com/api/brain`

Claude must detect mode at start and use correct URL throughout protocol.

---

## EYES AND HANDS — HOW CLAUDE CONTROLS THE COMPUTER

### CDP (Chrome DevTools Protocol) — THE MAIN SOLUTION
Chrome running with `--remote-debugging-port=9222` exposes CDP, giving Claude:

**EYES — Screenshots that WORK on macOS 12:**
```bash
# Take screenshot (captures from Chrome's rendering pipeline, NOT OS)
curl -s -X POST http://localhost:5000/api/control/screenshot | python3 -c "
import json,sys; d=json.load(sys.stdin)
print(f'Screenshot: {len(d.get(\"image\",\"\"))} chars base64')
print('ERROR' if 'error' in d else 'OK')
"
```

**HANDS — Click, type, scroll:**
```bash
# Click at coordinates
curl -s -X POST http://localhost:5000/api/control/click -H 'Content-Type: application/json' -d '{"x":500,"y":300}'

# Type text
curl -s -X POST http://localhost:5000/api/control/type -H 'Content-Type: application/json' -d '{"text":"hello"}'

# Press special key (Enter, Tab, Escape, etc)
curl -s -X POST http://localhost:5000/api/control/key -H 'Content-Type: application/json' -d '{"key":"Enter"}'

# Scroll
curl -s -X POST http://localhost:5000/api/control/scroll -H 'Content-Type: application/json' -d '{"x":500,"y":300,"direction":"down","amount":3}'

# Navigate to URL
curl -s -X POST http://localhost:5000/api/control/navigate -H 'Content-Type: application/json' -d '{"url":"https://stake.com"}'
```

**EXTENDED BRAIN — JS in ANY frame (even cross-origin):**
```bash
# Execute JS in main tab (stake.com)
curl -s -X POST http://localhost:5000/api/control/evaluate -H 'Content-Type: application/json' -d '{"expression":"document.title"}'

# Execute JS in cross-origin evo-games.com iframe (!!!)
curl -s -X POST http://localhost:5000/api/control/evaluate -H 'Content-Type: application/json' -d '{"expression":"document.title","frame_url":"evo-games"}'

# View open tabs
curl -s http://localhost:5000/api/control/tabs

# CDP connection status
curl -s http://localhost:5000/api/control/status
```

### Why CDP works where everything else fails
| Method | macOS 12 | Cross-origin iframe | Requires extension |
|--------|----------|--------------------|--------------------|
| `mcp__computer-use__screenshot` | FAILS (needs macOS 14) | No | No |
| `screencapture -x` | BLACK (GPU) | No | No |
| `captureVisibleTab()` | WORKS | No (only visible tab) | Yes (manual reload) |
| **CDP `Page.captureScreenshot`** | **WORKS** | **No** | **No** |
| **CDP `Runtime.evaluate`** | **WORKS** | **YES (cross-origin!)** | **No** |

### Real-time data channels (most to least reliable)
1. **CDP screenshot + Haiku vision** → main, always works if Chrome has CDP
2. **CDP evaluate_js in evo-games.com iframe** → can read game DOM directly
3. **Extension captureVisibleTab** → complementary, requires extension reload
4. **Extension WebSocket interception** → faster but depends on extension

---

## ON CONNECTING TO BETTERME — MANDATORY PROTOCOL

When user says "connect to BETTERME" (or any variation), execute EVERYTHING without asking:

### Step 1: Start everything automatically
```bash
bash scripts/betterme_startup.sh
```

### Step 2: Verify server health
```bash
curl -s http://localhost:5000/api/system_health | python3 -m json.tool
```

### Step 3: Check CDP connection
```bash
curl -s http://localhost:5000/api/control/status
```

### Step 4: Open dashboard
```bash
open http://localhost:5000
```

### Step 5: Start monitoring
Begin monitoring loop checking every 30-60 seconds:
- Vision Watcher status
- Brain predictions
- Event logs
- System health

---

## Repository
https://github.com/LAVACAMARIPOSA/BETTERME
