import json
import sys

def evaluate(data):
    event_name = data.get("event_name", "")
    changes_result = data.get("changes_result", "")
    changes = data.get("changes", {})
    jobs = data.get("jobs", {})

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
