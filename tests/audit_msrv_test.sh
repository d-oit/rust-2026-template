#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

FIXTURE="$TMP_DIR/repo"
BIN="$TMP_DIR/bin"
mkdir -p "$FIXTURE" "$BIN"

cat >"$BIN/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'cargo %s\n' "$*" >>"$TEST_LOG"
if [[ "${1:-}" == "metadata" ]]; then
  printf '{"packages":[{"name":"fixture","version":"0.1.0","rust_version":"%s"}]}\n' \
    "${DEPENDENCY_RUST_VERSION:-1.70}"
  exit 0
fi
if [[ "${FAKE_CARGO_FAIL:-0}" == "1" ]]; then
  exit 1
fi
EOF

cat >"$BIN/rustc" <<'EOF'
#!/usr/bin/env bash
printf 'rustc %s (fixture)\n' "${FAKE_RUSTC_VERSION:-1.100.0}"
EOF

cat >"$BIN/rustup" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "toolchain" && "${2:-}" == "list" ]]; then
  [[ ! -f "$INSTALLED_TOOLCHAINS" ]] || cat "$INSTALLED_TOOLCHAINS"
  exit 0
fi
if [[ "${1:-}" == "toolchain" && "${2:-}" == "install" ]]; then
  printf 'install %s\n' "${3:-}" >>"$TEST_LOG"
  if [[ "${FAKE_RUSTUP_FAIL:-0}" == "1" ]]; then
    exit 1
  fi
  printf '%s-x86_64-unknown-linux-gnu\n' "${3:?toolchain required}" >>"$INSTALLED_TOOLCHAINS"
  exit 0
fi
exit 2
EOF

chmod +x "$BIN/cargo" "$BIN/rustc" "$BIN/rustup"

write_manifest() {
  local msrv="$1"
  cat >"$FIXTURE/Cargo.toml" <<EOF
[workspace]
members = []

[workspace.package]
rust-version = "$msrv"
EOF
  printf '[toolchain]\nchannel = "stable"\n' >"$FIXTURE/rust-toolchain.toml"
}

run_audit() {
  PATH="$BIN:$PATH" \
    AUDIT_MSRV_ROOT="$FIXTURE" \
    TEST_LOG="$TMP_DIR/commands.log" \
    INSTALLED_TOOLCHAINS="$TMP_DIR/toolchains.txt" \
    "$REPO_ROOT/scripts/audit-msrv.sh"
}

reset_fixture() {
  rm -f "$TMP_DIR/commands.log" "$TMP_DIR/toolchains.txt"
  write_manifest "${1:-1.88}"
}

reset_fixture
if ! OUTPUT="$(run_audit 2>&1)"; then
  printf '%s\n' "$OUTPUT" >&2
  echo "ERROR: audit should read MSRV from Cargo.toml when toolchain is stable" >&2
  exit 1
fi
grep -q "Workspace MSRV: 1.88" <<<"$OUTPUT"
grep -q '^install 1.88$' "$TMP_DIR/commands.log"
grep -q '^cargo +1.88 check --workspace --all-targets --all-features$' "$TMP_DIR/commands.log"

reset_fixture "1.bad"
if run_audit >/dev/null 2>&1; then
  echo "ERROR: malformed workspace rust-version must fail" >&2
  exit 1
fi
if [[ -f "$TMP_DIR/commands.log" ]]; then
  echo "ERROR: malformed MSRV must fail before invoking Cargo" >&2
  exit 1
fi

reset_fixture
if FAKE_RUSTUP_FAIL=1 run_audit >/dev/null 2>&1; then
  echo "ERROR: failed MSRV toolchain installation must fail the audit" >&2
  exit 1
fi

reset_fixture
if DEPENDENCY_RUST_VERSION="1.89.0" run_audit >/dev/null 2>&1; then
  echo "ERROR: dependency requiring a newer Rust version must fail the audit" >&2
  exit 1
fi

reset_fixture
if FAKE_RUSTC_VERSION="1.88.0" run_audit >/dev/null 2>&1; then
  if grep -q '^install ' "$TMP_DIR/commands.log"; then
    echo "ERROR: installed MSRV compiler should not trigger a toolchain install" >&2
    exit 1
  fi
else
  echo "ERROR: matching current MSRV compiler should pass the audit" >&2
  exit 1
fi

echo "PASS: MSRV source, malformed versions, dependency mismatch, and toolchain installation"
