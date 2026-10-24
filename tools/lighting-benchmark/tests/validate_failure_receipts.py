#!/usr/bin/env python3
"""Test-only JSON Schema and cross-field checks for lighting failure receipts."""
from __future__ import annotations

import copy
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, ValidationError

ROOT = Path(__file__).resolve().parents[3]
SCHEMA_PATH = ROOT / "docs/research/lighting-failure-receipt.schema.json"
CASE_PATH = ROOT / "docs/research/fixtures/task-light-room-v0.1.json"
FIXTURE_GLOB = "lighting-failure-*.v0.1.json"


def fail(message: str) -> None:
    print(f"failure receipt schema tests: FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def semantic_errors(receipt: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    observation = receipt["observation"]
    samples = observation["samples"]
    expected = observation["expected_sample_count"]
    observed = observation["observed_sample_count"]

    if observed != len(samples):
        errors.append("observed_sample_count must equal the number of serialized samples")
    if observed > expected:
        errors.append("observed sample count cannot exceed expected count")

    completeness = observation["completeness"]
    if completeness == "none" and (observed != 0 or samples):
        errors.append("none completeness requires zero samples")
    elif completeness == "partial" and not (0 < observed < expected):
        errors.append("partial completeness requires 0 < observed < expected")
    elif completeness == "complete" and observed != expected:
        errors.append("complete completeness requires observed == expected")

    indices = [sample["grid_index"] for sample in samples]
    if indices != sorted(set(indices)):
        errors.append("serialized sample indices must be unique and ascending")

    refs = [
        receipt["inputs"][name]
        for name in ("case", "design", "scene", "property_data", "adapter_config")
    ] + [
        receipt["toolchain"][name]
        for name in ("executable", "environment", "settings")
    ] + [
        receipt["artifacts"][name]
        for name in ("stdout", "stderr", "normalized_samples")
    ]
    for index, artifact in enumerate(refs):
        if artifact["availability"] == "available":
            digest = artifact["sha256"]
            if not isinstance(digest, str) or len(digest) != 64:
                errors.append(f"artifact reference {index} has an invalid available digest")
            elif any(ch not in "0123456789abcdef" for ch in digest):
                errors.append(f"artifact reference {index} digest must be lowercase hexadecimal")
            if artifact["reason"] is not None:
                errors.append(f"available artifact reference {index} must not have an unavailable reason")
        elif artifact["availability"] == "unavailable":
            if artifact["sha256"] is not None or not artifact["reason"]:
                errors.append(f"unavailable artifact reference {index} needs null digest and a reason")

    if receipt["summary"] is not None or receipt["power_observation"] is not None:
        errors.append("failure receipts must not contain rankable summary or power metrics")
    scoring = receipt["scoring"]
    if scoring["eligible"] is not False or scoring["service_energy_metric_wh_per_compliant_task_hour"] is not None:
        errors.append("failure receipts must always be non-rankable")
    gate_names = [gate["name"] for gate in receipt["gate_results"]]
    if len(gate_names) != len(set(gate_names)):
        errors.append("gate names must be unique")

    if receipt["failure_code"] == "artifact_digest_mismatch":
        findings = receipt["integrity_findings"]
        if not findings or not any(
            finding["claimed_sha256"] != finding["observed_sha256"] for finding in findings
        ):
            errors.append("digest mismatch needs at least one differing claimed/observed digest")

    if receipt["failure_code"] == "unknown_required_gate" and not any(
        gate["status"] == "unknown" for gate in receipt["gate_results"]
    ):
        errors.append("unknown required gate must preserve an unknown gate result")

    return errors


def main() -> None:
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    case = json.loads(CASE_PATH.read_text(encoding="utf-8"))
    fixtures = sorted((ROOT / "docs/research/fixtures").glob(FIXTURE_GLOB))
    require(len(fixtures) >= 5, f"expected at least 5 failure receipt fixtures, found {len(fixtures)}")

    loaded: list[tuple[Path, dict[str, Any]]] = []
    for path in fixtures:
        receipt = json.loads(path.read_text(encoding="utf-8"))
        errors = sorted(validator.iter_errors(receipt), key=lambda err: list(err.path))
        require(not errors, f"{path.name} violates JSON Schema: " + "; ".join(e.message for e in errors))
        require(receipt["case_id"] == case["case_id"], f"{path.name} has unexpected case identity")
        sem = semantic_errors(receipt)
        require(not sem, f"{path.name} violates cross-field semantics: " + "; ".join(sem))
        loaded.append((path, receipt))
        print(f"failure receipt schema tests: valid non-rankable fixture: {path.name}")

    by_code = {receipt["failure_code"]: receipt for _, receipt in loaded}

    # The schema must reject attempts to smuggle a failed run back into ranking.
    for name, receipt in loaded:
        mutated = copy.deepcopy(receipt)
        mutated["run_status"] = "compliant"
        require(
            list(validator.iter_errors(mutated)),
            f"{name.name}: schema accepted a compliant status on a failure receipt",
        )

        mutated = copy.deepcopy(receipt)
        mutated["scoring"]["eligible"] = True
        require(
            list(validator.iter_errors(mutated)),
            f"{name.name}: schema accepted scoring.eligible=true",
        )

        mutated = copy.deepcopy(receipt)
        mutated["summary"] = {
            "average_lx": 500,
            "minimum_lx": 300,
            "maximum_lx": 700,
            "minimum_to_average_uniformity": 0.6,
        }
        require(
            list(validator.iter_errors(mutated)),
            f"{name.name}: schema accepted candidate-supplied summary values",
        )

        mutated = copy.deepcopy(receipt)
        mutated["power_observation"] = {"energy_wh": 1}
        require(
            list(validator.iter_errors(mutated)),
            f"{name.name}: schema accepted power data on a failure receipt",
        )

    missing = by_code.get("missing_executable")
    require(missing is not None, "missing-executable negative control is absent")
    mutated = copy.deepcopy(missing)
    mutated["process"]["invocation_attempted"] = True
    require(
        list(validator.iter_errors(mutated)),
        "schema accepted a missing-executable receipt that claims invocation began",
    )

    timeout = by_code.get("timeout")
    require(timeout is not None, "timeout negative control is absent")
    mutated = copy.deepcopy(timeout)
    mutated["process"]["timed_out"] = False
    require(
        list(validator.iter_errors(mutated)),
        "schema accepted a timeout receipt with timed_out=false",
    )

    digest_mismatch = by_code.get("artifact_digest_mismatch")
    require(digest_mismatch is not None, "digest-mismatch negative control is absent")
    mutated = copy.deepcopy(digest_mismatch)
    mutated["integrity_findings"] = []
    require(
        list(validator.iter_errors(mutated)),
        "schema accepted digest-mismatch receipt without an integrity finding",
    )

    unknown_gate = by_code.get("unknown_required_gate")
    require(unknown_gate is not None, "unknown-required-gate negative control is absent")
    mutated = copy.deepcopy(unknown_gate)
    mutated["gate_results"][0]["status"] = "pass"
    require(
        list(validator.iter_errors(mutated)),
        "schema accepted an unknown-required-gate failure without an unknown gate",
    )

    # Custom cross-field semantics catch relationships that JSON Schema alone
    # cannot express portably, such as exact count equality and digest inequality.
    timeout = by_code["timeout"]
    mutated = copy.deepcopy(timeout)
    mutated["observation"]["observed_sample_count"] += 1
    require(
        semantic_errors(mutated),
        "semantic checks accepted a sample count that differs from the serialized sample array",
    )

    digest_mismatch = by_code["artifact_digest_mismatch"]
    mutated = copy.deepcopy(digest_mismatch)
    mutated["integrity_findings"][0]["observed_sha256"] = mutated["integrity_findings"][0]["claimed_sha256"]
    require(
        semantic_errors(mutated),
        "semantic checks accepted identical digests as proof of a digest mismatch",
    )

    print(
        "failure receipt schema tests: PASS "
        f"({len(fixtures)} valid fixtures; fail-closed schema and semantic negative controls)"
    )
    print(
        "scope: schema and receipt semantics only; not solver-parser correctness, "
        "optical accuracy, calibration or physical-product performance"
    )


if __name__ == "__main__":
    main()
