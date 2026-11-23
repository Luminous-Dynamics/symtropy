#!/usr/bin/env python3
"""Independent standard-library oracle for the exact small-instance planner."""

import hashlib
import itertools
import json
import sys
from fractions import Fraction
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-truss-inspectability.planning-corpus.v0.1"
BENCHMARK_ID = "ORBITAL-TRUSS-PLANNING-001"
RECEIPT_SCHEMA = "luminous.orbital-truss-inspectability.planning-corpus-receipt.v0.1"
CASE_RECEIPT_SCHEMA = "luminous.orbital-truss-inspectability.planning-receipt.v0.1"
CLAIM_CEILING = "synthetic_2d_geometric_observability_and_view_selection_only"
CASE_COUNT = 5


def problem_bytes(problem):
    return json.dumps(problem, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def coverage_domains(problem, selected, robust):
    indices = {feature["feature_id"]: i for i, feature in enumerate(problem["features"])}
    result = [set() for _ in problem["features"]]
    for view, include in zip(problem["views"], selected):
        if include:
            field = "robust_feature_ids" if robust else "nominal_feature_ids"
            for feature_id in view[field]:
                result[indices[feature_id]].add(view["failure_domain_id"])
    return result


def covered(domains, features):
    return all(len(values) >= feature["required_failure_domains"]
               for values, feature in zip(domains, features))


def make_solution(problem, selected):
    domains = coverage_domains(problem, selected, True)
    missing = [f["feature_id"] for values, f in zip(domains, problem["features"])
               if len(values) < f["required_failure_domains"]]
    ids = sorted(v["view_id"] for v, take in zip(problem["views"], selected) if take)
    cost = sum(v["cost_units"] for v, take in zip(problem["views"], selected) if take)
    return {
        "selected_view_ids": ids,
        "total_cost_units": cost,
        "selected_view_count": len(ids),
        "robustly_covered": not missing,
        "uncovered_feature_ids": missing,
    }


def key(plan):
    return plan["total_cost_units"], plan["selected_view_count"], plan["selected_view_ids"]


def greedy(problem, ratio):
    views = problem["views"]
    features = problem["features"]
    chosen = [False] * len(views)
    total_cost = 0
    while True:
        current = coverage_domains(problem, chosen, True)
        if covered(current, features) or sum(chosen) >= problem["max_selected_views"]:
            break
        choices = []
        for index, view in enumerate(views):
            cost = view["cost_units"]
            if chosen[index] or total_cost + cost > problem["max_total_cost_units"]:
                continue
            gain = 0
            for feature_id in view["robust_feature_ids"]:
                fi = next(i for i, f in enumerate(features) if f["feature_id"] == feature_id)
                need = features[fi]["required_failure_domains"]
                if len(current[fi]) < need and view["failure_domain_id"] not in current[fi]:
                    gain += 1
            if gain:
                choices.append((Fraction(gain, cost) if ratio else Fraction(0, 1),
                                cost, view["view_id"], index))
        if not choices:
            break
        best = min(choices, key=lambda item: (-item[0], item[1], item[2]) if ratio
                   else (item[1], item[2]))
        chosen[best[3]] = True
        total_cost += views[best[3]]["cost_units"]
    return make_solution(problem, chosen)


def solve(problem):
    views = problem["views"]
    features = problem["features"]
    if not 1 <= len(views) <= 20:
        raise ValueError("PLANNING_PROBLEM_BOUNDS_INVALID")

    subsets = eligible = nominal_count = robust_count = 0
    feasible = []
    for size in range(len(views) + 1):
        for indices in itertools.combinations(range(len(views)), size):
            selected = [i in indices for i in range(len(views))]
            subsets += 1
            total = sum(v["cost_units"] for v, take in zip(views, selected) if take)
            if size > problem["max_selected_views"] or total > problem["max_total_cost_units"]:
                continue
            eligible += 1
            if covered(coverage_domains(problem, selected, False), features):
                nominal_count += 1
            if covered(coverage_domains(problem, selected, True), features):
                robust_count += 1
                feasible.append(make_solution(problem, selected))

    all_views = [True] * len(views)
    nominal_all = covered(coverage_domains(problem, all_views, False), features)
    robust_all = covered(coverage_domains(problem, all_views, True), features)
    candidate_disposition = "INFEASIBLE" if not nominal_all else (
        "INDETERMINATE" if not robust_all else "FEASIBLE"
    )

    feasible.sort(key=key)
    optimum = feasible[0] if feasible else None
    best_by_cost = {}
    for plan in feasible:
        cost = plan["total_cost_units"]
        if cost not in best_by_cost or key(plan) < key(best_by_cost[cost]):
            best_by_cost[cost] = plan
    frontier = []
    best_count = 2**64 - 1
    for cost in sorted(best_by_cost):
        plan = best_by_cost[cost]
        if plan["selected_view_count"] < best_count:
            best_count = plan["selected_view_count"]
            frontier.append(plan)

    greedy_plan = greedy(problem, True)
    cheap_plan = greedy(problem, False)
    if optimum is not None:
        planning_disposition = "FEASIBLE"
    elif candidate_disposition == "FEASIBLE":
        planning_disposition = "BUDGET_INFEASIBLE"
    else:
        planning_disposition = candidate_disposition
    optimum_cost = optimum["total_cost_units"] if optimum is not None else None

    return {
        "schema": CASE_RECEIPT_SCHEMA,
        "case_id": problem["case_id"],
        "coordinate_frame_id": problem["coordinate_frame_id"],
        "input_sha256": hashlib.sha256(problem_bytes(problem)).hexdigest(),
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM_CEILING,
        "candidate_geometry_disposition": candidate_disposition,
        "planning_disposition": planning_disposition,
        "candidate_view_count": len(views),
        "max_selected_views": problem["max_selected_views"],
        "max_total_cost_units": problem["max_total_cost_units"],
        "subsets_evaluated": subsets,
        "budget_eligible_subsets": eligible,
        "nominally_feasible_subsets": nominal_count,
        "robustly_feasible_subsets": robust_count,
        "exact_optimum": optimum,
        "pareto_frontier": frontier,
        "greedy_next_best_view": greedy_plan,
        "cheapest_first": cheap_plan,
        "greedy_cost_gap_units": (
            greedy_plan["total_cost_units"] - optimum_cost
            if optimum is not None and greedy_plan["robustly_covered"] else None
        ),
        "cheapest_first_cost_gap_units": (
            cheap_plan["total_cost_units"] - optimum_cost
            if optimum is not None and cheap_plan["robustly_covered"] else None
        ),
        "authority_granted": False,
        "structural_acceptance_authorized": False,
        "flight_authorized": False,
        "physical_validation": "NOT_ASSESSED",
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: planning_reference_oracle.py <planning-corpus.json> <40-hex-source-commit>")
    raw = Path(sys.argv[1]).read_bytes()
    source_commit = sys.argv[2]
    if len(source_commit) != 40 or any(c not in "0123456789abcdefABCDEF" for c in source_commit):
        raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw)
    if (corpus.get("schema") != CORPUS_SCHEMA or corpus.get("benchmark_id") != BENCHMARK_ID
            or corpus.get("evidence_class") != "synthetic" or corpus.get("claim_ceiling") != CLAIM_CEILING
            or len(corpus.get("cases", [])) != CASE_COUNT):
        raise SystemExit("PLANNING_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")
    identifiers = [case["case_id"] for case in corpus["cases"]]
    if len(identifiers) != len(set(identifiers)):
        raise SystemExit("PLANNING_CORPUS_CASE_ID_INVALID")

    cases = []
    matches = 0
    for case in corpus["cases"]:
        if not case["case_id"] or case["problem"]["case_id"] != case["case_id"]:
            raise SystemExit("PLANNING_CORPUS_CASE_ID_INVALID")
        receipt = solve(case["problem"])
        geometry_match = receipt["candidate_geometry_disposition"] == case["expected_candidate_geometry_disposition"]
        planning_match = receipt["planning_disposition"] == case["expected_planning_disposition"]
        matches += int(geometry_match and planning_match)
        cases.append({
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
        "case_count": len(cases),
        "expected_outcome_matches": matches,
        "expected_outcome_mismatches": len(cases) - matches,
        "authority_granted": False,
        "physical_validation": "NOT_ASSESSED",
        "cases": cases,
    }
    print(json.dumps(output, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
