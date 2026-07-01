# Command: bump-template-version

## Description
Bumps the **template** repository's own internal version — the version tracked
in `.template/CHANGELOG-TEMPLATE.md`'s link footer. **This is the SOLE place
where the template's version lives**, by design — `VERSION`, `CHANGELOG.md`,
`Cargo.toml [workspace.package].version`, and the `README.md` version badge are
all kept free of any version number so a `bump-template-version` run mutates
exactly one file.

The per-repo version that derived repos use (in their own `VERSION` /
`Cargo.toml [workspace.package].version` / `CHANGELOG.md`) starts at `0.0.0`
(the init value they inherit) and is bumped separately by
`scripts/bump-version.sh` after a repo is initialized via `init-template.sh`.

## When to use
After merging a batch of template improvements (CI/workflow fixes, new
badges, new scripts, skill additions) and you're ready to publish the next
template version. Typical cadence: few times per month, not per-PR.

## Execution Protocol
1. **Pre-flight (required):**
   - Ensure `.template/CHANGELOG-TEMPLATE.md` has a populated `## [Unreleased]`
     section with the changes you want to publish.
   - Confirm `VERSION`, `CHANGELOG.md`, and `Cargo.toml [workspace.package].version`
     are still in their initial (untouched) state (`VERSION` = `0.0.0`, the
     generated-project starter version; `CHANGELOG.md` = empty Keep-a-Changelog
     skeleton). The script will refuse to touch them, but verify as a sanity
     check.
2. **Dry run (always do this first):**
   ```bash
   bash scripts/bump-template-version.sh
   ```
   Default: auto-increments the PATCH component of the current version.
   Prints exactly which lines in `.template/CHANGELOG-TEMPLATE.md` would
   change without modifying anything.
3. **Apply:**
   ```bash
   bash scripts/bump-template-version.sh --execute
   ```
   Optional flags:
   - `--minor`, `--major` to bump a non-patch component.
   - `--version=X.Y.Z` to set an explicit version (skips auto-increment).
   - `--date=YYYY-MM-DD` to override today's date in the changelog.
4. **Review and commit:**
   - `git diff .template/CHANGELOG-TEMPLATE.md` (should be exactly ONE file)
   - `git add .template/CHANGELOG-TEMPLATE.md`
   - `git commit -m 'chore(template): bump version to X.Y.Z'`
5. **Tag and push:**
   ```bash
   git tag vX.Y.Z
   git push origin main --tags
   ```

## What it does NOT touch (intentionally single-file)
- `.template/CHANGELOG-TEMPLATE.md` is the **only** file mutated.
- `VERSION` (kept at `0.0.0` — this is the generated-project starter version, not the template's own)
- `CHANGELOG.md` (kept in its initial Keep-a-Changelog skeleton state — this is the generated-project changelog, not the template's own)
- `Cargo.toml` `[workspace.package].version` (workspace baseline; stays `0.0.0` — derived repos inherit and bump locally via `scripts/bump-version.sh`)
- `README.md` (no version badge — the changelog IS the source of truth)
- `rust-toolchain.toml` (toolchain channel, not crate version)
- `deny.toml`, `target/`, `.git/`

## Goal
Keep the **template's own version** atomic to a single file so the diff for
any `bump-template-version` run is trivially reviewable and reviewable in
isolation from per-repo version changes.

## Scope note: template-only
`scripts/bump-template-version.sh` is **template-infrastructure only**: it
bumps `.template/CHANGELOG-TEMPLATE.md` on the rust-2026-template repo
itself. **`init-template.sh` does NOT install this script into derived
repos** — derived repos use `scripts/bump-version.sh` for their own per-repo
versioning (which lives in `VERSION` / `Cargo.toml [workspace.package].version` /
their own `CHANGELOG.md`, not in the template's `.template/` directory).
