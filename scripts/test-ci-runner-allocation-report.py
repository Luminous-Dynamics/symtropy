#!/usr/bin/env python3
from __future__ import annotations

import datetime as dt
import importlib.util
import pathlib
import unittest

SCRIPT = pathlib.Path(__file__).with_name("ci-runner-allocation-report.py")
SPEC = importlib.util.spec_from_file_location("ci_runner_allocation_report", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"unable to load {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

UTC = dt.timezone.utc


class AllocationWaitTests(unittest.TestCase):
    def test_expired_unassigned_attempt_uses_completion_time(self) -> None:
        job = {
            "created_at": "2026-09-07T14:40:56Z",
            "started_at": "2026-09-07T14:40:56Z",
            "completed_at": "2026-09-08T14:40:57Z",
            "runner_id": 0,
        }
        now = dt.datetime(2026, 9, 8, 18, 33, tzinfo=UTC)
        self.assertAlmostEqual(MODULE.allocation_wait_hours(job, now), 24.0002777778)

    def test_fresh_rerun_uses_new_job_created_at(self) -> None:
        job = {
            "created_at": "2026-09-08T15:14:27Z",
            "started_at": "2026-09-08T15:14:27Z",
            "completed_at": None,
            "runner_id": 0,
        }
        now = dt.datetime(2026, 9, 8, 18, 33, tzinfo=UTC)
        self.assertAlmostEqual(MODULE.allocation_wait_hours(job, now), 3.3091666667)

    def test_assigned_job_measures_time_to_runner_start(self) -> None:
        job = {
            "created_at": "2026-09-07T14:40:56Z",
            "started_at": "2026-09-07T18:41:46Z",
            "completed_at": "2026-09-07T18:44:47Z",
            "runner_id": 1000080235,
        }
        now = dt.datetime(2026, 9, 8, 18, 33, tzinfo=UTC)
        self.assertAlmostEqual(MODULE.allocation_wait_hours(job, now), 4.0138888889)


if __name__ == "__main__":
    unittest.main()
