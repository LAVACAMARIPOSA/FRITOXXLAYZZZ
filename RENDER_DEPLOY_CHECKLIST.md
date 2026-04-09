# Render Deploy Checklist (30s)

1. Entra a Render y haz click en `New +`.
2. Selecciona `Blueprint`.
3. Conecta el repo `LAVACAMARIPOSA/FRITOXXLAYZZZ`.
4. Confirma el servicio `solana-zero-capital-beast`.
5. Abre `Environment` del servicio y define:
   - `RPC_URL`: URL RPC privada de tu proveedor.
   - `SOLANA_KEYPAIR_JSON`: keypair completo como JSON array de 64 bytes.
   - `RUST_LOG`: `info`.
6. Lanza `Manual Deploy`.

## Validacion rapida

1. En `Logs`, busca `Wallet cargada`.
2. Verifica que no aparezca `Crea keypair.json`.
3. Verifica ciclos de `Nuevo ciclo de búsqueda` cada ~5 segundos.

## Errores comunes

1. `SOLANA_KEYPAIR_JSON no es un array JSON valido`:
   - El secreto no es JSON valido.
   - Debe verse como `[12,34,56,...]` sin comillas extra.
2. `Wallet cargada` no aparece:
   - Variable `SOLANA_KEYPAIR_JSON` no guardada o vacia.
3. Timeout o respuestas lentas:
   - Cambia `RPC_URL` a un endpoint dedicado (Helius/QuickNode/Triton).

## Nota operativa

`DRY_RUN` esta fijo en `true` en el codigo, por lo que no enviara bundles reales en esta version.
