// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Bounded coplanar triangular-surface transfer oracle.
// This is not a general CFD/FEA coupling library or aircraft analysis tool.

use std::fmt::Write as _;

const PLANE_TOLERANCE_M: f64 = 1.0e-10;
const BARYCENTRIC_TOLERANCE: f64 = 1.0e-12;
const DEGENERACY_RELATIVE_TOLERANCE: f64 = 1.0e-14;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Vec3([f64; 3]);

impl Vec3 {
    const ZERO: Self = Self([0.0, 0.0, 0.0]);

    fn add(self, rhs: Self) -> Self {
        Self([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
        ])
    }

    fn sub(self, rhs: Self) -> Self {
        Self([
            self.0[0] - rhs.0[0],
            self.0[1] - rhs.0[1],
            self.0[2] - rhs.0[2],
        ])
    }

    fn scale(self, factor: f64) -> Self {
        Self([self.0[0] * factor, self.0[1] * factor, self.0[2] * factor])
    }

    fn dot(self, rhs: Self) -> f64 {
        self.0[0] * rhs.0[0] + self.0[1] * rhs.0[1] + self.0[2] * rhs.0[2]
    }

    fn cross(self, rhs: Self) -> Self {
        Self([
            self.0[1] * rhs.0[2] - self.0[2] * rhs.0[1],
            self.0[2] * rhs.0[0] - self.0[0] * rhs.0[2],
            self.0[0] * rhs.0[1] - self.0[1] * rhs.0[0],
        ])
    }

    fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    fn is_finite(self) -> bool {
        self.0.iter().all(|value| value.is_finite())
    }

    fn max_abs_delta(self, rhs: Self) -> f64 {
        (0..3)
            .map(|axis| (self.0[axis] - rhs.0[axis]).abs())
            .fold(0.0, f64::max)
    }
}

#[derive(Clone, Debug)]
struct SurfaceMesh {
    nodes_m: Vec<Vec3>,
    triangles: Vec<[usize; 3]>,
}

#[derive(Clone, Copy, Debug)]
struct QuadratureLoad {
    position_m: Vec3,
    force_n: Vec3,
}

#[derive(Clone, Copy, Debug)]
struct BarycentricHit {
    triangle_index: usize,
    node_ids: [usize; 3],
    weights: [f64; 3],
}

#[derive(Clone, Debug)]
struct TransferResult {
    nodal_forces_n: Vec<Vec3>,
    hits: Vec<BarycentricHit>,
}

#[derive(Clone, Copy, Debug)]
struct Resultants {
    force_n: Vec3,
    moment_nm: Vec3,
}

fn validate_mesh(mesh: &SurfaceMesh) -> Result<(), String> {
    if mesh.nodes_m.is_empty() {
        return Err("surface mesh has no nodes".to_owned());
    }
    if mesh.triangles.is_empty() {
        return Err("surface mesh has no triangles".to_owned());
    }
    if mesh.nodes_m.iter().any(|node| !node.is_finite()) {
        return Err("surface mesh contains a non-finite node".to_owned());
    }

    for (face_index, face) in mesh.triangles.iter().enumerate() {
        if face[0] >= mesh.nodes_m.len()
            || face[1] >= mesh.nodes_m.len()
            || face[2] >= mesh.nodes_m.len()
        {
            return Err(format!(
                "triangle {face_index} has an out-of-range node index"
            ));
        }
        if face[0] == face[1] || face[1] == face[2] || face[0] == face[2] {
            return Err(format!("triangle {face_index} repeats a node index"));
        }

        let a = mesh.nodes_m[face[0]];
        let b = mesh.nodes_m[face[1]];
        let c = mesh.nodes_m[face[2]];
        let edge_scale = b.sub(a).norm().max(c.sub(a).norm()).max(c.sub(b).norm());
        let twice_area = b.sub(a).cross(c.sub(a)).norm();
        if !edge_scale.is_finite()
            || !twice_area.is_finite()
            || edge_scale <= 0.0
            || twice_area <= DEGENERACY_RELATIVE_TOLERANCE * edge_scale.powi(2)
        {
            return Err(format!("triangle {face_index} is degenerate or ill-scaled"));
        }
    }
    Ok(())
}

fn barycentric_coordinates(
    point: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) -> Result<([f64; 3], f64), String> {
    let ab = b.sub(a);
    let ac = c.sub(a);
    let ap = point.sub(a);
    let d00 = ab.dot(ab);
    let d01 = ab.dot(ac);
    let d11 = ac.dot(ac);
    let d20 = ap.dot(ab);
    let d21 = ap.dot(ac);
    let denominator = d00 * d11 - d01 * d01;
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err("target triangle has singular barycentric coordinates".to_owned());
    }

    let weight_b = (d11 * d20 - d01 * d21) / denominator;
    let weight_c = (d00 * d21 - d01 * d20) / denominator;
    let weight_a = 1.0 - weight_b - weight_c;
    let weights = [weight_a, weight_b, weight_c];
    let reconstructed = a
        .scale(weight_a)
        .add(b.scale(weight_b))
        .add(c.scale(weight_c));
    let plane_distance_m = reconstructed.sub(point).norm();
    if !plane_distance_m.is_finite() || weights.iter().any(|weight| !weight.is_finite()) {
        return Err("barycentric projection produced a non-finite result".to_owned());
    }
    Ok((weights, plane_distance_m))
}

fn locate_point(
    point_m: Vec3,
    target: &SurfaceMesh,
    plane_tolerance_m: f64,
    barycentric_tolerance: f64,
) -> Result<BarycentricHit, String> {
    validate_mesh(target)?;
    if !point_m.is_finite() {
        return Err("source quadrature point is non-finite".to_owned());
    }

    // Deterministic tie-break: the first triangle in the frozen face order wins
    // for points exactly on a shared edge or vertex.
    for (triangle_index, node_ids) in target.triangles.iter().enumerate() {
        let a = target.nodes_m[node_ids[0]];
        let b = target.nodes_m[node_ids[1]];
        let c = target.nodes_m[node_ids[2]];
        let Ok((weights, plane_distance_m)) = barycentric_coordinates(point_m, a, b, c) else {
            continue;
        };

        let inside = weights.iter().all(|weight| {
            *weight >= -barycentric_tolerance && *weight <= 1.0 + barycentric_tolerance
        });
        if plane_distance_m <= plane_tolerance_m && inside {
            return Ok(BarycentricHit {
                triangle_index,
                node_ids: *node_ids,
                weights,
            });
        }
    }

    Err(
        "source point is outside target support or off the target surface; extrapolation rejected"
            .to_owned(),
    )
}

fn triangle_area_m2(mesh: &SurfaceMesh, triangle: [usize; 3]) -> f64 {
    let a = mesh.nodes_m[triangle[0]];
    let b = mesh.nodes_m[triangle[1]];
    let c = mesh.nodes_m[triangle[2]];
    0.5 * b.sub(a).cross(c.sub(a)).norm()
}

fn integrate_linear_surface_traction(
    source: &SurfaceMesh,
    nodal_traction_n_per_m2: &[Vec3],
) -> Result<Vec<QuadratureLoad>, String> {
    validate_mesh(source)?;
    if nodal_traction_n_per_m2.len() != source.nodes_m.len() {
        return Err("source traction count does not match source node count".to_owned());
    }
    if nodal_traction_n_per_m2
        .iter()
        .any(|traction| !traction.is_finite())
    {
        return Err("source traction field contains a non-finite vector".to_owned());
    }

    // Three symmetric degree-2 triangle integration points. For a linear
    // traction field, this integrates resultant force and first moment exactly.
    let quadrature = [
        [2.0 / 3.0, 1.0 / 6.0, 1.0 / 6.0],
        [1.0 / 6.0, 2.0 / 3.0, 1.0 / 6.0],
        [1.0 / 6.0, 1.0 / 6.0, 2.0 / 3.0],
    ];

    let mut loads = Vec::with_capacity(source.triangles.len() * quadrature.len());
    for triangle in &source.triangles {
        let area_m2 = triangle_area_m2(source, *triangle);
        if !area_m2.is_finite() || area_m2 <= 0.0 {
            return Err("source triangle area is invalid".to_owned());
        }
        let positions = [
            source.nodes_m[triangle[0]],
            source.nodes_m[triangle[1]],
            source.nodes_m[triangle[2]],
        ];
        let tractions = [
            nodal_traction_n_per_m2[triangle[0]],
            nodal_traction_n_per_m2[triangle[1]],
            nodal_traction_n_per_m2[triangle[2]],
        ];

        for weights in quadrature {
            let mut position = Vec3::ZERO;
            let mut traction = Vec3::ZERO;
            for local_node in 0..3 {
                position = position.add(positions[local_node].scale(weights[local_node]));
                traction = traction.add(tractions[local_node].scale(weights[local_node]));
            }
            loads.push(QuadratureLoad {
                position_m: position,
                force_n: traction.scale(area_m2 / 3.0),
            });
        }
    }
    Ok(loads)
}

fn source_resultants(loads: &[QuadratureLoad]) -> Resultants {
    let mut force_n = Vec3::ZERO;
    let mut moment_nm = Vec3::ZERO;
    for load in loads {
        force_n = force_n.add(load.force_n);
        moment_nm = moment_nm.add(load.position_m.cross(load.force_n));
    }
    Resultants { force_n, moment_nm }
}

fn transfer_loads(
    loads: &[QuadratureLoad],
    target: &SurfaceMesh,
) -> Result<TransferResult, String> {
    validate_mesh(target)?;
    let mut nodal_forces_n = vec![Vec3::ZERO; target.nodes_m.len()];
    let mut hits = Vec::with_capacity(loads.len());

    for load in loads {
        if !load.position_m.is_finite() || !load.force_n.is_finite() {
            return Err("quadrature load contains a non-finite position or force".to_owned());
        }
        let hit = locate_point(
            load.position_m,
            target,
            PLANE_TOLERANCE_M,
            BARYCENTRIC_TOLERANCE,
        )?;
        for local_node in 0..3 {
            let node_id = hit.node_ids[local_node];
            nodal_forces_n[node_id] =
                nodal_forces_n[node_id].add(load.force_n.scale(hit.weights[local_node]));
        }
        hits.push(hit);
    }

    Ok(TransferResult {
        nodal_forces_n,
        hits,
    })
}

fn target_resultants(target: &SurfaceMesh, nodal_forces_n: &[Vec3]) -> Result<Resultants, String> {
    validate_mesh(target)?;
    if nodal_forces_n.len() != target.nodes_m.len() {
        return Err("target force count does not match target node count".to_owned());
    }
    let mut force_n = Vec3::ZERO;
    let mut moment_nm = Vec3::ZERO;
    for (position, force) in target.nodes_m.iter().zip(nodal_forces_n) {
        if !force.is_finite() {
            return Err("target nodal force is non-finite".to_owned());
        }
        force_n = force_n.add(*force);
        moment_nm = moment_nm.add(position.cross(*force));
    }
    Ok(Resultants { force_n, moment_nm })
}

fn affine_displacement(position_m: Vec3) -> Vec3 {
    let x = position_m.0[0];
    let y = position_m.0[1];
    let z = position_m.0[2];
    Vec3([
        0.001 + 0.0002 * x - 0.0001 * y,
        -0.002 + 0.0003 * x + 0.0004 * y,
        0.0005 + 0.0001 * y,
    ])
}

fn interpolated_displacement(hit: BarycentricHit, nodal_displacements_m: &[Vec3]) -> Vec3 {
    let mut displacement = Vec3::ZERO;
    for local_node in 0..3 {
        displacement = displacement
            .add(nodal_displacements_m[hit.node_ids[local_node]].scale(hit.weights[local_node]));
    }
    displacement
}

fn virtual_work(
    loads: &[QuadratureLoad],
    transfer: &TransferResult,
    target: &SurfaceMesh,
    nodal_displacements_m: &[Vec3],
) -> Result<(f64, f64, f64), String> {
    if transfer.hits.len() != loads.len() {
        return Err("transfer hit count does not match source quadrature count".to_owned());
    }
    if nodal_displacements_m.len() != target.nodes_m.len() {
        return Err("displacement count does not match target node count".to_owned());
    }
    if nodal_displacements_m.iter().any(|value| !value.is_finite()) {
        return Err("target displacement field contains a non-finite vector".to_owned());
    }

    let mut source_work_j = 0.0;
    for (load, hit) in loads.iter().zip(&transfer.hits) {
        let point_displacement = interpolated_displacement(*hit, nodal_displacements_m);
        source_work_j += load.force_n.dot(point_displacement);
    }

    let mut target_work_j = 0.0;
    for (force, displacement) in transfer.nodal_forces_n.iter().zip(nodal_displacements_m) {
        target_work_j += force.dot(*displacement);
    }

    let mut affine_interpolation_error_m: f64 = 0.0;
    for (load, hit) in loads.iter().zip(&transfer.hits) {
        let exact = affine_displacement(load.position_m);
        let interpolated = interpolated_displacement(*hit, nodal_displacements_m);
        affine_interpolation_error_m =
            affine_interpolation_error_m.max(exact.max_abs_delta(interpolated));
    }

    Ok((source_work_j, target_work_j, affine_interpolation_error_m))
}

fn fixture_meshes() -> (SurfaceMesh, SurfaceMesh, Vec<Vec3>) {
    let source_nodes_m = vec![
        Vec3([0.0, 0.0, 0.0]),
        Vec3([1.0, 0.0, 0.0]),
        Vec3([1.0, 1.0, 0.0]),
        Vec3([0.0, 1.0, 0.0]),
        Vec3([0.5, 0.5, 0.0]),
    ];
    let source = SurfaceMesh {
        nodes_m: source_nodes_m,
        triangles: vec![[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]],
    };
    let target = SurfaceMesh {
        nodes_m: vec![
            Vec3([0.0, 0.0, 0.0]),
            Vec3([1.0, 0.0, 0.0]),
            Vec3([1.0, 1.0, 0.0]),
            Vec3([0.0, 1.0, 0.0]),
        ],
        triangles: vec![[0, 1, 2], [0, 2, 3]],
    };
    let nodal_traction_n_per_m2 = vec![
        Vec3([1_000.0, -500.0, 100.0]),
        Vec3([1_120.0, -480.0, 110.0]),
        Vec3([1_170.0, -560.0, 140.0]),
        Vec3([1_050.0, -580.0, 130.0]),
        Vec3([1_085.0, -530.0, 120.0]),
    ];
    (source, target, nodal_traction_n_per_m2)
}

fn solve_fixture() -> Result<
    (
        Vec<QuadratureLoad>,
        TransferResult,
        Resultants,
        Resultants,
        f64,
        f64,
        f64,
    ),
    String,
> {
    let (source_mesh, target_mesh, tractions) = fixture_meshes();
    let loads = integrate_linear_surface_traction(&source_mesh, &tractions)?;
    let transferred = transfer_loads(&loads, &target_mesh)?;
    let source_total = source_resultants(&loads);
    let target_total = target_resultants(&target_mesh, &transferred.nodal_forces_n)?;
    let displacements = target_mesh
        .nodes_m
        .iter()
        .copied()
        .map(affine_displacement)
        .collect::<Vec<_>>();
    let (source_work, target_work, affine_error) =
        virtual_work(&loads, &transferred, &target_mesh, &displacements)?;
    Ok((
        loads,
        transferred,
        source_total,
        target_total,
        source_work,
        target_work,
        affine_error,
    ))
}

fn json_vec3(value: Vec3) -> String {
    format!(
        "[{:.17e},{:.17e},{:.17e}]",
        value.0[0], value.0[1], value.0[2]
    )
}

fn json_usize_array(values: &[usize]) -> String {
    let parts = values.iter().map(usize::to_string).collect::<Vec<_>>();
    format!("[{}]", parts.join(","))
}

fn result_json() -> Result<String, String> {
    let (loads, transfer, source_total, target_total, source_work, target_work, affine_error) =
        solve_fixture()?;
    let force_residual = source_total.force_n.max_abs_delta(target_total.force_n);
    let moment_residual = source_total.moment_nm.max_abs_delta(target_total.moment_nm);
    let work_residual = (source_work - target_work).abs();

    let mut output = String::new();
    write!(
        output,
        concat!(
            "{{",
            "\"schema_version\":\"aero-conservative-surface-transfer/0.1.0\",",
            "\"claim_ceiling\":\"coplanar_synthetic_surface_transfer_verification_only\",",
            "\"aircraft_feasibility\":\"NOT_ASSESSED\",",
            "\"source_node_count\":5,",
            "\"source_triangle_count\":4,",
            "\"target_node_count\":4,",
            "\"target_triangle_count\":2,",
            "\"quadrature_point_count\":{},",
            "\"target_hit_triangle_indices\":{},",
            "\"source_force_n\":{},",
            "\"target_force_n\":{},",
            "\"source_moment_nm\":{},",
            "\"target_moment_nm\":{},",
            "\"force_residual_n\":{:.17e},",
            "\"moment_residual_nm\":{:.17e},",
            "\"source_virtual_work_j\":{:.17e},",
            "\"target_virtual_work_j\":{:.17e},",
            "\"virtual_work_residual_j\":{:.17e},",
            "\"affine_displacement_interpolation_error_m\":{:.17e},",
            "\"unsupported_point_control\":\"REJECTED\",",
            "\"mesh_geometry_scope\":\"coplanar_nonmatching_triangular_surfaces\",",
            "\"unassessed_claims\":[",
            "\"curved_or_noncoincident_surface_transfer\",",
            "\"general_quadrilateral_and_higher_order_elements\",",
            "\"full_surface_mortar_or_dual_mortar_projection\",",
            "\"dynamic_fsi_and_unsteady_load_transfer\",",
            "\"aircraft_specific_structural_validity\",",
            "\"airworthiness_and_certification\"]",
            "}}"
        ),
        loads.len(),
        json_usize_array(
            &transfer
                .hits
                .iter()
                .map(|hit| hit.triangle_index)
                .collect::<Vec<_>>(),
        ),
        json_vec3(source_total.force_n),
        json_vec3(target_total.force_n),
        json_vec3(source_total.moment_nm),
        json_vec3(target_total.moment_nm),
        force_residual,
        moment_residual,
        source_work,
        target_work,
        work_residual,
        affine_error,
    )
    .map_err(|error| error.to_string())?;
    Ok(output)
}

#[cfg(test)]
fn rotate_z(value: Vec3) -> Vec3 {
    Vec3([-value.0[1], value.0[0], value.0[2]])
}

#[cfg(test)]
fn rigid_transform_mesh(mesh: &SurfaceMesh, translation_m: Vec3) -> SurfaceMesh {
    SurfaceMesh {
        nodes_m: mesh
            .nodes_m
            .iter()
            .map(|node| rotate_z(*node).add(translation_m))
            .collect(),
        triangles: mesh.triangles.clone(),
    }
}

#[cfg(test)]
fn rigid_transform_loads(loads: &[QuadratureLoad], translation_m: Vec3) -> Vec<QuadratureLoad> {
    loads
        .iter()
        .map(|load| QuadratureLoad {
            position_m: rotate_z(load.position_m).add(translation_m),
            force_n: rotate_z(load.force_n),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, relative_tolerance: f64) {
        let threshold = 1.0e-12 + relative_tolerance * expected.abs();
        assert!(
            (actual - expected).abs() <= threshold,
            "actual={actual:.16e}, expected={expected:.16e}, threshold={threshold:.16e}"
        );
    }

    #[test]
    fn linear_traction_triangle_quadrature_matches_locked_resultants() {
        let (_, _, source, _, _, _, _) = solve_fixture().expect("fixture must solve");
        close(source.force_n.0[0], 1_085.0, 1.0e-12);
        close(source.force_n.0[1], -530.0, 1.0e-12);
        close(source.force_n.0[2], 120.0, 1.0e-12);
        close(source.moment_nm.0[0], 62.5, 1.0e-12);
        close(source.moment_nm.0[1], -60.833333333333336, 1.0e-12);
        close(source.moment_nm.0[2], -810.0, 1.0e-12);
    }

    #[test]
    fn nonmatching_mesh_transfer_preserves_force_and_moment() {
        let (_, _, source, target, _, _, _) = solve_fixture().expect("fixture must solve");
        assert!(source.force_n.max_abs_delta(target.force_n) <= 1.0e-10);
        assert!(source.moment_nm.max_abs_delta(target.moment_nm) <= 1.0e-10);
    }

    #[test]
    fn dual_transfer_preserves_virtual_work_for_target_nodal_variations() {
        let (loads, transfer, _, _, source_work, target_work, affine_error) =
            solve_fixture().expect("fixture must solve");
        assert_eq!(loads.len(), 12);
        assert_eq!(transfer.hits.len(), loads.len());
        close(source_work, 2.0794166666666665, 1.0e-12);
        close(target_work, source_work, 1.0e-12);
        assert!(affine_error <= 1.0e-12);
    }

    #[test]
    fn rigid_rotation_and_translation_preserve_equilibrium() {
        let (source, target, tractions) = fixture_meshes();
        let loads = integrate_linear_surface_traction(&source, &tractions).expect("loads");
        let original = source_resultants(&loads);
        let translation = Vec3([3.0, -2.0, 0.5]);
        let moved_source = rigid_transform_mesh(&source, translation);
        let moved_target = rigid_transform_mesh(&target, translation);
        let moved_loads = rigid_transform_loads(&loads, translation);
        let mapped = transfer_loads(&moved_loads, &moved_target).expect("transformed transfer");
        let mapped_total =
            target_resultants(&moved_target, &mapped.nodal_forces_n).expect("target resultants");
        let moved_source_total = source_resultants(&moved_loads);
        let expected_force = rotate_z(original.force_n);
        let expected_moment = rotate_z(original.moment_nm).add(translation.cross(expected_force));
        assert!(moved_source_total.force_n.max_abs_delta(expected_force) <= 1.0e-10);
        assert!(mapped_total.force_n.max_abs_delta(expected_force) <= 1.0e-10);
        assert!(moved_source_total.moment_nm.max_abs_delta(expected_moment) <= 1.0e-9);
        assert!(mapped_total.moment_nm.max_abs_delta(expected_moment) <= 1.0e-9);
        assert_eq!(moved_source.triangles, source.triangles);
    }

    #[test]
    fn out_of_support_points_are_rejected_not_extrapolated() {
        let (_, target, _) = fixture_meshes();
        let outside = QuadratureLoad {
            position_m: Vec3([1.1, 0.5, 0.0]),
            force_n: Vec3([1.0, 0.0, 0.0]),
        };
        assert!(transfer_loads(&[outside], &target).is_err());
    }

    #[test]
    fn off_surface_points_are_rejected() {
        let (_, target, _) = fixture_meshes();
        let off_surface = QuadratureLoad {
            position_m: Vec3([0.5, 0.5, 0.01]),
            force_n: Vec3([1.0, 0.0, 0.0]),
        };
        assert!(transfer_loads(&[off_surface], &target).is_err());
    }

    #[test]
    fn degenerate_triangles_are_rejected() {
        let degenerate = SurfaceMesh {
            nodes_m: vec![
                Vec3([0.0, 0.0, 0.0]),
                Vec3([1.0, 0.0, 0.0]),
                Vec3([2.0, 0.0, 0.0]),
            ],
            triangles: vec![[0, 1, 2]],
        };
        assert!(validate_mesh(&degenerate).is_err());
    }

    #[test]
    fn invalid_source_traction_count_and_nonfinite_fields_are_rejected() {
        let (source, _, tractions) = fixture_meshes();
        assert!(integrate_linear_surface_traction(&source, &tractions[..4]).is_err());
        let mut nonfinite = tractions.clone();
        nonfinite[2] = Vec3([f64::NAN, 0.0, 0.0]);
        assert!(integrate_linear_surface_traction(&source, &nonfinite).is_err());
    }
}

fn main() {
    match result_json() {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("surface_transfer_oracle_error={error}");
            std::process::exit(2);
        }
    }
}
