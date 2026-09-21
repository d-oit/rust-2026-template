#!/usr/bin/env bash
# tests/distill_strikes_test.sh — Test suite for distill-strikes.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== Running distill-strikes.sh Test Suite ==="

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

MOCK_MATRIX="$TMP_DIR/mock-matrix.json"
MOCK_DRAFTS="$TMP_DIR/drafts"
MOCK_SKILLS="$TMP_DIR/skills"
mkdir -p "$MOCK_DRAFTS" "$MOCK_SKILLS"

cat <<'EOF' > "$MOCK_MATRIX"
{
  "version": "1.0",
  "cases": [
    {
      "id": "case-1",
      "workflow": "CI",
      "run_id": "1001",
      "subsystem": "test-sensor",
      "summary": "First failure of test-sensor",
      "reproduction_command": "cargo test --test foo",
      "detection_signatures": ["test_foo_failed", "assertion failed"],
      "remediation_hint": "Fix the failing assertion in tests/foo.rs."
    },
    {
      "id": "case-2",
      "workflow": "CI",
      "run_id": "1002",
      "subsystem": "test-sensor",
      "summary": "Second failure of test-sensor",
      "reproduction_command": "cargo test --test foo",
      "detection_signatures": ["test_foo_failed"],
      "remediation_hint": "Fix the failing assertion in tests/foo.rs."
    },
    {
      "id": "case-3",
      "workflow": "CI",
      "run_id": "1003",
      "subsystem": "test-sensor",
      "summary": "Third failure of test-sensor",
      "reproduction_command": "cargo test --test foo",
      "detection_signatures": ["test_foo_failed"],
      "remediation_hint": "Fix the failing assertion in tests/foo.rs."
    },
    {
      "id": "case-single",
      "workflow": "CI",
      "run_id": "1004",
      "subsystem": "one-off-sensor",
      "summary": "Single failure of one-off-sensor",
      "reproduction_command": "cargo check",
      "detection_signatures": ["check_failed"],
      "remediation_hint": "Do not scaffold single failures."
    }
  ]
}
EOF

echo "[1/4] Testing threshold detection (3x strikes trigger draft generation)..."
OUTPUT=$(./scripts/distill-strikes.sh --threshold 3 --matrix "$MOCK_MATRIX" --output-dir "$MOCK_DRAFTS" --skills-dir "$MOCK_SKILLS")
echo "$OUTPUT"

if [ ! -f "$MOCK_DRAFTS/test-sensor/SKILL.md" ]; then
  echo "ERROR: Expected draft skill $MOCK_DRAFTS/test-sensor/SKILL.md was not created."
  exit 1
fi

if [ ! -f "$MOCK_DRAFTS/test-sensor/fixtures/red_fixture.sample" ]; then
  echo "ERROR: Expected red fixture $MOCK_DRAFTS/test-sensor/fixtures/red_fixture.sample was not created."
  exit 1
fi

if [ -d "$MOCK_DRAFTS/one-off-sensor" ]; then
  echo "ERROR: Draft created for one-off-sensor which only had 1 strike (threshold was 3)."
  exit 1
fi
echo "✓ Threshold detection test passed."

echo "[2/4] Verifying verbatim failure output & HARNESS VIOLATION hints..."
SKILL_CONTENT=$(cat "$MOCK_DRAFTS/test-sensor/SKILL.md")
FIXTURE_CONTENT=$(cat "$MOCK_DRAFTS/test-sensor/fixtures/red_fixture.sample")

if ! echo "$SKILL_CONTENT" | grep -q "Fix the failing assertion in tests/foo.rs."; then
  echo "ERROR: Verbatim remediation hint missing from generated SKILL.md."
  exit 1
fi

if ! echo "$SKILL_CONTENT" | grep -q "test_foo_failed"; then
  echo "ERROR: Verbatim detection signature missing from generated SKILL.md."
  exit 1
fi

if ! echo "$FIXTURE_CONTENT" | grep -q "Fix the failing assertion in tests/foo.rs."; then
  echo "ERROR: Verbatim remediation hint missing from generated red fixture."
  exit 1
fi

echo "✓ Verbatim content test passed."

echo "[3/4] Testing existing skill protection (never overwrite existing skills)..."
# Create existing skill in MOCK_SKILLS/test-sensor
mkdir -p "$MOCK_SKILLS/test-sensor"
echo "Existing skill content" > "$MOCK_SKILLS/test-sensor/SKILL.md"

# Clear drafts dir
rm -rf "${MOCK_DRAFTS:?}"/*

OVERWRITE_OUTPUT=$(./scripts/distill-strikes.sh --threshold 3 --matrix "$MOCK_MATRIX" --output-dir "$MOCK_DRAFTS" --skills-dir "$MOCK_SKILLS")
echo "$OVERWRITE_OUTPUT"

if echo "$OVERWRITE_OUTPUT" | grep -q "REFUSED: Existing skill already exists"; then
  echo "✓ Refusal to overwrite existing skill confirmed."
else
  echo "ERROR: Did not detect refusal to overwrite existing skill."
  exit 1
fi

if [ -f "$MOCK_DRAFTS/test-sensor/SKILL.md" ]; then
  echo "ERROR: Overwrote or re-drafted skill despite existing skill presence."
  exit 1
fi

echo "[4/4] Validating frontmatter structure of drafted skills..."
# Clear mock skills and re-run to test frontmatter validation
rm -rf "$MOCK_SKILLS/test-sensor"
rm -rf "${MOCK_DRAFTS:?}"/*
./scripts/distill-strikes.sh --threshold 3 --matrix "$MOCK_MATRIX" --output-dir "$MOCK_DRAFTS" --skills-dir "$MOCK_SKILLS" >/dev/null

# Symlink or test frontmatter check directly
python3 -c "
import json, sys

with open('$MOCK_DRAFTS/test-sensor/SKILL.md') as f:
    lines = f.readlines()

assert lines[0].strip() == '---', 'Frontmatter header missing'
assert any(l.startswith('name: test-sensor') for l in lines), 'Skill name missing or incorrect'
assert any(l.startswith('category: rust') for l in lines), 'Skill category missing'
"
echo "✓ Frontmatter structure validated."

echo "=== All distill-strikes.sh tests PASSED successfully ==="
