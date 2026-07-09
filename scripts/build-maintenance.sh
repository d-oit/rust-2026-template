#!/usr/bin/env bash
set -euo pipefail

echo "🔍 Build maintenance check..."

if [ -d "target" ]; then
    SIZE_KB=$(du -sk target/ | awk '{print $1}')
    SIZE_GB=$(awk "BEGIN {printf \"%.1f\", $SIZE_KB / 1048576}")
    if [ "$SIZE_KB" -gt 5242880 ]; then  # >5GB
        echo "⚠ target/ is ${SIZE_GB}GB (threshold: 5GB)"
        echo "  Run: cargo clean"
        if [ "${1:-}" = "--fix" ]; then
            cargo clean
            echo "  ✓ Cleaned"
        fi
    else
        echo "✓ target/ size OK: ${SIZE_GB}GB"
    fi
else
    echo "✓ No target/ directory"
fi

if grep -q "profile.dev.package" .cargo/config.toml 2>/dev/null; then
    echo "✓ Dependency debug info disabled in .cargo/config.toml"
else
    echo "⚠ Missing [profile.dev.package.\"*\"] debug = 0 in .cargo/config.toml"
fi

echo ""
echo "Quick reference:"
echo "  cargo clippy -p <crate> -- -D warnings   # Fast single-crate lint"
echo "  cargo clean                               # Reset when target/ bloats"
echo "  du -sh target/                            # Check current size"
