# Solana Zero-Capital Beast v1.0

Bot de "dinero del aire" usando Flash Loans de Kamino + Arbitrage con Jupiter.

## Despliegue en Render (facil)

1. Ve a Render -> **New +** -> **Blueprint**
2. Conecta el repo `LAVACAMARIPOSA/FRITOXXLAYZZZ`
3. Agrega estas variables de entorno:
   - `RPC_URL` = `https://api.mainnet-beta.solana.com` (o tu RPC premium)
   - `SOLANA_KEYPAIR_JSON` = `[tu array de 64 numeros]` -> **marcar como Secret**
4. Pulsa Deploy

**Importante**: Manten `DRY_RUN = true` en la primera ejecucion.

## Ejecucion local

```bash
solana-keygen new -o keypair.json --no-passphrase
cargo run
```

Nunca uses capital propio.
