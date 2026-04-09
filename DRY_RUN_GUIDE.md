# Guía de DRY_RUN - Solana Zero-Capital Beast

## ⚠️ IMPORTANTE: No hay Testnet para DeFi en Solana

Los protocolos DeFi como **Kamino**, **Jupiter** y **Jito** **solo operan en mainnet**. No existen en devnet/testnet.

### ¿Qué significa esto?
- ❌ No puedes probar en testnet
- ✅ Puedes ejecutar en **DRY_RUN mode** en mainnet (sin enviar transacciones)
- ✅ Las operaciones de **solo lectura** funcionan (escanear liquidaciones, obtener quotes)

---

## 🛡️ ¿Qué es DRY_RUN?

DRY_RUN es un modo seguro donde el bot:
- ✅ **Lee** datos de mainnet (obligaciones, precios, quotes)
- ✅ **Simula** transacciones localmente
- ❌ **NO envía** transacciones reales a la blockchain
- ❌ **NO gasta** SOL en fees
- ❌ **NO ejecuta** flash loans reales

---

## 🚀 Cómo Ejecutar en DRY_RUN

### 1. Verificar que DRY_RUN está activado

```bash
# Debe decir: DRY_RUN: bool = true
grep "DRY_RUN" src/config.rs
```

### 2. Ejecutar el bot

```bash
# Opción 1: Script automático
./run.sh

# Opción 2: Directo con cargo
cargo run --release
```

### 3. Lo que verás

```
🚀 Solana Zero-Capital Beast v1.0 - Iniciando
🔒 MODO DRY RUN ACTIVADO - No se enviarán transacciones reales
✅ Wallet cargada: 7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv6PfF
🔗 Conectando a: https://api.mainnet-beta.solana.com
✅ Conectado. Slot actual: 123456789
💰 Balance: 0.0000 SOL

🔄 === Ciclo #1 ===
📊 Estrategia 1: Arbitrage con Flash Loans
💰 Mejor ruta Jupiter: +$1.23 (impacto: 0.05%)
💰 Oportunidad de arbitrage detectada: +$1.23
⚡ Construyendo Flash Loan con Kamino Lending...
🔒 DRY RUN - Bundle con tip 984000000 lamports (0.984 SOL)
   Congestion: Medium, Profit: 1.23 SOL

📊 Estrategia 2: Escaneo de Liquidaciones
🔍 Escaneando posiciones underwater $10-$500 en Kamino...
✅ Escaneo completado. 3 oportunidades de liquidación encontradas

📌 Oportunidad #1
   Obligation: 8J...xYz
   Health Factor: 0.8543
   Depósitos: $250.00
   Deuda: $200.00
   Ganancia Estimada: $7.00
   ✅ Rentable - Procesando...
   🔒 Modo DRY RUN - Saltando ejecución
```

---

## 📋 Qué Hace el Bot en DRY_RUN

### Estrategia 1: Arbitrage
1. Obtiene quotes reales de Jupiter
2. Calcula profit potencial
3. Construye la transacción de flash loan
4. **Simula** la transacción
5. Muestra: "🔒 DRY RUN - Bundle con tip X lamports"

### Estrategia 2: Liquidaciones
1. Escanea obligaciones reales de Kamino
2. Calcula Health Factor real
3. Identifica posiciones liquidables
4. Muestra oportunidades encontradas
5. **NO ejecuta** liquidaciones

### Estrategia 3: Flash Test (cada 10 ciclos)
1. Construye flash loan simple
2. Verifica que el CPI funciona
3. Muestra resultado de la simulación

---

## ✅ Qué Puedes Verificar en DRY_RUN

| Funcionalidad | ¿Funciona? | Descripción |
|--------------|------------|-------------|
| Conexión RPC | ✅ | Conecta a mainnet y obtiene slot |
| Balance | ✅ | Lee balance de la wallet |
| Jupiter Quotes | ✅ | Obtiene precios reales |
| Escaneo Kamino | ✅ | Lee obligaciones on-chain |
| Simulación | ✅ | Simula transacciones localmente |
| Construcción TX | ✅ | Construye transacciones válidas |

---

## ⚠️ Limitaciones de DRY_RUN

1. **No puedes probar la ejecución real**: Las transacciones no se envían
2. **No puedes verificar el bundle**: Jito no recibe nada
3. **No hay cambio de estado**: Los flash loans no modifican nada
4. **Competencia real**: Otros bots sí están ejecutando

---

## 🔧 Configuración Recomendada para DRY_RUN

```rust
// src/config.rs
pub const DRY_RUN: bool = true;           // Mantener true
pub const MIN_PROFIT_USD: f64 = 0.1;      // Bajar para ver más oportunidades
```

```bash
# Variables de entorno (opcional)
export RPC_URL="https://api.mainnet-beta.solana.com"
# No necesitas SOLANA_KEYPAIR_JSON para DRY_RUN
```

---

## 📊 Interpretando Resultados

### Si ves muchas oportunidades:
- El bot está funcionando correctamente
- Pero recuerda: en producción hay competencia

### Si no ves oportunidades:
- Puede ser normal (mercado eficiente)
- O el MIN_PROFIT_USD es muy alto
- O el RPC está limitando requests

### Si ves errores:
- RPC rate limit: Cambia a un RPC premium
- Simulación fallida: La ruta puede no ser válida
- Timeout: Congestión de red

---

## 🎯 Próximos Pasos Después de DRY_RUN

### 1. Cuando estés listo para producción:
```rust
// Cambiar en src/config.rs
pub const DRY_RUN: bool = false;
```

### 2. Configurar RPC premium:
```bash
export RPC_URL="https://mainnet.helius-rpc.com/?api-key=TU_API_KEY"
```

### 3. Configurar keypair con fondos:
```bash
export SOLANA_KEYPAIR_JSON="[64,23,12,...]"
```

### 4. Ejecutar:
```bash
cargo run --release
```

---

## ❓ FAQ

### ¿Puedo perder dinero en DRY_RUN?
**NO**. DRY_RUN no envía transacciones, solo lee datos y simula.

### ¿Por qué no hay testnet?
Los protocolos DeFi necesitan liquidez real, oráculos reales, y precios reales. No funcionan en testnet.

### ¿Cuánto debería dejar corriendo en DRY_RUN?
Recomendado: 1-2 horas para ver patrones de oportunidades.

### ¿Los resultados de DRY_RUN son los mismos en producción?
**NO exactamente**. En producción:
- Hay más competencia (otros bots)
- Los precios cambian más rápido
- Los bundles pueden fallar
- La congestión afecta

### ¿Puedo usar el RPC público para DRY_RUN?
Sí, pero tiene rate limits (~10 req/s). Para mejor experiencia, usa un RPC gratuito de Helius o QuickNode.

---

## 🆘 Troubleshooting

### Error: "No se encontró keypair"
- En DRY_RUN no es necesario, el bot genera uno temporal
- O crea uno: `solana-keygen new -o keypair.json --no-passphrase`

### Error: "429 Too Many Requests"
- El RPC público tiene límites
- Solución: Usa RPC premium o espera entre requests

### Error: "Simulación fallida"
- La ruta de arbitrage puede no ser válida
- O el flash loan no tiene suficiente liquidez
- Normal en DRY_RUN, no indica problema

---

## ✅ Checklist Antes de Producción

- [ ] Ejecutar en DRY_RUN por al menos 1 hora
- [ ] Verificar que encuentra oportunidades
- [ ] Revisar logs de errores
- [ ] Configurar RPC premium
- [ ] Configurar keypair con fondos
- [ ] Cambiar DRY_RUN a false
- [ ] Empezar con fondos mínimos (0.05 SOL)
- [ ] Monitorear de cerca los primeros minutos

---

**Recuerda: DRY_RUN es tu amigo. Úsalo extensivamente antes de arriesgar fondos reales.**
