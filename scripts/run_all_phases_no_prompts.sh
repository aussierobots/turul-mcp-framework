#!/bin/bash
#
# Run all 8 phases without prompts and collect results
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

echo "======================================================================"
echo "Running All 8 Phases - Complete Verification Campaign"
echo "======================================================================"
echo ""

RESULTS_FILE="/tmp/all_phases_results.txt"
> "$RESULTS_FILE"

for phase in 1 2 3 4 5 6 7 8; do
    echo ""
    echo "======================================================================" | tee -a "$RESULTS_FILE"
    echo "PHASE $phase" | tee -a "$RESULTS_FILE"
    echo "======================================================================" | tee -a "$RESULTS_FILE"

    bash scripts/verify_phase$phase.sh 2>&1 | tee -a "/tmp/phase${phase}_full.log" | tail -40 | tee -a "$RESULTS_FILE"

    # Extract summary
    PHASE_EXIT=${PIPESTATUS[0]}
    if [ $PHASE_EXIT -eq 0 ]; then
        echo "✅ Phase $phase PASSED" | tee -a "$RESULTS_FILE"
    else
        echo "❌ Phase $phase FAILED" | tee -a "$RESULTS_FILE"
    fi
    echo "" | tee -a "$RESULTS_FILE"
done

echo "======================================================================"
echo "All Phases Complete - Results saved to $RESULTS_FILE"
echo "======================================================================"
cat "$RESULTS_FILE" | grep -E "^(✅|❌|Phase [0-9] Summary)"