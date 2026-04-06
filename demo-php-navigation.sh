#!/bin/bash

set -e

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
ENGINE_DIR="${REPO_ROOT}/crates/navigation-engine"
PHP_EXAMPLE="${REPO_ROOT}/examples/php"

echo "═══════════════════════════════════════════════════════"
echo "  🐘 PHP Navigation Agent MCP - Demo Completo"
echo "═══════════════════════════════════════════════════════"
echo ""

# Build first
echo "📦 Building navigation-engine..."
cd "${ENGINE_DIR}"
cargo build --quiet 2>/dev/null || cargo build
echo "✅ Build complete"
echo ""

echo "═══════════════════════════════════════════════════════"
echo "  1️⃣  FIND_SYMBOL - Buscar símbolos PHP"
echo "═══════════════════════════════════════════════════════"
echo ""
echo "📍 Símbolos en UserService.php:"
cargo run --quiet --bin inspect_php_symbols "${PHP_EXAMPLE}/src/Service/UserService.php" 2>/dev/null | head -20
echo ""

echo "📍 Símbolos en UserRepository.php:"
cargo run --quiet --bin inspect_php_symbols "${PHP_EXAMPLE}/src/Repository/UserRepository.php" 2>/dev/null
echo ""

echo "═══════════════════════════════════════════════════════"
echo "  2️⃣  TRACE_FLOW - Rastrear llamadas salientes (callees)"
echo "═══════════════════════════════════════════════════════"
echo ""

echo "📍 Método: UserService::createUser"
cargo run --quiet --bin inspect_php_callees "${PHP_EXAMPLE}/src/Service/UserService.php" createUser 2>/dev/null
echo ""

echo "📍 Método: UserService::updateUser"
cargo run --quiet --bin inspect_php_callees "${PHP_EXAMPLE}/src/Service/UserService.php" updateUser 2>/dev/null
echo ""

echo "📍 Método: UserService::getUserById"
cargo run --quiet --bin inspect_php_callees "${PHP_EXAMPLE}/src/Service/UserService.php" getUserById 2>/dev/null
echo ""

echo "📍 Método: UserExportService::exportToJson"
cargo run --quiet --bin inspect_php_callees "${PHP_EXAMPLE}/src/Service/UserExportService.php" exportToJson 2>/dev/null
echo ""

echo "📍 Método: UserExportService::exportToCsv"
cargo run --quiet --bin inspect_php_callees "${PHP_EXAMPLE}/src/Service/UserExportService.php" exportToCsv 2>/dev/null
echo ""

echo "═══════════════════════════════════════════════════════"
echo "  3️⃣  TRACE_CALLERS - Rastrear llamadas entrantes (callers)"
echo "═══════════════════════════════════════════════════════"
echo ""

echo "📍 ¿Quién llama a UserRepository::list?"
echo "   En UserService:"
cargo run --quiet --bin inspect_php_callers "${PHP_EXAMPLE}/src/Repository/UserRepository.php" list "${PHP_EXAMPLE}/src/Service/UserService.php" 2>/dev/null
echo ""

echo "   En UserValidationService:"
cargo run --quiet --bin inspect_php_callers "${PHP_EXAMPLE}/src/Repository/UserRepository.php" list "${PHP_EXAMPLE}/src/Service/UserValidationService.php" 2>/dev/null
echo ""

echo "   En UserExportService:"
cargo run --quiet --bin inspect_php_callers "${PHP_EXAMPLE}/src/Repository/UserRepository.php" list "${PHP_EXAMPLE}/src/Service/UserExportService.php" 2>/dev/null
echo ""

echo "📍 ¿Quién llama a UserRepository::findById?"
echo "   En UserService:"
cargo run --quiet --bin inspect_php_callers "${PHP_EXAMPLE}/src/Repository/UserRepository.php" findById "${PHP_EXAMPLE}/src/Service/UserService.php" 2>/dev/null
echo ""

echo "   En UserValidationService:"
cargo run --quiet --bin inspect_php_callers "${PHP_EXAMPLE}/src/Repository/UserRepository.php" findById "${PHP_EXAMPLE}/src/Service/UserValidationService.php" 2>/dev/null
echo ""

echo "📍 ¿Quién llama a UserRepository::save?"
echo "   En UserService:"
cargo run --quiet --bin inspect_php_callers "${PHP_EXAMPLE}/src/Repository/UserRepository.php" save "${PHP_EXAMPLE}/src/Service/UserService.php" 2>/dev/null
echo ""

echo "═══════════════════════════════════════════════════════"
echo "  ✅ Demo completado exitosamente!"
echo "═══════════════════════════════════════════════════════"
echo ""
echo "📊 Resumen de capacidades probadas:"
echo "   ✅ find_symbol: Detecta clases, interfaces, métodos"
echo "   ✅ trace_flow: Rastrea llamadas salientes con contexto"
echo "   ✅ trace_callers: Rastrea impacto de cambios (quién usa qué)"
echo ""
echo "🎯 El soporte de PHP está completamente funcional."
echo ""
