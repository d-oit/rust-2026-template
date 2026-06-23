---
name: harness
description: >
  Harness engineering guide — maps all sensors and feedforward guides,
  teaches self-correction protocol when a CI sensor fires.
  Use when a sensor fires, before making code changes, or when setting up
  agent context for a new task.
  Triggers: "harness", "sensor fire", "CI failure", "self-correction".
category: rust
license: MIT
metadata:
  author: d-oit
  version: "1.0"
  tags: harness sensors ci quality self-correction
---

# Skill: harness

## What Is the Harness

Agent = Model + Harness. The harness is the system of feedforward guides (what to do before coding) and feedback sensors (what catches violations after coding). It has two axes:

- **Feedforward (guides):** Context, constraints, conventions that prevent errors before they happen.
- **Feedback (sensors):** Automated checks that fire after code changes, providing structured error output.

The harness has two modes:
- **Computational:** Deterministic checks (fmt, clippy, deny) — always trust the output.
- **Inferential:** LLM-based guidance (skill docs, agent context) — direction, not commands.

## Sensor Response Protocol

When a computational sensor fires:

1. **Read the full error message** — it includes a fix hint.
2. **Classify the error:** fmt / lint / test / arch / security.
3. **Apply the minimal fix** — do not refactor unrelated code.
4. **Re-run the specific sensor** — `cargo fmt`, `cargo clippy`, etc.
5. **Only commit when the sensor is green.**
6. **Write a metrics event** to `.agents/events/YYYY/MM/DD/` per the `metrics-reporter` skill.

## Sensor Quick Reference

| Sensor | Command | Config | Stage |
|--------|---------|--------|-------|
| fmt | `cargo fmt --all -- --check` | `.pre-commit-config.yaml` | pre-commit |
| clippy | `cargo clippy --all-targets --all-features -- -D warnings` | `.clippy.toml`, `.pre-commit-config.yaml` | pre-commit + CI |
| deny | `cargo deny check` | `deny.toml` | pre-commit + CI |
| nextest | `cargo nextest run` | `Cargo.toml` | CI |
| mutants | `cargo mutants` | `[workspace.metadata.cargo-mutants]` in `Cargo.toml` | CI weekly |
| arch_fitness | `cargo test --test arch_fitness` | `tests/arch_fitness.rs` | CI |
| insta snapshots | `cargo insta review` | `tests/behaviour_harness.rs` | CI |
| gitleaks | `gitleaks detect` | `.gitleaks.toml` | CI |

## Steering Loop

When any sensor fires **repeatedly** (>2 times in one sprint):

1. Identify the root cause category (maintainability / architecture / behaviour).
2. Update the corresponding **feedforward guide** to prevent recurrence.
3. If no guide exists, create one in `.agents/skills/` using the `skill-creator` skill.
4. Document the update in `CHANGELOG.md`.

The steering loop closes the harness: sensors fire → humans and agents update guides → sensors fire less.

## References

- Full harness map: [`HARNESS.md`](../../HARNESS.md) at repo root.
- Agent-optimised error output: `scripts/harness-check.sh` runs each sensor and emits structured error output with `HARNESS VIOLATION` prefix.
