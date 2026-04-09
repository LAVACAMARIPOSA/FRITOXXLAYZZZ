#!/bin/bash

# Script de ejecución segura para Solana Zero-Capital Beast
# Este script ejecuta el bot en modo DRY_RUN (sin enviar transacciones reales)

set -e

echo "=========================================="
echo "🚀 Solana Zero-Capital Beast - DRY RUN"
echo "=========================================="
echo ""

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Verificar que estamos en el directorio correcto
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ Error: No se encontró Cargo.toml${NC}"
    echo "Ejecuta este script desde el directorio raíz del proyecto"
    exit 1
fi

# Verificar DRY_RUN
echo "🔍 Verificando configuración..."
if grep -q "DRY_RUN: bool = true" src/config.rs; then
    echo -e "${GREEN}✅ DRY_RUN está activado - Es seguro ejecutar${NC}"
else
    echo -e "${RED}⚠️  ADVERTENCIA: DRY_RUN está desactivado${NC}"
    echo "Esto significa que el bot enviará transacciones reales a mainnet"
    echo ""
    read -p "¿Estás seguro de que quieres continuar? (yes/no): " confirm
    if [ "$confirm" != "yes" ]; then
        echo "Operación cancelada"
        exit 1
    fi
fi

echo ""
echo "📦 Verificando compilación..."

# Verificar si cargo está instalado
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Cargo no está instalado${NC}"
    echo "Instala Rust desde: https://rustup.rs/"
    echo ""
    echo "Comando de instalación:"
    echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Compilar en modo check primero
echo "🔧 Verificando código..."
cargo check 2>&1 | head -20

if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Error de compilación${NC}"
    echo "Revisa los errores arriba"
    exit 1
fi

echo -e "${GREEN}✅ Código verificado correctamente${NC}"
echo ""

# Verificar keypair
echo "🔑 Verificando keypair..."
if [ -n "$SOLANA_KEYPAIR_JSON" ]; then
    echo -e "${GREEN}✅ Variable de entorno SOLANA_KEYPAIR_JSON configurada${NC}"
elif [ -f "keypair.json" ]; then
    echo -e "${GREEN}✅ Archivo keypair.json encontrado${NC}"
else
    echo -e "${YELLOW}⚠️  No se encontró keypair${NC}"
    echo "El bot generará un keypair temporal para DRY_RUN"
    echo ""
    echo "Para configurar un keypair permanente:"
    echo "  1. Genera uno nuevo: solana-keygen new -o keypair.json --no-passphrase"
    echo "  2. O configura la variable de entorno: export SOLANA_KEYPAIR_JSON='[...]'"
fi

echo ""
echo "🌐 Configuración de red:"
echo "  RPC_URL: ${RPC_URL:-https://api.mainnet-beta.solana.com (default)}"
echo ""

# Preguntar antes de ejecutar
echo -e "${BLUE}ℹ️  Este bot ejecutará:${NC}"
echo "  • Escaneo de oportunidades de arbitrage (sin ejecutar)"
echo "  • Escaneo de liquidaciones en Kamino (solo lectura)"
echo "  • Simulación de transacciones (sin enviar)"
echo ""

if [ "$1" == "--auto" ]; then
    echo "Modo automático activado (sin confirmación)"
else
    read -p "¿Quieres iniciar el bot? (s/n): " start
    if [ "$start" != "s" ] && [ "$start" != "S" ]; then
        echo "Operación cancelada"
        exit 0
    fi
fi

echo ""
echo "=========================================="
echo "🚀 Iniciando bot..."
echo "=========================================="
echo ""
echo "Presiona Ctrl+C para detener"
echo ""

# Ejecutar el bot
cargo run --release 2>&1

# Si llegamos aquí, el bot terminó
echo ""
echo "=========================================="
echo "🏁 Bot finalizado"
echo "=========================================="
