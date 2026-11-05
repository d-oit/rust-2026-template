---
name: verify-actions
description: >
  Verify GitHub Actions / GitLab CI action versions against actual releases before
  pinning. Never trust version comments — always resolve the real SHA from the
  upstream repository. Use when editing CI workflows, adding new actions, auditing
  pinned versions, or fixing version mismatches.

license: MIT
compatibility: Works with Claude Code, OpenCode, and similar agents. Requires gh CLI for GitHub or glab CLI for GitLab.
allowed-tools: Bash Read Grep Glob WebFetch
metadata:
  category: ci
  author: d-oit
  version: "1.1"
  tags: ci cd github-actions gitlab-actions sha-pinning security supply-chain
---

# Verify Actions

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
| `actions/checkout` | `3d3c42e5aac5ba805825da76410c181273ba90b1` | v7.0.1 | 2026-11-05 |
| `actions/upload-artifact` | `cf430e030ddbb5b0abf93d22962f4752f3646cd9` | v7.0.2 | 2026-11-05 |
| `actions/download-artifact` | `9000827ccba6bdab643e8b6fd33ac0654aef8333` | v8.0.2 | 2026-11-05 |
| `actions/github-script` | `3a2844b7e9c422d3c10d287c895573f7108da1b3` | v9.0.0 | 2026-11-05 |
| `actions/labeler` | `bf12e9b00b37c5c0ca2b87b79b2daf7891dbda13` | v7.0.0 | 2026-11-05 |
| `actions/setup-node` | `949feb2413d6458794dcd2491c4babbbce0c15c1` | v7.1.0 | 2026-11-05 |
| `actions/setup-python` | `6d4fd22b20465ccc0f7354476588e8df7c9ca0e0` | v7.1.0 | 2026-11-05 |
| `actions/attest-build-provenance` | `e8998f949152b193b063cb0ec769d69d929409be` | v2.4.0 | 2026-11-05 |
| `codecov/codecov-action` | `303a32d7a59b442fa8d48b6a1cc6825c09c847a5` | v7.1.1 | 2026-11-05 |
| `dependabot/fetch-metadata` | `25dd0e34f4fe68f24cc83900b1fe3fe149efef98` | v3.1.0 | 2026-11-05 |
| `dorny/paths-filter` | `ceb8a2b8f2d89434be7ff52d3de7ec3738c5cc9d` | v4.0.3 | 2026-11-05 |
| `dtolnay/rust-toolchain` | `b725eda525362e7cf34af9a7243cac1e3f01a7ec` | stable | 2026-11-05 |
| `mozilla-actions/sccache-action` | `9e7fa8a12102821edf02ca5dbea1acd0f89a2696` | v0.0.10 | 2026-11-05 |
| `peter-evans/create-pull-request` | `5f6978faf089d4d20b00c7766989d076bb2fc7f1` | v8.1.1 | 2026-11-05 |
| `Swatinem/rust-cache` | `d1527c8f67ef9af357290475d4af4b0fab8f12e0` | v2.9.3 | 2026-11-05 |
| `taiki-e/install-action` | `86f7a0ffb25f40114606501a311ebcfaf882ea32` | v2.88.0 | 2026-11-05 |
| `wagoid/commitlint-github-action` | `b948419dd99f3fd78a6548d48f94e3df7f6bf3ed` | v6.2.1 | 2026-11-05 |

> **Rule:** Before using a SHA from this table, verify it still matches the
> claimed version. Run `gh api` to confirm. This table drifts: the previous
> revision listed `actions/checkout` v6, `actions/github-script` v7, and
> `Swatinem/rust-cache` v2 SHAs that were two major versions behind, and every
> one of them ran on the deprecated Node.js 20 runtime.

### Checking for runtime deprecations

An action major version can be pinned correctly and still be unusable because
GitHub retired its Node runtime. Check the declared runtime directly:

```bash
gh api "repos/{owner}/{repo}/contents/action.yml?ref={tag}" \
  --jq '.content' | base64 -d | grep -m1 'using:'
```

A `node20` result produces this warning in every job that uses it:

```
Node.js 20 is deprecated. The following actions target Node.js 20 but are being
forced to run on Node.js 24: ...
```

Fixing it requires a **major** version bump, because the Node 24 runtime landed
in a new major line for nearly every action. Enumerate them all at once:

```bash
grep -rhoE "uses: [a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+@[a-f0-9]{40}" .github/ \
  | sed 's/uses: //' | sort -u \
  | while IFS=@ read -r repo sha; do
      printf '%-45s %s\n' "$repo@${sha:0:7}" \
        "$(gh api "repos/$repo/contents/action.yml?ref=$sha" --jq '.content' \
            | base64 -d | grep -m1 'using:')"
    done

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
| "The version comment already tells me which version this is" | The comment is a claim, not a fact. Resolve the SHA against the upstream tag before trusting it. |
| "I'll just bump the SHA to the latest commit on main" | Pinning `main` defeats the purpose of SHA pinning — the supply chain is unpinned again. |
| "Only one workflow uses this action, so no consistency check needed" | Version skew usually appears later, when a second workflow is added or a matrix job runs. Check on every change. |
