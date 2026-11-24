use crate::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const FRUSTUM_CORPUS_SCHEMA: &str = "luminous.orbital-truss-rectilinear-frustum.corpus.v0.1";
pub const FRUSTUM_RECEIPT_SCHEMA: &str = "luminous.orbital-truss-rectilinear-frustum.receipt.v0.1";
pub const FRUSTUM_BENCHMARK_ID: &str = "ORBITAL-TRUSS-RECTILINEAR-FRUSTUM-001";
pub const FRUSTUM_CLAIM_CEILING: &str =
    "synthetic_3d_support_direction_cap_vs_rectilinear_pinhole_frustum_only_occlusion_not_assessed";
pub const FRUSTUM_CASE_COUNT: usize = 9;

const UNIT_EPS: f64 = 1.0e-6;
const GEOMETRY_EPS: f64 = 1.0e-12;
const DISTANCE_SCALE: f64 = 1.0e9;
const ANGLE_SCALE: f64 = 1.0e9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FrustumStatus {
    Within,
    Outside,
    Indeterminate,
    InvalidInput,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirectionalSupport {
    pub support_id: String,
    pub center_m: [f64; 3],
    pub enclosing_ball_radius_m: f64,
    pub enclosing_ball_asserted: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RectilinearCamera {
    pub camera_id: String,
    pub camera_model_id: String,
    pub position_m: [f64; 3],
    pub optical_axis_unit: [f64; 3],
    pub image_up_unit: [f64; 3],
    pub horizontal_fov_rad: f64,
    pub vertical_fov_rad: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RectilinearFrustumCase {
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub expected_status: FrustumStatus,
    pub support: DirectionalSupport,
    pub camera: RectilinearCamera,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RectilinearFrustumCorpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<RectilinearFrustumCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RectilinearFrustumCertificate {
    pub case_id: String,
    pub support_id: String,
    pub camera_id: String,
    pub status: FrustumStatus,
    pub reason_codes: Vec<String>,
    pub center_distance_nm: Option<u64>,
    pub support_direction_spread_nrad: Option<u64>,
    pub halfspace_inside_margin_nrad: [Option<i64>; 5],
    pub halfspace_outside_margin_nrad: [Option<i64>; 5],
    pub minimum_horizontal_inside_margin_nrad: Option<i64>,
    pub minimum_vertical_inside_margin_nrad: Option<i64>,
    pub forward_hemisphere_inside_margin_nrad: Option<i64>,
    pub support_bound_input_asserted: bool,
    pub precondition_proofs_verified_independently: bool,
    pub occlusion_status: String,
    pub within_rectilinear_frustum_certified: bool,
    pub may_count_toward_visibility: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RectilinearFrustumReceipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub structural_acceptance_authorized: bool,
    pub flight_authorized: bool,
    pub physical_validation: String,
    pub occlusion_assessed: bool,
    pub precondition_proofs_verified_independently: bool,
    pub case_count: usize,
    pub expected_outcome_matches: usize,
    pub expected_outcome_mismatches: usize,
    pub cases: Vec<RectilinearFrustumCertificate>,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(a: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn finite(a: [f64; 3]) -> bool {
    a.iter().all(|x| x.is_finite())
}
fn normalize(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(a);
    (n.is_finite() && n > GEOMETRY_EPS).then(|| scale(a, 1.0 / n))
}
fn fixed_u(value: f64, factor: f64) -> Option<u64> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let n = (value * factor + 0.5).floor();
    (n.is_finite() && n <= u64::MAX as f64).then_some(n as u64)
}
fn fixed_i(value: f64, factor: f64) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    let n = (value * factor).round();
    (n.is_finite() && n >= i64::MIN as f64 && n <= i64::MAX as f64).then_some(n as i64)
}

// Exact spherical-cap support function, evaluated in f64.
fn cap_max_dot(a: [f64; 3], direction: [f64; 3], spread: f64) -> Option<f64> {
    let n = norm(a);
    if !n.is_finite() || n <= GEOMETRY_EPS {
        return None;
    }
    let theta = (dot(a, direction) / n).clamp(-1.0, 1.0).acos();
    Some(if theta <= spread {
        n
    } else {
        n * (theta - spread).cos()
    })
}
fn cap_min_dot(a: [f64; 3], direction: [f64; 3], spread: f64) -> Option<f64> {
    cap_max_dot(scale(a, -1.0), direction, spread).map(|v| -v)
}

fn invalid(case: &RectilinearFrustumCase, reason: &str) -> RectilinearFrustumCertificate {
    RectilinearFrustumCertificate {
        case_id: case.case_id.clone(),
        support_id: case.support.support_id.clone(),
        camera_id: case.camera.camera_id.clone(),
        status: FrustumStatus::InvalidInput,
        reason_codes: vec![reason.to_owned()],
        center_distance_nm: None,
        support_direction_spread_nrad: None,
        halfspace_inside_margin_nrad: [None; 5],
        halfspace_outside_margin_nrad: [None; 5],
        minimum_horizontal_inside_margin_nrad: None,
        minimum_vertical_inside_margin_nrad: None,
        forward_hemisphere_inside_margin_nrad: None,
        support_bound_input_asserted: case.support.enclosing_ball_asserted,
        precondition_proofs_verified_independently: false,
        occlusion_status: "NOT_ASSESSED".to_owned(),
        within_rectilinear_frustum_certified: false,
        may_count_toward_visibility: false,
    }
}
fn indeterminate(case: &RectilinearFrustumCase, reason: &str) -> RectilinearFrustumCertificate {
    let mut c = invalid(case, reason);
    c.status = FrustumStatus::Indeterminate;
    c
}
fn validate(case: &RectilinearFrustumCase) -> Result<(), &'static str> {
    if case.case_id.trim().is_empty() || case.coordinate_frame_id.trim().is_empty() {
        return Err("CASE_ID_OR_COORDINATE_FRAME_INVALID");
    }
    let s = &case.support;
    if s.support_id.trim().is_empty()
        || !finite(s.center_m)
        || !s.enclosing_ball_radius_m.is_finite()
        || s.enclosing_ball_radius_m <= 0.0
    {
        return Err("SUPPORT_ID_CENTER_OR_ENCLOSURE_INVALID");
    }
    let c = &case.camera;
    if c.camera_id.trim().is_empty() || c.camera_model_id.trim().is_empty() || !finite(c.position_m)
    {
        return Err("CAMERA_ID_OR_POSITION_INVALID");
    }
    if !finite(c.optical_axis_unit)
        || !finite(c.image_up_unit)
        || (norm(c.optical_axis_unit) - 1.0).abs() > UNIT_EPS
        || (norm(c.image_up_unit) - 1.0).abs() > UNIT_EPS
    {
        return Err("CAMERA_BASIS_VECTORS_NOT_UNIT");
    }
    if dot(c.optical_axis_unit, c.image_up_unit).abs() > UNIT_EPS {
        return Err("CAMERA_IMAGE_UP_NOT_ORTHOGONAL_TO_OPTICAL_AXIS");
    }
    if !c.horizontal_fov_rad.is_finite()
        || !c.vertical_fov_rad.is_finite()
        || c.horizontal_fov_rad <= 0.0
        || c.vertical_fov_rad <= 0.0
        || c.horizontal_fov_rad >= std::f64::consts::PI
        || c.vertical_fov_rad >= std::f64::consts::PI
    {
        return Err("RECTILINEAR_CAMERA_FOV_INVALID");
    }
    Ok(())
}

pub fn evaluate_rectilinear_frustum(
    case: &RectilinearFrustumCase,
) -> RectilinearFrustumCertificate {
    if let Err(reason) = validate(case) {
        return invalid(case, reason);
    }
    if !case.support.enclosing_ball_asserted {
        return indeterminate(case, "SUPPORT_ENCLOSING_BALL_UNPROVEN");
    }
    let vector = sub(case.support.center_m, case.camera.position_m);
    let distance = norm(vector);
    if !distance.is_finite() || distance <= case.support.enclosing_ball_radius_m {
        return indeterminate(case, "CAMERA_NOT_OUTSIDE_SUPPORT_ENCLOSURE");
    }
    let direction = scale(vector, 1.0 / distance);
    let spread = (case.support.enclosing_ball_radius_m / distance)
        .clamp(0.0, 1.0)
        .asin();

    let forward = case.camera.optical_axis_unit;
    let image_up = case.camera.image_up_unit;
    // Right-handed basis: right = forward × image-up; up = right × forward.
    let right = match normalize(cross(forward, image_up)) {
        Some(v) => v,
        None => return invalid(case, "CAMERA_BASIS_DEGENERATE"),
    };
    let up = match normalize(cross(right, forward)) {
        Some(v) => v,
        None => return invalid(case, "CAMERA_BASIS_DEGENERATE"),
    };
    if dot(up, image_up) < 1.0 - UNIT_EPS {
        return invalid(case, "CAMERA_BASIS_HANDEDNESS_INCONSISTENT");
    }

    let th = (case.camera.horizontal_fov_rad / 2.0).tan();
    let tv = (case.camera.vertical_fov_rad / 2.0).tan();
    // Each plane is a·u <= 0 for an admitted ray direction u.
    let planes = [
        scale(forward, -1.0),
        sub(right, scale(forward, th)),
        sub(scale(right, -1.0), scale(forward, th)),
        sub(up, scale(forward, tv)),
        sub(scale(up, -1.0), scale(forward, tv)),
    ];
    let mut inside = [0.0_f64; 5];
    let mut outside = [0.0_f64; 5];
    let mut all_inside = true;
    let mut some_plane_wholly_outside = false;
    for (i, plane) in planes.iter().enumerate() {
        let theta = (dot(*plane, direction) / norm(*plane))
            .clamp(-1.0, 1.0)
            .acos();
        inside[i] = theta - spread - std::f64::consts::FRAC_PI_2;
        outside[i] = std::f64::consts::FRAC_PI_2 - theta - spread;
        all_inside &= cap_max_dot(*plane, direction, spread).expect("valid plane") <= EPS;
        some_plane_wholly_outside |=
            cap_min_dot(*plane, direction, spread).expect("valid plane") > EPS;
    }
    let status = if all_inside {
        FrustumStatus::Within
    } else if some_plane_wholly_outside {
        FrustumStatus::Outside
    } else {
        FrustumStatus::Indeterminate
    };
    let mut reasons = match status {
        FrustumStatus::Within => vec![
            "WHOLE_SUPPORT_RECTILINEAR_FRUSTUM_BOUNDS_PASS".to_owned(),
            "OCCLUSION_NOT_ASSESSED".to_owned(),
        ],
        FrustumStatus::Outside => vec!["WHOLE_SUPPORT_OUTSIDE_RECTILINEAR_FRUSTUM".to_owned()],
        FrustumStatus::Indeterminate => {
            vec!["RECTILINEAR_FRUSTUM_BOUNDARY_OR_SUPPORT_STRADDLE".to_owned()]
        }
        FrustumStatus::InvalidInput => vec!["UNREACHABLE_INVALID_STATUS".to_owned()],
    };
    reasons.sort();
    reasons.dedup();
    RectilinearFrustumCertificate {
        case_id: case.case_id.clone(),
        support_id: case.support.support_id.clone(),
        camera_id: case.camera.camera_id.clone(),
        status,
        reason_codes: reasons,
        center_distance_nm: fixed_u(distance, DISTANCE_SCALE),
        support_direction_spread_nrad: fixed_u(spread, ANGLE_SCALE),
        halfspace_inside_margin_nrad: inside.map(|v| fixed_i(v, ANGLE_SCALE)),
        halfspace_outside_margin_nrad: outside.map(|v| fixed_i(v, ANGLE_SCALE)),
        minimum_horizontal_inside_margin_nrad: fixed_i(inside[1].min(inside[2]), ANGLE_SCALE),
        minimum_vertical_inside_margin_nrad: fixed_i(inside[3].min(inside[4]), ANGLE_SCALE),
        forward_hemisphere_inside_margin_nrad: fixed_i(inside[0], ANGLE_SCALE),
        support_bound_input_asserted: case.support.enclosing_ball_asserted,
        precondition_proofs_verified_independently: false,
        occlusion_status: "NOT_ASSESSED".to_owned(),
        within_rectilinear_frustum_certified: status == FrustumStatus::Within,
        may_count_toward_visibility: false,
    }
}

pub fn audit_rectilinear_frustum_corpus(
    bytes: &[u8],
    source_commit: &str,
) -> Result<RectilinearFrustumReceipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let corpus: RectilinearFrustumCorpus =
        serde_json::from_slice(bytes).map_err(|e| format!("FRUSTUM_CORPUS_PARSE_FAILED:{e}"))?;
    if corpus.schema != FRUSTUM_CORPUS_SCHEMA
        || corpus.benchmark_id != FRUSTUM_BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != FRUSTUM_CLAIM_CEILING
        || corpus.cases.len() != FRUSTUM_CASE_COUNT
    {
        return Err("FRUSTUM_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }
    let mut ids = BTreeSet::new();
    let mut cases = Vec::with_capacity(corpus.cases.len());
    let mut matches = 0usize;
    for case in &corpus.cases {
        if !ids.insert(case.case_id.as_str()) {
            return Err("DUPLICATE_FRUSTUM_CASE_ID".to_owned());
        }
        let result = evaluate_rectilinear_frustum(case);
        matches += usize::from(result.status == case.expected_status);
        cases.push(result);
    }
    Ok(RectilinearFrustumReceipt {
        schema: FRUSTUM_RECEIPT_SCHEMA.to_owned(),
        benchmark_id: FRUSTUM_BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        input_sha256: sha256_hex(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        authority_granted: false,
        structural_acceptance_authorized: false,
        flight_authorized: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        occlusion_assessed: false,
        precondition_proofs_verified_independently: false,
        case_count: cases.len(),
        expected_outcome_matches: matches,
        expected_outcome_mismatches: cases.len() - matches,
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str =
        include_str!("../../../docs/research/orbital-truss/fixtures/rectilinear-frustum-v0_1.json");

    #[test]
    fn frozen_corpus_matches_all_expected_outcomes() {
        let receipt =
            audit_rectilinear_frustum_corpus(FIXTURE.as_bytes(), &"a".repeat(40)).unwrap();
        assert_eq!(receipt.case_count, FRUSTUM_CASE_COUNT);
        assert_eq!(receipt.expected_outcome_matches, FRUSTUM_CASE_COUNT);
        assert_eq!(receipt.expected_outcome_mismatches, 0);
        assert!(!receipt.authority_granted);
        assert!(!receipt.occlusion_assessed);
        assert!(receipt.cases.iter().all(|c| !c.may_count_toward_visibility));
    }

    #[test]
    fn narrow_vertical_fov_rejects_a_patch_that_passes_the_horizontal_margin() {
        let c: RectilinearFrustumCorpus = serde_json::from_str(FIXTURE).unwrap();
        let result = evaluate_rectilinear_frustum(&c.cases[1]);
        assert_eq!(result.status, FrustumStatus::Outside);
        assert!(result.minimum_horizontal_inside_margin_nrad.unwrap() > 0);
        assert!(result.minimum_vertical_inside_margin_nrad.unwrap() < 0);
    }

    #[test]
    fn camera_roll_rotates_the_rectangular_frustum_axes() {
        let c: RectilinearFrustumCorpus = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(
            evaluate_rectilinear_frustum(&c.cases[5]).status,
            FrustumStatus::Within
        );
    }

    #[test]
    fn support_intersection_with_a_frustum_boundary_abstains() {
        let c: RectilinearFrustumCorpus = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(
            evaluate_rectilinear_frustum(&c.cases[3]).status,
            FrustumStatus::Indeterminate
        );
        assert_eq!(
            evaluate_rectilinear_frustum(&c.cases[4]).status,
            FrustumStatus::Indeterminate
        );
    }

    #[test]
    fn unasserted_support_bound_never_certifies_a_pass() {
        let c: RectilinearFrustumCorpus = serde_json::from_str(FIXTURE).unwrap();
        let result = evaluate_rectilinear_frustum(&c.cases[7]);
        assert_eq!(result.status, FrustumStatus::Indeterminate);
        assert!(!result.within_rectilinear_frustum_certified);
    }

    #[test]
    fn invalid_camera_basis_is_rejected() {
        let c: RectilinearFrustumCorpus = serde_json::from_str(FIXTURE).unwrap();
        let mut case = c.cases[0].clone();
        case.camera.image_up_unit = [0.0, 0.0, -1.0];
        let result = evaluate_rectilinear_frustum(&case);
        assert_eq!(result.status, FrustumStatus::InvalidInput);
        assert_eq!(
            result.reason_codes,
            vec!["CAMERA_IMAGE_UP_NOT_ORTHOGONAL_TO_OPTICAL_AXIS"]
        );
    }

    #[test]
    fn enlarging_a_support_enclosure_cannot_upgrade_a_frustum_result() {
        let corpus: RectilinearFrustumCorpus = serde_json::from_str(FIXTURE).unwrap();
        let mut case = corpus.cases[0].clone();
        let baseline = evaluate_rectilinear_frustum(&case);
        assert_eq!(baseline.status, FrustumStatus::Within);
        case.support.enclosing_ball_radius_m = 0.5;
        let enlarged = evaluate_rectilinear_frustum(&case);
        assert_ne!(enlarged.status, FrustumStatus::Within);
        assert!(!enlarged.within_rectilinear_frustum_certified);
    }

    #[test]
    fn narrowing_vertical_or_horizontal_fov_cannot_upgrade_a_frustum_result() {
        let corpus: RectilinearFrustumCorpus = serde_json::from_str(FIXTURE).unwrap();
        let mut vertical = corpus.cases[0].clone();
        assert_eq!(
            evaluate_rectilinear_frustum(&vertical).status,
            FrustumStatus::Within
        );
        vertical.camera.vertical_fov_rad = 0.005;
        assert_ne!(
            evaluate_rectilinear_frustum(&vertical).status,
            FrustumStatus::Within
        );

        let mut horizontal = corpus.cases[0].clone();
        horizontal.camera.horizontal_fov_rad = 0.005;
        assert_ne!(
            evaluate_rectilinear_frustum(&horizontal).status,
            FrustumStatus::Within
        );
    }

    #[test]
    fn rigid_frame_transform_preserves_margins() {
        let c: RectilinearFrustumCorpus = serde_json::from_str(FIXTURE).unwrap();
        let original = evaluate_rectilinear_frustum(&c.cases[0]);
        let mut transformed = c.cases[0].clone();
        let angle: f64 = 0.731;
        let (sin, cos) = angle.sin_cos();
        let translation = [1.7, -2.3, 0.9];
        let point = |v: [f64; 3]| {
            [
                cos * v[0] - sin * v[1] + translation[0],
                sin * v[0] + cos * v[1] + translation[1],
                v[2] + translation[2],
            ]
        };
        let vector = |v: [f64; 3]| [cos * v[0] - sin * v[1], sin * v[0] + cos * v[1], v[2]];
        transformed.support.center_m = point(transformed.support.center_m);
        transformed.camera.position_m = point(transformed.camera.position_m);
        transformed.camera.optical_axis_unit = vector(transformed.camera.optical_axis_unit);
        transformed.camera.image_up_unit = vector(transformed.camera.image_up_unit);
        let actual = evaluate_rectilinear_frustum(&transformed);
        assert_eq!(actual.status, original.status);
        assert_eq!(actual.center_distance_nm, original.center_distance_nm);
        assert_eq!(
            actual.halfspace_inside_margin_nrad,
            original.halfspace_inside_margin_nrad
        );
        assert_eq!(
            actual.halfspace_outside_margin_nrad,
            original.halfspace_outside_margin_nrad
        );
    }
}
