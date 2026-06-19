#!/usr/bin/env python3
# .agents/skills-evaluation/scripts/generate_report.py

import json
import sys
import os
from datetime import datetime

def generate_report(iteration_dir):
    structure_file = os.path.join(iteration_dir, "structure_check.json")
    report_file = os.path.join(iteration_dir, "report.md")

    if not os.path.exists(structure_file):
        print(f"Error: {structure_file} not found")
        return

    with open(structure_file, 'r') as f:
        data = json.load(f)

    with open(report_file, 'w') as f:
        f.write("# Skill Evaluation Report\n\n")
        f.write(f"Date: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
        f.write(f"Iteration: {os.path.basename(iteration_dir)}\n\n")

        f.write("## Summary\n\n")
        total = len(data)
        passes = len([x for x in data if x['verdict'] == 'PASS'])
        needs_work = len([x for x in data if x['verdict'] == 'NEEDS_WORK'])
        fails = len([x for x in data if x['verdict'] == 'FAIL'])

        f.write(f"- Total Skills: {total}\n")
        f.write(f"- PASS: {passes}\n")
        f.write(f"- NEEDS_WORK: {needs_work}\n")
        f.write(f"- FAIL: {fails}\n\n")

        f.write("## Detailed Results\n\n")
        f.write("| Skill | Score | Verdict | Evals | Assertions |\n")
        f.write("|-------|-------|---------|-------|------------|\n")
        for item in sorted(data, key=lambda x: x['skill']):
            f.write(f"| {item['skill']} | {item['score']}/{item['max_score']} | {item['verdict']} | {item['eval_count']} | {item['assertion_count']} |\n")

    print(f"Report generated: {report_file}")

if __name__ == "__main__":
    iter_dir = sys.argv[1] if len(sys.argv) > 1 else ".agents/skills-evaluation/iterations/iteration-1"
    generate_report(iter_dir)
