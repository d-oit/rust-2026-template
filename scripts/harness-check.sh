#!/usr/bin/env bash
# harness-check.sh — run a harness sensor with agent-optimised error output
# Usage: ./scripts/harness-check.sh <sensor>
# Sensors: fmt | clippy | deny | test | arch | all
set -euo pipefail

SENSOR=${1:-""}
PASSED="\033[0;32m✅ HARNESS OK\033[0m"
FAILED="\033[0;31m❌ HARNESS VIOLATION\033[0m"

run_with_hint() {
    local name="$1" cmd="$2" hint="$3"
    echo "▶ Running sensor: $name"
    if eval "$cmd"; then
        echo -e "$PASSED [$name]"
        return 0
    else
        echo -e "$FAILED [$name]"
        echo ""
        echo "  AGENT FIX HINT: $hint"
        echo "  See HARNESS.md for the full sensor ↔ guide map."
        return 1
    fi
}

case "$SENSOR" in
  fmt)
    run_with_hint "cargo fmt" \
        "cargo fmt --all -- --check" \
        "Run: cargo fmt --all"
    ;;
  clippy)
    run_with_hint "cargo clippy" \
        "cargo clippy --workspace --all-targets --all-features -- -D warnings" \
        "Fix all warnings. Check .clippy.toml for allowed exceptions. Do not add #[allow(...)] without justification comment."
    ;;
  deny)
    run_with_hint "cargo deny" \
        "cargo deny check" \
        "Check deny.toml for the violation type: license/bans/advisories/sources. For layering: see crate diagram in Cargo.toml workspace comments."
    ;;
  test)
    run_with_hint "cargo nextest" \
        "cargo nextest run --workspace" \
        "Fix the failing test. If behaviour changed intentionally, update the test and run: cargo insta review"
    ;;
  arch)
    run_with_hint "arch fitness" \
        "cargo test --test arch_fitness" \
        "LAYERING VIOLATION: Move code to the correct crate layer. See tests/arch_fitness.rs error for the specific fix."
    ;;
  all)
    run_with_hint "cargo fmt" "cargo fmt --all -- --check" "Run: cargo fmt --all"
    run_with_hint "cargo clippy" "cargo clippy --workspace --all-targets --all-features -- -D warnings" "Fix all warnings. Check .clippy.toml."
    run_with_hint "cargo deny" "cargo deny check" "Check deny.toml for the violation type."
    run_with_hint "cargo nextest" "cargo nextest run --workspace" "Fix the failing test."
    ;;
  *)
    echo "Usage: $0 <fmt|clippy|deny|test|arch|all>"
    exit 1
    ;;
esac
