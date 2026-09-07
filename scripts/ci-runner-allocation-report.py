#!/usr/bin/env python3
"""Read-only GitHub Actions runner-allocation report.

Uses only Python stdlib and GitHub REST APIs. Workflow-level ``queued`` state can
hide sibling jobs that already ran, so this report inspects individual jobs,
runner assignment, age, and runner-pool labels.

Examples:
  GITHUB_TOKEN=... python3 scripts/ci-runner-allocation-report.py
  python3 scripts/ci-runner-allocation-report.py --limit 30 --stale-after-hours 24
  python3 scripts/ci-runner-allocation-report.py --json
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import urllib.parse
import urllib.request
from dataclasses import asdict, dataclass

UTC = dt.timezone.utc


@dataclass(frozen=True)
class JobRow:
    run_id: int
    run_number: int | None
    workflow: str
    branch: str
    job_id: int
    job_name: str
    labels: list[str]
    status: str
    conclusion: str | None
    runner_id: int
    runner_name: str
    created_at: str
    started_at: str | None
    age_hours: float
    stale: bool


def api_get(url: str, token: str | None) -> dict:
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "symtropy-ci-runner-allocation-report",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def parse_time(value: str | None) -> dt.datetime | None:
    if not value:
        return None
    return dt.datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(UTC)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Inspect queued GitHub Actions jobs by runner pool."
    )
    parser.add_argument(
        "--repo",
        default=os.getenv("GITHUB_REPOSITORY", "Luminous-Dynamics/symtropy"),
        help="owner/repo (defaults to GITHUB_REPOSITORY or this repository)",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=30,
        help="Maximum queued workflow runs to inspect (default: 30)",
    )
    parser.add_argument(
        "--stale-after-hours",
        type=float,
        default=24.0,
        help="Flag zero-step unassigned jobs older than this threshold (default: 24)",
    )
    parser.add_argument("--json", action="store_true", help="Emit JSON")
    args = parser.parse_args()

    if "/" not in args.repo:
        raise SystemExit("--repo must be in owner/repo form")
    if not 1 <= args.limit <= 100:
        raise SystemExit("--limit must be between 1 and 100")
    if args.stale_after_hours < 0:
        raise SystemExit("--stale-after-hours must be >= 0")

    owner, name = args.repo.split("/", 1)
    token = os.getenv("GITHUB_TOKEN")
    now = dt.datetime.now(UTC)
    query = urllib.parse.urlencode(
        {"status": "queued", "per_page": args.limit, "page": 1}
    )
    runs = api_get(
        f"https://api.github.com/repos/{owner}/{name}/actions/runs?{query}", token
    ).get("workflow_runs", [])

    rows: list[JobRow] = []
    for run in runs:
        created = parse_time(run.get("created_at"))
        age_hours = ((now - created).total_seconds() / 3600.0) if created else 0.0
        jobs = api_get(
            f"https://api.github.com/repos/{owner}/{name}/actions/runs/{run['id']}/jobs?per_page=100",
            token,
        ).get("jobs", [])
        for job in jobs:
            runner_id = int(job.get("runner_id") or 0)
            status = str(job.get("status", ""))
            stale = (
                status == "queued"
                and runner_id == 0
                and age_hours >= args.stale_after_hours
            )
            rows.append(
                JobRow(
                    run_id=int(run["id"]),
                    run_number=run.get("run_number"),
                    workflow=str(run.get("name", "")),
                    branch=str(run.get("head_branch", "")),
                    job_id=int(job["id"]),
                    job_name=str(job.get("name", "")),
                    labels=[str(label) for label in job.get("labels", [])],
                    status=status,
                    conclusion=job.get("conclusion"),
                    runner_id=runner_id,
                    runner_name=str(job.get("runner_name") or ""),
                    created_at=str(run.get("created_at", "")),
                    started_at=job.get("started_at"),
                    age_hours=round(age_hours, 2),
                    stale=stale,
                )
            )

    partial_runs: dict[int, dict[str, int]] = {}
    for row in rows:
        state = partial_runs.setdefault(
            row.run_id, {"unassigned": 0, "completed": 0, "other": 0}
        )
        if row.status == "queued" and row.runner_id == 0:
            state["unassigned"] += 1
        elif row.status == "completed":
            state["completed"] += 1
        else:
            state["other"] += 1

    summary = {
        "repository": args.repo,
        "observed_at": now.isoformat(),
        "queued_workflows_inspected": len(runs),
        "jobs_inspected": len(rows),
        "zero_step_unassigned_jobs": sum(
            1 for row in rows if row.status == "queued" and row.runner_id == 0
        ),
        "stale_zero_step_jobs": sum(1 for row in rows if row.stale),
        "stale_after_hours": args.stale_after_hours,
        "partially_drained_workflows": sum(
            1
            for state in partial_runs.values()
            if state["unassigned"] > 0 and (state["completed"] > 0 or state["other"] > 0)
        ),
    }

    if args.json:
        print(
            json.dumps(
                {"summary": summary, "jobs": [asdict(row) for row in rows]},
                indent=2,
                sort_keys=True,
            )
        )
        return 0

    print(
        "repo={repository} observed_at={observed_at} queued_workflows={queued_workflows_inspected} "
        "jobs={jobs_inspected} unassigned={zero_step_unassigned_jobs} stale={stale_zero_step_jobs} "
        "partial={partially_drained_workflows}".format(**summary)
    )
    print("run/job".ljust(24), "age(h)".rjust(7), "runner".ljust(13), "status".ljust(10), "pool/job")
    for row in sorted(rows, key=lambda item: (-item.age_hours, item.run_id, item.job_id)):
        runner = str(row.runner_id) if row.runner_id else "-"
        marker = " STALE" if row.stale else ""
        pool = ",".join(row.labels) if row.labels else "-"
        print(
            f"{row.run_id}/{row.job_id}".ljust(24),
            f"{row.age_hours:7.2f}",
            runner.ljust(13),
            row.status.ljust(10),
            f"{pool} :: {row.job_name}{marker}",
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
