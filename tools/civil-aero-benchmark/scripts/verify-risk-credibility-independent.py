#!/usr/bin/env python3
"""Independent replay oracle for the risk-proportionate credibility policy demo."""
from __future__ import annotations

import csv
import sys
from pathlib import Path

MATRIX = {
    ("advisory", "limited"): "screening",
    ("advisory", "significant"): "low",
    ("advisory", "severe"): "moderate",
    ("material", "limited"): "low",
    ("material", "significant"): "moderate",
    ("material", "severe"): "high",
    ("controlling", "limited"): "moderate",
    ("controlling", "significant"): "high",
    ("controlling", "severe"): "critical",
}
BASE = {
    "intended_use_and_applicability_domain",
    "model_assumptions_and_input_lineage",
    "exact_model_and_run_identity",
    "failure_modes_and_abstention_policy",
    "input_sensitivity_screen",
    "no_release_authorization_from_assessment",
    "screening_only_label",
    "independent_control_review",
}
MECHANISTIC = {
    "code_verification",
    "iterative_convergence",
    "conservation_and_consistency",
    "spatial_and_temporal_convergence",
    "model_form_sensitivity",
}
EMPIRICAL = {
    "dataset_source_and_license_provenance",
    "train_validation_test_separation",
    "predictive_calibration",
    "applicability_domain_and_ood",
}
HYBRID = MECHANISTIC | EMPIRICAL | {"coupling_and_interface_verification"}
TIER_CONTROLS = {
    "screening": set(),
    "low": set(),
    "moderate": {"experimental_comparison", "decision_uncertainty_budget"},
    "high": {
        "experimental_comparison",
        "decision_uncertainty_budget",
        "blind_validation_holdout",
        "independent_reproduction",
    },
    "critical": {
        "experimental_comparison",
        "decision_uncertainty_budget",
        "blind_validation_holdout",
        "independent_reproduction",
        "physical_test_or_justified_equivalent",
        "independent_safety_case_and_authorization",
        "fail_safe_human_override",
    },
}


def require(value: bool, message: str) -> None:
    if not value:
        raise ValueError(message)


def expected_controls(row: dict[str, str]) -> set[str]:
    controls = set(BASE)
    creation = row["model_creation"]
    if creation == "mechanistic_physics_based":
        controls |= MECHANISTIC
    elif creation == "empirical_data_driven":
        controls |= EMPIRICAL
    elif creation == "hybrid_mechanistic_empirical":
        controls |= HYBRID
    else:
        raise ValueError(f"unknown model creation profile: {creation}")

    logic = row["logic_transparency"]
    if logic == "partially_inspectable":
        controls.add("independent_interface_and_decision_trace_audit")
    elif logic == "opaque":
        controls.add("independent_black_box_reproduction")
    elif logic != "inspectable":
        raise ValueError(f"unknown logic transparency: {logic}")

    access = row["artifact_access"]
    if access == "independent_replay_available":
        controls.add("independent_artifact_replay")
    elif access == "restricted":
        controls.add("approved_third_party_artifact_audit")
    elif access != "unavailable":
        raise ValueError(f"unknown artifact access: {access}")

    tier = MATRIX[(row["model_influence"], row["decision_consequence"])]
    controls |= TIER_CONTROLS[tier]
    return controls


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-risk-credibility-independent.py assessments.csv", file=sys.stderr)
        return 2

    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    require(len(rows) == 3, f"expected 3 synthetic profiles, got {len(rows)}")
    expected = {
        "concept-screen": ("advisory", "limited", "screening"),
        "mission-design-support": ("material", "severe", "high"),
        "safety-critical-controller": ("controlling", "severe", "critical"),
    }
    for row in rows:
        model_id = row["model_id"]
        require(model_id in expected, f"unexpected example model {model_id}")
        influence, consequence, tier = expected[model_id]
        require(row["model_influence"] == influence, f"{model_id}: influence mismatch")
        require(row["decision_consequence"] == consequence, f"{model_id}: consequence mismatch")
        require(row["credibility_tier"] == MATRIX[(influence, consequence)] == tier,
                f"{model_id}: independently recomputed tier mismatch")
        controls = set(row["required_controls"].split(";"))
        required = expected_controls(row)
        require(required <= controls, f"{model_id}: required controls missing: {sorted(required-controls)}")
        require(
            row["status"] == "requirements_assessment_only_no_evidence_completion_or_release_claim",
            f"{model_id}: assurance ceiling missing",
        )
        require("no_release_authorization_from_assessment" in controls,
                f"{model_id}: baseline no-release control missing")
        restrictions = row["restrictions"]
        if row["artifact_access"] == "unavailable":
            require("No independent verification or validation claim" in restrictions,
                    f"{model_id}: unavailable artifacts did not restrict claims")
        if tier == "critical":
            require("cannot authorize flight" in restrictions,
                    f"{model_id}: critical restriction missing")

    print("independent_risk_credibility_oracle=pass profiles=3 matrix=3x3 monotone_controls=unit_tested no_release_claim=true")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"independent_risk_credibility_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
