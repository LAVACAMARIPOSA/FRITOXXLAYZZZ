# Solana Zero-Capital Beast

Bot de arbitrage y liquidaciones en Solana preparado para correr como worker en Render.

## Deploy en Render

Este repo incluye `render.yaml` para crear un worker de Render.

Variables de entorno necesarias en Render:

- `RPC_URL`: endpoint RPC de Solana mainnet o tu proveedor.
- `SOLANA_KEYPAIR_JSON`: keypair completo en formato JSON array de 64 bytes.

Variables opcionales:

- `KEYPAIR_PATH`: ruta local del keypair si no usas `SOLANA_KEYPAIR_JSON`.
- `RUST_LOG`: nivel de logs.

## Ejecucion local

```bash
cargo build
cargo run
```

Si ejecutas localmente con archivo:

```bash
solana-keygen new -o keypair.json --no-passphrase
```
