#!/usr/bin/env bash
# scripts/setup-harness.sh
# Pinned, reproducible, opt-in setup for do-harness.
# Reuses the upstream installer and checksum verification.

set -euo pipefail

DEFAULT_VERSION="v0.3.4"
DO_HARNESS_VERSION="${DO_HARNESS_VERSION:-$DEFAULT_VERSION}"
INSTALL_DIR="${DO_HARNESS_INSTALL_DIR:-$HOME/.local/bin}"
DRY_RUN=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Installs a pinned release of do-harness into $INSTALL_DIR (or \$DO_HARNESS_INSTALL_DIR).
Reuses the upstream installer with checksum verification. Setup is optional.

Options:
  --version VERSION   Version tag to install (default: $DEFAULT_VERSION)
  --bin-dir DIR       Target directory for binary (default: $HOME/.local/bin)
  --dry-run           Preview installation without executing
  -h, --help          Show this help message
EOF
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) DO_HARNESS_VERSION="$2"; shift 2 ;;
    --bin-dir) INSTALL_DIR="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage ;;
    *) echo "Unknown option: $1" >&2; usage ;;
  esac
done

echo "==> Setting up do-harness ($DO_HARNESS_VERSION)"

if [[ $DRY_RUN -eq 1 ]]; then
  echo "  [dry-run] Would download and verify do-harness $DO_HARNESS_VERSION into $INSTALL_DIR"
  exit 0
fi

mkdir -p "$INSTALL_DIR"

echo "  Downloading upstream installer and verifying checksums for $DO_HARNESS_VERSION..."
curl -fsSL https://raw.githubusercontent.com/d-o-hub/do-harness/main/scripts/install.sh \
  | sh -s -- --version "$DO_HARNESS_VERSION" --bin-dir "$INSTALL_DIR"

if command -v "$INSTALL_DIR/do-harness" &>/dev/null || command -v do-harness &>/dev/null; then
  echo "  ✓ do-harness ($DO_HARNESS_VERSION) installed successfully."
  echo "  Note: Existing project configuration (do-harness.toml) and git hooks (.githooks) are preserved."
else
  echo "  ! Installed do-harness binary but it is not currently on PATH."
  echo "    Ensure $INSTALL_DIR is in your PATH."
fi
