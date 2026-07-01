#!/usr/bin/env bash
# init-template.sh - Initialize a new project from the rust-2026-template.
# Prompts for project details and rewrites all placeholder references.
# Run after: git clone https://github.com/d-oit/rust-2026-template.git YOUR_REPO
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# --- colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log()   { printf "${CYAN}==> %s${NC}\n" "$*"; }
ok()    { printf "${GREEN}  ✓ %s${NC}\n" "$*"; }
warn()  { printf "${YELLOW}  ! %s${NC}\n" "$*"; }
fail()  { printf "${RED}  ✗ %s${NC}\n" "$*" >&2; exit 1; }

DRY_RUN=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Initialize a new project from the rust-2026-template.

Options:
  --dry-run             Preview changes without applying them
  --name NAME           Project/crate name (e.g., "my-app")
  --description DESC    Project description
  --author AUTHOR       Author name (e.g., "Jane Doe")
  --repo REPO           GitHub repo (e.g., "myorg/my-app")
  -h, --help            Show this help message

If options are not provided, the script will prompt interactively.
EOF
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)   DRY_RUN=1; shift ;;
    --name)      PROJECT_NAME="$2"; shift 2 ;;
    --description) PROJECT_DESC="$2"; shift 2 ;;
    --author)    AUTHOR="$2"; shift 2 ;;
    --repo)      REPO="$2"; shift 2 ;;
    -h|--help)   usage ;;
    *)           fail "Unknown option: $1" ;;
  esac
done

# --- interactive prompts ---
if [[ -z "${PROJECT_NAME:-}" ]]; then
  printf "${CYAN}Project name (e.g., my-app):${NC} "
  read -r PROJECT_NAME
fi
[[ -z "$PROJECT_NAME" ]] && fail "Project name is required"

if [[ -z "${PROJECT_DESC:-}" ]]; then
  printf "${CYAN}Description:${NC} "
  read -r PROJECT_DESC
fi
[[ -z "$PROJECT_DESC" ]] && fail "Description is required"

if [[ -z "${AUTHOR:-}" ]]; then
  printf "${CYAN}Author name:${NC} "
  read -r AUTHOR
fi
[[ -z "$AUTHOR" ]] && fail "Author is required"

if [[ -z "${REPO:-}" ]]; then
  printf "${CYAN}GitHub repo (org/name):${NC} "
  read -r REPO
fi
[[ -z "$REPO" ]] && fail "GitHub repo is required"

REPO_URL="https://github.com/${REPO}"

log "Initializing project: $PROJECT_NAME"
log "  Description: $PROJECT_DESC"
log "  Author: $AUTHOR"
log "  Repo: $REPO_URL"
echo ""

# --- rename example-crate ---
CrateName=$(echo "$PROJECT_NAME" | tr '-' '_')
log "Renaming crates/example-crate -> crates/$PROJECT_NAME"

if [[ $DRY_RUN -eq 1 ]]; then
  ok "(dry run) Would rename crates/example-crate -> crates/$PROJECT_NAME"
else
  if [[ -d "crates/$PROJECT_NAME" ]]; then
    warn "crates/$PROJECT_NAME already exists, skipping rename"
  else
    mv "crates/example-crate" "crates/$PROJECT_NAME"
    ok "Renamed crates/example-crate -> crates/$PROJECT_NAME"
  fi
fi

# --- sed helper (GNU/BSD compatible) ---
sedi() {
  if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' "$@"
  else
    sed -i "$@"
  fi
}

# --- replace in file ---
replace_in_file() {
  local file="$1"
  local pattern="$2"
  local replacement="$3"
  if [[ $DRY_RUN -eq 1 ]]; then
    ok "(dry run) Would update $file"
  elif [[ -f "$file" ]]; then
    sedi "s|$pattern|$replacement|g" "$file"
    ok "Updated $file"
  fi
}

# --- rewrite Cargo.toml (workspace) ---
log "Updating Cargo.toml"
replace_in_file "Cargo.toml" 'name = "rust-2026-template"' "name = \"$PROJECT_NAME\""
replace_in_file "Cargo.toml" 'description = "A production-ready Rust workspace template.*"' "description = \"$PROJECT_DESC\""
replace_in_file "Cargo.toml" 'authors = \["Your Name"\]' "authors = [\"$AUTHOR\"]"
replace_in_file "Cargo.toml" 'repository = "https://github.com/your-org/your-repo"' "repository = \"$REPO_URL\""
replace_in_file "Cargo.toml" 'homepage = "https://github.com/your-org/your-repo"' "homepage = \"$REPO_URL\""
replace_in_file "Cargo.toml" 'documentation = "https://docs.rs/your-crate"' "documentation = \"https://docs.rs/$PROJECT_NAME\""

# --- rewrite crate Cargo.toml ---
CRATE_TOML="crates/$PROJECT_NAME/Cargo.toml"
log "Updating $CRATE_TOML"
replace_in_file "$CRATE_TOML" 'name = "example-crate"' "name = \"$PROJECT_NAME\""
replace_in_file "$CRATE_TOML" 'description = "Example crate in the rust-2026-template workspace"' "description = \"$PROJECT_DESC\""

# --- rewrite sample-app Cargo.toml ---
log "Updating crates/sample-app/Cargo.toml"
replace_in_file "crates/sample-app/Cargo.toml" 'description = "Sample application demonstrating the rust-2026-template"' "description = \"Sample application for $PROJECT_NAME\""

# --- rewrite template crate Cargo.toml files ---
for crate in actor-runtime-template checkpoint-template hybrid-storage-template mcp-server-template; do
  TOML="crates/$crate/Cargo.toml"
  if [[ -f "$TOML" ]]; then
    log "Updating $TOML"
    replace_in_file "$TOML" "Actor runtime template using ractor for message-passing concurrency" "$PROJECT_DESC"
    replace_in_file "$TOML" "Checkpoint template for serializable application state with migration support" "$PROJECT_DESC"
    replace_in_file "$TOML" "Hybrid storage template with SQL + KV backends and caching" "$PROJECT_DESC"
    replace_in_file "$TOML" "MCP server template crate with tool registration and dispatch" "$PROJECT_DESC"
  fi
done

# --- rewrite AGENTS.md ---
log "Updating AGENTS.md"
replace_in_file "AGENTS.md" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite CLAUDE.md ---
log "Updating CLAUDE.md"
replace_in_file "CLAUDE.md" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite GEMINI.md ---
log "Updating GEMINI.md"
replace_in_file "GEMINI.md" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite QWEN.md ---
log "Updating QWEN.md"
replace_in_file "QWEN.md" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite README.md ---
log "Updating README.md"
# Specific badge-URL rewrites must run BEFORE the generic `rust-2026-template`
# substitution below, otherwise the generic replace would mangle the badge URLs.
# The version-badge search pattern is version-flexible (matches any semver) so
# the script stays correct when the template's own version is bumped.
replace_in_file "README.md" '# Rust 2026 Template' "# $PROJECT_NAME"
replace_in_file "README.md" 'version-[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*-blue\.svg' "version-$(cat VERSION)-blue.svg"
replace_in_file "README.md" 'https://github.com/d-oit/rust-2026-template' "$REPO_URL"
replace_in_file "README.md" 'https://codecov.io/gh/d-oit/rust-2026-template' "https://codecov.io/gh/${REPO}"
replace_in_file "README.md" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite CONTRIBUTING.md ---
log "Updating CONTRIBUTING.md"
replace_in_file "CONTRIBUTING.md" '# Contributing to rust-2026-template' "# Contributing to $PROJECT_NAME"
replace_in_file "CONTRIBUTING.md" 'https://github.com/d-oit/rust-2026-template' "$REPO_URL"

# --- rewrite SECURITY.md ---
log "Updating SECURITY.md"
replace_in_file "SECURITY.md" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite QUICKSTART.md ---
log "Updating QUICKSTART.md"
replace_in_file "QUICKSTART.md" '# Quick Start — rust-2026-template' "# Quick Start — $PROJECT_NAME"
replace_in_file "QUICKSTART.md" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite MIGRATION.md ---
log "Updating MIGRATION.md"
replace_in_file "MIGRATION.md" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite flake.nix ---
log "Updating flake.nix"
replace_in_file "flake.nix" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite llms.txt header ---
log "Updating llms.txt"
replace_in_file "llms.txt" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite example crate README ---
EXAMPLE_README="crates/$PROJECT_NAME/README.md"
if [[ -f "$EXAMPLE_README" ]]; then
  log "Updating $EXAMPLE_README"
  replace_in_file "$EXAMPLE_README" '# example-crate' "# $PROJECT_NAME"
  replace_in_file "$EXAMPLE_README" 'example-crate' "$PROJECT_NAME"
fi

# --- rewrite example crate lib.rs ---
EXAMPLE_LIB="crates/$PROJECT_NAME/src/lib.rs"
if [[ -f "$EXAMPLE_LIB" ]]; then
  log "Updating $EXAMPLE_LIB"
  replace_in_file "$EXAMPLE_LIB" '# example-crate' "# $PROJECT_NAME"
  replace_in_file "$EXAMPLE_LIB" 'example-crate' "$PROJECT_NAME"
  replace_in_file "$EXAMPLE_LIB" 'example_crate' "$CrateName"
fi

# --- rewrite examples ---
EXAMPLES_DIR="examples/hello_world"
if [[ -f "$EXAMPLES_DIR/src/main.rs" ]]; then
  log "Updating examples/hello_world/src/main.rs"
  replace_in_file "$EXAMPLES_DIR/src/main.rs" 'example_crate' "$CrateName"
fi

# --- validate ---
log "Validating build"
if [[ $DRY_RUN -eq 1 ]]; then
  echo ""
  ok "Dry run complete. No changes were applied."
  echo ""
  log "Next steps:"
  echo "  1. Run: $(basename "$0")"
  echo "  2. Verify: cargo build --workspace"
  echo "  3. Commit: git add -A && git commit -m 'feat: initialize from rust-2026-template'"
  exit 0
fi

echo ""
log "Verifying the build compiles..."
if command -v cargo >/dev/null 2>&1; then
  if cargo check --workspace 2>/dev/null; then
    ok "Build check passed"
  else
    warn "Build check had issues - review output above"
  fi
else
  warn "cargo not found - skipping build verification"
fi

echo ""
ok "Template initialized successfully!"
echo ""
log "Next steps:"
echo "  1. Review changes: git diff"
echo "  2. Build: cargo build --workspace"
echo "  3. Test: cargo nextest run --workspace"
echo "  4. Quality gate: ./scripts/quality-gates.sh"
echo "  5. Commit: git add -A && git commit -m 'feat: initialize from rust-2026-template'"
echo "  6. Push: git push origin main"
