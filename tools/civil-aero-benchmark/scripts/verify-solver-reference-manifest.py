#!/usr/bin/env python3
"""Fail-closed validator for immutable SU2 source-reference pins (not run receipts)."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

EXPECTED = {
    "v8.0.1": ("8.0.1", "8ef4b1be045122b2fdb485bfb5fe4eecd1bc4246",
               "historical_participant_reproduction"),
    "v8.6.0": ("8.6.0", "7048fd35f56d4bf52e586429d6746a36a10255f8",
               "current_release_comparison"),
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-solver-reference-manifest.py manifest.json", file=sys.stderr)
        return 2
    data = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    require(data.get("schema") == "civil-aero-solver-reference-manifest/0.1.0",
            "unexpected solver-reference schema")
    require(data.get("repository") == "su2code/SU2", "unexpected solver repository")
    require(
        data.get("manifest_status")
        == "immutable_source_tag_pins_only_not_built_not_run_not_validated",
        "manifest must not imply that a solver has been built, run or validated",
    )
    references = data.get("references")
    require(isinstance(references, list) and len(references) == 2,
            "exactly two explicitly separate reference lanes are required")
    actual_tags = {row.get("tag"): row for row in references}
    require(set(actual_tags) == set(EXPECTED), "historical/current SU2 tags are incomplete or duplicated")
    for tag, (version, sha, role) in EXPECTED.items():
        row = actual_tags[tag]
        require(row.get("version") == version, f"{tag}: version/tag disagreement")
        require(row.get("resolved_commit_sha") == sha, f"{tag}: immutable commit pin changed")
        require(re.fullmatch(r"[0-9a-f]{40}", row.get("resolved_commit_sha", "")) is not None,
                f"{tag}: commit SHA must be full 40-character lowercase hex")
        require(row.get("comparison_role") == role, f"{tag}: comparison role mismatch")
        require(row.get("tag_ref") == f"refs/tags/{tag}", f"{tag}: tag ref mismatch")
        require(row.get("execution_status") == "not_built_not_run",
                f"{tag}: source pin must not claim execution")
        require(row.get("validation_status") == "not_validated",
                f"{tag}: source pin must not claim validation")
        require(row.get("upstream_release_url") == f"https://github.com/su2code/SU2/releases/tag/{tag}",
                f"{tag}: release URL mismatch")
    controls = data.get("comparison_controls", {})
    for field in (
        "historical_and_current_lanes_must_remain_separate",
        "release_tag_is_not_a_build_digest",
        "source_pin_is_not_execution_attestation",
        "release_notes_are_not_case_validation",
    ):
        require(controls.get(field) is True, f"comparison control {field} must be explicit")
    required = controls.get("required_run_receipt_fields")
    require(isinstance(required, list) and len(required) >= 10,
            "run-receipt fields must cover build, environment, geometry, mesh and outputs")
    require(len(required) == len(set(required)), "run-receipt fields must be unique")
    print("solver_reference_manifest=pass references=2 historical=8.0.1 current=8.6.0 execution=none validation=none")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"solver_reference_manifest=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
