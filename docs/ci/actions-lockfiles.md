# GitHub Actions workflow lockfile

GitHub Actions workflow dependency lockfiles became generally available on October
28, 2026. See GitHub's [feature announcement](https://github.blog/changelog/2026-10-28-github-actions-workflow-lockfiles-are-generally-available/)
and [workflow lockfile documentation](https://docs.github.com/en/actions/concepts/security/workflow-dependency-lockfiles).
The generated `.github/workflows/actions.lock` uses format `v0.0.3` and pins direct
and transitive action dependencies to verified commits. The repository overrides
the generic `*.lock` gitattributes pattern so the lockfile is reviewed as text and
its dependency diff remains visible in pull requests. Reusable workflow calls are
not covered by the caller's lockfile; the repository that defines a reusable
workflow must lock its own dependencies.

The official extension supports GitHub.com and GitHub Enterprise Cloud, including
data residency; it does not support GitHub Enterprise Server. It requires GitHub
CLI, but the upstream installation guide does not state a minimum CLI version. The
verification workflow uses the runner-provided CLI and pins the extension release.

## Updating the lockfile

Install the official [`gh-actions-lock` extension](https://github.com/github/gh-actions-lock)
with GitHub CLI, then regenerate the file after adding or changing workflow or
composite-action dependencies:

```sh
gh extension install github/gh-actions-lock --pin v0.4.1
gh actions-lock --migrate-local-actions --no-narrow
```

`--migrate-local-actions` rewrites local composite-action references from `./` to
`$/`, so the workflow lock can cover their remote action dependencies.
`--no-narrow` preserves the repository's existing full-SHA action references.
Review the complete diff, especially any changed `uses:` SHA or lockfile `commit`,
before committing. Do not edit `actions.lock` by hand.

The `verify-actions-lock` workflow runs the read-only `gh actions-lock --verify`
check on pull requests, pushes to `main`, and weekly. It never writes to the
checkout. Dependabot already updates GitHub Actions in this repository; for
onboarded workflows it also refreshes matching lockfile entries in its version
update pull requests.

Run the same verification locally before submitting workflow changes:

```sh
gh actions-lock --verify
```

If verification reports a stale or missing entry, regenerate the lockfile with the
command above and review the resulting dependency changes. An `unreachable-pin`
error means the pinned commit is no longer reachable from an upstream tag or
branch. Review the upstream history and update the workflow to a current verified
full SHA before regenerating; do not use `--accept-moved` in automation.

Template profiles may remove workflows, but the lockfile contains no adopter
repository coordinates. `cargo xtask template init` adds a `refresh-actions-lock`
checklist item whenever the selected profile removes workflows. After initializing
a profile or deleting workflows, run the full-directory regeneration command to
prune entries for removed workflows.
The lockfile covers action commits only, not container images or other workflow
inputs.
