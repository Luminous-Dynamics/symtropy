#!/usr/bin/env python3
"""Independent standard-library oracle for planning corpus v0.2."""

import hashlib
import itertools
import json
import sys
from fractions import Fraction
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-truss-inspectability.planning-corpus.v0.2"
BENCHMARK_ID = "ORBITAL-TRUSS-PLANNING-002"
RECEIPT_SCHEMA = "luminous.orbital-truss-inspectability.planning-corpus-receipt.v0.2"
CASE_RECEIPT_SCHEMA = "luminous.orbital-truss-inspectability.planning-receipt.v0.2"
CLAIM_CEILING = "synthetic_2d_geometric_observability_and_view_selection_only"
CASE_COUNT = 8
MAX_EXACT_VIEWS = 20


def problem_bytes(problem):
    return json.dumps(problem, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def coverage_records(problem, selected):
    feature_index = {feature["feature_id"]: i for i, feature in enumerate(problem["features"])}
    nominal_views = [0] * len(problem["features"])
    robust_views = [0] * len(problem["features"])
    nominal_domains = [set() for _ in problem["features"]]
    robust_domains = [set() for _ in problem["features"]]

    for view, include in zip(problem["views"], selected):
        if not include:
            continue
        for feature_id in view["nominal_feature_ids"]:
            index = feature_index[feature_id]
            nominal_views[index] += 1
            nominal_domains[index].add(view["failure_domain_id"])
        for feature_id in view["robust_feature_ids"]:
            index = feature_index[feature_id]
            robust_views[index] += 1
            robust_domains[index].add(view["failure_domain_id"])

    return [
        {
            "feature_id": feature["feature_id"],
            "required_view_count": feature["required_view_count"],
            "nominal_view_count": nominal_views[index],
            "robust_view_count": robust_views[index],
            "required_failure_domains": feature["required_failure_domains"],
            "nominal_failure_domain_count": len(nominal_domains[index]),
            "robust_failure_domain_count": len(robust_domains[index]),
        }
        for index, feature in enumerate(problem["features"])
    ]


def deficits(records):
    return sum(
        max(0, row["required_view_count"] - row["robust_view_count"])
        + max(0, row["required_failure_domains"] - row["robust_failure_domain_count"])
        for row in records
    )


def solution(problem, selected):
    coverage = coverage_records(problem, selected)
    under_view = sorted(
        row["feature_id"] for row in coverage
        if row["robust_view_count"] < row["required_view_count"]
    )
    under_domains = sorted(
        row["feature_id"] for row in coverage
        if row["robust_failure_domain_count"] < row["required_failure_domains"]
    )
    missing = sorted(set(under_view) | set(under_domains))
    ids = sorted(view["view_id"] for view, take in zip(problem["views"], selected) if take)
    cost = sum(view["cost_units"] for view, take in zip(problem["views"], selected) if take)
    return {
        "selected_view_ids": ids,
        "total_cost_units": cost,
        "selected_view_count": len(ids),
        "robustly_covered": not missing,
        "uncovered_feature_ids": missing,
        "undercovered_view_count_feature_ids": under_view,
        "undercovered_failure_domain_feature_ids": under_domains,
        "feature_coverage": coverage,
    }


def key(plan):
    return plan["total_cost_units"], plan["selected_view_count"], plan["selected_view_ids"]


def validate_problem(problem):
    if (not problem["case_id"] or not problem["coordinate_frame_id"]
            or not problem["features"] or not problem["views"]
            or len(problem["views"]) > MAX_EXACT_VIEWS
            or problem["max_selected_views"] > len(problem["views"])):
        raise ValueError("PLANNING_PROBLEM_BOUNDS_INVALID")
    feature_ids = [feature["feature_id"] for feature in problem["features"]]
    if len(feature_ids) != len(set(feature_ids)) or any(
        not item or feature["required_view_count"] < 1 or feature["required_failure_domains"] < 1
        for item, feature in zip(feature_ids, problem["features"])
    ):
        raise ValueError("PLANNING_FEATURE_INVALID")
    view_ids = [view["view_id"] for view in problem["views"]]
    if len(view_ids) != len(set(view_ids)) or any(
        not view["view_id"] or not view["failure_domain_id"] or view["cost_units"] <= 0
        for view in problem["views"]
    ):
        raise ValueError("PLANNING_VIEW_INVALID")
    known = set(feature_ids)
    total_cost = 0
    for view in problem["views"]:
        total_cost += view["cost_units"]
        nominal = view["nominal_feature_ids"]
        robust = view["robust_feature_ids"]
        if (len(nominal) != len(set(nominal)) or len(robust) != len(set(robust))
                or not set(nominal).issubset(known) or not set(robust).issubset(set(nominal))):
            raise ValueError("PLANNING_VIEW_COVERAGE_INVALID")
    if total_cost > 2**64 - 1:
        raise ValueError("PLANNING_TOTAL_COST_OVERFLOW")


def greedy(problem, ratio):
    views = problem["views"]
    selected = [False] * len(views)
    selected_count = 0
    total_cost = 0

    while True:
        current_deficit = deficits(coverage_records(problem, selected))
        if current_deficit == 0 or selected_count >= problem["max_selected_views"]:
            break
        choices = []
        for index, view in enumerate(views):
            if selected[index] or total_cost + view["cost_units"] > problem["max_total_cost_units"]:
                continue
            selected[index] = True
            next_deficit = deficits(coverage_records(problem, selected))
            selected[index] = False
            gain = current_deficit - next_deficit
            if gain:
                choices.append((
                    Fraction(gain, view["cost_units"]) if ratio else Fraction(0, 1),
                    view["cost_units"],
                    view["view_id"],
                    index,
                    gain,
                ))
        if not choices:
            break
        if ratio:
            best = min(choices, key=lambda item: (-item[0], item[1], item[2]))
        else:
            best = min(choices, key=lambda item: (item[1], item[2]))
        index = best[3]
        selected[index] = True
        selected_count += 1
        total_cost += views[index]["cost_units"]
    return solution(problem, selected)


def solve(problem):
    validate_problem(problem)
    views = problem["views"]
    features = problem["features"]
    n = len(views)
    subsets = eligible = nominal_feasible = robust_feasible = 0
    feasible_plans = []

    for size in range(n + 1):
        for indices in itertools.combinations(range(n), size):
            selected = [index in indices for index in range(n)]
            subsets += 1
            total_cost = sum(view["cost_units"] for view, take in zip(views, selected) if take)
            if size > problem["max_selected_views"] or total_cost > problem["max_total_cost_units"]:
                continue
            eligible += 1
            coverage = coverage_records(problem, selected)
            nominal_ok = all(
                row["nominal_view_count"] >= row["required_view_count"]
                and row["nominal_failure_domain_count"] >= row["required_failure_domains"]
                for row in coverage
            )
            robust_ok = all(
                row["robust_view_count"] >= row["required_view_count"]
                and row["robust_failure_domain_count"] >= row["required_failure_domains"]
                for row in coverage
            )
            nominal_feasible += int(nominal_ok)
            if robust_ok:
                robust_feasible += 1
                feasible_plans.append(solution(problem, selected))

    all_coverage = coverage_records(problem, [True] * n)
    nominal_candidate_shortfall = any(
        row["nominal_view_count"] < row["required_view_count"]
        or row["nominal_failure_domain_count"] < row["required_failure_domains"]
        for row in all_coverage
    )
    robust_candidate_shortfall = any(
        row["robust_view_count"] < row["required_view_count"]
        or row["robust_failure_domain_count"] < row["required_failure_domains"]
        for row in all_coverage
    )
    if nominal_candidate_shortfall:
        candidate_disposition = "INFEASIBLE"
    elif robust_candidate_shortfall:
        candidate_disposition = "INDETERMINATE"
    else:
        candidate_disposition = "FEASIBLE"

    feasible_plans.sort(key=key)
    optimum = feasible_plans[0] if feasible_plans else None
    by_cost = {}
    for plan in feasible_plans:
        cost = plan["total_cost_units"]
        if cost not in by_cost or key(plan) < key(by_cost[cost]):
            by_cost[cost] = plan
    frontier = []
    best_count = 2**64 - 1
    for cost in sorted(by_cost):
        plan = by_cost[cost]
        if plan["selected_view_count"] < best_count:
            best_count = plan["selected_view_count"]
            frontier.append(plan)

    greedy_plan = greedy(problem, True)
    cheapest_plan = greedy(problem, False)
    if optimum is not None:
        disposition = "FEASIBLE"
    elif candidate_disposition == "FEASIBLE":
        disposition = "BUDGET_INFEASIBLE"
    else:
        disposition = candidate_disposition
    optimum_cost = optimum["total_cost_units"] if optimum is not None else None

    return {
        "schema": CASE_RECEIPT_SCHEMA,
        "case_id": problem["case_id"],
        "coordinate_frame_id": problem["coordinate_frame_id"],
        "input_sha256": hashlib.sha256(problem_bytes(problem)).hexdigest(),
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM_CEILING,
        "candidate_geometry_disposition": candidate_disposition,
        "planning_disposition": disposition,
        "candidate_view_count": n,
        "candidate_pool_coverage": all_coverage,
        "max_selected_views": problem["max_selected_views"],
        "max_total_cost_units": problem["max_total_cost_units"],
        "subsets_evaluated": subsets,
        "budget_eligible_subsets": eligible,
        "nominally_feasible_subsets": nominal_feasible,
        "robustly_feasible_subsets": robust_feasible,
        "exact_optimum": optimum,
        "pareto_frontier": frontier,
        "greedy_next_best_view": greedy_plan,
        "cheapest_first": cheapest_plan,
        "greedy_cost_gap_units": (
            greedy_plan["total_cost_units"] - optimum_cost
            if optimum is not None and greedy_plan["robustly_covered"] else None
        ),
        "cheapest_first_cost_gap_units": (
            cheapest_plan["total_cost_units"] - optimum_cost
            if optimum is not None and cheapest_plan["robustly_covered"] else None
        ),
        "authority_granted": False,
        "structural_acceptance_authorized": False,
        "flight_authorized": False,
        "physical_validation": "NOT_ASSESSED",
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: planning_reference_oracle_v0_2.py <planning-corpus.json> <40-hex-source-commit>")
    raw = Path(sys.argv[1]).read_bytes()
    source_commit = sys.argv[2]
    if len(source_commit) != 40 or any(char not in "0123456789abcdefABCDEF" for char in source_commit):
        raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw)
    if (corpus.get("schema") != CORPUS_SCHEMA or corpus.get("benchmark_id") != BENCHMARK_ID
            or corpus.get("evidence_class") != "synthetic" or corpus.get("claim_ceiling") != CLAIM_CEILING
            or len(corpus.get("cases", [])) != CASE_COUNT):
        raise SystemExit("PLANNING_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")

    ids = [case["case_id"] for case in corpus["cases"]]
    if len(ids) != len(set(ids)):
        raise SystemExit("PLANNING_CORPUS_CASE_ID_INVALID")
    receipts = []
    matches = 0
    for case in corpus["cases"]:
        if not case["case_id"] or case["problem"]["case_id"] != case["case_id"]:
            raise SystemExit("PLANNING_CORPUS_CASE_ID_INVALID")
        receipt = solve(case["problem"])
        geometry_match = receipt["candidate_geometry_disposition"] == case["expected_candidate_geometry_disposition"]
        planning_match = receipt["planning_disposition"] == case["expected_planning_disposition"]
        matches += int(geometry_match and planning_match)
        receipts.append({
            "case_id": case["case_id"],
            "expected_candidate_geometry_disposition": case["expected_candidate_geometry_disposition"],
            "expected_planning_disposition": case["expected_planning_disposition"],
            "candidate_geometry_matches": geometry_match,
            "planning_matches": planning_match,
            "planning": receipt,
        })
    output = {
        "schema": RECEIPT_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "source_commit": source_commit.lower(),
        "input_sha256": hashlib.sha256(raw).hexdigest(),
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM_CEILING,
        "case_count": len(receipts),
        "expected_outcome_matches": matches,
        "expected_outcome_mismatches": len(receipts) - matches,
        "authority_granted": False,
        "physical_validation": "NOT_ASSESSED",
        "cases": receipts,
    }
    print(json.dumps(output, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
