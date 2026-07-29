#!/bin/bash
# Test Lambda middleware authentication example with cargo lambda watch

set -e

echo "🧪 Testing Lambda Middleware Authentication Example"
echo "===================================================="
echo ""

# Check if cargo lambda is installed
if ! command -v cargo-lambda &> /dev/null; then
    echo "❌ cargo-lambda not found. Install with: cargo install cargo-lambda"
    exit 1
fi

echo "📦 Building Lambda middleware example..."
cargo lambda build --release --package middleware-auth-lambda

if [ $? -ne 0 ]; then
    echo "❌ Failed to build middleware-auth-lambda"
    exit 1
fi

echo "✅ Lambda middleware example built successfully"
echo ""
echo "🚀 To test locally, run in separate terminal:"
echo "   cargo lambda watch --package middleware-auth-lambda"
echo ""
echo "Then run these tests (2026-07-28 stateless: no initialize/session, every"
echo "request carries MCP-Protocol-Version + Mcp-Method and a _meta block):"
echo ""
echo "# Test 1: server/discover without API key (should succeed - discover skips auth)"
echo "curl -X POST http://localhost:9000/lambda-url/middleware-auth-lambda \\"
echo "  -H 'Content-Type: application/json' \\"
echo "  -H 'Accept: application/json' \\"
echo "  -H 'MCP-Protocol-Version: 2026-07-28' \\"
echo "  -H 'Mcp-Method: server/discover' \\"
echo "  -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\",\"params\":{\"_meta\":{\"io.modelcontextprotocol/protocolVersion\":\"2026-07-28\",\"io.modelcontextprotocol/clientCapabilities\":{}}}}'"
echo ""
echo "# Test 2: tools/list without API key (should fail with -32001)"
echo "curl -X POST http://localhost:9000/lambda-url/middleware-auth-lambda \\"
echo "  -H 'Content-Type: application/json' \\"
echo "  -H 'Accept: application/json' \\"
echo "  -H 'MCP-Protocol-Version: 2026-07-28' \\"
echo "  -H 'Mcp-Method: tools/list' \\"
echo "  -d '{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{\"_meta\":{\"io.modelcontextprotocol/protocolVersion\":\"2026-07-28\",\"io.modelcontextprotocol/clientCapabilities\":{}}}}'"
echo ""
echo "# Test 3: tools/list with valid API key (should succeed; needs a live"
echo "# 'mcp-sessions' DynamoDB table, see scripts/test_lambda_middleware_live.sh)"
echo "curl -X POST http://localhost:9000/lambda-url/middleware-auth-lambda \\"
echo "  -H 'Content-Type: application/json' \\"
echo "  -H 'Accept: application/json' \\"
echo "  -H 'MCP-Protocol-Version: 2026-07-28' \\"
echo "  -H 'Mcp-Method: tools/list' \\"
echo "  -H 'X-API-Key: secret-key-123' \\"
echo "  -d '{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/list\",\"params\":{\"_meta\":{\"io.modelcontextprotocol/protocolVersion\":\"2026-07-28\",\"io.modelcontextprotocol/clientCapabilities\":{}}}}'"
echo ""
echo "Valid API keys:"
echo "  - secret-key-123 (user-alice)"
echo "  - secret-key-456 (user-bob)"
echo ""
echo "===================================================="
echo "✅ Build verification complete"
echo ""
echo "NOTE: Full Lambda testing requires AWS credentials and DynamoDB."
echo "      For local testing, use 'cargo lambda watch' (see above)."
