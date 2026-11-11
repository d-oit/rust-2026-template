# Template Gap Analysis (2026-11)

Scope: the whole template at `5c0a04b` (v0.4.1): Rust crates, CI/scripts, docs and agent tooling.

Baseline verified locally on Rust 1.88 / aarch64 Linux:

- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo test --workspace --all-features`: all suites pass.

The template compiles and is lint-clean. The gaps below are **behavioural**: features the READMEs promise but the code does not do, plus adopter-facing friction.

Legend: **[V]** = re-verified by hand in this review; otherwise found by static reading with file:line evidence.

---

## P0: correctness bugs and missing implementations

| # | Area | Finding | Evidence | Suggested fix |
|---|------|---------|----------|---------------|
| 1 | hybrid-storage | **[V]** The sync progress hook fires *before* the page is committed, so progress is reported for pages that may never be written | `crates/hybrid-storage-template/src/lib.rs:477-479` | Call `on_page_synced` after `apply_transaction` |
| 2 | hybrid-storage | **[V]** `trim_start_matches("rec:")` strips the prefix repeatedly, so ID `rec:x` comes back as `x` | `crates/hybrid-storage-template/src/lib.rs:348` | Use `strip_prefix` |
| 3 | hybrid-storage | `get_record` does not perform the ID check its doc promises | `lib.rs:270-287` | Compare `record.id == id`; return `CorruptRecord` on mismatch |
| 4 | hybrid-storage | "Monotonic watermark" is written but never enforced and never drives sync. "Bounded keyset pagination" loads and sorts every key on every page, so a full sync is O(N²/page) | `lib.rs:10-12,256-259,339-362` | Reject watermark regressions; push ordering and limits into `Backend`, or downgrade the README claims |
| 5 | checkpoint | Migration is a stub: `MigrationRegistry::migrate` returns its input unchanged, and `load` returns `VersionMismatch` without calling `Storable::migrate`. The payload is bincode but migration takes `serde_json::Value` | `crates/checkpoint-template/src/migration.rs:57-69`, `src/lib.rs:293-298` | Implement the migration chain on version mismatch, or remove "automatic migration hooks" from the README |
| 6 | actor-runtime | The supervisor never supervises: the strategy is unused (`_strategy`), JoinHandles are dropped, nothing restarts. `Backoff` and `OnError` are ignored in `run` | `crates/actor-runtime-template/src/supervisor.rs:25-55`, `src/actor.rs:137` | Keep JoinHandles, restart with `next_backoff`/`max_retries`, and add tests |
| 7 | actor-runtime | An invalid `Init` state is stored before it is validated | `src/actor.rs:99-100` | Validate first, then assign |
| 8 | mcp-server | No MCP protocol: no JSON-RPC, no stdio transport, no `initialize`/`tools/list`. "JSON schema generation" is advertised but `Tool` has no schema method | `crates/mcp-server-template/src/lib.rs:29`, `src/tool.rs:103-131` | Add `input_schema()` and a minimal stdio JSON-RPC loop, or reword the crate docs |
| 9 | examples | The egress validator fails open: hostname-only requests (e.g. `169.254.169.254` with `ip=None`) pass strict mode | `examples/security_egress_and_path_validation/src/main.rs:230-277` | Parse IP-literal hosts; fail closed when the IP is unknown |
| 10 | examples | The path validator ignores symlinks (text-only containment) | `examples/security_egress_and_path_validation/src/path_validator.rs:121-129` | Canonicalize both sides, or document the limitation |
| 11 | examples | `redact_text` panics on a non-char-boundary offset, and an inverted range leaks the secret. `DetectorConfig` lacks `deny_unknown_fields` | `examples/security_secret_detector/src/lib.rs:70,411-424` | Validate offsets; add the attribute |
| 12 | release | **[V]** cargo-release fails on its first run: `search = "unreleased"` with `min = 1` but CHANGELOG only has `Unreleased`. `VERSION` uses `search = ".*"` with `exactly = 1` | `release.toml:10-12` | Drop the lowercase entry; anchor the regex (`^\d+\.\d+\.\d+$`) |

## P1: agent tooling and template adoption

| # | Finding | Evidence | Suggested fix |
|---|---------|----------|---------------|
| 13 | **[V]** The Claude hooks don't match Claude Code's hook schema: `matcher` is an object with non-Claude tool names, and `SessionStart` lacks the `hooks` array. The deny.toml guard that ADR docs/adr/0002 relies on is therefore not enforced | `.claude/settings.json:3-30` | `"matcher": "Write\|Edit\|MultiEdit"` with the `{hooks:[…]}` wrapper |
| 14 | The Stop hook runs a full `cargo test` on every agent stop, bypassing nextest and the quality-gates SSOT | `.claude/settings.json:33-38` | Remove it, or scope it to changed crates |
| 15 | **[V]** `cargo xtask …` is documented but no alias exists | `.cargo/config.toml` `[alias]`; `README.md:217`, `agents-docs/tool-adapters.md` | Add `xtask = "run -p xtask --bin xtask --"` |
| 16 | Init rewrites `rust-2026-template` but leaves `d-oit/` in 46 files (NOTICE, badges, schema, skills), producing wrong `d-oit/<new-name>` URLs. `documentation = "https://docs.rs/your-crate"` is never rewritten | `crates/xtask/src/template_init/plan.rs:343-348,440-455`; `CONTRIBUTING.md:7`, `SECURITY.md:19`, `Cargo.toml:16` | Replace `d-oit/rust-2026-template` as a whole with `identity.repo`; add the missing placeholders |
| 17 | Template history ships to adopters: 690 of 1164 tracked files are event JSONs (`benchmarks/events`, `.agents/events`), plus `reports/`, `dora-metrics.jsonl`, `llms-full.txt` (gitignored *and* tracked), `.agents/ci/*`, and template entries in the root `CHANGELOG.md` | `git ls-files` | Add a "reset history" step to init |
| 18 | `cleanup-ci-status.yml` prunes with `find -mtime`, which never matches on a fresh checkout. It then pushes to protected `main` | `.github/workflows/cleanup-ci-status.yml:24-38` | Select by the `YYYY/MM/DD` path date; open a PR |
| 19 | **[V]** The root `tests/behaviour_harness.rs` is never compiled (virtual workspace) and is byte-identical to the copy in `workspace-tests` | `tests/behaviour_harness.rs` | Delete the root copy |
| 20 | Four `tests/*.sh` suites are never invoked by CI or xtask: `ci_required_evaluate`, `compare_benchmarks`, `distill_strikes`, `fuzz_dependency_regression` | `config/xtask.json`, `.github/workflows/ci.yml` | Wire them into a quality tier |
| 21 | Hook systems conflict: `core.hooksPath=.githooks` stops `pre-commit install`, so the commitlint and pre-push hooks in `.pre-commit-config.yaml` never run | `scripts/bootstrap.sh:158` | Pick one system |

## P2: CI coverage and hardening

| # | Finding | Evidence | Suggested fix |
|---|---------|----------|---------------|
| 22 | `rust-toolchain.toml` pins `1.88` (= MSRV), which overrides every `toolchain: stable` step, so **current stable is never tested** | `rust-toolchain.toml:4`; `ci.yml`, `docs-check.yml:32` | `channel = "stable"` + a dedicated MSRV job reading `rust-version` |
| 23 | CI is Linux x86_64 only. The toolchain file forces the x86_64 std download on ARM/macOS hosts, and dist has no `aarch64-unknown-linux-gnu` | `rust-toolchain.toml:12`, `dist-workspace.toml:18-23` | OS/arch matrix; drop `targets`; add the aarch64 dist target |
| 24 | **[V]** No `timeout-minutes` in any of the 21 workflows (default limit: 6 h) | `grep timeout-minutes .github/workflows` → 0 | Add per-job timeouts |
| 25 | Unpinned remote code: the actionlint installer comes from `main`; `gitleaks:latest`; mdbook `curl` without checksum | `ci.yml:301`, `release.yml:96`, `security-scan.yml:42`, `deploy-docs.yml:32` | Pin by SHA/digest/version |
| 26 | Bot PRs (`[skip ci]` + `GITHUB_TOKEN`) never get required checks, including the patch-release bump | `dora-report.yml:80`, `update-architecture-diagram.yml:60`, `patch-release-on-label.yml:170` | Use a GitHub App token; drop `[skip ci]` |
| 27 | `release.yml` has no concurrency group, and no `environment:` protection on the job that holds the write token | `release.yml:259` | Add both |
| 28 | Missing standard repo files: CODEOWNERS, CODE_OF_CONDUCT, SUPPORT, `ISSUE_TEMPLATE/config.yml`. Dependabot ignores semver-major updates for actions and has no `/fuzz` entry | `.github/dependabot.yml:18-37` | Add the files; allow major updates for actions; add `/fuzz` |

## P3: hygiene

- The SWAR printable-ASCII check is copied six times (sample-app, mcp, checkpoint, registry, path_validator, bench), and the bench copy has drifted. Extract a shared crate.
- Several benches don't measure the real code: `checkpoint_bench.rs` uses local copies, and `memory_usage.rs` measures no memory.
- The fuzz target asserts no invariants and sits outside the workspace (no lints, no MSRV). Add invariant asserts plus config and checkpoint-decode targets.
- `sample-app`: `init_logging` uses a panicking `.init()`, so use `try_init`. `process_items` silently truncates above 99,999. The docs mention YAML and anyhow, neither of which is used.
- ADRs are split across `plans/adr` and `docs/adr`, with duplicate 0001/0002 numbers and no 0004. `agents-docs/LESSONS.md` cites the wrong ADR numbers.
- `AGENTS.md` points agents at a stale `.agents/ci/ci-summary.md`. `docflow.json` is described as hook config but nothing reads it.
- Broken doc references: `make secrets-lint` (secret-lint skill) and `scripts/generate_diagram.py` (architecture-diagram reference) don't exist.
- Three divergent `dora-metrics.md` copies.
- The Makefile `.PHONY` list is incomplete and uses GNU-only `sed -i`.
- The composite `setup-rust` action declares inputs it never uses, and stacks sccache on top of rust-cache.

## Suggested order of work

1. P0 #1–#3, #7, #12, P1 #15, #19: small, mechanical, each with a regression test.
2. P1 #13–#14: makes the agent guardrails actually work.
3. P0 #5, #6, #8: implement or honestly downscope the advertised features.
4. P1 #16–#17: clean adoption, so adopters get no template identity or history.
5. P2: CI matrix, stable plus MSRV split, timeouts, pinning.
