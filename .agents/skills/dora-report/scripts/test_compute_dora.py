import os
import sys
import tempfile
import unittest
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

# Add script directory to sys.path
SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))

import compute_dora

TEMPLATE_PATH = SCRIPT_DIR.parent / "templates" / "DORA-REPORT.md.jinja"

# Mirrors the first committed snapshot in reports/dora-history.jsonl (from reports/dora-manifest.json).
SEEDED_HISTORY_RECORD = {
    "generated_at": "2026-09-28T07:43:42Z",
    "period_days": 30,
    "metrics": {
        "deployment_frequency": {"count": 0, "per_day": 0.0, "tier": "Low"},
        "change_lead_time": {"avg_hours": 21.35, "tier": "High"},
        "change_failure_rate": {"hotfixes": 0, "total": 0, "rate": 0, "tier": "N/A"},
        "failed_deployment_recovery_time": {"avg_hours": 0, "tier": "N/A"},
    },
}
SEEDED_HISTORY_LINE = json.dumps(SEEDED_HISTORY_RECORD, sort_keys=True) + "\n"


def snapshot_record(generated_at, per_day, avg_hours, rate, fdrt_hours):
    return {"generated_at": generated_at, "period_days": 30, "metrics": {
        "deployment_frequency": {"count": 1, "per_day": per_day, "tier": "Medium"},
        "change_lead_time": {"avg_hours": avg_hours, "tier": "High"},
        "change_failure_rate": {"hotfixes": 1, "total": 10, "rate": rate, "tier": "High"},
        "failed_deployment_recovery_time": {"avg_hours": fdrt_hours, "tier": "High"},
    }}


def write_history(path, records):
    path.write_text("".join(json.dumps(r, sort_keys=True) + "\n" for r in records))


def read_history(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


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

    def test_deployment_frequency_ignores_release_with_null_published(self):
        # `gh release list` emits `published: null` for drafts and for tags that
        # were never released. This crashed the scheduled DORA Report every week
        # from at least 2026-07-13: the attribute access on None raised
        # AttributeError, which the surrounding except (ValueError, KeyError) did
        # not cover, so the whole report aborted.
        now = datetime(2026, 9, 26, tzinfo=timezone.utc)
        releases = [
            {"tag": "v1", "published": None},
            {"tag": "v2", "published": "2026-09-20T00:00:00Z"},
            {"tag": "v3", "published": "2026-01-01T00:00:00Z"},
        ]

        result = compute_dora.deployment_frequency(releases, 30, now_dt=now, policy=None)

        # Only v2 is both non-null and inside the 30-day window.
        self.assertEqual(result["count"], 1)

    def test_deployment_frequency_tolerates_all_null_published(self):
        now = datetime(2026, 9, 26, tzinfo=timezone.utc)
        result = compute_dora.deployment_frequency(
            [{"tag": "v1", "published": None}], 30, now_dt=now, policy=None
        )
        self.assertEqual(result["count"], 0)

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

    def _run_compute_cli(self, out_path, now, history_path=None, releases=None, prs=None,
                         manifest_path=None):
        rel_path = self.dir_path / "releases.json"
        pr_path = self.dir_path / "prs.json"
        agent_path = self.dir_path / "agent.jsonl"
        dora_path = self.dir_path / "dora.jsonl"
        with open(rel_path, "w") as f:
            json.dump(releases if releases is not None else [], f)
        with open(pr_path, "w") as f:
            json.dump(prs if prs is not None else [], f)
        agent_path.touch()
        dora_path.touch()

        cmd = [
            sys.executable, str(SCRIPT_DIR / "compute_dora.py"),
            "--releases", str(rel_path), "--prs", str(pr_path),
            "--agent-metrics", str(agent_path), "--dora-metrics", str(dora_path),
            "--template", str(TEMPLATE_PATH), "--output", str(out_path),
            "--period-days", "30", "--repo", "d-oit/rust-2026-template", "--now", now,
        ]
        if history_path is not None:
            cmd += ["--history", str(history_path)]
        if manifest_path is not None:
            cmd += ["--manifest-output", str(manifest_path)]
        return subprocess.run(cmd, capture_output=True, text=True)

    def test_cli_without_history_renders_current_row_only(self):
        out_path = self.dir_path / "DORA-REPORT.md"
        res = self._run_compute_cli(out_path, now="2026-10-05T00:00:00Z")
        self.assertEqual(res.returncode, 0, f"compute_dora.py failed: {res.stderr}")

        content = out_path.read_text()
        self.assertIn("## Trend: Latest 3 Weekly Reports", content)
        self.assertIn("| 2026-10-05 | 30 | 0.0/day (0 releases) | N/A | N/A | N/A |", content)
        self.assertNotIn("<!--", content)
        # Without --history no snapshot file is created anywhere.
        self.assertEqual(list(self.dir_path.glob("*history*")), [])

    def test_cli_history_missing_file_starts_history_with_current_row(self):
        history_path = self.dir_path / "dora-history.jsonl"
        out_path = self.dir_path / "DORA-REPORT.md"
        res = self._run_compute_cli(out_path, now="2026-10-05T00:00:00Z", history_path=history_path)
        self.assertEqual(res.returncode, 0, f"compute_dora.py failed: {res.stderr}")

        records = read_history(history_path)
        self.assertEqual(len(records), 1)
        self.assertEqual(records[0]["generated_at"], "2026-10-05T00:00:00Z")
        self.assertEqual(records[0]["period_days"], 30)
        self.assertEqual(records[0]["metrics"], {
            "deployment_frequency": {"count": 0, "per_day": 0.0, "tier": "Low"},
            "change_lead_time": {"avg_hours": 0, "tier": "N/A"},
            "change_failure_rate": {"hotfixes": 0, "total": 0, "rate": 0, "tier": "N/A"},
            "failed_deployment_recovery_time": {"avg_hours": 0, "tier": "N/A"},
        })
        self.assertTrue(history_path.read_text().endswith("\n"))

        self.assertIn("| 2026-10-05 | 30 | 0.0/day (0 releases) | N/A | N/A | N/A |",
                      out_path.read_text())

    def test_cli_history_renders_older_and_current_rows_chronologically(self):
        history_path = self.dir_path / "dora-history.jsonl"
        history_path.write_text(SEEDED_HISTORY_LINE)
        out_path = self.dir_path / "DORA-REPORT.md"

        res = self._run_compute_cli(
            out_path,
            now="2026-10-05T00:00:00Z",
            history_path=history_path,
            releases=[
                {"tag": "v1.0.0", "published": "2026-10-01T00:00:00Z"},
                {"tag": "v1.1.0", "published": "2026-10-02T00:00:00Z"},
                {"tag": "v1.2.0", "published": "2026-10-03T00:00:00Z"},
            ],
            prs=[{"pr": 1, "created": "2026-10-04T00:00:00Z", "merged": "2026-10-04T00:30:00Z"}],
        )
        self.assertEqual(res.returncode, 0, f"compute_dora.py failed: {res.stderr}")

        records = read_history(history_path)
        self.assertEqual([r["generated_at"] for r in records],
                         [SEEDED_HISTORY_RECORD["generated_at"], "2026-10-05T00:00:00Z"])
        self.assertEqual(records[0], SEEDED_HISTORY_RECORD)
        self.assertEqual(records[1]["metrics"]["deployment_frequency"],
                         {"count": 3, "per_day": 0.1, "tier": "Medium"})
        self.assertEqual(records[1]["metrics"]["change_lead_time"],
                         {"avg_hours": 0.5, "tier": "Elite"})

        trend_rows = [l for l in out_path.read_text().splitlines() if l.startswith("| 2026-")]
        self.assertEqual(trend_rows, [
            "| 2026-09-28 | 30 | 0.0/day (0 releases) | 21.35 h | N/A | N/A |",
            "| 2026-10-05 | 30 | 0.1/day (3 releases) | 0.5 h | N/A | N/A |",
        ])

    def test_cli_history_same_iso_week_replaces_instead_of_appending(self):
        history_path = self.dir_path / "dora-history.jsonl"
        out_path = self.dir_path / "DORA-REPORT.md"

        first = self._run_compute_cli(out_path, now="2026-10-05T00:00:00Z", history_path=history_path)
        self.assertEqual(first.returncode, 0, f"compute_dora.py failed: {first.stderr}")
        self.assertEqual(len(read_history(history_path)), 1)

        second = self._run_compute_cli(
            out_path,
            now="2026-10-05T00:00:00Z",
            history_path=history_path,
            prs=[{"pr": 7, "created": "2026-10-04T00:00:00Z", "merged": "2026-10-04T00:30:00Z"}],
        )
        self.assertEqual(second.returncode, 0, f"compute_dora.py failed: {second.stderr}")

        records = read_history(history_path)
        self.assertEqual(len(records), 1)
        self.assertEqual(records[0]["generated_at"], "2026-10-05T00:00:00Z")
        self.assertEqual(records[0]["metrics"]["change_lead_time"], {"avg_hours": 0.5, "tier": "Elite"})

        trend_rows = [line for line in out_path.read_text().splitlines() if line.startswith("| 2026-")]
        self.assertEqual(trend_rows, ["| 2026-10-05 | 30 | 0.0/day (0 releases) | 0.5 h | N/A | N/A |"])

    def test_cli_history_keeps_all_weeks_but_renders_latest_three(self):
        history_path = self.dir_path / "dora-history.jsonl"
        seeded = [
            snapshot_record("2026-09-14T00:00:00Z", 0.011, 10.0, 0.1, 4.0),
            snapshot_record("2026-09-21T00:00:00Z", 0.022, 11.0, 0.2, 5.0),
            snapshot_record("2026-09-28T00:00:00Z", 0.033, 12.0, 0.3, 6.0),
        ]
        write_history(history_path, seeded)
        out_path = self.dir_path / "DORA-REPORT.md"

        res = self._run_compute_cli(out_path, now="2026-10-05T00:00:00Z", history_path=history_path)
        self.assertEqual(res.returncode, 0, f"compute_dora.py failed: {res.stderr}")

        records = read_history(history_path)
        self.assertEqual(len(records), 4)
        self.assertEqual([r["generated_at"] for r in records],
                         ["2026-09-14T00:00:00Z", "2026-09-21T00:00:00Z",
                          "2026-09-28T00:00:00Z", "2026-10-05T00:00:00Z"])
        self.assertEqual(records[0], seeded[0])

        content = out_path.read_text()
        trend_rows = [line for line in content.splitlines() if line.startswith("| 2026-")]
        self.assertEqual(trend_rows, [
            "| 2026-09-21 | 30 | 0.022/day (1 releases) | 11.0 h | 20.0% (1/10) | 5.0 h |",
            "| 2026-09-28 | 30 | 0.033/day (1 releases) | 12.0 h | 30.0% (1/10) | 6.0 h |",
            "| 2026-10-05 | 30 | 0.0/day (0 releases) | N/A | N/A | N/A |",
        ])
        self.assertNotIn("2026-09-14", content)

    def test_cli_history_malformed_line_fails_without_touching_files(self):
        history_path = self.dir_path / "dora-history.jsonl"
        original = SEEDED_HISTORY_LINE + "not-json\n"
        history_path.write_text(original)
        out_path = self.dir_path / "DORA-REPORT.md"
        manifest_path = self.dir_path / "dora-manifest.json"

        res = self._run_compute_cli(
            out_path,
            now="2026-10-05T00:00:00Z",
            history_path=history_path,
            manifest_path=manifest_path,
        )
        self.assertNotEqual(res.returncode, 0)
        self.assertIn(str(history_path), res.stderr)
        self.assertIn("line 2", res.stderr)
        self.assertEqual(history_path.read_text(), original)
        self.assertFalse(out_path.exists())
        self.assertFalse(manifest_path.exists())

    def test_cli_history_missing_required_fields_fails(self):
        history_path = self.dir_path / "dora-history.jsonl"
        incomplete = json.dumps({"generated_at": "2026-09-28T07:43:42Z", "period_days": 30}) + "\n"
        history_path.write_text(incomplete)
        out_path = self.dir_path / "DORA-REPORT.md"

        res = self._run_compute_cli(out_path, now="2026-10-05T00:00:00Z", history_path=history_path)
        self.assertNotEqual(res.returncode, 0)
        self.assertIn(str(history_path), res.stderr)
        self.assertIn("line 1", res.stderr)
        self.assertEqual(history_path.read_text(), incomplete)
        self.assertFalse(out_path.exists())

if __name__ == "__main__":
    unittest.main()
