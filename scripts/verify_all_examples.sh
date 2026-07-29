#!/bin/bash
#
# Master Script: Verify All Example Servers
# Executes all verification scripts and generates comprehensive report
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}======================================================================"
echo "Turul MCP Framework - Example Verification Campaign"
echo "======================================================================${NC}"
echo ""
echo "Starting comprehensive verification of all 44+ examples..."
echo "Date: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# Phase tracking
TOTAL_PHASES=8
PASSED_PHASES=0
FAILED_PHASES=0

# Result storage
declare -a PHASE_RESULTS
declare -a PHASE_NAMES

# Run each phase
run_phase() {
    local phase_num=$1
    local phase_name=$2
    local phase_script=$3

    echo -e "${BLUE}======================================================================"
    echo "PHASE $phase_num: $phase_name"
    echo "======================================================================${NC}"
    echo ""

    PHASE_NAMES[$phase_num]="$phase_name"

    if [ ! -f "$phase_script" ]; then
        echo -e "${RED}ERROR${NC}: Phase script not found: $phase_script"
        PHASE_RESULTS[$phase_num]="MISSING"
        FAILED_PHASES=$((FAILED_PHASES + 1))
        return 1
    fi

    # Run phase script and capture result
    if bash "$phase_script"; then
        PHASE_RESULTS[$phase_num]="PASSED"
        PASSED_PHASES=$((PASSED_PHASES + 1))
        echo -e "${GREEN}✅ Phase $phase_num PASSED${NC}"
    else
        PHASE_RESULTS[$phase_num]="FAILED"
        FAILED_PHASES=$((FAILED_PHASES + 1))
        echo -e "${RED}❌ Phase $phase_num FAILED${NC}"
    fi

    echo ""
    echo "Press Enter to continue to next phase..."
    read -r
}

# Execute all verification scripts
run_phase 1 "Calculator Learning Progression" "$SCRIPT_DIR/verify_calculator_examples.sh"
run_phase 2 "Resource Servers" "$SCRIPT_DIR/verify_resource_servers.sh"
run_phase 3 "Prompts & Special Features" "$SCRIPT_DIR/verify_prompts_examples.sh"
run_phase 4 "Session Storage Backends" "$SCRIPT_DIR/verify_storage_backends.sh"
run_phase 5 "Advanced/Composite Servers" "$SCRIPT_DIR/verify_advanced_servers.sh"
run_phase 6 "Clients & Test Utilities" "$SCRIPT_DIR/verify_client_examples.sh"
run_phase 7 "Lambda Examples" "$SCRIPT_DIR/verify_lambda_examples.sh"
run_phase 8 "Meta Examples" "$SCRIPT_DIR/verify_meta_examples.sh"

# Generate final report
echo -e "${BLUE}======================================================================"
echo "FINAL VERIFICATION REPORT"
echo "======================================================================${NC}"
echo ""
echo "Verification completed at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""
echo "Phase Results:"
echo "----------------------------------------"

for i in {1..8}; do
    phase_name="${PHASE_NAMES[$i]}"
    phase_result="${PHASE_RESULTS[$i]}"

    if [ "$phase_result" = "PASSED" ]; then
        echo -e "Phase $i: ${GREEN}✅ PASSED${NC} - $phase_name"
    elif [ "$phase_result" = "FAILED" ]; then
        echo -e "Phase $i: ${RED}❌ FAILED${NC} - $phase_name"
    else
        echo -e "Phase $i: ${YELLOW}⚠️  MISSING${NC} - $phase_name"
    fi
done

echo ""
echo "Summary:"
echo "----------------------------------------"
echo "Total Phases:  $TOTAL_PHASES"
echo -e "Passed:        ${GREEN}$PASSED_PHASES${NC}"
echo -e "Failed:        ${RED}$FAILED_PHASES${NC}"
echo ""

# Overall result
if [ $FAILED_PHASES -eq 0 ]; then
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}✅ ALL PHASES PASSED${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "All 44+ examples verified successfully!"
    exit 0
else
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}❌ $FAILED_PHASES PHASE(S) FAILED${NC}"
    echo -e "${RED}========================================${NC}"
    echo ""
    echo "Please review the failed phases above."
    exit 1
fi