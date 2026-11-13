---
name: dora-report
description: Generate a DORA-REPORT.md for this repository by computing the five core DORA software delivery performance metrics and three DORA agentic metrics from available data sources. Use this when asked to generate a DORA report or assess delivery performance.

license: MIT
metadata:
  category: metrics
  author: d-oit
  version: "1.0"
---

# Skill: dora-report

## Purpose
Generate a `DORA-REPORT.md` for this repository by computing the five core DORA software
delivery performance metrics and three DORA agentic metrics from available data sources.

## When to use
- When asked to generate a DORA report or assess delivery performance
- During sprint reviews or quarterly assessments
- When evaluating the ROI of AI tooling adoption
- **Run this skill monthly** to track trends

## Prerequisites
- GitHub CLI (`gh`) installed and authenticated
- Python 3.9+ available
- `.agents/metrics.jsonl` exists
- `dora-metrics.jsonl` exists (event log: deployment, change-failure and recovery events)
- `reports/dora-history.jsonl` exists (weekly report snapshots for the trend table; created on first run when missing)

## Steps

### 1. Fetch Deployment Frequency data from GitHub API

```bash
# Note: Combine paginated GitHub API JSON arrays using jq -s 'add // []' to avoid concatenated JSON files
# See historical failure case dora-report-gen-20260914 in .agents/ci/regression-matrix.json
gh api "/repos/{owner}/{repo}/releases" --paginate \
  --jq '[.[] | {tag: .tag_name, published: .published_at}]' \
  | jq -s 'add // []' \
  > /tmp/releases.json
```

### 2. Fetch PR Lead Time data

```bash
gh api "/repos/{owner}/{repo}/pulls?state=closed&base=main" --paginate \
  --jq '[.[] | select(.merged_at != null) | {pr: .number, created: .created_at, merged: .merged_at}]' \
  | jq -s 'add // []' \
  > /tmp/prs.json
```

### 3. Run the compute script

```bash
python3 .agents/skills/dora-report/scripts/compute_dora.py \
  --releases /tmp/releases.json \
  --prs /tmp/prs.json \
  --agent-metrics .agents/metrics.jsonl \
  --dora-metrics dora-metrics.jsonl \
  --policy plans/dora.json \
  --template .agents/skills/dora-report/templates/DORA-REPORT.md.jinja \
  --output reports/DORA-REPORT.md \
  --manifest-output reports/dora-manifest.json \
  --history reports/dora-history.jsonl \
  --period-days 30 \
  --repo $(git remote get-url origin | sed 's/.*github.com[:\/]\(.*\)\.git/\1/')
```

`--dora-metrics` is the event log; `--history` is the report snapshot history. The script replaces any stored
snapshot from the same UTC ISO week with the current one, appends snapshots from new weeks, and renders the
latest three snapshots (rolling windows ending on each report date) as the report's trend table.

### 4. Validate and commit the report

Generated artifacts are validated offline by a contract check that reuses the
generator's own schema. Run it before committing; CI runs the same script through
the required quality gate:

```bash
python3 scripts/validate-dora-artifacts.py
```

It fails closed on malformed manifest JSON, malformed history JSONL, missing metric
sections, invalid timestamps, malformed input hashes, and — most importantly — drift
between the report, the manifest, and the latest history entry for the same
generation. Never update one artifact without regenerating all three.

```bash
git add reports/DORA-REPORT.md reports/dora-manifest.json reports/dora-history.jsonl
# No commit-message CI skip: the maintenance fast path keeps real checks cheap,
# while a skipped required workflow would leave the PR blocked.
git commit -m "docs(dora): update reports/DORA-REPORT.md"
```

### 5. Log the report generation in metrics.jsonl

Append to `.agents/metrics.jsonl`:

```bash
echo '{"timestamp":"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'","agent":"claude","skill":"dora-report","task_description":"Generated monthly DORA report","success":true,"human_interventions":0}' >> .agents/metrics.jsonl
```

## Success Criteria
- `reports/DORA-REPORT.md` is generated with up-to-date metrics.
- All four core DORA metrics and three agentic metrics are populated.
- The report is committed to the repository.
- The activity is logged in `.agents/metrics.jsonl`.

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "We shipped a lot of features, so DORA must be good." | DORA measures flow efficiency, not volume. Ship fast but measure lead time. |
| "Our MTTR is low because we roll back quickly." | Rollback isn't recovery. Recovery means the fix is deployed and working. |
| "Manual deployment is fine for our team size." | Manual deployment is the #1 blocker for Deployment Frequency improvement. |
| "I'll add `[skip ci]` so this report PR merges faster." | A skipped required workflow leaves `Quality Gate (pull-request)` pending forever, which blocks merge. The generated-maintenance fast path is the supported way to keep this PR cheap. |
| "Only the report changed, so I can edit the manifest by hand." | The validator cross-checks report, manifest, and history. Regenerate all three with the script; hand-editing one artifact fails the gate. |

## Red Flags

- [ ] Running DORA report less than monthly (trends need data points)
- [ ] Ignoring Lead Time for Changes (the hardest metric to improve)
- [ ] Not tracking human_interventions in metrics (hides true agent performance)
- [ ] Committing generated artifacts without running `scripts/validate-dora-artifacts.py`
- [ ] Adding `[skip ci]` or path filters to make the report PR "fast"

## GH CLI Usage

Use `gh run watch` to gate DORA recovery recording on CI passing:

```bash
# Find latest CI run for a branch
RUN_ID=$(gh run list --branch hotfix/my-fix --workflow ci.yml \
  --limit 1 --json databaseId --jq '.[0].databaseId')

# Block until CI completes; exits non-zero if CI fails
gh run watch "$RUN_ID" --exit-status
```

This ensures FDRT recovery is only counted after a verified green build.
