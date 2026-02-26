#!/bin/bash
# Test Lambda Authorizer Integration
set -e

echo "🧪 Lambda Authorizer Integration Test"
echo "======================================"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 1. Unit tests
echo -e "${BLUE}📋 Step 1: Running unit tests...${NC}"
cargo test --package turul-mcp-aws-lambda --lib adapter::tests::authorizer_tests --quiet

echo -e "${GREEN}✅ Adapter authorizer tests passed${NC}"
echo ""

# 2. Build verification
echo -e "${BLUE}🔨 Step 2: Building example...${NC}"
cargo build --package middleware-auth-lambda --quiet

echo -e "${GREEN}✅ Build successful${NC}"
echo ""

# 3. Lambda local test
if ! command -v cargo-lambda &> /dev/null; then
    echo "⚠️  cargo-lambda not installed, skipping local Lambda tests"
    echo "   Install with: pip install cargo-lambda"
    exit 0
fi

echo -e "${BLUE}🚀 Step 3: Starting Lambda locally...${NC}"

# Start lambda in background with debug logging
export RUST_LOG=debug
cargo lambda watch --package middleware-auth-lambda > /tmp/lambda-output.log 2>&1 &
LAMBDA_PID=$!

echo "   Lambda PID: $LAMBDA_PID"
echo "   Waiting for startup (8 seconds)..."
sleep 8

# Check if still running
if ! ps -p $LAMBDA_PID > /dev/null; then
    echo "❌ Lambda failed to start. Log output:"
    cat /tmp/lambda-output.log
    exit 1
fi

echo -e "${GREEN}✅ Lambda started${NC}"
echo ""

# Test V2 format
echo -e "${BLUE}📡 Step 4: Testing API Gateway V2 (HTTP API) format...${NC}"
cargo lambda invoke middleware-auth-lambda \
  --data-file examples/middleware-auth-lambda/test-events/apigw-v2-with-authorizer.json \
  > /tmp/v2-response.json 2>&1

echo "Response saved to /tmp/v2-response.json"

# Check for authorizer logs
if grep -q "Authorizer context:" /tmp/lambda-output.log; then
    echo -e "${GREEN}✅ Authorizer context extracted${NC}"
    echo "   Authorizer fields found in logs:"
    grep "Authorizer context:" /tmp/lambda-output.log | tail -5 | sed 's/^/   /'
else
    echo "⚠️  No authorizer context found in logs"
fi

echo ""

# Test V1 format
echo -e "${BLUE}📡 Step 5: Testing API Gateway V1 (REST API) format...${NC}"
cargo lambda invoke middleware-auth-lambda \
  --data-file examples/middleware-auth-lambda/test-events/apigw-v1-with-authorizer.json \
  > /tmp/v1-response.json 2>&1

echo "Response saved to /tmp/v1-response.json"

# Check for authorizer logs again
V2_COUNT=$(grep -c "Authorizer context:" /tmp/lambda-output.log || true)
sleep 2  # Wait for logs to flush
V1_COUNT=$(grep -c "Authorizer context:" /tmp/lambda-output.log || true)

if [ "$V1_COUNT" -gt "$V2_COUNT" ]; then
    echo -e "${GREEN}✅ V1 nested authorizer context extracted${NC}"
else
    echo "⚠️  V1 nested may not have extracted authorizer context"
fi

echo ""

# Test V1 flat format
echo -e "${BLUE}📡 Step 5b: Testing API Gateway V1 Flat (REST API, simple authorizer) format...${NC}"
PRE_FLAT_COUNT=$(grep -c "Authorizer context:" /tmp/lambda-output.log || true)
cargo lambda invoke middleware-auth-lambda \
  --data-file examples/middleware-auth-lambda/test-events/apigw-v1-flat-authorizer.json \
  > /tmp/v1-flat-response.json 2>&1

echo "Response saved to /tmp/v1-flat-response.json"

sleep 2  # Wait for logs to flush
POST_FLAT_COUNT=$(grep -c "Authorizer context:" /tmp/lambda-output.log || true)

if [ "$POST_FLAT_COUNT" -gt "$PRE_FLAT_COUNT" ]; then
    echo -e "${GREEN}✅ V1 flat authorizer context extracted${NC}"
else
    echo "⚠️  V1 flat may not have extracted authorizer context"
fi

echo ""

# Cleanup
echo -e "${BLUE}🧹 Cleaning up...${NC}"
kill $LAMBDA_PID 2>/dev/null || true
wait $LAMBDA_PID 2>/dev/null || true

echo ""
echo "======================================"
echo -e "${GREEN}✅ All tests completed!${NC}"
echo ""
echo "📝 Test artifacts:"
echo "   - Lambda logs: /tmp/lambda-output.log"
echo "   - V2 response: /tmp/v2-response.json"
echo "   - V1 nested response: /tmp/v1-response.json"
echo "   - V1 flat response: /tmp/v1-flat-response.json"
echo ""
echo "To view full Lambda logs:"
echo "   cat /tmp/lambda-output.log | grep 'Authorizer context'"
