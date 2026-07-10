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
MINIMAL=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Initialize a new project from the rust-2026-template.

Options:
  --dry-run             Preview changes without applying them
  --minimal             Slim workspace for typical apps: keep sample-app + your
                        renamed lib crate + xtask; remove optional pattern crates
                        and optional workflows (DORA, mutants, eval, docs deploy…)
  --name NAME           Project/crate name (e.g., "my-app")
  --description DESC    Project description
  --author AUTHOR       Author name (e.g., "Jane Doe")
  --repo REPO           GitHub repo (e.g., "myorg/my-app")
  -h, --help            Show this help message

If options are not provided, the script will prompt interactively.

Recommended for most new codebases:
  $(basename "$0") --minimal --name my-app --description "..." --author "..." --repo org/my-app
EOF
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)   DRY_RUN=1; shift ;;
    --minimal)   MINIMAL=1; shift ;;
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
if [[ $MINIMAL -eq 1 ]]; then
  log "  Profile: minimal (typical app workspace)"
else
  log "  Profile: full (keeps pattern crates and optional workflows)"
fi
echo ""

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

# --- remove path (dry-run aware) ---
remove_path() {
  local path="$1"
  if [[ $DRY_RUN -eq 1 ]]; then
    ok "(dry run) Would remove $path"
  elif [[ -e "$path" ]]; then
    rm -rf "$path"
    ok "Removed $path"
  fi
}

# --- minimal profile: drop optional pattern crates + optional workflows ---
# Keeps: crates/sample-app, crates/example-crate (renamed), crates/xtask,
# core CI (ci, commitlint, release*, security/secretlint, hotfix, dependabot).
if [[ $MINIMAL -eq 1 ]]; then
  log "Applying --minimal profile"

  for crate in \
    actor-runtime-template \
    checkpoint-template \
    hybrid-storage-template \
    mcp-server-template \
    example-registry-pattern \
    example-storage-pattern
  do
    remove_path "crates/$crate"
  done

  for wf in \
    dora-fdrt.yml \
    dora-report.yml \
    eval.yml \
    mutants.yml \
    skills-evaluation.yml \
    update-architecture-diagram.yml \
    cleanup-ci-status.yml \
    sync-labels.yml \
    labeler.yml \
    patch-release-on-label.yml \
    deploy-docs.yml \
    fuzz.yml
  do
    remove_path ".github/workflows/$wf"
  done

  # Optional meta-template surface (safe for adopters to drop)
  remove_path "fuzz"
  remove_path "benchmarks"
  remove_path "docs/patterns"
  remove_path ".template"

  # Workspace members use crates/* examples/* benchmarks — drop benchmarks member if dir gone
  # Cargo.toml members = ["crates/*", "examples/*", "benchmarks"] — fix when benchmarks removed
  if [[ $DRY_RUN -eq 0 ]] && [[ ! -d benchmarks ]]; then
    if grep -q '"benchmarks"' Cargo.toml 2>/dev/null; then
      sedi 's/, "benchmarks"//' Cargo.toml
      sedi 's/"benchmarks", //' Cargo.toml
      ok "Removed benchmarks from workspace members"
    fi
  fi
fi

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

# --- rewrite template crate Cargo.toml files (skipped if --minimal removed them) ---
for crate in actor-runtime-template checkpoint-template hybrid-storage-template mcp-server-template; do
  TOML="crates/$crate/Cargo.toml"
  if [[ -f "$TOML" ]]; then
    log "Updating $TOML"
    replace_in_file "$TOML" "Hand-rolled actor runtime template: mailbox, state, and supervision patterns" "$PROJECT_DESC"
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
# NOTE: no version-badge rewrite here by design — the template's README no longer
# carries a version badge; the template's version is tracked exclusively in
# `.template/CHANGELOG-TEMPLATE.md` (see scripts/bump-template-version.sh).
replace_in_file "README.md" '# Rust 2026 Template' "# $PROJECT_NAME"
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

# --- rewrite release.toml ---
log "Updating release.toml"
replace_in_file "release.toml" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite scripts/bump-version.sh ---
log "Updating scripts/bump-version.sh"
replace_in_file "scripts/bump-version.sh" 'your-org/your-repo' "$REPO"

# --- rewrite CHANGELOG.md link footer ---
log "Updating CHANGELOG.md"
replace_in_file "CHANGELOG.md" 'your-org/your-repo' "$REPO"

# --- rewrite llms.txt header ---
log "Updating llms.txt"
replace_in_file "llms.txt" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite roast-scorer.sh ---
log "Updating scripts/roast-scorer.sh"
replace_in_file "scripts/roast-scorer.sh" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite HARNESS.md ---
log "Updating HARNESS.md"
replace_in_file "HARNESS.md" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite scripts/bootstrap.sh ---
log "Updating scripts/bootstrap.sh"
replace_in_file "scripts/bootstrap.sh" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite scripts/generate-llms-txt.sh ---
log "Updating scripts/generate-llms-txt.sh"
replace_in_file "scripts/generate-llms-txt.sh" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite .clippy.toml ---
log "Updating .clippy.toml"
replace_in_file ".clippy.toml" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite docs/architecture/context.yaml ---
log "Updating docs/architecture/context.yaml"
replace_in_file "docs/architecture/context.yaml" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite agents-docs/structure.md ---
log "Updating agents-docs/structure.md"
replace_in_file "agents-docs/structure.md" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite crates/sample-app/README.md ---
log "Updating crates/sample-app/README.md"
replace_in_file "crates/sample-app/README.md" 'rust-2026-template' "$PROJECT_NAME"

# --- rewrite fuzz/Cargo.toml (may be removed under --minimal) ---
if [[ -f "fuzz/Cargo.toml" ]]; then
  log "Updating fuzz/Cargo.toml"
  replace_in_file "fuzz/Cargo.toml" 'rust-2026-template' "$PROJECT_NAME"
fi

# --- rewrite benchmarks/Cargo.toml (may be removed under --minimal) ---
if [[ -f "benchmarks/Cargo.toml" ]]; then
  log "Updating benchmarks/Cargo.toml"
  replace_in_file "benchmarks/Cargo.toml" 'rust-2026-template' "$PROJECT_NAME"
fi

# --- rewrite CI workflows with repo-specific checks (files may be gone under --minimal) ---
log "Updating CI workflow repo checks"
replace_in_file ".github/workflows/dora-fdrt.yml" 'd-oit/rust-2026-template' "$REPO"
replace_in_file ".github/workflows/dora-report.yml" 'd-oit/rust-2026-template' "$REPO"
replace_in_file ".github/workflows/sync-labels.yml" 'd-oit/rust-2026-template' "$REPO"

# --- rewrite label scripts ---
log "Updating label management scripts"
replace_in_file "scripts/learn-labels.sh" 'd-oit/rust-2026-template' "$REPO"
replace_in_file "scripts/setup-github-labels.sh" 'd-oit/rust-2026-template' "$REPO"

# --- rewrite skill author metadata ---
log "Updating skill author metadata"
for skill_md in .agents/skills/*/SKILL.md; do
  if grep -q 'author: d-oit' "$skill_md" 2>/dev/null; then
    replace_in_file "$skill_md" 'author: d-oit' "author: $AUTHOR"
  fi
done

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
