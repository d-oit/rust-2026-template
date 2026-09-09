#!/usr/bin/env bash
# init-template.sh - Thin compatibility wrapper around `cargo xtask template init`.
#
# All blueprint/profile logic lives in crates/xtask (issue #286): the six shipped
# profiles (minimal, library, cli, service, workspace, ai-agent) are validated TOML
# blueprints under config/template-profiles/. This script preserves the documented
# interactive entrypoint and maps the legacy `--minimal` flag to `--profile minimal`.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

DRY_RUN=0
PROFILE="workspace"   # default: keep the full reference workspace (old `full` behaviour)
PROJECT_NAME=""
PROJECT_DESC=""
AUTHOR=""
REPO=""

usage() {
  cat <<'EOF'
Usage: $(basename "$0") [OPTIONS]

Initialize a new project from the rust-2026-template (delegates to
`cargo run -p xtask --bin xtask -- template init --profile <id> ...`).

Profiles (config/template-profiles/): minimal | library | cli | service | workspace | ai-agent

Options:
  --profile PROFILE     Blueprint to apply (default: workspace)
  --minimal             Shorthand for --profile minimal (kept for backwards compatibility)
  --dry-run             Preview changes without applying them
  --name NAME           Project/crate name (e.g., "my-app")
  --description DESC    Project description
  --author AUTHOR       Author name (e.g., "Jane Doe")
  --repo REPO           GitHub repo (e.g., "myorg/my-app")
  -h, --help            Show this help message
EOF
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)    DRY_RUN=1; shift ;;
    --minimal)    PROFILE="minimal"; shift ;;
    --profile)    PROFILE="${2:?--profile requires a value}"; shift 2 ;;
    --name)       PROJECT_NAME="${2:?--name requires a value}"; shift 2 ;;
    --description) PROJECT_DESC="${2:?--description requires a value}"; shift 2 ;;
    --author)     AUTHOR="${2:?--author requires a value}"; shift 2 ;;
    --repo)       REPO="${2:?--repo requires a value}"; shift 2 ;;
    -h|--help)    usage ;;
    *)            echo "Unknown option: $1" >&2; usage ;;
  esac
done

# Interactive prompts (preserved from the legacy script).
prompt() { # var label
  local var="$1"
  if [[ -z "${!var:-}" ]]; then
    printf "%s: " "$2"
    read -r val
    printf -v "$var" '%s' "$val"
  fi
}
prompt PROJECT_NAME "Project name (e.g., my-app)"
[[ -n "$PROJECT_NAME" ]] || { echo "Project name is required" >&2; exit 1; }
prompt PROJECT_DESC "Description"
prompt AUTHOR "Author name"
prompt REPO "GitHub repo (org/name)"

ARGS=(--profile "$PROFILE" --name "$PROJECT_NAME")
[[ -n "$PROJECT_DESC" ]] && ARGS+=(--description "$PROJECT_DESC")
[[ -n "$AUTHOR" ]] && ARGS+=(--author "$AUTHOR")
[[ -n "$REPO" ]] && ARGS+=(--repo "$REPO")
[[ $DRY_RUN -eq 1 ]] && ARGS+=(--dry-run)

echo "==> Delegating to: cargo run -p xtask --bin xtask -- template init ${ARGS[*]}"
exec cargo run -p xtask --bin xtask -- template init "${ARGS[@]}"
