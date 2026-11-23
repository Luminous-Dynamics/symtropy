#!/usr/bin/env python3
"""Independent standard-library oracle for conservative 3D surface support."""

import hashlib
import importlib.util
import json
import math
import sys
from pathlib import Path

SUPPORT_SCHEMA = "luminous.orbital-surface-support.receipt.v0.1"
SUPPORT_BENCHMARK_ID = "ORBITAL-SURFACE-SUPPORT-VOXEL-001"
SUPPORT_CLAIM = "synthetic_3d_voxel_surface_support_envelope_and_conservative_occlusion_only"
POSITION_MARGIN = 1.0e-6
AXIS_MARGIN = 2.0e-6
EPS = 1.0e-12
FACES = [
    ((-1, 0, 0), (-1, 0, 0)), ((1, 0, 0), (1, 0, 0)),
    ((0, -1, 0), (0, -1, 0)), ((0, 1, 0), (0, 1, 0)),
    ((0, 0, -1), (0, 0, -1)), ((0, 0, 1), (0, 0, 1)),
]


def add(a, b): return tuple(x + y for x, y in zip(a, b))
def sub(a, b): return tuple(x - y for x, y in zip(a, b))
def mul(a, factor): return tuple(x * factor for x in a)
def dot(a, b): return sum(x * y for x, y in zip(a, b))
def norm(a): return math.sqrt(dot(a, a))
def unit(a):
    length = norm(a)
    return mul(a, 1.0 / length) if length > 1.0e-12 else None
def angle(a, b):
    left = unit(a)
    right = unit(b)
    if left is None or right is None:
        return None
    return math.acos(max(-1.0, min(1.0, dot(left, right))))
def quant(value, scale):
    scaled = value * scale
    return math.floor(scaled + 0.5) if scaled >= 0 else math.ceil(scaled - 0.5)
def floor_scaled(value, scale): return math.floor(value * scale)
def ceil_scaled(value, scale): return math.ceil(value * scale)
def cell_id(cell): return "V-%d-%d-%d" % tuple(cell)


def load_upstream_oracle():
    tools_dir = Path(__file__).resolve().parents[2]
    upstream_path = tools_dir / "orbital-gsd-kcoverage-bench" / "tests" / "reference_oracle.py"
    spec = importlib.util.spec_from_file_location("orbital_gsd_reference_oracle", upstream_path)
    module = importlib.util.module_from_spec(spec)
    if spec is None or spec.loader is None:
        raise RuntimeError("UPSTREAM_GSD_ORACLE_LOAD_FAILED")
    spec.loader.exec_module(module)
    return module


def build_supports(case):
    grid = case["grid"]
    origin = tuple(grid["origin_m"])
    voxel = grid["voxel_size_m"]
    dimensions = tuple(grid["dimensions"])
    occupied = {tuple(cell) for cell in grid["occupied_cells"]}
    padded = tuple(value + 2 for value in dimensions)
    queue = [(0, 0, 0)]
    exterior_padded = {(0, 0, 0)}
    while queue:
        current = queue.pop()
        for delta, _ in FACES:
            nxt = add(current, delta)
            if any(nxt[i] < 0 or nxt[i] >= padded[i] for i in range(3)):
                continue
            original = tuple(nxt[i] - 1 for i in range(3))
            inside = all(0 <= original[i] < dimensions[i] for i in range(3))
            if inside and original in occupied:
                continue
            if nxt not in exterior_padded:
                exterior_padded.add(nxt)
                queue.append(nxt)
    exterior = {
        (x, y, z)
        for x in range(dimensions[0])
        for y in range(dimensions[1])
        for z in range(dimensions[2])
        if (x, y, z) not in occupied and (x + 1, y + 1, z + 1) in exterior_padded
    }

    supports = []
    for cell in sorted(occupied):
        normals = []
        for delta, normal in FACES:
            neighbour = add(cell, delta)
            if neighbour in occupied:
                continue
            inside = all(0 <= neighbour[i] < dimensions[i] for i in range(3))
            if not inside or neighbour in exterior:
                normals.append(normal)
        if not normals:
            continue
        summed = tuple(sum(normal[i] for normal in normals) for i in range(3))
        centre = tuple(origin[i] + (cell[i] + 0.5) * voxel for i in range(3))
        if norm(summed) <= 1.0e-9:
            for normal_index, normal in enumerate(normals):
                face_centre = add(centre, mul(normal, voxel * 0.5))
                half = [voxel * 0.5] * 3
                axis = next(i for i, value in enumerate(normal) if value)
                half[axis] = 0.0
                supports.append(make_support(
                    "%s-N%d" % (cell_id(cell), normal_index),
                    "%s-N%d" % (cell_id(cell), normal_index),
                    cell, face_centre, half, normal, 0.0, "PLANAR_VOXEL_FACE"
                ))
        else:
            mean_normal = unit(summed)
            cone = max(angle(mean_normal, normal) for normal in normals)
            supports.append(make_support(
                cell_id(cell), cell_id(cell), cell, centre,
                [voxel * 0.5] * 3, mean_normal, cone,
                "EXTERIOR_FACE_UNION_BOUND"
            ))
    return sorted(supports, key=lambda row: row["support_id"])


def make_support(support_id, feature_id, cell, centre, half, normal, cone, kind):
    nq = [quant(value, 1.0e6) for value in unit(normal)]
    qnormal = unit([value / 1.0e6 for value in nq])
    normal_error = angle(unit(normal), qnormal)
    return {
        "support_id": support_id,
        "feature_id": feature_id,
        "cell": list(cell),
        "centre_um": [quant(value, 1.0e6) for value in centre],
        "half_extents_um": [ceil_scaled(value, 1.0e6) + 1 for value in half],
        "outward_normal_q1e6": nq,
        "normal_cone_half_angle_microrad": ceil_scaled(cone + normal_error, 1.0e6) + 1,
        "support_kind": kind,
    }


def overlaps(a_min, a_max, b_min, b_max):
    return all(a_max[i] + EPS >= b_min[i] and b_max[i] + EPS >= a_min[i]
               for i in range(3))


def occlusion_candidates(case, support, sensor):
    grid = case["grid"]
    origin = grid["origin_m"]
    voxel = grid["voxel_size_m"]
    centre = [value / 1.0e6 for value in support["centre_um"]]
    half = [value / 1.0e6 for value in support["half_extents_um"]]
    ray_min = [min(sensor[i], centre[i] - half[i]) for i in range(3)]
    ray_max = [max(sensor[i], centre[i] + half[i]) for i in range(3)]
    result = []
    for cell in sorted(tuple(item) for item in grid["occupied_cells"]):
        if list(cell) == support["cell"]:
            continue
        lo = [origin[i] + cell[i] * voxel for i in range(3)]
        hi = [lo[i] + voxel for i in range(3)]
        if overlaps(ray_min, ray_max, lo, hi):
            result.append(list(cell))
    return result


def centre_not_visible(support, candidate, slant_limit):
    return {
        "support_id": support["support_id"],
        "feature_id": support["feature_id"],
        "source_cell": support["cell"],
        "view_id": candidate["view_id"],
        "centre_visible_in_upstream_model": False,
        "disposition": "CENTRE_NOT_VISIBLE",
        "reason_codes": ["UPSTREAM_CENTRE_VISIBILITY_FALSE"],
        "support_radius_um_upper": None,
        "centre_distance_um": None,
        "minimum_distance_um_lower": None,
        "maximum_distance_um_upper": None,
        "direction_variation_microrad_upper": None,
        "centre_fov_microrad": None,
        "support_fov_microrad_upper": None,
        "centre_incidence_microrad": None,
        "support_incidence_microrad_upper": None,
        "gsd_microm_per_pixel_upper": None,
        "derived_slant_range_um_upper_limit": slant_limit,
        "occlusion_disposition": "NOT_EVALUATED_CENTRE_NOT_VISIBLE",
        "occlusion_candidate_cells": [],
    }


def evaluate_pair(case, source_receipt, support, candidate):
    slant_limit = source_receipt["slant_range_max_um"]
    if support["feature_id"] not in candidate["visible_feature_ids"]:
        return centre_not_visible(support, candidate, slant_limit)

    centre = [value / 1.0e6 for value in support["centre_um"]]
    half = [value / 1.0e6 for value in support["half_extents_um"]]
    radius = norm(half) + POSITION_MARGIN
    sensor = [value / 1.0e6 for value in candidate["position_um"]]
    axis = [value / 1.0e6 for value in candidate["optical_axis_q1e6"]]
    normal = [value / 1.0e6 for value in support["outward_normal_q1e6"]]
    cone = support["normal_cone_half_angle_microrad"] / 1.0e6
    to_support = sub(centre, sensor)
    distance = norm(to_support)
    reasons = []
    dmin_um = dmax_um = delta_um = eta_um = eta_upper_um = None
    alpha_um = alpha_upper_um = gsd_nano = None
    radius_um = ceil_scaled(radius, 1.0e6)

    if (not math.isfinite(radius) or not math.isfinite(distance)
            or radius <= 0 or not math.isfinite(cone) or cone < 0 or cone > math.pi):
        reasons.append("SUPPORT_GEOMETRY_BOUND_INVALID")
    elif distance <= radius + EPS:
        reasons.append("SENSOR_NOT_OUTSIDE_SUPPORT_BOUND")
    else:
        delta = math.asin(max(0.0, min(1.0, radius / distance)))
        dmin = distance - radius
        dmax = distance + radius
        eta0 = angle(axis, to_support)
        alpha0 = angle(normal, sub(sensor, centre))
        if eta0 is None or alpha0 is None:
            reasons.append("SUPPORT_DIRECTION_OR_NORMAL_UNDEFINED")
        else:
            eta_upper = eta0 + delta + AXIS_MARGIN
            alpha_upper = alpha0 + delta + cone
            slant_limit_m = slant_limit / 1.0e6 - POSITION_MARGIN
            if dmax > slant_limit_m + EPS:
                reasons.append("SUPPORT_RANGE_UPPER_BOUND_EXCEEDS_LIMIT")
            if eta_upper > case["camera"]["horizontal_fov_rad"] * 0.5 + EPS:
                reasons.append("SUPPORT_FOV_UPPER_BOUND_EXCEEDS_LIMIT")
            if alpha_upper >= math.pi * 0.5 - EPS:
                reasons.append("SUPPORT_INCIDENCE_BOUND_REACHES_GRAZING")
            elif alpha_upper > case["camera"]["max_oblique_angle_rad"] + EPS:
                reasons.append("SUPPORT_INCIDENCE_UPPER_BOUND_EXCEEDS_LIMIT")
            else:
                gsd = (
                    dmax * (2.0 * math.tan(case["camera"]["horizontal_fov_rad"] * 0.5))
                    / (case["camera"]["horizontal_pixel_count"] * math.cos(alpha_upper))
                )
                gsd_nano = ceil_scaled(gsd, 1.0e9)
                if not math.isfinite(gsd) or gsd > case["camera"]["gsd_max_m_per_pixel"] + EPS:
                    reasons.append("SUPPORT_GSD_UPPER_BOUND_EXCEEDS_LIMIT")
            dmin_um = floor_scaled(dmin, 1.0e6)
            dmax_um = ceil_scaled(dmax, 1.0e6)
            delta_um = ceil_scaled(delta, 1.0e6)
            eta_um = quant(eta0, 1.0e6)
            eta_upper_um = ceil_scaled(eta_upper, 1.0e6)
            alpha_um = quant(alpha0, 1.0e6)
            alpha_upper_um = ceil_scaled(alpha_upper, 1.0e6)

    occ_candidates = []
    if not reasons:
        occ_candidates = occlusion_candidates(case, support, sensor)
        if occ_candidates:
            reasons.append("OCCLUSION_CONSERVATIVE_ENCLOSURE_OVERLAP")
            occ_disposition = "INDETERMINATE_CONSERVATIVE_ENCLOSURE_OVERLAP"
        else:
            occ_disposition = "CLEAR_CERTIFIED_BY_RAY_ENCLOSURE_AABB_DISJOINTNESS"
    else:
        occ_disposition = "NOT_EVALUATED_GEOMETRY_BOUND_INDETERMINATE"

    return {
        "support_id": support["support_id"],
        "feature_id": support["feature_id"],
        "source_cell": support["cell"],
        "view_id": candidate["view_id"],
        "centre_visible_in_upstream_model": True,
        "disposition": "CERTIFIED_VISIBLE" if not reasons else "INDETERMINATE",
        "reason_codes": reasons,
        "support_radius_um_upper": radius_um,
        "centre_distance_um": quant(distance, 1.0e6),
        "minimum_distance_um_lower": dmin_um,
        "maximum_distance_um_upper": dmax_um,
        "direction_variation_microrad_upper": delta_um,
        "centre_fov_microrad": eta_um,
        "support_fov_microrad_upper": eta_upper_um,
        "centre_incidence_microrad": alpha_um,
        "support_incidence_microrad_upper": alpha_upper_um,
        "gsd_microm_per_pixel_upper": (
            (gsd_nano + 999) // 1000 if gsd_nano is not None else None
        ),
        "derived_slant_range_um_upper_limit": slant_limit,
        "occlusion_disposition": occ_disposition,
        "occlusion_candidate_cells": occ_candidates,
    }


def evaluate(case, raw, source_commit, gsd_receipt):
    supports = build_supports(case)
    candidates = gsd_receipt["candidate_views"]
    certificates = [
        evaluate_pair(case, gsd_receipt, support, candidate)
        for candidate in candidates
        for support in supports
    ]
    centre_visible = sum(row["centre_visible_in_upstream_model"] for row in certificates)
    centre_not_visible = len(certificates) - centre_visible
    certified = sum(row["disposition"] == "CERTIFIED_VISIBLE" for row in certificates)
    indeterminate = sum(row["disposition"] == "INDETERMINATE" for row in certificates)
    if centre_visible + centre_not_visible != len(certificates) or certified + indeterminate != centre_visible:
        raise ValueError("SUPPORT_RECEIPT_ACCOUNTING_INVARIANT_FAILED")

    return {
        "schema": SUPPORT_SCHEMA,
        "benchmark_id": SUPPORT_BENCHMARK_ID,
        "source_commit": source_commit.lower(),
        "source_case_id": case["case_id"],
        "coordinate_frame_id": case["coordinate_frame_id"],
        "source_input_sha256": hashlib.sha256(raw).hexdigest(),
        "evidence_class": "synthetic",
        "claim_ceiling": SUPPORT_CLAIM,
        "source_candidate_view_count": len(candidates),
        "source_surface_support_count": len(supports),
        "candidate_support_pair_count": len(certificates),
        "centre_visible_pair_count": centre_visible,
        "centre_not_visible_pair_count": centre_not_visible,
        "certified_visible_pair_count": certified,
        "indeterminate_pair_count": indeterminate,
        "authority_granted": False,
        "structural_acceptance_authorized": False,
        "flight_authorized": False,
        "physical_validation": "NOT_ASSESSED",
        "certificates": certificates,
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: support_reference_oracle.py <gsd-case.json> <40-hex-source-commit>")
    path = Path(sys.argv[1])
    raw = path.read_bytes()
    source_commit = sys.argv[2]
    if len(source_commit) != 40 or any(c not in "0123456789abcdefABCDEF" for c in source_commit):
        raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    case = json.loads(raw)
    oracle = load_upstream_oracle()
    source_receipt = oracle.evaluate(case, raw, source_commit)
    print(json.dumps(evaluate(case, raw, source_commit, source_receipt), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
