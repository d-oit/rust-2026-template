---
name: dora-report
description: Generate a DORA-REPORT.md for this repository by computing the four core DORA software delivery performance metrics and the agentic metrics from available data sources. Use this when asked to generate a DORA report or assess delivery performance.

license: MIT
metadata:
  category: metrics
  author: d-oit
  version: "1.0"
---

# Skill: dora-report

## Purpose
Generate a `DORA-REPORT.md` for this repository by computing the four core DORA software
delivery performance metrics (Deployment Frequency, Change Lead Time, Change Failure Rate,
Failed Deployment Recovery Time) and the agentic metrics (task total, success rate, human
intervention rate, average tokens) from available data sources.

## When to use
- When asked to generate a DORA report or assess delivery performance
- During sprint reviews or quarterly assessments
- When evaluating the ROI of AI tooling adoption
- **Run this skill monthly** to track trends

## Prerequisites
- GitHub CLI (`gh`) installed and authenticated
- Python 3.10+ available (the workflow pins 3.10) and `jq` installed
- `.agents/metrics.jsonl` — agent activity log (legacy line format; `touch` it if missing,
  as `dora-report.yml` does; empty means agentic metrics report `0`)
- `dora-metrics.jsonl` — event log for Change Failure Rate / FDRT. Written by the
  `release.yml`, `hotfix.yml`, and `dora-fdrt.yml` workflows as artifacts; normally absent
  or empty in a fresh clone (`touch` it) — with no events, those two tiers report `N/A`.
- `reports/dora-history.jsonl` — weekly report snapshots for the trend table; created on
  first run when missing (an existing file is NOT required)

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
# Match the canonical invocation in .github/workflows/dora-report.yml:
# create missing optional inputs, then run the script.
mkdir -p reports
touch .agents/metrics.jsonl dora-metrics.jsonl

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
  --repo "$(git remote get-url origin | sed 's#.*github.com[:/]\(.*\)\.git#\1#')"
```

`--dora-metrics` is the event log; `--history` is the report snapshot history. The script replaces any stored
snapshot from the same UTC ISO week with the current one, appends snapshots from new weeks, and renders the
latest three snapshots (rolling windows ending on each report date) as the report's trend table.

### 4. Review and commit the report

```bash
git add reports/DORA-REPORT.md reports/dora-manifest.json reports/dora-history.jsonl
git commit -m "docs(dora): update reports/DORA-REPORT.md [skip ci]"
```

### 5. Log the report generation with metrics-reporter

Write an event file to `.agents/events/YYYY/MM/DD/` with `skill: "dora-report"`
per the `metrics-reporter` skill (the canonical Event File Format in
`.agents/ORCHESTRATION.md`). Do not append to the legacy `.agents/metrics.jsonl`.

## Success Criteria
- `reports/DORA-REPORT.md` is generated with up-to-date metrics.
- All four core DORA metrics and the agentic metrics are populated (agentic
  metrics show `0`/`N/A` when `.agents/metrics.jsonl` is empty).
- The report is committed (directly or via the `dora-report.yml` workflow PR).
- The activity is recorded as an event file under `.agents/events/`.

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "We shipped a lot of features, so DORA must be good." | DORA measures flow efficiency, not volume. Ship fast but measure lead time. |
| "Our MTTR is low because we roll back quickly." | Rollback isn't recovery. Recovery means the fix is deployed and working. |
| "Manual deployment is fine for our team size." | Manual deployment is the #1 blocker for Deployment Frequency improvement. |

## Red Flags

- [ ] Running DORA report less than monthly (trends need data points)
- [ ] Ignoring Lead Time for Changes (the hardest metric to improve)
- [ ] Not tracking human_interventions in metrics (hides true agent performance)

## GH CLI Usage

`dora-metrics.jsonl` is not hand-written: the workflows emit it as an artifact —
`hotfix.yml` (`metric: "change_failure"`, `type: "hotfix"`/`"rework"`),
`release.yml` (`metric: "deployment"` or `"change_failure"`), and `dora-fdrt.yml`
(`metric: "fdrt"`). This skill only passes the file through unchanged — never
rewrite existing event lines. CFR counts `change_failure` events against
`deployment` events; with neither present both tiers render `N/A`.

Use `gh run watch` when you need to gate a manual recovery record on CI passing:

```bash
# Find latest CI run for a branch
RUN_ID=$(gh run list --branch hotfix/my-fix --workflow ci.yml \
  --limit 1 --json databaseId --jq '.[0].databaseId')

# Block until CI completes; exits non-zero if CI fails
gh run watch "$RUN_ID" --exit-status
```

This ensures FDRT recovery is only counted after a verified green build (the
`hotfix.yml` workflow does this automatically for hotfix events).
