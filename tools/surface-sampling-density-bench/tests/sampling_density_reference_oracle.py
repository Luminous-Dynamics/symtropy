#!/usr/bin/env python3
"""Independent reference oracle for non-uniform surface ray sampling density."""

import hashlib
import json
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-sampling-density.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-sampling-density.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-SAMPLING-DENSITY-001"
CLAIM_CEILING = "synthetic_ray_intersection_density_accounting_on_declared_surface_elements_only"
CASE_COUNT = 12
SCALE = 1_000_000


def density_meets(hits, area_um2, target_micro):
    return hits * SCALE * SCALE >= target_micro * area_um2


def invalid_case(case, reason):
    return {
        "case_id": case["case_id"],
        "status": "INVALID_INPUT",
        "reason_codes": [reason],
        "candidate_pool_sufficient": False,
        "selected_plan_sufficient": False,
        "selected_view_budget_met": False,
        "provenance_complete": False,
        "candidate_view_count": len(case.get("candidate_views", [])),
        "selected_view_count": len(case.get("selected_view_ids", [])),
        "max_selected_views": case.get("max_selected_views", 0),
        "elements": [],
    }


def validation_reason(case):
    if (not case["case_id"].strip() or not case["coordinate_frame_id"].strip()
            or case["area_unit"] != "SQUARE_MICROMETRE"
            or case["density_unit"] != "RAYS_PER_SQUARE_MILLIMETRE"
            or not case["candidate_views"] or not case["elements"]):
        return "CASE_ID_FRAME_UNITS_OR_COLLECTIONS_INVALID"

    candidates = set(case["candidate_views"])
    if (len(candidates) != len(case["candidate_views"])
            or any(not view_id.strip() for view_id in candidates)):
        return "CANDIDATE_VIEW_IDS_DUPLICATE_OR_BLANK"

    selected = set(case["selected_view_ids"])
    if (len(selected) != len(case["selected_view_ids"])
            or any(view_id not in candidates for view_id in selected)):
        return "SELECTED_VIEW_IDS_DUPLICATE_OR_UNKNOWN"

    seen_elements = set()
    for element in case["elements"]:
        if (not element["element_id"].strip()
                or not element["mesh_revision_id"].strip()
                or element["element_id"] in seen_elements
                or element["area_um2"] == 0
                or element["minimum_density_micro_rays_per_mm2"] == 0
                or element["minimum_contributing_views"] == 0
                or element["minimum_contributing_views"] > len(case["candidate_views"])):
            return "SURFACE_ELEMENT_ID_AREA_OR_TARGET_INVALID"
        seen_elements.add(element["element_id"])
        row_ids = [row["view_id"] for row in element["ray_counts"]]
        if len(row_ids) != len(set(row_ids)) or set(row_ids) != candidates:
            return "RAY_COUNT_MATRIX_MUST_MATCH_CANDIDATE_VIEWS_EXACTLY"

    if case["max_selected_views"] > len(case["candidate_views"]):
        return "MAX_SELECTED_VIEWS_EXCEEDS_CANDIDATE_POOL"
    return None


def evaluate(case):
    reason = validation_reason(case)
    if reason is not None:
        return invalid_case(case, reason)
    candidates = set(case["candidate_views"])
    selected = set(case["selected_view_ids"])

    reports = []
    candidate_sufficient = True
    selected_sufficient = True
    provenance_complete = True
    element_ids = set()
    for element in sorted(case["elements"], key=lambda item: item["element_id"]):
        if (not element["element_id"].strip() or not element["mesh_revision_id"].strip()
                or element["element_id"] in element_ids or element["area_um2"] <= 0
                or element["minimum_density_micro_rays_per_mm2"] <= 0
                or element["minimum_contributing_views"] <= 0
                or element["minimum_contributing_views"] > len(candidates)):
            return invalid_case(case)
        element_ids.add(element["element_id"])
        rows = element["ray_counts"]
        row_ids = [row["view_id"] for row in rows]
        if len(row_ids) != len(set(row_ids)) or set(row_ids) != candidates:
            return invalid_case(case)

        candidate_hits = sum(row["intersection_count"] for row in rows)
        selected_rows = [row for row in rows if row["view_id"] in selected]
        selected_hits = sum(row["intersection_count"] for row in selected_rows)
        candidate_view_count = sum(row["intersection_count"] > 0 for row in rows)
        selected_view_count = sum(row["intersection_count"] > 0 for row in selected_rows)
        element_provenance = all(row["provenance"] == "EXACT_SYNTHETIC_ORACLE" for row in rows)
        provenance_complete = provenance_complete and element_provenance

        candidate_density = density_meets(candidate_hits, element["area_um2"], element["minimum_density_micro_rays_per_mm2"])
        selected_density = density_meets(selected_hits, element["area_um2"], element["minimum_density_micro_rays_per_mm2"])
        candidate_view_target = candidate_view_count >= element["minimum_contributing_views"]
        selected_view_target = selected_view_count >= element["minimum_contributing_views"]
        candidate_sufficient = candidate_sufficient and candidate_density and candidate_view_target
        selected_sufficient = selected_sufficient and selected_density and selected_view_target

        contributions = [
            {
                "view_id": row["view_id"],
                "intersection_count": row["intersection_count"],
                "selected": row["view_id"] in selected,
                "provenance": row["provenance"],
            }
            for row in sorted(rows, key=lambda row: row["view_id"])
        ]
        reports.append({
            "element_id": element["element_id"],
            "mesh_revision_id": element["mesh_revision_id"],
            "area_um2": element["area_um2"],
            "minimum_density_micro_rays_per_mm2": element["minimum_density_micro_rays_per_mm2"],
            "minimum_contributing_views": element["minimum_contributing_views"],
            "candidate_total_intersections": candidate_hits,
            "selected_total_intersections": selected_hits,
            "candidate_contributing_views": candidate_view_count,
            "selected_contributing_views": selected_view_count,
            "candidate_density_meets_target": candidate_density,
            "selected_density_meets_target": selected_density,
            "candidate_view_count_meets_target": candidate_view_target,
            "selected_view_count_meets_target": selected_view_target,
            "provenance_complete": element_provenance,
            "selected_density_rational_numerator_micro_mm2": str(selected_hits * SCALE),
            "density_rational_denominator_area_um2": element["area_um2"],
            "view_contributions": contributions,
        })

    selected_budget_met = len(selected) <= case["max_selected_views"]
    if not provenance_complete:
        status = "INDETERMINATE"
    elif not candidate_sufficient:
        status = "INFEASIBLE"
    elif not selected_sufficient or not selected_budget_met:
        status = "BUDGET_INFEASIBLE"
    else:
        status = "FEASIBLE"

    reasons = []
    if not provenance_complete:
        reasons.append("RAY_COUNT_PROVENANCE_UNVERIFIED")
    if not candidate_sufficient:
        reasons.append("CANDIDATE_POOL_DENSITY_OR_VIEW_SHORTFALL")
    if not selected_sufficient:
        reasons.append("SELECTED_PLAN_DENSITY_OR_VIEW_SHORTFALL")
    if not selected_budget_met:
        reasons.append("SELECTED_VIEW_BUDGET_EXCEEDED")

    return {
        "case_id": case["case_id"],
        "status": status,
        "reason_codes": sorted(set(reasons)),
        "candidate_pool_sufficient": candidate_sufficient,
        "selected_plan_sufficient": selected_sufficient,
        "selected_view_budget_met": selected_budget_met,
        "provenance_complete": provenance_complete,
        "candidate_view_count": len(candidates),
        "selected_view_count": len(selected),
        "max_selected_views": case["max_selected_views"],
        "elements": reports,
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: sampling_density_reference_oracle.py <corpus.json> <40-hex-source-commit>")
    raw = Path(sys.argv[1]).read_bytes()
    source = sys.argv[2]
    if len(source) != 40 or any(char not in "0123456789abcdefABCDEF" for char in source):
        raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw)
    if (corpus.get("schema") != CORPUS_SCHEMA or corpus.get("benchmark_id") != BENCHMARK_ID
            or corpus.get("evidence_class") != "synthetic" or corpus.get("claim_ceiling") != CLAIM_CEILING
            or len(corpus.get("cases", [])) != CASE_COUNT):
        raise SystemExit("SAMPLING_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")
    ids = [case["case_id"] for case in corpus["cases"]]
    if len(ids) != len(set(ids)):
        raise SystemExit("DUPLICATE_SAMPLING_CASE_ID")
    receipts = [evaluate(case) for case in corpus["cases"]]
    matches = sum(receipt["status"] == case["expected_status"] for receipt, case in zip(receipts, corpus["cases"]))
    output = {
        "schema": RECEIPT_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "source_commit": source.lower(),
        "fixture_sha256": hashlib.sha256(raw).hexdigest(),
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
