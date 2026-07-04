#!/usr/bin/env bash
# scripts/check-linker.sh — Cross-platform fast linker detection for bootstrap/doctor
# Called from bootstrap.sh and doctor.sh to verify the environment has the right linker.
set -euo pipefail

check_linux_linker() {
    if command -v mold &>/dev/null && command -v clang &>/dev/null; then
        echo "✅ mold + clang detected — maximum link speed"
    elif rustc --version 2>/dev/null | grep -qE '1\.(9[0-9]|[1-9][0-9]{2})\.'; then
        echo "⚠️  mold not found — Rust 1.90+ will use rust-lld (still fast)"
        echo "   For maximum speed: sudo apt install mold clang"
    else
        echo "⚠️  mold not found and Rust < 1.90 — using default ld (slow)"
        echo "   Install: sudo apt install mold clang"
    fi
}

check_macos_linker() {
    echo "✅ Using default macOS linker (ld64 — already optimized)"
}

check_windows_linker() {
    if command -v rust-lld &>/dev/null; then
        echo "✅ rust-lld detected (configured in .cargo/config.toml)"
    else
        echo "⚠️  rust-lld not found — should ship with Rust toolchain"
        echo "   Verify your Rust installation: rustup show"
    fi
}

case "$(uname -s)" in
    Linux*)  check_linux_linker ;;
    Darwin*) check_macos_linker ;;
    MINGW*|MSYS*|CYGWIN*) check_windows_linker ;;
    *) echo "Unknown platform: $(uname -s)" ;;
esac
