#!/usr/bin/env bash
# preflight-lint.sh — Non-authoritative grep-based preflight checks
#
# This script performs FAST, LOCAL checks for common Turul MCP Framework violations.
# It is NOT a substitute for the full release gate test suite.
#
# Usage: bash scripts/preflight-lint.sh [directory]
#   directory: path to scan (defaults to current directory)

set -euo pipefail

TARGET="${1:-.}"
ERRORS=0

echo "=== Turul MCP Preflight Lint (non-authoritative) ==="
echo "Scanning: $TARGET"
echo ""

# Check 1: Direct versioned protocol crate references (FORBIDDEN)
echo "--- Check 1: Protocol re-export violations ---"
VIOLATIONS=$(grep -rn 'use turul_mcp_protocol_2025_' "$TARGET/src" 2>/dev/null || true)
if [ -n "$VIOLATIONS" ]; then
    echo "FAIL: Direct versioned protocol crate references found."
    echo "      Use turul_mcp_protocol::* instead."
    echo "$VIOLATIONS"
    ERRORS=$((ERRORS + 1))
else
    echo "PASS"
fi
echo ""

# Check 2: Direct JsonRpcError construction in handlers (FORBIDDEN)
echo "--- Check 2: JsonRpcError construction in business logic ---"
VIOLATIONS=$(grep -rn 'JsonRpcError' "$TARGET/src" 2>/dev/null | grep -v '// Intentional' || true)
if [ -n "$VIOLATIONS" ]; then
    echo "WARN: Possible JsonRpcError construction in business logic."
    echo "      Handlers should return McpError; dispatcher converts automatically."
    echo "$VIOLATIONS"
    ERRORS=$((ERRORS + 1))
else
    echo "PASS"
fi
echo ""

# Check 3: Method strings in tool macros (FORBIDDEN)
echo "--- Check 3: Method string annotations ---"
VIOLATIONS=$(grep -rn 'method\s*=' "$TARGET/src" 2>/dev/null | grep -i 'mcp_tool\|McpTool\|mcp_resource\|mcp_prompt' || true)
if [ -n "$VIOLATIONS" ]; then
    echo "FAIL: Method string annotations found. Framework auto-determines methods."
    echo "$VIOLATIONS"
    ERRORS=$((ERRORS + 1))
else
    echo "PASS"
fi
echo ""

# Check 4: Snake_case JSON fields without serde rename
echo "--- Check 4: Potential snake_case JSON fields ---"
VIOLATIONS=$(grep -rn 'pub [a-z_]*_[a-z]' "$TARGET/src" 2>/dev/null | grep -v 'serde' | grep -v '#\[' | grep -v '//' | head -20 || true)
if [ -n "$VIOLATIONS" ]; then
    echo "INFO: Fields with underscores found — verify they use #[serde(rename = \"camelCase\")]"
    echo "      (May be false positives for Rust-internal fields)"
    echo "$VIOLATIONS" | head -10
else
    echo "PASS"
fi
echo ""

# Check 5: Role::System usage (FORBIDDEN in MCP)
echo "--- Check 5: Role::System usage ---"
VIOLATIONS=$(grep -rn 'Role::System' "$TARGET/src" 2>/dev/null || true)
if [ -n "$VIOLATIONS" ]; then
    echo "FAIL: Role::System is not part of the MCP protocol. Use Role::User or Role::Assistant."
    echo "$VIOLATIONS"
    ERRORS=$((ERRORS + 1))
else
    echo "PASS"
fi
echo ""

# Summary
echo "=== Summary ==="
if [ "$ERRORS" -gt 0 ]; then
    echo "Found $ERRORS issue(s). Review above."
    echo ""
    echo "NOTE: This is a non-authoritative preflight check."
    echo "For full MCP compliance, run the Turul framework release gate tests."
    echo "See: https://github.com/aussierobots/turul-mcp-framework/blob/main/AGENTS.md#release-readiness-notes-2025-10-01"
    exit 1
else
    echo "All preflight checks passed."
    echo ""
    echo "NOTE: This is a non-authoritative preflight check."
    echo "For full MCP compliance, run the Turul framework release gate tests."
    exit 0
fi
