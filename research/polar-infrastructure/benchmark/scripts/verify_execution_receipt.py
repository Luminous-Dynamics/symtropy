#!/usr/bin/env python3
"""Independent execution-receipt verifier.

This checks payload-backed SHA-256 values and binds a receipt to the selected
case in the paired comparison fixture. It does not run a simulator or infer
physical validity from hashes.
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

SHA256_RE = re.compile(r"^[a-f0-9]{64}$")
REVISION_RE = re.compile(r"^([a-f0-9]{40}|[a-f0-9]{64})$")
REQUIRED_INPUT_ROLES = {
    "plant_model", "run_configuration", "executable",
    "runtime_environment", "observation_policy", "resource_policy", "command",
}
REQUIRED_OUTPUT_ROLES = {"output_trace", "stdout", "stderr"}
RESOURCE_KEYS = {
    "wall_time_ms", "cpu_time_ms", "peak_memory_bytes", "simulator_calls", "observation_count"
}


def need(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def is_sha(value: Any) -> bool:
    return isinstance(value, str) and SHA256_RE.fullmatch(value) is not None


def verify_artifact_bytes(artifact: dict[str, Any]) -> bool:
    need(is_sha(artifact.get("sha256")), f"invalid SHA-256 for artifact {artifact.get('artifact_id')}")
    payload = artifact.get("payload")
    uri = artifact.get("source_uri")
    need((isinstance(payload, str) and bool(payload) and uri is None)
         or (payload is None and isinstance(uri, str) and bool(uri)),
         "artifact must contain exactly one of a payload or source URI")
    if payload is None:
        return False
    actual = hashlib.sha256(payload.encode("utf-8")).hexdigest()
    need(actual == artifact["sha256"], f"payload SHA-256 mismatch for artifact {artifact['artifact_id']}")
    return True


def verify_receipt(receipt: dict[str, Any], paired: dict[str, Any]) -> None:
    need(receipt.get("schema_version") == "0.1.0"
         and receipt.get("artifact_kind") == "symtropy-controller-execution-receipt",
         "unsupported receipt header")
    need(receipt.get("claim_policy") == {
        "synthetic_payloads_are_execution_evidence": False,
        "scoring_requires_completed_execution": True,
        "scoring_requires_independent_output_hash_verification": True,
        "resource_limits_must_be_shared_across_comparison_arms": True,
    }, "receipt claim policy is missing or weakened")

    inventory = receipt.get("artifact_inventory")
    need(isinstance(inventory, list), "artifact_inventory must be an array")
    ids, roles = set(), set()
    verified_payload_count = 0
    for artifact in inventory:
        need(isinstance(artifact, dict), "artifact inventory entry must be an object")
        need(artifact.get("artifact_id") not in ids, "duplicate artifact inventory ID")
        need(artifact.get("role") not in roles, "duplicate artifact inventory role")
        ids.add(artifact.get("artifact_id"))
        roles.add(artifact.get("role"))
        verified_payload_count += int(verify_artifact_bytes(artifact))
    need(roles == REQUIRED_INPUT_ROLES, "artifact inventory must contain exactly the required input roles")

    bindings = receipt.get("paired_input_bindings", {})
    for key in ("trajectory_sha256", "initial_state_sha256", "controller_artifact_sha256"):
        need(is_sha(bindings.get(key)), f"invalid paired input binding: {key}")

    case = next((c for c in paired.get("cases", [])
                 if c.get("case_id") == receipt.get("case_id")), None)
    need(case is not None, "receipt case_id is absent from paired input fixture")
    need(case["controller_id"] == receipt["controller_id"], "receipt controller does not match paired case")
    need(case["trajectory_id"] == bindings.get("trajectory_id")
         and case["trajectory_sha256"] == bindings.get("trajectory_sha256")
         and case["initial_state_id"] == bindings.get("initial_state_id")
         and case["initial_state_sha256"] == bindings.get("initial_state_sha256")
         and case["controller_artifact_sha256"] == bindings.get("controller_artifact_sha256"),
         "receipt input bindings disagree with the paired case row")
    trajectory = next((a for a in paired.get("trajectory_artifacts", [])
                       if a.get("trajectory_id") == bindings.get("trajectory_id")), None)
    controller = next((a for a in paired.get("controller_artifacts", [])
                       if a.get("controller_id") == receipt["controller_id"]), None)
    need(trajectory is not None and controller is not None, "paired input artifact registry is incomplete")
    need(trajectory["trajectory_sha256"] == bindings["trajectory_sha256"]
         and trajectory["initial_state_sha256"] == bindings["initial_state_sha256"]
         and controller["controller_sha256"] == bindings["controller_artifact_sha256"],
         "receipt digest does not resolve to the paired input artifact registry")

    limits = receipt.get("resource_limits", {})
    observed = receipt.get("observed_resources", {})
    need(set(limits) == RESOURCE_KEYS and set(observed) == RESOURCE_KEYS,
         "resource limits/measurements have incorrect keys")
    need(all(isinstance(limits[k], int) and not isinstance(limits[k], bool) and limits[k] > 0
             for k in RESOURCE_KEYS), "resource limits must be positive integers")
    state = receipt.get("status")
    evidence = receipt.get("execution_evidence", {})
    independent = receipt.get("independent_verification", {})

    if state == "design_only_not_executed":
        need(receipt.get("evidence_class") == "synthetic_fixture"
             and receipt.get("execution_state") == "not_executed"
             and receipt.get("execution_performed") is False
             and receipt.get("scoring_eligible") is False,
             "design-only receipt must remain synthetic and ineligible")
        need(all(observed[k] is None for k in RESOURCE_KEYS), "unexecuted receipt cannot claim measured resources")
        for key in ("started_at_utc", "ended_at_utc", "process_exit_code", "stdout_sha256", "stderr_sha256", "output_trace_sha256"):
            need(evidence.get(key) is None, f"unexecuted receipt cannot contain {key}")
        need(evidence.get("output_artifacts") == [], "unexecuted receipt cannot claim output artifacts")
        need(independent == {
            "status": "not_run", "verifier_id": None, "verifier_revision": None, "verified_artifact_count": 0
        }, "unexecuted receipt cannot claim independent verification")
    elif state in {"completed", "failed"}:
        need(receipt.get("execution_performed") is True, "executed receipt must declare execution_performed")
        need(receipt.get("execution_state") == state, "receipt status/execution_state mismatch")
        need(all(isinstance(observed[k], int) and not isinstance(observed[k], bool) and observed[k] >= 0
                 for k in RESOURCE_KEYS), "executed receipt needs resource measurements")
        outputs = evidence.get("output_artifacts")
        need(isinstance(outputs, list), "output_artifacts must be an array")
        out_roles, out_ids = set(), set()
        for artifact in outputs:
            need(artifact.get("role") not in out_roles and artifact.get("artifact_id") not in out_ids,
                 "duplicate output artifact role or ID")
            out_roles.add(artifact.get("role"))
            out_ids.add(artifact.get("artifact_id"))
            verify_artifact_bytes(artifact)
        need(out_roles == REQUIRED_OUTPUT_ROLES, "executed receipt must bind trace, stdout and stderr outputs")
        by_role = {a["role"]: a for a in outputs}
        need(evidence.get("output_trace_sha256") == by_role["output_trace"]["sha256"]
             and evidence.get("stdout_sha256") == by_role["stdout"]["sha256"]
             and evidence.get("stderr_sha256") == by_role["stderr"]["sha256"],
             "output digest index does not match output artifact inventory")
        if receipt.get("scoring_eligible"):
            need(state == "completed" and receipt.get("evidence_class") != "synthetic_fixture",
                 "only completed non-synthetic execution can be scoring eligible")
            need(evidence.get("process_exit_code") == 0, "scoring-eligible run must exit successfully")
            need(all(observed[k] <= limits[k] for k in RESOURCE_KEYS),
                 "scoring-eligible run exceeded declared resource budget")
            need(independent.get("status") == "passed"
                 and isinstance(independent.get("verifier_id"), str) and independent["verifier_id"]
                 and isinstance(independent.get("verifier_revision"), str)
                 and REVISION_RE.fullmatch(independent["verifier_revision"]) is not None
                 and independent.get("verified_artifact_count", 0) >= 3,
                 "scoring-eligible run lacks independent verification")
            need(all(a.get("payload") is not None for a in inventory + outputs),
                 "current offline verifier requires payload-backed artifacts for scoring eligibility")
    else:
        need(state == "invalid" and receipt.get("execution_state") == "invalid"
             and receipt.get("scoring_eligible") is False,
             "invalid receipt must remain ineligible")

    # Source-URI artifacts remain admissible for provenance but are not byte-verified here.
    if receipt.get("scoring_eligible"):
        need(verified_payload_count == len(inventory),
             "every score-eligible input artifact must be payload-backed and hash-verified")


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("usage: verify_execution_receipt.py RECEIPT.json PAIRED_INPUT.json", file=sys.stderr)
        return 2
    try:
        receipt = json.loads(Path(argv[1]).read_text())
        paired = json.loads(Path(argv[2]).read_text())
        verify_receipt(receipt, paired)
    except (OSError, json.JSONDecodeError, ValueError, KeyError, TypeError) as exc:
        print(f"execution receipt verification failed: {exc}", file=sys.stderr)
        return 1
    print("execution receipt and payload SHA-256 verification passed; template remains not executed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
