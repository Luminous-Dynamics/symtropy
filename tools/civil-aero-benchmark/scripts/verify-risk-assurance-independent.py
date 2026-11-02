#!/usr/bin/env python3
"""Independent policy and canonical-digest verifier for aero-risk-assurance/0.1.0."""
from __future__ import annotations

import csv
import hashlib
import struct
import sys
from pathlib import Path

SCHEMA = "aero-risk-assurance/0.1.0"
EVIDENCE = (
    "policy_profile_from_declared_risk_inputs_and_caller_reported_evidence_"
    "references_and_digests_not_independently_replayed_not_risk_estimate_or_regulatory_approval"
)

GATE_ORDER = [
    "problem_identity",
    "input_artifact_provenance",
    "protocol_conformance",
    "process_receipt_replay",
    "native_termination",
    "declared_convergence",
    "conservation",
    "spatial_grid_convergence",
    "sensitivity_analysis",
    "input_uncertainty_quantification",
    "temporal_resolution",
    "per_time_step_inner_convergence",
    "statistical_window_convergence",
    "coupled_discipline_consistency",
    "independent_model_comparison",
    "experimental_validation",
    "blind_domain_shift_challenge",
    "physical_confirmation",
    "independent_reproduction",
    "independent_review",
]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream))


def add_u64(buffer: bytearray, value: int) -> None:
    buffer.extend(struct.pack("<Q", value))


def add_string(buffer: bytearray, value: str) -> None:
    raw = value.encode("utf-8")
    add_u64(buffer, len(raw))
    buffer.extend(raw)


def valid_sha256(value: str) -> bool:
    return len(value) == 64 and all(ch in "0123456789abcdefABCDEF" for ch in value)


def assurance_tier(model_influence: str, consequence: str, exposure: str) -> str:
    require(model_influence in {"low", "moderate", "high"}, "unknown model influence")
    require(consequence in {"low", "moderate", "high", "safety_critical"},
            "unknown decision consequence")
    require(exposure in {"in_domain", "near_boundary", "out_of_domain"},
            "unknown domain exposure")
    if consequence == "safety_critical":
        return "maximum"
    if (model_influence == "high" or consequence == "high"
            or (model_influence == "moderate" and consequence == "moderate")):
        tier = 2
    elif model_influence == "moderate" or consequence == "moderate":
        tier = 1
    else:
        tier = 0
    tier += {"in_domain": 0, "near_boundary": 1, "out_of_domain": 2}[exposure]
    return ["screening", "standard", "enhanced", "maximum"][min(tier, 3)]


def required_gates(tier: str, regime: str) -> list[str]:
    gates = GATE_ORDER[:7]
    tier_value = {"screening": 0, "standard": 1, "enhanced": 2, "maximum": 3}[tier]
    if tier_value >= 1:
        gates.extend([
            "spatial_grid_convergence",
            "sensitivity_analysis",
            "input_uncertainty_quantification",
        ])
    regime_gates = {
        "steady": [],
        "unsteady_dual_time": [
            "temporal_resolution",
            "per_time_step_inner_convergence",
            "statistical_window_convergence",
        ],
        "periodic_statistical": [
            "temporal_resolution",
            "statistical_window_convergence",
        ],
        "transient_endpoint": ["temporal_resolution"],
        "coupled_transient": [
            "temporal_resolution",
            "coupled_discipline_consistency",
        ],
    }
    require(regime in regime_gates, "unknown analysis regime")
    gates.extend(regime_gates[regime])
    if tier_value >= 2:
        gates.extend([
            "independent_model_comparison",
            "experimental_validation",
            "blind_domain_shift_challenge",
        ])
    if tier_value >= 3:
        gates.extend([
            "physical_confirmation",
            "independent_reproduction",
            "independent_review",
        ])
    return sorted(set(gates), key=GATE_ORDER.index)


def canonical_digest(
    report: dict[str, str],
    required: list[str],
    observations: list[dict[str, str]],
    passed: list[str],
    supplemental: list[str],
    blockers: list[str],
) -> str:
    canonical = bytearray()
    for value in (
        report["schema"],
        report["claim_id"],
        report["model_influence"],
        report["decision_consequence"],
        report["domain_exposure"],
        report["analysis_regime"],
        report["assurance_tier"],
        report["admission_status"],
        report["evidence_level"],
    ):
        add_string(canonical, value)
    add_u64(canonical, len(required))
    for gate in required:
        add_string(canonical, gate)
    add_u64(canonical, len(observations))
    for observation in observations:
        for key in ("gate", "status", "source_reference", "source_sha256", "rationale"):
            add_string(canonical, observation[key])
    add_u64(canonical, len(passed))
    for gate in passed:
        add_string(canonical, gate)
    add_u64(canonical, len(supplemental))
    for value in supplemental:
        add_string(canonical, value)
    add_u64(canonical, len(blockers))
    for blocker in blockers:
        add_string(canonical, blocker)
    return hashlib.sha256(canonical).hexdigest()


def main() -> int:
    if len(sys.argv) != 3:
        raise ValueError("usage: verify-risk-assurance-independent.py REPORT.csv OBSERVATIONS.csv")
    report_rows = read_rows(Path(sys.argv[1]))
    observations = read_rows(Path(sys.argv[2]))
    require(len(report_rows) == 1, "expected one risk-assurance report row")
    report = report_rows[0]
    require(report["schema"] == SCHEMA, "risk-assurance schema mismatch")
    require(report["evidence_level"] == EVIDENCE, "risk-assurance evidence ceiling mismatch")
    require(report["claim_id"].strip(), "claim ID is empty")

    tier = assurance_tier(
        report["model_influence"],
        report["decision_consequence"],
        report["domain_exposure"],
    )
    require(report["assurance_tier"] == tier, "assurance tier does not match declared risk axes")
    required = required_gates(tier, report["analysis_regime"])
    reported_required = report["required_gates"].split(";") if report["required_gates"] else []
    require(reported_required == required, "required gate profile differs from independent policy")

    by_gate: dict[str, dict[str, str]] = {}
    for observation in observations:
        gate = observation["gate"]
        status = observation["status"]
        require(gate in GATE_ORDER, f"unknown evidence gate {gate!r}")
        require(status in {"passed", "failed", "unknown"}, f"unknown evidence status {status!r}")
        require(gate not in by_gate, f"duplicate evidence observation for {gate}")
        if status == "passed":
            require(observation["source_reference"].strip() and valid_sha256(observation["source_sha256"]),
                    f"passing evidence {gate} lacks source reference/SHA-256")
            require(not observation["rationale"], f"passing evidence {gate} has a failure rationale")
        else:
            require(observation["rationale"].strip(), f"{status} evidence {gate} needs a rationale")
            require(not observation["source_sha256"] or valid_sha256(observation["source_sha256"]),
                    f"evidence {gate} has malformed source SHA-256")
        by_gate[gate] = observation

    canonical_observations = sorted(
        observations, key=lambda row: GATE_ORDER.index(row["gate"])
    )
    passed = []
    blockers = []
    for gate in required:
        observation = by_gate.get(gate)
        if observation is None:
            blockers.append(f"missing:{gate}")
        elif observation["status"] == "passed":
            passed.append(gate)
        else:
            blockers.append(f"{observation['status']}:{gate}")
    supplemental = [
        f"{row['gate']}:{row['status']}"
        for row in canonical_observations
        if row["gate"] not in required
    ]

    expected_status = (
        "policy_profile_satisfied_by_declared_observations"
        if not blockers
        else "policy_profile_withheld_required_observations_missing_failed_or_unknown"
    )
    require(report["admission_status"] == expected_status, "policy admission status mismatch")
    require((report["passed_gates"].split(";") if report["passed_gates"] else []) == passed,
            "passed gate list mismatch")
    require(
        (report["supplemental_observations"].split(";")
         if report["supplemental_observations"] else []) == supplemental,
        "supplemental observation list mismatch",
    )
    require((report["blockers"].split(";") if report["blockers"] else []) == blockers,
            "blocker list mismatch")

    digest = canonical_digest(report, required, canonical_observations, passed, supplemental, blockers)
    require(report["assessment_sha256"] == digest, "canonical risk-assurance digest mismatch")
    print(
        "independent_risk_assurance_oracle=pass "
        f"tier={tier} required_gates={len(required)} "
        f"passed={len(passed)} blockers={len(blockers)} "
        f"supplemental={len(supplemental)} assessment_sha256={digest}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent risk-assurance oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
