#!/usr/bin/env python3
"""CI-only JSON Schema and cross-record invariant checks for Power Fabric contracts.

This script is not a production admission authority. It proves only that the
schemas, fixtures and explicit fail-closed structural invariants are consistent.
"""
from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
PROFILE_SCHEMA_PATH = ROOT / "docs/schemas/power-fabric/physical-model-profile-v0.1.schema.json"
RECEIPT_SCHEMA_PATH = ROOT / "docs/schemas/power-fabric/model-validation-receipt-v0.1.schema.json"
PROFILE_FIXTURE_PATH = ROOT / "docs/research/fixtures/power-fabric/physical-model-profile-synthetic-gfm-v0.1.json"
RECEIPT_FIXTURE_PATH = ROOT / "docs/research/fixtures/power-fabric/model-validation-receipt-synthetic-not-evaluable-v0.1.json"


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"{path.relative_to(ROOT)}: cannot load JSON: {exc}") from exc


def validate_schema(schema_path: Path, instance_path: Path) -> dict[str, Any]:
    schema = load_json(schema_path)
    instance = load_json(instance_path)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(
        validator.iter_errors(instance),
        key=lambda error: (list(map(str, error.absolute_path)), error.message),
    )
    if errors:
        details = [
            f"{instance_path.relative_to(ROOT)} at /{'/'.join(map(str, error.absolute_path))}: {error.message}"
            for error in errors
        ]
        raise SystemExit("\n".join(details))
    print(f"PASS schema: {schema_path.relative_to(ROOT)}")
    print(f"PASS fixture: {instance_path.relative_to(ROOT)}")
    return instance


def state_is_resolved(value: dict[str, Any]) -> bool:
    return value.get("state") in {"known", "not_applicable"}


def check_profile(profile: dict[str, Any]) -> None:
    axis_ids = [axis["quantity"] for axis in profile["operating_envelope"]["axes"]]
    output_ids = [output["output_id"] for output in profile["outputs"]]
    port_ids = [port["port_id"] for port in profile["boundary"]["ports"]]
    for label, values in (("axis", axis_ids), ("output", output_ids), ("port", port_ids)):
        if len(values) != len(set(values)):
            raise SystemExit(f"profile has duplicate {label} identities")

    axis_set = set(axis_ids)
    for axis in profile["operating_envelope"]["axes"]:
        coverage = axis["coverage"]
        if coverage["state"] == "bounded" and coverage["lower"] >= coverage["upper"]:
            raise SystemExit(f"operating-envelope axis {axis['quantity']} has an empty or inverted range")

    for output in profile["outputs"]:
        undeclared = set(output["validity_axes"]) - axis_set
        if undeclared:
            raise SystemExit(
                f"profile output {output['output_id']} references undeclared axes: {sorted(undeclared)}"
            )
        coverage = output["uncertainty"]
        if coverage["state"] == "quantified" and coverage.get("value", -1) < 0:
            raise SystemExit(f"profile output {output['output_id']} has negative uncertainty")

    if profile["record_status"] == "admitted":
        model = profile["model"]
        target = profile["target"]
        required_refs = [
            model["revision"], model["source_artifact"], model["build_artifact"],
            model["dependency_artifact"], target["asset_id"], target["module_id"],
            target["hardware_revision"], target["firmware_revision"],
            target["parameter_set"], target["as_built_configuration"],
            target["service_graph_revision"], profile["boundary"]["topology_revision"],
        ]
        if not all(state_is_resolved(ref) for ref in required_refs):
            raise SystemExit("admitted profile contains unresolved model/asset/configuration identity")
        if any(axis["coverage"]["state"] == "unknown" for axis in profile["operating_envelope"]["axes"]):
            raise SystemExit("admitted profile contains an unknown operating-envelope axis")
        if profile["numerical_envelope"]["precision"]["state"] == "unknown":
            raise SystemExit("admitted profile has unknown numerical precision")
        if any(output["claim_ceiling"] == "structural_only" for output in profile["outputs"]):
            raise SystemExit("admitted profile still has structural-only output claims")


def check_receipt(profile: dict[str, Any], receipt: dict[str, Any]) -> None:
    if receipt["profile_id"] != profile["profile_id"]:
        raise SystemExit("receipt profile_id does not match the supplied profile")
    expected_outputs = {output["output_id"] for output in profile["outputs"]}
    actual_outputs = [output["output_id"] for output in receipt["output_results"]]
    if len(actual_outputs) != len(set(actual_outputs)):
        raise SystemExit("receipt has duplicate output result identities")
    if set(actual_outputs) != expected_outputs:
        raise SystemExit("receipt output result identities do not exactly cover the profile outputs")

    profile_outputs = {output["output_id"]: output for output in profile["outputs"]}
    for output in receipt["output_results"]:
        profile_output = profile_outputs[output["output_id"]]
        if output["unit"] != profile_output["unit"]:
            raise SystemExit(f"receipt output {output['output_id']} unit differs from its profile")
        if output["status"] in {"pass", "fail"}:
            recomputed_error = abs(output["observed"] - output["reference"])
            if abs(recomputed_error - output["absolute_error"]) > max(1e-12, 1e-9 * recomputed_error):
                raise SystemExit(f"receipt output {output['output_id']} reports an inconsistent absolute error")
        if output["status"] == "pass" and output["absolute_error"] > output["tolerance"]:
            raise SystemExit(f"output {output['output_id']} claims pass above its tolerance")
        if output["status"] == "fail" and output["absolute_error"] <= output["tolerance"]:
            raise SystemExit(f"output {output['output_id']} claims fail within its tolerance")

    for control in receipt["negative_controls"]:
        if control["status"] == "pass" and control["observed_disposition"] != control["expected_disposition"]:
            raise SystemExit(f"negative control {control['control_id']} passed with the wrong disposition")

    if receipt["overall_disposition"] == "complete_pass":
        if profile["record_status"] != "admitted":
            raise SystemExit("complete_pass receipt references a profile that is not admitted")
        known_refs = [
            receipt["profile_artifact"], receipt["target"]["asset_id"],
            receipt["target"]["hardware_revision"], receipt["target"]["firmware_revision"],
            receipt["target"]["as_built_configuration"], receipt["run"]["model_revision"],
            receipt["run"]["input_artifact"], receipt["run"]["run_artifact"],
            receipt["protocol"]["revision"], receipt["protocol"]["protocol_artifact"],
        ]
        if not all(state_is_resolved(ref) for ref in known_refs):
            raise SystemExit("complete_pass receipt contains unresolved identity or artifact references")
        if not receipt["reference_methods"]:
            raise SystemExit("complete_pass receipt has no independent reference method")
        if any(result["status"] != "pass" for result in receipt["output_results"]):
            raise SystemExit("complete_pass receipt has a non-passing output")
        if any(control["status"] != "pass" for control in receipt["negative_controls"]):
            raise SystemExit("complete_pass receipt has an unexecuted or failed negative control")
        if receipt["claim_ceiling"] == "structural_only":
            raise SystemExit("complete_pass receipt cannot have a structural-only claim ceiling")
    if receipt["overall_disposition"] == "not_evaluable":
        if all(result["status"] == "pass" for result in receipt["output_results"]):
            raise SystemExit("not_evaluable receipt must not contain only passing outputs")


def main() -> int:
    profile = validate_schema(PROFILE_SCHEMA_PATH, PROFILE_FIXTURE_PATH)
    receipt = validate_schema(RECEIPT_SCHEMA_PATH, RECEIPT_FIXTURE_PATH)
    check_profile(profile)
    check_receipt(profile, receipt)
    if receipt["overall_disposition"] != "not_evaluable":
        raise SystemExit("the authored synthetic receipt must remain not_evaluable")
    print("PASS: cross-record and fail-closed fixture invariants")
    print("Claim ceiling: schema/fixture consistency only; no physical validation is asserted.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
