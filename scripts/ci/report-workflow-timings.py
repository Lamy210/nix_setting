#!/usr/bin/env python3
"""Summarize GitHub Actions macOS job timing without changing CI gates.

Input: saved JSON from `gh api repos/OWNER/REPO/actions/runs/RUN_ID/jobs?per_page=100`.
For accurate comparisons, supply jobs from matched workflow configurations.
"""

import argparse
from collections import defaultdict
from datetime import datetime
import json
import math
from pathlib import Path
import sys


def seconds_between(start, end, context):
    if not isinstance(start, str) or not isinstance(end, str):
        raise ValueError(f"{context}: missing or invalid timestamps")
    try:
        left = datetime.fromisoformat(start.replace("Z", "+00:00"))
        right = datetime.fromisoformat(end.replace("Z", "+00:00"))
        delta = (right - left).total_seconds()
    except (ValueError, TypeError) as exc:
        raise ValueError(f"{context}: invalid timestamps: {exc}") from exc
    if not math.isfinite(delta) or delta < 0:
        raise ValueError(f"{context}: negative or non-finite duration")
    return delta


def nearest_rank(values, percentile):
    if not values:
        return None
    values = sorted(values)
    return values[max(0, math.ceil(len(values) * percentile / 100) - 1)]


def pretty(seconds):
    if seconds is None:
        return "—"
    value = round(seconds)
    return f"{value // 60}m {value % 60:02d}s"


def target(name):
    return name.startswith("macos-stable (") or name == "release-artifact-check"


def report(files):
    by_name = defaultdict(lambda: {"lag": [], "runtime": [], "home": [], "pending": 0, "cancelled": 0, "other": 0})
    seen = set()
    for filename in files:
        obj = json.loads(Path(filename).read_text(encoding="utf-8"))
        if not isinstance(obj, dict) or not isinstance(obj.get("jobs"), list):
            raise ValueError(f"{filename}: expected GitHub jobs response object with a jobs array")
        for job in obj["jobs"]:
            if not isinstance(job, dict) or not isinstance(job.get("name"), str):
                raise ValueError(f"{filename}: invalid job record")
            if not target(job["name"]):
                continue
            job_id = job.get("id")
            if not isinstance(job_id, int) or isinstance(job_id, bool):
                raise ValueError(f"{filename}: job missing integer id")
            if job_id in seen:
                continue  # Same API page imported twice must not skew percentiles.
            seen.add(job_id)
            row = by_name[job["name"]]
            if job.get("status") != "completed":
                row["pending"] += 1
                continue
            if job.get("conclusion") == "cancelled":
                row["cancelled"] += 1
                continue
            if job.get("conclusion") != "success":
                row["other"] += 1
                continue
            context = f"job {job_id}"
            row["lag"].append(seconds_between(job.get("created_at"), job.get("started_at"), context))
            row["runtime"].append(seconds_between(job.get("started_at"), job.get("completed_at"), context))
            for step in job.get("steps", []):
                if step.get("name") == "build macos home-manager" and step.get("conclusion") == "success":
                    row["home"].append(seconds_between(step.get("started_at"), step.get("completed_at"), f"{context} home-manager"))
                    break
    lines = [
        "| Job | Successful n | Pending | Cancelled | Other conclusions | Created→started p50 / p95 | Started→completed p50 / p95 | Home Manager p50 / p95 |",
        "| --- | ---: | ---: | ---: | ---: | --- | --- | --- |",
    ]
    for name, row in sorted(by_name.items()):
        fmt = lambda values: f"{pretty(nearest_rank(values, 50))} / {pretty(nearest_rank(values, 95))}"
        lines.append(f"| {name} | {len(row['lag'])} | {row['pending']} | {row['cancelled']} | {row['other']} | {fmt(row['lag'])} | {fmt(row['runtime'])} | {fmt(row['home'])} |")
    lines.extend([
        "",
        "Times are from GitHub's job/step timestamps. Created→started includes scheduling and runner acquisition; it does not prove runner queueing alone.",
        "Percentiles use nearest-rank; only successful completed jobs contribute to timing distributions. Cancelled/failed/pending jobs are counted separately.",
        "Samples with n<10 are exploratory. Compare equivalent workflows, OS/Xcode configurations, run attempts and cache states; no cache hit rates are inferred.",
    ])
    return "\n".join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("jobs_json", nargs="+", type=Path, help="saved Actions run jobs JSON (one file per run/page)")
    args = parser.parse_args(argv)
    try:
        print(report(args.jobs_json))
    except (ValueError, OSError, json.JSONDecodeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
