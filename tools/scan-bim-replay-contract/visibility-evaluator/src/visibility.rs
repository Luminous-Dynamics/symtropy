// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evaluator-only visibility derivation for Scan-to-BIM reference surfaces.
//!
//! This is a bounded triangle ray-query adapter, not a general ray-tracing
//! engine. Probe positions and represented areas are caller-supplied and must
//! come from a separately declared deterministic sampling profile. The
//! evaluator must keep reference geometry, occluders and these results outside
//! candidate-visible inputs.

use scan_bim_replay_contract::metrics::Point3;
use scan_bim_replay_contract::sampling::{
    SurfaceSample, SurfaceSamplingReceipt, SurfaceVisibility, verify_surface_sampling_receipt,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{PI, TAU};

const VECTOR_EPSILON: f64 = 1.0e-12;
const INTERSECTION_EPSILON_M: f64 = 1.0e-8;
const BASIS_TOLERANCE: f64 = 1.0e-8;
const DEGENERATE_TWICE_AREA_EPSILON: f64 = 1.0e-15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OccluderKind {
    SameBuilding,
    External,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OccluderCoverage {
    Complete,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewVisibilityStatus {
    Observable,
    Backface,
    OutsideSensorCoverage,
    SelfOccluded,
    ExternallyOccluded,
    Unknown,
    Contradictory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibilityProbe {
    pub probe_id: String,
    pub surface_id: String,
    pub triangle_id: String,
    pub point: Point3,
    /// Barycentric coordinates on the exact reference triangle (a, b, c).
    pub barycentric: [f64; 3],
    /// Outward normal in the same declared frame as point. The evaluator
    /// validates and normalizes it before the sensor-incidence test.
    pub normal: Point3,
    /// Area represented by this probe, in square metres. Probe generation
    /// and area conservation are caller-owned and must be receipt-bound.
    pub represented_area_m2: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibilityView {
    pub view_id: String,
    pub frame_id: String,
    pub origin: Point3,
    /// Orthonormal, right-handed camera/sensor basis in the shared frame.
    pub forward: Point3,
    pub right: Point3,
    pub up: Point3,
    pub horizontal_fov_rad: f64,
    pub vertical_fov_rad: f64,
    pub max_range_m: f64,
    pub pose_evidence_sha256: String,
    pub configuration_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OccluderTriangle {
    pub surface_id: String,
    pub triangle_id: String,
    pub a: Point3,
    pub b: Point3,
    pub c: Point3,
    pub kind: OccluderKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibilityProfile {
    /// Maximum number of probe × view × occluder ray/triangle tests. Exceeding
    /// this budget is an error; the evaluator never silently truncates.
    pub max_ray_tests: usize,
    /// Minimum cosine between the outward surface normal and the direction
    /// from the probe toward the sensor. Zero accepts only front-facing views.
    pub min_incidence_cosine: f64,
}

pub struct VisibilityEvaluation<'a> {
    pub scene_id: &'a str,
    pub frame_id: &'a str,
    pub reference_model_sha256: &'a str,
    pub probe_set_sha256: &'a str,
    pub occluder_manifest_sha256: &'a str,
    pub occluder_coverage: OccluderCoverage,
    pub probes: &'a [VisibilityProbe],
    pub views: &'a [VisibilityView],
    pub occluders: &'a [OccluderTriangle],
    pub profile: VisibilityProfile,
}

/// Complete evaluator input. Reference geometry, probe sets and occluder
/// inventory are evaluator-side artifacts and must never enter candidate code.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibilityRequest {
    pub scene_id: String,
    pub frame_id: String,
    pub reference_model_sha256: String,
    pub reference_sampling: SurfaceSamplingReceipt,
    pub occluder_manifest_sha256: String,
    pub occluder_coverage: OccluderCoverage,
    pub views: Vec<VisibilityView>,
    pub occluders: Vec<OccluderTriangle>,
    pub profile: VisibilityProfile,
}

impl VisibilityRequest {
    pub fn evaluate(&self) -> Result<VisibilityReceipt, String> {
        let probes = probes_from_reference_sampling(&self.reference_sampling)?;
        derive_surface_visibility(VisibilityEvaluation {
            scene_id: &self.scene_id,
            frame_id: &self.frame_id,
            reference_model_sha256: &self.reference_model_sha256,
            probe_set_sha256: &self.reference_sampling.sample_set_sha256,
            occluder_manifest_sha256: &self.occluder_manifest_sha256,
            occluder_coverage: self.occluder_coverage,
            probes: &probes,
            views: &self.views,
            occluders: &self.occluders,
            profile: self.profile,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ViewObservation {
    pub view_id: String,
    pub status: ViewVisibilityStatus,
    pub blocker_kind: Option<OccluderKind>,
    pub blocker_triangle_ids: Vec<String>,
    pub blocker_distance_m: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VisibilityProbeResult {
    pub probe_id: String,
    pub surface_id: String,
    pub triangle_id: String,
    pub point: Point3,
    pub normal: Point3,
    pub represented_area_m2: f64,
    pub visibility: SurfaceVisibility,
    pub views: Vec<ViewObservation>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VisibilitySurfaceArea {
    pub surface_id: String,
    pub observable_area_m2: f64,
    pub backface_area_m2: f64,
    pub self_occluded_area_m2: f64,
    pub externally_occluded_area_m2: f64,
    pub outside_sensor_coverage_area_m2: f64,
    pub unknown_area_m2: f64,
    pub contradictory_area_m2: f64,
    pub mixed_occlusion_area_m2: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VisibilityReceipt {
    pub algorithm_id: String,
    pub scene_id: String,
    pub frame_id: String,
    pub reference_model_sha256: String,
    pub probe_set_sha256: String,
    pub occluder_manifest_sha256: String,
    pub occluder_coverage: OccluderCoverage,
    pub min_incidence_cosine: f64,
    pub probe_count: usize,
    pub view_count: usize,
    pub ray_test_count: usize,
    pub observable_area_m2: f64,
    pub backface_area_m2: f64,
    pub self_occluded_area_m2: f64,
    pub externally_occluded_area_m2: f64,
    pub outside_sensor_coverage_area_m2: f64,
    pub unknown_area_m2: f64,
    pub contradictory_area_m2: f64,
    pub mixed_occlusion_area_m2: f64,
    pub surfaces: Vec<VisibilitySurfaceArea>,
    pub probes: Vec<VisibilityProbeResult>,
    pub visibility_sha256: String,
}

#[derive(Debug, Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn from_point(point: Point3) -> Self {
        Self {
            x: point.x,
            y: point.y,
            z: point.z,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn norm(self) -> f64 {
        self.x.hypot(self.y).hypot(self.z)
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    fn to_point(self) -> Point3 {
        Point3::new(self.x, self.y, self.z)
    }
}

fn finite_point(point: Point3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

fn validate_sha256(field: &str, digest: &str) -> Result<(), String> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{field} must be 64 lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

fn validate_triangle(
    label: &str,
    a: Point3,
    b: Point3,
    c: Point3,
) -> Result<(), String> {
    if !finite_point(a) || !finite_point(b) || !finite_point(c) {
        return Err(format!("{label} contains a non-finite coordinate"));
    }
    let edge_a = Vec3::from_point(b).sub(Vec3::from_point(a));
    let edge_b = Vec3::from_point(c).sub(Vec3::from_point(a));
    let twice_area = edge_a.cross(edge_b).norm();
    if !twice_area.is_finite() || twice_area <= DEGENERATE_TWICE_AREA_EPSILON {
        return Err(format!("{label} is degenerate"));
    }
    Ok(())
}

fn validate_view(view: &VisibilityView, frame_id: &str) -> Result<(), String> {
    if view.view_id.trim().is_empty() {
        return Err("view_id must not be empty".to_owned());
    }
    if view.frame_id != frame_id {
        return Err(format!(
            "view {} frame {} does not match scene frame {}",
            view.view_id, view.frame_id, frame_id
        ));
    }
    if !finite_point(view.origin) {
        return Err(format!("view {} origin is non-finite", view.view_id));
    }
    let forward = Vec3::from_point(view.forward);
    let right = Vec3::from_point(view.right);
    let up = Vec3::from_point(view.up);
    let vectors = [forward, right, up];
    if vectors.iter().any(|vector| {
        !vector.x.is_finite() || !vector.y.is_finite() || !vector.z.is_finite()
    }) {
        return Err(format!("view {} basis contains non-finite values", view.view_id));
    }
    if vectors
        .iter()
        .any(|vector| (vector.norm() - 1.0).abs() > BASIS_TOLERANCE)
        || forward.dot(right).abs() > BASIS_TOLERANCE
        || forward.dot(up).abs() > BASIS_TOLERANCE
        || right.dot(up).abs() > BASIS_TOLERANCE
        || right.cross(up).dot(forward) < 1.0 - BASIS_TOLERANCE
    {
        return Err(format!(
            "view {} basis must be orthonormal and right-handed",
            view.view_id
        ));
    }
    if !view.horizontal_fov_rad.is_finite()
        || view.horizontal_fov_rad <= 0.0
        || view.horizontal_fov_rad > TAU
        || !view.vertical_fov_rad.is_finite()
        || view.vertical_fov_rad <= 0.0
        || view.vertical_fov_rad > PI
        || !view.max_range_m.is_finite()
        || view.max_range_m <= 0.0
    {
        return Err(format!("view {} has invalid FOV or range", view.view_id));
    }
    validate_sha256("view.pose_evidence_sha256", &view.pose_evidence_sha256)?;
    validate_sha256(
        "view.configuration_sha256",
        &view.configuration_sha256,
    )?;
    Ok(())
}

fn view_covers_probe(view: &VisibilityView, point: Point3) -> Result<Option<(Vec3, f64)>, String> {
    let origin = Vec3::from_point(view.origin);
    let delta = Vec3::from_point(point).sub(origin);
    let distance = delta.norm();
    if !distance.is_finite() {
        return Err("probe-to-view distance became non-finite".to_owned());
    }
    if distance <= INTERSECTION_EPSILON_M {
        return Err(format!(
            "view {} origin coincides with a visibility probe",
            view.view_id
        ));
    }
    if distance > view.max_range_m {
        return Ok(None);
    }

    let forward = Vec3::from_point(view.forward);
    let right = Vec3::from_point(view.right);
    let up = Vec3::from_point(view.up);
    let forward_component = delta.dot(forward);
    if forward_component <= 0.0 {
        return Ok(None);
    }
    let right_component = delta.dot(right);
    let up_component = delta.dot(up);
    let horizontal_angle = right_component.atan2(forward_component).abs();
    let vertical_denominator = forward_component.hypot(right_component);
    let vertical_angle = up_component.atan2(vertical_denominator).abs();
    if horizontal_angle > view.horizontal_fov_rad * 0.5
        || vertical_angle > view.vertical_fov_rad * 0.5
    {
        return Ok(None);
    }
    Ok(Some((delta.scale(1.0 / distance), distance)))
}

/// Return the positive ray distance to the first interior/edge hit on a
/// triangle, if it lies strictly between the sensor and the target probe.
fn ray_triangle_distance(
    origin: Vec3,
    direction: Vec3,
    max_distance: f64,
    triangle: &OccluderTriangle,
) -> Option<f64> {
    let a = Vec3::from_point(triangle.a);
    let edge_a = Vec3::from_point(triangle.b).sub(a);
    let edge_b = Vec3::from_point(triangle.c).sub(a);
    let p_vector = direction.cross(edge_b);
    let determinant = edge_a.dot(p_vector);
    if determinant.abs() <= VECTOR_EPSILON {
        return None;
    }
    let inverse_determinant = 1.0 / determinant;
    let origin_offset = origin.sub(a);
    let u = origin_offset.dot(p_vector) * inverse_determinant;
    if !(-VECTOR_EPSILON..=1.0 + VECTOR_EPSILON).contains(&u) {
        return None;
    }
    let q_vector = origin_offset.cross(edge_a);
    let v = direction.dot(q_vector) * inverse_determinant;
    if v < -VECTOR_EPSILON || u + v > 1.0 + VECTOR_EPSILON {
        return None;
    }
    let distance = edge_b.dot(q_vector) * inverse_determinant;
    if !distance.is_finite()
        || distance <= INTERSECTION_EPSILON_M
        || distance >= max_distance - INTERSECTION_EPSILON_M
    {
        return None;
    }
    Some(distance)
}

fn observe_from_view(
    view: &VisibilityView,
    probe: &VisibilityProbe,
    occluders: &[OccluderTriangle],
    occluder_coverage: OccluderCoverage,
    profile: VisibilityProfile,
    remaining_ray_tests: usize,
) -> Result<(ViewObservation, usize), String> {
    let Some((direction, target_distance)) = view_covers_probe(view, probe.point)? else {
        return Ok((
            ViewObservation {
                view_id: view.view_id.clone(),
                status: ViewVisibilityStatus::OutsideSensorCoverage,
                blocker_kind: None,
                blocker_triangle_ids: Vec::new(),
                blocker_distance_m: None,
            },
            0,
        ));
    };

    let normal = Vec3::from_point(probe.normal);
    let normal_length = normal.norm();
    if !normal_length.is_finite() || normal_length <= VECTOR_EPSILON {
        return Err(format!(
            "probe {} has a zero or invalid normal",
            probe.probe_id
        ));
    }
    let incidence_cosine = normal
        .scale(1.0 / normal_length)
        .dot(direction.scale(-1.0));
    if !incidence_cosine.is_finite() {
        return Err(format!(
            "probe {} incidence became non-finite",
            probe.probe_id
        ));
    }
    if incidence_cosine <= profile.min_incidence_cosine {
        return Ok((
            ViewObservation {
                view_id: view.view_id.clone(),
                status: ViewVisibilityStatus::Backface,
                blocker_kind: None,
                blocker_triangle_ids: Vec::new(),
                blocker_distance_m: None,
            },
            0,
        ));
    }

    let origin = Vec3::from_point(view.origin);
    let mut ray_tests = 0usize;
    let mut nearest_distance: Option<f64> = None;
    let mut nearest: Vec<(&OccluderTriangle, f64)> = Vec::new();
    for occluder in occluders {
        if occluder.surface_id == probe.surface_id
            && occluder.triangle_id == probe.triangle_id
        {
            continue;
        }
        if ray_tests >= remaining_ray_tests {
            return Err("visibility ray-test budget exceeded during ray traversal".to_owned());
        }
        ray_tests += 1;
        if let Some(distance) =
            ray_triangle_distance(origin, direction, target_distance, occluder)
        {
            match nearest_distance {
                None => {
                    nearest_distance = Some(distance);
                    nearest.push((occluder, distance));
                }
                Some(best) if distance < best - INTERSECTION_EPSILON_M => {
                    nearest_distance = Some(distance);
                    nearest.clear();
                    nearest.push((occluder, distance));
                }
                Some(best) if (distance - best).abs() <= INTERSECTION_EPSILON_M => {
                    nearest.push((occluder, distance));
                }
                Some(_) => {}
            }
        }
    }

    if occluder_coverage == OccluderCoverage::Incomplete {
        let mut blocker_ids: Vec<String> = nearest
            .iter()
            .map(|(occluder, _)| format!("{}/{}", occluder.surface_id, occluder.triangle_id))
            .collect();
        blocker_ids.sort();
        let blocker_kinds: BTreeSet<OccluderKind> =
            nearest.iter().map(|(occluder, _)| occluder.kind).collect();
        return Ok((
            ViewObservation {
                view_id: view.view_id.clone(),
                status: ViewVisibilityStatus::Unknown,
                blocker_kind: if blocker_kinds.len() == 1 {
                    blocker_kinds.iter().next().copied()
                } else {
                    None
                },
                blocker_triangle_ids: blocker_ids,
                blocker_distance_m: nearest_distance,
            },
            ray_tests,
        ));
    }

    if nearest.is_empty() {
        return Ok((
            ViewObservation {
                view_id: view.view_id.clone(),
                status: ViewVisibilityStatus::Observable,
                blocker_kind: None,
                blocker_triangle_ids: Vec::new(),
                blocker_distance_m: None,
            },
            ray_tests,
        ));
    }

    nearest.sort_by(|left, right| {
        left.0
            .surface_id
            .cmp(&right.0.surface_id)
            .then_with(|| left.0.triangle_id.cmp(&right.0.triangle_id))
    });
    let mut kinds = BTreeSet::new();
    let mut blocker_triangle_ids = Vec::with_capacity(nearest.len());
    for (occluder, _) in &nearest {
        kinds.insert(occluder.kind);
        blocker_triangle_ids.push(format!(
            "{}/{}",
            occluder.surface_id, occluder.triangle_id
        ));
    }
    let status = if kinds.len() > 1 {
        ViewVisibilityStatus::Contradictory
    } else {
        match kinds.iter().next().copied() {
            Some(OccluderKind::SameBuilding) => ViewVisibilityStatus::SelfOccluded,
            Some(OccluderKind::External) => ViewVisibilityStatus::ExternallyOccluded,
            Some(OccluderKind::Unknown) => ViewVisibilityStatus::Unknown,
            None => return Err("ray query produced a blocker without a class".to_owned()),
        }
    };
    Ok((
        ViewObservation {
            view_id: view.view_id.clone(),
            status,
            blocker_kind: if status == ViewVisibilityStatus::Contradictory {
                None
            } else {
                kinds.iter().next().copied()
            },
            blocker_triangle_ids,
            blocker_distance_m: nearest_distance,
        },
        ray_tests,
    ))
}

fn aggregate_probe_visibility(views: &[ViewObservation]) -> SurfaceVisibility {
    if views
        .iter()
        .any(|view| view.status == ViewVisibilityStatus::Observable)
    {
        return SurfaceVisibility::Observable;
    }
    if views
        .iter()
        .any(|view| view.status == ViewVisibilityStatus::Contradictory)
    {
        return SurfaceVisibility::Contradictory;
    }
    if views
        .iter()
        .any(|view| view.status == ViewVisibilityStatus::Unknown)
    {
        return SurfaceVisibility::Unknown;
    }
    let has_self = views
        .iter()
        .any(|view| view.status == ViewVisibilityStatus::SelfOccluded);
    let has_external = views
        .iter()
        .any(|view| view.status == ViewVisibilityStatus::ExternallyOccluded);
    match (has_self, has_external) {
        (true, false) => SurfaceVisibility::SelfOccluded,
        (false, true) => SurfaceVisibility::ExternallyOccluded,
        (true, true) => SurfaceVisibility::MixedOcclusion,
        (false, false)
            if views
                .iter()
                .any(|view| view.status == ViewVisibilityStatus::Backface) =>
        {
            SurfaceVisibility::Backface
        }
        (false, false)
            if views
                .iter()
                .all(|view| view.status == ViewVisibilityStatus::OutsideSensorCoverage) =>
        {
            SurfaceVisibility::OutsideSensorCoverage
        }
        (false, false) => SurfaceVisibility::Unknown,
    }
}

fn visibility_label(value: SurfaceVisibility) -> &'static str {
    match value {
        SurfaceVisibility::Observable => "observable",
        SurfaceVisibility::SelfOccluded => "self_occluded",
        SurfaceVisibility::ExternallyOccluded => "externally_occluded",
        SurfaceVisibility::Backface => "backface",
        SurfaceVisibility::MixedOcclusion => "mixed_occlusion",
        SurfaceVisibility::OutsideSensorCoverage => "outside_sensor_coverage",
        SurfaceVisibility::Unknown => "unknown",
        SurfaceVisibility::Contradictory => "contradictory",
    }
}

fn update_string(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn update_f64(hasher: &mut Sha256, value: f64) {
    hasher.update(value.to_bits().to_le_bytes());
}

fn update_point(hasher: &mut Sha256, point: Point3) {
    update_f64(hasher, point.x);
    update_f64(hasher, point.y);
    update_f64(hasher, point.z);
}

fn receipt_digest(
    evaluation: &VisibilityEvaluation<'_>,
    probes: &[VisibilityProbeResult],
    views: &[VisibilityView],
    occluders: &[OccluderTriangle],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(evaluation.profile.max_ray_tests.to_le_bytes());
    update_f64(&mut hasher, evaluation.profile.min_incidence_cosine);
    update_string(&mut hasher, "scan_bim_evaluator_visibility_ray_v1");
    update_string(&mut hasher, evaluation.scene_id);
    update_string(&mut hasher, evaluation.frame_id);
    update_string(&mut hasher, evaluation.reference_model_sha256);
    update_string(&mut hasher, evaluation.probe_set_sha256);
    update_string(&mut hasher, evaluation.occluder_manifest_sha256);
    update_string(
        &mut hasher,
        match evaluation.occluder_coverage {
            OccluderCoverage::Complete => "complete",
            OccluderCoverage::Incomplete => "incomplete",
        },
    );

    for view in views {
        update_string(&mut hasher, &view.view_id);
        update_string(&mut hasher, &view.frame_id);
        update_point(&mut hasher, view.origin);
        update_point(&mut hasher, view.forward);
        update_point(&mut hasher, view.right);
        update_point(&mut hasher, view.up);
        update_f64(&mut hasher, view.horizontal_fov_rad);
        update_f64(&mut hasher, view.vertical_fov_rad);
        update_f64(&mut hasher, view.max_range_m);
        update_string(&mut hasher, &view.pose_evidence_sha256);
        update_string(&mut hasher, &view.configuration_sha256);
    }
    for occluder in occluders {
        update_string(&mut hasher, &occluder.surface_id);
        update_string(&mut hasher, &occluder.triangle_id);
        update_point(&mut hasher, occluder.a);
        update_point(&mut hasher, occluder.b);
        update_point(&mut hasher, occluder.c);
        update_string(
            &mut hasher,
            match occluder.kind {
                OccluderKind::SameBuilding => "same_building",
                OccluderKind::External => "external",
                OccluderKind::Unknown => "unknown",
            },
        );
    }
    for probe in probes {
        update_string(&mut hasher, &probe.probe_id);
        update_string(&mut hasher, &probe.surface_id);
        update_string(&mut hasher, &probe.triangle_id);
        update_point(&mut hasher, probe.point);
        update_point(&mut hasher, probe.normal);
        update_f64(&mut hasher, probe.represented_area_m2);
        update_string(&mut hasher, visibility_label(probe.visibility));
        for view in &probe.views {
            update_string(&mut hasher, &view.view_id);
            update_string(
                &mut hasher,
                match view.status {
                    ViewVisibilityStatus::Observable => "observable",
                    ViewVisibilityStatus::Backface => "backface",
                    ViewVisibilityStatus::OutsideSensorCoverage => "outside_sensor_coverage",
                    ViewVisibilityStatus::SelfOccluded => "self_occluded",
                    ViewVisibilityStatus::ExternallyOccluded => "externally_occluded",
                    ViewVisibilityStatus::Unknown => "unknown",
                    ViewVisibilityStatus::Contradictory => "contradictory",
                },
            );
            if let Some(distance) = view.blocker_distance_m {
                update_f64(&mut hasher, distance);
            }
            update_string(
                &mut hasher,
                match view.blocker_kind {
                    Some(OccluderKind::SameBuilding) => "same_building",
                    Some(OccluderKind::External) => "external",
                    Some(OccluderKind::Unknown) => "unknown",
                    None => "none_or_conflicting",
                },
            );
            for blocker in &view.blocker_triangle_ids {
                update_string(&mut hasher, blocker);
            }
        }
    }
    format!("{:x}", hasher.finalize())
}

/// Convert the full deterministic reference-surface sample set into probes.
/// This is an evaluator-only operation; candidate-visible sampling must never
/// receive the reference model or the resulting visibility labels.
pub fn probes_from_reference_sampling(
    receipt: &SurfaceSamplingReceipt,
) -> Result<Vec<VisibilityProbe>, String> {
    verify_surface_sampling_receipt(receipt)?;
    let probes: Vec<VisibilityProbe> = receipt
        .samples
        .iter()
        .map(|sample: &SurfaceSample| VisibilityProbe {
            probe_id: format!("{}:{}", sample.surface_id, sample.sample_ordinal),
            surface_id: sample.surface_id.clone(),
            triangle_id: sample.triangle_id.clone(),
            point: sample.point,
            barycentric: sample.barycentric,
            normal: sample.normal,
            represented_area_m2: sample.represented_area_m2,
        })
        .collect();
    if probes.is_empty() {
        return Err("reference sampling receipt contains no visibility probes".to_owned());
    }
    Ok(probes)
}

fn validate_reference_probe_set(
    probes: &[&VisibilityProbe],
    occluders: &[OccluderTriangle],
) -> Result<(), String> {
    let mut target_triangles: BTreeMap<(&str, &str), &OccluderTriangle> = BTreeMap::new();
    let mut reference_area_by_surface: BTreeMap<&str, f64> = BTreeMap::new();

    for triangle in occluders {
        if triangle.kind != OccluderKind::SameBuilding {
            continue;
        }
        let key = (triangle.surface_id.as_str(), triangle.triangle_id.as_str());
        if target_triangles.insert(key, triangle).is_some() {
            return Err("reference geometry contains duplicate target triangle identities".to_owned());
        }
        let area = triangle_area_m2(triangle.a, triangle.b, triangle.c)?;
        let total = reference_area_by_surface
            .entry(triangle.surface_id.as_str())
            .or_insert(0.0);
        *total += area;
        if !total.is_finite() {
            return Err("reference surface area overflowed".to_owned());
        }
    }

    let mut represented_area_by_surface: BTreeMap<&str, f64> = BTreeMap::new();
    for probe in probes {
        let triangle = target_triangles
            .get(&(probe.surface_id.as_str(), probe.triangle_id.as_str()))
            .ok_or("visibility probe does not reference a triangle in the declared BIM reference geometry")?;
        if probe
            .barycentric
            .iter()
            .any(|weight| !weight.is_finite() || *weight < -1.0e-12 || *weight > 1.0 + 1.0e-12)
            || (probe.barycentric.iter().sum::<f64>() - 1.0).abs() > 1.0e-10
        {
            return Err(format!(
                "probe {} has invalid source-triangle barycentric coordinates",
                probe.probe_id
            ));
        }

        let [wa, wb, wc] = probe.barycentric;
        let expected_point = Point3::new(
            wa * triangle.a.x + wb * triangle.b.x + wc * triangle.c.x,
            wa * triangle.a.y + wb * triangle.b.y + wc * triangle.c.y,
            wa * triangle.a.z + wb * triangle.b.z + wc * triangle.c.z,
        );
        let point_error = Vec3::from_point(probe.point)
            .sub(Vec3::from_point(expected_point))
            .norm();
        let coordinate_scale = [
            triangle.a.x.abs(),
            triangle.a.y.abs(),
            triangle.a.z.abs(),
            triangle.b.x.abs(),
            triangle.b.y.abs(),
            triangle.b.z.abs(),
            triangle.c.x.abs(),
            triangle.c.y.abs(),
            triangle.c.z.abs(),
        ]
        .into_iter()
        .fold(1.0f64, f64::max);
        if !point_error.is_finite() || point_error > coordinate_scale * 1.0e-9 {
            return Err(format!(
                "probe {} point does not match its source-triangle barycentric coordinates",
                probe.probe_id
            ));
        }

        let edge_a = Vec3::from_point(triangle.b).sub(Vec3::from_point(triangle.a));
        let edge_b = Vec3::from_point(triangle.c).sub(Vec3::from_point(triangle.a));
        let cross = edge_a.cross(edge_b);
        let normal_length = cross.norm();
        if !normal_length.is_finite() || normal_length <= DEGENERATE_TWICE_AREA_EPSILON {
            return Err("reference target triangle has a degenerate normal".to_owned());
        }
        let expected_normal = cross.scale(1.0 / normal_length);
        let actual_normal = Vec3::from_point(probe.normal);
        if !finite_point(probe.normal)
            || (actual_normal.norm() - 1.0).abs() > 1.0e-8
            || actual_normal.sub(expected_normal).norm() > 1.0e-8
        {
            return Err(format!(
                "probe {} normal does not match source-triangle winding",
                probe.probe_id
            ));
        }

        let total = represented_area_by_surface
            .entry(probe.surface_id.as_str())
            .or_insert(0.0);
        *total += probe.represented_area_m2;
        if !total.is_finite() {
            return Err("represented reference surface area overflowed".to_owned());
        }
    }

    if represented_area_by_surface.len() != reference_area_by_surface.len() {
        return Err(
            "reference sampling surface inventory does not match reference geometry".to_owned(),
        );
    }
    for (surface_id, reference_area) in reference_area_by_surface {
        let represented_area = represented_area_by_surface
            .get(surface_id)
            .ok_or("reference geometry surface has no area-weighted visibility probes")?;
        if (represented_area - reference_area).abs()
            > reference_area.abs().max(1.0) * 1.0e-9
        {
            return Err(format!(
                "reference sampling weights do not conserve geometry area for surface {surface_id}"
            ));
        }
    }
    Ok(())
}

/// Derive evaluator-only visibility labels for area-weighted probes.
///
/// Coordinates are metres in one declared frame. A probe is observable when
/// at least one in-range, in-FOV qualified view has an unobstructed segment to
/// it. Unknown/incomplete blockers and conflicting nearest blockers remain
/// explicit. Reference geometry and this receipt must remain evaluator-only.
pub fn derive_surface_visibility(
    evaluation: VisibilityEvaluation<'_>,
) -> Result<VisibilityReceipt, String> {
    if evaluation.scene_id.trim().is_empty() || evaluation.frame_id.trim().is_empty() {
        return Err("scene_id and frame_id must not be empty".to_owned());
    }
    validate_sha256("reference_model_sha256", evaluation.reference_model_sha256)?;
    validate_sha256("probe_set_sha256", evaluation.probe_set_sha256)?;
    validate_sha256(
        "occluder_manifest_sha256",
        evaluation.occluder_manifest_sha256,
    )?;
    if evaluation.probes.is_empty() || evaluation.views.is_empty() {
        return Err("visibility requires at least one probe and one view".to_owned());
    }
    if evaluation.profile.max_ray_tests == 0 {
        return Err("max_ray_tests must be positive".to_owned());
    }
    if !evaluation.profile.min_incidence_cosine.is_finite()
        || !(0.0..=1.0).contains(&evaluation.profile.min_incidence_cosine)
    {
        return Err("min_incidence_cosine must be finite and within [0, 1]".to_owned());
    }

    let mut ordered_views: Vec<&VisibilityView> = evaluation.views.iter().collect();
    ordered_views.sort_by(|left, right| left.view_id.cmp(&right.view_id));
    let mut view_ids = BTreeSet::new();
    for view in &ordered_views {
        validate_view(view, evaluation.frame_id)?;
        if !view_ids.insert(view.view_id.as_str()) {
            return Err("duplicate visibility view_id".to_owned());
        }
    }

    let mut ordered_occluders: Vec<&OccluderTriangle> = evaluation.occluders.iter().collect();
    ordered_occluders.sort_by(|left, right| {
        left.surface_id
            .cmp(&right.surface_id)
            .then_with(|| left.triangle_id.cmp(&right.triangle_id))
    });
    let mut occluder_ids = BTreeSet::new();
    for occluder in &ordered_occluders {
        if occluder.surface_id.trim().is_empty() || occluder.triangle_id.trim().is_empty() {
            return Err("occluder surface and triangle IDs must not be empty".to_owned());
        }
        if !occluder_ids.insert((occluder.surface_id.as_str(), occluder.triangle_id.as_str())) {
            return Err("duplicate occluder triangle identity".to_owned());
        }
        validate_triangle("occluder triangle", occluder.a, occluder.b, occluder.c)?;
    }
    let ordered_occluders: Vec<OccluderTriangle> =
        ordered_occluders.into_iter().map(|value| (*value).clone()).collect();

    let mut ordered_probes: Vec<&VisibilityProbe> = evaluation.probes.iter().collect();
    ordered_probes.sort_by(|left, right| left.probe_id.cmp(&right.probe_id));
    let mut probe_ids = BTreeSet::new();
    for probe in &ordered_probes {
        if probe.probe_id.trim().is_empty()
            || probe.surface_id.trim().is_empty()
            || probe.triangle_id.trim().is_empty()
        {
            return Err("probe, surface and triangle IDs must not be empty".to_owned());
        }
        if !probe_ids.insert(probe.probe_id.as_str()) {
            return Err("duplicate visibility probe_id".to_owned());
        }
        if !finite_point(probe.point)
            || !finite_point(probe.normal)
            || !probe.represented_area_m2.is_finite()
            || probe.represented_area_m2 <= 0.0
            || Vec3::from_point(probe.normal).norm() <= VECTOR_EPSILON
        {
            return Err(
                "probe coordinate/normal and represented area must be finite; area and normal must be positive"
                    .to_owned(),
            );
        }
    }

    let owned_views: Vec<VisibilityView> =
        ordered_views.iter().map(|value| (**value).clone()).collect();
    validate_reference_probe_set(&ordered_probes, &ordered_occluders)?;

    let mut results = Vec::with_capacity(evaluation.probes.len());
    let mut ray_test_count = 0usize;
    let mut surface_totals: BTreeMap<String, [f64; 8]> = BTreeMap::new();

    for probe in ordered_probes {
        let mut per_view = Vec::with_capacity(owned_views.len());
        for view in &owned_views {
            let (observation, tests) = observe_from_view(
                view,
                probe,
                &ordered_occluders,
                evaluation.occluder_coverage,
                evaluation.profile,
                evaluation.profile.max_ray_tests.saturating_sub(ray_test_count),
            )?;
            ray_test_count = ray_test_count
                .checked_add(tests)
                .ok_or("visibility ray-test count overflowed")?;
            if ray_test_count > evaluation.profile.max_ray_tests {
                return Err("visibility ray-test budget exceeded during evaluation".to_owned());
            }
            per_view.push(observation);
        }

        let visibility = aggregate_probe_visibility(&per_view);
        let totals = surface_totals
            .entry(probe.surface_id.clone())
            .or_insert([0.0; 8]);
        let index = match visibility {
            SurfaceVisibility::Observable => 0,
            SurfaceVisibility::Backface => 1,
            SurfaceVisibility::SelfOccluded => 2,
            SurfaceVisibility::ExternallyOccluded => 3,
            SurfaceVisibility::OutsideSensorCoverage => 4,
            SurfaceVisibility::Unknown => 5,
            SurfaceVisibility::Contradictory => 6,
            SurfaceVisibility::MixedOcclusion => 7,
        };
        totals[index] += probe.represented_area_m2;
        if !totals[index].is_finite() {
            return Err("visibility area accumulation overflowed".to_owned());
        }
        results.push(VisibilityProbeResult {
            probe_id: probe.probe_id.clone(),
            surface_id: probe.surface_id.clone(),
            triangle_id: probe.triangle_id.clone(),
            point: probe.point,
            normal: probe.normal,
            represented_area_m2: probe.represented_area_m2,
            visibility,
            views: per_view,
        });
    }

    let mut surfaces = Vec::with_capacity(surface_totals.len());
    for (surface_id, areas) in surface_totals {
        surfaces.push(VisibilitySurfaceArea {
            surface_id,
            observable_area_m2: areas[0],
            backface_area_m2: areas[1],
            self_occluded_area_m2: areas[2],
            externally_occluded_area_m2: areas[3],
            outside_sensor_coverage_area_m2: areas[4],
            unknown_area_m2: areas[5],
            contradictory_area_m2: areas[6],
            mixed_occlusion_area_m2: areas[7],
        });
    }

    let totals = surfaces.iter().fold([0.0f64; 8], |mut acc, surface| {
        acc[0] += surface.observable_area_m2;
        acc[1] += surface.backface_area_m2;
        acc[2] += surface.self_occluded_area_m2;
        acc[3] += surface.externally_occluded_area_m2;
        acc[4] += surface.outside_sensor_coverage_area_m2;
        acc[5] += surface.unknown_area_m2;
        acc[6] += surface.contradictory_area_m2;
        acc[7] += surface.mixed_occlusion_area_m2;
        acc
    });
    if totals.iter().any(|value| !value.is_finite()) {
        return Err("visibility total area overflowed".to_owned());
    }

    let visibility_sha256 = receipt_digest(
        &evaluation,
        &results,
        &owned_views,
        &ordered_occluders,
    );

    Ok(VisibilityReceipt {
        algorithm_id: "evaluator_bim_visibility_ray_v1".to_owned(),
        scene_id: evaluation.scene_id.to_owned(),
        frame_id: evaluation.frame_id.to_owned(),
        reference_model_sha256: evaluation.reference_model_sha256.to_owned(),
        probe_set_sha256: evaluation.probe_set_sha256.to_owned(),
        occluder_manifest_sha256: evaluation.occluder_manifest_sha256.to_owned(),
        occluder_coverage: evaluation.occluder_coverage,
        min_incidence_cosine: evaluation.profile.min_incidence_cosine,
        probe_count: results.len(),
        view_count: owned_views.len(),
        ray_test_count,
        observable_area_m2: totals[0],
        backface_area_m2: totals[1],
        self_occluded_area_m2: totals[2],
        externally_occluded_area_m2: totals[3],
        outside_sensor_coverage_area_m2: totals[4],
        unknown_area_m2: totals[5],
        contradictory_area_m2: totals[6],
        mixed_occlusion_area_m2: totals[7],
        surfaces,
        probes: results,
        visibility_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        OccluderCoverage, OccluderKind, OccluderTriangle, VisibilityEvaluation,
        VisibilityProbe, VisibilityProfile, VisibilityView, ViewVisibilityStatus,
        derive_surface_visibility,
    };
    use scan_bim_replay_contract::metrics::Point3;
    use scan_bim_replay_contract::sampling::{SurfaceSample, SurfaceSamplingReceipt, SurfaceVisibility};

    fn view(id: &str, origin: [f64; 3]) -> VisibilityView {
        VisibilityView {
            view_id: id.to_owned(),
            frame_id: "BUILDING-LOCAL".to_owned(),
            origin: Point3::new(origin[0], origin[1], origin[2]),
            forward: Point3::new(1.0, 0.0, 0.0),
            right: Point3::new(0.0, 1.0, 0.0),
            up: Point3::new(0.0, 0.0, 1.0),
            horizontal_fov_rad: 1.2,
            vertical_fov_rad: 1.2,
            max_range_m: 20.0,
            pose_evidence_sha256: "a".repeat(64),
            configuration_sha256: "b".repeat(64),
        }
    }

    fn probe(point: [f64; 3]) -> VisibilityProbe {
        VisibilityProbe {
            probe_id: "P-1".to_owned(),
            surface_id: "TARGET".to_owned(),
            triangle_id: "TARGET-TRI-1".to_owned(),
            point: Point3::new(point[0], point[1], point[2]),
            normal: Point3::new(-1.0, 0.0, 0.0),
            represented_area_m2: 2.0,
        }
    }

    fn blocker(kind: OccluderKind) -> OccluderTriangle {
        OccluderTriangle {
            surface_id: "BLOCKER".to_owned(),
            triangle_id: "BLOCKER-TRI-1".to_owned(),
            a: Point3::new(2.0, -1.0, -1.0),
            b: Point3::new(2.0, 1.0, -1.0),
            c: Point3::new(2.0, 0.0, 1.0),
            kind,
        }
    }

    fn evaluate(
        probes: &[VisibilityProbe],
        views: &[VisibilityView],
        occluders: &[OccluderTriangle],
    ) -> Result<super::VisibilityReceipt, String> {
        evaluate_with_coverage(
            probes,
            views,
            occluders,
            OccluderCoverage::Complete,
        )
    }

    fn evaluate_with_coverage(
        probes: &[VisibilityProbe],
        views: &[VisibilityView],
        occluders: &[OccluderTriangle],
        occluder_coverage: OccluderCoverage,
    ) -> Result<super::VisibilityReceipt, String> {
        derive_surface_visibility(VisibilityEvaluation {
            scene_id: "SCENE-1",
            frame_id: "BUILDING-LOCAL",
            reference_model_sha256: &"1".repeat(64),
            probe_set_sha256: &"3".repeat(64),
            occluder_manifest_sha256: &"2".repeat(64),
            occluder_coverage,
            probes,
            views,
            occluders,
            profile: VisibilityProfile {
                max_ray_tests: 10_000,
                min_incidence_cosine: 0.0,
            },
        })
    }

    #[test]
    fn unobstructed_probe_is_observable() {
        let receipt = evaluate(&[probe([5.0, 0.0, 0.0])], &[view("VIEW-1", [0.0, 0.0, 0.0])], &[])
            .expect("unobstructed view");
        assert_eq!(receipt.probes[0].visibility, SurfaceVisibility::Observable);
        assert_eq!(receipt.observable_area_m2, 2.0);
    }

    #[test]
    fn same_building_occluder_is_self_occluded() {
        let receipt = evaluate(
            &[probe([5.0, 0.0, 0.0])],
            &[view("VIEW-1", [0.0, 0.0, 0.0])],
            &[blocker(OccluderKind::SameBuilding)],
        )
        .expect("self occlusion");
        assert_eq!(receipt.probes[0].visibility, SurfaceVisibility::SelfOccluded);
        assert_eq!(receipt.self_occluded_area_m2, 2.0);
    }

    #[test]
    fn external_occluder_is_not_conflated_with_self_occlusion() {
        let receipt = evaluate(
            &[probe([5.0, 0.0, 0.0])],
            &[view("VIEW-1", [0.0, 0.0, 0.0])],
            &[blocker(OccluderKind::External)],
        )
        .expect("external occlusion");
        assert_eq!(
            receipt.probes[0].visibility,
            SurfaceVisibility::ExternallyOccluded
        );
        assert_eq!(receipt.externally_occluded_area_m2, 2.0);
    }

    #[test]
    fn unknown_occluder_never_becomes_observable() {
        let receipt = evaluate(
            &[probe([5.0, 0.0, 0.0])],
            &[view("VIEW-1", [0.0, 0.0, 0.0])],
            &[blocker(OccluderKind::Unknown)],
        )
        .expect("unknown occlusion");
        assert_eq!(receipt.probes[0].visibility, SurfaceVisibility::Unknown);
        assert_eq!(receipt.unknown_area_m2, 2.0);
    }

    #[test]
    fn target_outside_field_of_view_is_outside_coverage() {
        let receipt = evaluate(
            &[probe([5.0, 10.0, 0.0])],
            &[view("VIEW-1", [0.0, 0.0, 0.0])],
            &[],
        )
        .expect("outside FOV");
        assert_eq!(
            receipt.probes[0].visibility,
            SurfaceVisibility::OutsideSensorCoverage
        );
    }

    #[test]
    fn any_unobstructed_qualified_view_makes_probe_observable() {
        let second_view = VisibilityView {
            view_id: "VIEW-2".to_owned(),
            frame_id: "BUILDING-LOCAL".to_owned(),
            origin: Point3::new(0.0, 10.0, 0.0),
            forward: Point3::new(5.0 / 125.0_f64.sqrt(), -10.0 / 125.0_f64.sqrt(), 0.0),
            right: Point3::new(10.0 / 125.0_f64.sqrt(), 5.0 / 125.0_f64.sqrt(), 0.0),
            up: Point3::new(0.0, 0.0, 1.0),
            horizontal_fov_rad: 1.2,
            vertical_fov_rad: 1.2,
            max_range_m: 20.0,
            pose_evidence_sha256: "c".repeat(64),
            configuration_sha256: "d".repeat(64),
        };
        let receipt = evaluate(
            &[probe([5.0, 0.0, 0.0])],
            &[view("VIEW-1", [0.0, 0.0, 0.0]), second_view],
            &[blocker(OccluderKind::SameBuilding)],
        )
        .expect("multi-view union");
        assert_eq!(receipt.probes[0].visibility, SurfaceVisibility::Observable);
        assert_eq!(receipt.observable_area_m2, 2.0);
    }

    #[test]
    fn conflicting_nearest_blockers_are_contradictory() {
        let mut external = blocker(OccluderKind::External);
        external.surface_id = "BLOCKER-EXTERNAL".to_owned();
        let receipt = evaluate(
            &[probe([5.0, 0.0, 0.0])],
            &[view("VIEW-1", [0.0, 0.0, 0.0])],
            &[blocker(OccluderKind::SameBuilding), external],
        )
        .expect("contradictory blockers");
        assert_eq!(
            receipt.probes[0].visibility,
            SurfaceVisibility::Contradictory
        );
        assert_eq!(receipt.contradictory_area_m2, 2.0);
    }

    #[test]
    fn frame_mismatch_fails_closed() {
        let mut stale = view("VIEW-1", [0.0, 0.0, 0.0]);
        stale.frame_id = "STALE-FRAME".to_owned();
        let error = evaluate(&[probe([5.0, 0.0, 0.0])], &[stale], &[])
            .expect_err("stale frame must fail");
        assert!(error.contains("does not match scene frame"));
    }

    #[test]
    fn insufficient_ray_budget_fails_closed() {
        let error = derive_surface_visibility(VisibilityEvaluation {
            scene_id: "SCENE-1",
            frame_id: "BUILDING-LOCAL",
            reference_model_sha256: &"1".repeat(64),
            probe_set_sha256: &"3".repeat(64),
            occluder_manifest_sha256: &"2".repeat(64),
            occluder_coverage: OccluderCoverage::Complete,
            probes: &[probe([5.0, 0.0, 0.0])],
            views: &[view("VIEW-1", [0.0, 0.0, 0.0])],
            occluders: &[blocker(OccluderKind::SameBuilding)],
            profile: VisibilityProfile {
                max_ray_tests: 0,
                min_incidence_cosine: 0.0,
            },
        })
        .expect_err("zero budget must fail");
        assert!(error.contains("max_ray_tests"));
    }

    #[test]
    fn input_permutation_preserves_visibility_receipt() {
        let mut second_probe = probe([5.0, 1.0, 0.0]);
        second_probe.probe_id = "P-2".to_owned();
        second_probe.triangle_id = "TARGET-TRI-2".to_owned();
        let mut second_blocker = blocker(OccluderKind::SameBuilding);
        second_blocker.surface_id = "BLOCKER-2".to_owned();
        second_blocker.triangle_id = "BLOCKER-TRI-2".to_owned();

        let left = evaluate(
            &[probe([5.0, 0.0, 0.0]), second_probe.clone()],
            &[
                view("VIEW-1", [0.0, 0.0, 0.0]),
                view("VIEW-2", [0.0, 0.0, 0.0]),
            ],
            &[
                blocker(OccluderKind::SameBuilding),
                second_blocker.clone(),
            ],
        )
        .expect("canonical ordering");
        let right = evaluate(
            &[second_probe, probe([5.0, 0.0, 0.0])],
            &[
                view("VIEW-2", [0.0, 0.0, 0.0]),
                view("VIEW-1", [0.0, 0.0, 0.0]),
            ],
            &[second_blocker, blocker(OccluderKind::SameBuilding)],
        )
        .expect("permuted ordering");
        assert_eq!(left.visibility_sha256, right.visibility_sha256);
    }

    #[test]
    fn receipt_binds_reference_and_occluder_manifests() {
        let left = evaluate(
            &[probe([5.0, 0.0, 0.0])],
            &[view("VIEW-1", [0.0, 0.0, 0.0])],
            &[],
        )
        .expect("initial receipt");
        let changed = derive_surface_visibility(VisibilityEvaluation {
            scene_id: "SCENE-1",
            frame_id: "BUILDING-LOCAL",
            reference_model_sha256: &"3".repeat(64),
            probe_set_sha256: &"3".repeat(64),
            occluder_manifest_sha256: &"2".repeat(64),
            occluder_coverage: OccluderCoverage::Complete,
            probes: &[probe([5.0, 0.0, 0.0])],
            views: &[view("VIEW-1", [0.0, 0.0, 0.0])],
            occluders: &[],
            profile: VisibilityProfile {
                max_ray_tests: 100,
                min_incidence_cosine: 0.0,
            },
        })
        .expect("changed reference");
        assert_ne!(left.visibility_sha256, changed.visibility_sha256);
    }
}
