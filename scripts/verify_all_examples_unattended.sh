#!/bin/bash
#
# Verify All Example Servers - Unattended Mode
# Runs all verification scripts without prompts and collects results

set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

echo "======================================================================"
echo "Verifying All Examples - Complete Verification Campaign"
echo "======================================================================"
echo ""

RESULTS_FILE="/tmp/all_examples_results.txt"
> "$RESULTS_FILE"

# Define verification scripts in order. Lambda and meta examples are not
# here: verify_lambda_examples.sh and verify_meta_examples.sh were removed
# as superseded/orphaned (see scripts/ci-gates.sh comments) — Lambda
# coverage lives in scripts/test_lambda_middleware*.sh and
# scripts/e2e-lambda-*.sh instead.
declare -a SCRIPTS=(
    "verify_calculator_examples.sh"
    "verify_resource_servers.sh"
    "verify_prompts_examples.sh"
    "verify_storage_backends.sh"
    "verify_advanced_servers.sh"
    "verify_client_examples.sh"
)

declare -a TITLES=(
    "Calculator Learning Progression"
    "Resource Servers"
    "Prompts & Special Features"
    "Session Storage Backends"
    "Advanced/Composite Servers"
    "Clients & Test Utilities"
)

fail=0

for i in "${!SCRIPTS[@]}"; do
    phase=$((i + 1))
    script="${SCRIPTS[$i]}"
    title="${TITLES[$i]}"

    echo ""
    echo "======================================================================" | tee -a "$RESULTS_FILE"
    echo "Verification $phase: $title" | tee -a "$RESULTS_FILE"
    echo "======================================================================" | tee -a "$RESULTS_FILE"

    bash "scripts/$script" 2>&1 | tee -a "/tmp/verify_${phase}_full.log" | tail -40 | tee -a "$RESULTS_FILE"

    # Extract summary
    PHASE_EXIT=${PIPESTATUS[0]}
    if [ "$PHASE_EXIT" -eq 0 ]; then
        echo "✅ Verification $phase PASSED" | tee -a "$RESULTS_FILE"
    else
        echo "❌ Verification $phase FAILED" | tee -a "$RESULTS_FILE"
        fail=1
    fi
    echo "" | tee -a "$RESULTS_FILE"
done

echo "======================================================================"
echo "All Verifications Complete - Results saved to $RESULTS_FILE"
echo "======================================================================"
grep -E "^(✅|❌|Verification [0-9]+ Summary)" "$RESULTS_FILE"
exit "$fail"
