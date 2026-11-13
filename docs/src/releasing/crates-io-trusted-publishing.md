# crates.io Trusted Publishing

This guide covers the full lifecycle for publishing crates from this template:
first publication, publisher registration, OIDC releases from GitHub Actions,
verification, enforcement, and rollback.

Official references stay authoritative:

- [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing)
- [RFC 3691: Trusted Publishing for crates.io](https://github.com/rust-lang/rfcs/blob/master/text/3691-trusted-publishing-cratesio.md)
- [cargo#17114: `cargo trustpub` proposal](https://github.com/rust-lang/cargo/issues/17114) (open, not a released subcommand)
- [rust-lang/crates-io-auth-action](https://github.com/rust-lang/crates-io-auth-action)

## Which packages are publishable

Publication is opt-in. `config/trusted-publishing.json` lists the crates that
may be published and the exact OIDC identity tuple:

```json
{
  "repository_owner": "d-oit",
  "repository_name": "rust-2026-template",
  "workflow_filename": "release.yml",
  "environment": "release",
  "packages": ["checkpoint-template"]
}
```

The template ships with a single canary entry, `checkpoint-template`, which is
the crate used to prove the full trusted-publishing path works in production.
Every other workspace member still sets `publish = false`, and
`publish = false` makes Cargo refuse to upload the package — the release tooling
refuses to select it even if it is added to the allowlist by mistake. To publish
another crate, remove its `publish = false`, replace the `0.0.0` placeholder
version, add its name to `packages`, and add a `LICENSE` file (every crate's
`include` list already references one).

The release workflow resolves the allowlist through
`./scripts/trustpub.sh release-plan`. Invalid configuration fails the
`publish_plan` job before any upload is attempted: missing crates, duplicates,
`publish = false`, placeholder versions, and dependency cycles all fail closed.

## Initial publication

crates.io only allows a Trusted Publisher to be configured after the crate
exists, so the first version is a one-time manual step. It is performed once per
crate, from a committed tree:

1. Create a short-lived, crate-scoped crates.io API token with the
   `publish-new` endpoint scope. Do not store it in GitHub Secrets or in a
   file. Export it only in the current shell:

   ```bash
   read -rsp 'crates.io token: ' CARGO_REGISTRY_TOKEN && export CARGO_REGISTRY_TOKEN
   ```

   `read -s` keeps the value out of shell history and terminal output.

2. Confirm the package, version, repository, and operation printed by the
   bootstrap command. It validates the allowlist, packaging contents, and
   `publish = false` before publishing:

   ```bash
   ./scripts/trustpub.sh bootstrap --package my-crate
   ```

3. For non-interactive use, add `--yes`. The command runs
   `cargo publish --locked --package my-crate`, waits until the version is
   visible, then registers the publisher. If publication succeeds but
   registration fails, it prints the safe follow-up command and never retries
   the publish.

Bootstrap never enables trusted-publishing-only enforcement.

## Publisher registration

`./scripts/trustpub.sh` is the repository's setup tool. It uses the documented
management API:

- `GET /api/v1/trusted_publishing/github_configs?crate=<name>` (read-only)
- `POST /api/v1/trusted_publishing/github_configs`
- `DELETE /api/v1/trusted_publishing/github_configs/<id>`
- `PATCH /api/v1/crates/<name>` with `{"crate": {"trustpub_only": true|false}}`

There is no released stable `cargo trustpub` subcommand. Do not document or
run one.

Use a token with the `trusted-publishing` endpoint scope, read only from
`CARGO_REGISTRY_TOKEN`:

```bash
read -rsp 'crates.io token: ' CARGO_REGISTRY_TOKEN && export CARGO_REGISTRY_TOKEN

# Read-only plan and current registrations.
./scripts/trustpub.sh plan
./scripts/trustpub.sh list --all-configured

# Register missing publishers; safe to rerun.
./scripts/trustpub.sh configure --package my-crate --yes

# Replace a mismatched registration only when explicitly requested.
./scripts/trustpub.sh configure --package my-crate --replace --yes

# Verify the exact expected tuple.
./scripts/trustpub.sh verify --all-configured

# Remove a registration by its returned id.
./scripts/trustpub.sh remove --id 42 --yes
```

The expected identity for this repository is:

| Field | Value |
|-------|-------|
| GitHub owner | `d-oit` |
| GitHub repository | `rust-2026-template` |
| Workflow file | `release.yml` |
| Environment | `release` |

`configure` treats an existing exact match as configured and does not create a
duplicate. A differing registration is reported as a diff and requires
`--replace`; existing registrations are never deleted automatically.

## GitHub Actions

`.github/workflows/release.yml` adds a `publish_crates` job that runs after the
existing preflight, release-tier quality gate, cargo-dist build, provenance
attestations, and GitHub Release creation. A crates.io failure therefore shows
up as a failed release job.

The job uses:

- `environment: release`, matching the registered OIDC claims;
- job-level permissions `contents: read` and `id-token: write` only;
- `rust-lang/crates-io-auth-action` pinned by SHA, which exchanges the GitHub
  OIDC token for a short-lived crates.io token;
- `CARGO_REGISTRY_TOKEN` set from `steps.auth.outputs.token` for the publish
  step only — never a GitHub Secret;
- `python3 scripts/trustpub.py release-publish`, which publishes only the
  allowlisted crates, skips versions already on crates.io, and respects
  workspace dependency order.

The action's post step revokes the temporary token when the job finishes.

### Create the `release` environment first

GitHub creates an environment named by a workflow job if it does not already
exist, and the new environment has **no protection rules**. The `publish_crates`
job would therefore run ungated while still appearing to target `release`.

Create the environment in **Settings → Environments** before the first release
tag, and add the protection the repository wants (required reviewers, a
deployment branch policy limited to release tags, or both). Only a repository
administrator can configure it, so this is a one-time manual step that a
workflow change cannot perform.

Verify it exists:

```bash
gh api repos/{owner}/{repo}/environments --jq '.environments[].name'
```

## Verification

Confirm registration before the first OIDC release:

```bash
./scripts/trustpub.sh verify --all-configured
```

Then push a release tag and confirm the `Publish to crates.io` job succeeds
and the version appears on crates.io. Common failures:

| Symptom | Cause | Fix |
|---------|-------|-----|
| `crates.io API responded 404` | Crate has no published version | Run `bootstrap` first |
| `crates.io API responded 401` | Token missing, expired, or invalid | Export a new `CARGO_REGISTRY_TOKEN` |
| `crates.io API responded 403` | Token lacks `trusted-publishing`, or you do not own the crate | Recreate the token with the right scope |
| `crates.io API responded 429` | Rate limited | Wait for the `Retry-After` delay, then retry |
| OIDC exchange fails | Workflow filename or environment mismatch | Fix the registration or workflow; both must match exactly |
| OIDC exchange fails | Wrong repository owner/name | Re-register with owner `d-oit`, repo `rust-2026-template` |
| Publish rejects package | Package still has `publish = false` | Remove the flag only if publication is intended |
| `publish_plan` fails | Allowlist contains a missing or placeholder crate | Fix `config/trusted-publishing.json` |

## Enforcement

Trusted-publishing-only mode is a separate crate setting, disabled by default.
Enable it only after at least one successful OIDC publish:

```bash
./scripts/trustpub.sh require --package my-crate --enabled \
  --confirm-oidc-publish --yes
```

The command verifies that the exact publisher tuple exists before changing the
setting, then re-fetches the crate and reports the effective value. It never
enables enforcement from `configure` or `bootstrap`.

Rollback restores token-based publishing:

```bash
./scripts/trustpub.sh require --package my-crate --disabled --yes
```

## Security

- Keep `id-token: write` scoped to the publish job; no other job needs it.
- Never add a long-lived crates.io token to repository or environment secrets.
- Never echo a token, pass it on a command line, or write it to disk.
- Pin the auth action by commit SHA under the repository policy; see
  `.agents/skills/verify-actions/SKILL.md`.
- Keep the allowlist explicit. Workspace membership is not publishability.
