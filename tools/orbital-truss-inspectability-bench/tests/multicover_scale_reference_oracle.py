#!/usr/bin/env python3
"""Independent standard-library oracle for the sparse large-pool k-multicover benchmark."""

import hashlib
import json
import sys
from fractions import Fraction
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-truss.multicover-scale-corpus.v0.1"
BENCHMARK_ID = "ORBITAL-MULTICOVER-SCALE-001"
RECEIPT_SCHEMA = "luminous.orbital-truss.multicover-scale-receipt.v0.1"
CLAIM_CEILING = "synthetic_sparse_nominal_k_multicover_with_robust_diagnostics_only"
FIXED_SCALE = 1_000_000_000_000_000_000
EXP_NEGATIVE_ONE_FIXED = 367_879_441_171_442_321
MAX_NORMALIZED_EXPONENT = 64
MAX_VIEWS = 100_000
MAX_FEATURES = 250_000
MAX_EDGES = 50_000_000
MAX_K = 10_000


def decay_table():
    values = [FIXED_SCALE]
    for _ in range(MAX_NORMALIZED_EXPONENT):
        values.append(values[-1] * EXP_NEGATIVE_ONE_FIXED // FIXED_SCALE)
    return values


def validate(problem):
    features = problem["features"]
    views = problem["views"]
    if (not problem["case_id"] or not problem["coordinate_frame_id"]
            or not features or len(features) > MAX_FEATURES or len(views) > MAX_VIEWS):
        raise ValueError("MULTICOVER_PROBLEM_BOUNDS_INVALID")
    ids = [item["feature_id"] for item in features]
    if len(ids) != len(set(ids)) or any(
        not feature_id or feature["requested_k"] < 1 or feature["requested_k"] > MAX_K
        for feature_id, feature in zip(ids, features)
    ):
        raise ValueError("MULTICOVER_FEATURE_INVALID")
    view_ids = [view["view_id"] for view in views]
    if len(view_ids) != len(set(view_ids)) or any(
        not view_id or not view["failure_domain_id"]
        for view_id, view in zip(view_ids, views)
    ):
        raise ValueError("MULTICOVER_VIEW_IDENTITY_INVALID")
    known = set(ids)
    nominal_edges = robust_edges = 0
    for view in views:
        nominal = view["nominal_feature_ids"]
        robust = view["robust_feature_ids"]
        if (len(nominal) != len(set(nominal)) or len(robust) != len(set(robust))
                or not set(nominal).issubset(known)
                or not set(robust).issubset(set(nominal))):
            raise ValueError("MULTICOVER_VIEW_COVERAGE_INVALID")
        nominal_edges += len(nominal)
        robust_edges += len(robust)
    if nominal_edges > MAX_EDGES or robust_edges > MAX_EDGES:
        raise ValueError("MULTICOVER_VISIBILITY_EDGE_LIMIT_EXCEEDED")


def index_problem(problem):
    features = problem["features"]
    views = problem["views"]
    index = {feature["feature_id"]: i for i, feature in enumerate(features)}
    nominal_by_view = []
    robust_by_view = []
    feature_to_views = [[] for _ in features]
    nominal_counts = [0] * len(features)
    robust_counts = [0] * len(features)
    nominal_domains = [set() for _ in features]
    robust_domains = [set() for _ in features]

    for view_index, view in enumerate(views):
        nominal = sorted(index[name] for name in view["nominal_feature_ids"])
        robust = sorted(index[name] for name in view["robust_feature_ids"])
        nominal_by_view.append(nominal)
        robust_by_view.append(robust)
        for feature_index in nominal:
            nominal_counts[feature_index] += 1
            nominal_domains[feature_index].add(view["failure_domain_id"])
            feature_to_views[feature_index].append(view_index)
        for feature_index in robust:
            robust_counts[feature_index] += 1
            robust_domains[feature_index].add(view["failure_domain_id"])

    return {
        "feature_ids": [feature["feature_id"] for feature in features],
        "view_ids": [view["view_id"] for view in views],
        "requested_k": [feature["requested_k"] for feature in features],
        "domains": [view["failure_domain_id"] for view in views],
        "nominal_by_view": nominal_by_view,
        "robust_by_view": robust_by_view,
        "feature_to_views": feature_to_views,
        "nominal_counts": nominal_counts,
        "robust_counts": robust_counts,
        "nominal_domains": nominal_domains,
        "robust_domains": robust_domains,
    }


def paper_style_greedy(indexed):
    requested = indexed["requested_k"]
    target = [min(k, count) for k, count in zip(requested, indexed["nominal_counts"])]
    remaining = list(indexed["nominal_counts"])
    covered = [0] * len(requested)
    selected = [False] * len(indexed["view_ids"])
    selection_order = []
    exp = decay_table()
    view_ids = indexed["view_ids"]
    nominal_by_view = indexed["nominal_by_view"]

    while any(actual < required for actual, required in zip(covered, target)):
        sigma = [0] * len(requested)
        deficits = [0] * len(requested)
        min_sigma = None
        for i, need in enumerate(target):
            deficit = max(0, need - covered[i])
            deficits[i] = deficit
            if deficit:
                sigma[i] = max(0, remaining[i] - deficit)
                min_sigma = sigma[i] if min_sigma is None else min(min_sigma, sigma[i])

        weights = [0] * len(requested)
        for i, deficit in enumerate(deficits):
            if not deficit:
                continue
            delta = sigma[i] - min_sigma
            exp_weight = exp[delta] if delta < len(exp) else 0
            weights[i] = exp_weight * deficit * deficit // (requested[i] * requested[i])

        best_index = None
        best_score = 0
        for view_index, feature_indices in enumerate(nominal_by_view):
            if selected[view_index]:
                continue
            score = sum(weights[feature_index] for feature_index in feature_indices)
            if score > best_score or (
                score > 0 and score == best_score
                and (best_index is None or view_ids[view_index] < view_ids[best_index])
            ):
                best_index = view_index
                best_score = score
        if best_index is None or best_score == 0:
            raise ValueError("MULTICOVER_GREEDY_STALLED_WITH_UNMET_CAPPED_TARGET")

        selected[best_index] = True
        selection_order.append(best_index)
        for feature_index in nominal_by_view[best_index]:
            covered[feature_index] += 1
            remaining[feature_index] -= 1

    removed = []
    changed = True
    while changed:
        changed = False
        for view_index in selection_order:
            if not selected[view_index]:
                continue
            if all(
                covered[feature_index] - 1 >= target[feature_index]
                for feature_index in nominal_by_view[view_index]
            ):
                selected[view_index] = False
                removed.append(view_index)
                for feature_index in nominal_by_view[view_index]:
                    covered[feature_index] -= 1
                changed = True
                break
    return selected, selection_order, removed


def build_dual(indexed):
    requested = indexed["requested_k"]
    target = [min(k, count) for k, count in zip(requested, indexed["nominal_counts"])]
    adjacency = indexed["feature_to_views"]
    feature_ids = indexed["feature_ids"]
    views = len(indexed["view_ids"])

    def one_order(name):
        order = [i for i, need in enumerate(target) if need > 0 and adjacency[i]]
        if name == "DEMAND_PER_CANDIDATE_DESC":
            order.sort(key=lambda i: (-Fraction(target[i], len(adjacency[i])), feature_ids[i]))
        elif name == "CANDIDATE_SCARCITY_ASC":
            order.sort(key=lambda i: (len(adjacency[i]), feature_ids[i]))
        else:
            order.sort(key=lambda i: (-target[i], feature_ids[i]))

        weights = [0] * len(target)
        loads = [0] * views
        objective = 0
        for feature_index in order:
            slack = min(FIXED_SCALE - loads[view_index] for view_index in adjacency[feature_index])
            weights[feature_index] = slack
            objective += target[feature_index] * slack
            for view_index in adjacency[feature_index]:
                loads[view_index] += slack
        return {
            "ordering": name,
            "objective": objective,
            "weights": weights,
            "max_load": max(loads, default=0),
        }

    candidates = [
        one_order("DEMAND_PER_CANDIDATE_DESC"),
        one_order("CANDIDATE_SCARCITY_ASC"),
        one_order("DEMAND_DESC"),
    ]
    best = min(candidates, key=lambda item: (-item["objective"], item["ordering"]))
    floor = best["objective"] // FIXED_SCALE
    ceil = floor + int(best["objective"] % FIXED_SCALE != 0)
    return {
        "method": "FEASIBLE_SET_MULTICOVER_LP_DUAL_V0_1",
        "ordering": best["ordering"],
        "objective_numerator": best["objective"],
        "objective_denominator": FIXED_SCALE,
        "lower_bound_floor": floor,
        "lower_bound_ceiling": ceil,
        "maximum_candidate_constraint_load_numerator": best["max_load"],
        "candidate_constraint_count": views,
        "dual_feature_weight_numerators": best["weights"],
        "dual_constraints_verified": best["max_load"] <= FIXED_SCALE,
        "minimum_cardinality_optimum_certified": False,
    }


def build_feature_coverage(indexed, selected):
    count = len(indexed["feature_ids"])
    selected_nominal = [0] * count
    selected_robust = [0] * count
    selected_nominal_domains = [set() for _ in range(count)]
    selected_robust_domains = [set() for _ in range(count)]
    for view_index, include in enumerate(selected):
        if not include:
            continue
        for feature_index in indexed["nominal_by_view"][view_index]:
            selected_nominal[feature_index] += 1
            selected_nominal_domains[feature_index].add(indexed["domains"][view_index])
        for feature_index in indexed["robust_by_view"][view_index]:
            selected_robust[feature_index] += 1
            selected_robust_domains[feature_index].add(indexed["domains"][view_index])

    reports = []
    for i, feature_id in enumerate(indexed["feature_ids"]):
        requested = indexed["requested_k"][i]
        available = indexed["nominal_counts"][i]
        attainable = min(requested, available)
        reports.append({
            "feature_id": feature_id,
            "requested_k": requested,
            "available_nominal_candidate_views": available,
            "available_robust_candidate_views": indexed["robust_counts"][i],
            "attainable_capped_target": attainable,
            "selected_nominal_views": selected_nominal[i],
            "selected_robust_views": selected_robust[i],
            "requested_k_met": selected_nominal[i] >= requested,
            "attainable_target_met": selected_nominal[i] >= attainable,
            "robust_requested_k_met": selected_robust[i] >= requested,
            "nominal_failure_domain_count_available": len(indexed["nominal_domains"][i]),
            "robust_failure_domain_count_available": len(indexed["robust_domains"][i]),
            "nominal_failure_domain_count_selected": len(selected_nominal_domains[i]),
            "robust_failure_domain_count_selected": len(selected_robust_domains[i]),
        })
    return reports


def solve(problem):
    validate(problem)
    indexed = index_problem(problem)
    selected, selection_order, removed = paper_style_greedy(indexed)
    dual = build_dual(indexed)
    coverage = build_feature_coverage(indexed, selected)
    selected_ids = sorted(
        view_id for view_id, include in zip(indexed["view_ids"], selected) if include
    )
    selection_order_ids = [indexed["view_ids"][index] for index in selection_order]
    removed_ids = [indexed["view_ids"][index] for index in removed]
    selected_count = len(selected_ids)
    if dual["lower_bound_ceiling"] > selected_count:
        raise ValueError("MULTICOVER_DUAL_LOWER_BOUND_EXCEEDS_FEASIBLE_INCUMBENT")

    candidate_k = all(
        item["available_nominal_candidate_views"] >= item["requested_k"] for item in coverage
    )
    selected_k = all(item["requested_k_met"] for item in coverage)
    capped = all(item["attainable_target_met"] for item in coverage)
    robust = all(item["robust_requested_k_met"] for item in coverage)
    excess = selected_count - dual["lower_bound_ceiling"]
    lower = dual["lower_bound_ceiling"]
    relative = ((excess * 10_000 + lower - 1) // lower) if lower > 0 else None
    return {
        "schema": RECEIPT_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "case_id": problem["case_id"],
        "coordinate_frame_id": problem["coordinate_frame_id"],
        "source_commit": "",
        "input_sha256": "",
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM_CEILING,
        "candidate_view_count": len(problem["views"]),
        "feature_count": len(problem["features"]),
        "nominal_visibility_edge_count": sum(len(row) for row in indexed["nominal_by_view"]),
        "robust_visibility_edge_count": sum(len(row) for row in indexed["robust_by_view"]),
        "selected_view_ids": selected_ids,
        "selection_order_view_ids": selection_order_ids,
        "cleanup_removed_view_ids": removed_ids,
        "selected_view_count": selected_count,
        "candidate_pool_requested_k_fully_available": candidate_k,
        "selected_requested_k_fully_covered": selected_k,
        "selected_attainable_targets_fully_covered": capped,
        "selected_robust_requested_k_fully_covered": robust,
        "feature_coverage": coverage,
        "dual_bound": dual,
        "selected_minus_dual_lower_bound": excess,
        "relative_excess_over_dual_lower_bound_basis_points": relative,
        "authority_granted": False,
        "physical_validation": "NOT_ASSESSED",
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit(
            "usage: multicover_scale_reference_oracle.py <multicover-corpus.json> <40-hex-source-commit>"
        )
    raw = Path(sys.argv[1]).read_bytes()
    source_commit = sys.argv[2]
    if len(source_commit) != 40 or any(c not in "0123456789abcdefABCDEF" for c in source_commit):
        raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw)
    if (corpus.get("schema") != CORPUS_SCHEMA
            or corpus.get("benchmark_id") != BENCHMARK_ID
            or corpus.get("evidence_class") != "synthetic"
            or corpus.get("claim_ceiling") != CLAIM_CEILING):
        raise SystemExit("MULTICOVER_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")
    receipt = solve(corpus["problem"])
    receipt["source_commit"] = source_commit.lower()
    receipt["input_sha256"] = hashlib.sha256(raw).hexdigest()
    print(json.dumps(receipt, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
