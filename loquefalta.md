# ROADMAP — Bot Kamino Flash-Loan (ACTUALIZADO)

## COMPLETADO

### FASE 1 — Motor real (COMPLETADO)

#### 1.1 `src/flash_loan.rs` — CPI Kamino real
- [x] Llamadas reales con CPI (FlashBorrowReserveLiquidity + FlashRepayReserveLiquidity)
- [x] Discriminadores de Anchor (SHA-256)
- [x] PDAs: lending_market_authority, reserve_liquidity_supply, reserve_fee_receiver
- [x] Fees 0.09% (scaled fraction)
- [x] Integración Jupiter swaps
- [x] ATA creation automática
- [x] Direcciones reales de mainnet (USDC Reserve: D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59)
- [x] Tests unitarios

#### 1.2 `src/liquidation.rs` — Scanner on-chain real
- [x] get_program_accounts con filtros de discriminador
- [x] Deserialización borsh de Obligation
- [x] Health Factor y LTV
- [x] Filtro liquidables (HF < 1.0, $10-$500)
- [x] Profit estimado (~7% bonus)
- [x] Tests unitarios

#### 1.3 `src/jupiter.rs` — Swap execution
- [x] Jupiter API v6 quotes + swap
- [x] Slippage dinámico (50-500 bps)
- [x] Instrucciones para bundles
- [x] Retry con backoff exponencial
- [x] base64 crate 0.22 API
- [x] Tests unitarios

#### 1.4 `src/bundle.rs` — Jito bundles
- [x] HTTP API (sendBundle)
- [x] Tips dinámicos por congestión
- [x] Simulación previa
- [x] Confirmación on-chain
- [x] Reintentos automáticos
- [x] Tests unitarios

### FASE 2 — Infraestructura

#### 2.1 DRY_RUN mode (COMPLETADO)
- [x] Lee datos reales de mainnet
- [x] Quotes reales de Jupiter
- [x] Simula transacciones localmente
- [x] Simula flujo completo de liquidación
- [x] Bloquea envío de transacciones reales
- [x] Estadísticas de oportunidades

#### 2.2 Deployment
- [ ] RPC premium (Helius/QuickNode, $50-150/mes)
- [ ] Keypair con fondos (~0.05-0.1 SOL)
- [ ] Deploy a Render (blueprint configurado)

## ESTADO POR MODULO

| Modulo | Estado | Compila | Tests |
|---|---|---|---|
| config.rs | listo | si | - |
| utils.rs | listo | si | - |
| jupiter.rs | completo | si | 4 pass |
| flash_loan.rs | completo | si | 3 pass |
| bundle.rs | completo | si | 5 pass |
| liquidation.rs | completo | si | 2 pass |
| main.rs | integrado | si | - |
| **TOTAL** | **~3,600 lineas** | **si** | **14 pass** |

## PARA PRODUCCION (cuando quieras salir de DRY_RUN)

1. Cambiar `DRY_RUN = false` en `src/config.rs`
2. Configurar `RPC_URL` con endpoint premium
3. Configurar `SOLANA_KEYPAIR_JSON` con wallet con fondos
4. Verificar `MIN_PROFIT_USD` (default: $0.80)
5. Compilar: `cargo build --release`
6. Ejecutar con fondos minimos inicialmente
7. Monitorear logs
