#!/usr/bin/env bash
# scripts/learn-labels.sh
set -euo pipefail
#
# Self-learning label sync for d-oit/rust-2026-template.
#
# "Self-learning" means: mine the repo's own activity (closed issues, merged
# PRs, commit messages, file paths touched) to discover label gaps, then
# create only the labels that are genuinely missing.  No external AI API is
# used — all signal comes from the GitHub API via `gh`.
#
# ── What it learns from ───────────────────────────────────────────────────────
#
#   1. COMMIT MESSAGES (last 90 days)
#      Conventional-commit prefixes (feat, fix, docs, chore, perf, refactor,
#      test, ci, build, style, revert) → ensure a matching label exists.
#
#   2. CLOSED ISSUES & MERGED PRs (last 90 days)
#      Body text is scanned for recurring keywords (crate names, Rust concepts,
#      workflow terms).  Any keyword that appears ≥ KEYWORD_THRESHOLD times
#      and has no existing label becomes a candidate.
#
#   3. FILE PATHS TOUCHED IN RECENT PRs
#      Paths like `.github/`, `scripts/`, `crates/<name>/`, `docs/` map to
#      area labels (area: ci, area: scripts, area: <crate>, area: docs).
#      Only created when the area has seen ≥ PATH_THRESHOLD PR touches.
#
#   4. EXISTING LABEL USAGE
#      Labels that have never been applied to any issue or PR in the last
#      90 days are flagged as stale (reported only — never auto-deleted).
#
# ── What it never does ────────────────────────────────────────────────────────
#   - Delete labels (safe by design)
#   - Modify labels that already exist (--force only on new ones)
#   - Create labels whose name collides with an existing one (idempotent)
#   - Require any external secret beyond GITHUB_TOKEN
#
# ── Output ────────────────────────────────────────────────────────────────────
#   Writes a human-readable report to stdout and, when --execute is passed,
#   calls `gh label create` for each net-new label.
#   In CI the workflow captures stdout as the job summary.
#
# Usage:
#   ./scripts/learn-labels.sh [--execute] [--days N] [--repo OWNER/REPO]
#
# Defaults:
#   --days  90
#   --repo  current repo detected via `gh repo view`

set -euo pipefail

# ── Colour helpers ────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; NC='\033[0m'
info()  { echo -e "${CYAN}[learn]${NC} $*"; }
ok()    { echo -e "${GREEN}[learn]${NC} $*"; }
warn()  { echo -e "${YELLOW}[learn]${NC} $*"; }
die()   { echo -e "${RED}[learn] ERROR:${NC} $*" >&2; exit 1; }
header(){ echo -e "\n${BOLD}$*${NC}"; }

# ── Argument parsing ──────────────────────────────────────────────────────────
EXECUTE=false
DAYS=90
REPO=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --execute)       EXECUTE=true ;;
    --days)          DAYS="${2:?--days requires a value}"; shift ;;
    --repo)          REPO="${2:?--repo requires a value}"; shift ;;
    --help|-h)
      grep '^#' "$0" | sed 's/^# \?//' | head -60
      exit 0
      ;;
    *) die "Unknown argument: $1" ;;
  esac
  shift
done

# ── Dependency check ──────────────────────────────────────────────────────────
for cmd in gh jq git; do
  command -v "$cmd" &>/dev/null || die "$cmd is required but not installed."
done

# ── Resolve repo ──────────────────────────────────────────────────────────────
if [[ -z "$REPO" ]]; then
  REPO=$(gh repo view --json nameWithOwner --jq '.nameWithOwner' 2>/dev/null) \
    || die "Could not detect repo. Pass --repo OWNER/REPO explicitly."
fi
info "Repository : $REPO"
info "Lookback   : $DAYS days"
$EXECUTE || warn "DRY-RUN — pass --execute to create labels"

SINCE=$(date -u -d "-${DAYS} days" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null \
  || date -u -v-"${DAYS}"d +%Y-%m-%dT%H:%M:%SZ)   # macOS fallback

# ── Thresholds ────────────────────────────────────────────────────────────────
KEYWORD_THRESHOLD=3   # keyword must appear in ≥ N issue/PR bodies
PATH_THRESHOLD=2      # path area must appear in ≥ N PR file sets

# ── 1. Fetch existing labels ──────────────────────────────────────────────────
header "── Fetching existing labels"
EXISTING_JSON=$(gh label list --repo "$REPO" --limit 200 --json name,color,description)
EXISTING_NAMES=$(echo "$EXISTING_JSON" | jq -r '.[].name' | sort)
EXISTING_COUNT=$(echo "$EXISTING_NAMES" | grep -c . || true)
info "Found $EXISTING_COUNT existing labels"

label_exists() { echo "$EXISTING_NAMES" | grep -qxF "$1"; }

# ── 2. Collect recent closed issues + merged PRs ──────────────────────────────
header "── Fetching recent issues and PRs (since $SINCE)"

ISSUES_JSON=$(gh issue list \
  --repo "$REPO" \
  --state closed \
  --limit 200 \
  --json number,title,body,labels,closedAt \
  2>/dev/null || echo "[]")

PRS_JSON=$(gh pr list \
  --repo "$REPO" \
  --state merged \
  --limit 200 \
  --json number,title,body,labels,mergedAt,files \
  2>/dev/null || echo "[]")

ISSUE_COUNT=$(echo "$ISSUES_JSON" | jq 'length')
PR_COUNT=$(echo "$PRS_JSON"    | jq 'length')
info "Closed issues : $ISSUE_COUNT"
info "Merged PRs    : $PR_COUNT"

# ── 3. Learn from conventional-commit prefixes in recent commits ──────────────
header "── Analysing commit messages"

# Map conventional-commit type → label name + colour + description
declare -A CC_LABEL CC_COLOR CC_DESC
CC_LABEL=(
  [feat]="feature"         [fix]="bug"              [docs]="documentation"
  [chore]="chore"          [perf]="performance"     [refactor]="refactor"
  [test]="tests"           [ci]="ci: improvement"   [build]="chore"
  [style]="chore"          [revert]="revert"
)
CC_COLOR=(
  [feat]="a2eeef"  [fix]="d73a4a"   [docs]="0075ca"  [chore]="fef2c0"
  [perf]="5319e7"  [refactor]="0366d6" [test]="f4c542" [ci]="0075ca"
  [build]="fef2c0" [style]="fef2c0" [revert]="e4e669"
)
CC_DESC=(
  [feat]="New feature or enhancement"
  [fix]="Something isn't working"
  [docs]="Improvements or additions to documentation"
  [chore]="Maintenance task, tooling update, cleanup"
  [perf]="Performance-related improvement"
  [refactor]="Code improvements without behaviour change"
  [test]="Related to automated or manual tests"
  [ci]="CI pipeline improvement or speed-up"
  [build]="Build system or dependency change"
  [style]="Code style or formatting change"
  [revert]="Reverts a previous commit"
)

COMMIT_TYPES=()
if git -C . log --since="$SINCE" --format="%s" 2>/dev/null | \
    grep -oP '^(feat|fix|docs|chore|perf|refactor|test|ci|build|style|revert)(?:\([^)]+\))?' | \
    sed 's/(.*//' | sort -u > /tmp/ll_cc_types.txt 2>/dev/null; then
  while IFS= read -r cc_type; do
    [[ -n "$cc_type" ]] && COMMIT_TYPES+=("$cc_type")
  done < /tmp/ll_cc_types.txt
fi
info "Conventional-commit types found: ${COMMIT_TYPES[*]:-none}"

# ── 4. Learn from issue/PR body keywords ─────────────────────────────────────
header "── Mining issue and PR bodies for recurring keywords"

# Combine all titles + bodies into one stream for keyword frequency analysis
ALL_TEXT=$(
  echo "$ISSUES_JSON" | jq -r '.[] | "\(.title) \(.body // "")"'
  echo "$PRS_JSON"    | jq -r '.[] | "\(.title) \(.body // "")"'
)

# Keyword → candidate label mapping
# Format: "keyword|label-name|color|description"
# Keywords are matched case-insensitively as whole words.
KEYWORD_MAP=(
  "clippy|rust: clippy|f9d0c4|Clippy lint warning or fix"
  "unsafe|rust: unsafe|ee0701|Involves unsafe Rust code — needs extra scrutiny"
  "async|rust: async|6f42c1|Async / Tokio runtime concern"
  "tokio|rust: async|6f42c1|Async / Tokio runtime concern"
  "msrv|rust: msrv|c5def5|Minimum Supported Rust Version concern"
  "edition|rust: edition|bfd4f2|Rust edition migration or compatibility"
  "no.std|rust: no-std|e4e669|no_std compatibility"
  "soundness|rust: soundness|b60205|Soundness or undefined behaviour concern"
  "panic|rust: soundness|b60205|Soundness or undefined behaviour concern"
  "security|security|b60205|Security-related issue or vulnerability"
  "vulnerability|security|b60205|Security-related issue or vulnerability"
  "audit|security|b60205|Security-related issue or vulnerability"
  "performance|performance|5319e7|Performance-related improvement"
  "benchmark|performance|5319e7|Performance-related improvement"
  "flaky|ci: flaky|fbca04|Intermittently failing CI job"
  "intermittent|ci: flaky|fbca04|Intermittently failing CI job"
  "timeout|ci: flaky|fbca04|Intermittently failing CI job"
  "breaking|release: breaking|b60205|Contains a breaking API change"
  "breaking.change|release: breaking|b60205|Contains a breaking API change"
  "semver|release: breaking|b60205|Contains a breaking API change"
  "wasm|platform: wasm|c2e0c6|WebAssembly target concern"
  "windows|platform: windows|0075ca|Windows-specific issue"
  "macos|platform: macos|0075ca|macOS-specific issue"
  "linux|platform: linux|0075ca|Linux-specific issue"
  "docker|platform: docker|0075ca|Docker or container concern"
  "memory|rust: perf|5319e7|Rust-specific performance optimisation"
  "allocation|rust: perf|5319e7|Rust-specific performance optimisation"
  "deadlock|rust: async|6f42c1|Async / Tokio runtime concern"
  "race.condition|rust: soundness|b60205|Soundness or undefined behaviour concern"
  "documentation|documentation|0075ca|Improvements or additions to documentation"
  "readme|documentation|0075ca|Improvements or additions to documentation"
  "changelog|documentation|0075ca|Improvements or additions to documentation"
  "refactor|refactor|0366d6|Code improvements without behaviour change"
  "cleanup|chore|fef2c0|Maintenance task, tooling update, cleanup"
  "dependency|deps|cfd3d7|Dependency updates or changes"
  "upgrade|deps|cfd3d7|Dependency updates or changes"
  "bump|deps|cfd3d7|Dependency updates or changes"
)

# Count keyword occurrences
declare -A KW_COUNT
for entry in "${KEYWORD_MAP[@]}"; do
  kw=$(echo "$entry" | cut -d'|' -f1)
  count=$(echo "$ALL_TEXT" | grep -ciE "\b${kw}\b" 2>/dev/null || echo 0)
  KW_COUNT["$kw"]=$count
done

# ── 5. Learn from file paths touched in recent PRs ───────────────────────────
header "── Analysing file paths touched in recent PRs"

declare -A PATH_COUNT
while IFS= read -r path_entry; do
  [[ -z "$path_entry" ]] && continue
  # Map path prefixes to area labels
  case "$path_entry" in
    .github/*)   PATH_COUNT["area: ci"]=$(( ${PATH_COUNT["area: ci"]:-0} + 1 )) ;;
    scripts/*)   PATH_COUNT["area: scripts"]=$(( ${PATH_COUNT["area: scripts"]:-0} + 1 )) ;;
    docs/*)      PATH_COUNT["area: docs"]=$(( ${PATH_COUNT["area: docs"]:-0} + 1 )) ;;
    plans/*)     PATH_COUNT["area: docs"]=$(( ${PATH_COUNT["area: docs"]:-0} + 1 )) ;;
    src/*)       PATH_COUNT["area: core"]=$(( ${PATH_COUNT["area: core"]:-0} + 1 )) ;;
    crates/*)
      # Extract crate name: crates/<name>/...
      crate=$(echo "$path_entry" | cut -d'/' -f2)
      [[ -n "$crate" ]] && \
        PATH_COUNT["area: $crate"]=$(( ${PATH_COUNT["area: $crate"]:-0} + 1 ))
      ;;
  esac
done < <(echo "$PRS_JSON" | jq -r '.[] | .files[]?.path // empty' 2>/dev/null || true)

# ── 6. Check label usage (stale detection) ───────────────────────────────────
header "── Checking label usage in the last $DAYS days"

USED_LABELS=$(
  { echo "$ISSUES_JSON"; echo "$PRS_JSON"; } \
  | jq -r '.[] | .labels[]?.name' 2>/dev/null \
  | sort -u
)

STALE_LABELS=()
while IFS= read -r lname; do
  [[ -z "$lname" ]] && continue
  if ! echo "$USED_LABELS" | grep -qxF "$lname"; then
    STALE_LABELS+=("$lname")
  fi
done <<< "$EXISTING_NAMES"

# ── 7. Build the candidate list ───────────────────────────────────────────────
header "── Computing net-new label candidates"

declare -A CANDIDATES  # name → "color|description"

# From conventional commits
for cc_type in "${COMMIT_TYPES[@]}"; do
  lname="${CC_LABEL[$cc_type]:-}"
  [[ -z "$lname" ]] && continue
  label_exists "$lname" && continue
  CANDIDATES["$lname"]="${CC_COLOR[$cc_type]:-cccccc}|${CC_DESC[$cc_type]:-Conventional commit type: $cc_type}"
done

# From keyword mining
for entry in "${KEYWORD_MAP[@]}"; do
  kw=$(echo "$entry"   | cut -d'|' -f1)
  lname=$(echo "$entry" | cut -d'|' -f2)
  color=$(echo "$entry" | cut -d'|' -f3)
  desc=$(echo "$entry"  | cut -d'|' -f4)
  count=${KW_COUNT["$kw"]:-0}
  [[ "$count" -lt "$KEYWORD_THRESHOLD" ]] && continue
  label_exists "$lname" && continue
  CANDIDATES["$lname"]="${color}|${desc} (seen ${count}×)"
done

# From file path analysis
for area_label in "${!PATH_COUNT[@]}"; do
  count=${PATH_COUNT["$area_label"]}
  [[ "$count" -lt "$PATH_THRESHOLD" ]] && continue
  label_exists "$area_label" && continue
  CANDIDATES["$area_label"]="bfd4f2|Files in this area were touched in ${count} recent PRs"
done

# ── 8. Report ─────────────────────────────────────────────────────────────────
header "── Results"

CANDIDATE_COUNT=${#CANDIDATES[@]}

if [[ "$CANDIDATE_COUNT" -eq 0 ]]; then
  ok "No new labels needed — existing set covers all observed activity."
else
  echo ""
  printf "%-35s %-8s %s\n" "LABEL" "COLOR" "DESCRIPTION"
  printf "%-35s %-8s %s\n" "-----" "-----" "-----------"
  for lname in $(echo "${!CANDIDATES[@]}" | tr ' ' '\n' | sort); do
    color=$(echo "${CANDIDATES[$lname]}" | cut -d'|' -f1)
    desc=$(echo "${CANDIDATES[$lname]}"  | cut -d'|' -f2-)
    printf "%-35s #%-7s %s\n" "$lname" "$color" "$desc"
  done
fi

if [[ "${#STALE_LABELS[@]}" -gt 0 ]]; then
  echo ""
  warn "Labels unused in the last $DAYS days (not deleted — review manually):"
  for sl in "${STALE_LABELS[@]}"; do
    echo "    $sl"
  done
fi

# ── 9. Apply ──────────────────────────────────────────────────────────────────
CREATED=0
SKIPPED=0

if [[ "$CANDIDATE_COUNT" -gt 0 ]]; then
  echo ""
  if $EXECUTE; then
    header "── Creating $CANDIDATE_COUNT new label(s)"
    for lname in $(echo "${!CANDIDATES[@]}" | tr ' ' '\n' | sort); do
      color=$(echo "${CANDIDATES[$lname]}" | cut -d'|' -f1)
      desc=$(echo "${CANDIDATES[$lname]}"  | cut -d'|' -f2-)
      # Strip the "(seen N×)" annotation from the description before creating
      clean_desc=$(echo "$desc" | sed 's/ (seen [0-9]*×)//')
      if gh label create "$lname" \
          --repo "$REPO" \
          --color "$color" \
          --description "$clean_desc" \
          --force 2>/dev/null; then
        ok "Created: $lname"
        (( CREATED++ )) || true
      else
        warn "Failed:  $lname"
        (( SKIPPED++ )) || true
      fi
    done
  else
    warn "DRY-RUN: would create $CANDIDATE_COUNT label(s). Pass --execute to apply."
    SKIPPED=$CANDIDATE_COUNT
  fi
fi

# ── 10. Machine-readable summary for CI ──────────────────────────────────────
echo ""
echo "SUMMARY: existing=${EXISTING_COUNT} candidates=${CANDIDATE_COUNT} created=${CREATED} skipped=${SKIPPED} stale=${#STALE_LABELS[@]}"

# Emit GitHub Actions outputs when running in CI
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  echo "existing_count=${EXISTING_COUNT}"   >> "$GITHUB_OUTPUT"
  echo "candidate_count=${CANDIDATE_COUNT}" >> "$GITHUB_OUTPUT"
  echo "created_count=${CREATED}"           >> "$GITHUB_OUTPUT"
  echo "stale_count=${#STALE_LABELS[@]}"    >> "$GITHUB_OUTPUT"
fi
