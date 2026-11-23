#!/usr/bin/env python3
"""Independent standard-library oracle for the analytic circle measurability corpus."""

import hashlib
import json
import math
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-feature-measurability.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-feature-measurability.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-FMQ-ANALYTIC-001"
CLAIM_CEILING = "synthetic_2d_circle_geometric_identifiability_only"
FEATURE_MODEL = "CIRCLE_2D_CENTER_X_CENTER_Y_RADIUS"
ANGLE_SCALE = 1_000_000.0
MATRIX_SCALE = 1_000_000_000.0
CONDITION_SCALE = 10_000.0


def quantize(value, scale):
    if not math.isfinite(value) or value < 0:
        raise ValueError("FEATURE_INFORMATION_NUMERIC_INVALID")
    scaled = math.floor(value * scale + 0.5)
    if scaled > 2**63 - 1:
        raise ValueError("FEATURE_INFORMATION_NUMERIC_OVERFLOW")
    return scaled

def quantize_signed(value, scale):
    if not math.isfinite(value):
        raise ValueError("FEATURE_INFORMATION_NUMERIC_INVALID")
    scaled = math.floor(value * scale + 0.5) if value >= 0 else math.ceil(value * scale - 0.5)
    if scaled < -(2**63) or scaled > 2**63 - 1:
        raise ValueError("FEATURE_INFORMATION_NUMERIC_OVERFLOW")
    return scaled


def determinant(m):
    return (
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    )


def symmetric_eigenvalues(matrix):
    q = (matrix[0][0] + matrix[1][1] + matrix[2][2]) / 3.0
    p2 = (
        (matrix[0][0] - q) ** 2
        + (matrix[1][1] - q) ** 2
        + (matrix[2][2] - q) ** 2
        + 2.0 * (matrix[0][1] ** 2 + matrix[0][2] ** 2 + matrix[1][2] ** 2)
    )
    p = math.sqrt(p2 / 6.0)
    if p <= float.fromhex("0x1.0000000000000p-52"):
        return [max(q, 0.0)] * 3
    normalized = [[matrix[i][j] / p for j in range(3)] for i in range(3)]
    for i in range(3):
        normalized[i][i] -= q / p
    r = max(-1.0, min(1.0, determinant(normalized) / 2.0))
    phi = math.acos(r) / 3.0
    largest = q + 2.0 * p * math.cos(phi)
    smallest = q + 2.0 * p * math.cos(phi + 2.0 * math.pi / 3.0)
    middle = 3.0 * q - largest - smallest
    return sorted([max(smallest, 0.0), max(middle, 0.0), max(largest, 0.0)])


def diagnostic(case, robust_only):
    admitted = [
        row for row in case["observations"]
        if row["robustly_observed"] if robust_only
    ] if robust_only else [
        row for row in case["observations"] if row["nominally_observed"]
    ]
    angle_keys = sorted({
        math.floor(row["contour_angle_deg"] * ANGLE_SCALE + 0.5)
        for row in admitted
    })
    matrix = [[0.0] * 3 for _ in range(3)]
    if angle_keys:
        for key in angle_keys:
            angle = key / ANGLE_SCALE * math.pi / 180.0
            row = [-math.cos(angle), -math.sin(angle), -1.0]
            for i in range(3):
                for j in range(3):
                    matrix[i][j] += row[i] * row[j]
        for i in range(3):
            for j in range(3):
                matrix[i][j] /= len(angle_keys)
    eigenvalues = symmetric_eigenvalues(matrix)
    maximum = eigenvalues[2]
    threshold = maximum * case["relative_rank_tolerance"]
    rank = sum(1 for value in eigenvalues if maximum > 0 and value > threshold)
    condition = maximum / eigenvalues[0] if rank == 3 and eigenvalues[0] > 0 else None
    if condition is None:
        status = "NOT_MEASURABLE"
    elif condition > case["max_condition_number"]:
        status = "ILL_CONDITIONED"
    else:
        status = "MEASURABLE"
    domains = sorted({row["failure_domain_id"] for row in admitted})
    return {
        "status": status,
        "observation_row_count": len(admitted),
        "unique_geometric_support_count": len(angle_keys),
        "distinct_failure_domain_count": len(domains),
        "information_matrix_q1e9": [
            [quantize_signed(value, MATRIX_SCALE) for value in row]
            for row in matrix
        ],
        "eigenvalues_q1e9": [quantize(value, MATRIX_SCALE) for value in eigenvalues],
        "numerical_rank": rank,
        "condition_number_q1e4": (
            quantize(condition, CONDITION_SCALE) if condition is not None else None
        ),
        "max_condition_number_q1e4": quantize(
            case["max_condition_number"], CONDITION_SCALE
        ),
    }


def evaluate(case):
    if (
        not case["case_id"]
        or not case["feature_id"]
        or case["feature_model"] != FEATURE_MODEL
        or not math.isfinite(case["max_condition_number"])
        or case["max_condition_number"] <= 1
        or not math.isfinite(case["relative_rank_tolerance"])
        or not 0 < case["relative_rank_tolerance"] < 1
        or not case["observations"]
        or len(case["observations"]) > 4096
    ):
        raise ValueError("FEATURE_CASE_BOUNDS_INVALID")
    ids = [row["observation_id"] for row in case["observations"]]
    for row in case["observations"]:
        angle = row["contour_angle_deg"]
        if (
            not row["observation_id"] or not row["view_id"] or not row["failure_domain_id"]
            or not math.isfinite(angle) or not 0 <= angle < 360
            or (row["robustly_observed"] and not row["nominally_observed"])
        ):
            raise ValueError("FEATURE_OBSERVATION_INVALID")
    if len(ids) != len(set(ids)):
        raise ValueError("FEATURE_OBSERVATION_INVALID")
    nominal = diagnostic(case, False)
    robust = diagnostic(case, True)
    if nominal["status"] == "NOT_MEASURABLE":
        disposition = "INFEASIBLE"
    elif nominal["status"] == "MEASURABLE" and robust["status"] == "MEASURABLE":
        disposition = "FEASIBLE"
    else:
        disposition = "INDETERMINATE"
    match = (
        nominal["status"] == case["expected_nominal_status"]
        and robust["status"] == case["expected_robust_status"]
        and disposition == case["expected_disposition"]
    )
    return {
        "case_id": case["case_id"],
        "feature_id": case["feature_id"],
        "nominal": nominal,
        "robust": robust,
        "disposition": disposition,
        "expected_nominal_status": case["expected_nominal_status"],
        "expected_robust_status": case["expected_robust_status"],
        "expected_disposition": case["expected_disposition"],
        "expected_outcome_matches": match,
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: reference_oracle.py <fixture.json> <40-hex-source-commit>")
    raw = Path(sys.argv[1]).read_bytes()
    source_commit = sys.argv[2]
    if len(source_commit) != 40 or any(c not in "0123456789abcdefABCDEF" for c in source_commit):
        raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw)
    if (
        corpus.get("schema") != CORPUS_SCHEMA
        or corpus.get("benchmark_id") != BENCHMARK_ID
        or corpus.get("evidence_class") != "synthetic"
        or corpus.get("claim_ceiling") != CLAIM_CEILING
        or len(corpus.get("cases", [])) != 5
    ):
        raise SystemExit("FEATURE_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")
    cases = [evaluate(case) for case in corpus["cases"]]
    matches = sum(1 for case in cases if case["expected_outcome_matches"])
    receipt = {
        "schema": RECEIPT_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "source_commit": source_commit.lower(),
        "input_sha256": hashlib.sha256(raw).hexdigest(),
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM_CEILING,
        "case_count": len(cases),
        "expected_outcome_matches": matches,
        "expected_outcome_mismatches": len(cases) - matches,
        "authority_granted": False,
        "measurement_uncertainty_estimated": False,
        "probability_of_detection_assessed": False,
        "physical_validation": "NOT_ASSESSED",
        "cases": cases,
    }
    print(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    main()
