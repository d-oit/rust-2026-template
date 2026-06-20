#!/usr/bin/env python3
# .agents/skills-evaluation/scripts/grade_benchmark.py

import json
import sys
import os

def grade_skill(skill_dir, responses_file):
    evals_json = os.path.join(skill_dir, "evals/evals.json")
    if not os.path.exists(evals_json):
        return {"error": f"No evals.json found in {skill_dir}"}

    if not os.path.exists(responses_file):
        return {"error": f"No responses file found: {responses_file}"}

    with open(evals_json, 'r') as f:
        evals_data = json.load(f)

    with open(responses_file, 'r') as f:
        responses_data = json.load(f)

    results = []

    # Simple keyword-based grading for demonstration/baseline
    # In a real scenario, this would call an LLM grader.
    for eval_case in evals_data.get('evals', []):
        case_id = eval_case.get('id')
        response = responses_data.get(str(case_id), "")

        case_results = {
            "id": case_id,
            "prompt": eval_case.get('prompt'),
            "assertions": []
        }

        passed_count = 0
        for assertion in eval_case.get('assertions', []):
            # Case-insensitive substring match as a fallback
            passed = assertion.lower() in response.lower()
            case_results["assertions"].append({
                "assertion": assertion,
                "passed": passed,
                "evidence": "Substring match found" if passed else "No substring match"
            })
            if passed:
                passed_count += 1

        case_results["score"] = f"{passed_count}/{len(eval_case.get('assertions', []))}"
        case_results["status"] = "PASS" if passed_count == len(eval_case.get('assertions', [])) else "FAIL"
        results.append(case_results)

    return results

def main():
    if len(sys.argv) < 3:
        print("Usage: grade_benchmark.py <skill_dir> <responses_json>")
        sys.exit(1)

    skill_dir = sys.argv[1]
    responses_file = sys.argv[2]

    results = grade_skill(skill_dir, responses_file)
    print(json.dumps(results, indent=2))

if __name__ == "__main__":
    main()
