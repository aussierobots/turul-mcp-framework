#!/bin/bash
# Test all MCP examples

cd "$(dirname "$0")/.."

echo "======================================"
echo "EXAMPLE VERIFICATION - Full Test Run"
echo "======================================"
echo ""

# Build all first
echo "Building all examples..."
cargo build --workspace --bins --examples > /dev/null 2>&1
echo "✅ Build complete"
echo ""

# Test 1: Calculator Examples
echo "=== Calculator Learning Progression ==="
bash scripts/verify_calculator_examples.sh 2>&1 | grep -E "(Testing:|PASSED|FAILED|Summary)"
echo ""

# Test 2: Resource Examples
echo "=== Resource Servers ==="
bash scripts/verify_resource_servers.sh 2>&1 | grep -E "(Testing:|PASSED|FAILED|Summary)"
echo ""

# Test 3: Prompt Examples
echo "=== Prompts & Special Features ==="
bash scripts/verify_prompts_examples.sh 2>&1 | grep -E "(Testing:|PASSED|FAILED|Summary)"
echo ""

# Test 4: Session Storage
echo "=== Session Storage Backends ==="
bash scripts/verify_storage_backends.sh 2>&1 | grep -E "(Testing:|PASSED|FAILED|Summary)"
echo ""

# Test 5: Advanced Examples
echo "=== Advanced/Composite Servers ==="
bash scripts/verify_advanced_servers.sh 2>&1 | grep -E "(Testing:|PASSED|FAILED|Summary)"
echo ""

echo "======================================"
echo "VERIFICATION COMPLETE"
echo "======================================"
