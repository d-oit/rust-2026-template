# DORA Report Skill

This skill allows AI agents to autonomously generate a DORA delivery performance report for the repository.

## Capabilities

The skill computes:
- **Deployment Frequency**: How often the team successfully releases to production.
- **Change Lead Time**: How long it takes for a commit to reach production.
- **Change Failure Rate**: The percentage of deployments that cause a failure in production.
- **Failed Deployment Recovery Time**: How long it takes to recover from a production failure.
- **Agentic Metrics**: Success rate and efficiency of AI agents working on the codebase.

## Data Sources

- **GitHub API**: For releases and pull request timestamps.
- **dora-metrics.jsonl**: Event log for change failure and recovery events; it is an input, not report history.
- **.agents/metrics.jsonl**: For agent activity and success metrics.
- **reports/dora-history.jsonl**: Weekly report snapshots (rolling windows ending on each report date) that back the trend table. The CLI replaces the snapshot of the current UTC ISO week and appends new weeks.

## Automation

A GitHub Actions workflow is configured to run this skill every Monday, ensuring the `reports/DORA-REPORT.md` is always current.
