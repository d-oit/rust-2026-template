import json
import sys

def evaluate(data):
    event_name = data.get("event_name", "")
    changes_result = data.get("changes_result", "")
    changes = data.get("changes", {})
    jobs = data.get("jobs", {})
    maintenance = data.get("maintenance") or {}

    if changes_result != "success":
        return False, [
            f"Changes detection job did not succeed (result: '{changes_result}'). Cannot determine required job status."
        ]

    code_changed = str(changes.get("code", "")).lower() == "true"
    docs_changed = str(changes.get("docs", "")).lower() == "true"
    heavy_changed = str(changes.get("heavy", "")).lower() == "true"
    agents_changed = str(changes.get("agents", "")).lower() == "true"
    workflows_changed = str(changes.get("workflows", "")).lower() == "true"
    is_pr = (event_name == "pull_request")

    # The generated-maintenance fast path replaces the full pull-request tier
    # with the allowlisted validators. Accept it only when the classifier
    # explicitly granted it; anything ambiguous keeps the normal contract
    # (issue #503 - a fast path must never be inferred from a missing payload).
    # Job outputs arrive as strings, so normalise before trusting them.
    granted = str(maintenance.get("maintenance_only", "")).strip().lower()
    maintenance_only = granted == "true"
    maintenance_source = str(maintenance.get("source", "")).strip()
    if maintenance_only and maintenance_source not in ("classifier",):
        return False, [
            f"Maintenance classification came from an untrusted source "
            f"('{maintenance_source or '<missing>'}'); only the file-set classifier "
            "may grant the fast path."
        ]

    if maintenance_only and not is_pr:
        return False, [
            "Maintenance fast path was requested for a non-PR event "
            f"('{event_name}'); the fast path only applies to pull requests."
        ]

    if maintenance_only and (code_changed or agents_changed or workflows_changed or heavy_changed):
        return False, [
            "Maintenance fast path was granted for a PR that also changed code, "
            "agents, workflows, or heavy paths. The classifier must not allow this; "
            "falling back to the full contract."
        ]

    job_configs = {
        "validate-agents": {
            "required_in_pr": agents_changed or workflows_changed,
            "hint": "Agent entrypoints/manifest validation failed or skipped unexpectedly (run `cargo run -p xtask -- agents validate`).",
        },
        "lint": {
            "required_in_pr": code_changed or agents_changed or workflows_changed,
            "hint": "Linting failed or skipped unexpectedly (run `yamllint .` and `npx markdownlint-cli2`).",
        },
        "bench": {
            "required_in_pr": heavy_changed or workflows_changed,
            "hint": "Benchmarks failed or skipped unexpectedly (run `cargo bench --workspace`).",
        },
        "version-check": {
            "required_in_pr": code_changed or agents_changed or workflows_changed,
            "hint": "VERSION, llms.txt, or SKILLS.md check failed or skipped unexpectedly.",
        },
        "quality-gate": {
            "required_in_pr": code_changed or docs_changed or workflows_changed,
            "hint": "Quality gate failed or skipped unexpectedly (run `cargo run -p xtask -- quality run`).",
        },
    }

    failures = []
    for job_name, config in job_configs.items():
        if job_name not in jobs:
            failures.append(f"Job '{job_name}' result is missing from job results payload.")
            continue

        res = jobs[job_name]
        if res == "success":
            continue

        if res == "skipped":
            if not is_pr:
                failures.append(
                    f"Job '{job_name}' was skipped on non-PR event '{event_name}'. "
                    f"All jobs must run on push or manual dispatch. -> {config['hint']}"
                )
            elif config["required_in_pr"]:
                failures.append(
                    f"Job '{job_name}' was skipped unexpectedly on PR event. "
                    f"Changes detected: code={code_changed}, heavy={heavy_changed}, "
                    f"agents={agents_changed}, workflows={workflows_changed}. "
                    f"This job was required by the path filter contract. -> {config['hint']}"
                )
            else:
                continue
        else:
            failures.append(f"Job '{job_name}' ended with status '{res}'. -> {config['hint']}")

    # A maintenance-only PR is required to run the fast-path validators inside the
    # required quality gate, which the loop above already enforces. Nothing else
    # may be skipped differently, so no extra exemptions are granted here.
    return (len(failures) == 0), failures

def main():
    input_file = sys.argv[1] if len(sys.argv) > 1 and sys.argv[1] != "-" else None
    try:
        if input_file:
            with open(input_file, "r", encoding="utf-8") as f:
                data = json.load(f)
        else:
            data = json.load(sys.stdin)
    except Exception as e:
        print(f"::error::Failed to parse evaluation JSON payload: {e}")
        sys.exit(1)

    ok, failures = evaluate(data)
    if not ok:
        print("::error::CI required job evaluation failed:")
        for f in failures:
            print(f"  - {f}")
        sys.exit(1)

    print("All required CI jobs passed or were legitimately skipped per path-filter contract.")

if __name__ == "__main__":
    main()
