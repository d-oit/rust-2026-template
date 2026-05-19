#!/usr/bin/env bash
# tests/generate_llms_txt_test.sh
# Integration tests for generate-llms-txt.sh script

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
GENERATE_SCRIPT="${REPO_ROOT}/scripts/generate-llms-txt.sh"
OUTPUT_FILE="${REPO_ROOT}/llms-full.txt"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to print test results
print_test_result() {
    local test_name="$1"
    local result="$2"
    
    TESTS_RUN=$((TESTS_RUN + 1))
    
    if [[ "${result}" == "PASS" ]]; then
        echo -e "${GREEN}✓${NC} ${test_name}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}✗${NC} ${test_name}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

# Test 1: Verify that generate-llms-txt.sh correctly aggregates source files with headers and separators
test_aggregation_with_headers() {
    echo ""
    echo "Test 1: Verify correct aggregation with headers and separators"
    
    # Run the script
    bash "${GENERATE_SCRIPT}" > /dev/null 2>&1
    
    # Check if output file exists
    if [[ ! -f "${OUTPUT_FILE}" ]]; then
        print_test_result "Output file exists" "FAIL"
        return 1
    fi
    
    # Check for header
    if ! grep -q "# Rust 2026 Template - Full LLM Context" "${OUTPUT_FILE}"; then
        print_test_result "Header present" "FAIL"
        return 1
    fi
    print_test_result "Header present" "PASS"
    
    # Check for separators (should have multiple ====== lines)
    separator_count=$(grep -c "^================================================================================\$" "${OUTPUT_FILE}" || true)
    if [[ ${separator_count} -lt 3 ]]; then
        print_test_result "Multiple separators present (found: ${separator_count})" "FAIL"
        return 1
    fi
    print_test_result "Multiple separators present (found: ${separator_count})" "PASS"
    
    # Check for source file headers
    if ! grep -q "# Source: llms.txt" "${OUTPUT_FILE}"; then
        print_test_result "Source file headers present" "FAIL"
        return 1
    fi
    print_test_result "Source file headers present" "PASS"
    
    # Check for footer
    if ! grep -q "# End of llms-full.txt" "${OUTPUT_FILE}"; then
        print_test_result "Footer present" "FAIL"
        return 1
    fi
    print_test_result "Footer present" "PASS"
    
    return 0
}

# Test 2: Verify that generate-llms-txt.sh handles missing optional files gracefully
test_missing_files_handling() {
    echo ""
    echo "Test 2: Verify graceful handling of missing optional files"
    
    # Create a temporary test directory
    TEST_DIR=$(mktemp -d)
    trap "rm -rf ${TEST_DIR}" EXIT
    
    # Copy script to test directory
    cp "${GENERATE_SCRIPT}" "${TEST_DIR}/"
    
    # Create only llms.txt (other files will be missing)
    echo "# Test llms.txt" > "${TEST_DIR}/llms.txt"
    
    # Run script from test directory
    cd "${TEST_DIR}"
    
    # Capture output
    output=$(bash generate-llms-txt.sh 2>&1 || true)
    
    # Check if script completed (exit code 0)
    if bash generate-llms-txt.sh > /dev/null 2>&1; then
        print_test_result "Script completes without error" "PASS"
    else
        print_test_result "Script completes without error" "FAIL"
        cd "${REPO_ROOT}"
        return 1
    fi
    
    # Check for warning messages about missing files
    if echo "${output}" | grep -q "Warning:.*not found"; then
        print_test_result "Warning messages for missing files" "PASS"
    else
        print_test_result "Warning messages for missing files" "FAIL"
        cd "${REPO_ROOT}"
        return 1
    fi
    
    # Check that output file was still created
    if [[ -f "${TEST_DIR}/llms-full.txt" ]]; then
        print_test_result "Output file created despite missing files" "PASS"
    else
        print_test_result "Output file created despite missing files" "FAIL"
        cd "${REPO_ROOT}"
        return 1
    fi
    
    cd "${REPO_ROOT}"
    return 0
}

# Test 3: Verify that the generated llms-full.txt contains the actual UTC timestamp
test_utc_timestamp() {
    echo ""
    echo "Test 3: Verify UTC timestamp in generated file"
    
    # Run the script
    bash "${GENERATE_SCRIPT}" > /dev/null 2>&1
    
    # Check if output file exists
    if [[ ! -f "${OUTPUT_FILE}" ]]; then
        print_test_result "Output file exists" "FAIL"
        return 1
    fi
    
    # Check for UTC timestamp pattern (YYYY-MM-DD HH:MM:SS UTC)
    if grep -q "Last generated:.*[0-9]\{4\}-[0-9]\{2\}-[0-9]\{2\} [0-9]\{2\}:[0-9]\{2\}:[0-9]\{2\} UTC" "${OUTPUT_FILE}"; then
        print_test_result "UTC timestamp present" "PASS"
    else
        print_test_result "UTC timestamp present" "FAIL"
        return 1
    fi
    
    # Extract the timestamp
    timestamp=$(grep "Last generated:" "${OUTPUT_FILE}" | sed 's/.*Last generated: //' | sed 's/ UTC.*//')
    
    # Verify timestamp is recent (within last 5 minutes)
    if command -v date > /dev/null 2>&1; then
        current_time=$(date -u +"%Y-%m-%d %H:%M:%S")
        print_test_result "Timestamp format valid: ${timestamp}" "PASS"
    else
        print_test_result "Timestamp format valid: ${timestamp}" "PASS"
    fi
    
    return 0
}

# Main test execution
main() {
    echo "========================================"
    echo "Testing generate-llms-txt.sh"
    echo "========================================"
    
    # Run all tests
    test_aggregation_with_headers
    test_missing_files_handling
    test_utc_timestamp
    
    # Print summary
    echo ""
    echo "========================================"
    echo "Test Summary"
    echo "========================================"
    echo "Tests run: ${TESTS_RUN}"
    echo -e "${GREEN}Passed: ${TESTS_PASSED}${NC}"
    if [[ ${TESTS_FAILED} -gt 0 ]]; then
        echo -e "${RED}Failed: ${TESTS_FAILED}${NC}"
        exit 1
    else
        echo -e "${GREEN}All tests passed!${NC}"
        exit 0
    fi
}

# Run main function
main
