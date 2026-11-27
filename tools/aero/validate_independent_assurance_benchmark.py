#!/usr/bin/env python3
"""CI-only structural and cross-reference validator for AERO IAB v0.3.

This script is not part of AERO's production decision path. It validates the
published benchmark schemas, instances, metric contracts and challenge seed.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[2]
AVIATION = ROOT / "docs" / "research" / "aviation"


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"FAIL: cannot load {path.relative_to(ROOT)}: {exc}") from exc


def validate_instance(instance: Any, schema: Any, label: str) -> None:
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(
        validator.iter_errors(instance),
        key=lambda error: (list(map(str, error.absolute_path)), error.message),
    )
    if errors:
        details = "\n".join(
            f"  - {label} at /{'/'.join(map(str, error.absolute_path))}: "
            f"{error.message}"
            for error in errors
        )
        raise SystemExit(f"FAIL: JSON Schema validation failed:\n{details}")


def require_unique(values: list[str], label: str) -> None:
    duplicates = sorted({value for value in values if values.count(value) > 1})
    if duplicates:
        raise SystemExit(f"FAIL: duplicate {label}: {', '.join(duplicates)}")


def main() -> int:
    registry_schema_path = AVIATION / "schemas" / "aero_independent_assurance_benchmark_v0_3.schema.json"
    registry_path = AVIATION / "examples" / "aero_independent_assurance_benchmark_v0_3.json"
    challenge_schema_path = AVIATION / "schemas" / "aero_independent_assurance_challenge_v0_3.schema.json"
    challenge_path = AVIATION / "examples" / "aero_independent_assurance_challenge_v0_3.json"

    registry_schema = load_json(registry_schema_path)
    registry = load_json(registry_path)
    challenge_schema = load_json(challenge_schema_path)
    challenge = load_json(challenge_path)

    validate_instance(registry, registry_schema, "benchmark registry")
    validate_instance(challenge, challenge_schema, "challenge corpus")

    definitions = registry["metric_definitions"]
    metric_ids = [definition["id"] for definition in definitions]
    require_unique(metric_ids, "metric IDs")
    metric_by_id = {definition["id"]: definition for definition in definitions}

    family_ids = [family["id"] for family in registry["families"]]
    require_unique(family_ids, "benchmark family IDs")
    if len(family_ids) != 7:
        raise SystemExit(f"FAIL: expected 7 benchmark families, found {len(family_ids)}")

    family_metric_ids = [
        metric_id
        for family in registry["families"]
        for metric_id in family["primary_metrics"]
    ]
    missing_family_metrics = sorted(set(family_metric_ids) - set(metric_ids))
    if missing_family_metrics:
        raise SystemExit(
            "FAIL: family references undefined metrics: "
            + ", ".join(missing_family_metrics)
        )

    thresholds = registry["thresholds"]
    threshold_ids = [threshold["metric"] for threshold in thresholds]
    require_unique(threshold_ids, "threshold metric IDs")
    for threshold in thresholds:
        metric_id = threshold["metric"]
        definition = metric_by_id.get(metric_id)
        if definition is None:
            raise SystemExit(f"FAIL: threshold references undefined metric {metric_id}")
        if threshold["unit"] != definition["unit"]:
            raise SystemExit(f"FAIL: unit mismatch for threshold {metric_id}")
        expected_operator = (
            "minimum" if definition["optimization"] == "maximize" else "maximum"
        )
        if threshold["operator"] != expected_operator:
            raise SystemExit(
                f"FAIL: threshold operator for {metric_id} must be {expected_operator}"
            )
        if threshold["unit"] == "proportion" and not 0.0 <= threshold["target"] <= 1.0:
            raise SystemExit(f"FAIL: proportion threshold out of range for {metric_id}")

    cases = challenge["cases"]
    if len(cases) != 32:
        raise SystemExit(f"FAIL: v0.3 challenge seed must contain 32 cases, found {len(cases)}")
    case_ids = [case["case_id"] for case in cases]
    require_unique(case_ids, "challenge case IDs")
    case_classes = {case["case_class"] for case in cases}
    required_classes = {"positive_control", "single_fault", "coupled_fault", "metamorphic"}
    if case_classes != required_classes:
        raise SystemExit(
            f"FAIL: challenge classes differ: expected {sorted(required_classes)}, "
            f"found {sorted(case_classes)}"
        )
    if not any(case["case_class"] == "positive_control" for case in cases):
        raise SystemExit("FAIL: challenge corpus has no positive control")

    state_schema_path = AVIATION / "schemas" / "aero_assurance_state_vector_v0_1.schema.json"
    state_vectors_path = AVIATION / "examples" / "aero_assurance_state_vectors_v0_1.json"
    state_schema = load_json(state_schema_path)
    state_vectors = load_json(state_vectors_path)
    validate_instance(state_vectors, state_schema, "assurance state vectors")

    state_cases = state_vectors["cases"]
    if len(state_cases) != 13:
        raise SystemExit(f"FAIL: expected 13 state-vector cases, found {len(state_cases)}")
    state_case_ids = [case["case_id"] for case in state_cases]
    require_unique(state_case_ids, "state-vector case IDs")
    required_projections = {
        "REJECTED", "OUT_OF_SCOPE", "NOT_KNOWN_AT_SNAPSHOT",
        "NOT_EFFECTIVE_AT_SUBJECT_TIME", "UNVERIFIED", "INCOMPLETE",
        "REVIEWABLE_FOR_INDEPENDENT_DECISION",
    }
    projections = {case["expected_disposition"] for case in state_cases}
    if not required_projections.issubset(projections):
        raise SystemExit(
            "FAIL: state-vector examples do not cover all disposition projections"
        )

    for case in state_cases:
        obligations = case["obligations"]
        obligation_ids = [obligation["obligation_id"] for obligation in obligations]
        require_unique(obligation_ids, f"obligation IDs in {case['case_id']}")
        if not set(case["expected_blocking_obligation_ids"]).issubset(set(obligation_ids)):
            raise SystemExit(f"FAIL: unknown blocking obligation in {case['case_id']}")
        mandatory = [item for item in obligations if item["mandatory"]]
        disposition = case["expected_disposition"]
        if disposition == "REVIEWABLE_FOR_INDEPENDENT_DECISION":
            for item in mandatory:
                if (
                    item["proposition_evidence"] != "SUPPORTED_ONLY"
                    or item["applicability"] != "IN_SCOPE"
                    or item["authority"] not in {"AUTHORIZED", "NOT_REQUIRED"}
                    or item["temporal_knowledge"] not in {"KNOWN_AT_CUTOFF", "NOT_TEMPORAL"}
                    or item["observer_coverage"] not in {"COMPLETE", "NOT_APPLICABLE"}
                    or item["verification"] != "VERIFIED"
                ):
                    raise SystemExit(
                        f"FAIL: reviewable fixture has a blocking axis in {case['case_id']}"
                    )
            if any(
                item["status"] == "UNRESOLVED" and item["materiality"] == "BLOCKING"
                for item in case["defeaters"]
            ):
                raise SystemExit(f"FAIL: reviewable fixture has unresolved blocking defeater in {case['case_id']}")
        elif disposition == "REJECTED":
            if not any(
                (item["hard_gate"] and (
                    (item["proposition_evidence"] == "REFUTED_ONLY"
                     and item["verification"] == "VERIFIED")
                    or item["verification"] == "FAILED"
                ))
                or (item["authority"] == "DENIED" and item["verification"] == "VERIFIED")
                for item in mandatory
            ) and case["subject_time_effectivity"] != "INVALID_INTERVAL":
                raise SystemExit(f"FAIL: rejected fixture has no verified hard blocker in {case['case_id']}")
        elif disposition == "OUT_OF_SCOPE":
            if not any(item["applicability"] == "OUT_OF_SCOPE" for item in mandatory):
                raise SystemExit(f"FAIL: out-of-scope fixture has no out-of-scope obligation in {case['case_id']}")
        elif disposition == "NOT_KNOWN_AT_SNAPSHOT":
            if not any(item["temporal_knowledge"] == "NOT_KNOWN_AT_CUTOFF" for item in mandatory):
                raise SystemExit(f"FAIL: historical fixture has no late evidence in {case['case_id']}")
        elif disposition == "UNVERIFIED":
            if not any(
                item["authority"] == "UNKNOWN" or item["verification"] == "UNVERIFIED"
                for item in mandatory
            ):
                raise SystemExit(f"FAIL: unverified fixture has no unverified axis in {case['case_id']}")
        elif disposition == "NOT_EFFECTIVE_AT_SUBJECT_TIME":
            if case["subject_time_effectivity"] != "DEFINITELY_NOT_YET_EFFECTIVE":
                raise SystemExit(f"FAIL: subject-time disposition mismatch in {case['case_id']}")
        if case["subject_time_effectivity"] == "INVALID_INTERVAL" and disposition != "REJECTED":
            raise SystemExit(f"FAIL: invalid subject-time interval was not rejected in {case['case_id']}")
        if case["subject_time_effectivity"] == "BOUNDARY_AMBIGUOUS" and disposition != "INCOMPLETE":
            raise SystemExit(f"FAIL: ambiguous subject-time interval did not abstain in {case['case_id']}")
        elif disposition == "INCOMPLETE":
            if not any(
                item["proposition_evidence"] in {"NEITHER", "BOTH"}
                or item["observer_coverage"] in {"UNKNOWN", "INCOMPLETE"}
                for item in mandatory
            ) and not any(
                item["status"] == "UNRESOLVED" and item["materiality"] == "BLOCKING"
                for item in case["defeaters"]
            ) and case["subject_time_effectivity"] != "BOUNDARY_AMBIGUOUS":
                raise SystemExit(f"FAIL: incomplete fixture has no missing/conflicting obligation in {case['case_id']}")

    print(
        "PASS: AERO IAB v0.3 structural validation "
        f"({len(definitions)} metrics, {len(thresholds)} thresholds, "
        f"{len(registry['families'])} families, {len(cases)} challenge cases)."
    )
    print("NOTE: schema/registry validity is not oracle adjudication or benchmark execution.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
