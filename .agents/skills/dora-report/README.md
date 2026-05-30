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
- **dora-metrics.jsonl**: For change failure and recovery events.
- **.agents/metrics.jsonl**: For agent activity and success metrics.

## Automation

A GitHub Actions workflow is configured to run this skill on the first Monday of every month, ensuring the `DORA-REPORT.md` in the root directory is always current.
