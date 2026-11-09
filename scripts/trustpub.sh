#!/usr/bin/env bash
# scripts/trustpub.sh
# Thin wrapper around scripts/trustpub.py — see its --help for commands and scope.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "${SCRIPT_DIR}/trustpub.py" "$@"
