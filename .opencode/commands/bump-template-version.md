# Command: bump-template-version

## Description
Bumps the **template** repository's own internal version — the version tracked
in `.template/CHANGELOG-TEMPLATE.md`'s link footer and reflected in the
`README.md` version badge. This is **distinct** from the per-repo version
tracked in `VERSION` / `Cargo.toml` `[workspace.package].version`, which stays
at `0.0.0` (the init value that derived repos inherit) and is bumped separately
by `scripts/bump-version.sh` after a repo is initialized via `init-template.sh`.

## When to use
After merging a batch of template improvements (CI/workflow fixes, new
badges, new scripts, skill additions) and you're ready to publish the next
template version. Typical cadence: few times per month, not per-PR.

## Execution Protocol
1. **Pre-flight (required):**
   - Ensure `.template/CHANGELOG-TEMPLATE.md` has a populated `## [Unreleased]`
     section with the changes you want to publish.
   - Confirm `VERSION` and `Cargo.toml` `[workspace.package].version` are
     still at `0.0.0` (the script will refuse to touch them, but verify
     anyway as a sanity check).
2. **Dry run (always do this first):**
   ```bash
   bash scripts/bump-template-version.sh
   ```
   Default: auto-increments the PATCH component of the current version.
   Prints exactly which files/lines would change without modifying anything.
3. **Apply:**
   ```bash
   bash scripts/bump-template-version.sh --execute
   ```
   Optional flags:
   - `--minor`, `--major` to bump a non-patch component.
   - `--version=X.Y.Z` to set an explicit version (skips auto-increment).
   - `--date=YYYY-MM-DD` to override today's date in the changelog.
4. **Review and commit:**
   - `git diff .template/CHANGELOG-TEMPLATE.md README.md`
   - `git add .template/CHANGELOG-TEMPLATE.md README.md`
   - `git commit -m 'chore(template): bump version to X.Y.Z'`
5. **Tag and push:**
   ```bash
   git tag vX.Y.Z
   git push origin main --tags
   ```

## Color-flexibility
The version badge rewrite is regex-driven:
`version-<DIGITS>.<DIGITS>.<DIGITS>-<ANYCOLOR>.svg` → `version-<NEXT>-blue.svg`.
It tolerates `blue`, `informational`, `green`, etc. so a shields.io colour
change in `README.md` won't break the script.

## What it does NOT touch
- `VERSION` (per-instance init value, must stay at `0.0.0`)
- `Cargo.toml` `[workspace.package].version` (workspace baseline)
- `CHANGELOG.md` (per-instance template skeleton, stays empty)
- `rust-toolchain.toml` (toolchain channel, not crate version)
- `deny.toml`, `target/`, `.git/`

## Goal
Automate semantic version promotion for the **template infrastructure** (the
files other repos consume) without colliding with the per-repo versioning
that derived repos do on their own.

## Scope note: template-only
`scripts/bump-template-version.sh` is **template-infrastructure only**: it
bumps `.template/CHANGELOG-TEMPLATE.md` and `README.md` on the rust-2026-template
repo itself. **`init-template.sh` does NOT install this script into derived
repos** — derived repos use `scripts/bump-version.sh` for their own per-repo
versioning (which lives in `VERSION` / `Cargo.toml [workspace.package].version` /
their own `CHANGELOG.md`, not in the template's `.template/` directory).
