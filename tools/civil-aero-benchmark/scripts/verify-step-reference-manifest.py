#!/usr/bin/env python3
"""Validate only the reference metadata for the external STEP suite; no source files are imported."""
from __future__ import annotations

import json
import re
import sys
from collections import Counter
from pathlib import Path

EXPECTED = [
    ("STEP1", "DC2-DTLZ3", 2, 2),
    ("STEP2", "MW7", 2, 2),
    ("STEP3", "MW11", 2, 2),
    ("STEP4", "MW11", 2, 3),
    ("STEP5", "DC2-DTLZ1", 2, 3),
    ("STEP6", "CF4", 2, 3),
    ("STEP7", "C2-DTLZ2", 2, 5),
    ("STEP8", "RCM2", 2, 5),
    ("STEP9", "DC2-DTLZ1", 2, 5),
    ("STEP10", "C2-DTLZ2", 2, 10),
    ("STEP11", "CF4", 2, 10),
    ("STEP12", "MW8", 2, 10),
    ("STEP13", "DC2-DTLZ1", 2, 30),
    ("STEP14", "ZXHCF16", 2, 30),
    ("STEP15", "MW11", 2, 30),
    ("STEP16", "DC2-DTLZ3", 3, 2),
    ("STEP17", "DAS-CMOP8", 3, 2),
    ("STEP18", "DC1-DTLZ3", 3, 2),
    ("STEP19", "DC2-DTLZ3", 3, 3),
    ("STEP20", "C3-DTLZ4", 3, 3),
    ("STEP21", "DAS-CMOP8", 3, 3),
    ("STEP22", "DC2-DTLZ3", 3, 5),
    ("STEP23", "C3-DTLZ4", 3, 5),
    ("STEP24", "C1-DTLZ1", 3, 5),
    ("STEP25", "DC2-DTLZ1", 3, 10),
    ("STEP26", "MW8", 3, 10),
    ("STEP27", "C3-DTLZ4", 3, 10),
    ("STEP28", "DC2-DTLZ3", 3, 30),
    ("STEP29", "DAS-CMOP9", 3, 30),
    ("STEP30", "CF8", 3, 30),
]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def main() -> int:
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("data/cmo_step_reference_manifest.v0.1.json")
    require(len(sys.argv) <= 2, "usage: verify-step-reference-manifest.py [manifest.json]")
    manifest = json.loads(path.read_text(encoding="utf-8"))

    require(
        manifest.get("schema") == "civil-aero-cmo-step-reference-manifest/0.1.0",
        "unexpected manifest schema",
    )
    require(
        manifest.get("status")
        == "reference_metadata_only_no_source_code_or_problem_data_vendored_no_step_rust_implementation_yet",
        "manifest evidence status was promoted",
    )
    external = manifest["external_repository"]
    require(external["repository"] == "jsi-ci/cmo", "unexpected external repository")
    require(external["commit"] == "9ba3f0649c87005dc7675b18698c2518659dc88b", "external commit changed")
    require(external["tree"] == "a0ccc5a94b4d6ffe0ffbffb2c1a5d2c559ef7252", "external tree changed")
    require(external["spdx_license"] == "GPL-3.0", "external license boundary changed")
    require(
        "do_not_copy_or_vendor" in external["vendoring_policy"],
        "missing explicit no-vendoring policy",
    )
    require(
        manifest["paper_run_data"]["status"]
        == "external_pointer_only_license_not_verified_bytes_not_retained",
        "external run data must remain pointer-only until terms review",
    )

    source_files = {row["path"]: row for row in manifest["reference_source_files"]}
    for required_path in (
        "LICENSE",
        "cmo/problems/step.py",
        "cmo/problems/factory.py",
        "cmo/problems/cdtlz.py",
        "cmo/problems/dcdtlz.py",
    ):
        require(required_path in source_files, f"missing pinned reference source {required_path}")
    for row in source_files.values():
        require(re.fullmatch(r"[0-9a-f]{40}", row["git_blob_sha1"]) is not None,
                f"invalid upstream Git blob id: {row['path']}")
        require(isinstance(row["byte_length"], int) and row["byte_length"] > 0,
                f"invalid upstream byte length: {row['path']}")

    tasks = manifest.get("tasks", [])
    require(len(tasks) == 30, f"expected 30 tasks, found {len(tasks)}")
    observed = []
    ids = set()
    for task in tasks:
        task_id = task["task_id"]
        require(task_id not in ids, f"duplicate task id {task_id}")
        ids.add(task_id)
        observed.append((task_id, task["problem"], task["objectives"], task["variables"]))
        expected_path = (
            f"cmo/problems/pf/STEP/{task_id.lower()}_M{task['objectives']}_D{task['variables']}.pf"
        )
        require(task["external_front_path"] == expected_path,
                f"{task_id}: reference-front path/dimensions disagree")
        require(re.fullmatch(r"[0-9a-f]{40}", task["external_front_git_blob_sha1"]) is not None,
                f"{task_id}: invalid reference-front Git blob id")
        require(task["external_front_byte_length"] > 0, f"{task_id}: invalid reference-front size")
        require(
            task["front_terms_status"] == "not_reviewed_external_pointer_only_not_downloaded",
            f"{task_id}: data terms/status was promoted",
        )
    require(observed == EXPECTED, "STEP task map/order/dimensions differ from pinned paper table")

    bins = Counter((task["objectives"], task["variables"]) for task in tasks)
    expected_bins = {(m, d): 3 for m in (2, 3) for d in (2, 3, 5, 10, 30)}
    require(dict(bins) == expected_bins, "expected exactly three tasks in each of ten dimension/objective bins")
    require(manifest["current_repository_coverage"]["step_tasks_implemented"] == 2,
            "expected exactly two clean-room formula pilots")
    require(manifest["current_repository_coverage"]["implemented_pilot_tasks"] == ["STEP7", "STEP24"],
            "unexpected pilot task set")
    require(manifest["current_repository_coverage"]["full_step_coverage"] is False,
            "full STEP coverage was claimed prematurely")

    print(
        "step_reference_manifest=pass tasks=30 bins=10 tasks_per_bin=3 "
        "source_commit_pinned=true gpl_vendoring=blocked front_data=external_pointer_only "
        "rust_formula_pilots=2 full_step_coverage=false"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"step_reference_manifest=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
