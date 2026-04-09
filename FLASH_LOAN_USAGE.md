# Flash Loans con Kamino Lending - Guía de Uso

## Descripción

Este m implementa flash loans reales usando el protocolo Kamino Lending en Solana Mainnet.

## Características

- ✅ Flash Loans reales de Kamino Lending (no mocks)
- ✅ Integración con Jupiter para swaps
- ✅ Cálculo automático de fees (0.09% estándar)
- ✅ Creación automática de ATAs si es necesario
- ✅ Simulación de transacciones antes de enviar
- ✅ Soporte para múltiples reserves (USDC, USDT, SOL, etc.)

## Constantes Importantes

### Program IDs
```rust
KAMINO_LEND_PROGRAM: "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD"
TOKEN_PROGRAM_ID: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
```

### Flash Loan Fee
- Fee estándar: 0.09% (9 bps)
- Representación en scaled fraction: `900_000_000_000_000` (18 decimales)
- Fórmula: `fee = amount * flash_loan_fee_sf / 10^18`

## Uso Básico

### Flash Loan Simple (Borrow + Repay)

```rust
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;

let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
let keypair = Keypair::new(); // Tu keypair

// Crear flash loan de 1000 USDC
let tx = flash_loan::build_simple_flash_loan_tx(
    &client,
    &keypair,
    1_000_000_000, // 1000 USDC (6 decimales)
    flash_loan::kamino_reserves::USDC,
    flash_loan::USDC_MINT,
).await?;
```

### Flash Loan con Arbitrage

```rust
// Crear flash loan con swap integrado de Jupiter
let tx = flash_loan::build_flash_loan_tx(
    &client,
    &keypair,
    10_000_000_000, // 10,000 USDC
).await?;
```

## Estructura de la Transacción

Un flash loan típico incluye:

1. **Setup** (opcional): Crear ATAs si no existen
2. **Flash Borrow**: Tomar prestado del reserve
3. **Operación**: Swap en Jupiter, liquidación, etc.
4. **Flash Repay**: Repagar préstamo + fee

## Cuentas Necesarias

### FlashBorrowReserveLiquidity
- `user_transfer_authority`: Firma de la transacción (signer)
- `lending_market_authority`: PDA del lending market
- `lending_market`: Cuenta del mercado de lending
- `reserve`: Cuenta del reserve de liquidez
- `reserve_liquidity_mint`: Mint del token
- `reserve_source_liquidity`: Vault del reserve (PDA)
- `user_destination_liquidity`: ATA del usuario para recibir fondos
- `reserve_liquidity_fee_receiver`: Cuenta de fees del reserve
- `referrer_token_state`: Opcional, para referrals
- `referrer_account`: Opcional, para referrals
- `sysvar_info`: Sysvar de instrucciones
- `token_program`: SPL Token program

### FlashRepayReserveLiquidity
Mismas cuentas que borrow, excepto:
- `reserve_destination_liquidity`: Misma que `reserve_source_liquidity`
- `user_source_liquidity`: Misma que `user_destination_liquidity`

## PDAs (Program Derived Addresses)

### Lending Market Authority
```rust
seeds = [b"lending_market_auth", lending_market.key()]
program = KAMINO_LEND_PROGRAM
```

### Reserve Liquidity Supply
```rust
seeds = [b"reserve_liq_supply", reserve.key()]
program = KAMINO_LEND_PROGRAM
```

### Reserve Fee Receiver
```rust
seeds = [b"fee_receiver", reserve.key()]
program = KAMINO_LEND_PROGRAM
```

## Obtener Reserves Reales

Para obtener las direcciones reales de los reserves en mainnet:

### Opción 1: Kamino SDK (TypeScript)
```typescript
import { KaminoMarket } from '@kamino-finance/klend-sdk';

const market = await KaminoMarket.load(
  connection,
  new PublicKey("7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv6PfF"),
  200, // scope
  new PublicKey("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD")
);

const reserves = market.getReserves();
```

### Opción 2: Explorador de Solana
1. Ve a https://solscan.io/account/7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv6PfF
2. Revisa las cuentas asociadas

### Opción 3: App de Kamino
1. Abre https://app.kamino.finance
2. Inspecciona las transacciones de lending

## Seguridad

### Validaciones Importantes
1. **Flash loans no pueden ser llamados via CPI**
2. **Solo un flash borrow por transacción**
3. **El repay debe incluir el fee correspondiente**
4. **Todas las operaciones deben ser atómicas**

### Errores Comunes
- `FlashLoansDisabled`: El reserve tiene flash loans deshabilitados
- `InvalidFlashRepay`: Las cuentas no coinciden entre borrow y repay
- `NoFlashRepayFound`: No se encontró la instrucción de repay
- `FlashBorrowCpi`: Intentando llamar flash loan via CPI
- `MultipleFlashBorrows`: Más de un flash borrow en la transacción

## Testing

### Tests Unitarios
```bash
cargo test flash_loan::tests
```

### Test en Devnet
1. Cambia `MAIN_LENDING_MARKET` a la dirección de devnet
2. Usa tokens de devnet (faucet)
3. Ejecuta con `DRY_RUN = true` primero

## Dependencias Agregadas

```toml
[dependencies]
borsh = { version = "1.5", features = ["derive"] }
bs58 = "0.5"
spl-token = "6.0"
spl-associated-token-account = "5.0"
kamino-lend = "0.4"
```

## Recursos

- [Documentación de Kamino](https://docs.kamino.finance)
- [Kamino SDK](https://github.com/Kamino-Finance/klend-sdk)
- [Flash Loans Docs](https://kamino-finance.gitbook.io/klend/flash-loans)

## Notas de Implementación

1. Los discriminadores de Anchor se calculan dinámicamente usando SHA-256
2. Las fees se calculan usando scaled fraction con 18 decimales
3. La integración con Jupiter usa la API v6 de quotes
4. Las ATAs se crean automáticamente si no existen
