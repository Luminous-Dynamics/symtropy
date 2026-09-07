#!/usr/bin/env python3
"""Read-only GitHub Actions runner-allocation report.

Uses only Python stdlib and GitHub REST APIs. Workflow-level status can hide
sibling jobs in a different state, so this report inspects individual jobs,
runner assignment, age, and runner-pool labels across queued and in-progress
workflow runs.

The report paginates active workflow runs and job lists before applying an
optional reporting limit, so old stranded jobs are not silently hidden by a
newer first page.

Examples:
  GITHUB_TOKEN=... python3 scripts/ci-runner-allocation-report.py
  python3 scripts/ci-runner-allocation-report.py --limit 50 --stale-after-hours 24
  python3 scripts/ci-runner-allocation-report.py --json
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import urllib.parse
import urllib.request
from collections import defaultdict
from dataclasses import asdict, dataclass

UTC = dt.timezone.utc
ACTIVE_WORKFLOW_STATES = ("queued", "in_progress")
PER_PAGE = 100
MAX_ACTIVE_RUNS = 1000


@dataclass(frozen=True)
class JobRow:
    run_id: int
    run_number: int | None
    workflow: str
    workflow_status: str
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


def paged_items(
    base_url: str,
    item_key: str,
    token: str | None,
    *,
    max_items: int | None = None,
) -> list[dict]:
    items: list[dict] = []
    page = 1
    while True:
        separator = "&" if "?" in base_url else "?"
        url = f"{base_url}{separator}{urllib.parse.urlencode({'per_page': PER_PAGE, 'page': page})}"
        payload = api_get(url, token)
        batch = payload.get(item_key, [])
        if not batch:
            break
        items.extend(batch)
        if max_items is not None and len(items) >= max_items:
            return items[:max_items]
        if len(batch) < PER_PAGE:
            break
        page += 1
    return items


def fetch_active_runs(owner: str, name: str, token: str | None) -> list[dict]:
    by_id: dict[int, dict] = {}
    for status in ACTIVE_WORKFLOW_STATES:
        query = urllib.parse.urlencode({"status": status})
        runs = paged_items(
            f"https://api.github.com/repos/{owner}/{name}/actions/runs?{query}",
            "workflow_runs",
            token,
            max_items=MAX_ACTIVE_RUNS,
        )
        for run in runs:
            by_id[int(run["id"])] = run
    if len(by_id) >= MAX_ACTIVE_RUNS:
        raise SystemExit(
            f"active workflow population reached safety cap {MAX_ACTIVE_RUNS}; "
            "raise MAX_ACTIVE_RUNS before treating this report as complete"
        )
    return sorted(
        by_id.values(),
        key=lambda run: str(run.get("created_at", "")),
    )


def fetch_jobs(owner: str, name: str, run_id: int, token: str | None) -> list[dict]:
    return paged_items(
        f"https://api.github.com/repos/{owner}/{name}/actions/runs/{run_id}/jobs",
        "jobs",
        token,
    )


def pool_name(labels: list[str]) -> str:
    for label in labels:
        if label.endswith("-latest") or label == "ubuntu-slim":
            return label
    return ",".join(labels) if labels else "unlabeled"


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Inspect active GitHub Actions jobs by runner pool."
    )
    parser.add_argument(
        "--repo",
        default=os.getenv("GITHUB_REPOSITORY", "Luminous-Dynamics/symtropy"),
        help="owner/repo (defaults to GITHUB_REPOSITORY or this repository)",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=0,
        help=(
            "Maximum oldest active workflow runs to inspect after full pagination; "
            "0 means all active runs (default: 0)"
        ),
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
    if args.limit < 0:
        raise SystemExit("--limit must be >= 0")
    if args.stale_after_hours < 0:
        raise SystemExit("--stale-after-hours must be >= 0")

    owner, name = args.repo.split("/", 1)
    token = os.getenv("GITHUB_TOKEN")
    now = dt.datetime.now(UTC)
    all_active_runs = fetch_active_runs(owner, name, token)
    runs = all_active_runs[: args.limit] if args.limit else all_active_runs

    rows: list[JobRow] = []
    for run in runs:
        created = parse_time(run.get("created_at"))
        age_hours = ((now - created).total_seconds() / 3600.0) if created else 0.0
        jobs = fetch_jobs(owner, name, int(run["id"]), token)
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
                    workflow_status=str(run.get("status", "")),
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

    run_states: dict[int, dict[str, int]] = {}
    pool_stats: dict[str, dict[str, float | int]] = defaultdict(
        lambda: {"jobs": 0, "unassigned": 0, "stale": 0, "oldest_unassigned_hours": 0.0}
    )
    for row in rows:
        state = run_states.setdefault(
            row.run_id, {"unassigned": 0, "completed": 0, "other": 0}
        )
        unassigned = row.status == "queued" and row.runner_id == 0
        if unassigned:
            state["unassigned"] += 1
        elif row.status == "completed":
            state["completed"] += 1
        else:
            state["other"] += 1

        pool = pool_name(row.labels)
        stats = pool_stats[pool]
        stats["jobs"] = int(stats["jobs"]) + 1
        if unassigned:
            stats["unassigned"] = int(stats["unassigned"]) + 1
            stats["oldest_unassigned_hours"] = max(
                float(stats["oldest_unassigned_hours"]), row.age_hours
            )
        if row.stale:
            stats["stale"] = int(stats["stale"]) + 1

    summary = {
        "repository": args.repo,
        "observed_at": now.isoformat(),
        "active_workflows_total": len(all_active_runs),
        "active_workflows_inspected": len(runs),
        "workflow_states": list(ACTIVE_WORKFLOW_STATES),
        "jobs_inspected": len(rows),
        "zero_step_unassigned_jobs": sum(
            1 for row in rows if row.status == "queued" and row.runner_id == 0
        ),
        "stale_zero_step_jobs": sum(1 for row in rows if row.stale),
        "stale_after_hours": args.stale_after_hours,
        "partially_drained_workflows": sum(
            1
            for state in run_states.values()
            if state["unassigned"] > 0 and (state["completed"] > 0 or state["other"] > 0)
        ),
        "pools": dict(sorted(pool_stats.items())),
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
        "repo={repository} observed_at={observed_at} active_total={active_workflows_total} "
        "active_inspected={active_workflows_inspected} jobs={jobs_inspected} "
        "unassigned={zero_step_unassigned_jobs} stale={stale_zero_step_jobs} "
        "partial={partially_drained_workflows}".format(**summary)
    )
    for pool, stats in summary["pools"].items():
        print(
            f"pool={pool} jobs={stats['jobs']} unassigned={stats['unassigned']} "
            f"stale={stats['stale']} oldest_unassigned_h={stats['oldest_unassigned_hours']:.2f}"
        )

    print(
        "run/job".ljust(24),
        "age(h)".rjust(7),
        "runner".ljust(13),
        "status".ljust(11),
        "pool/job",
    )
    for row in sorted(rows, key=lambda item: (-item.age_hours, item.run_id, item.job_id)):
        runner = str(row.runner_id) if row.runner_id else "-"
        marker = " STALE" if row.stale else ""
        pool = pool_name(row.labels)
        print(
            f"{row.run_id}/{row.job_id}".ljust(24),
            f"{row.age_hours:7.2f}",
            runner.ljust(13),
            row.status.ljust(11),
            f"{pool} :: {row.job_name}{marker}",
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
