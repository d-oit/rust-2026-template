#!/usr/bin/env bash
# scripts/bump-template-version.sh
set -euo pipefail
#
# Canonical source: the link footer of `.template/CHANGELOG-TEMPLATE.md`
# (e.g. `[0.3.2]: https://...compare/v0.3.1...v0.3.2`). Mirrors the structure
# of the standard Keep-a-Changelog footer.
#
# Design principle: `.template/CHANGELOG-TEMPLATE.md` is the SOLE source of
# truth for the template's own version. All other files that could otherwise
# carry a version are intentionally NOT touched, so a `bump-template-version`
# run mutates exactly one file and produces a clean atomic diff.
#
# What this script updates:
#   - `.template/CHANGELOG-TEMPLATE.md`:
#       - Promotes `## [Unreleased]` → `## [NEXT] - <DATE>`
#       - Inserts a fresh `[Unreleased]` skeleton above the new entry
#       - Updates the `[Unreleased]` diff link to start from the new version
#       - Adds a new `[NEXT]` link comparing the previous version to the new one
#
# Files intentionally NOT touched (this is the whole point):
#   - VERSION                   (per-template init value; intentionally stays 0.0.0)
#   - CHANGELOG.md              (per-template generated-project changelog; intentionally
#                                stays in its initial skeleton state — derived repos
#                                own this file via scripts/bump-version.sh)
#   - Cargo.toml workspace.package.version
#                                (workspace baseline; intentionally stays 0.0.0 —
#                                 derived repos inherit and bump separately)
#   - README.md                 (no version badge — the changelog-template IS the
#                                single source of truth for the template's version)
#   - rust-toolchain.toml       (toolchain channel, not crate version)
#   - deny.toml                 (supply-chain policy)
#
# Usage:
#   ./scripts/bump-template-version.sh            # dry-run; bumps PATCH
#   ./scripts/bump-template-version.sh --execute  # apply PATCH bump
#   ./scripts/bump-template-version.sh --execute --minor
#   ./scripts/bump-template-version.sh --execute --version=1.2.3
#   ./scripts/bump-template-version.sh --execute --version=2.0.0 --date=2026-07-15
#
# Exit codes:
#   0  success (dry-run or --execute completed)
#   1  error (missing files, parse failure, etc.)

# ── Colour helpers ────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
info()  { echo -e "${CYAN}[bump-template]${NC} $*"; }
ok()    { echo -e "${GREEN}[bump-template]${NC} $*"; }
warn()  { echo -e "${YELLOW}[bump-template]${NC} $*"; }
die()   { echo -e "${RED}[bump-template] ERROR:${NC} $*" >&2; exit 1; }

# ── Argument parsing ──────────────────────────────────────────────────────────
EXECUTE=false
BUMP_TYPE="patch"
FORCED_VERSION=""
DATE_OVERRIDE="$(date -u +%Y-%m-%d)"

print_help() {
  sed -n '2,/^# Usage/p' "$0" | grep '^#' | sed 's/^# \?//'
  exit 0
}

for arg in "$@"; do
  case "$arg" in
    --execute)               EXECUTE=true ;;
    --major)                 BUMP_TYPE="major" ;;
    --minor)                 BUMP_TYPE="minor" ;;
    --patch)                 BUMP_TYPE="patch" ;;
    --version=*)             FORCED_VERSION="${arg#*=}" ;;
    --date=*)                DATE_OVERRIDE="${arg#*=}" ;;
    --help|-h)               print_help ;;
    *)                       die "Unknown argument: $arg. Use --help." ;;
  esac
done

# ── Locate repo root ──────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

CHANGELOG="$ROOT/.template/CHANGELOG-TEMPLATE.md"
[[ -f "$CHANGELOG" ]] || die "Changelog not found at $CHANGELOG"

# ── Helper: sed in-place (portable GNU + BSD/macOS) ───────────────────────────
sedi() {
  # $1 = sed expression, $2 = file
  if sed --version 2>/dev/null | grep -q GNU; then
    sed -i "$1" "$2"
  else
    sed -i '' "$1" "$2"
  fi
}

# ── Read current template version from the changelog link footer ────────────
# Header is `[Unreleased]:`. The remaining lines are `[X.Y.Z]:` entries. Filter
# the versioned entries, then pick the semver-maximum (robust to footer being
# in either descending or ascending order).
CURRENT_VERSION=$(grep -oE '^\[[0-9]+\.[0-9]+\.[0-9]+\]: ' "$CHANGELOG" \
  | sed -E 's/^\[([^]]+)\].*/\1/' \
  | sort -V \
  | tail -n 1)
[[ -n "$CURRENT_VERSION" ]] || die "Could not parse current version from $CHANGELOG footer"

# ── Compute the next version ─────────────────────────────────────────────────
if [[ -n "$FORCED_VERSION" ]]; then
  if ! [[ "$FORCED_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    die "--version must match semver (X.Y.Z); got '$FORCED_VERSION'"
  fi
  NEXT_VERSION="$FORCED_VERSION"
else
  IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"
  [[ "$MAJOR" =~ ^[0-9]+$ && "$MINOR" =~ ^[0-9]+$ && "$PATCH" =~ ^[0-9]+$ ]] \
    || die "Current version '$CURRENT_VERSION' is not valid semver"
  case "$BUMP_TYPE" in
    major) NEXT_VERSION="$((MAJOR + 1)).0.0" ;;
    minor) NEXT_VERSION="${MAJOR}.$((MINOR + 1)).0" ;;
    patch) NEXT_VERSION="${MAJOR}.${MINOR}.$((PATCH + 1))" ;;
    *)     die "Invalid bump type: $BUMP_TYPE" ;;
  esac
fi

info "Template version: $CURRENT_VERSION → $NEXT_VERSION (${DATE_OVERRIDE})"
$EXECUTE || warn "DRY-RUN mode — pass --execute to apply changes"
echo ""

# ── Pre-flight checks ────────────────────────────────────────────────────────
grep -q "^## \[Unreleased\]" "$CHANGELOG" \
  || die "No '## [Unreleased]' section in $CHANGELOG — cannot promote"

if grep -qF "[${NEXT_VERSION}]:" "$CHANGELOG"; then
  die "Link footer already contains [${NEXT_VERSION}]: — refusing to duplicate"
fi

# ── 1. Update `.template/CHANGELOG-TEMPLATE.md` ─────────────────────────────
if $EXECUTE; then
  # a) Rename ## [Unreleased] → ## [NEXT] - DATE
  sedi "s/^## \\[Unreleased\\]/## [${NEXT_VERSION}] - ${DATE_OVERRIDE}/" "$CHANGELOG"

  # b) Insert a fresh [Unreleased] skeleton above the new versioned entry.
  #    Using `awk` instead of `sed`'s `i\` insert because the block contains
  #    lines starting with `-` (the `---` separator) that sed misinterprets as
  #    commands.  `awk` inserts the block once before `## [NEXT_VERSION]` and
  #    is portable across GNU and BSD.
  UNRELEASED_BLOCK=$(printf '## [Unreleased]\n\n### Added\n\n- for new features.\n\n### Changed\n\n- for changes in existing functionality.\n\n### Deprecated\n\n- for soon-to-be removed features.\n\n### Removed\n\n- for now removed features.\n\n### Fixed\n\n- for any bug fixes.\n\n### Security\n\n- in case of vulnerabilities.\n\n---\n\n')
  awk -v block="$UNRELEASED_BLOCK" -v ver="## [${NEXT_VERSION}]" '
    index($0, ver) == 1 && !done { printf "%s", block; done=1 }
    { print }
  ' "$CHANGELOG" > "${CHANGELOG}.tmp" && mv "${CHANGELOG}.tmp" "$CHANGELOG"

  # c) Update the [Unreleased] diff link (compare starts at the new version)
  sedi "s|compare/v${CURRENT_VERSION}\\.\\.\\.HEAD|compare/v${NEXT_VERSION}...HEAD|g" "$CHANGELOG"

  # d) Add the [NEXT_VERSION] link directly below the [Unreleased] link.
  #    Using `awk` instead of `sed`'s `a\` append because the `[` character
  #    in the link line is misinterpreted by sed as a command.
  NEW_LINK="[${NEXT_VERSION}]: https://github.com/d-oit/rust-2026-template/compare/v${CURRENT_VERSION}...v${NEXT_VERSION}"
  awk -v new_link="$NEW_LINK" '
    { print }
    /^\[Unreleased\]:/ && !done { print new_link; done=1 }
  ' "$CHANGELOG" > "${CHANGELOG}.tmp" && mv "${CHANGELOG}.tmp" "$CHANGELOG"

  ok "Updated $CHANGELOG ([Unreleased] promoted to [$NEXT_VERSION] - $DATE_OVERRIDE)"
else
  warn "Would update $CHANGELOG (promote [Unreleased] → [$NEXT_VERSION] - $DATE_OVERRIDE)"
fi

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
if $EXECUTE; then
  ok "Template version bumped: $CURRENT_VERSION → $NEXT_VERSION"
  info "Next steps:"
  echo "  1. Review changes: git diff .template/CHANGELOG-TEMPLATE.md"
  echo "  2. Validate:       bash scripts/quality-gates.sh"
  echo "  3. Commit:         git add .template/CHANGELOG-TEMPLATE.md && \\"
  echo "                       git commit -m 'chore(template): bump version to $NEXT_VERSION'"
  echo "  4. Tag:            git tag v${NEXT_VERSION} && git push origin main --tags"
else
  info "Dry-run complete. Re-run with --execute to apply."
fi
