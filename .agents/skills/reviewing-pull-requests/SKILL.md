---
name: reviewing-pull-requests
description: >
  Review GitHub pull requests and triage open PRs and issues: classify blast
  radius, decide review versus close, and draft evidence-linked comments.
  Use when the user asks to review a PR, triage a PR queue, roast a change,
  decide whether a stale PR still matters, or address open review comments —
  even when they only name a PR number, and even when they never say "review".
  Triggers: "review PR", "roast this PR", "triage open PRs", "close stale PRs",
  "all open PRs", "address PR comments", "is this PR still needed".
license: MIT
metadata:
  category: workflow
  author: d-oit
  version: "1.0"
compatibility: Requires the gh CLI authenticated against the target repository. GitHub only; GitLab repositories need the issue-triage skill instead.
allowed-tools: Bash Read Grep Glob
---

Your knowledge of this repository's conventions, its GitHub state, and what a
given diff actually does is unreliable. **Prefer retrieval over pre-training.**
Read the PR body, the full files, the prior comments, and the check runs before
asserting anything about a change.

## Principles

- **Evidence or nothing.** A finding without a `path:line` and a verifiable
  reason is dropped, not softened.
- **Precision over recall.** A missed finding costs far less than a wrong one.
  Engineers forgive a tool that misses something; they abandon one that cries wolf.
- **Close, never delete.** Closing is reversible with a comment; deletion is not.
- **Never post unvetted.** Draft everything; a human submits every comment,
  verdict, and close. Never send `--approve` or `--request-changes` unprompted.
- **Staleness is not impact.** Age alone never justifies closing.

## Prerequisites

```bash
gh auth status || exit 1
gh repo view --json defaultBranchRef >/dev/null || exit 1
```

If `gh` is missing or unauthenticated, stop and report it. Do not fall back to
`curl` — you would lose the credential scoping that keeps writes auditable.

## Decision Rule: Review Or Close

Impact is **blast radius**, not diff size. A 2,000-line lockfile bump is low
impact; a 3-line change to an auth check is high impact.

| Signal | Low impact | Needs review |
|---|---|---|
| Touches public API surface, auth/crypto, permission checks, non-additive schema/migrations, build/release/CI config | no | yes |
| `mergeStateStatus` | `UNBLOCKED` | `DIRTY` |
| `reviewDecision` | not `CHANGES_REQUESTED` | `CHANGES_REQUESTED` |
| Required checks | all `SUCCESS` or `SKIPPED` | any failing or pending |
| `authorAssociation` / `maintainerCanModify` | `MEMBER`/`OWNER`/`COLLABORATOR`, or `maintainerCanModify: true` | `FIRST_TIME_CONTRIBUTOR` and `false` |
| `isDraft` | false | true (skip drafts entirely) |
| Diff is already in the base branch (`git diff base...head` empty) | yes | — |
| Pure docs/formatting/lockfile/generated churn | yes | — |

- Any row in **Needs review** → review it. Never close, regardless of age.
- Every row low-impact **and** idle past the repo's stale window → *propose* a
  close. Do not close without explicit instruction.
- Already implemented, or superseded by a merged PR → close with evidence
  naming the PR or commit that landed it.
- Duplicate issue → `gh issue close N --duplicate-of M -c "<evidence>"`.
- Not going to happen → `gh issue close N -r "not planned" -c "<evidence>"`.

`gh pr close` has **no `--reason` flag**; `gh issue close` does. For PRs, put
the whole rationale in `--comment`.

## What NOT to Flag

Ban complaint *categories*, not topics. Each line below is a class of noise:

- Style not enforced by a linter in this repository.
- "Could be cleaner" when the code is correct and clear.
- Performance concerns with no measurement attached.
- Security concerns with no concrete exploit path.
- Pre-existing issues in lines the diff did not touch.
- Missing tests for code that has no logic branch worth covering.
- Missing features that were never in scope.
- Rewording the author's prose when it is clear.

## Review Categories

| Category | Expectation |
|---|---|
| Correctness | Compiles. Handles the error paths it introduces. Logic matches intent. |
| Scope honesty | The PR description matches what the diff does. A `fix(security):` that touches only tests is a finding. |
| API compatibility | Public surface unchanged, or the break is deliberate and documented. |
| Security | Only concrete exploit paths. |
| Tests | Assert behaviour, not that a mock was called. |
| Style | **Not reviewed here** — delegated to the repo's linter and formatter. |

**Tests that only assert a mock was invoked are the most common worthless test.**

## Claims: verify before you believe

Verification is not optional. For every finding:

1. **Read the full file**, not just the changed hunk. Code that looks wrong in
   isolation may be correct given the surrounding logic.
2. **Re-read the exact `path:line`.** "Obviously valid" is a rationalisation
   word. Consensus is not correctness — two reviewers agreeing proves nothing.
3. **Check whether the repo's own history already answers it.** `git log -S`,
   blame, and the merged equivalent in the base branch.

Then classify the claim:

| Verdict | Meaning |
|---|---|
| VALID | Evidence directly supports it |
| PARTIAL | Real, but overstated |
| UNFOUNDED | Evidence contradicts it or it does not exist |
| SUBJECTIVE | Preference or style, not a defect |

Only VALID and PARTIAL become findings. UNFOUNDED goes in Dismissed, with the
evidence that disproved it. SUBJECTIVE is listed and the human decides.

## Commands

Field lists, REST payloads for inline comments, token scopes, and 403/422
recovery live in `references/gh-recipes.md`. Read it when a call fails or when
you need a line-level comment. Substitute `{owner}/{repo}` in every call.

| Task | Command |
|---|---|
| List open PRs | `gh pr list --state open --limit 300 --json number,title,url,createdAt,updatedAt,author,additions,deletions,changedFiles,isDraft,reviewDecision,mergeStateStatus,statusCheckRollup,maintainerCanModify,headRefName,baseRefName` |
| List open issues | `gh issue list --state open --limit 300 --json number,title,url,labels,author,createdAt,updatedAt,comments` |
| Read one PR | `gh pr view N --json number,title,body,url,author,createdAt,updatedAt,isDraft,reviewDecision,mergeStateStatus,statusCheckRollup,latestReviews,files,commits,maintainerCanModify` |
| Read one issue | `gh issue view N --json number,title,body,url,labels,author,createdAt,updatedAt,comments` |
| Files only | `gh pr diff N --name-only` |
| Full diff | `gh pr diff N` |
| CI conclusions | `gh pr checks N` |
| Prior inline comments | `gh api repos/{owner}/{repo}/pulls/N/comments --paginate` |
| Prior reviews | `gh api repos/{owner}/{repo}/pulls/N/reviews --paginate` |
| Draft review | `gh pr review N --comment --body-file <file>` |
| Close PR | `gh pr close N --comment "<evidence>"` |
| Close issue | `gh issue close N --reason "not planned" --comment "<evidence>"` |

Two traps: `reviewDecision: null` means no review was submitted, not approval.
Line-level comments need a REST payload keyed to the diff, not a line number —
a wrong `line` returns 422, so fall back to one body comment rather than
retrying. Write bulk findings to a file and use `--body-file`; never pass a long
roast through `-b`.

## Workflow

```
- [ ] 1. GATE     Verify gh auth and default branch. Stop on failure.
- [ ] 2. GATHER   Read full files, prior comments, and check runs. Never
                  repeat a comment already on the PR.
- [ ] 3. CLASSIFY Apply the decision table. Record which row decided it.
- [ ] 4. VERIFY   Re-read every candidate path:line. Drop what you cannot
                  confirm by reading it.
- [ ] 5. SET ASIDE List every behaviour dismissed, one line each, with the
                  reason. Nothing is dropped silently.
- [ ] 6. DRAFT    Emit the report. No GitHub mutation in this step.
- [ ] 7. CONFIRM  Present numbered findings. Act only on the numbers chosen.
- [ ] 8. ACT      Post only the confirmed subset. Re-read state first.
- [ ] 9. REPORT   State what was posted, what stayed open, what was set aside.
```

Step 6 and step 8 are separated on purpose. A draft you did not approve is a
comment a stranger has to argue with.

## Output Format

```markdown
## Verdict
REVIEW (N findings) or CLOSE (N items) — one line, plus the deciding row.

## Findings
**[SEVERITY]** one-line claim
`path/to/file.rs:42` — evidence: the tool output, type error, or test failure
Confidence: HIGH | MEDIUM — what would change my mind
Fix: the concrete change

## Considered And Set Aside
| Behaviour | Why set aside |
|---|---|

## Dismissed
Claims I raised and then disproved, with the file:line that disproved them.

## Proposed Closes
| # | Reason | Comment to post |
|---|---|---|

## Blocking
Items needing your decision before I act.
```

Every finding carries a `path:line` that appears in the diff. A finding without
one is not a finding.

## Roast Register

Critique the code, never the author. Severity drives intensity, and severity is
earned by evidence, not by volume.

- **CRITICAL** — security hole, data loss, broken public contract. Fix now.
- **MEDIUM** — real defect or a genuine unhandled path. Fix before merge.
- **LOW** — correct but worth knowing. No urgency.

Rules:

- Every roast is paired with a fix. A complaint with no remedy is noise.
- No praise that is not evidence-backed. "Looks good" without checking is banned.
- Never inflate a nitpick into a critical finding.
- Bound the volume: at most 3 findings per category, and if more exist, say how
  many you omitted. A 40-comment review gets ignored; a 5-comment review gets read.
- Strip mockery from the final text. Blunt is fine; sneering is not.

## Metrics

Post an event to `.agents/events/YYYY/MM/DD/` per the `metrics-reporter` skill
when you close items; set `human_interventions > 0` if the user corrected a
classification or dismissed a finding.

## Rationalizations

| Rationalization | Reality |
|---|---|
| "It's a small diff, so it can't break anything" | A 3-line change to an auth check breaks more than a 900-line refactor. Judge blast radius, not size. |
| "It's stale, so close it" | Staleness is not impact. An old PR touching public API still needs review. |
| "The test passes, so the test is good" | A test that asserts a mock was called passes and proves nothing. Check what it would catch. |
| "The title says `fix(security)`, so a vulnerability was fixed" | Check whether production code changed. A tests-only diff titled `fix(security)` is a finding in itself. |
| "The author is experienced, so skip the review" | Trust is not verification. Same evidence bar for everyone. |

## Red Flags

- [ ] Posting a comment, verdict, or close without explicit approval of that specific item
- [ ] Using `--approve` or `--request-changes` unprompted
- [ ] Deleting a PR or branch instead of closing it
- [ ] Closing a PR for age alone, with no impact analysis
- [ ] Flagging pre-existing issues in untouched lines
- [ ] Reporting a finding without a `path:line`, or repeating a comment the PR already has
- [ ] Accepting a claim because it sounds right, or because reviewers agreed
- [ ] Using `--reason` on `gh pr close` (it does not exist) instead of `--comment`
- [ ] Retrying a 422 inline-comment payload repeatedly instead of falling back to a body comment

## Related Skills

- `issue-triage` — batch issue implementation into a single PR
- `codacy` — static analysis findings and false-positive suppression
- `anti-ai-slop` — audit for boilerplate, over-abstraction, hollow docs
- `verify-actions` — confirm CI action SHAs before trusting a workflow change
