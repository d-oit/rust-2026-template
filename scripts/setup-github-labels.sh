#!/usr/bin/env bash
# scripts/setup-github-labels.sh
#
# Creates the full label set for d-oit/rust-2026-template.
# Includes all standard labels plus Rust-specific and release automation labels.
#
# Usage:
#   ./scripts/setup-github-labels.sh
#
# Requirements:
#   gh  — GitHub CLI (https://cli.github.com/)
#   jq  — JSON processor (https://stedolan.github.io/jq/)
#
# The script will prompt before deleting existing labels.
# All `gh label create` calls use --force so re-running is safe (idempotent).

set -euo pipefail

# ── Dependency check ──────────────────────────────────────────────────────────
if ! command -v gh &>/dev/null || ! command -v jq &>/dev/null; then
  echo "Error: GitHub CLI (gh) and jq are required."
  echo "  Install gh:  https://cli.github.com/"
  echo "  Install jq:  https://stedolan.github.io/jq/"
  exit 1
fi

# ── Optional: delete all existing labels first ────────────────────────────────
read -rp "Delete ALL existing labels before creating? (y/N) " confirm

if [[ "$confirm" =~ ^[Yy](es)?$ ]]; then
  echo "Deleting all existing labels..."
  label_names=$(gh label list --limit 200 --json name --jq '.[].name')
  if [[ -n "$label_names" ]]; then
    while IFS= read -r label; do
      [[ -n "$label" ]] || continue
      echo "  Deleting: $label"
      gh label delete "$label" --yes 2>/dev/null || echo "  Failed to delete: $label"
    done <<< "$label_names"
    echo "Deletion complete."
  else
    echo "No existing labels found."
  fi
else
  echo "Skipping deletion — existing labels will be kept."
fi

echo ""
echo "Creating labels..."

# ── Helper ────────────────────────────────────────────────────────────────────
label() {
  # label <name> <color-hex-no-hash> <description>
  gh label create "$1" --color "$2" --description "$3" --force
  echo "  ✓ $1"
}

# ── Type labels ───────────────────────────────────────────────────────────────
label "bug"           "d73a4a" "Something isn't working"
label "feature"       "a2eeef" "New feature or enhancement request"
label "documentation" "0075ca" "Improvements or additions to documentation"
label "question"      "d876e3" "Further information is requested"
label "discussion"    "8b949e" "Open-ended conversation or design discussion"
label "refactor"      "0366d6" "Code improvements without behaviour change"
label "performance"   "5319e7" "Performance-related improvement"
label "tests"         "f4c542" "Related to automated or manual tests"
label "chore"         "fef2c0" "Maintenance task, tooling update, cleanup"
label "deps"          "cfd3d7" "Dependency updates or changes"

# ── Priority labels ───────────────────────────────────────────────────────────
label "priority: high"   "b60205" "Critical, needs immediate attention"
label "priority: medium" "fbca04" "Important but not urgent"
label "priority: low"    "0e8a16" "Low urgency, can wait"

# ── Status labels ─────────────────────────────────────────────────────────────
label "status: in progress"   "1d76db" "Currently being worked on"
label "status: needs review"  "dbab09" "Waiting for review"
label "status: needs triage"  "e4e669" "Needs categorisation or investigation"
label "status: blocked"       "e4e669" "Cannot proceed due to a dependency or blocker"
label "status: duplicate"     "cccccc" "Duplicate of another issue or PR"
label "status: wontfix"       "ffffff" "Not planned to be fixed or implemented"

# ── Security label ────────────────────────────────────────────────────────────
label "security" "b60205" "Security-related issue or vulnerability"

# ── Rust-specific labels ──────────────────────────────────────────────────────
label "rust: unsafe"      "ee0701" "Involves unsafe Rust code — needs extra scrutiny"
label "rust: async"       "6f42c1" "Async / Tokio runtime concern"
label "rust: clippy"      "f9d0c4" "Clippy lint warning or fix"
label "rust: msrv"        "c5def5" "Minimum Supported Rust Version concern"
label "rust: edition"     "bfd4f2" "Rust edition migration or compatibility"
label "rust: no-std"      "e4e669" "no_std compatibility"
label "rust: perf"        "5319e7" "Rust-specific performance optimisation"
label "rust: soundness"   "b60205" "Soundness or undefined behaviour concern"

# ── CI / CD labels ────────────────────────────────────────────────────────────
label "ci: failing"       "d73a4a" "CI pipeline is broken"
label "ci: flaky"         "fbca04" "Intermittently failing CI job"
label "ci: improvement"   "0075ca" "CI pipeline improvement or speed-up"

# ── Release automation labels ─────────────────────────────────────────────────
# `release:patch` is the trigger for .github/workflows/patch-release-on-label.yml
label "release:patch"     "0e8a16" "Trigger: bump patch version on merge (automated)"
label "release: breaking" "b60205" "Contains a breaking API change"
label "release: skip"     "cccccc" "Exclude this PR from release notes"

echo ""
echo "All labels created successfully."
