---
name: dora-report
description: Generate a DORA-REPORT.md for this repository by computing the five core DORA software delivery performance metrics and three DORA agentic metrics from available data sources. Use this when asked to generate a DORA report or assess delivery performance.
category: metrics
license: MIT
metadata:
  author: d-oit
  version: "1.0"
---

# Skill: dora-report

## When to Use

- User asks for this skill's functionality


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
- `dora-metrics.jsonl` exists

## Steps

### 1. Fetch Deployment Frequency data from GitHub API

```bash
gh api "/repos/{owner}/{repo}/releases" --paginate \
  --jq '[.[] | {tag: .tag_name, published: .published_at}]' \
  > /tmp/releases.json
```

### 2. Fetch PR Lead Time data

```bash
gh api "/repos/{owner}/{repo}/pulls?state=closed&base=main" --paginate \
  --jq '[.[] | select(.merged_at != null) | {pr: .number, created: .created_at, merged: .merged_at}]' \
  > /tmp/prs.json
```

### 3. Run the compute script

```bash
python3 .agents/skills/dora-report/scripts/compute_dora.py \
  --releases /tmp/releases.json \
  --prs /tmp/prs.json \
  --agent-metrics .agents/metrics.jsonl \
  --dora-metrics dora-metrics.jsonl \
  --template .agents/skills/dora-report/templates/DORA-REPORT.md.jinja \
  --output reports/DORA-REPORT.md \
  --period-days 30 \
  --repo $(git remote get-url origin | sed 's/.*github.com[:\/]\(.*\)\.git/\1/')
```

### 4. Review and commit the report

```bash
git add reports/DORA-REPORT.md
git commit -m "docs(dora): update reports/DORA-REPORT.md [skip ci]"
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

## Red Flags

- [ ] Running DORA report less than monthly (trends need data points)
- [ ] Ignoring Lead Time for Changes (the hardest metric to improve)
- [ ] Not tracking human_interventions in metrics (hides true agent performance)
