#!/usr/bin/env python3
"""Independent stdlib verification for the AER-01 3-D surface-transfer oracle.

This implementation intentionally does not import or call the Rust crate. It
recomputes triangle quadrature, point containment, barycentric transfer,
resultants, virtual work and rigid-transform invariants from the frozen JSON.
"""

from __future__ import annotations

import argparse
import copy
import json
import math
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "data" / "known-answer-cases-v0.1.0.json"
REL_TOL = 1.0e-10
ABS_TOL = 1.0e-12


class AuditError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def close(actual: float, expected: float, field: str, relative_tolerance: float = REL_TOL) -> None:
    require(
        math.isfinite(float(actual))
        and math.isfinite(float(expected))
        and abs(float(actual) - float(expected))
        <= ABS_TOL + relative_tolerance * abs(float(expected)),
        f"{field}: expected {expected:.16g}, got {actual!r}",
    )


def vec(value: list[float]) -> tuple[float, float, float]:
    require(len(value) == 3, "vectors must have three components")
    result = (float(value[0]), float(value[1]), float(value[2]))
    require(all(math.isfinite(x) for x in result), "vector contains a non-finite value")
    return result


def add(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def scale(a, factor):
    return (a[0] * factor, a[1] * factor, a[2] * factor)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def norm(a):
    return math.sqrt(dot(a, a))


def max_delta(a, b):
    return max(abs(a[i] - b[i]) for i in range(3))


def validate_mesh(mesh: dict[str, Any]) -> None:
    nodes = [vec(value) for value in mesh["nodes_m"]]
    triangles = mesh["triangles"]
    require(bool(nodes), "mesh has no nodes")
    require(bool(triangles), "mesh has no triangles")
    for face_index, face in enumerate(triangles):
        require(len(face) == 3, f"triangle {face_index} is not triangular")
        ids = [int(value) for value in face]
        require(all(0 <= index < len(nodes) for index in ids), "triangle index is out of range")
        require(len(set(ids)) == 3, "triangle repeats a node index")
        a, b, c = (nodes[index] for index in ids)
        edge_scale = max(norm(sub(b, a)), norm(sub(c, a)), norm(sub(c, b)))
        twice_area = norm(cross(sub(b, a), sub(c, a)))
        require(
            edge_scale > 0.0
            and twice_area > 1.0e-14 * edge_scale * edge_scale,
            f"triangle {face_index} is degenerate or ill-scaled",
        )


def triangle_area(mesh: dict[str, Any], face: list[int]) -> float:
    nodes = [vec(value) for value in mesh["nodes_m"]]
    a, b, c = (nodes[int(index)] for index in face)
    area = 0.5 * norm(cross(sub(b, a), sub(c, a)))
    require(math.isfinite(area) and area > 0.0, "triangle area is invalid")
    return area


def barycentric(point, a, b, c):
    ab = sub(b, a)
    ac = sub(c, a)
    ap = sub(point, a)
    d00 = dot(ab, ab)
    d01 = dot(ab, ac)
    d11 = dot(ac, ac)
    d20 = dot(ap, ab)
    d21 = dot(ap, ac)
    denominator = d00 * d11 - d01 * d01
    require(math.isfinite(denominator) and denominator > 0.0, "singular target triangle")
    wb = (d11 * d20 - d01 * d21) / denominator
    wc = (d00 * d21 - d01 * d20) / denominator
    wa = 1.0 - wb - wc
    reconstructed = add(add(scale(a, wa), scale(b, wb)), scale(c, wc))
    return (wa, wb, wc), norm(sub(reconstructed, point))


def locate_point(point, mesh: dict[str, Any], plane_tol: float, bary_tol: float):
    validate_mesh(mesh)
    nodes = [vec(value) for value in mesh["nodes_m"]]
    for face_index, face in enumerate(mesh["triangles"]):
        ids = tuple(int(value) for value in face)
        weights, distance = barycentric(
            point, nodes[ids[0]], nodes[ids[1]], nodes[ids[2]]
        )
        inside = all(-bary_tol <= weight <= 1.0 + bary_tol for weight in weights)
        if distance <= plane_tol and inside:
            return face_index, ids, weights
    raise AuditError("source point outside target support or off surface: extrapolation rejected")


def integrate_traction(source: dict[str, Any], integration: dict[str, Any]):
    validate_mesh(source)
    nodes = [vec(value) for value in source["nodes_m"]]
    tractions = [vec(value) for value in source["traction_n_per_m2_at_nodes"]]
    require(len(tractions) == len(nodes), "traction count does not match source nodes")
    quadrature = [tuple(float(w) for w in point) for point in integration["quadrature_barycentric_points"]]
    area_fraction = float(integration["quadrature_weight_fraction_of_triangle_area"])
    loads = []
    for face in source["triangles"]:
        ids = tuple(int(index) for index in face)
        points = [nodes[index] for index in ids]
        face_tractions = [tractions[index] for index in ids]
        area = triangle_area(source, face)
        for weights in quadrature:
            position = (0.0, 0.0, 0.0)
            traction = (0.0, 0.0, 0.0)
            for local in range(3):
                position = add(position, scale(points[local], weights[local]))
                traction = add(traction, scale(face_tractions[local], weights[local]))
            loads.append((position, scale(traction, area * area_fraction)))
    return loads


def resultants(loads):
    force = (0.0, 0.0, 0.0)
    moment = (0.0, 0.0, 0.0)
    for position, applied_force in loads:
        force = add(force, applied_force)
        moment = add(moment, cross(position, applied_force))
    return force, moment


def transfer(loads, target, integration):
    validate_mesh(target)
    nodes = [vec(value) for value in target["nodes_m"]]
    forces = [(0.0, 0.0, 0.0) for _ in nodes]
    hits = []
    for position, applied_force in loads:
        hit = locate_point(
            position,
            target,
            float(integration["plane_tolerance_m"]),
            float(integration["barycentric_tolerance"]),
        )
        face_index, ids, weights = hit
        for local in range(3):
            node_id = ids[local]
            forces[node_id] = add(forces[node_id], scale(applied_force, weights[local]))
        hits.append((face_index, ids, weights))
    mapped_loads = list(zip(nodes, forces))
    return forces, hits, mapped_loads


def affine_displacement(position, coefficients):
    x, y, z = position
    constant = vec(coefficients["constant_m"])
    xcoef = vec(coefficients["x_coefficient"])
    ycoef = vec(coefficients["y_coefficient"])
    zcoef = vec(coefficients["z_coefficient"])
    return add(add(add(constant, scale(xcoef, x)), scale(ycoef, y)), scale(zcoef, z))


def virtual_work(loads, forces, hits, target, coefficients):
    nodes = [vec(value) for value in target["nodes_m"]]
    nodal_displacements = [affine_displacement(node, coefficients) for node in nodes]
    source_work = 0.0
    affine_error = 0.0
    for (position, applied_force), (_, ids, weights) in zip(loads, hits):
        u_interp = (0.0, 0.0, 0.0)
        for local in range(3):
            u_interp = add(
                u_interp,
                scale(nodal_displacements[ids[local]], weights[local]),
            )
        exact = affine_displacement(position, coefficients)
        affine_error = max(affine_error, max_delta(u_interp, exact))
        source_work += dot(applied_force, u_interp)

    target_work = sum(dot(force, displacement) for force, displacement in zip(forces, nodal_displacements))
    return source_work, target_work, affine_error


def rotate_z(value):
    return (-value[1], value[0], value[2])


def transformed_mesh(mesh, translation):
    return {
        "nodes_m": [
            list(add(rotate_z(vec(node)), translation)) for node in mesh["nodes_m"]
        ],
        "triangles": copy.deepcopy(mesh["triangles"]),
    }


def transformed_loads(loads, translation):
    return [
        (add(rotate_z(position), translation), rotate_z(applied_force))
        for position, applied_force in loads
    ]


def hostile_controls(fixture, loads):
    integration = fixture["integration"]
    target = fixture["target_mesh"]

    def rejected(action, label):
        try:
            action()
        except (AuditError, KeyError, TypeError, ValueError):
            print(f"negative_control=pass case={label}")
            return
        raise AuditError(f"negative control unexpectedly passed: {label}")

    rejected(
        lambda: locate_point((1.1, 0.5, 0.0), target, integration["plane_tolerance_m"], integration["barycentric_tolerance"]),
        "out_of_support_extrapolation",
    )
    rejected(
        lambda: locate_point((0.5, 0.5, 0.01), target, integration["plane_tolerance_m"], integration["barycentric_tolerance"]),
        "off_surface_projection",
    )
    degenerate = {
        "nodes_m": [[0, 0, 0], [1, 0, 0], [2, 0, 0]],
        "triangles": [[0, 1, 2]],
    }
    rejected(lambda: validate_mesh(degenerate), "degenerate_target_triangle")

    bad_source = copy.deepcopy(fixture["source_mesh"])
    bad_source["traction_n_per_m2_at_nodes"][2][0] = float("nan")
    rejected(lambda: integrate_traction(bad_source, integration), "nonfinite_source_traction")

    # A result mutation must not be accepted by the cross-language verifier.
    return True


def verify_python_oracle(fixture):
    require(fixture["schema_version"] == "aero-conservative-surface-transfer-fixture/0.1.0", "fixture schema mismatch")
    require(fixture["claim_ceiling"] == "coplanar_synthetic_surface_transfer_verification_only", "fixture claim ceiling mismatch")

    loads = integrate_traction(fixture["source_mesh"], fixture["integration"])
    require(len(loads) == fixture["expected"]["quadrature_point_count"], "unexpected quadrature count")
    source_force, source_moment = resultants(loads)
    target_forces, hits, mapped_loads = transfer(loads, fixture["target_mesh"], fixture["integration"])
    target_force, target_moment = resultants(mapped_loads)

    expected = fixture["expected"]
    for axis in range(3):
        close(source_force[axis], expected["source_force_n"][axis], f"source_force[{axis}]")
        close(target_force[axis], source_force[axis], f"target_force[{axis}]")
        close(source_moment[axis], expected["source_moment_nm"][axis], f"source_moment[{axis}]")
        close(target_moment[axis], source_moment[axis], f"target_moment[{axis}]")

    source_work, target_work, affine_error = virtual_work(
        loads, target_forces, hits, fixture["target_mesh"], fixture["affine_displacement"]
    )
    close(source_work, expected["source_virtual_work_j"], "source_virtual_work")
    close(target_work, source_work, "target_virtual_work")
    require(affine_error <= expected["affine_displacement_interpolation_error_max_m"], "affine displacement interpolation error exceeded bound")

    # Whole-interface rotation + translation: force rotates, while moment also
    # receives translation cross resultant-force about the unchanged origin.
    translation = (3.0, -2.0, 0.5)
    moved_source = transformed_mesh(fixture["source_mesh"], translation)
    moved_target = transformed_mesh(fixture["target_mesh"], translation)
    moved_loads = transformed_loads(loads, translation)
    moved_target_forces, _, moved_mapped = transfer(moved_loads, moved_target, fixture["integration"])
    moved_source_force, moved_source_moment = resultants(moved_loads)
    moved_target_force, moved_target_moment = resultants(moved_mapped)
    expected_force = rotate_z(source_force)
    expected_moment = add(rotate_z(source_moment), cross(translation, expected_force))
    for axis in range(3):
        close(moved_source_force[axis], expected_force[axis], f"rotated_source_force[{axis}]")
        close(moved_target_force[axis], expected_force[axis], f"rotated_target_force[{axis}]")
        close(moved_source_moment[axis], expected_moment[axis], f"rotated_source_moment[{axis}]")
        close(moved_target_moment[axis], expected_moment[axis], f"rotated_target_moment[{axis}]")

    hostile_controls(fixture, loads)
    return {
        "loads": loads,
        "source_force": source_force,
        "source_moment": source_moment,
        "target_force": target_force,
        "target_moment": target_moment,
        "target_forces": target_forces,
        "hits": hits,
        "target_triangle_hit_indices": [hit[0] for hit in hits],
        "source_work": source_work,
        "target_work": target_work,
        "affine_error": affine_error,
    }


def verify_rust_output(output, fixture, computed):
    require(output.get("schema_version") == "aero-conservative-surface-transfer/0.1.0", "Rust output schema mismatch")
    require(output.get("claim_ceiling") == fixture["claim_ceiling"], "Rust claim ceiling mismatch")
    require(output.get("aircraft_feasibility") == "NOT_ASSESSED", "Rust output promoted aircraft feasibility")
    require(output.get("source_node_count") == len(fixture["source_mesh"]["nodes_m"]), "source node count mismatch")
    require(output.get("source_triangle_count") == len(fixture["source_mesh"]["triangles"]), "source triangle count mismatch")
    require(output.get("target_node_count") == len(fixture["target_mesh"]["nodes_m"]), "target node count mismatch")
    require(output.get("target_triangle_count") == len(fixture["target_mesh"]["triangles"]), "target triangle count mismatch")
    require(output.get("quadrature_point_count") == len(computed["loads"]), "quadrature point count mismatch")
    require(
        output.get("target_hit_triangle_indices") == computed["target_triangle_hit_indices"],
        "target facet assignment does not match independent locator",
    )

    for field, expected in (
        ("source_force_n", computed["source_force"]),
        ("target_force_n", computed["target_force"]),
        ("source_moment_nm", computed["source_moment"]),
        ("target_moment_nm", computed["target_moment"]),
    ):
        actual = output[field]
        for axis in range(3):
            close(actual[axis], expected[axis], f"rust.{field}[{axis}]")

    close(output["force_residual_n"], 0.0, "rust.force_residual")
    close(output["moment_residual_nm"], 0.0, "rust.moment_residual")
    close(output["source_virtual_work_j"], computed["source_work"], "rust.source_work")
    close(output["target_virtual_work_j"], computed["target_work"], "rust.target_work")
    close(output["virtual_work_residual_j"], 0.0, "rust.virtual_work_residual")
    close(output["affine_displacement_interpolation_error_m"], computed["affine_error"], "rust.affine_error")
    require(output.get("unsupported_point_control") == "REJECTED", "unsupported-point hostile result missing")
    require(
        output.get("mesh_geometry_scope") == "coplanar_nonmatching_triangular_surfaces",
        "Rust output overstates geometry scope",
    )
    require(
        output.get("unassessed_claims") == [
            "curved_or_noncoincident_surface_transfer",
            "general_quadrilateral_and_higher_order_elements",
            "full_surface_mortar_or_dual_mortar_projection",
            "dynamic_fsi_and_unsteady_load_transfer",
            "aircraft_specific_structural_validity",
            "airworthiness_and_certification",
        ],
        "Rust unassessed claims changed or omitted",
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path, help="JSON emitted by the Rust executable")
    args = parser.parse_args()
    try:
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        computed = verify_python_oracle(fixture)
        if args.rust_output is None:
            boundary = "rust_python_boundary=not_run"
        else:
            output = json.loads(args.rust_output.read_text(encoding="utf-8"))
            verify_rust_output(output, fixture, computed)
            boundary = "rust_python_boundary=pass"
        print(
            "conservative_surface_transfer=pass "
            f"quadrature_points={len(computed['loads'])} "
            f"force_residual={max_delta(computed['source_force'], computed['target_force']):.3e}N "
            f"moment_residual={max_delta(computed['source_moment'], computed['target_moment']):.3e}Nm "
            f"virtual_work_residual={abs(computed['source_work']-computed['target_work']):.3e}J "
            f"affine_error={computed['affine_error']:.3e}m {boundary}"
        )
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError, IndexError) as error:
        print(f"conservative_surface_transfer_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
