// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Work-conjugate surface load/displacement transfer verification.
// This is a bounded planar triangular-patch benchmark, not an FSI solver.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const FIXTURE_TEXT: &str = include_str!("../data/nonmatching-planar-patch-v0.1.json");

type Vec3 = [f64; 3];

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Surface {
    nodes_xyz_m: Vec<Vec3>,
    triangles: Vec<[usize; 3]>,
    expected_face_normals: Vec<Vec3>,
    #[serde(default)]
    nodal_traction_pa: Vec<Vec3>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Fixture {
    schema_version: String,
    configuration_sha256: String,
    geometry_sha256: String,
    loading_state_sha256: String,
    mission_phase_sha256: String,
    environment_sha256: String,
    moment_reference_origin_m: Vec3,
    geometry_tolerance_m: f64,
    source_surface: Surface,
    target_surface: Surface,
    target_nodal_displacements_m: Vec<Vec3>,
}

#[derive(Clone, Debug, PartialEq)]
struct TransferError(String);

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TransferError {}

type Result<T> = std::result::Result<T, TransferError>;

fn fail<T>(message: impl Into<String>) -> Result<T> {
    Err(TransferError(message.into()))
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: Vec3, s: f64) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}

fn unit(a: Vec3, label: &str) -> Result<Vec3> {
    let length = norm(a);
    if !length.is_finite() || length <= 1.0e-14 {
        return fail(format!("{label}: zero or non-finite vector"));
    }
    Ok(scale(a, 1.0 / length))
}

fn finite_vec(a: Vec3) -> bool {
    a.iter().all(|value| value.is_finite())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn digest_hex(digest: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing into String cannot fail");
    }
    output
}

fn hash_bytes(bytes: &[u8]) -> String {
    digest_hex(&Sha256::digest(bytes))
}

fn hash_surface(surface: &Surface, include_traction: bool) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"aero-surface-v0.1\0");
    hasher.update((surface.nodes_xyz_m.len() as u64).to_le_bytes());
    for node in &surface.nodes_xyz_m {
        for value in node {
            hasher.update(value.to_le_bytes());
        }
    }
    hasher.update((surface.triangles.len() as u64).to_le_bytes());
    for face in &surface.triangles {
        for index in face {
            hasher.update((*index as u64).to_le_bytes());
        }
    }
    for normal in &surface.expected_face_normals {
        for value in normal {
            hasher.update(value.to_le_bytes());
        }
    }
    if include_traction {
        hasher.update(b"traction-pa\0");
        for traction in &surface.nodal_traction_pa {
            for value in traction {
                hasher.update(value.to_le_bytes());
            }
        }
    }
    digest_hex(&hasher.finalize())
}

fn hash_displacements(values: &[Vec3]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"aero-target-displacements-m-v0.1\0");
    for value in values {
        for component in value {
            hasher.update(component.to_le_bytes());
        }
    }
    hash_bytes(&hasher.finalize())
}

fn validate_identity(fixture: &Fixture) -> Result<()> {
    if fixture.schema_version != "aero-work-conjugate-transfer/0.1.0" {
        return fail("unsupported transfer fixture schema");
    }
    for (label, digest) in [
        ("configuration", &fixture.configuration_sha256),
        ("geometry", &fixture.geometry_sha256),
        ("loading state", &fixture.loading_state_sha256),
        ("mission phase", &fixture.mission_phase_sha256),
        ("environment", &fixture.environment_sha256),
    ] {
        if !valid_sha256(digest) {
            return fail(format!("{label} identity is not a lowercase SHA-256"));
        }
    }
    if !finite_vec(fixture.moment_reference_origin_m) {
        return fail("moment reference origin contains non-finite values");
    }
    if !fixture.geometry_tolerance_m.is_finite() || fixture.geometry_tolerance_m <= 0.0 {
        return fail("geometry tolerance must be finite and positive");
    }
    Ok(())
}

fn validate_surface(surface: &Surface, label: &str, require_traction: bool) -> Result<()> {
    if surface.nodes_xyz_m.len() < 3 {
        return fail(format!("{label}: at least three nodes are required"));
    }
    if surface.triangles.is_empty() {
        return fail(format!("{label}: no triangular faces"));
    }
    if surface.expected_face_normals.len() != surface.triangles.len() {
        return fail(format!(
            "{label}: expected face-normal count does not match triangle count"
        ));
    }
    if require_traction && surface.nodal_traction_pa.len() != surface.nodes_xyz_m.len() {
        return fail(format!(
            "{label}: nodal traction count does not match node count"
        ));
    }
    if surface.nodes_xyz_m.iter().any(|node| !finite_vec(*node)) {
        return fail(format!("{label}: non-finite node coordinate"));
    }
    if require_traction
        && surface
            .nodal_traction_pa
            .iter()
            .any(|value| !finite_vec(*value))
    {
        return fail(format!("{label}: non-finite nodal traction"));
    }

    let mut unique_faces = BTreeSet::new();
    for (face_index, face) in surface.triangles.iter().enumerate() {
        if face.iter().any(|index| *index >= surface.nodes_xyz_m.len()) {
            return fail(format!(
                "{label}: face {face_index} has an out-of-range node index"
            ));
        }
        if face[0] == face[1] || face[1] == face[2] || face[2] == face[0] {
            return fail(format!("{label}: face {face_index} repeats a node index"));
        }
        let mut key = *face;
        key.sort_unstable();
        if !unique_faces.insert(key) {
            return fail(format!(
                "{label}: duplicate triangle connectivity at face {face_index}"
            ));
        }

        let a = surface.nodes_xyz_m[face[0]];
        let b = surface.nodes_xyz_m[face[1]];
        let c = surface.nodes_xyz_m[face[2]];
        let area_vector = cross(sub(b, a), sub(c, a));
        let area = 0.5 * norm(area_vector);
        if !area.is_finite() || area <= 1.0e-14 {
            return fail(format!("{label}: degenerate triangle {face_index}"));
        }
        let actual_normal = unit(area_vector, "face normal")?;
        let declared_normal = unit(
            surface.expected_face_normals[face_index],
            "declared face normal",
        )?;
        if dot(actual_normal, declared_normal) < 1.0 - 1.0e-10 {
            return fail(format!(
                "{label}: face {face_index} winding disagrees with declared outward normal"
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct QuadratureLoad {
    position: Vec3,
    force_n: Vec3,
    source_face: usize,
}

#[derive(Clone, Copy, Debug)]
struct TargetShape {
    face_index: usize,
    nodes: [usize; 3],
    weights: [f64; 3],
    plane_distance_m: f64,
}

fn triangle_area_normal(surface: &Surface, face: [usize; 3]) -> Result<(f64, Vec3)> {
    let a = surface.nodes_xyz_m[face[0]];
    let b = surface.nodes_xyz_m[face[1]];
    let c = surface.nodes_xyz_m[face[2]];
    let area_vector = cross(sub(b, a), sub(c, a));
    let area = 0.5 * norm(area_vector);
    if !area.is_finite() || area <= 1.0e-14 {
        return fail("degenerate triangle in area calculation");
    }
    Ok((area, unit(area_vector, "triangle normal")?))
}

fn source_quadrature(source: &Surface) -> Result<Vec<QuadratureLoad>> {
    // Three-point degree-2 barycentric quadrature integrates linear traction
    // and its first moment exactly on a planar linear triangle.
    const BARY: [[f64; 3]; 3] = [
        [2.0 / 3.0, 1.0 / 6.0, 1.0 / 6.0],
        [1.0 / 6.0, 2.0 / 3.0, 1.0 / 6.0],
        [1.0 / 6.0, 1.0 / 6.0, 2.0 / 3.0],
    ];
    let mut output = Vec::new();
    for (face_index, face) in source.triangles.iter().enumerate() {
        let (area, _normal) = triangle_area_normal(source, *face)?;
        let nodes = [
            source.nodes_xyz_m[face[0]],
            source.nodes_xyz_m[face[1]],
            source.nodes_xyz_m[face[2]],
        ];
        let tractions = [
            source.nodal_traction_pa[face[0]],
            source.nodal_traction_pa[face[1]],
            source.nodal_traction_pa[face[2]],
        ];
        for bary in BARY {
            let mut position = [0.0; 3];
            let mut traction = [0.0; 3];
            for local in 0..3 {
                position = add(position, scale(nodes[local], bary[local]));
                traction = add(traction, scale(tractions[local], bary[local]));
            }
            output.push(QuadratureLoad {
                position,
                force_n: scale(traction, area / 3.0),
                source_face: face_index,
            });
        }
    }
    Ok(output)
}

fn barycentric_projection(
    point: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    tolerance_m: f64,
) -> Option<([f64; 3], f64)> {
    let v0 = sub(b, a);
    let v1 = sub(c, a);
    let normal = unit(cross(v0, v1), "target projection normal").ok()?;
    let signed_distance = dot(sub(point, a), normal);
    let projected = sub(point, scale(normal, signed_distance));
    let v2 = sub(projected, a);
    let d00 = dot(v0, v0);
    let d01 = dot(v0, v1);
    let d11 = dot(v1, v1);
    let d20 = dot(v2, v0);
    let d21 = dot(v2, v1);
    let denominator = d00 * d11 - d01 * d01;
    if denominator.abs() <= 1.0e-20 {
        return None;
    }
    let w1 = (d11 * d20 - d01 * d21) / denominator;
    let w2 = (d00 * d21 - d01 * d20) / denominator;
    let w0 = 1.0 - w1 - w2;
    let mut weights = [w0, w1, w2];
    let bary_tol = (tolerance_m * 1.0e3).max(1.0e-10);
    if weights
        .iter()
        .any(|weight| *weight < -bary_tol || *weight > 1.0 + bary_tol)
    {
        return None;
    }
    if signed_distance.abs() > tolerance_m {
        return None;
    }
    for weight in &mut weights {
        if *weight < 0.0 {
            *weight = 0.0;
        }
    }
    let sum: f64 = weights.iter().sum();
    if sum <= 0.0 {
        return None;
    }
    for weight in &mut weights {
        *weight /= sum;
    }
    Some((weights, signed_distance.abs()))
}

fn locate_on_target(point: Vec3, target: &Surface, tolerance_m: f64) -> Result<TargetShape> {
    let mut best: Option<TargetShape> = None;
    for (face_index, face) in target.triangles.iter().enumerate() {
        let a = target.nodes_xyz_m[face[0]];
        let b = target.nodes_xyz_m[face[1]];
        let c = target.nodes_xyz_m[face[2]];
        if let Some((weights, distance)) = barycentric_projection(point, a, b, c, tolerance_m) {
            let candidate = TargetShape {
                face_index,
                nodes: *face,
                weights,
                plane_distance_m: distance,
            };
            if best
                .as_ref()
                .map(|existing| distance < existing.plane_distance_m)
                .unwrap_or(true)
            {
                best = Some(candidate);
            }
        }
    }
    best.ok_or_else(|| {
        TransferError(
            "source quadrature point is outside target support or geometry tolerance".to_owned(),
        )
    })
}

fn resultant_force(loads: &[QuadratureLoad]) -> Vec3 {
    loads
        .iter()
        .fold([0.0; 3], |total, load| add(total, load.force_n))
}

fn resultant_moment(loads: &[QuadratureLoad], origin: Vec3) -> Vec3 {
    loads.iter().fold([0.0; 3], |total, load| {
        add(total, cross(sub(load.position, origin), load.force_n))
    })
}

fn nodal_force_resultants(nodes: &[Vec3], forces: &[Vec3], origin: Vec3) -> (Vec3, Vec3) {
    let force = forces
        .iter()
        .fold([0.0; 3], |total, value| add(total, *value));
    let moment = nodes
        .iter()
        .zip(forces)
        .fold([0.0; 3], |total, (position, value)| {
            add(total, cross(sub(*position, origin), *value))
        });
    (force, moment)
}

fn norm_residual(a: Vec3, b: Vec3) -> f64 {
    norm(sub(a, b))
}

fn work_conjugate_transfer(
    fixture: &Fixture,
    expected_source_sha256: &str,
    expected_target_sha256: &str,
) -> Result<TransferReport> {
    validate_identity(fixture)?;
    validate_surface(&fixture.source_surface, "source surface", true)?;
    validate_surface(&fixture.target_surface, "target surface", false)?;
    if fixture.target_nodal_displacements_m.len() != fixture.target_surface.nodes_xyz_m.len() {
        return fail("target displacement count does not match target node count");
    }
    if fixture
        .target_nodal_displacements_m
        .iter()
        .any(|value| !finite_vec(*value))
    {
        return fail("target displacement field contains non-finite values");
    }

    let source_mesh_sha256 = hash_surface(&fixture.source_surface, false);
    let target_mesh_sha256 = hash_surface(&fixture.target_surface, false);
    if source_mesh_sha256 != expected_source_sha256 {
        return fail("source mesh digest mismatch");
    }
    if target_mesh_sha256 != expected_target_sha256 {
        return fail("target mesh digest mismatch");
    }

    let source_loads = source_quadrature(&fixture.source_surface)?;
    if source_loads.is_empty() {
        return fail("source load quadrature is empty");
    }

    let mut target_forces = vec![[0.0; 3]; fixture.target_surface.nodes_xyz_m.len()];
    let mut target_nearest_forces = vec![[0.0; 3]; fixture.target_surface.nodes_xyz_m.len()];
    let mut source_work = 0.0;
    let mut target_work = 0.0;
    let mut nearest_target_work = 0.0;
    let mut maximum_projection_distance = 0.0_f64;
    let mut mapped_quadrature_points = 0usize;
    let mut face_usage = vec![0usize; fixture.target_surface.triangles.len()];

    for load in &source_loads {
        let shape = locate_on_target(
            load.position,
            &fixture.target_surface,
            fixture.geometry_tolerance_m,
        )?;
        mapped_quadrature_points += 1;
        face_usage[shape.face_index] += 1;
        maximum_projection_distance = maximum_projection_distance.max(shape.plane_distance_m);

        let mut interpolated_displacement = [0.0; 3];
        for local in 0..3 {
            let node = shape.nodes[local];
            let contribution = scale(load.force_n, shape.weights[local]);
            target_forces[node] = add(target_forces[node], contribution);
            interpolated_displacement = add(
                interpolated_displacement,
                scale(
                    fixture.target_nodal_displacements_m[node],
                    shape.weights[local],
                ),
            );
        }
        source_work += dot(load.force_n, interpolated_displacement);

        // Deliberately mismatched nearest-node force transfer is retained as a
        // negative baseline. It is compared against the same FE displacement
        // interpolation to expose loss of force/displacement work conjugacy.
        let (nearest_index, _) = fixture
            .target_surface
            .nodes_xyz_m
            .iter()
            .enumerate()
            .map(|(index, node)| (index, norm(sub(*node, load.position))))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .ok_or_else(|| TransferError("target mesh has no nodes".to_owned()))?;
        target_nearest_forces[nearest_index] =
            add(target_nearest_forces[nearest_index], load.force_n);
    }

    for (force, displacement) in target_forces
        .iter()
        .zip(&fixture.target_nodal_displacements_m)
    {
        target_work += dot(*force, *displacement);
    }
    for (force, displacement) in target_nearest_forces
        .iter()
        .zip(&fixture.target_nodal_displacements_m)
    {
        nearest_target_work += dot(*force, *displacement);
    }

    let origin = fixture.moment_reference_origin_m;
    let source_force = resultant_force(&source_loads);
    let source_moment = resultant_moment(&source_loads, origin);
    let (target_force, target_moment) =
        nodal_force_resultants(&fixture.target_surface.nodes_xyz_m, &target_forces, origin);
    let (nearest_force, nearest_moment) = nodal_force_resultants(
        &fixture.target_surface.nodes_xyz_m,
        &target_nearest_forces,
        origin,
    );

    let force_scale = norm(source_force).max(1.0e-12);
    let moment_scale = norm(source_moment)
        .max(force_scale * characteristic_length(&fixture.target_surface))
        .max(1.0e-12);
    let source_traction_sha256 = hash_surface(&fixture.source_surface, true);
    let target_displacement_sha256 = hash_displacements(&fixture.target_nodal_displacements_m);

    Ok(TransferReport {
        schema_version: "aero-work-conjugate-transfer-result/0.1.0".to_owned(),
        method: "piecewise_linear_consistent_triangle_transfer".to_owned(),
        identity: EvidenceIdentity {
            configuration_sha256: fixture.configuration_sha256.clone(),
            geometry_sha256: fixture.geometry_sha256.clone(),
            loading_state_sha256: fixture.loading_state_sha256.clone(),
            mission_phase_sha256: fixture.mission_phase_sha256.clone(),
            environment_sha256: fixture.environment_sha256.clone(),
        },
        source_mesh_sha256,
        target_mesh_sha256,
        source_traction_sha256,
        target_displacement_sha256,
        source_quadrature_points: source_loads.len(),
        mapped_quadrature_points,
        target_faces_used: face_usage.iter().filter(|count| **count > 0).count(),
        maximum_projection_distance_m: maximum_projection_distance,
        source_resultant_force_n: source_force,
        target_resultant_force_n: target_force,
        source_resultant_moment_nm: source_moment,
        target_resultant_moment_nm: target_moment,
        force_residual_n: norm_residual(source_force, target_force),
        moment_residual_nm: norm_residual(source_moment, target_moment),
        normalized_force_residual: norm_residual(source_force, target_force) / force_scale,
        normalized_moment_residual: norm_residual(source_moment, target_moment) / moment_scale,
        source_virtual_work_j: source_work,
        target_virtual_work_j: target_work,
        virtual_work_residual_j: (source_work - target_work).abs(),
        nearest_neighbor_baseline: BaselineReport {
            resultant_force_n: nearest_force,
            resultant_moment_nm: nearest_moment,
            force_residual_n: norm_residual(source_force, nearest_force),
            moment_residual_nm: norm_residual(source_moment, nearest_moment),
            source_virtual_work_j: source_work,
            target_virtual_work_j: nearest_target_work,
            virtual_work_residual_j: (source_work - nearest_target_work).abs(),
        },
        coverage_fraction: mapped_quadrature_points as f64 / source_loads.len() as f64,
        units_status: "SI_DECLARED_FOR_SYNTHETIC_FIXTURE".to_owned(),
        surface_overlap_status: "ALL_SOURCE_QUADRATURE_POINTS_INSIDE_TARGET_TRIANGLES".to_owned(),
        solver_result: "NOT_ASSESSED".to_owned(),
        structural_response: "NOT_ASSESSED".to_owned(),
        claim_ceiling: "planar_triangular_patch_work_conjugate_transfer_only".to_owned(),
    })
}

fn characteristic_length(surface: &Surface) -> f64 {
    let mut maximum = 0.0_f64;
    for a in 0..surface.nodes_xyz_m.len() {
        for b in (a + 1)..surface.nodes_xyz_m.len() {
            maximum = maximum.max(norm(sub(surface.nodes_xyz_m[a], surface.nodes_xyz_m[b])));
        }
    }
    maximum
}

#[derive(Clone, Debug, Serialize)]
struct EvidenceIdentity {
    configuration_sha256: String,
    geometry_sha256: String,
    loading_state_sha256: String,
    mission_phase_sha256: String,
    environment_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct BaselineReport {
    resultant_force_n: Vec3,
    resultant_moment_nm: Vec3,
    force_residual_n: f64,
    moment_residual_nm: f64,
    source_virtual_work_j: f64,
    target_virtual_work_j: f64,
    virtual_work_residual_j: f64,
}

#[derive(Clone, Debug, Serialize)]
struct TransferReport {
    schema_version: String,
    method: String,
    identity: EvidenceIdentity,
    source_mesh_sha256: String,
    target_mesh_sha256: String,
    source_traction_sha256: String,
    target_displacement_sha256: String,
    source_quadrature_points: usize,
    mapped_quadrature_points: usize,
    target_faces_used: usize,
    maximum_projection_distance_m: f64,
    source_resultant_force_n: Vec3,
    target_resultant_force_n: Vec3,
    source_resultant_moment_nm: Vec3,
    target_resultant_moment_nm: Vec3,
    force_residual_n: f64,
    moment_residual_nm: f64,
    normalized_force_residual: f64,
    normalized_moment_residual: f64,
    source_virtual_work_j: f64,
    target_virtual_work_j: f64,
    virtual_work_residual_j: f64,
    nearest_neighbor_baseline: BaselineReport,
    coverage_fraction: f64,
    units_status: String,
    surface_overlap_status: String,
    solver_result: String,
    structural_response: String,
    claim_ceiling: String,
}

fn rotate_z(v: Vec3) -> Vec3 {
    [-v[1], v[0], v[2]]
}

fn transform_point(v: Vec3) -> Vec3 {
    add(rotate_z(v), [4.0, -2.0, 3.0])
}

fn transform_fixture(original: &Fixture) -> Fixture {
    let mut transformed = original.clone();
    transformed.moment_reference_origin_m = transform_point(original.moment_reference_origin_m);

    for node in &mut transformed.source_surface.nodes_xyz_m {
        *node = transform_point(*node);
    }
    for node in &mut transformed.target_surface.nodes_xyz_m {
        *node = transform_point(*node);
    }
    for normal in &mut transformed.source_surface.expected_face_normals {
        *normal = rotate_z(*normal);
    }
    for normal in &mut transformed.target_surface.expected_face_normals {
        *normal = rotate_z(*normal);
    }
    for traction in &mut transformed.source_surface.nodal_traction_pa {
        *traction = rotate_z(*traction);
    }
    for displacement in &mut transformed.target_nodal_displacements_m {
        *displacement = rotate_z(*displacement);
    }
    transformed
}

fn parse_fixture() -> Fixture {
    serde_json::from_str(FIXTURE_TEXT).expect("embedded transfer fixture must be valid JSON")
}

fn main() {
    let fixture = parse_fixture();
    let source_sha = hash_surface(&fixture.source_surface, false);
    let target_sha = hash_surface(&fixture.target_surface, false);
    match work_conjugate_transfer(&fixture, &source_sha, &target_sha) {
        Ok(report) => println!(
            "{}",
            serde_json::to_string(&report).expect("transfer report serialization must succeed")
        ),
        Err(error) => {
            eprintln!("work_conjugate_transfer_error={error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> (Fixture, TransferReport) {
        let fixture = parse_fixture();
        let source_sha = hash_surface(&fixture.source_surface, false);
        let target_sha = hash_surface(&fixture.target_surface, false);
        let report =
            work_conjugate_transfer(&fixture, &source_sha, &target_sha).expect("fixture maps");
        (fixture, report)
    }

    #[test]
    fn nonmatching_consistent_transfer_preserves_force_moment_and_work() {
        let (_fixture, report) = baseline();
        assert_eq!(report.source_quadrature_points, 6);
        assert_eq!(report.mapped_quadrature_points, 6);
        assert_eq!(report.target_faces_used, 4);
        assert_eq!(report.coverage_fraction, 1.0);
        assert!(report.normalized_force_residual < 1.0e-12);
        assert!(report.normalized_moment_residual < 1.0e-12);
        assert!(report.virtual_work_residual_j < 1.0e-12);
        assert_eq!(report.solver_result, "NOT_ASSESSED");
        assert_eq!(report.structural_response, "NOT_ASSESSED");
    }

    #[test]
    fn nearest_node_baseline_preserves_force_but_fails_moment_and_work() {
        let (_fixture, report) = baseline();
        assert!(report.nearest_neighbor_baseline.force_residual_n < 1.0e-12);
        assert!(report.nearest_neighbor_baseline.moment_residual_nm > 1.0e-4);
        assert!(report.nearest_neighbor_baseline.virtual_work_residual_j > 1.0e-4);
    }

    #[test]
    fn stale_source_mesh_digest_is_rejected() {
        let fixture = parse_fixture();
        let wrong = "0".repeat(64);
        let target = hash_surface(&fixture.target_surface, false);
        assert!(work_conjugate_transfer(&fixture, &wrong, &target).is_err());
    }

    #[test]
    fn stale_target_mesh_digest_is_rejected() {
        let fixture = parse_fixture();
        let source = hash_surface(&fixture.source_surface, false);
        let wrong = "f".repeat(64);
        assert!(work_conjugate_transfer(&fixture, &source, &wrong).is_err());
    }

    #[test]
    fn uncovered_source_patch_abstains_without_extrapolation() {
        let mut fixture = parse_fixture();
        for node in &mut fixture.target_surface.nodes_xyz_m {
            node[0] += 10.0;
        }
        let source = hash_surface(&fixture.source_surface, false);
        let target = hash_surface(&fixture.target_surface, false);
        assert!(work_conjugate_transfer(&fixture, &source, &target).is_err());
    }

    #[test]
    fn reversed_source_face_winding_is_rejected() {
        let mut fixture = parse_fixture();
        fixture.source_surface.triangles[0] = [0, 2, 1];
        let source = hash_surface(&fixture.source_surface, false);
        let target = hash_surface(&fixture.target_surface, false);
        assert!(work_conjugate_transfer(&fixture, &source, &target).is_err());
    }

    #[test]
    fn degenerate_target_triangle_is_rejected() {
        let mut fixture = parse_fixture();
        fixture.target_surface.triangles[0] = [0, 1, 1];
        let source = hash_surface(&fixture.source_surface, false);
        let target = hash_surface(&fixture.target_surface, false);
        assert!(work_conjugate_transfer(&fixture, &source, &target).is_err());
    }

    #[test]
    fn rigid_translation_and_rotation_preserve_transformed_resultants() {
        let (fixture, original) = baseline();
        let transformed_fixture = transform_fixture(&fixture);
        let source = hash_surface(&transformed_fixture.source_surface, false);
        let target = hash_surface(&transformed_fixture.target_surface, false);
        let transformed = work_conjugate_transfer(&transformed_fixture, &source, &target)
            .expect("rigid transform maps");

        let expected_force = rotate_z(original.source_resultant_force_n);
        let expected_moment = rotate_z(original.source_resultant_moment_nm);
        assert!(norm(sub(expected_force, transformed.source_resultant_force_n)) < 1.0e-10);
        assert!(norm(sub(expected_moment, transformed.source_resultant_moment_nm)) < 1.0e-10);
        assert!(transformed.normalized_force_residual < 1.0e-12);
        assert!(transformed.normalized_moment_residual < 1.0e-12);
        assert!(transformed.virtual_work_residual_j < 1.0e-12);
    }

    #[test]
    fn target_surface_duplicate_connectivity_is_rejected() {
        let mut fixture = parse_fixture();
        fixture
            .target_surface
            .triangles
            .push(fixture.target_surface.triangles[0]);
        fixture
            .target_surface
            .expected_face_normals
            .push([0.0, 0.0, 1.0]);
        let source = hash_surface(&fixture.source_surface, false);
        let target = hash_surface(&fixture.target_surface, false);
        assert!(work_conjugate_transfer(&fixture, &source, &target).is_err());
    }

    #[test]
    fn invalid_geometry_tolerance_is_rejected() {
        let mut fixture = parse_fixture();
        fixture.geometry_tolerance_m = 0.0;
        let source = hash_surface(&fixture.source_surface, false);
        let target = hash_surface(&fixture.target_surface, false);
        assert!(work_conjugate_transfer(&fixture, &source, &target).is_err());
    }

    #[test]
    fn identity_change_is_not_silently_accepted() {
        let mut fixture = parse_fixture();
        fixture.loading_state_sha256 = "not-a-digest".to_owned();
        let source = hash_surface(&fixture.source_surface, false);
        let target = hash_surface(&fixture.target_surface, false);
        assert!(work_conjugate_transfer(&fixture, &source, &target).is_err());
    }
}
