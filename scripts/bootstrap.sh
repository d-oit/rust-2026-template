#!/usr/bin/env bash
# bootstrap.sh - Single-command first-time setup for this Rust template.
# Installs skill symlinks, the pre-commit hook, validates skills, runs the quality gate.
# Idempotent: safe to re-run. See: scripts/doctor.sh for diagnostics on failure.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

DRY_RUN=0

# --- usage ---
usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Single-command first-time setup for the rust-2026-template.

Options:
  --dry-run    Preview changes without applying them
  -h, --help   Show this help message

Steps performed:
  1. Check environment (git, cargo)
  2. Install skill symlinks (if symlinks supported)
  3. Configure git hooks
  4. Validate skills
  5. Run quality gate

Prerequisites:
  - git 2.30+
  - Rust stable via rustup (toolchain pinned in rust-toolchain.toml)
  - Python 3 (optional, for TOML/YAML validation in pre-commit hook)

Platform notes:
  - Linux: mold linker recommended (auto-installed in CI)
  - macOS: mold not available; use zld or lld via: brew install zld
  - Windows: Enable Developer Mode for symlinks, or use WSL2

EOF
  exit 0
}

# --- parse args ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage ;;
    *) echo "Unknown option: $1" >&2; usage ;;
  esac
done

log()  { printf '==> %s\n' "$*"; return 0; }
ok()   { printf '  \033[0;32m✓\033[0m %s\n' "$*"; return 0; }
warn() { printf '  ! %s\n' "$*"; return 0; }
fail() { printf '\n\033[0;31m✗ %s\033[0m\n' "$*" >&2; exit 1; }

# --- platform detection ---
detect_platform() {
  case "$(uname -s)" in
    Linux*)  echo "linux" ;;
    Darwin*) echo "macos" ;;
    MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
    *)       echo "unknown" ;;
  esac
}

PLATFORM=$(detect_platform)
log "Platform: $PLATFORM"

# --- pre-flight ---
log "Checking environment"
command -v git >/dev/null 2>&1 || fail "git not found - install git first"
[[ -d .git ]] || fail "Run bootstrap.sh from the repository root"
ok "git present and inside a repository"

command -v cargo >/dev/null 2>&1 || fail "cargo not found - install Rust via rustup (https://rustup.rs)"
ok "cargo present"

# --- optional: gh CLI ---
if command -v gh &>/dev/null; then
  ok "gh CLI present (PR workflows available)"
  if ! gh extension list 2>/dev/null | grep -q gh-stack; then
    warn "gh-stack not installed (optional, for stacked PRs): gh extension install github/gh-stack"
  fi
fi

# --- linker hints ---
case "$PLATFORM" in
  linux)
    CLANG_MISSING=0
    MOLD_MISSING=0
    command -v clang >/dev/null 2>&1 || CLANG_MISSING=1
    command -v mold >/dev/null 2>&1 || MOLD_MISSING=1
    if [[ $CLANG_MISSING -eq 1 || $MOLD_MISSING -eq 1 ]]; then
      MISSING=()
      [[ $CLANG_MISSING -eq 1 ]] && MISSING+=("clang")
      [[ $MOLD_MISSING -eq 1 ]] && MISSING+=("mold")
      if [[ $DRY_RUN -eq 0 ]]; then
        log "Installing ${MISSING[*]}..."
        sudo apt-get update && sudo apt-get install -y "${MISSING[@]}"
      else
        warn "Would install: ${MISSING[*]} (dry run)"
      fi
      ok "${MISSING[*]} installed"
    else
      ok "mold + clang detected — maximum link speed"
    fi
    ;;
  macos)
    if command -v mold >/dev/null 2>&1; then
      ok "mold linker found"
    else
      warn "mold is not available on macOS. For faster builds, consider:"
      warn "  brew install zld"
      warn "  Then add to .cargo/config.toml: [target.aarch64-apple-darwin]\nlinker = \"zld\""
      warn "Continuing without mold..."
    fi
    ;;
  windows)
    warn "Windows detected. For full feature support:"
    warn "  - Enable Developer Mode for symlinks (Settings > For developers)"
    warn "  - Or use WSL2: https://aka.ms/wsl"
    ;;
esac

# --- skill symlinks ---
SYMLINK_TEST="$(mktemp -u)"
if ln -sf /dev/null "$SYMLINK_TEST" 2>/dev/null; then
  rm -f -- "$SYMLINK_TEST"
  log "Setting up skills"
  if [[ $DRY_RUN -eq 1 ]]; then
    ok "Skills setup (dry run - would run ./scripts/setup-skills.sh)"
  elif ./scripts/setup-skills.sh; then
    ok "Skills ready"
  else
    fail "setup-skills.sh failed - run ./scripts/doctor.sh for diagnostics"
  fi
else
  case "$PLATFORM" in
    windows)
      warn "Symlinks unavailable on Windows. Enable Developer Mode or use WSL2."
      ;;
    *)
      warn "Symlinks unavailable. Skills setup will be skipped."
      ;;
  esac
fi

# --- git hook ---
log "Configuring git hooks via .githooks"
if git config core.hooksPath | grep -q '.githooks' >/dev/null 2>&1 && [[ -d ".githooks" ]]; then
  ok "hooks already configured (core.hooksPath = .githooks)"
else
  if [[ $DRY_RUN -eq 1 ]]; then
    ok "Git hooks (dry run - would set core.hooksPath = .githooks)"
  else
    git config core.hooksPath .githooks
    chmod +x .githooks/* 2>/dev/null || true
    ok "git hooks configured (core.hooksPath = .githooks)"
  fi
fi

# --- pre-push hooks (for split commit/push pre-commit stages) ---
if command -v pre-commit &>/dev/null; then
  if [[ $DRY_RUN -eq 1 ]]; then
    ok "Pre-push hooks (dry run - would run pre-commit install --hook-type pre-push)"
  else
    pre-commit install --hook-type pre-push 2>/dev/null || warn "pre-commit install --hook-type pre-push failed (non-fatal)"
  fi
fi

# --- validate ---
log "Validating skills"
if [[ $DRY_RUN -eq 1 ]]; then
  ok "Skill validation (dry run - would run ./scripts/validate-skills.sh)"
elif ./scripts/validate-skills.sh >/dev/null 2>&1; then
  ok "Skills valid"
else
  warn "Skills validation reported issues - run ./scripts/doctor.sh for details"
fi

# --- quality gate ---
log "Checking linker configuration"
if [[ $DRY_RUN -eq 1 ]]; then
  ok "Linker check (dry run - would run ./scripts/check-linker.sh)"
else
  bash scripts/check-linker.sh
fi

log "Running quality gate"
if [[ $DRY_RUN -eq 1 ]]; then
  ok "Quality gate (dry run - would run ./scripts/quality-gates.sh)"
  printf '\nBootstrap dry run complete. No changes were made.\n'
  exit 0
elif ./scripts/quality-gates.sh; then
  ok "Quality gate passed"
  printf '\nBootstrap complete. Repository is ready for AI agent workflows.\n'
  exit 0
else
  printf '\nBootstrap completed with quality gate issues.\n' >&2
  printf 'Run ./scripts/doctor.sh for environment diagnostics.\n' >&2
  exit 1
fi
