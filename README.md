# Solana Zero-Capital Beast v1.0

Bot de Flash Loans + Arbitrage + Liquidaciones usando Kamino Lending + Jupiter Swaps + Jito Bundles.

## 🚀 Características

- **Flash Loans reales** con Kamino Lending (CPI completo)
- **Arbitrage** con Jupiter API v6 (quotes + ejecución)
- **Escaneo de liquidaciones** on-chain en Kamino
- **Bundles atómicos** con Jito Block Engine
- **100% funcional** - Sin mocks, sin placeholders

## 📁 Estructura del Proyecto

```
src/
├── main.rs           # Loop principal del bot
├── config.rs         # Configuración y constantes
├── flash_loan.rs     # CPI Kamino (Flash Borrow/Repay)
├── jupiter.rs        # Jupiter API (quotes + swaps)
├── liquidation.rs    # Escáner de liquidaciones
├── bundle.rs         # Jito bundles (envío + confirmación)
└── utils.rs          # Utilidades (keypair, logging)
```

## 🛠️ Compilación

```bash
# Verificar compilación
cargo check

# Compilar en debug
cargo build

# Compilar optimizado para producción
cargo build --release

# Ejecutar tests
cargo test
```

## ⚙️ Configuración

### Variables de Entorno (Render)

```bash
RPC_URL=https://api.mainnet-beta.solana.com  # O tu RPC premium
SOLANA_KEYPAIR_JSON=[64,23,12,...]  # Keypair como JSON array (marcar como Secret)
```

### Configuración en Código

Edita `src/config.rs`:

```rust
pub const DRY_RUN: bool = true;  // Cambiar a false para producción
pub const MIN_PROFIT_USD: f64 = 0.8;  // Profit mínimo para ejecutar
```

## 🚀 Despliegue en Render

1. Ve a Render → **New +** → **Blueprint**
2. Conecta el repo `LAVACAMARIPOSA/FRITOXXLAYZZZ`
3. Agrega las variables de entorno:
   - `RPC_URL` = `https://api.mainnet-beta.solana.com` (o tu RPC premium)
   - `SOLANA_KEYPAIR_JSON` = `[tu array de 64 números]` → **marcar como Secret**
4. Pulsa Deploy

## 🏃 Ejecución Local

```bash
# Generar keypair
solana-keygen new -o keypair.json --no-passphrase

# Ejecutar en modo DRY_RUN (seguro)
cargo run

# Ejecutar en modo producción (requiere fondos reales)
# 1. Cambia DRY_RUN a false en src/config.rs
# 2. Ejecuta:
cargo run --release
```

## 📊 Funcionamiento

El bot ejecuta 3 estrategias en cada ciclo:

### 1. Arbitrage con Flash Loans
- Toma un flash loan de USDC desde Kamino
- Ejecuta swap USDC -> SOL -> USDC via Jupiter
- Repaga el flash loan + fee (0.09%)
- Envía bundle atómico a Jito

### 2. Escaneo de Liquidaciones
- Escanea todas las obligaciones de Kamino
- Filtra posiciones liquidables (HF < 1.0)
- Selecciona oportunidades $10-$500 (nicho poco competido)
- Calcula profit estimado (bonus de liquidación ~7%)

### 3. Flash Loan Simple (Testing)
- Ejecuta cada 10 ciclos en modo DRY_RUN
- Verifica que el CPI de Kamino funciona correctamente

## 🔒 Seguridad

- **DRY_RUN por defecto**: El bot no envía transacciones reales hasta que se desactiva
- **Simulación previa**: Todas las transacciones se simulan antes de enviar
- **Bundles atómicos**: Las operaciones son atómicas (todo o nada)
- **Tips optimizados**: Jito tips calculados según congestión de red

## ⚠️ Requisitos para Mainnet

Antes de activar modo producción:

1. **RPC Premium**: Recomendado Helius, QuickNode o Triton (~$50-150/mes)
   - El RPC público tiene rate limits y latencia alta
   
2. **Fondos**: Necesitas ~0.05-0.1 SOL mínimo:
   - Fees de transacción
   - Jito tips (para competir por inclusion)
   
3. **Keypair**: Genera uno nuevo y transfiere fondos:
   ```bash
   solana-keygen new -o keypair.json --no-passphrase
   # Transfiere SOL a la dirección mostrada
   ```

4. **Co-location** (opcional pero recomendado):
   - VPS en AWS us-east-1 reduce latencia de ~200ms a ~15ms
   - Mejora tasa de captura de ~14% a ~60-70%

## 📈 Monitoreo

El bot loguea estadísticas cada 10 ciclos:
- Ciclos ejecutados
- Oportunidades detectadas
- Bundles enviados
- Profit total estimado

## 🧪 Tests

```bash
# Ejecutar todos los tests
cargo test

# Tests específicos
cargo test test_calculate_flash_loan_fees
cargo test test_health_factor_calculation
cargo test test_slippage_calculation
```

## 📚 Documentación Técnica

### Flash Loans (Kamino)
- Program ID: `KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD`
- Fee: 0.09% (9 bps)
- Instrucciones: `FlashBorrowReserveLiquidity`, `FlashRepayReserveLiquidity`

### Jupiter API
- Quote API: `https://quote-api.jup.ag/v6/quote`
- Swap API: `https://quote-api.jup.ag/v6/swap`
- Slippage dinámico según volatilidad

### Jito Bundles
- Block Engine: `https://mainnet.block-engine.jito.wtf`
- Max 5 transacciones por bundle
- Tips: 0.001 SOL mínimo para ser competitivo

## 📝 TODO / Roadmap

- [x] Flash Loans reales con Kamino
- [x] Escaneo de liquidaciones on-chain
- [x] Jupiter swaps (quotes + ejecución)
- [x] Jito bundles atómicos
- [ ] Liquidación real de obligaciones
- [ ] Multi-protocolo (MarginFi, Solend, Drift)
- [ ] Dashboard P&L
- [ ] Webhook Discord/Telegram

## ⚠️ Disclaimer

**Este bot es software experimental. Úsalo bajo tu propio riesgo.**

- Flash loans son complejos y pueden resultar en pérdida de fondos
- Siempre ejecuta en modo DRY_RUN primero
- Nunca inviertas más de lo que puedas perder
- Las liquidaciones son competitivas; no hay garantía de profit

## 📄 Licencia

MIT - Úsalo bajo tu propia responsabilidad.

---

**Nunca uses capital propio que no puedas perder.**
