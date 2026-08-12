## Release Health Check Failed

**Release:** ${RELEASE_TAG}
**Failed at:** ${FAILED_AT}
**Workflow run:** ${RUN_URL}

## Required Actions
1. Identify the regression (check workflow logs above)
2. Create a `hotfix/*` branch and fix the issue
3. Record recovery time by closing this issue with a comment: `Recovered at: YYYY-MM-DDTHH:MM:SSZ`

## DORA FDRT Tracking
Time to detect: automatically recorded above.
Time to recover: close this issue when the hotfix is deployed.
