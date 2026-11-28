#!/usr/bin/env python3
"""Structural and exact-commitment checks for AERO robust EVSI v0.1.

This is a CI-only contract validator. Rust remains the production arithmetic
path; this script independently reconstructs the finite scenario-set digest and
checks scenario scope, chronology and calibration-table commitments.
"""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[2]
AVIATION = ROOT / "docs" / "research" / "aviation"
SCHEMA_PATH = AVIATION / "schemas" / "aero_evidence_action_robust_evsi_v0_1.schema.json"
EXAMPLE_PATH = AVIATION / "examples" / "aero_evidence_action_robust_evsi_v0_1.json"


def digest_json(value: Any) -> str:
    payload = json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def outcome_likelihoods_sha256(action: dict[str, Any], outcome_model_sha256: str) -> str:
    return digest_json({
        "schema": "AeroOutcomeLikelihoodTableV0_1",
        "canonicalization_profile": "rust-serde-json-struct-order-v1",
        "action_id": action["action_id"],
        "outcome_model_sha256": outcome_model_sha256,
        "likelihoods": sorted(
            action["outcome_likelihoods"],
            key=lambda item: (item["state_id"], item["outcome_id"]),
        ),
    })


def scenario_set_sha256(source: dict[str, Any]) -> str:
    entries = []
    for scenario in sorted(source["scenarios"], key=lambda item: item["scenario_id"]):
        item = scenario["input"]
        actions = []
        for action in sorted(item["actions"], key=lambda value: value["action_id"]):
            receipt = action["calibration_receipt"]
            actions.append({
                "action_id": action["action_id"],
                "outcome_model_owner_namespace": action["outcome_model_owner_namespace"],
                "calibration_owner_namespace": (
                    receipt["owner_namespace"] if receipt is not None else None
                ),
                "calibration_receipt_sha256": (
                    receipt["calibration_report_sha256"] if receipt is not None else None
                ),
                "calibration_profile_sha256": (
                    receipt["calibration_profile_sha256"] if receipt is not None else None
                ),
                "outcome_likelihoods_sha256": (
                    receipt["outcome_likelihoods_sha256"] if receipt is not None else None
                ),
            })
        entries.append({
            "scenario_id": scenario["scenario_id"],
            "decision_model_sha256": item["decision_model_sha256"],
            "outcome_model_sha256": item["outcome_model_sha256"],
            "action_catalogue_sha256": item["action_catalogue_sha256"],
            "unresolved_obligation_manifest_sha256": item["unresolved_obligation_manifest_sha256"],
            "action_calibration_commitments": actions,
        })
    return digest_json({
        "schema": "AeroEVSIAmbiguityScenarioSetV0_1",
        "canonicalization_profile": "rust-serde-json-struct-order-v1",
        "scenario_set_id": source["scenario_set_id"],
        "entries": entries,
    })


def validate(source: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    scenarios = source["scenarios"]
    ids = [item["scenario_id"] for item in scenarios]
    if len(ids) != len(set(ids)):
        errors.append("scenario IDs are duplicated")
    if len(scenarios) < 2:
        errors.append("finite ambiguity set requires at least two scenarios")
    if scenario_set_sha256(source) != source["scenario_set_sha256"]:
        errors.append("scenario-set digest mismatch")

    first = scenarios[0]["input"]
    cutoff = first["as_known_at_unix_ms"]
    approval = source["approval_receipt"]
    if approval["verification_status"] != "VERIFIED":
        errors.append("scenario-set approval is not upstream-verified")
    if (
        approval["claim_id"] != first["claim_id"]
        or approval["decision_context_sha256"] != first["decision_context_sha256"]
        or approval["configuration_ref"] != first["configuration_ref"]
        or approval["scenario_set_sha256"] != source["scenario_set_sha256"]
        or approval["scenario_count"] != len(scenarios)
    ):
        errors.append("scenario-set approval scope/count mismatch")
    if approval["receiver_first_seen_at_unix_ms"] > cutoff:
        errors.append("scenario-set approval was not known at the selected cutoff")
    if not (
        approval["effective_from_unix_ms"] <= cutoff < approval["effective_until_unix_ms"]
    ):
        errors.append("scenario-set approval is not effective at the selected cutoff")

    for scenario in scenarios:
        item = scenario["input"]
        if (
            item["claim_id"] != first["claim_id"]
            or item["frozen_claim_disposition"] != first["frozen_claim_disposition"]
            or item["configuration_ref"] != first["configuration_ref"]
            or item["as_known_at_unix_ms"] != cutoff
            or item["decision_context_sha256"] != first["decision_context_sha256"]
            or item["assurance_state_sha256"] != first["assurance_state_sha256"]
            or item["assurance_state_receipt"] != first["assurance_state_receipt"]
            or item["unresolved_obligation_manifest_sha256"]
                != first["unresolved_obligation_manifest_sha256"]
            or item["action_catalogue_sha256"] != first["action_catalogue_sha256"]
            or item["action_catalogue_receipt"] != first["action_catalogue_receipt"]
        ):
            errors.append(f"scenario {scenario['scenario_id']} has a claim/state/catalogue context mismatch")

        for action in item["actions"]:
            receipt = action["calibration_receipt"]
            if receipt is not None:
                if receipt["owner_namespace"] != action["outcome_model_owner_namespace"]:
                    errors.append(
                        f"scenario {scenario['scenario_id']} action {action['action_id']}: "
                        "calibration owner mismatch"
                    )
                if receipt["outcome_model_sha256"] != item["outcome_model_sha256"]:
                    errors.append(
                        f"scenario {scenario['scenario_id']} action {action['action_id']}: "
                        "outcome-model digest mismatch"
                    )
                if receipt["outcome_likelihoods_sha256"] != outcome_likelihoods_sha256(
                    action, item["outcome_model_sha256"]
                ):
                    errors.append(
                        f"scenario {scenario['scenario_id']} action {action['action_id']}: "
                        "likelihood-table commitment mismatch"
                    )
                if receipt["receiver_first_seen_at_unix_ms"] > cutoff:
                    errors.append(
                        f"scenario {scenario['scenario_id']} action {action['action_id']}: "
                        "calibration receipt was late at cutoff"
                    )
                if not (
                    receipt["effective_from_unix_ms"] <= cutoff
                    < receipt["effective_until_unix_ms"]
                ):
                    errors.append(
                        f"scenario {scenario['scenario_id']} action {action['action_id']}: "
                        "calibration receipt not effective at cutoff"
                    )
    return errors


def main() -> int:
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    instance = json.loads(EXAMPLE_PATH.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    schema_errors = sorted(
        validator.iter_errors(instance),
        key=lambda error: (list(map(str, error.absolute_path)), error.message),
    )
    if schema_errors:
        for error in schema_errors:
            print(f"SCHEMA FAIL /{'/'.join(map(str, error.absolute_path))}: {error.message}")
        return 1

    errors = validate(instance)
    if errors:
        for error in errors:
            print(f"CONTRACT FAIL: {error}")
        return 1
    print(
        "PASS: AERO robust EVSI v0.1 scenario-set schema, shared-context, "
        "chronology and calibration commitments."
    )
    print(
        "NOTE: the published example uses synthetic placeholder owner receipts; "
        "this structural check is not owner-signature authentication, independent "
        "scenario adjudication or physical qualification."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
