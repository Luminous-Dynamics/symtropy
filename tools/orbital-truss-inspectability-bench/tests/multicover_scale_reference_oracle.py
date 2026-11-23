#!/usr/bin/env python3
"""Independent standard-library oracle for the sparse large-pool k-multicover benchmark."""

import hashlib
import json
import sys
from fractions import Fraction
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-truss.multicover-scale-corpus.v0.2"
BENCHMARK_ID = "ORBITAL-MULTICOVER-SCALE-002"
RECEIPT_SCHEMA = "luminous.orbital-truss.multicover-scale-receipt.v0.3"
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
    robust_feature_to_views = [[] for _ in features]
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
            robust_feature_to_views[feature_index].append(view_index)

    return {
        "feature_ids": [feature["feature_id"] for feature in features],
        "view_ids": [view["view_id"] for view in views],
        "vertical_order_ranks": [view["vertical_order_rank"] for view in views],
        "requested_k": [feature["requested_k"] for feature in features],
        "domains": [view["failure_domain_id"] for view in views],
        "nominal_by_view": nominal_by_view,
        "robust_by_view": robust_by_view,
        "feature_to_views": feature_to_views,
        "robust_feature_to_views": robust_feature_to_views,
        "nominal_counts": nominal_counts,
        "robust_counts": robust_counts,
        "nominal_domains": nominal_domains,
        "robust_domains": robust_domains,
    }


def paper_style_greedy(indexed, robust_relation=False):
    requested = indexed["requested_k"]
    relation = "robust" if robust_relation else "nominal"
    candidate_counts = indexed[f"{relation}_counts"]
    by_view = indexed[f"{relation}_by_view"]
    target = [min(k, count) for k, count in zip(requested, candidate_counts)]
    remaining = list(candidate_counts)
    covered = [0] * len(requested)
    selected = [False] * len(indexed["view_ids"])
    selection_order = []
    exp = decay_table()
    view_ids = indexed["view_ids"]

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
        for view_index, feature_indices in enumerate(by_view):
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
        for feature_index in by_view[best_index]:
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
                for feature_index in by_view[view_index]
            ):
                selected[view_index] = False
                removed.append(view_index)
                for feature_index in by_view[view_index]:
                    covered[feature_index] -= 1
                changed = True
                break
    return selected, selection_order, removed


def build_dual(indexed, robust_relation=False):
    requested = indexed["requested_k"]
    relation = "robust" if robust_relation else "nominal"
    candidate_counts = indexed[f"{relation}_counts"]
    adjacency = indexed["robust_feature_to_views" if robust_relation else "feature_to_views"]
    target = [min(k, count) for k, count in zip(requested, candidate_counts)]
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
        "method": (
            "FEASIBLE_ROBUST_SET_MULTICOVER_LP_DUAL_V0_1"
            if robust_relation else "FEASIBLE_NOMINAL_SET_MULTICOVER_LP_DUAL_V0_1"
        ),
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
        available_nominal = indexed["nominal_counts"][i]
        available_robust = indexed["robust_counts"][i]
        attainable = min(requested, available_nominal)
        robust_attainable = min(requested, available_robust)
        reports.append({
            "feature_id": feature_id,
            "requested_k": requested,
            "available_nominal_candidate_views": available_nominal,
            "available_robust_candidate_views": available_robust,
            "attainable_capped_target": attainable,
            "robust_attainable_capped_target": robust_attainable,
            "selected_nominal_views": selected_nominal[i],
            "selected_robust_views": selected_robust[i],
            "requested_k_met": selected_nominal[i] >= requested,
            "attainable_target_met": selected_nominal[i] >= attainable,
            "robust_requested_k_met": selected_robust[i] >= requested,
            "robust_attainable_target_met": selected_robust[i] >= robust_attainable,
            "nominal_failure_domain_count_available": len(indexed["nominal_domains"][i]),
            "robust_failure_domain_count_available": len(indexed["robust_domains"][i]),
            "nominal_failure_domain_count_selected": len(selected_nominal_domains[i]),
            "robust_failure_domain_count_selected": len(selected_robust_domains[i]),
        })
    return reports


def overlap_pruning(indexed, threshold_percent, ordering):
    if ordering == "VERTICAL_BOTTOM_TO_TOP":
        order = sorted(
            range(len(indexed["view_ids"])),
            key=lambda index: (
                indexed["vertical_order_ranks"][index],
                indexed["view_ids"][index],
            ),
        )
    elif ordering == "VISIBILITY_DESCENDING":
        order = sorted(
            range(len(indexed["view_ids"])),
            key=lambda index: (
                -len(indexed["nominal_by_view"][index]),
                indexed["view_ids"][index],
            ),
        )
    else:
        raise ValueError("OVERLAP_ORDERING_UNSUPPORTED")

    covered = [False] * len(indexed["feature_ids"])
    selected = [False] * len(indexed["view_ids"])
    selection_order = []
    zero_visibility_skipped = 0
    for view_index in order:
        visible = indexed["nominal_by_view"][view_index]
        if not visible:
            zero_visibility_skipped += 1
            continue
        overlap_count = sum(covered[feature_index] for feature_index in visible)
        if overlap_count * 100 < threshold_percent * len(visible):
            selected[view_index] = True
            selection_order.append(view_index)
            for feature_index in visible:
                covered[feature_index] = True

    coverage = build_feature_coverage(indexed, selected)
    selected_ids = sorted(
        view_id for view_id, include in zip(indexed["view_ids"], selected) if include
    )
    selection_order_ids = [
        indexed["view_ids"][index] for index in selection_order
    ]
    return {
        "ordering": ordering,
        "threshold_percent": threshold_percent,
        "selected_view_ids": selected_ids,
        "selection_order_view_ids": selection_order_ids,
        "selected_view_count": len(selected_ids),
        "zero_visibility_candidates_skipped": zero_visibility_skipped,
        "full_nominal_union_covered": all(
            item["selected_nominal_views"] > 0 for item in coverage
        ),
        "requested_k_fully_covered": all(item["requested_k_met"] for item in coverage),
        "attainable_capped_targets_fully_covered": all(
            item["attainable_target_met"] for item in coverage
        ),
        "feature_coverage": coverage,
    }


def solve(problem):
    validate(problem)
    indexed = index_problem(problem)
    selected, selection_order, removed = paper_style_greedy(indexed, robust_relation=False)
    dual = build_dual(indexed, robust_relation=False)
    coverage = build_feature_coverage(indexed, selected)
    robust_selected, robust_order, robust_removed = paper_style_greedy(
        indexed, robust_relation=True
    )
    robust_dual = build_dual(indexed, robust_relation=True)
    robust_coverage = build_feature_coverage(indexed, robust_selected)
    overlap_baselines = [
        overlap_pruning(indexed, threshold, ordering)
        for threshold in (50, 70)
        for ordering in ("VERTICAL_BOTTOM_TO_TOP", "VISIBILITY_DESCENDING")
    ]

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
    robust_diag = all(item["robust_requested_k_met"] for item in coverage)
    excess = selected_count - dual["lower_bound_ceiling"]
    lower = dual["lower_bound_ceiling"]
    relative = ((excess * 10_000 + lower - 1) // lower) if lower > 0 else None
    dual["minimum_cardinality_optimum_certified"] = (
        dual["dual_constraints_verified"] and capped
        and dual["lower_bound_ceiling"] == selected_count
    )

    robust_ids = sorted(
        view_id for view_id, include in zip(indexed["view_ids"], robust_selected) if include
    )
    robust_order_ids = [indexed["view_ids"][index] for index in robust_order]
    robust_removed_ids = [indexed["view_ids"][index] for index in robust_removed]
    robust_count = len(robust_ids)
    if robust_dual["lower_bound_ceiling"] > robust_count:
        raise ValueError("MULTICOVER_ROBUST_DUAL_LOWER_BOUND_EXCEEDS_FEASIBLE_INCUMBENT")
    robust_candidate_k = all(
        item["available_robust_candidate_views"] >= item["requested_k"]
        for item in robust_coverage
    )
    robust_selected_k = all(item["robust_requested_k_met"] for item in robust_coverage)
    robust_capped = all(item["robust_attainable_target_met"] for item in robust_coverage)
    if not robust_capped:
        raise ValueError("MULTICOVER_ROBUST_PRIMAL_INCUMBENT_DOES_NOT_MEET_CAPPED_TARGETS")
    robust_dual["minimum_cardinality_optimum_certified"] = (
        robust_dual["dual_constraints_verified"] and robust_capped
        and robust_dual["lower_bound_ceiling"] == robust_count
    )
    nominal_set = set(selected_ids)
    robust_set = set(robust_ids)
    robust_excess = robust_count - robust_dual["lower_bound_ceiling"]
    robust_lower = robust_dual["lower_bound_ceiling"]
    robust_relative = (
        (robust_excess * 10_000 + robust_lower - 1) // robust_lower
        if robust_lower > 0 else None
    )
    robust_companion = {
        "algorithm": "ROBUST_VISIBILITY_GLOBALLY_INFORMED_GREEDY_K_VARIANT_V0_1",
        "selected_view_ids": robust_ids,
        "selection_order_view_ids": robust_order_ids,
        "cleanup_removed_view_ids": robust_removed_ids,
        "nominal_only_view_ids": sorted(nominal_set - robust_set),
        "robust_only_view_ids": sorted(robust_set - nominal_set),
        "nominal_selected_view_count": selected_count,
        "robust_selected_view_count": robust_count,
        "view_count_delta_vs_nominal": robust_count - selected_count,
        "candidate_pool_requested_k_fully_available": robust_candidate_k,
        "selected_requested_k_fully_covered": robust_selected_k,
        "selected_attainable_targets_fully_covered": robust_capped,
        "full_requested_k_comparable_with_nominal": (
            candidate_k and selected_k and robust_candidate_k and robust_selected_k
        ),
        "feature_coverage": robust_coverage,
        "dual_bound": robust_dual,
        "selected_minus_dual_lower_bound": robust_excess,
        "relative_excess_over_dual_lower_bound_basis_points": robust_relative,
    }

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
        "selected_robust_requested_k_fully_covered": robust_diag,
        "feature_coverage": coverage,
        "robust_selection_companion": robust_companion,
        "overlap_pruning_baselines": overlap_baselines,
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
