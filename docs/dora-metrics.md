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
1. `.github/workflows/dora-lead-time.yml` triggers on PR merge.
2. It calculates the difference between `merged_at` and `created_at`.
3. Results are recorded in `dora-metrics.jsonl`.

**Target (DORA Elite):** < 24 hours
