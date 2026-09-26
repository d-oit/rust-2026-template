# gh Recipes for PR and Issue Triage

Verified invocations for reading and mutating GitHub PRs and issues from an
agent. Read this when a command in `SKILL.md` needs detail, when a REST payload
is required, or when a call fails with 403/422.

Substitute `{owner}/{repo}` and the PR or issue number in every example. Read
the repository once (`gh repo view --json nameWithOwner`) rather than guessing.

## Contents

- [Reading state](#reading-state)
- [Inline line comments](#inline-line-comments)
- [Closing and reasons](#closing-and-reasons)
- [Auth scopes](#auth-scopes)
- [Error recovery](#error-recovery)
- [Rate limits and pagination](#rate-limits-and-pagination)

## Reading state

`gh pr list` fields worth requesting together — one call, no round trips:

```bash
gh pr list --state open --limit 300 --json \
  number,title,url,createdAt,updatedAt,author,additions,deletions,changedFiles,\
isDraft,reviewDecision,mergeStateStatus,statusCheckRollup,maintainerCanModify,\
headRefName,baseRefName
```

`reviewDecision` is `null` when no review has been submitted. That is not the
same as `APPROVED`; treat `null` as "no blocking review" and do not read it as
endorsement.

`statusCheckRollup` nests one entry per check with `name`, `conclusion`, and
`status`. A `PENDING` or `SKIPPED` entry is not a failure; only `FAILURE`,
`TIMED_OUT`, `CANCELLED`, and `ACTION_REQUIRED` block.

Per-item reads:

```bash
gh pr view N --json number,title,body,url,author,createdAt,updatedAt,isDraft,\
reviewDecision,mergeStateStatus,statusCheckRollup,latestReviews,files,commits,\
maintainerCanModify

gh issue view N --json number,title,body,url,labels,author,createdAt,updatedAt,comments
```

Prior comments and reviews, which are **not** included in `gh pr view`:

```bash
gh api "repos/{owner}/{repo}/pulls/N/comments" --paginate   # inline, line-level
gh api "repos/{owner}/{repo}/pulls/N/reviews"   --paginate   # top-level verdicts
gh pr checks N                                       # human-readable table
```

Author association, needed for the blast-radius table:

```bash
gh api graphql -f query='
  query($owner:String!,$repo:String!,$number:Int!){
    repository(owner:$owner,name:$repo){
      pullRequest(number:$number){ authorAssociation maintainerCanModify }
    }
  }' -F owner=OWNER -F repo=REPO -F number=N
```

`authorAssociation` values: `OWNER`, `MEMBER`, `COLLABORATOR`, `CONTRIBUTOR`,
`FIRST_TIME_CONTRIBUTOR`, `FIRST_TIMER`, `NONE`.

## Inline line comments

Line-level comments cannot be posted with `gh pr review`; they need the REST
reviews endpoint. `line` is a **line in the diff**, not a file line number, and
it must exist in the head commit.

```bash
cat > /tmp/findings.json <<'JSON'
{
  "commit_id": "<head sha>",
  "body": "Review summary",
  "event": "COMMENT",
  "comments": [
    { "path": "crates/foo/src/lib.rs", "line": 42, "side": "RIGHT",
      "body": "**[MEDIUM]** This branch drops the error.\n`crates/foo/src/lib.rs:42`\nFix: propagate with `?`." }
  ]
}
JSON
gh api "repos/{owner}/{repo}/pulls/N/reviews" --method POST --input /tmp/findings.json
```

Get the head SHA from `gh pr view N --json headRefOid --jq .headRefOid`.

`event` is one of `APPROVE`, `REQUEST_CHANGES`, `COMMENT`, `PENDING`. Use
`COMMENT` unless a human explicitly asked for a verdict.

## Closing and reasons

The two commands are **not** symmetric:

```bash
gh pr close N --comment "<evidence>"          # no --reason flag exists
gh issue close N --reason "not planned" --comment "<evidence>"
gh issue close N --duplicate-of M --comment "<evidence>"
```

Valid issue reasons: `completed`, `not planned`, `duplicate`. Issues expose
`state_reason` over REST; pull requests do not, which is why a PR close must
carry its whole rationale in the comment.

`--delete-branch` also removes the branch locally and remotely. Omit it unless
the user asked; the branch is the author's undo path.

## Auth scopes

| Operation | Scope needed |
|---|---|
| Read PRs, issues, diffs, checks | `repo` (private) or public read |
| Post a review or comment | `pull-requests: write` |
| Close a PR | `pull-requests: write` |
| Close an issue | `issues: write` |

`gh auth status` reports the current scopes. A `GITHUB_TOKEN` from Actions is
usually read-only unless the workflow declares `permissions:`. When a write
returns 403, check scopes before retrying — retrying does not fix authorisation.

## Error recovery

| Status | Cause | Response |
|---|---|---|
| 403 | Missing scope, or SSO not authorised | Report the scope needed. Do not retry. |
| 404 | Wrong repo, or a private repo the token cannot see | Verify `gh repo view`. |
| 422 | Inline comment `line` not in the diff, or `commit_id` is stale | Re-read the head SHA. If it persists, fall back to one body-level comment via `gh pr review N --comment --body-file`. Never guess a `line` repeatedly. |
| 429 / "spammed" | Secondary rate limit on writes | Stop writing. Batch remaining posts with a delay. |

A 422 on an inline comment is a payload problem, not a transient failure. Two
retries with the same payload will never succeed.

## Rate limits and pagination

REST is 5,000 requests/hour per token; GraphQL is 5,000 points/hour. Writes are
further limited by a secondary rate limit that triggers on bursts regardless of
hourly quota.

- `--paginate` on every `gh api` list call; the default is a single page.
- One `gh pr list --json` call beats 300 individual `gh pr view` calls.
- For a large queue, page with `--limit`/`--slurp` and sleep between writes.
- Re-read each item's state immediately before mutating it. A triage queue
  changes while you read it.
