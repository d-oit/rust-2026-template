import os
import sys
import tempfile
import unittest
import json
import subprocess
from pathlib import Path

# Add script directory to sys.path
SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))

import compute_dora

TEMPLATE_PATH = SCRIPT_DIR.parent / "templates" / "DORA-REPORT.md.jinja"

class TestComputeDora(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.dir_path = Path(self.temp_dir.name)

    def tearDown(self):
        self.temp_dir.cleanup()

    def test_load_json_valid(self):
        file_path = self.dir_path / "valid.json"
        with open(file_path, "w") as f:
            json.dump([{"a": 1}], f)

        data = compute_dora.load_json(file_path, required=True)
        self.assertEqual(data, [{"a": 1}])

    def test_load_json_malformed_required_raises(self):
        file_path = self.dir_path / "malformed.json"
        with open(file_path, "w") as f:
            f.write('[{"tag": "v1.0.0"}] [{"tag": "v1.0.1"}]')  # Concatenated JSON arrays (historical failure mode)

        with self.assertRaises(ValueError) as cm:
            compute_dora.load_json(file_path, required=True)
        self.assertIn("Failed to parse required JSON file", str(cm.exception))
        self.assertIn(str(file_path), str(cm.exception))

    def test_load_json_non_list_required_raises(self):
        file_path = self.dir_path / "dict.json"
        with open(file_path, "w") as f:
            json.dump({"error": "Not Found"}, f)

        with self.assertRaises(ValueError) as cm:
            compute_dora.load_json(file_path, required=True)
        self.assertIn("must contain a JSON array", str(cm.exception))

    def test_load_json_missing_required_raises(self):
        file_path = self.dir_path / "nonexistent.json"
        with self.assertRaises(FileNotFoundError) as cm:
            compute_dora.load_json(file_path, required=True)
        self.assertIn("Required input JSON file not found", str(cm.exception))

    def test_load_json_missing_optional_returns_empty(self):
        file_path = self.dir_path / "optional_missing.json"
        data = compute_dora.load_json(file_path, required=False)
        self.assertEqual(data, [])

    def test_load_jsonl_graceful(self):
        file_path = self.dir_path / "metrics.jsonl"
        with open(file_path, "w") as f:
            f.write('{"metric": "deployment"}\ninvalid json line\n{"metric": "change_failure"}\n')

        data = compute_dora.load_jsonl(file_path)
        self.assertEqual(len(data), 2)
        self.assertEqual(data[0]["metric"], "deployment")
        self.assertEqual(data[1]["metric"], "change_failure")

    def test_multi_page_paginated_input(self):
        # Fixture representing multi-page GitHub API output combined into a single array
        releases_data = [
            {"tag": "v1.0.0", "published": "2026-09-01T10:00:00Z"},
            {"tag": "v1.1.0", "published": "2026-09-10T12:00:00Z"},
            {"tag": "v1.2.0", "published": "2026-09-15T14:00:00Z"}
        ]
        prs_data = [
            {"pr": 1, "created": "2026-09-01T08:00:00Z", "merged": "2026-09-01T10:00:00Z"}, # 2h
            {"pr": 2, "created": "2026-09-09T10:00:00Z", "merged": "2026-09-10T10:00:00Z"}  # 24h
        ]

        rel_path = self.dir_path / "releases.json"
        pr_path = self.dir_path / "prs.json"
        agent_path = self.dir_path / "agent.jsonl"
        dora_path = self.dir_path / "dora.jsonl"
        out_path = self.dir_path / "DORA-REPORT.md"

        with open(rel_path, "w") as f:
            json.dump(releases_data, f)
        with open(pr_path, "w") as f:
            json.dump(prs_data, f)
        agent_path.touch()
        dora_path.touch()

        # Run compute_dora CLI with fixed reference timestamp
        now = "2026-09-17T00:00:00Z"
        cmd = [
            sys.executable,
            str(SCRIPT_DIR / "compute_dora.py"),
            "--releases", str(rel_path),
            "--prs", str(pr_path),
            "--agent-metrics", str(agent_path),
            "--dora-metrics", str(dora_path),
            "--template", str(TEMPLATE_PATH),
            "--output", str(out_path),
            "--period-days", "30",
            "--repo", "d-oit/rust-2026-template",
            "--now", now
        ]

        res = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(res.returncode, 0, f"compute_dora.py failed: {res.stderr}")

        content = out_path.read_text()
        self.assertIn("**Generated:** 2026-09-17T00:00:00Z", content)
        self.assertIn("Deployment Frequency | 0.1/day (3 releases) | **Medium**", content)
        self.assertIn("Change Lead Time | 13.0 hours avg | **High**", content)

    def test_cli_fails_on_malformed_input(self):
        rel_path = self.dir_path / "releases_malformed.json"
        pr_path = self.dir_path / "prs.json"
        agent_path = self.dir_path / "agent.jsonl"
        dora_path = self.dir_path / "dora.jsonl"
        out_path = self.dir_path / "DORA-REPORT.md"

        # Write concatenated JSON arrays representing paginated output without jq -s
        with open(rel_path, "w") as f:
            f.write('[{"tag": "v1.0.0"}] [{"tag": "v1.1.0"}]')
        with open(pr_path, "w") as f:
            json.dump([], f)
        agent_path.touch()
        dora_path.touch()

        cmd = [
            sys.executable,
            str(SCRIPT_DIR / "compute_dora.py"),
            "--releases", str(rel_path),
            "--prs", str(pr_path),
            "--agent-metrics", str(agent_path),
            "--dora-metrics", str(dora_path),
            "--template", str(TEMPLATE_PATH),
            "--output", str(out_path)
        ]

        res = subprocess.run(cmd, capture_output=True, text=True)
        self.assertNotEqual(res.returncode, 0)
        self.assertIn("Failed to parse required JSON file", res.stderr)
        self.assertIn(str(rel_path), res.stderr)

    def test_deterministic_byte_identical_snapshots_with_pinned_now(self):
        releases_data = [
            {"tag": "v1.0.0", "published": "2026-09-01T10:00:00Z"},
            {"tag": "v1.1.0", "published": "2026-09-10T12:00:00Z"}
        ]
        prs_data = [
            {"pr": 1, "created": "2026-09-01T08:00:00Z", "merged": "2026-09-01T10:00:00Z"}
        ]
        policy_data = {
            "version": "1.0",
            "period_days": 30,
            "bot_allowlist": ["dependabot[bot]"],
            "merge_strategy": "squash_rebase_merge",
            "percentile_method": "mean",
            "revert_predicate": "title_contains_revert"
        }

        rel_path = self.dir_path / "releases.json"
        pr_path = self.dir_path / "prs.json"
        agent_path = self.dir_path / "agent.jsonl"
        dora_path = self.dir_path / "dora.jsonl"
        policy_path = self.dir_path / "policy.json"

        out_path1 = self.dir_path / "run1" / "DORA-REPORT.md"
        manifest_path1 = self.dir_path / "run1" / "dora-manifest.json"
        out_path2 = self.dir_path / "run2" / "DORA-REPORT.md"
        manifest_path2 = self.dir_path / "run2" / "dora-manifest.json"

        with open(rel_path, "w") as f:
            json.dump(releases_data, f)
        with open(pr_path, "w") as f:
            json.dump(prs_data, f)
        with open(policy_path, "w") as f:
            json.dump(policy_data, f)
        agent_path.touch()
        dora_path.touch()

        now = "2026-09-17T00:00:00Z"

        def run_cli(out_p, manifest_p):
            cmd = [
                sys.executable,
                str(SCRIPT_DIR / "compute_dora.py"),
                "--releases", str(rel_path),
                "--prs", str(pr_path),
                "--agent-metrics", str(agent_path),
                "--dora-metrics", str(dora_path),
                "--policy", str(policy_path),
                "--template", str(TEMPLATE_PATH),
                "--output", str(out_p),
                "--manifest-output", str(manifest_p),
                "--repo", "d-oit/rust-2026-template",
                "--now", now
            ]
            res = subprocess.run(cmd, capture_output=True, text=True)
            self.assertEqual(res.returncode, 0, f"compute_dora.py failed: {res.stderr}")

        run_cli(out_path1, manifest_path1)
        run_cli(out_path2, manifest_path2)

        # Verify reports and manifests are byte-identical across runs
        self.assertEqual(out_path1.read_bytes(), out_path2.read_bytes())
        self.assertEqual(manifest_path1.read_bytes(), manifest_path2.read_bytes())

    def test_derivation_manifest_contents(self):
        rel_path = self.dir_path / "releases.json"
        pr_path = self.dir_path / "prs.json"
        agent_path = self.dir_path / "agent.jsonl"
        dora_path = self.dir_path / "dora.jsonl"
        policy_path = self.dir_path / "policy.json"
        out_path = self.dir_path / "DORA-REPORT.md"
        manifest_path = self.dir_path / "dora-manifest.json"

        with open(rel_path, "w") as f:
            json.dump([{"tag": "v1.0.0", "published": "2026-09-01T10:00:00Z"}], f)
        with open(pr_path, "w") as f:
            json.dump([], f)
        with open(policy_path, "w") as f:
            json.dump({"version": "1.0", "period_days": 30, "bot_allowlist": ["renovate[bot]"]}, f)
        agent_path.touch()
        dora_path.touch()

        now = "2026-09-17T00:00:00Z"
        cmd = [
            sys.executable,
            str(SCRIPT_DIR / "compute_dora.py"),
            "--releases", str(rel_path),
            "--prs", str(pr_path),
            "--agent-metrics", str(agent_path),
            "--dora-metrics", str(dora_path),
            "--policy", str(policy_path),
            "--template", str(TEMPLATE_PATH),
            "--output", str(out_path),
            "--manifest-output", str(manifest_path),
            "--now", now
        ]
        res = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(res.returncode, 0)

        with open(manifest_path, "r") as f:
            manifest = json.load(f)

        self.assertEqual(manifest["schema_version"], "1.0")
        self.assertEqual(manifest["evaluation_window"]["now_iso"], "2026-09-17T00:00:00Z")
        self.assertEqual(manifest["evaluation_window"]["start_iso"], "2026-08-18T00:00:00Z")
        self.assertEqual(manifest["scanned_counts"]["releases"], 1)
        self.assertEqual(manifest["policy"]["bot_allowlist"], ["renovate[bot]"])
        self.assertIn("releases", manifest["input_hashes"])
        self.assertIn("policy", manifest["input_hashes"])

if __name__ == "__main__":
    unittest.main()
