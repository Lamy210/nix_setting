"""Offline CLI regression checks for GitHub Actions timing summary."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "ci" / "report-workflow-timings.py"


def job(job_id, name="macos-stable (macos-15, /Applications/Xcode_26.3.app/Contents/Developer)", conclusion="success"):
    return {
        "id": job_id,
        "name": name,
        "status": "completed",
        "conclusion": conclusion,
        "created_at": "2026-10-22T15:00:00Z",
        "started_at": "2026-10-22T15:01:00Z",
        "completed_at": "2026-10-22T15:03:00Z",
        "steps": [{"name": "build macos home-manager", "conclusion": "success", "started_at": "2026-10-22T15:01:20Z", "completed_at": "2026-10-22T15:02:30Z"}],
    }


class TimingReportTests(unittest.TestCase):
    def call(self, *groups):
        with tempfile.TemporaryDirectory() as root:
            paths = []
            for idx, group in enumerate(groups):
                path = Path(root) / f"{idx}.json"
                path.write_text(json.dumps({"jobs": group}), encoding="utf-8")
                paths.append(str(path))
            return subprocess.run([sys.executable, str(SCRIPT), *paths], text=True, capture_output=True, check=False)

    def test_success_duration_and_home_manager_step(self):
        result = self.call([job(1)])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("| 1 | 0 | 0 | 0 | 1m 00s / 1m 00s | 2m 00s / 2m 00s | 1m 10s / 1m 10s |", result.stdout)

    def test_cancelled_pending_failed_excluded(self):
        cancelled = job(1, conclusion="cancelled")
        pending = job(2)
        pending["status"] = "queued"
        pending["conclusion"] = None
        failed = job(3, conclusion="failure")
        result = self.call([cancelled, pending, failed])
        self.assertEqual(result.returncode, 0)
        self.assertIn("| 0 | 1 | 1 | 1 | — / —", result.stdout)

    def test_other_jobs_ignored_and_duplicates_deduplicated(self):
        result = self.call([job(1), job(9, name="rust-check")], [job(1)])
        self.assertEqual(result.returncode, 0)
        self.assertIn("| 1 | 0 | 0 | 0 |", result.stdout)
        self.assertNotIn("rust-check", result.stdout)

    def test_release_artifact_has_no_home_manager(self):
        run = job(4, name="release-artifact-check")
        run["steps"] = []
        result = self.call([run])
        self.assertEqual(result.returncode, 0)
        self.assertIn("release-artifact-check", result.stdout)
        self.assertIn("| — / — |", result.stdout)

    def test_negative_duration_fails_closed(self):
        run = job(1)
        run["started_at"] = "2026-10-22T14:59:00Z"
        result = self.call([run])
        self.assertEqual(result.returncode, 2)
        self.assertIn("negative", result.stderr)

    def test_missing_date_fails_closed(self):
        run = job(1)
        del run["started_at"]
        result = self.call([run])
        self.assertEqual(result.returncode, 2)
        self.assertIn("missing", result.stderr)

    def test_invalid_job_document_fails_closed(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "bad.json"
            path.write_text('[]', encoding='utf-8')
            result = subprocess.run([sys.executable, str(SCRIPT), str(path)], text=True, capture_output=True, check=False)
            self.assertEqual(result.returncode, 2)
            self.assertIn("expected GitHub jobs", result.stderr)


if __name__ == "__main__":
    unittest.main()
