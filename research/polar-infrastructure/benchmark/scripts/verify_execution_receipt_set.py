#!/usr/bin/env python3
"""Verify a complete, fair set of controller execution receipts.

Usage: verify_execution_receipt_set.py PAIRED_INPUT.json RECEIPT_1.json ...
Receipts may be not-executed design templates for contract tests, but those rows
remain ineligible for performance scoring. This is a comparison-integrity gate,
not a simulator or physical-model validator.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from verify_execution_receipt import _verify_receipt_validated
from verify_paired_comparison_v0_4 import validate_input as validate_paired_input

SHARED_GLOBAL_ROLES = {
    "plant_model", "executable", "runtime_environment", "observation_policy", "resource_policy",
}
SHARED_PER_TRAJECTORY_ROLES = {"run_configuration"}


def need(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def inventory_by_role(receipt: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {item["role"]: item for item in receipt["artifact_inventory"]}


def verify_receipt_set(paired: dict[str, Any], receipts: list[dict[str, Any]]) -> None:
    # Validate the v0.4 pair input once, including rehashing every trajectory,
    # initial-state, and controller payload. Reuse indexes for every receipt
    # instead of repeating full-fixture validation N times.
    validate_paired_input(paired)
    expected_cases = {case["case_id"]: case for case in paired.get("cases", [])}
    controllers = set(paired.get("controllers", []))
    trajectories = {artifact["trajectory_id"]: artifact for artifact in paired.get("trajectory_artifacts", [])}
    controllers_by_id = {artifact["controller_id"]: artifact for artifact in paired.get("controller_artifacts", [])}
    need(bool(expected_cases) and controllers and trajectories, "paired input declarations are incomplete")
    need(len(receipts) == len(expected_cases), "receipt set must contain one receipt per declared controller/trajectory case")

    seen_case_ids: set[str] = set()
    seen_pairs: set[tuple[str, str]] = set()
    controller_artifact_by_controller: dict[str, str] = {}
    trajectory_artifact_by_id: dict[str, tuple[str, str, str]] = {}
    resource_limits_by_case: dict[str, dict[str, int]] = {}
    global_role_digests: dict[str, str] = {}
    trajectory_role_digests: dict[tuple[str, str], str] = {}

    for receipt in receipts:
        _verify_receipt_validated(
            receipt,
            paired,
            paired_case_by_id=expected_cases,
            trajectory_artifacts_by_id=trajectories,
            controller_artifacts_by_id=controllers_by_id,
        )
        case_id = receipt["case_id"]
        need(case_id in expected_cases, f"receipt references undeclared paired case: {case_id}")
        need(case_id not in seen_case_ids, f"duplicate receipt for case: {case_id}")
        seen_case_ids.add(case_id)

        case = expected_cases[case_id]
        controller_id = receipt["controller_id"]
        trajectory_id = case["trajectory_id"]
        need(controller_id in controllers and case["controller_id"] == controller_id,
             f"receipt controller does not match declared case {case_id}")
        pair = (controller_id, trajectory_id)
        need(pair not in seen_pairs, f"duplicate controller/trajectory receipt: {pair}")
        seen_pairs.add(pair)

        binding = receipt["paired_input_bindings"]
        trajectory_signature = (
            binding["trajectory_sha256"],
            binding["initial_state_id"],
            binding["initial_state_sha256"],
        )
        previous_signature = trajectory_artifact_by_id.get(trajectory_id)
        need(previous_signature is None or previous_signature == trajectory_signature,
             f"paired arms disagree on trajectory or initial-state bytes: {trajectory_id}")
        trajectory_artifact_by_id[trajectory_id] = trajectory_signature

        previous_controller_digest = controller_artifact_by_controller.get(controller_id)
        current_controller_digest = binding["controller_artifact_sha256"]
        need(previous_controller_digest is None or previous_controller_digest == current_controller_digest,
             f"controller artifact changes between trajectories: {controller_id}")
        controller_artifact_by_controller[controller_id] = current_controller_digest

        resource_limits_by_case[case_id] = receipt["resource_limits"]
        roles = inventory_by_role(receipt)
        need(roles["controller_artifact"]["sha256"] == binding["controller_artifact_sha256"],
             f"receipt controller artifact inventory is not bound to its paired input: {controller_id}")
        for role in SHARED_GLOBAL_ROLES:
            digest = roles[role]["sha256"]
            previous = global_role_digests.get(role)
            need(previous is None or previous == digest,
                 f"comparison arms do not share the same {role} artifact digest")
            global_role_digests[role] = digest
        for role in SHARED_PER_TRAJECTORY_ROLES:
            key = (trajectory_id, role)
            digest = roles[role]["sha256"]
            previous = trajectory_role_digests.get(key)
            need(previous is None or previous == digest,
                 f"paired arms do not share the same {role} for trajectory {trajectory_id}")
            trajectory_role_digests[key] = digest

    need(seen_case_ids == set(expected_cases), "receipt set is missing one or more declared cases")
    expected_pairs = {(case["controller_id"], case["trajectory_id"]) for case in expected_cases.values()}
    need(seen_pairs == expected_pairs, "receipt set does not cover the complete controller/trajectory matrix")
    limit_signatures = {json.dumps(limits, sort_keys=True) for limits in resource_limits_by_case.values()}
    need(len(limit_signatures) == 1,
         "comparison arms have unequal numeric resource limits; matched resource budgets are mandatory")
    need(set(controller_artifact_by_controller) == controllers,
         "receipt set lacks one or more controller artifact identities")
    need(set(trajectory_artifact_by_id) == set(trajectories),
         "receipt set lacks one or more declared trajectory identities")


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print("usage: verify_execution_receipt_set.py PAIRED_INPUT.json RECEIPT_1.json [RECEIPT_2.json ...]",
              file=sys.stderr)
        return 2
    try:
        paired = json.loads(Path(argv[1]).read_text())
        receipts = [json.loads(Path(path).read_text()) for path in argv[2:]]
        verify_receipt_set(paired, receipts)
    except (OSError, json.JSONDecodeError, ValueError, KeyError, TypeError) as exc:
        print(f"execution receipt set verification failed: {exc}", file=sys.stderr)
        return 1
    score_eligible = sum(bool(r.get("scoring_eligible")) for r in receipts)
    print(f"complete receipt set verified; receipts={len(receipts)}; scoring_eligible={score_eligible}; physical validity not inferred")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
