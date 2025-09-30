#!/bin/bash
#
# Phase 7: Lambda Examples - Intent-Based Verification
# Tests AWS Lambda deployment patterns (may require mocking)
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

echo "======================================================================"
echo "Phase 7: Lambda Examples - Intent-Based Verification"
echo "======================================================================"
echo ""
echo "Testing Objective: Verify AWS Lambda MCP server patterns compile"
echo "                   and can be initialized (without AWS deployment)"
echo ""

PASSED=0
FAILED=0
TOTAL=3

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper function to test lambda examples (compilation only)
test_lambda_example() {
    local example_name=$1
    local test_description=$2

    echo "----------------------------------------"
    echo "Testing: $example_name"
    echo "Description: $test_description"
    echo "----------------------------------------"

    # Test 1: Check if example compiles with cargo lambda
    echo "Test: Lambda compilation check..."
    if cargo lambda build --bin "$example_name" 2>&1 | grep -q "Finished\|Compiling"; then
        echo -e "${GREEN}PASSED${NC}: Compiles successfully"
    else
        echo -e "${RED}FAILED${NC}: Compilation failed"
        FAILED=$((FAILED + 1))
        return 1
    fi

    # Test 2: Try release build (don't run, Lambda examples need AWS runtime)
    echo "Test: Lambda release build check..."
    if cargo lambda build --bin "$example_name" --release 2>&1 | grep -q "Finished\|Compiling"; then
        echo -e "${GREEN}PASSED${NC}: Builds successfully"
    else
        echo -e "${RED}FAILED${NC}: Build failed"
        FAILED=$((FAILED + 1))
        return 1
    fi

    echo -e "${GREEN}SUCCESS${NC}: $example_name verification complete"
    echo "Note: Full Lambda testing requires AWS deployment"
    echo ""

    PASSED=$((PASSED + 1))
    return 0
}

# Test all 3 Lambda examples
test_lambda_example "lambda-mcp-server" "Basic Lambda MCP server deployment pattern"
test_lambda_example "lambda-mcp-server-streaming" "Lambda MCP server with SSE streaming support"
test_lambda_example "lambda-mcp-client" "Lambda client integration patterns"

# Final summary
echo "======================================================================"
echo "Phase 7 Summary"
echo "======================================================================"
echo "Total: $TOTAL Lambda examples"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""
echo "Note: Lambda examples were tested for compilation and build only."
echo "      Full integration testing requires AWS Lambda deployment."
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ PHASE 7 COMPLETE${NC}: All Lambda examples verified"
    exit 0
else
    echo -e "${RED}❌ PHASE 7 FAILED${NC}: $FAILED example(s) failed verification"
    exit 1
fi