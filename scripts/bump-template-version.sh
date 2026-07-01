#!/usr/bin/env bash
# scripts/bump-template-version.sh
set -euo pipefail
#
# Canonical source: the link footer of `.template/CHANGELOG-TEMPLATE.md`
# (e.g. `[0.3.2]: https://...compare/v0.3.1...v0.3.2`). Mirrors the structure
# of the standard Keep-a-Changelog footer.
#
# What this script updates:
#   1. `.template/CHANGELOG-TEMPLATE.md`
#      - Promotes `## [Unreleased]` → `## [NEXT] - <DATE>`
#      - Inserts a fresh `[Unreleased]` skeleton above the new entry
#      - Updates the `[Unreleased]` diff link to start from the new version
#      - Adds a new `[NEXT]` link comparing the previous version to the new one
#   2. `README.md`
#      - Rewrites the version badge from any semver to `<NEXT>-blue.svg`
#        (color-flexible; also handles `informational`, `green`, etc.)
#
# Files intentionally NOT touched:
#   - VERSION                   (per-instance init value, stays 0.0.0)
#   - Cargo.toml workspace.package.version
#                                (workspace baseline, stays 0.0.0 — derived
#                                 repos get their own versioning via
#                                 scripts/bump-version.sh after init)
#   - CHANGELOG.md              (per-instance template-skeleton, stays empty)
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
README="$ROOT/README.md"
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
  #    Using `printf` instead of literal `\n` escapes — GNU sed interprets
  #    `\n` in inserted text as newlines, but BSD sed (macOS) treats them
  #    literally, which would render the block on a single line. `printf`
  #    expands `\n` at the shell level so both sed variants see real newlines.
  UNRELEASED_BLOCK=$(printf '## [Unreleased]\n\n### Added\n\n### Changed\n\n### Deprecated\n\n### Removed\n\n### Fixed\n\n### Security\n\n---\n')
  sedi "/^## \\[${NEXT_VERSION}\\]/i\\\\
${UNRELEASED_BLOCK}" "$CHANGELOG"

  # c) Update the [Unreleased] diff link (compare starts at the new version)
  sedi "s|compare/v${CURRENT_VERSION}\\.\\.\\.HEAD|compare/v${NEXT_VERSION}...HEAD|g" "$CHANGELOG"

  # d) Add the [NEXT_VERSION] link directly below the [Unreleased] link
  NEW_LINK="[${NEXT_VERSION}]: https://github.com/d-oit/rust-2026-template/compare/v${CURRENT_VERSION}...v${NEXT_VERSION}"
  sedi "/^\\[Unreleased\\]:/a\\\\
${NEW_LINK}\\" "$CHANGELOG"

  ok "Updated $CHANGELOG ([Unreleased] promoted to [$NEXT_VERSION] - $DATE_OVERRIDE)"
else
  warn "Would update $CHANGELOG (promote [Unreleased] → [$NEXT_VERSION] - $DATE_OVERRIDE)"
fi

# ── 2. Update `README.md` version badge ──────────────────────────────────────
# Color-flexible: matches version-<SEMVER>-<ANYCOLOR>.svg and rewrites to
# version-<NEXT>-blue.svg. Works for blue, informational, green, yellow, etc.
if [[ -f "$README" ]]; then
  BADGE_PATTERN='version-[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*-[a-zA-Z][a-zA-Z]*\.svg'
  BADGE_REPLACE="version-${NEXT_VERSION}-blue.svg"

  if grep -qE "$BADGE_PATTERN" "$README"; then
    if $EXECUTE; then
      sedi "s|${BADGE_PATTERN}|${BADGE_REPLACE}|g" "$README"
      ok "Updated $README (version badge: ${CURRENT_VERSION} → ${NEXT_VERSION})"
    else
      warn "Would update $README (version badge: ${CURRENT_VERSION} → ${NEXT_VERSION})"
    fi
  else
    warn "No version badge found in $README (pattern: ${BADGE_PATTERN}) — skipping"
  fi
fi

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
if $EXECUTE; then
  ok "Template version bumped: $CURRENT_VERSION → $NEXT_VERSION"
  info "Next steps:"
  echo "  1. Review changes: git diff .template/CHANGELOG-TEMPLATE.md README.md"
  echo "  2. Validate:       bash scripts/quality-gates.sh"
  echo "  3. Commit:         git add .template/CHANGELOG-TEMPLATE.md README.md && \\"
  echo "                       git commit -m 'chore(template): bump version to $NEXT_VERSION'"
  echo "  4. Tag:            git tag v${NEXT_VERSION} && git push origin main --tags"
else
  info "Dry-run complete. Re-run with --execute to apply."
fi
