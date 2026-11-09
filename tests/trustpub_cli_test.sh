#!/usr/bin/env bash
# tests/trustpub_cli_test.sh
# Offline contract tests for scripts/trustpub.sh. No network, no credentials.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLI="$REPO_ROOT/scripts/trustpub.sh"
TOKEN="trustpub-test"
TOKEN="${TOKEN}-credential-placeholder"
WORK="$(mktemp -d)"
SERVER_LOG="$WORK/server.log"
PUBLISH_LOG="$WORK/publish.log"

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT

server_pid_from_log() {
  local port=""
  for _ in $(seq 1 100); do
    port="$(head -n1 "$SERVER_LOG" 2>/dev/null || true)"
    if [[ "$port" =~ ^[0-9]+$ ]]; then
      echo "$port"
      return 0
    fi
    sleep 0.05
  done
  echo "mock server did not start" >&2
  return 1
}

start_server() {
  : > "$SERVER_LOG"
  python3 "$REPO_ROOT/tests/fixtures/trustpub/mock_server.py" 0 > "$SERVER_LOG" &
  SERVER_PID=$!
  SERVER_PORT="$(server_pid_from_log)"
  API="http://127.0.0.1:${SERVER_PORT}"
}

start_server

WS="$WORK/ws"
mkdir -p "$WS"
cat > "$WS/Cargo.toml" <<'EOF'
[workspace]
members = [
  "dep-lib", "demo", "blocked", "placeholder", "unauth", "ratelimit",
  "servererr", "mismatch", "regfail",
]
resolver = "2"
EOF

make_crate() {
  local name="$1" version="$2" publish_block="$3" deps="$4"
  mkdir -p "$WS/$name/src"
  cat > "$WS/$name/Cargo.toml" <<EOF
[package]
name = "$name"
version = "$version"
edition = "2021"
$publish_block
$deps
EOF
  echo "pub fn marker() {}" > "$WS/$name/src/lib.rs"
}

make_crate dep-lib 0.1.0 "" ""
make_crate demo 0.1.0 "" $'[dependencies]\ndep-lib = { path = "../dep-lib" }'
make_crate blocked 0.1.0 "publish = false" ""
make_crate placeholder 0.0.0 "" ""
make_crate unauth 0.1.0 "" ""
make_crate ratelimit 0.1.0 "" ""
make_crate servererr 0.1.0 "" ""
make_crate mismatch 0.1.0 "" ""
make_crate regfail 0.1.0 "" ""

write_config() {
  local packages="$1"
  cat > "$WORK/config.json" <<EOF
{
  "repository_owner": "d-oit",
  "repository_name": "rust-2026-template",
  "workflow_filename": "release.yml",
  "environment": "release",
  "packages": [$packages]
}
EOF
}

run() {
  TRUSTPUB_API_URL="$API" \
  TRUSTPUB_ROOT="$WS" \
  TRUSTPUB_CONFIG="$WORK/config.json" \
  CARGO_REGISTRY_TOKEN="$TOKEN" \
  TRUSTPUB_POLL_ATTEMPTS=1 \
  TRUSTPUB_INDEX_WAIT_SECONDS=0 \
  TRUSTPUB_CARGO_PUBLISH="bash $WORK/fake_publish.sh" \
  "$@"
}

expect_fail() {
  local name="$1" needle="$2"
  shift 2
  local output
  if output="$(run "$@" 2>&1)"; then
    echo "FAIL: $name unexpectedly succeeded" >&2
    exit 1
  fi
  if [[ "$output" != *"$needle"* ]]; then
    echo "FAIL: $name output missing '$needle'" >&2
    echo "$output" >&2
    exit 1
  fi
}

state() {
  python3 - "$API" <<'PY'
import json, sys, urllib.request
with urllib.request.urlopen(sys.argv[1] + "/__state") as response:
    print(json.dumps(json.load(response)))
PY
}

cat > "$WORK/fake_publish.sh" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$*" >> "$PUBLISH_LOG"
EOF

write_config '"demo", "dep-lib"'

# 1. plan is read-only and orders dependencies before dependents.
BEFORE="$(state)"
PLAN="$(run "$CLI" plan)"
AFTER="$(state)"
request_count() {
  python3 -c 'import json,sys; print(len([r for r in json.loads(sys.argv[1])["requests"] if r[1] != "/__state"]))' "$1"
}
[[ "$(request_count "$AFTER")" == "$(request_count "$BEFORE")" ]] || {
  echo "FAIL: plan made network requests" >&2; exit 1; }
[[ "$PLAN" == *"dep-lib 0.1.0 -> d-oit/rust-2026-template workflow=release.yml environment=release"* ]] || {
  echo "FAIL: plan missing dep-lib tuple" >&2; exit 1; }
printf '%s\n' "$PLAN" | python3 -c '
import sys
lines = sys.stdin.read().splitlines()
order = [line.split()[0] for line in lines if line.startswith("  ")]
assert order == ["dep-lib", "demo"], order
' || { echo "FAIL: plan dependency order is wrong" >&2; echo "$PLAN"; exit 1; }

# 2. Invalid allowlists fail closed.
write_config '"blocked"'
expect_fail "publish=false allowlist" "publish = false" run "$CLI" plan
write_config '"placeholder"'
expect_fail "placeholder version" "0.0.0" run "$CLI" plan
write_config '"demo", "demo"'
expect_fail "duplicate allowlist entry" "duplicates" run "$CLI" plan
write_config '"missing"'
expect_fail "missing workspace crate" "not a workspace member" run "$CLI" plan

# 3. Empty allowlist skips publish without a token.
write_config ''
NO_TOKEN_OUT="$(TRUSTPUB_API_URL="$API" TRUSTPUB_ROOT="$WS" TRUSTPUB_CONFIG="$WORK/config.json" \
  "$CLI" release-plan)"
[[ "$NO_TOKEN_OUT" == *"has_packages=false"* ]] || {
  echo "FAIL: empty allowlist must report has_packages=false" >&2; exit 1; }

write_config '"demo", "dep-lib"'

# 4. configure registers once, is idempotent, and never leaks the token.
CONFIGURE_OUT="$(run "$CLI" configure --package demo --yes)"
[[ "$CONFIGURE_OUT" == *"demo: configured"* ]] || {
  echo "FAIL: configure did not report success" >&2; exit 1; }
run "$CLI" configure --package demo --yes >/dev/null
[[ "$(state | python3 -c 'import json,sys; print(json.load(sys.stdin)["posts"])')" == "1" ]] || {
  echo "FAIL: configure created a duplicate registration" >&2; exit 1; }
for output in "$CONFIGURE_OUT" "$(state)"; do
  [[ "$output" != *"$TOKEN"* ]] || { echo "FAIL: token leaked into output" >&2; exit 1; }
done
state | python3 -c '
import json, sys
config = json.load(sys.stdin)["configs"]["demo"][0]
assert config["repository_owner"] == "d-oit"
assert config["repository_name"] == "rust-2026-template"
assert config["workflow_filename"] == "release.yml"
assert config["environment"] == "release"
'

# 5. verify passes for the exact tuple and fails for a mismatch.
run "$CLI" verify --package demo >/dev/null
python3 - "$API" <<'PY'
import json, sys, urllib.request
payload = {
    "crate": "mismatch",
    "config": {
        "crate": "mismatch",
        "repository_owner": "d-oit",
        "repository_name": "rust-2026-template",
        "workflow_filename": "release.yml",
        "environment": "staging",
    },
}
request = urllib.request.Request(
    sys.argv[1] + "/__seed",
    data=json.dumps(payload).encode(),
    headers={"Content-Type": "application/json"},
    method="POST",
)
urllib.request.urlopen(request).read()
PY
expect_fail "mismatched verify" "verification failed" run "$CLI" verify --package mismatch

# 6. Mismatch requires explicit replacement; --replace deletes then recreates.
expect_fail "mismatch without replace" "without --replace" run "$CLI" configure --package mismatch --yes
run "$CLI" configure --package mismatch --replace --yes >/dev/null
[[ "$(state | python3 -c 'import json,sys; print(json.load(sys.stdin)["deletes"])')" -ge 1 ]] || {
  echo "FAIL: --replace did not delete the mismatched config" >&2; exit 1; }
run "$CLI" verify --package mismatch >/dev/null

# 7. API failures are actionable and fail closed.
expect_fail "unauthorized" "401" run "$CLI" list --package unauth
expect_fail "rate limited" "429" run "$CLI" list --package ratelimit
expect_fail "server error" "500" run "$CLI" list --package servererr

# 8. remove requires an explicit id and confirmation.
CONFIG_ID="$(state | python3 -c 'import json,sys; print(json.load(sys.stdin)["configs"]["demo"][0]["id"])')"
run "$CLI" remove --id "$CONFIG_ID" --yes >/dev/null
[[ "$(state | python3 -c 'import json,sys; print(len(json.load(sys.stdin)["configs"]["demo"]))')" == "0" ]] || {
  echo "FAIL: remove did not delete the config" >&2; exit 1; }

# 9. bootstrap publishes once and registers; registration failure does not republish.
write_config '"demo", "dep-lib", "regfail"'
expect_fail "bootstrap without confirmation" "requires --yes" run "$CLI" bootstrap --package demo
expect_fail "enforcement before publisher exists" "missing" \
  run "$CLI" require --package demo --enabled --confirm-oidc-publish --yes
run "$CLI" bootstrap --package demo --yes >/dev/null
[[ "$(wc -l < "$PUBLISH_LOG")" == "1" ]] || {
  echo "FAIL: bootstrap did not publish exactly once" >&2; exit 1; }
grep -q -- "--locked --package demo" "$PUBLISH_LOG" || {
  echo "FAIL: bootstrap publish arguments are wrong" >&2; exit 1; }
[[ "$(cat "$PUBLISH_LOG")" != *"$TOKEN"* ]] || {
  echo "FAIL: token passed as a process argument" >&2; exit 1; }
expect_fail "bootstrap registration failure" "Do not publish again" \
  run "$CLI" bootstrap --package regfail --yes
[[ "$(wc -l < "$PUBLISH_LOG")" == "2" ]] || {
  echo "FAIL: failed registration triggered a second publish" >&2; exit 1; }

# 10. Enforcement requires a verified publisher and explicit OIDC confirmation.
expect_fail "enforcement without OIDC confirmation" "confirm-oidc-publish" \
  run "$CLI" require --package demo --enabled --yes
run "$CLI" require --package demo --enabled --confirm-oidc-publish --yes >/dev/null
[[ "$(state | python3 -c 'import json,sys; print(json.load(sys.stdin)["trustpub_only"]["demo"])')" == "True" ]] || {
  echo "FAIL: enforcement was not enabled" >&2; exit 1; }
run "$CLI" require --package demo --disabled --yes >/dev/null
[[ "$(state | python3 -c 'import json,sys; print(json.load(sys.stdin)["trustpub_only"]["demo"])')" == "False" ]] || {
  echo "FAIL: enforcement rollback failed" >&2; exit 1; }

if grep -rq -- "$TOKEN" "$WORK" 2>/dev/null; then
  echo "FAIL: token persisted to disk" >&2
  exit 1
fi

echo "PASS: trusted publishing CLI contract tests"
