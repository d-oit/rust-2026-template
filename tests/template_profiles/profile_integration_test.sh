#!/usr/bin/env bash
# tests/template_profiles/profile_integration_test.sh
# Integration tests for template profiles.
# Validates profile schemas and tests initialization of profiles in temp directories.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# Pre-build xtask in main workspace so `cargo run -p xtask` doesn't rebuild from scratch repeatedly.
cargo build -p xtask

XTASK_BIN="$REPO_ROOT/target/debug/xtask"
export CARGO_TARGET_DIR="$REPO_ROOT/target/profile_tests_target"

PROFILES=("minimal" "library" "cli" "service" "workspace" "ai-agent")

echo "=== 1. Validating all profile TOML files via xtask ==="
for profile in "${PROFILES[@]}"; do
  echo "Validating profile: $profile"
  "$XTASK_BIN" template validate-profile --profile "config/template-profiles/${profile}.toml"
done

echo "=== 2. Testing profile template init in temp workspace ==="
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT

for profile in "${PROFILES[@]}"; do
  echo "--- Testing initialization for profile: $profile ---"
  TEST_WORKSPACE="$TEMP_DIR/test-$profile"
  mkdir -p "$TEST_WORKSPACE"

  # Copy current repo into test workspace (excluding target, .git, and temp files)
  rsync -a --exclude='target' --exclude='.git' --exclude='node_modules' "$REPO_ROOT/" "$TEST_WORKSPACE/"

  (
    cd "$TEST_WORKSPACE"
    # Initialize workspace using compiled xtask binary directly
    "$XTASK_BIN" template init --profile "$profile" --name "my-test-crate" --description "Integration test crate" --author "Test Runner" --repo "test/my-test-crate"

    # Check that workspace builds
    echo "Running cargo check in $profile initialized workspace..."
    cargo check --workspace --all-targets

    # Additional profile assertions
    if [[ "$profile" == "minimal" ]]; then
      echo "Verifying minimal profile drops excluded paths..."
      if [[ -d "benchmarks" ]]; then
        echo "Error: benchmarks path was retained in minimal profile!" >&2
        exit 1
      fi
      if [[ -d "fuzz" ]]; then
        echo "Error: fuzz path was retained in minimal profile!" >&2
        exit 1
      fi
      if [[ -d "crates/actor-runtime-template" ]]; then
        echo "Error: actor-runtime-template crate was retained in minimal profile!" >&2
        exit 1
      fi
    fi
  )
done

echo "=== All template profile integration tests passed successfully! ==="
