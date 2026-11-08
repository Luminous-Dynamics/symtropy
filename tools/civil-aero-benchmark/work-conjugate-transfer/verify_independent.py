#!/usr/bin/env python3
"""Independent oracle for the planar non-matching triangular transfer fixture.

This module does not import the Rust crate. It independently integrates source
traction, locates target triangles, computes barycentric shape functions, maps
loads/displacements and verifies the returned Rust result. The claim is limited
to a planar triangular patch with exact geometric overlap.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import struct
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "data" / "nonmatching-planar-patch-v0.1.json"


class AuditError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def add(a, b):
    return [a[i] + b[i] for i in range(3)]


def sub(a, b):
    return [a[i] - b[i] for i in range(3)]


def mul(a, scalar):
    return [x * scalar for x in a]


def dot(a, b):
    return sum(a[i] * b[i] for i in range(3))


def cross(a, b):
    return [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]


def norm(a):
    return math.sqrt(dot(a, a))


def unit(a, label):
    length = norm(a)
    require(math.isfinite(length) and length > 1.0e-14, f"{label}: zero/non-finite vector")
    return mul(a, 1.0 / length)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def hash_surface(surface: dict[str, Any], include_traction: bool) -> str:
    digest = hashlib.sha256()
    digest.update(b"aero-surface-v0.1\0")
    nodes = surface["nodes_xyz_m"]
    faces = surface["triangles"]
    digest.update(struct.pack("<Q", len(nodes)))
    for node in nodes:
        for value in node:
            digest.update(struct.pack("<d", float(value)))
    digest.update(struct.pack("<Q", len(faces)))
    for face in faces:
        for index in face:
            digest.update(struct.pack("<Q", int(index)))
    for normal in surface["expected_face_normals"]:
        for value in normal:
            digest.update(struct.pack("<d", float(value)))
    if include_traction:
        digest.update(b"traction-pa\0")
        for traction in surface["nodal_traction_pa"]:
            for value in traction:
                digest.update(struct.pack("<d", float(value)))
    return digest.hexdigest()


def hash_displacements(values) -> str:
    digest = hashlib.sha256()
    digest.update(b"aero-target-displacements-m-v0.1\0")
    for value in values:
        for component in value:
            digest.update(struct.pack("<d", float(component)))
    return digest.hexdigest()


def validate_identity(data):
    require(data["schema_version"] == "aero-work-conjugate-transfer/0.1.0", "schema mismatch")
    for name in (
        "configuration_sha256",
        "geometry_sha256",
        "loading_state_sha256",
        "mission_phase_sha256",
        "environment_sha256",
    ):
        value = data[name]
        require(
            len(value) == 64 and all(c in "0123456789abcdef" for c in value),
            f"{name} is not lowercase SHA-256",
        )
    require(math.isfinite(data["geometry_tolerance_m"]) and data["geometry_tolerance_m"] > 0.0,
            "geometry tolerance must be positive")


def validate_surface(surface, name, require_traction):
    nodes = surface["nodes_xyz_m"]
    faces = surface["triangles"]
    normals = surface["expected_face_normals"]
    require(len(nodes) >= 3 and faces, f"{name}: missing nodes/faces")
    require(len(normals) == len(faces), f"{name}: normal count mismatch")
    if require_traction:
        require(len(surface["nodal_traction_pa"]) == len(nodes), f"{name}: traction count mismatch")
    for node in nodes:
        require(len(node) == 3 and all(math.isfinite(float(v)) for v in node), f"{name}: invalid node")
    if require_traction:
        for value in surface["nodal_traction_pa"]:
            require(len(value) == 3 and all(math.isfinite(float(v)) for v in value),
                    f"{name}: invalid traction")

    seen = set()
    for index, face in enumerate(faces):
        require(len(face) == 3 and all(0 <= int(n) < len(nodes) for n in face),
                f"{name}: face {index} has invalid node")
        require(len(set(face)) == 3, f"{name}: face {index} repeats node")
        key = tuple(sorted(int(v) for v in face))
        require(key not in seen, f"{name}: duplicate triangle {index}")
        seen.add(key)
        a, b, c = [nodes[int(v)] for v in face]
        area_vector = cross(sub(b, a), sub(c, a))
        area = 0.5 * norm(area_vector)
        require(math.isfinite(area) and area > 1.0e-14, f"{name}: degenerate triangle {index}")
        actual = unit(area_vector, "actual face normal")
        expected = unit(normals[index], "expected face normal")
        require(dot(actual, expected) >= 1.0 - 1.0e-10,
                f"{name}: face {index} winding disagrees with declared normal")


def triangle_area(surface, face):
    a, b, c = [surface["nodes_xyz_m"][int(i)] for i in face]
    area_vector = cross(sub(b, a), sub(c, a))
    area = 0.5 * norm(area_vector)
    require(area > 1.0e-14, "degenerate source triangle")
    return area


def source_quadrature(surface):
    barycentrics = (
        (2.0 / 3.0, 1.0 / 6.0, 1.0 / 6.0),
        (1.0 / 6.0, 2.0 / 3.0, 1.0 / 6.0),
        (1.0 / 6.0, 1.0 / 6.0, 2.0 / 3.0),
    )
    output = []
    for face_index, face in enumerate(surface["triangles"]):
        area = triangle_area(surface, face)
        nodes = [surface["nodes_xyz_m"][int(i)] for i in face]
        tractions = [surface["nodal_traction_pa"][int(i)] for i in face]
        for bary in barycentrics:
            position = [0.0, 0.0, 0.0]
            traction = [0.0, 0.0, 0.0]
            for local in range(3):
                position = add(position, mul(nodes[local], bary[local]))
                traction = add(traction, mul(tractions[local], bary[local]))
            output.append(
                {
                    "position": position,
                    "force": mul(traction, area / 3.0),
                    "source_face": face_index,
                }
            )
    return output


def barycentric_projection(point, a, b, c, tolerance):
    v0 = sub(b, a)
    v1 = sub(c, a)
    normal = unit(cross(v0, v1), "projection normal")
    signed = dot(sub(point, a), normal)
    projected = sub(point, mul(normal, signed))
    v2 = sub(projected, a)
    d00, d01, d11 = dot(v0, v0), dot(v0, v1), dot(v1, v1)
    d20, d21 = dot(v2, v0), dot(v2, v1)
    denominator = d00 * d11 - d01 * d01
    require(abs(denominator) > 1.0e-20, "degenerate target projection")
    w1 = (d11 * d20 - d01 * d21) / denominator
    w2 = (d00 * d21 - d01 * d20) / denominator
    weights = [1.0 - w1 - w2, w1, w2]
    bary_tol = max(tolerance * 1.0e3, 1.0e-10)
    if any(v < -bary_tol or v > 1.0 + bary_tol for v in weights):
        return None
    if abs(signed) > tolerance:
        return None
    weights = [max(0.0, v) for v in weights]
    total = sum(weights)
    if total <= 0.0:
        return None
    return [v / total for v in weights], abs(signed)


def locate(point, target, tolerance):
    best = None
    for face_index, face in enumerate(target["triangles"]):
        a, b, c = [target["nodes_xyz_m"][int(i)] for i in face]
        candidate = barycentric_projection(point, a, b, c, tolerance)
        if candidate is None:
            continue
        weights, distance = candidate
        if best is None or distance < best[2]:
            best = (face_index, weights, distance, [int(i) for i in face])
    require(best is not None, "source quadrature point outside target support")
    return best


def resultants(loads, origin):
    force = [0.0, 0.0, 0.0]
    moment = [0.0, 0.0, 0.0]
    for load in loads:
        force = add(force, load["force"])
        moment = add(moment, cross(sub(load["position"], origin), load["force"]))
    return force, moment


def nodal_resultants(nodes, forces, origin):
    force = [0.0, 0.0, 0.0]
    moment = [0.0, 0.0, 0.0]
    for node, value in zip(nodes, forces):
        force = add(force, value)
        moment = add(moment, cross(sub(node, origin), value))
    return force, moment


def transfer(data):
    validate_identity(data)
    validate_surface(data["source_surface"], "source", True)
    validate_surface(data["target_surface"], "target", False)
    require(len(data["target_nodal_displacements_m"]) == len(data["target_surface"]["nodes_xyz_m"]),
            "target displacement count mismatch")
    tolerance = data["geometry_tolerance_m"]
    loads = source_quadrature(data["source_surface"])
    target = data["target_surface"]
    displacements = data["target_nodal_displacements_m"]
    forces = [[0.0, 0.0, 0.0] for _ in target["nodes_xyz_m"]]
    nearest = copy.deepcopy(forces)
    source_work = 0.0
    target_work = 0.0
    nearest_work = 0.0
    used = set()
    max_distance = 0.0

    for load in loads:
        face_index, weights, distance, face = locate(load["position"], target, tolerance)
        used.add(face_index)
        max_distance = max(max_distance, distance)
        uq = [0.0, 0.0, 0.0]
        for local, node in enumerate(face):
            share = mul(load["force"], weights[local])
            forces[node] = add(forces[node], share)
            uq = add(uq, mul(displacements[node], weights[local]))
        source_work += dot(load["force"], uq)

        nearest_index = min(
            range(len(target["nodes_xyz_m"])),
            key=lambda i: norm(sub(target["nodes_xyz_m"][i], load["position"])),
        )
        nearest[nearest_index] = add(nearest[nearest_index], load["force"])

    for force, displacement in zip(forces, displacements):
        target_work += dot(force, displacement)
    for force, displacement in zip(nearest, displacements):
        nearest_work += dot(force, displacement)

    origin = data["moment_reference_origin_m"]
    sf, sm = resultants(loads, origin)
    tf, tm = nodal_resultants(target["nodes_xyz_m"], forces, origin)
    nf, nm = nodal_resultants(target["nodes_xyz_m"], nearest, origin)
    force_res = norm(sub(sf, tf))
    moment_res = norm(sub(sm, tm))
    nearest_force_res = norm(sub(sf, nf))
    nearest_moment_res = norm(sub(sm, nm))
    nearest_work_res = abs(source_work - nearest_work)
    fscale = max(norm(sf), 1.0e-12)
    char_len = max(
        (norm(sub(a, b)) for i, a in enumerate(target["nodes_xyz_m"])
         for b in target["nodes_xyz_m"][i + 1 :]),
        default=0.0,
    )
    mscale = max(norm(sm), fscale * char_len, 1.0e-12)

    return {
        "schema_version": "aero-work-conjugate-transfer-result/0.1.0",
        "method": "piecewise_linear_consistent_triangle_transfer",
        "identity": {
            key: data[key]
            for key in (
                "configuration_sha256",
                "geometry_sha256",
                "loading_state_sha256",
                "mission_phase_sha256",
                "environment_sha256",
            )
        },
        "source_mesh_sha256": hash_surface(data["source_surface"], False),
        "target_mesh_sha256": hash_surface(data["target_surface"], False),
        "source_traction_sha256": hash_surface(data["source_surface"], True),
        "target_displacement_sha256": hash_displacements(displacements),
        "source_quadrature_points": len(loads),
        "mapped_quadrature_points": len(loads),
        "target_faces_used": len(used),
        "maximum_projection_distance_m": max_distance,
        "source_resultant_force_n": sf,
        "target_resultant_force_n": tf,
        "source_resultant_moment_nm": sm,
        "target_resultant_moment_nm": tm,
        "force_residual_n": force_res,
        "moment_residual_nm": moment_res,
        "normalized_force_residual": force_res / fscale,
        "normalized_moment_residual": moment_res / mscale,
        "source_virtual_work_j": source_work,
        "target_virtual_work_j": target_work,
        "virtual_work_residual_j": abs(source_work - target_work),
        "nearest_neighbor_baseline": {
            "resultant_force_n": nf,
            "resultant_moment_nm": nm,
            "force_residual_n": nearest_force_res,
            "moment_residual_nm": nearest_moment_res,
            "source_virtual_work_j": source_work,
            "target_virtual_work_j": nearest_work,
            "virtual_work_residual_j": nearest_work_res,
        },
        "coverage_fraction": 1.0,
        "units_status": "SI_DECLARED_FOR_SYNTHETIC_FIXTURE",
        "surface_overlap_status": "ALL_SOURCE_QUADRATURE_POINTS_INSIDE_TARGET_TRIANGLES",
        "solver_result": "NOT_ASSESSED",
        "structural_response": "NOT_ASSESSED",
        "claim_ceiling": "planar_triangular_patch_work_conjugate_transfer_only",
    }


def expect_rejected(label, action):
    try:
        action()
    except (AuditError, ValueError, ZeroDivisionError):
        print(f"negative_control=pass case={label}")
        return
    raise AuditError(f"negative control unexpectedly passed: {label}")


def verify_negative_controls(data):
    bad = copy.deepcopy(data)
    bad["target_surface"]["nodes_xyz_m"] = [
        [x + 10.0, y, z] for x, y, z in bad["target_surface"]["nodes_xyz_m"]
    ]
    expect_rejected("no_geometric_overlap", lambda: transfer(bad))

    bad = copy.deepcopy(data)
    bad["source_surface"]["triangles"][0] = [0, 2, 1]
    expect_rejected("reversed_source_face_winding", lambda: transfer(bad))

    bad = copy.deepcopy(data)
    bad["target_surface"]["triangles"][0] = [0, 1, 1]
    expect_rejected("degenerate_target_triangle", lambda: transfer(bad))

    bad = copy.deepcopy(data)
    bad["loading_state_sha256"] = "not-a-digest"
    expect_rejected("stale_loading_state_identity", lambda: transfer(bad))

    bad = copy.deepcopy(data)
    bad["geometry_tolerance_m"] = 0.0
    expect_rejected("invalid_geometry_tolerance", lambda: transfer(bad))

    bad = copy.deepcopy(data)
    bad["target_surface"]["triangles"].append(bad["target_surface"]["triangles"][0])
    bad["target_surface"]["expected_face_normals"].append([0.0, 0.0, 1.0])
    expect_rejected("duplicate_target_face", lambda: transfer(bad))


def verify_output(actual, expected):
    exact_keys = [
        "schema_version",
        "method",
        "identity",
        "source_mesh_sha256",
        "target_mesh_sha256",
        "source_traction_sha256",
        "target_displacement_sha256",
        "source_quadrature_points",
        "mapped_quadrature_points",
        "target_faces_used",
        "solver_result",
        "structural_response",
        "claim_ceiling",
    ]
    for key in exact_keys:
        require(actual.get(key) == expected[key], f"Rust/Python mismatch for {key}")

    numeric_keys = [
        "maximum_projection_distance_m",
        "force_residual_n",
        "moment_residual_nm",
        "normalized_force_residual",
        "normalized_moment_residual",
        "source_virtual_work_j",
        "target_virtual_work_j",
        "virtual_work_residual_j",
        "coverage_fraction",
    ]
    for key in numeric_keys:
        av, ev = float(actual[key]), float(expected[key])
        require(abs(av - ev) <= 1.0e-11 + 1.0e-10 * abs(ev),
                f"Rust/Python mismatch for {key}: {av} != {ev}")

    for key in (
        "source_resultant_force_n",
        "target_resultant_force_n",
        "source_resultant_moment_nm",
        "target_resultant_moment_nm",
    ):
        require(len(actual[key]) == 3, f"{key} must be Vec3")
        for a, e in zip(actual[key], expected[key]):
            require(abs(float(a) - float(e)) <= 1.0e-11 + 1.0e-10 * abs(float(e)),
                    f"Rust/Python mismatch for {key}")

    actual_base = actual["nearest_neighbor_baseline"]
    expected_base = expected["nearest_neighbor_baseline"]
    for key in (
        "force_residual_n",
        "moment_residual_nm",
        "source_virtual_work_j",
        "target_virtual_work_j",
        "virtual_work_residual_j",
    ):
        av, ev = float(actual_base[key]), float(expected_base[key])
        require(abs(av - ev) <= 1.0e-11 + 1.0e-10 * abs(ev),
                f"Rust/Python mismatch for nearest baseline {key}")

    require(expected["normalized_force_residual"] < 1.0e-12, "consistent mapping force conservation failed")
    require(expected["normalized_moment_residual"] < 1.0e-12, "consistent mapping moment conservation failed")
    require(expected["virtual_work_residual_j"] < 1.0e-12, "work conjugacy failed")
    require(expected_base["force_residual_n"] < 1.0e-12, "nearest baseline should preserve total force")
    require(expected_base["moment_residual_nm"] > 1.0e-4, "nearest baseline did not expose moment error")
    require(expected_base["virtual_work_residual_j"] > 1.0e-4,
            "nearest baseline did not expose work-conjugacy error")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path)
    args = parser.parse_args()
    try:
        data = json.loads(FIXTURE.read_text(encoding="utf-8"))
        expected = transfer(data)
        verify_negative_controls(data)
        if args.rust_output:
            actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
            verify_output(actual, expected)
            boundary = "rust_python_boundary=pass"
        else:
            boundary = "rust_python_boundary=not_run"
        print(
            "work_conjugate_transfer_oracle=pass "
            f"source_quadrature={expected['source_quadrature_points']} "
            f"target_faces_used={expected['target_faces_used']} "
            f"force_rel={expected['normalized_force_residual']:.3e} "
            f"moment_rel={expected['normalized_moment_residual']:.3e} "
            f"virtual_work={expected['virtual_work_residual_j']:.3e} "
            f"nearest_moment={expected['nearest_neighbor_baseline']['moment_residual_nm']:.3e} "
            f"nearest_work={expected['nearest_neighbor_baseline']['virtual_work_residual_j']:.3e} "
            f"{boundary}"
        )
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"work_conjugate_transfer_error={error}", file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
