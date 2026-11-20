#!/usr/bin/env python3
"""Independent exhaustive reference for tiny interval-simplex service-risk cases.

This is a CI/reference utility, not a production service or probability engine.
It enumerates every integer-basis-point probability vector in the declared
three-branch intervals and checks the Rust qualifier's reported maxima.
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

TOTAL = 10_000
TOLERANCE = 1e-8


def load_json(path: str) -> dict:
    try:
        value = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise SystemExit(f"{path}: expected a JSON object")
    return value


def metric_map(branch: dict) -> dict[str, dict]:
    assessments = branch.get("serviceAssessments")
    if not isinstance(assessments, list):
        raise SystemExit(f"{branch.get('branchId')}: serviceAssessments must be a list")
    result = {}
    for item in assessments:
        metric_id = item.get("metricId")
        if not isinstance(metric_id, str) or metric_id in result:
            raise SystemExit(f"{branch.get('branchId')}: invalid/duplicate metric ID")
        result[metric_id] = item
    return result


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: interval_simplex_reference.py CASE.json REPORT.json")
    case = load_json(sys.argv[1])
    report = load_json(sys.argv[2])
    branches = case["probabilityEnvelope"]["branches"]
    service_branches = case["serviceOutcomeManifest"]["branches"]
    if len(branches) != 3 or len(service_branches) != 3:
        raise SystemExit("v0.1 independent reference requires exactly three mutually exclusive branches")

    branch_ids = [item["branchId"] for item in branches]
    if branch_ids != [item["branchId"] for item in service_branches]:
        raise SystemExit("probability and service branch order/identity mismatch")

    point = [int(item["pointBasisPoints"]) for item in branches]
    lower = [int(item["lowerBasisPoints"]) for item in branches]
    upper = [int(item["upperBasisPoints"]) for item in branches]
    if sum(point) != TOTAL or sum(lower) > TOTAL or sum(upper) < TOTAL:
        raise SystemExit("invalid point distribution or infeasible interval simplex")
    if any(not (lo <= p <= hi) for lo, p, hi in zip(lower, point, upper)):
        raise SystemExit("point probability outside an interval")

    service_by_branch = [metric_map(item) for item in service_branches]
    metric_ids = set(service_by_branch[0])
    if any(set(items) != metric_ids for items in service_by_branch[1:]):
        raise SystemExit("service metric set differs by branch")

    point_expected = {metric: 0.0 for metric in metric_ids}
    robust_expected = {metric: -math.inf for metric in metric_ids}

    for metric in metric_ids:
        losses = [float(items[metric]["unservedQuantity"]) for items in service_by_branch]
        point_expected[metric] = sum(probability * loss for probability, loss in zip(point, losses)) / TOTAL

    # Exhaust all integer probability vectors in the three declared intervals.
    # p3 is fixed by conservation, so this is a complete finite enumeration.
    examined = 0
    for p1 in range(lower[0], upper[0] + 1):
        for p2 in range(lower[1], upper[1] + 1):
            p3 = TOTAL - p1 - p2
            if p3 < lower[2] or p3 > upper[2]:
                continue
            examined += 1
            vector = (p1, p2, p3)
            for metric in metric_ids:
                losses = [float(items[metric]["unservedQuantity"]) for items in service_by_branch]
                expected = sum(probability * loss for probability, loss in zip(vector, losses)) / TOTAL
                robust_expected[metric] = max(robust_expected[metric], expected)
    if examined == 0:
        raise SystemExit("no feasible integer basis-point vectors found")

    report_results = {item["metricId"]: item for item in report.get("intervalWorstCaseMetricResults", [])}
    if set(report_results) != metric_ids:
        raise SystemExit("Rust report metric IDs differ from the input manifest")

    risk_limits = {item["metricId"]: item for item in case["riskLimitManifest"]["limits"]}
    for metric in sorted(metric_ids):
        actual = report_results[metric]
        if abs(float(actual["pointExpectedUnservedQuantity"]) - point_expected[metric]) > TOLERANCE:
            raise SystemExit(f"{metric}: point expected value mismatch")
        if abs(float(actual["robustExpectedUnservedQuantity"]) - robust_expected[metric]) > TOLERANCE:
            raise SystemExit(f"{metric}: robust expectation mismatch")
        if actual["unit"] != risk_limits[metric]["unit"]:
            raise SystemExit(f"{metric}: report/risk-limit unit mismatch")

    floors = [
        (branch["branchId"], metric["metricId"])
        for branch in service_branches
        for metric in branch["serviceAssessments"]
        if float(metric["serviceFractionLowerBound"]) + TOLERANCE < float(metric["minimumServiceFloor"])
    ]
    if len(floors) != len(report.get("floorViolations", [])):
        raise SystemExit("floor violation count differs from independent reference")

    expected_status = (
        "inconclusive_service_evidence"
        if report.get("inconclusiveReasons")
        else "inadmissible_service_floor_breach"
        if floors
        else "inadmissible_robust_risk_budget_breach"
        if any(robust_expected[m] > float(risk_limits[m]["maximumRobustExpectedUnservedQuantity"]) + TOLERANCE for m in metric_ids)
        else "service_risk_certificate_consistent"
    )
    if report.get("resultStatus") != expected_status:
        raise SystemExit(f"resultStatus mismatch: expected {expected_status}, got {report.get('resultStatus')}")

    print(json.dumps({
        "status": "reference_match",
        "integer_probability_vectors_examined": examined,
        "metrics_verified": len(metric_ids),
        "point_expected": point_expected,
        "robust_expected": robust_expected,
        "expected_result_status": expected_status,
    }, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
