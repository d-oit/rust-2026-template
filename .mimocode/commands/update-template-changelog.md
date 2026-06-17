---
description: Update CHANGELOG-TEMPLATE.md with a new version section from git history
agent: build
---

Update `.template/CHANGELOG-TEMPLATE.md` and `README.md` for a new release version.

## Step 1: Determine the version

If $ARGUMENTS is provided, use it as the new version (e.g. `0.4.0`, `1.0.0`).
Otherwise, detect the current latest version from the changelog and bump the patch number.

## Step 2: Collect changes

Run this to see commits since the last documented version:

!`git log --oneline`

Analyze the commit messages and categorize each into Keep a Changelog sections:

| Commit prefix | Changelog section |
|---|---|
| `feat` | ### Added |
| `fix` | ### Fixed |
| `perf` | ### Changed |
| `refactor` | ### Changed |
| `chore` (deps, ci, tooling) | ### Changed |
| `docs` | ### Fixed |
| `security` | ### Security |
| `BREAKING CHANGE` or `!` | ### Changed (with breaking note) |

Skip `chore: update CI status` and `[skip ci]` commits — they are noise.

## Step 3: Update CHANGELOG-TEMPLATE.md

1. Read the current `.template/CHANGELOG-TEMPLATE.md`.
2. Insert a new `## [VERSION] - YYYY-MM-DD` section right after the `## [Unreleased]` block (after the `---` separator that follows Unreleased).
3. Add categorized entries under the appropriate `### Added`, `### Changed`, `### Fixed`, `### Security` subsections.
   - Only include subsections that have entries.
   - Use the exact format: `- \`component\` description.` or `- Description.` for each entry.
   - Reference files, skills, workflows, and crates where relevant.
4. Update the link references at the bottom:
   - Change `[Unreleased]` compare URL to point from the new version.
   - Add a new `[VERSION]` compare URL from previous version to new version.

## Step 4: Update README.md

1. Read `README.md`.
2. Update the `Template Version` badge to the new version number and anchor link.
   - Badge URL: `https://img.shields.io/badge/version-VERSION-blue`
   - Link target: `.template/CHANGELOG-TEMPLATE.md#VERSIONANCHOR`

## Constraints

- Follow the existing changelog style exactly (Keep a Changelog format).
- Use today's date in `YYYY-MM-DD` format.
- Do not modify entries in older versions.
- Keep the `[Unreleased]` section as a clean template skeleton.
- Do not add comments or explanatory text to the changelog — only release notes.
