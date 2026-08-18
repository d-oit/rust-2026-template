#!/usr/bin/env bash
# tests/template_profiles/test_profiles.sh
# Integration tests for rust-2026-template profiles (issue #286).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

PROFILES=("minimal" "library" "cli" "service" "workspace" "ai-agent")

print_result() {
    local name="$1"
    local status="$2"
    TESTS_RUN=$((TESTS_RUN + 1))
    if [[ "${status}" == "PASS" ]]; then
        echo -e "  ${GREEN}✓${NC} ${name}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "  ${RED}✗${NC} ${name}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

echo "========================================"
echo "Testing Template Profile Blueprints"
echo "========================================"

cd "${REPO_ROOT}"
export CARGO_TARGET_DIR="${REPO_ROOT}/target"

echo ""
echo "Phase 1: Validating Profile Blueprints"
for profile in "${PROFILES[@]}"; do
    if cargo run -q -p xtask --bin xtask -- template validate-profile --profile "config/template-profiles/${profile}.toml"; then
        print_result "Validate profile: ${profile}" "PASS"
    else
        print_result "Validate profile: ${profile}" "FAIL"
    fi
done

echo ""
echo "Phase 2: Inspecting Profiles"
for profile in "${PROFILES[@]}"; do
    if cargo run -q -p xtask --bin xtask -- template inspect --profile "${profile}" > /dev/null; then
        print_result "Inspect profile: ${profile}" "PASS"
    else
        print_result "Inspect profile: ${profile}" "FAIL"
    fi
done

echo ""
echo "Phase 3: Testing Wrapper & Dry Run"
if ./scripts/init-template.sh --minimal --dry-run --name "dry-app" --description "Dry run test" --author "Tester" --repo "org/dry-app" > /dev/null; then
    print_result "scripts/init-template.sh --minimal --dry-run" "PASS"
else
    print_result "scripts/init-template.sh --minimal --dry-run" "FAIL"
fi

echo ""
echo "Phase 4: Workspace Generation and Buildability Tests"

TEMP_BASE=$(mktemp -d)
trap "rm -rf '${TEMP_BASE}'" EXIT

for profile in "${PROFILES[@]}"; do
    echo "  -> Testing initialization for profile '${profile}'..."
    TARGET_DIR="${TEMP_BASE}/${profile}"
    mkdir -p "${TARGET_DIR}"

    # Copy template repository files excluding build artifacts and .git
    rsync -a --exclude='target' --exclude='.git' "${REPO_ROOT}/" "${TARGET_DIR}/"

    (
        cd "${TARGET_DIR}"

        # Initialize profile
        if cargo run -q -p xtask --bin xtask -- template init --profile "${profile}" --name "test-app" --description "Test project" --author "Test Author" --repo "testorg/test-app" > /dev/null; then
            # Verify buildability
            if cargo check --workspace > /dev/null 2>&1; then
                # Verify profile specific criteria
                case "${profile}" in
                    minimal)
                        if [[ ! -d "benchmarks" && ! -d "fuzz" && ! -d "crates/mcp-server-template" && ! -f ".github/workflows/fuzz.yml" ]]; then
                            print_result "Profile generation & verification: ${profile}" "PASS"
                        else
                            print_result "Profile generation & verification: ${profile} (minimal files retained)" "FAIL"
                        fi
                        ;;
                    library)
                        if [[ ! -d "crates/sample-app" && -d "crates/test-app" ]]; then
                            print_result "Profile generation & verification: ${profile}" "PASS"
                        else
                            print_result "Profile generation & verification: ${profile} (library shape mismatch)" "FAIL"
                        fi
                        ;;
                    *)
                        print_result "Profile generation & verification: ${profile}" "PASS"
                        ;;
                esac
            else
                print_result "Profile generation & verification: ${profile} (build/test failed)" "FAIL"
            fi
        else
            print_result "Profile initialization: ${profile} (init failed)" "FAIL"
        fi
    )
done

echo ""
echo "========================================"
echo "Profile Test Summary"
echo "========================================"
echo "Tests run: ${TESTS_RUN}"
echo -e "${GREEN}Passed: ${TESTS_PASSED}${NC}"
if [[ ${TESTS_FAILED} -gt 0 ]]; then
    echo -e "${RED}Failed: ${TESTS_FAILED}${NC}"
    exit 1
else
    echo -e "${GREEN}All template profile integration tests passed!${NC}"
    exit 0
fi
