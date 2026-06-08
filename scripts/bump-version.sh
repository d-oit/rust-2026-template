#!/usr/bin/env bash
# scripts/bump-version.sh
set -euo pipefail
#
# Canonical version source: [workspace.package] version in ./Cargo.toml
#
# What this script updates:
#   1. VERSION             — plain-text version "X.Y.Z" (template starter: 0.0.0)
#   2. Cargo.toml          — [workspace.package] version = "X.Y.Z"
#   3. Cargo.lock          — regenerated via `cargo update --workspace`
#   4. CHANGELOG.md        — promotes [Unreleased] to [X.Y.Z] and inserts a
#                            fresh [Unreleased] header; updates diff links
#   4. README.md           — badge URL containing the old version string
#   5. Any *.md / *.toml / *.yml / *.yaml / *.json file that contains an
#      explicit "version = \"OLD\"" or "version: OLD" line that matches the
#      workspace version exactly (skips dependency version lines)
#
# Files intentionally NOT touched:
#   - Dependency version specs in [dependencies] / [workspace.dependencies]
#   - rust-toolchain.toml  (toolchain channel, not crate version)
#   - deny.toml            (supply-chain policy, not crate version)
#   - target/              (build artefacts)
#   - .git/                (git internals)
#
# Usage:
#   ./scripts/bump-version.sh            # dry-run: prints what would change
#   ./scripts/bump-version.sh --execute  # applies all changes
#
# Exit codes:
#   0  success (or dry-run completed)
#   1  error (missing tools, parse failure, etc.)

set -euo pipefail

# ── Colour helpers ────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
info()  { echo -e "${CYAN}[bump]${NC} $*"; }
ok()    { echo -e "${GREEN}[bump]${NC} $*"; }
warn()  { echo -e "${YELLOW}[bump]${NC} $*"; }
die()   { echo -e "${RED}[bump] ERROR:${NC} $*" >&2; exit 1; }

# ── Argument parsing ──────────────────────────────────────────────────────────
EXECUTE=false
for arg in "$@"; do
  case "$arg" in
    --execute) EXECUTE=true ;;
    --help|-h)
      sed -n '2,/^# Usage/p' "$0" | grep '^#' | sed 's/^# \?//'
      exit 0
      ;;
    *) die "Unknown argument: $arg. Use --execute or --help." ;;
  esac
done

# ── Locate repo root ──────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

CARGO_TOML="$ROOT/Cargo.toml"
[[ -f "$CARGO_TOML" ]] || die "Cargo.toml not found at $CARGO_TOML"

# ── Read current version from [workspace.package] ────────────────────────────
# Matches the first bare `version = "X.Y.Z"` line inside [workspace.package].
# Uses awk so it is scoped to the correct TOML section and ignores dependency
# version specs that appear later in the file.
CURRENT_VERSION=$(awk '
  /^\[workspace\.package\]/ { in_section=1; next }
  /^\[/                     { in_section=0 }
  in_section && /^version[[:space:]]*=/ {
    match($0, /"([0-9]+\.[0-9]+\.[0-9]+)"/, arr)
    if (arr[1] != "") { print arr[1]; exit }
  }
' "$CARGO_TOML")

[[ -n "$CURRENT_VERSION" ]] || die "Could not parse version from $CARGO_TOML"

# ── Compute next patch version ────────────────────────────────────────────────
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"
[[ "$MAJOR" =~ ^[0-9]+$ && "$MINOR" =~ ^[0-9]+$ && "$PATCH" =~ ^[0-9]+$ ]] \
  || die "Version '$CURRENT_VERSION' is not valid semver"
NEXT_VERSION="$MAJOR.$MINOR.$((PATCH + 1))"

info "Current version : $CURRENT_VERSION"
info "Next version    : $NEXT_VERSION"
$EXECUTE || warn "DRY-RUN mode — pass --execute to apply changes"
echo ""

# ── Helper: sed in-place (portable across GNU and BSD/macOS) ─────────────────
sedi() {
  # $1 = sed expression, $2 = file
  if sed --version 2>/dev/null | grep -q GNU; then
    sed -i "$1" "$2"
  else
    sed -i '' "$1" "$2"
  fi
}

apply() {
  # apply <description> <file> <sed-expression>
  local desc="$1" file="$2" expr="$3"
  if grep -qE "${expr//\//\\/}" "$file" 2>/dev/null; then
    if $EXECUTE; then
      sedi "$expr" "$file"
      ok "Updated  $file  ($desc)"
    else
      warn "Would update  $file  ($desc)"
    fi
  fi
}

# ── 1. VERSION file ──────────────────────────────────────────────────────────
# Plain-text version file — keeps the template starter at 0.0.0 semantics.
VERSION_FILE="$ROOT/VERSION"
if [[ -f "$VERSION_FILE" ]]; then
  if $EXECUTE; then
    echo "$NEXT_VERSION" > "$VERSION_FILE"
    ok "Updated  $VERSION_FILE  ($CURRENT_VERSION → $NEXT_VERSION)"
  else
    warn "Would update  $VERSION_FILE  ($CURRENT_VERSION → $NEXT_VERSION)"
  fi
fi

# ── 2. Cargo.toml — workspace.package version ────────────────────────────────
# Only replaces the version line that is inside [workspace.package]; the awk
# approach above confirmed it exists. We use a targeted sed that matches the
# exact quoted version string on a line starting with `version`.
apply \
  "[workspace.package] version" \
  "$CARGO_TOML" \
  "s/^\\(version[[:space:]]*=[[:space:]]*\\)\"${CURRENT_VERSION}\"/\\1\"${NEXT_VERSION}\"/"

# ── 3. Any crate Cargo.toml with a standalone (non-workspace) version line ───
# Workspace members use `version.workspace = true`, so this only fires for
# crates that pin their own version explicitly.
while IFS= read -r -d '' toml; do
  [[ "$toml" == "$CARGO_TOML" ]] && continue  # already handled above
  apply \
    "standalone package version" \
    "$toml" \
    "s/^\\(version[[:space:]]*=[[:space:]]*\\)\"${CURRENT_VERSION}\"/\\1\"${NEXT_VERSION}\"/"
done < <(find "$ROOT/crates" -name "Cargo.toml" -print0 2>/dev/null)

# ── 4. CHANGELOG.md — promote [Unreleased] and insert fresh header ───────────
CHANGELOG="$ROOT/CHANGELOG.md"
TODAY=$(date -u +%Y-%m-%d)

if [[ -f "$CHANGELOG" ]]; then
  if $EXECUTE; then
    # a) Rename ## [Unreleased] → ## [NEXT_VERSION] - DATE
    sedi "s/^## \\[Unreleased\\]/## [${NEXT_VERSION}] - ${TODAY}/" "$CHANGELOG"

    # b) Insert a blank [Unreleased] section above the new versioned entry
    UNRELEASED_BLOCK="## [Unreleased]\n\n### Added\n\n### Changed\n\n### Fixed\n\n---\n"
    sedi "/^## \\[${NEXT_VERSION}\\]/i\\
${UNRELEASED_BLOCK}" "$CHANGELOG"

    # c) Update the [Unreleased] diff link
    sedi "s|compare/v${CURRENT_VERSION}\.\.\.HEAD|compare/v${NEXT_VERSION}...HEAD|g" "$CHANGELOG"

    # d) Add the new version diff link (after the existing [Unreleased] link line)
    NEW_LINK="[${NEXT_VERSION}]: https://github.com/d-oit/rust-2026-template/releases/tag/v${NEXT_VERSION}"
    # Insert after the [Unreleased] link line if not already present
    if ! grep -qF "[$NEXT_VERSION]:" "$CHANGELOG"; then
      sedi "/^\[Unreleased\]:/a\\
${NEW_LINK}" "$CHANGELOG"
    fi

    ok "Updated  $CHANGELOG  (promoted [Unreleased] → [$NEXT_VERSION])"
  else
    warn "Would update  $CHANGELOG  (promote [Unreleased] → [$NEXT_VERSION] - $TODAY)"
  fi
fi

# ── 5. README.md — version badge and any explicit version strings ─────────────
README="$ROOT/README.md"
if [[ -f "$README" ]]; then
  # Badge: rust-1.87%2B style URLs are toolchain badges, not crate version —
  # skip those. Only replace bare version strings like "0.1.0".
  apply \
    "version string" \
    "$README" \
    "s/${CURRENT_VERSION}/${NEXT_VERSION}/g"
fi

# ── 6. Broad scan: *.md, *.yml, *.yaml, *.json outside target/ and .git/ ─────
# Matches lines of the form:
#   version: "0.1.0"   (YAML)
#   version = "0.1.0"  (TOML / shell)
#   "version": "0.1.0" (JSON)
# Does NOT match lines like:
#   tokio = { version = "1", ... }   (dependency specs use short versions)
#   rust-version = "1.87"            (toolchain MSRV)
VERSION_LINE_PATTERN="^[[:space:]]*(\"version\"|version)[[:space:]]*[:=][[:space:]]*\"${CURRENT_VERSION}\""

while IFS= read -r -d '' file; do
  # Skip files already handled above
  [[ "$file" == "$CARGO_TOML" ]] && continue
  [[ "$file" == "$CHANGELOG" ]] && continue
  [[ "$file" == "$README" ]] && continue
  # Skip workflow files that reference the version only in comments
  if grep -qE "$VERSION_LINE_PATTERN" "$file" 2>/dev/null; then
    apply \
      "version line" \
      "$file" \
      "s/\\(^[[:space:]]*\\(\"version\"\\|version\\)[[:space:]]*[:=][[:space:]]*\\)\"${CURRENT_VERSION}\"/\\1\"${NEXT_VERSION}\"/"
  fi
done < <(find "$ROOT" \
  \( -path "$ROOT/target" -o -path "$ROOT/.git" \) -prune \
  -o \( -name "*.md" -o -name "*.yml" -o -name "*.yaml" -o -name "*.json" \) \
  -print0)

# ── 7. Regenerate Cargo.lock ──────────────────────────────────────────────────
if $EXECUTE; then
  info "Regenerating Cargo.lock..."
  cargo update --workspace --quiet
  ok "Cargo.lock updated"
else
  warn "Would run: cargo update --workspace  (to refresh Cargo.lock)"
fi

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
if $EXECUTE; then
  ok "Version bumped: $CURRENT_VERSION → $NEXT_VERSION"
else
  info "Dry-run complete. Run with --execute to apply."
fi
