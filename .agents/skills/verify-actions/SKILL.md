---
name: verify-actions
description: >
  Verify GitHub Actions / GitLab CI action versions against actual releases before
  pinning. Never trust version comments — always resolve the real SHA from the
  upstream repository. Use when editing CI workflows, adding new actions, auditing
  pinned versions, or fixing version mismatches.
category: ci
license: MIT
compatibility: Works with Claude Code, OpenCode, and similar agents. Requires gh CLI for GitHub or glab CLI for GitLab.
allowed-tools: Bash Read Grep Glob WebFetch
metadata:
  author: d-oit
  version: "1.1"
  tags: ci cd github-actions gitlab-actions sha-pinning security supply-chain
---

# Verify Actions

## When to Use

- User asks for this skill's functionality


Never trust version comments in workflow files. Always verify the correct SHA
against the actual upstream release before pinning or updating an action.

## Why This Matters

- Comments like `# v4` can be wrong — a SHA may actually point to v3 or a branch
- Version mismatches across workflows cause subtle bugs (e.g., upload-artifact
  v4 can't download v7 artifacts)
- Supply-chain attacks target CI — pinning a wrong SHA from `main` instead of a
  tag defeats the purpose

## Agent Workflow

### 1. Before Changing Any Action Reference

When you encounter a `uses: owner/action@SHA  # vX` line, **verify the SHA**:

**For GitHub Actions:**

```bash
# Look up what tag a SHA belongs to
gh api repos/{owner}/{repo}/git/ref/tags/{tag} --jq '.object.sha'

# Or list recent tags to find the right one
gh api repos/{owner}/{repo}/tags --jq '.[].name' | head -20

# Verify a specific SHA matches a tag
gh api repos/{owner}/{repo}/git/ref/tags/v4 --jq '.object.sha'
```

**For GitLab Actions (CI/CD):**

```bash
# Check the registry or project for the correct image tag
# Use the project API or container registry
glab api projects/{id}/repository/tags
```

**Fallback — web fetch:**

```
WebFetch: https://github.com/{owner}/{repo}/releases/tag/{tag}
Confirm the release exists and note the commit SHA.
```

### 2. Version Consistency Check

Before committing workflow changes, verify ALL action references across the
workspace are version-consistent:

```bash
# Find all action references and group by action name
grep -rn 'uses:' .github/workflows/ .github/actions/ \
  | sed 's/.*uses: \([^@]*\)@.*/\1/' \
  | sort | uniq -c | sort -rn

# For each unique action, verify all SHAs match
grep -rn 'uses: actions/upload-artifact@' .github/workflows/
grep -rn 'uses: actions/checkout@' .github/workflows/
grep -rn 'uses: actions/download-artifact@' .github/workflows/
```

### 3. When Adding a New Action

1. Find the latest release tag from the upstream repo
2. Resolve the SHA for that tag using `gh api`
3. Pin the SHA with a version comment
4. Verify no other workflow uses a different version of the same action

### 4. When Updating an Action

1. Identify the current SHA and version in use
2. Find the latest release tag
3. Resolve the new SHA
4. Update ALL files that reference this action to the same SHA
5. Run the consistency check from step 2

## Canonical SHA Reference

These are verified SHAs for commonly used actions. **Always cross-check** before
using — upstream may have released newer versions.

| Action | Verified SHA | Version | Last Verified |
|--------|-------------|---------|---------------|
| `actions/checkout` | `de0fac2e4500dabe0009e67214ff5f5447ce83dd` | v6 | 2026-06-17 |
| `actions/upload-artifact` | `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a` | v7 | 2026-06-17 |
| `actions/download-artifact` | `3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c` | v4 | 2026-06-17 |
| `actions/github-script` | `f28e40c7f34bde8b3046d885e986cb6290c5673b` | v7 | 2026-06-17 |
| `dtolnay/rust-toolchain` | `29eef336d9b2848a0b548edc03f92a220660cdb8` | stable | 2026-06-17 |
| `Swatinem/rust-cache` | `e18b497796c12c097a38f9edb9d0641fb99eee32` | v2 | 2026-06-17 |
| `taiki-e/install-action` | `7a79fe8c3a13344501c80d99cae481c1c9085912` | v2 | 2026-06-17 |

> **Rule:** Before using a SHA from this table, verify it still matches the
> claimed version. Run `gh api` to confirm.

## Red Flags

- [ ] Action pinned from a branch (`# main`, `# master`) instead of a tag
- [ ] Version comment says one thing but SHA resolves to another
- [ ] Same action used with different SHAs across workflow files
- [ ] `download-artifact` version doesn't match `upload-artifact` version
- [ ] No version comment on a SHA-pinned reference
- [ ] SHA not resolvable via `gh api` (may be from a fork or deleted tag)

## Common Mismatches to Watch

| Action Pair | Must Match |
|-------------|-----------|
| `upload-artifact` + `download-artifact` | Same major version |
| `actions/checkout` across all workflows | Same SHA |
| `actions/upload-artifact` across all workflows | Same SHA |
| Composite action internal pins | Match consuming workflow versions |


## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "Placeholder" | "Placeholder" |
