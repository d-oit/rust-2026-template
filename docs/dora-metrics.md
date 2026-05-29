---
# DORA Metrics

This repository tracks key DevOps Research and Assessment (DORA) metrics to
measure software delivery performance and stability.

## Change Lead Time

**Definition:** The time it takes for a commit to get into production.
**Tracking:** Measured from PR creation to merge into the `main` branch.
**Workflow:** `.github/workflows/dora-lead-time.yml`

## Change Failure Rate (CFR)

**Definition:** The percentage of deployments or releases that cause a failure
in production and require immediate remediation (e.g., hotfix, rollback, or
patch).

### How We Track CFR

1. **Hotfix Detection:**
   - PRs labeled with `hotfix` or `rework` are tracked as remediation events.
   - Branches prefixed with `hotfix/` trigger automatic tracking.
   - The `.github/workflows/hotfix.yml` workflow records these events.

2. **Hotfix Releases:**
   - Releases triggered from hotfix branches or containing "hotfix",
     "regression", or "critical" in the last commit message are marked as
     hotfix releases.
   - These releases are annotated in GitHub Release notes.

3. **Metrics Storage:**
   - Events are appended to `dora-metrics.jsonl` and uploaded as workflow
     artifacts.
   - Each record includes the date, metric type (`deployment` or
     `change_failure`), and relevant metadata.

### Calculation

**CFR = (Total Hotfix Releases) / (Total Releases)**

A "Hotfix Release" is any release marked with `metric: change_failure` and
`type: hotfix` in the metrics log.

## Measurement and AI Impact

Following the **2025 DORA AI Report**, we actively monitor these metrics to
ensure that AI-assisted contributions (labeled `agentic`) maintain or improve
stability while increasing velocity. High velocity (low Lead Time) must be
balanced with low instability (low CFR).
