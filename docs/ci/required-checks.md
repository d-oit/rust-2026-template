# Required Checks and the PR Merge Policy Gate

This document describes the branch-ruleset contract for `main`, the repository-owned
`PR Merge Policy Gate`, and the safe migration sequence for changing required contexts.

## Why a repository-owned gate exists

The `main` ruleset requires status contexts. GitHub cannot make a required context
conditional on a pull request's changed paths, and a workflow skipped by a `paths:`
filter leaves its required context pending forever. That combination means a
repository cannot simply "skip Codacy on report-only PRs": the check would never
report, and the PR would never be mergeable.

The repository therefore replaces the unconditional `Codacy Static Code Analysis`
context with one always-running context, `PR Merge Policy Gate`, which decides for
each PR whether Codacy is required.

## Required contexts

| Context | Source app | App ID |
|---|---|---|
| `Quality Gate (pull-request)` | GitHub Actions | `15368` |
| `PR Merge Policy Gate` | GitHub Actions | `15368` |

Codacy (`Codacy Static Code Analysis`, app ID `56611`) stays enabled and keeps
analyzing ordinary PRs, but it is enforced *through* the merge policy gate rather
than as a separate unconditional required context. See the migration section for the
intermediate state during rollout.

Both names must remain static literals in workflow YAML. A `name:` containing a
`${{ }}` expression is only evaluated when the job runs; a skipped job then reports
the raw expression as its check name and never satisfies the ruleset.

## What the gate enforces

The gate runs on every `pull_request` targeting `main` (`opened`, `synchronize`,
`reopened`, `ready_for_review`) and reads the exact current head SHA from
`github.event.pull_request.head.sha`.

**Generated maintenance PR.** Every changed file must match the literal allowlist in
`config/maintenance-pr.json`, the PR must target `main`, and the author and head
branch must be a recognized first-party automation identity. When those hold, the
gate requires a successful `Quality Gate (pull-request)` and intentionally does not
wait for Codacy. The PR must also have run the DORA artifact validators, which the
same required quality gate executes.

**Every other PR.** The gate requires a check run named exactly
`Codacy Static Code Analysis` from Codacy app ID `56611` on the current head SHA,
with conclusion `success`. It polls with a bounded timeout for pending runs.

The gate fails closed on: a missing or partial changed-file list, an unknown path, a
mixed maintenance-plus-source change, an untrusted actor or branch, a non-`main`
base, an API error, a truncated check-run page, a missing check run, a wrong app ID,
a non-`success` conclusion, and a timeout. PR titles, bodies, and labels are never
inputs to the decision.

Configuration lives in `config/maintenance-pr.json` and is validated by
`schema/maintenance-pr.schema.json`. The allowlist must be literal file paths; globs
and `..` segments are rejected so the fast path cannot widen accidentally.

## Ruleset migration

Do not remove the direct Codacy requirement before the new gate is live and has
proven itself on real PRs.

1. **Land the gate.** Merge `.github/workflows/pr-merge-policy.yml` and let it run
   on ordinary and generated PRs while the existing ruleset is unchanged.
2. **Shadow-compare.** Confirm the gate passes ordinary PRs that Codacy also passed,
   and that it fails when Codacy fails. Compare decisions for a full week.
3. **Add the required context.** In the `main` ruleset, add `PR Merge Policy Gate`
   sourced from GitHub Actions (app ID `15368`). Keep
   `Quality Gate (pull-request)` required.
4. **Remove the direct Codacy context.** Only after the gate has been green on
   ordinary PRs, remove `Codacy Static Code Analysis` as a direct required context.
5. **Verify.** Keep strict up-to-date branch policy, the CodeQL code-scanning rule,
   and the existing review policy. Add no administrator or bot bypass actor.

### Rollback

If the gate is unavailable or behaves incorrectly:

1. Re-add `Codacy Static Code Analysis` (app ID `56611`) as a required context.
2. Remove `PR Merge Policy Gate` from the required list.
3. Disable `.github/workflows/pr-merge-policy.yml` if it is failing spuriously.

Codacy integration itself is never disabled by this rollback, so ordinary PRs keep
receiving analysis throughout.

## Observability

The gate writes a summary to `GITHUB_STEP_SUMMARY` containing the decision, the
applied policy, the evaluated head SHA, the maintenance classification and reason,
the changed-file digest, and the enforced check's name, app ID, and conclusion. It
never logs tokens or raw API response bodies.

## Local verification

```bash
python3 scripts/classify-maintenance-pr.py \
  --files-json '["reports/DORA-REPORT.md"]' \
  --actor 'github-actions[bot]' --head-ref automation/dora-report \
  --base-ref main --event pull_request

python3 scripts/pr-merge-policy.py --pr <number> --head-sha <sha> \
  --actor <login> --head-ref <branch> --base-ref main

bash tests/maintenance_classification_test.sh
bash tests/pr_merge_policy_test.sh
bash tests/workflow_maintenance_contract_test.sh
```

The test suites are offline: they use fixtures instead of the GitHub API and need no
credentials.
