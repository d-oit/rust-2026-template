# Project Profiles (issue #286)

Profiles are **validated blueprints** for initializing a new project from this template.
Each profile decides, declaratively:

- which crates under `crates/` are kept (everything not listed is removed),
- which non-crate paths are removed (`benchmarks`, `fuzz`, `.template`, `docs/patterns`),
- which `.github/workflows/*` files are removed,
- the **default CI verification tier** written into `config/xtask.json`,
- the **lockfile policy** (`policy.lockfile = "committed"` un-ignores `Cargo.lock` for binary profiles; `policy.lockfile = "ignored"` keeps `Cargo.lock` ignored for library profiles),
- the **post-init checklist** printed for items that cannot travel through a GitHub template.

Profiles live in `config/template-profiles/*.toml` and are structurally validated against
`schema/template-profile.schema.json` before use.

## Selection table

| Profile | Best for | Keeps | CI default tier | Lockfile policy |
|---|---|---|---|---|
| `minimal` | Small app | `sample-app` + renamed lib + `xtask` + tests | `pull-request` | Committed (`Cargo.lock` tracked) |
| `library` | Reusable library | renamed lib + `xtask` + tests | `protected-branch` | Ignored (`Cargo.lock` ignored) |
| `cli` | Binary tool | `sample-app` + renamed lib + `xtask` + tests | `protected-branch` | Committed (`Cargo.lock` tracked) |
| `service` | Long-running service | actor/storage/registry patterns + `xtask` | `protected-branch` | Committed (`Cargo.lock` tracked) |
| `workspace` | Full reference | every crate, benchmark, and workflow | `protected-branch` | Committed (`Cargo.lock` tracked) |
| `ai-agent` | Agent-centric dev | `sample-app` + lib + `xtask` + agent tooling | `pull-request` | Committed (`Cargo.lock` tracked) |

## Commands

```bash
# Initialize with a profile (rename + shape + CI tier + checklist)
./scripts/init-template.sh --profile library --name my-lib
cargo run -p xtask --bin xtask -- template init --profile service --name my-service

# Validate a blueprint (by id or by path) against the schema
cargo run -p xtask --bin xtask -- template validate-profile --profile config/template-profiles/library.toml

# Inspect a profile's plan
cargo run -p xtask --bin xtask -- template inspect --profile minimal

# Dry-run without modifying anything
./scripts/init-template.sh --profile minimal --name my-app --dry-run
```

`./scripts/init-template.sh --minimal` is a **backwards-compatible shorthand** for
`--profile minimal` (it delegates to xtask; behavior is otherwise identical).

## File ownership

| Kind | Files |
|---|---|
| **Template-owned** | `config/template-profiles/*.toml`, `schema/template-profile.schema.json`, `crates/xtask/src/template_profile.rs`, `scripts/init-template.sh` |
| **Downstream-owned** (adjust after init) | `config/xtask.json` (CI tiers), `Cargo.toml` metadata, `VERSION`, release/changelog policy |

`VERSION` is always the downstream project's version source of truth;
`.template/CHANGELOG-TEMPLATE.md` stays template-internal only.
