#!/bin/bash
# Test all MCP examples

cd /Users/nick/turul-mcp-framework

echo "======================================"
echo "EXAMPLE VERIFICATION - Full Test Run"
echo "======================================"
echo ""

# Build all first
echo "Building all examples..."
cargo build --workspace --bins --examples > /dev/null 2>&1
echo "✅ Build complete"
echo ""

# Test Phase 1: Core Calculators
echo "=== PHASE 1: Core Calculator Examples ==="
bash scripts/verify_phase1.sh 2>&1 | grep -E "(Testing:|PASSED|FAILED|Phase 1 Summary)"
echo ""

# Test Phase 2: Resources
echo "=== PHASE 2: Resource Examples ==="
bash scripts/verify_phase2.sh 2>&1 | grep -E "(Testing:|PASSED|FAILED|Phase 2 Summary)"
echo ""

# Test Phase 3: Protocol Features
echo "=== PHASE 3: Protocol Features ==="
bash scripts/verify_phase3.sh 2>&1 | grep -E "(Testing:|PASSED|FAILED|Phase 3 Summary)"
echo ""

# Test Phase 4: Session Storage
echo "=== PHASE 4: Session Storage ==="
bash scripts/verify_phase4.sh 2>&1 | grep -E "(Testing:|PASSED|FAILED|Phase 4 Summary)"
echo ""

# Test Phase 5: Advanced
echo "=== PHASE 5: Advanced Examples ==="
bash scripts/verify_phase5.sh 2>&1 | grep -E "(Testing:|PASSED|FAILED|Phase 5 Summary)"
echo ""

echo "======================================"
echo "VERIFICATION COMPLETE"
echo "======================================"
