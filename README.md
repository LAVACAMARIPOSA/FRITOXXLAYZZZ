# Solana Zero-Capital Beast

Bot de arbitrage y liquidaciones en Solana preparado para correr como worker en Render.

## Deploy en Render

Este repo incluye `render.yaml` para crear un worker de Render.

Checklist express en: `RENDER_DEPLOY_CHECKLIST.md`.

Pasos exactos en Render:

1. Entra a Render y elige `New +`.
2. Selecciona `Blueprint`.
3. Conecta el repositorio `LAVACAMARIPOSA/FRITOXXLAYZZZ`.
4. Confirma la creación del worker `solana-zero-capital-beast`.
5. Define las variables `RPC_URL` y `SOLANA_KEYPAIR_JSON` antes del primer deploy.
6. Lanza el deploy.

Variables de entorno necesarias en Render:

- `RPC_URL`: endpoint RPC de Solana mainnet o tu proveedor.
- `SOLANA_KEYPAIR_JSON`: keypair completo en formato JSON array de 64 bytes.

Variables opcionales:

- `RUST_LOG`: nivel de logs.

Notas importantes para Render:

- En Render no debes depender de `keypair.json` subido al repo.
- Usa `SOLANA_KEYPAIR_JSON` como secret del servicio.
- Si tu proveedor RPC requiere headers o auth adicional, usa su URL privada completa en `RPC_URL`.

## Ejecucion local

```bash
cargo build
cargo run
```

Si ejecutas localmente con archivo:

```bash
solana-keygen new -o keypair.json --no-passphrase
```

Si prefieres entorno local con variables:

```bash
cp .env.example .env
```
