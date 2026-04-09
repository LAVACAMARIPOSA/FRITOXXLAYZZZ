# ROADMAP — Bot Kamino Flash-Loan al 100%

## FASE 1 — Motor real (bloqueante, sin esto no funciona en mainnet)

### 1.1 `src/flash_loan.rs` — CPI Kamino real
- Reemplazar el `Instruction` placeholder por llamadas reales con `klend-sdk`
- Construir las cuentas: `reserve`, `lendingMarket`, `obligationOwner`, `liquidityMint`, `destinationLiquidity`, etc.
- Agregar `klend-sdk` a `Cargo.toml`
- Implementar los dos ixs: `FlashBorrowReserveLiquidity` + `FlashRepayReserveLiquidity`

### 1.2 `src/liquidation.rs` — Scanner real on-chain
- Hoy es placeholder puro. Necesita iterar sobre cuentas `Obligation` de Kamino
- Deserializar `health_factor` de cada position
- Filtrar HF < 1.0 y tamaño > `MIN_PROFIT_USD`
- Usar `get_program_accounts` con filtros de discriminador

### 1.3 `DRY_RUN = false` en `src/config.rs`
- Solo activar cuando 1.1 y 1.2 estén probados

---

## FASE 2 — Infraestructura (sin esto el bot ejecuta pero mal)

### 2.1 RPC node de pago
- `https://api.mainnet-beta.solana.com` tiene rate limit ~10 req/s y latencia ~300ms
- Reemplazar por Helius, QuickNode o Triton (~$50-150/mes)
- Actualizar `RPC_URL` en Render env vars

### 2.2 Keypair con fondos
- Necesita ~0.05-0.1 SOL mínimo para fees + Jito tips
- Generar keypair: `solana-keygen new --outfile keypair.json`
- Convertir a JSON array de 64 bytes para `SOLANA_KEYPAIR_JSON`
- **Nunca subir el archivo al repo**

### 2.3 Deploy real en Render
- Render → New → Blueprint → conectar `LAVACAMARIPOSA/FRITOXXLAYZZZ`
- Set secrets: `RPC_URL` y `SOLANA_KEYPAIR_JSON`
- Verificar logs: "Wallet cargada" + "Escaneando liquidaciones"

---

## FASE 3 — Optimización (multiplica las ganancias)

### 3.1 Jito bundle real
- `src/bundle.rs` existe pero necesita conexión al endpoint Jito block-engine
- Configurar `JITO_BLOCK_ENGINE_URL` (ej. `mainnet.block-engine.jito.wtf`)
- Tip dinámico según congestion de red

### 3.2 Jupiter swap execution
- `src/jupiter.rs` solo hace quote, no ejecuta el swap
- Agregar `POST /swap` con el quote response para obtener `swapTransaction`
- Integrar la tx del swap dentro del bundle Jito atómico

### 3.3 Liquidation path completo (flash → liq → swap → repay)
- Unificar los módulos en un bundle atómico de 4 instrucciones:
  1. `FlashBorrowReserveLiquidity`
  2. Instrucción de liquidación Kamino
  3. Swap Jupiter (colateral → stable)
  4. `FlashRepayReserveLiquidity`

---

## FASE 4 — Producción seria

### 4.1 Monitoreo y alertas
- Webhook Discord/Telegram para cada liquidación capturada y errores
- Dashboard P&L con histórico

### 4.2 Multi-protocolo
- Agregar MarginFi, Solend, Drift como fuentes adicionales de liquidaciones
- Multiplicador ×3-5 en oportunidades detectadas

### 4.3 Co-location
- VPS en AWS us-east-1 (mismo datacenter que validators de Solana)
- Latencia pasa de ~200ms a ~15ms → tasa de captura del 14% sube a ~60-70%

---

## Estado actual por módulo

| Módulo | Estado | Bloquea prod? |
|---|---|---|
| `config.rs` | ✅ listo | no |
| `utils.rs` | ✅ listo | no |
| `jupiter.rs` | ⚠️ solo quote, falta swap | sí |
| `flash_loan.rs` | ❌ placeholder | **sí** |
| `bundle.rs` | ⚠️ estructura ok, falta endpoint real | sí |
| `liquidation.rs` | ❌ placeholder | **sí** |
| `main.rs` | ✅ loop ok | no |
| Render deploy | ⏳ pendiente | sí |

## Orden óptimo de implementación

```
1.2 (scanner) → 1.1 (flash loan CPI) → 3.2 (jupiter swap) → 3.3 (bundle atómico) → 2.1 (RPC pago) → 2.2 (keypair) → 2.3 (render deploy) → 1.3 (DRY_RUN=false)
```
