#!/bin/bash

# Script de verificación para Solana Zero-Capital Beast
# Este script verifica que todo esté correctamente implementado

echo "=========================================="
echo "🔍 Verificación del Bot Kamino Flash Loan"
echo "=========================================="
echo ""

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Verificar que existan todos los archivos necesarios
echo "📁 Verificando archivos fuente..."

FILES=(
    "src/main.rs"
    "src/config.rs"
    "src/flash_loan.rs"
    "src/jupiter.rs"
    "src/liquidation.rs"
    "src/bundle.rs"
    "src/utils.rs"
    "Cargo.toml"
)

ALL_PRESENT=true
for file in "${FILES[@]}"; do
    if [ -f "$file" ]; then
        lines=$(wc -l < "$file")
        echo -e "${GREEN}✓${NC} $file ($lines líneas)"
    else
        echo -e "${RED}✗${NC} $file NO ENCONTRADO"
        ALL_PRESENT=false
    fi
done

echo ""
echo "📦 Verificando dependencias en Cargo.toml..."

# Verificar dependencias críticas
DEPS=(
    "solana-sdk"
    "solana-client"
    "solana-program"
    "anchor-lang"
    "tokio"
    "reqwest"
    "serde"
    "borsh"
    "bs58"
    "spl-token"
    "spl-associated-token-account"
    "thiserror"
    "uuid"
    "tonic"
    "prost"
)

for dep in "${DEPS[@]}"; do
    if grep -q "$dep" Cargo.toml; then
        echo -e "${GREEN}✓${NC} $dep"
    else
        echo -e "${RED}✗${NC} $dep NO ENCONTRADO"
    fi
done

echo ""
echo "🔧 Verificando implementaciones..."

# Verificar funciones clave en flash_loan.rs
if grep -q "build_flash_loan_tx" src/flash_loan.rs; then
    echo -e "${GREEN}✓${NC} flash_loan::build_flash_loan_tx implementado"
else
    echo -e "${RED}✗${NC} flash_loan::build_flash_loan_tx NO encontrado"
fi

if grep -q "create_flash_borrow_instruction" src/flash_loan.rs; then
    echo -e "${GREEN}✓${NC} flash_loan::create_flash_borrow_instruction implementado"
else
    echo -e "${RED}✗${NC} flash_loan::create_flash_borrow_instruction NO encontrado"
fi

if grep -q "create_flash_repay_instruction" src/flash_loan.rs; then
    echo -e "${GREEN}✓${NC} flash_loan::create_flash_repay_instruction implementado"
else
    echo -e "${RED}✗${NC} flash_loan::create_flash_repay_instruction NO encontrado"
fi

# Verificar funciones clave en liquidation.rs
if grep -q "scan_small_liquidations" src/liquidation.rs; then
    echo -e "${GREEN}✓${NC} liquidation::scan_small_liquidations implementado"
else
    echo -e "${RED}✗${NC} liquidation::scan_small_liquidations NO encontrado"
fi

if grep -q "get_all_obligations" src/liquidation.rs; then
    echo -e "${GREEN}✓${NC} liquidation::get_all_obligations implementado"
else
    echo -e "${RED}✗${NC} liquidation::get_all_obligations NO encontrado"
fi

# Verificar funciones clave en jupiter.rs
if grep -q "get_best_jupiter_quote" src/jupiter.rs; then
    echo -e "${GREEN}✓${NC} jupiter::get_best_jupiter_quote implementado"
else
    echo -e "${RED}✗${NC} jupiter::get_best_jupiter_quote NO encontrado"
fi

if grep -q "execute_jupiter_swap" src/jupiter.rs; then
    echo -e "${GREEN}✓${NC} jupiter::execute_jupiter_swap implementado"
else
    echo -e "${RED}✗${NC} jupiter::execute_jupiter_swap NO encontrado"
fi

if grep -q "build_jupiter_instructions" src/jupiter.rs; then
    echo -e "${GREEN}✓${NC} jupiter::build_jupiter_instructions implementado"
else
    echo -e "${RED}✗${NC} jupiter::build_jupiter_instructions NO encontrado"
fi

# Verificar funciones clave en bundle.rs
if grep -q "send_jito_bundle" src/bundle.rs; then
    echo -e "${GREEN}✓${NC} bundle::send_jito_bundle implementado"
else
    echo -e "${RED}✗${NC} bundle::send_jito_bundle NO encontrado"
fi

if grep -q "JitoClient" src/bundle.rs; then
    echo -e "${GREEN}✓${NC} bundle::JitoClient implementado"
else
    echo -e "${RED}✗${NC} bundle::JitoClient NO encontrado"
fi

if grep -q "build_atomic_bundle" src/bundle.rs; then
    echo -e "${GREEN}✓${NC} bundle::build_atomic_bundle implementado"
else
    echo -e "${RED}✗${NC} bundle::build_atomic_bundle NO encontrado"
fi

echo ""
echo "📋 Verificando constantes importantes..."

# Verificar constantes
if grep -q "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD" src/flash_loan.rs; then
    echo -e "${GREEN}✓${NC} Kamino Program ID configurado"
else
    echo -e "${RED}✗${NC} Kamino Program ID NO encontrado"
fi

if grep -q "mainnet.block-engine.jito.wtf" src/bundle.rs; then
    echo -e "${GREEN}✓${NC} Jito Block Engine endpoint configurado"
else
    echo -e "${RED}✗${NC} Jito Block Engine endpoint NO encontrado"
fi

echo ""
echo "🧪 Verificando tests..."

# Contar tests
test_count=$(grep -r "#\[test\]" src/ --count 2>/dev/null || echo "0")
echo -e "${GREEN}✓${NC} $test_count tests encontrados"

echo ""
echo "=========================================="

if [ "$ALL_PRESENT" = true ]; then
    echo -e "${GREEN}✅ Todos los archivos fuente están presentes${NC}"
else
    echo -e "${RED}❌ Faltan algunos archivos fuente${NC}"
fi

echo ""
echo "Para compilar el proyecto:"
echo "  cargo check    # Verificar compilación"
echo "  cargo build    # Compilar en debug"
echo "  cargo build --release  # Compilar optimizado"
echo "  cargo test     # Ejecutar tests"
echo ""
echo "Para ejecutar el bot:"
echo "  cargo run      # Ejecutar en modo DRY_RUN"
echo ""
echo "⚠️  IMPORTANTE: Antes de ejecutar en mainnet:"
echo "  1. Configura RPC_URL con un endpoint premium (Helius/QuickNode)"
echo "  2. Configura SOLANA_KEYPAIR_JSON con tu keypair"
echo "  3. Cambia DRY_RUN a false en src/config.rs SOLO cuando estés listo"
echo "  4. Asegúrate de tener al menos 0.05 SOL para fees"
echo "=========================================="
