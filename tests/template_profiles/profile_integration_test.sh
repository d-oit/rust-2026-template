#!/usr/bin/env bash
# tests/template_profiles/profile_integration_test.sh
# Fixture-based integration test for template profiles.
# Validates schema correctness across all six profiles and verifies generated
# temporary workspaces via cargo check.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "=== 1. Validating Schema for All Shipped Profiles ==="
PROFILES=("minimal" "library" "cli" "service" "workspace" "ai-agent")

for profile in "${PROFILES[@]}"; do
  profile_path="config/template-profiles/${profile}.toml"
  echo "Validating ${profile_path}..."
  cargo run -p xtask --bin xtask -- template validate-profile --profile "$profile_path"
done

echo "=== 2. Testing Workspace Initialization for Each Profile ==="

TMP_PARENT=$(mktemp -d)
trap 'rm -rf "$TMP_PARENT"' EXIT

# Share target directory across profile checks to avoid re-compiling shared dependencies 6 times
export CARGO_TARGET_DIR="${TMP_PARENT}/shared-target"

for profile in "${PROFILES[@]}"; do
  echo "--- Testing initialization for profile: ${profile} ---"

  TMP_DIR="${TMP_PARENT}/${profile}-workspace"
  mkdir -p "$TMP_DIR"

  # Copy template repository files into temporary workspace (excluding target/ and .git)
  rsync -a --exclude='target' --exclude='.git' "${REPO_ROOT}/" "${TMP_DIR}/"

  (
    cd "$TMP_DIR"
    cargo run -p xtask --bin xtask -- template init \
      --profile "$profile" \
      --name "test-${profile}" \
      --description "Test project for ${profile}" \
      --author "Test Author" \
      --repo "test-org/test-${profile}"

    # Verify minimal profile specific expectations
    if [[ "$profile" == "minimal" ]]; then
      if [[ -d "benchmarks" ]]; then
        echo "Error: minimal profile retained benchmarks directory" >&2
        exit 1
      fi
      if [[ -d "fuzz" ]]; then
        echo "Error: minimal profile retained fuzz directory" >&2
        exit 1
      fi
      if [[ -d "crates/actor-runtime-template" ]]; then
        echo "Error: minimal profile retained actor-runtime-template crate" >&2
        exit 1
      fi
    fi

    # Verify cargo check passes in generated workspace
    echo "Running cargo check on initialized workspace (${profile})..."
    cargo check --workspace
  )
done

echo "=== All template profile integration tests passed successfully! ==="
