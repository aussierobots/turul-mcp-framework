#!/bin/bash
# Protocol Crate Purity Checker
# Ensures protocol crates contain ONLY MCP spec types, no framework features

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PROTOCOL_CRATES=(
    "crates/turul-mcp-protocol"
    "crates/turul-mcp-protocol-2025-06-18"
)

echo "🔍 Checking Protocol Crate Purity..."
echo ""

VIOLATIONS=0

for CRATE in "${PROTOCOL_CRATES[@]}"; do
    CRATE_PATH="$PROJECT_ROOT/$CRATE"

    if [ ! -d "$CRATE_PATH" ]; then
        echo "⚠️  Crate not found: $CRATE"
        continue
    fi

    echo "Checking: $CRATE"

    # Check for forbidden trait hierarchies
    if grep -r "trait Has.*Metadata\|trait.*Definition" "$CRATE_PATH/src" --include="*.rs" 2>/dev/null | grep -v "^Binary\|//"; then
        echo "❌ VIOLATION: Found trait hierarchies in $CRATE"
        echo "   Framework traits must be in turul-mcp-builders/src/traits/"
        VIOLATIONS=$((VIOLATIONS + 1))
    fi

    # Check for builder structs (not builder methods on concrete types)
    if grep -r "pub struct.*Builder\s*{" "$CRATE_PATH/src" --include="*.rs" 2>/dev/null | grep -v "^Binary"; then
        echo "❌ VIOLATION: Found builder structs in $CRATE"
        echo "   Runtime builders must be in turul-mcp-builders/"
        VIOLATIONS=$((VIOLATIONS + 1))
    fi

    # Check for blanket implementations
    if grep -r "impl<T>.*where" "$CRATE_PATH/src" --include="*.rs" 2>/dev/null | grep -v "^Binary\|JsonSchema\|// "; then
        echo "❌ VIOLATION: Found blanket implementations in $CRATE"
        echo "   Framework blanket impls must be in turul-mcp-builders/"
        VIOLATIONS=$((VIOLATIONS + 1))
    fi

    # Check for tutorial comments (very long comment blocks)
    if grep -r "^//.*Framework\|^//.*Level [0-9]" "$CRATE_PATH/src" --include="*.rs" 2>/dev/null | grep -v "^Binary"; then
        echo "⚠️  WARNING: Found tutorial comments in $CRATE"
        echo "   Framework tutorials should be in turul-mcp-builders/docs/"
    fi

    echo "✅ $CRATE checked"
    echo ""
done

if [ $VIOLATIONS -gt 0 ]; then
    echo ""
    echo "❌ PROTOCOL PURITY CHECK FAILED"
    echo "   Found $VIOLATIONS violation(s)"
    echo ""
    echo "Protocol crates must contain ONLY:"
    echo "  ✅ MCP spec types (Tool, Resource, Prompt, etc.)"
    echo "  ✅ Serialization derives"
    echo "  ✅ Basic builder methods on concrete types"
    echo "  ✅ MCP spec error types"
    echo ""
    echo "Framework features belong in turul-mcp-builders:"
    echo "  • Trait hierarchies → turul-mcp-builders/src/traits/"
    echo "  • Runtime builders → turul-mcp-builders/src/"
    echo "  • Blanket impls → turul-mcp-builders/src/traits/"
    echo ""
    exit 1
else
    echo "✅ Protocol Crate Purity Check PASSED"
    echo "   All protocol crates are spec-pure!"
    exit 0
fi
