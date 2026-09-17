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
    
    # Create a temporary fake repository: the generator derives its root from the
    # script location (`dirname $0/..`), so the script must live under `scripts/`.
    TEST_DIR=$(mktemp -d)
    trap 'rm -rf "${TEST_DIR}"' EXIT

    mkdir -p "${TEST_DIR}/scripts"
    cp "${GENERATE_SCRIPT}" "${TEST_DIR}/scripts/"

    # Create only llms.txt (other files will be missing)
    echo "# Test llms.txt" > "${TEST_DIR}/llms.txt"

    # Run script from test directory
    cd "${TEST_DIR}"

    # Capture output
    output=$(bash scripts/generate-llms-txt.sh 2>&1 || true)
    
    # Check if script completed (exit code 0)
    if bash scripts/generate-llms-txt.sh > /dev/null 2>&1; then
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

# Test 3: Verify that the generated llms-full.txt carries the project VERSION
#
# The generator deliberately emits `> Version: <VERSION>` instead of a timestamp
# (commit a991c6b) so the output is deterministic and can be diffed in CI.
test_version_header() {
    echo ""
    echo "Test 3: Verify the project VERSION is carried into the generated file"

    # Run the script
    bash "${GENERATE_SCRIPT}" > /dev/null 2>&1

    # Check if output file exists
    if [[ ! -f "${OUTPUT_FILE}" ]]; then
        print_test_result "Output file exists" "FAIL"
        return 1
    fi

    expected="> Version: $(cat "${REPO_ROOT}/VERSION")"
    if [[ "$(grep -m1 '^> Version: ' "${OUTPUT_FILE}")" == "${expected}" ]]; then
        print_test_result "Version header present: ${expected}" "PASS"
    else
        print_test_result "Version header present: ${expected}" "FAIL"
        return 1
    fi

    # The generated file must stay deterministic: no timestamp may creep back in.
    if grep -qE '[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2} UTC' "${OUTPUT_FILE}"; then
        print_test_result "Generated file contains no timestamp" "FAIL"
        return 1
    fi
    print_test_result "Generated file contains no timestamp" "PASS"

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
    test_version_header
    
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
