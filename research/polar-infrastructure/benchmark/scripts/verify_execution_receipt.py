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
    need(set(artifact) == {"artifact_id", "role", "source_uri", "payload", "sha256"},
         "artifact keys do not match the strict v0.1 contract")
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
    need(set(receipt) == {
        "schema_version", "artifact_kind", "evidence_class", "status", "receipt_id", "case_id",
        "controller_id", "execution_state", "execution_performed", "scoring_eligible", "execution_attestation", "provenance",
        "paired_input_bindings", "artifact_inventory", "resource_limits", "resource_limits_sha256",
        "observed_resources", "execution_evidence", "independent_verification", "claim_policy",
    }, "receipt keys do not match the strict v0.1 contract")
    need(receipt.get("schema_version") == "0.1.0"
         and receipt.get("artifact_kind") == "symtropy-controller-execution-receipt",
         "unsupported receipt header")
    need(receipt.get("claim_policy") == {
        "synthetic_payloads_are_execution_evidence": False,
        "scoring_requires_completed_execution": True,
        "scoring_requires_independent_output_hash_verification": True,
        "resource_limits_must_be_shared_across_comparison_arms": True,
        "scoring_requires_signed_execution_attestation": True,
        "signed_execution_attestation_verifier_implemented": False,
    }, "receipt claim policy is missing or weakened")
    need(receipt.get("scoring_eligible") is False,
         "v0.1 receipt contract cannot authorize scoring until signed execution attestation is verified")
    need(receipt.get("execution_attestation") == {
        "scheme": "none", "status": "not_implemented", "issuer_id": None,
        "signature_bundle_sha256": None, "signed_payload_sha256": None,
        "verifier_id": None, "verifier_revision": None,
    }, "v0.1 receipt contract requires the signed-attestation gate to remain explicitly unimplemented")

    inventory = receipt.get("artifact_inventory")
    need(isinstance(inventory, list), "artifact_inventory must be an array")
    ids, roles = set(), set()
    for artifact in inventory:
        need(isinstance(artifact, dict), "artifact inventory entry must be an object")
        need(artifact.get("artifact_id") not in ids, "duplicate artifact inventory ID")
        need(artifact.get("role") not in roles, "duplicate artifact inventory role")
        ids.add(artifact.get("artifact_id"))
        roles.add(artifact.get("role"))
        verify_artifact_bytes(artifact)
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
    canonical_limits = json.dumps(limits, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    expected_limits_digest = hashlib.sha256(canonical_limits).hexdigest()
    need(is_sha(receipt.get("resource_limits_sha256"))
         and receipt["resource_limits_sha256"] == expected_limits_digest,
         "numeric resource limits do not match their canonical SHA-256 binding")
    state = receipt.get("status")
    evidence = receipt.get("execution_evidence", {})
    independent = receipt.get("independent_verification", {})
    verification_state = independent.get("status")
    need(verification_state in {"not_run", "passed", "failed"}, "unsupported independent verification status")
    if verification_state == "not_run":
        need(independent.get("verifier_id") is None and independent.get("verifier_revision") is None
             and independent.get("verified_artifact_count") == 0,
             "not_run verification cannot claim verifier identity or verified artifacts")
    if verification_state == "passed":
        need(isinstance(independent.get("verifier_id"), str) and independent["verifier_id"]
             and isinstance(independent.get("verifier_revision"), str)
             and REVISION_RE.fullmatch(independent["verifier_revision"]) is not None
             and isinstance(independent.get("verified_artifact_count"), int)
             and independent["verified_artifact_count"] >= 3,
             "passed verification needs pinned verifier identity and at least three verified artifacts")

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
        need(receipt.get("evidence_class") != "synthetic_fixture",
             "synthetic fixture cannot claim that execution occurred")
        need(receipt.get("execution_performed") is True, "executed receipt must declare execution_performed")
        need(receipt.get("execution_state") == state, "receipt status/execution_state mismatch")
        need(isinstance(receipt.get("provenance", {}).get("receipt_created_at_utc"), str)
             and receipt["provenance"]["receipt_created_at_utc"],
             "executed receipt requires a creation timestamp")
        need(isinstance(evidence.get("started_at_utc"), str) and evidence["started_at_utc"]
             and isinstance(evidence.get("ended_at_utc"), str) and evidence["ended_at_utc"],
             "executed receipt requires start and end timestamps")
        need(isinstance(evidence.get("process_exit_code"), int) and not isinstance(evidence.get("process_exit_code"), bool),
             "executed receipt requires a process exit code")
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
        if state == "completed":
            need(out_roles == REQUIRED_OUTPUT_ROLES, "completed receipt must bind trace, stdout and stderr outputs")
        else:
            need(out_roles.issubset(REQUIRED_OUTPUT_ROLES), "failed receipt has an unsupported output role")
        by_role = {a["role"]: a for a in outputs}
        indexed_digests = {
            "output_trace": evidence.get("output_trace_sha256"),
            "stdout": evidence.get("stdout_sha256"),
            "stderr": evidence.get("stderr_sha256"),
        }
        for role, digest in indexed_digests.items():
            if role in by_role:
                need(digest == by_role[role]["sha256"], f"{role} digest index does not match output inventory")
            else:
                need(digest is None, f"{role} digest is declared without an output artifact")
    else:
        need(state == "invalid" and receipt.get("execution_state") == "invalid"
             and receipt.get("scoring_eligible") is False,
             "invalid receipt must remain ineligible")

    # Source-URI artifacts are accepted for provenance, but are not byte-verified here.


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
