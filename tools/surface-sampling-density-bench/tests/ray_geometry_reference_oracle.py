#!/usr/bin/env python3
"""Independent standard-library oracle for pinhole pixel-centre triangle ray counts."""
import hashlib
import json
import math
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-sampling-density.geometry-corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-sampling-density.geometry-receipt.v0.1"
BENCHMARK_ID = "ORBITAL-RAY-TRIANGLE-001"
CLAIM_CEILING = "synthetic_pinhole_pixel_centre_rays_and_nearest_triangle_intersections_only"
CASE_COUNT = 4
FRAME_TOLERANCE = 1e-9
BARYCENTRIC_TOLERANCE = 1e-12
RAY_TIE_EPSILON_M = 1e-9
MINIMUM_RAY_DISTANCE_M = 1e-9


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def add(a, b):
    return tuple(x + y for x, y in zip(a, b))


def scale(a, value):
    return tuple(x * value for x in a)


def cross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def norm(a):
    return math.sqrt(dot(a, a))


def unit(a):
    length = norm(a)
    if not math.isfinite(length) or length <= MINIMUM_RAY_DISTANCE_M:
        return None
    return scale(a, 1.0 / length)


def validate_camera(camera):
    vectors = [
        camera["origin_m"],
        camera["forward_axis"],
        camera["right_axis"],
        camera["up_axis"],
    ]
    if any(not math.isfinite(value) for vector in vectors for value in vector):
        raise ValueError("CAMERA_MODEL_BOUNDS_INVALID")
    forward = camera["forward_axis"]
    right = camera["right_axis"]
    up = camera["up_axis"]
    if (
        not camera["view_id"].strip()
        or not math.isfinite(camera["horizontal_fov_rad"])
        or not 0 < camera["horizontal_fov_rad"] < math.pi
        or not 1 <= camera["width_px"] <= 256
        or not 1 <= camera["height_px"] <= 256
        or camera["width_px"] * camera["height_px"] > 65536
    ):
        raise ValueError("CAMERA_MODEL_BOUNDS_INVALID")
    if (
        abs(norm(forward) - 1) > FRAME_TOLERANCE
        or abs(norm(right) - 1) > FRAME_TOLERANCE
        or abs(norm(up) - 1) > FRAME_TOLERANCE
        or abs(dot(forward, right)) > FRAME_TOLERANCE
        or abs(dot(forward, up)) > FRAME_TOLERANCE
        or abs(dot(right, up)) > FRAME_TOLERANCE
        or abs(dot(cross(right, up), forward) + 1) > FRAME_TOLERANCE
    ):
        raise ValueError("CAMERA_BASIS_MUST_BE_ORTHONORMAL_AND_RIGHT_HANDED")


def validate_case(case):
    if (
        not case["case_id"].strip()
        or not case["coordinate_frame_id"].strip()
        or case["area_unit"] != "SQUARE_MICROMETRE"
        or not case["cameras"]
        or not case["elements"]
    ):
        raise ValueError("GEOMETRY_CASE_ID_FRAME_UNITS_OR_COLLECTIONS_INVALID")
    view_ids = [camera["view_id"] for camera in case["cameras"]]
    if len(view_ids) != len(set(view_ids)):
        raise ValueError("DUPLICATE_CAMERA_VIEW_ID")
    for camera in case["cameras"]:
        validate_camera(camera)
    element_ids = [element["element_id"] for element in case["elements"]]
    if len(element_ids) != len(set(element_ids)):
        raise ValueError("SURFACE_ELEMENT_ID_AREA_OR_TRIANGLES_INVALID")
    triangle_ids = []
    for element in case["elements"]:
        if (
            not element["element_id"].strip()
            or not element["mesh_revision_id"].strip()
            or element["area_um2"] <= 0
            or not element["triangles"]
        ):
            raise ValueError("SURFACE_ELEMENT_ID_AREA_OR_TRIANGLES_INVALID")
        for triangle in element["triangles"]:
            if not triangle["triangle_id"].strip():
                raise ValueError("TRIANGLE_ID_DUPLICATE_OR_BLANK")
            triangle_ids.append(triangle["triangle_id"])
            vertices = triangle["vertices_m"]
            if any(not math.isfinite(value) for vertex in vertices for value in vertex):
                raise ValueError("TRIANGLE_VERTEX_NONFINITE")
            a, b, c = vertices
            if norm(cross(sub(b, a), sub(c, a))) <= 1e-12:
                raise ValueError("TRIANGLE_DEGENERATE")
    if len(triangle_ids) != len(set(triangle_ids)):
        raise ValueError("TRIANGLE_ID_DUPLICATE_OR_BLANK")
    expected_pairs = {
        (element_id, view_id)
        for element_id in element_ids
        for view_id in view_ids
    }
    expected_rows = [
        (row["element_id"], row["view_id"])
        for row in case["expected_hit_counts"]
    ]
    if len(expected_rows) != len(set(expected_rows)) or set(expected_rows) != expected_pairs:
        raise ValueError("EXPECTED_HIT_MATRIX_MUST_MATCH_ELEMENT_VIEW_PRODUCT")


def pixel_ray(camera, column, row):
    half_horizontal = math.tan(camera["horizontal_fov_rad"] / 2)
    x = (2 * (column + 0.5) / camera["width_px"] - 1) * half_horizontal
    y = (
        (1 - 2 * (row + 0.5) / camera["height_px"])
        * half_horizontal
        * camera["height_px"]
        / camera["width_px"]
    )
    direction = add(
        add(camera["forward_axis"], scale(camera["right_axis"], x)),
        scale(camera["up_axis"], y),
    )
    ray = unit(direction)
    if ray is None:
        raise ValueError("CAMERA_PIXEL_RAY_NORMALIZATION_FAILED")
    return ray


def triangle_intersection(origin, direction, triangle):
    a, b, c = triangle["vertices_m"]
    edge_one = sub(b, a)
    edge_two = sub(c, a)
    p = cross(direction, edge_two)
    determinant = dot(edge_one, p)
    if not math.isfinite(determinant) or abs(determinant) <= 1e-12:
        return None
    inverse = 1.0 / determinant
    from_vertex = sub(origin, a)
    u = dot(from_vertex, p) * inverse
    if (
        not math.isfinite(u)
        or u < -BARYCENTRIC_TOLERANCE
        or u > 1 + BARYCENTRIC_TOLERANCE
    ):
        return None
    q = cross(from_vertex, edge_one)
    v = dot(direction, q) * inverse
    if (
        not math.isfinite(v)
        or v < -BARYCENTRIC_TOLERANCE
        or u + v > 1 + BARYCENTRIC_TOLERANCE
    ):
        return None
    distance = dot(edge_two, q) * inverse
    return (
        distance
        if math.isfinite(distance) and distance > MINIMUM_RAY_DISTANCE_M
        else None
    )


def evaluate_case(case):
    validate_case(case)
    elements = sorted(case["elements"], key=lambda row: row["element_id"])
    cameras = sorted(case["cameras"], key=lambda row: row["view_id"])
    triangles = sorted(
        [
            {
                "element_id": element["element_id"],
                "triangle_id": triangle["triangle_id"],
                "vertices_m": triangle["vertices_m"],
            }
            for element in elements
            for triangle in element["triangles"]
        ],
        key=lambda row: (row["element_id"], row["triangle_id"]),
    )
    expected = {
        (row["element_id"], row["view_id"]): row["hit_pixel_count"]
        for row in case["expected_hit_counts"]
    }
    hit_rows = []
    view_rows = []
    total_rays = 0
    total_hits = 0
    total_ties = 0

    for camera in cameras:
        counts = {element["element_id"]: 0 for element in elements}
        rays = camera["width_px"] * camera["height_px"]
        hit_pixels = 0
        tie_pixels = 0
        triangle_tests = 0
        for row in range(camera["height_px"]):
            for column in range(camera["width_px"]):
                direction = pixel_ray(camera, column, row)
                best = None
                tied = False
                for triangle in triangles:
                    triangle_tests += 1
                    distance = triangle_intersection(camera["origin_m"], direction, triangle)
                    if distance is None:
                        continue
                    key = (triangle["element_id"], triangle["triangle_id"])
                    if best is None or distance < best[2] - RAY_TIE_EPSILON_M:
                        best = (key[0], key[1], distance)
                        tied = False
                    elif abs(distance - best[2]) <= RAY_TIE_EPSILON_M:
                        tied = True
                        nearest = min(best[2], distance)
                        if key < (best[0], best[1]):
                            best = (key[0], key[1], nearest)
                        else:
                            best = (best[0], best[1], nearest)
                if best is not None:
                    hit_pixels += 1
                    counts[best[0]] += 1
                    if tied:
                        tie_pixels += 1

        total_rays += rays
        total_hits += hit_pixels
        total_ties += tie_pixels
        view_rows.append(
            {
                "view_id": camera["view_id"],
                "rays_cast": rays,
                "hit_pixel_count": hit_pixels,
                "background_pixel_count": rays - hit_pixels,
                "tie_pixel_count": tie_pixels,
                "triangle_tests": triangle_tests,
            }
        )
        for element in elements:
            actual = counts[element["element_id"]]
            expected_value = expected[(element["element_id"], camera["view_id"])]
            hit_rows.append(
                {
                    "element_id": element["element_id"],
                    "mesh_revision_id": element["mesh_revision_id"],
                    "view_id": camera["view_id"],
                    "hit_pixel_count": actual,
                    "expected_hit_pixel_count": expected_value,
                    "expected_matches": actual == expected_value,
                }
            )

    matches = sum(row["expected_matches"] for row in hit_rows)
    return {
        "case_id": case["case_id"],
        "coordinate_frame_id": case["coordinate_frame_id"],
        "ray_model": "IDEAL_PINHOLE_PIXEL_CENTRE_NEAREST_TRIANGLE;MOLLER_TRUMBORE_TWO_SIDED",
        "area_provenance": "CALLER_DECLARED_SYNTHETIC;AREA_NOT_DERIVED_OR_PHYSICALLY_MEASURED",
        "element_count": len(elements),
        "triangle_count": len(triangles),
        "view_count": len(cameras),
        "expected_hit_count_rows": len(hit_rows),
        "expected_matches": matches,
        "expected_mismatches": len(hit_rows) - matches,
        "total_rays_cast": total_rays,
        "total_hit_pixels": total_hits,
        "total_background_pixels": total_rays - total_hits,
        "total_tie_pixels": total_ties,
        "element_view_hits": hit_rows,
        "views": view_rows,
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit(
            "usage: ray_geometry_reference_oracle.py <corpus.json> <40-hex-source-commit>"
        )
    raw = Path(sys.argv[1]).read_bytes()
    source = sys.argv[2]
    if len(source) != 40 or any(c not in "0123456789abcdefABCDEF" for c in source):
        raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw)
    if (
        corpus.get("schema") != CORPUS_SCHEMA
        or corpus.get("benchmark_id") != BENCHMARK_ID
        or corpus.get("evidence_class") != "synthetic"
        or corpus.get("claim_ceiling") != CLAIM_CEILING
        or len(corpus.get("cases", [])) != CASE_COUNT
    ):
        raise SystemExit("RAY_GEOMETRY_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")
    ids = [case["case_id"] for case in corpus["cases"]]
    if len(ids) != len(set(ids)):
        raise SystemExit("RAY_GEOMETRY_CASE_ID_DUPLICATE")
    cases = [evaluate_case(case) for case in corpus["cases"]]
    matches = sum(case["expected_mismatches"] == 0 for case in cases)
    print(
        json.dumps(
            {
                "schema": RECEIPT_SCHEMA,
                "benchmark_id": BENCHMARK_ID,
                "source_commit": source.lower(),
                "fixture_sha256": hashlib.sha256(raw).hexdigest(),
                "evidence_class": "synthetic",
                "claim_ceiling": CLAIM_CEILING,
                "case_count": len(cases),
                "expected_outcome_matches": matches,
                "expected_outcome_mismatches": len(cases) - matches,
                "authority_granted": False,
                "physical_validation": "NOT_ASSESSED",
                "cases": cases,
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
