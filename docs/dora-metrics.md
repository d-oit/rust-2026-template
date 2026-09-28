# DORA Metrics

This document describes the DORA (DevOps Research and Assessment) metrics tracked in this repository and how they are measured.

## Failed Deployment Recovery Time (FDRT)

FDRT measures how long it takes to recover from a failed deployment — from the moment the failure is detected to the moment a working version is restored.

**How it works:**
1. `release.yml` runs a post-release smoke test automatically.
2. On failure, a GitHub Issue is created with label `release-failure`.
3. The issue `created_at` timestamp = failure detection time.
4. When the hotfix is deployed, close the issue; `closed_at` = recovery time.
5. FDRT = `closed_at - created_at` in hours.

**Target (DORA Elite):** < 1 hour
**Acceptable:** < 24 hours
**Requires improvement:** > 24 hours

## Change Lead Time

Change Lead Time measures the time it takes for a commit to get into production. In this repository, it is measured as the time from PR creation to PR merge into the `main` branch.

**How it works:**
1. The DORA report workflow (`.github/workflows/dora-report.yml`) computes lead time from PR metadata.
2. It calculates the difference between `merged_at` and `created_at`.
3. Results are captured in the weekly report snapshot history, `reports/dora-history.jsonl`, and rendered in the report's trend table. `dora-metrics.jsonl` holds deployment/failure/recovery events only — it is not report history.

**Target (DORA Elite):** < 24 hours

## Weekly Trend

`reports/dora-history.jsonl` stores one snapshot per report run: `generated_at`, `period_days`, and the four core
metric objects. A run replaces the snapshot of the current UTC ISO week and appends a snapshot for a new week, so
re-running in the same week updates that week's point instead of adding a duplicate. `reports/DORA-REPORT.md` renders
the latest three snapshots as its `Trend` table; each row is a rolling window ending on its report date, so adjacent
rows overlap rather than covering disjoint calendar periods.
