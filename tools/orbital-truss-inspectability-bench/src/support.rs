use crate::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::f64::consts::PI;

pub const SUPPORT_CORPUS_SCHEMA: &str = "luminous.orbital-truss-support-envelope.corpus.v0.1";
pub const SUPPORT_RECEIPT_SCHEMA: &str = "luminous.orbital-truss-support-envelope.receipt.v0.1";
pub const SUPPORT_BENCHMARK_ID: &str = "ORBITAL-TRUSS-SUPPORT-ENVELOPE-001";
pub const SUPPORT_CLAIM_CEILING: &str =
    "synthetic_conditional_3d_support_sensing_envelope_bounds_only_occlusion_not_assessed";
pub const SUPPORT_CASE_COUNT: usize = 13;

const UNIT_EPS: f64 = 1.0e-6;
const GEOMETRY_EPS: f64 = 1.0e-12;
const DISTANCE_SCALE: f64 = 1.0e9;
const ANGLE_SCALE: f64 = 1.0e9;
const GSD_SCALE: f64 = 1.0e12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SupportStatus {
    ConditionalWithinSensingEnvelope,
    ConditionalOutsideSensingEnvelope,
    Indeterminate,
    InvalidInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OcclusionStatus {
    NotAssessed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportEnvelopeCorpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<SupportEnvelopeCase>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportEnvelopeCase {
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub length_unit: String,
    pub expected_status: SupportStatus,
    pub support: SurfaceSupport,
    pub sensor: SupportSensor,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceSupport {
    pub support_id: String,
    pub center_m: [f64; 3],
    pub enclosing_ball_radius_m: f64,
    pub enclosing_ball_asserted: bool,
    pub outward_normal_unit: [f64; 3],
    pub normal_cone_half_angle_rad: f64,
    pub normal_cone_asserted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TriangleSupportDerivation {
    pub support: SurfaceSupport,
    pub support_enclosure_derived_from_triangle_vertices: bool,
    pub planar_normal_cone_derived_from_triangle: bool,
    pub outward_winding_input_asserted: bool,
    pub numeric_method: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportSensor {
    pub sensor_id: String,
    pub camera_model_id: String,
    pub position_m: [f64; 3],
    pub optical_axis_unit: [f64; 3],
    pub horizontal_fov_rad: f64,
    pub image_width_px: u64,
    pub minimum_range_m: f64,
    pub maximum_range_m: f64,
    pub maximum_obliquity_rad: f64,
    pub maximum_gsd_m_per_px: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SupportEnvelopeCertificate {
    pub case_id: String,
    pub support_id: String,
    pub status: SupportStatus,
    pub reason_codes: Vec<String>,
    pub center_distance_nm: Option<u64>,
    pub support_direction_spread_nrad: Option<u64>,
    pub minimum_distance_lower_bound_nm: Option<u64>,
    pub maximum_distance_upper_bound_nm: Option<u64>,
    pub fov_angle_lower_bound_nrad: Option<u64>,
    pub fov_angle_upper_bound_nrad: Option<u64>,
    pub incidence_angle_lower_bound_nrad: Option<u64>,
    pub incidence_angle_upper_bound_nrad: Option<u64>,
    pub gsd_lower_bound_pm_per_px: Option<u64>,
    pub gsd_upper_bound_pm_per_px: Option<u64>,
    pub support_bound_input_asserted: bool,
    pub normal_cone_input_asserted: bool,
    pub precondition_proofs_verified_independently: bool,
    pub occlusion_status: OcclusionStatus,
    pub full_support_visibility_certified: bool,
    pub may_count_toward_full_support_coverage: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SupportEnvelopeReceipt {
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
    pub cases: Vec<SupportEnvelopeCertificate>,
}

fn finite3(value: [f64; 3]) -> bool {
    value.iter().all(|component| component.is_finite())
}

fn norm3(value: [f64; 3]) -> f64 {
    (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt()
}

fn dot3(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn subtract3(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn cross3(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

/// Derive a candidate support enclosure and constant normal cone for a planar triangle.
///
/// The triangle is the convex hull of its three vertices. Its centroid is used as
/// the ball centre and the maximum vertex distance as the radius. Convexity of the
/// Euclidean norm bounds every convex combination by that maximum. A small
/// coordinate-scale floating-point margin is added; this is not formal interval
/// arithmetic, nor a proof that the supplied vertices identify the physical part.
pub fn derive_planar_triangle_support(
    support_id: &str,
    vertices_m: [[f64; 3]; 3],
    outward_winding_input_asserted: bool,
) -> Result<TriangleSupportDerivation, String> {
    if support_id.trim().is_empty() || !vertices_m.iter().all(|vertex| finite3(*vertex)) {
        return Err("TRIANGLE_SUPPORT_ID_OR_VERTICES_INVALID".to_owned());
    }
    if !outward_winding_input_asserted {
        return Err("TRIANGLE_OUTWARD_WINDING_NOT_ASSERTED".to_owned());
    }

    let edge_a = subtract3(vertices_m[1], vertices_m[0]);
    let edge_b = subtract3(vertices_m[2], vertices_m[0]);
    let area_normal = cross3(edge_a, edge_b);
    let normal_length = norm3(area_normal);
    if !normal_length.is_finite() || normal_length <= GEOMETRY_EPS {
        return Err("TRIANGLE_SUPPORT_DEGENERATE".to_owned());
    }

    let center_m = [
        (vertices_m[0][0] + vertices_m[1][0] + vertices_m[2][0]) / 3.0,
        (vertices_m[0][1] + vertices_m[1][1] + vertices_m[2][1]) / 3.0,
        (vertices_m[0][2] + vertices_m[1][2] + vertices_m[2][2]) / 3.0,
    ];
    if !finite3(center_m) {
        return Err("TRIANGLE_SUPPORT_CENTROID_NONFINITE".to_owned());
    }

    let vertex_radius = vertices_m
        .iter()
        .map(|vertex| norm3(subtract3(*vertex, center_m)))
        .fold(0.0_f64, f64::max);
    let coordinate_scale = vertices_m
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(vertex_radius.max(1.0), f64::max);
    let enclosing_ball_radius_m = vertex_radius + (128.0 * f64::EPSILON * coordinate_scale);
    if !enclosing_ball_radius_m.is_finite() || enclosing_ball_radius_m <= 0.0 {
        return Err("TRIANGLE_SUPPORT_RADIUS_NONFINITE".to_owned());
    }

    let outward_normal_unit = [
        area_normal[0] / normal_length,
        area_normal[1] / normal_length,
        area_normal[2] / normal_length,
    ];
    Ok(TriangleSupportDerivation {
        support: SurfaceSupport {
            support_id: support_id.to_owned(),
            center_m,
            enclosing_ball_radius_m,
            enclosing_ball_asserted: true,
            outward_normal_unit,
            normal_cone_half_angle_rad: 0.0,
            normal_cone_asserted: true,
        },
        support_enclosure_derived_from_triangle_vertices: true,
        planar_normal_cone_derived_from_triangle: true,
        outward_winding_input_asserted: true,
        numeric_method:
            "FLOAT64_WITH_128_EPSILON_COORDINATE_SCALE_MARGIN_NOT_FORMAL_INTERVAL_ARITHMETIC"
                .to_owned(),
    })
}

fn fixed_nonnegative(value: f64, scale: f64) -> Option<u64> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let scaled = (value * scale + 0.5).floor();
    if !scaled.is_finite() || scaled > u64::MAX as f64 {
        None
    } else {
        Some(scaled as u64)
    }
}

fn invalid_certificate(case: &SupportEnvelopeCase, reason: &str) -> SupportEnvelopeCertificate {
    SupportEnvelopeCertificate {
        case_id: case.case_id.clone(),
        support_id: case.support.support_id.clone(),
        status: SupportStatus::InvalidInput,
        reason_codes: vec![reason.to_owned()],
        center_distance_nm: None,
        support_direction_spread_nrad: None,
        minimum_distance_lower_bound_nm: None,
        maximum_distance_upper_bound_nm: None,
        fov_angle_lower_bound_nrad: None,
        fov_angle_upper_bound_nrad: None,
        incidence_angle_lower_bound_nrad: None,
        incidence_angle_upper_bound_nrad: None,
        gsd_lower_bound_pm_per_px: None,
        gsd_upper_bound_pm_per_px: None,
        support_bound_input_asserted: case.support.enclosing_ball_asserted,
        normal_cone_input_asserted: case.support.normal_cone_asserted,
        precondition_proofs_verified_independently: false,
        occlusion_status: OcclusionStatus::NotAssessed,
        full_support_visibility_certified: false,
        may_count_toward_full_support_coverage: false,
    }
}

fn indeterminate_certificate(
    case: &SupportEnvelopeCase,
    mut reason_codes: Vec<String>,
) -> SupportEnvelopeCertificate {
    reason_codes.sort();
    reason_codes.dedup();
    SupportEnvelopeCertificate {
        case_id: case.case_id.clone(),
        support_id: case.support.support_id.clone(),
        status: SupportStatus::Indeterminate,
        reason_codes,
        center_distance_nm: None,
        support_direction_spread_nrad: None,
        minimum_distance_lower_bound_nm: None,
        maximum_distance_upper_bound_nm: None,
        fov_angle_lower_bound_nrad: None,
        fov_angle_upper_bound_nrad: None,
        incidence_angle_lower_bound_nrad: None,
        incidence_angle_upper_bound_nrad: None,
        gsd_lower_bound_pm_per_px: None,
        gsd_upper_bound_pm_per_px: None,
        support_bound_input_asserted: case.support.enclosing_ball_asserted,
        normal_cone_input_asserted: case.support.normal_cone_asserted,
        precondition_proofs_verified_independently: false,
        occlusion_status: OcclusionStatus::NotAssessed,
        full_support_visibility_certified: false,
        may_count_toward_full_support_coverage: false,
    }
}

fn validate_input(case: &SupportEnvelopeCase) -> Result<(), &'static str> {
    if case.case_id.trim().is_empty()
        || case.coordinate_frame_id.trim().is_empty()
        || case.length_unit != "METRE"
    {
        return Err("CASE_FRAME_OR_LENGTH_UNIT_INVALID");
    }
    let support = &case.support;
    if support.support_id.trim().is_empty()
        || !finite3(support.center_m)
        || !support.enclosing_ball_radius_m.is_finite()
        || support.enclosing_ball_radius_m <= 0.0
    {
        return Err("SUPPORT_ID_GEOMETRY_OR_RADIUS_INVALID");
    }
    let normal_length = norm3(support.outward_normal_unit);
    if !finite3(support.outward_normal_unit)
        || !normal_length.is_finite()
        || (normal_length - 1.0).abs() > UNIT_EPS
    {
        return Err("SUPPORT_NORMAL_NOT_UNIT");
    }
    if !support.normal_cone_half_angle_rad.is_finite()
        || support.normal_cone_half_angle_rad < 0.0
        || support.normal_cone_half_angle_rad >= PI
    {
        return Err("SUPPORT_NORMAL_CONE_INVALID");
    }

    let sensor = &case.sensor;
    let axis_length = norm3(sensor.optical_axis_unit);
    if sensor.sensor_id.trim().is_empty()
        || sensor.camera_model_id.trim().is_empty()
        || !finite3(sensor.position_m)
    {
        return Err("SENSOR_ID_OR_POSITION_INVALID");
    }
    if !finite3(sensor.optical_axis_unit)
        || !axis_length.is_finite()
        || (axis_length - 1.0).abs() > UNIT_EPS
    {
        return Err("SENSOR_OPTICAL_AXIS_NOT_UNIT");
    }
    if !sensor.horizontal_fov_rad.is_finite()
        || sensor.horizontal_fov_rad <= 0.0
        || sensor.horizontal_fov_rad >= PI
        || sensor.image_width_px == 0
    {
        return Err("CAMERA_FOV_OR_PIXEL_WIDTH_INVALID");
    }
    if !sensor.minimum_range_m.is_finite()
        || !sensor.maximum_range_m.is_finite()
        || sensor.minimum_range_m < 0.0
        || sensor.maximum_range_m <= sensor.minimum_range_m
    {
        return Err("SENSOR_RANGE_LIMITS_INVALID");
    }
    if !sensor.maximum_obliquity_rad.is_finite()
        || sensor.maximum_obliquity_rad < 0.0
        || sensor.maximum_obliquity_rad >= PI / 2.0
    {
        return Err("MAXIMUM_OBLIQUITY_INVALID");
    }
    if !sensor.maximum_gsd_m_per_px.is_finite() || sensor.maximum_gsd_m_per_px <= 0.0 {
        return Err("MAXIMUM_GSD_INVALID");
    }
    Ok(())
}

pub fn evaluate_support_envelope(case: &SupportEnvelopeCase) -> SupportEnvelopeCertificate {
    if let Err(reason) = validate_input(case) {
        return invalid_certificate(case, reason);
    }
    if !case.support.enclosing_ball_asserted {
        return indeterminate_certificate(case, vec!["SUPPORT_ENCLOSING_BALL_UNPROVEN".to_owned()]);
    }

    let support = &case.support;
    let sensor = &case.sensor;
    let sensor_to_center = subtract3(support.center_m, sensor.position_m);
    let center_distance = norm3(sensor_to_center);
    if !center_distance.is_finite() || center_distance <= support.enclosing_ball_radius_m {
        return indeterminate_certificate(
            case,
            vec!["SENSOR_NOT_OUTSIDE_SUPPORT_BOUND".to_owned()],
        );
    }

    let radius = support.enclosing_ball_radius_m;
    let spread = (radius / center_distance).clamp(0.0, 1.0).asin();
    let minimum_distance = center_distance - radius;
    let maximum_distance = center_distance + radius;
    let toward_support = [
        sensor_to_center[0] / center_distance,
        sensor_to_center[1] / center_distance,
        sensor_to_center[2] / center_distance,
    ];
    let toward_sensor = [-toward_support[0], -toward_support[1], -toward_support[2]];

    let center_fov_angle = dot3(sensor.optical_axis_unit, toward_support)
        .clamp(-1.0, 1.0)
        .acos();
    let fov_lower = (center_fov_angle - spread).max(0.0);
    let fov_upper = center_fov_angle + spread;

    let mut incidence_lower = None;
    let mut incidence_upper = None;
    let mut gsd_lower = None;
    let mut gsd_upper = None;
    if support.normal_cone_asserted {
        let center_incidence = dot3(support.outward_normal_unit, toward_sensor)
            .clamp(-1.0, 1.0)
            .acos();
        let normal_spread = support.normal_cone_half_angle_rad;
        let low = (center_incidence - spread - normal_spread).max(0.0);
        let high = center_incidence + spread + normal_spread;
        incidence_lower = Some(low);
        incidence_upper = Some(high);

        let gsd_factor =
            2.0 * (sensor.horizontal_fov_rad / 2.0).tan() / sensor.image_width_px as f64;
        if low < PI / 2.0 {
            gsd_lower = Some(gsd_factor * minimum_distance / low.cos());
        }
        if high < PI / 2.0 {
            gsd_upper = Some(gsd_factor * maximum_distance / high.cos());
        }
    }

    let mut outside_reasons = Vec::new();
    if maximum_distance < sensor.minimum_range_m - GEOMETRY_EPS {
        outside_reasons.push("ALL_SUPPORT_BELOW_MINIMUM_RANGE");
    }
    if minimum_distance > sensor.maximum_range_m + GEOMETRY_EPS {
        outside_reasons.push("ALL_SUPPORT_ABOVE_MAXIMUM_RANGE");
    }
    if fov_lower > sensor.horizontal_fov_rad / 2.0 + GEOMETRY_EPS {
        outside_reasons.push("ALL_SUPPORT_OUTSIDE_FIELD_OF_VIEW");
    }
    if let Some(low) = incidence_lower {
        if low > sensor.maximum_obliquity_rad + GEOMETRY_EPS {
            outside_reasons.push("ALL_SUPPORT_EXCEEDS_MAXIMUM_OBLIQUITY");
        }
    }
    if let Some(low) = gsd_lower {
        if low > sensor.maximum_gsd_m_per_px + GEOMETRY_EPS {
            outside_reasons.push("ALL_SUPPORT_EXCEEDS_MAXIMUM_GSD");
        }
    }

    let status = if !outside_reasons.is_empty() {
        SupportStatus::ConditionalOutsideSensingEnvelope
    } else {
        let distance_passes = minimum_distance >= sensor.minimum_range_m - GEOMETRY_EPS
            && maximum_distance <= sensor.maximum_range_m + GEOMETRY_EPS;
        let fov_passes = fov_upper <= sensor.horizontal_fov_rad / 2.0 + GEOMETRY_EPS;
        let incidence_passes = support.normal_cone_asserted
            && incidence_upper.is_some_and(|angle| {
                angle <= sensor.maximum_obliquity_rad + GEOMETRY_EPS && angle < PI / 2.0
            });
        let gsd_passes =
            gsd_upper.is_some_and(|gsd| gsd <= sensor.maximum_gsd_m_per_px + GEOMETRY_EPS);
        if distance_passes && fov_passes && incidence_passes && gsd_passes {
            SupportStatus::ConditionalWithinSensingEnvelope
        } else {
            SupportStatus::Indeterminate
        }
    };

    let mut reason_codes = if !outside_reasons.is_empty() {
        outside_reasons
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    } else if status == SupportStatus::ConditionalWithinSensingEnvelope {
        vec![
            "WHOLE_SUPPORT_RANGE_BOUNDS_PASS".to_owned(),
            "WHOLE_SUPPORT_FOV_BOUND_PASSES".to_owned(),
            "WHOLE_SUPPORT_INCIDENCE_BOUND_PASSES".to_owned(),
            "WHOLE_SUPPORT_GSD_BOUND_PASSES".to_owned(),
            "OCCLUSION_NOT_ASSESSED".to_owned(),
        ]
    } else {
        let mut reasons = Vec::new();
        let distance_passes = minimum_distance >= sensor.minimum_range_m - GEOMETRY_EPS
            && maximum_distance <= sensor.maximum_range_m + GEOMETRY_EPS;
        if !distance_passes {
            reasons.push("RANGE_UPPER_BOUND_NOT_CERTIFIED".to_owned());
        }
        if fov_upper > sensor.horizontal_fov_rad / 2.0 + GEOMETRY_EPS {
            reasons.push("FOV_UPPER_BOUND_NOT_CERTIFIED".to_owned());
        }
        if !support.normal_cone_asserted {
            reasons.push("NORMAL_CONE_UNPROVEN".to_owned());
        } else if incidence_upper.is_none_or(|angle| {
            angle > sensor.maximum_obliquity_rad + GEOMETRY_EPS || angle >= PI / 2.0
        }) {
            reasons.push("INCIDENCE_UPPER_BOUND_NOT_CERTIFIED".to_owned());
        }
        if gsd_upper.is_none_or(|gsd| gsd > sensor.maximum_gsd_m_per_px + GEOMETRY_EPS) {
            reasons.push("GSD_UPPER_BOUND_NOT_CERTIFIED".to_owned());
        }
        reasons
    };
    reason_codes.sort();
    reason_codes.dedup();

    SupportEnvelopeCertificate {
        case_id: case.case_id.clone(),
        support_id: support.support_id.clone(),
        status,
        reason_codes,
        center_distance_nm: fixed_nonnegative(center_distance, DISTANCE_SCALE),
        support_direction_spread_nrad: fixed_nonnegative(spread, ANGLE_SCALE),
        minimum_distance_lower_bound_nm: fixed_nonnegative(minimum_distance, DISTANCE_SCALE),
        maximum_distance_upper_bound_nm: fixed_nonnegative(maximum_distance, DISTANCE_SCALE),
        fov_angle_lower_bound_nrad: fixed_nonnegative(fov_lower, ANGLE_SCALE),
        fov_angle_upper_bound_nrad: fixed_nonnegative(fov_upper, ANGLE_SCALE),
        incidence_angle_lower_bound_nrad: incidence_lower
            .and_then(|value| fixed_nonnegative(value, ANGLE_SCALE)),
        incidence_angle_upper_bound_nrad: incidence_upper
            .and_then(|value| fixed_nonnegative(value, ANGLE_SCALE)),
        gsd_lower_bound_pm_per_px: gsd_lower.and_then(|value| fixed_nonnegative(value, GSD_SCALE)),
        gsd_upper_bound_pm_per_px: gsd_upper.and_then(|value| fixed_nonnegative(value, GSD_SCALE)),
        support_bound_input_asserted: support.enclosing_ball_asserted,
        normal_cone_input_asserted: support.normal_cone_asserted,
        precondition_proofs_verified_independently: false,
        occlusion_status: OcclusionStatus::NotAssessed,
        full_support_visibility_certified: false,
        may_count_toward_full_support_coverage: false,
    }
}

pub fn audit_support_corpus(
    bytes: &[u8],
    source_commit: &str,
) -> Result<SupportEnvelopeReceipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let corpus: SupportEnvelopeCorpus = serde_json::from_slice(bytes)
        .map_err(|error| format!("SUPPORT_CORPUS_PARSE_FAILED:{error}"))?;
    if corpus.schema != SUPPORT_CORPUS_SCHEMA
        || corpus.benchmark_id != SUPPORT_BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != SUPPORT_CLAIM_CEILING
        || corpus.cases.len() != SUPPORT_CASE_COUNT
    {
        return Err("SUPPORT_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }

    let mut ids = BTreeSet::new();
    let mut cases = Vec::with_capacity(corpus.cases.len());
    let mut expected_outcome_matches = 0;
    let mut expected_outcome_mismatches = 0;
    for case in &corpus.cases {
        if !ids.insert(case.case_id.as_str()) {
            return Err("DUPLICATE_SUPPORT_CASE_ID".to_owned());
        }
        let certificate = evaluate_support_envelope(case);
        if certificate.status == case.expected_status {
            expected_outcome_matches += 1;
        } else {
            expected_outcome_mismatches += 1;
        }
        cases.push(certificate);
    }

    Ok(SupportEnvelopeReceipt {
        schema: SUPPORT_RECEIPT_SCHEMA.to_owned(),
        benchmark_id: SUPPORT_BENCHMARK_ID.to_owned(),
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
        expected_outcome_matches,
        expected_outcome_mismatches,
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../docs/research/orbital-truss/fixtures/support-envelope-v0_1.json");

    #[test]
    fn frozen_support_corpus_matches_all_adjudicated_outcomes() {
        let receipt = audit_support_corpus(FIXTURE.as_bytes(), &"a".repeat(40)).unwrap();
        let corpus: SupportEnvelopeCorpus = serde_json::from_str(FIXTURE).unwrap();
        let mismatches = receipt
            .cases
            .iter()
            .zip(&corpus.cases)
            .filter(|(actual, expected)| actual.status != expected.expected_status)
            .map(|(actual, expected)| {
                format!(
                    "{} actual={:?} expected={:?} reasons={:?}",
                    actual.case_id, actual.status, expected.expected_status, actual.reason_codes
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(receipt.case_count, SUPPORT_CASE_COUNT);
        assert_eq!(
            receipt.expected_outcome_matches, SUPPORT_CASE_COUNT,
            "support-envelope corpus mismatches: {mismatches:?}"
        );
        assert_eq!(receipt.expected_outcome_mismatches, 0);
        assert!(!receipt.authority_granted);
        assert!(!receipt.occlusion_assessed);
        assert!(
            receipt
                .cases
                .iter()
                .all(|case| !case.full_support_visibility_certified)
        );
        assert!(
            receipt
                .cases
                .iter()
                .all(|case| !case.may_count_toward_full_support_coverage)
        );
    }

    #[test]
    fn rigid_frame_rotation_and_translation_preserve_the_certificate() {
        let corpus: SupportEnvelopeCorpus = serde_json::from_str(FIXTURE).unwrap();
        let original = evaluate_support_envelope(&corpus.cases[0]);
        let mut transformed = corpus.cases[0].clone();
        let angle: f64 = 0.731;
        let (sin, cos) = angle.sin_cos();
        let translation = [1.7, -2.3, 0.9];
        let rotate = |v: [f64; 3]| {
            [
                cos * v[0] - sin * v[1] + translation[0],
                sin * v[0] + cos * v[1] + translation[1],
                v[2] + translation[2],
            ]
        };
        let rotate_direction =
            |v: [f64; 3]| [cos * v[0] - sin * v[1], sin * v[0] + cos * v[1], v[2]];
        transformed.support.center_m = rotate(transformed.support.center_m);
        transformed.sensor.position_m = rotate(transformed.sensor.position_m);
        transformed.support.outward_normal_unit =
            rotate_direction(transformed.support.outward_normal_unit);
        transformed.sensor.optical_axis_unit =
            rotate_direction(transformed.sensor.optical_axis_unit);
        let actual = evaluate_support_envelope(&transformed);
        assert_eq!(actual.status, original.status);
        assert_eq!(actual.center_distance_nm, original.center_distance_nm);
        assert_eq!(
            actual.support_direction_spread_nrad,
            original.support_direction_spread_nrad
        );
        assert_eq!(
            actual.minimum_distance_lower_bound_nm,
            original.minimum_distance_lower_bound_nm
        );
        assert_eq!(
            actual.maximum_distance_upper_bound_nm,
            original.maximum_distance_upper_bound_nm
        );
        assert_eq!(
            actual.fov_angle_upper_bound_nrad,
            original.fov_angle_upper_bound_nrad
        );
        assert_eq!(
            actual.incidence_angle_upper_bound_nrad,
            original.incidence_angle_upper_bound_nrad
        );
        assert_eq!(
            actual.gsd_upper_bound_pm_per_px,
            original.gsd_upper_bound_pm_per_px
        );
    }

    #[test]
    fn enlarging_a_proven_support_cannot_upgrade_an_indeterminate_case() {
        let corpus: SupportEnvelopeCorpus = serde_json::from_str(FIXTURE).unwrap();
        let mut case = corpus.cases[4].clone();
        case.support.enclosing_ball_radius_m = 0.01;
        let small = evaluate_support_envelope(&case);
        assert_eq!(
            small.status,
            SupportStatus::ConditionalWithinSensingEnvelope
        );
        case.support.enclosing_ball_radius_m = 0.2;
        let large = evaluate_support_envelope(&case);
        assert_ne!(
            large.status,
            SupportStatus::ConditionalWithinSensingEnvelope
        );
    }

    #[test]
    fn a_wider_normal_cone_cannot_upgrade_a_certificate() {
        let corpus: SupportEnvelopeCorpus = serde_json::from_str(FIXTURE).unwrap();
        let mut case = corpus.cases[0].clone();
        let narrow = evaluate_support_envelope(&case);
        assert_eq!(
            narrow.status,
            SupportStatus::ConditionalWithinSensingEnvelope
        );
        case.support.normal_cone_half_angle_rad = 0.6;
        let wide = evaluate_support_envelope(&case);
        assert_ne!(wide.status, SupportStatus::ConditionalWithinSensingEnvelope);
    }

    #[test]
    fn planar_triangle_derivation_encloses_a_dense_barycentric_grid() {
        let vertices = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let derived = derive_planar_triangle_support("TRIANGLE-1", vertices, true).unwrap();
        let center = derived.support.center_m;
        let radius = derived.support.enclosing_ball_radius_m;
        assert!(derived.support_enclosure_derived_from_triangle_vertices);
        assert!(derived.planar_normal_cone_derived_from_triangle);
        assert!(derived.outward_winding_input_asserted);
        assert_eq!(derived.support.normal_cone_half_angle_rad, 0.0);
        assert!(derived.support.enclosing_ball_asserted);
        assert!(derived.support.normal_cone_asserted);

        for i in 0..=100 {
            for j in 0..=(100 - i) {
                let u = i as f64 / 100.0;
                let v = j as f64 / 100.0;
                let w = 1.0 - u - v;
                let point = [
                    vertices[0][0] * w + vertices[1][0] * u + vertices[2][0] * v,
                    vertices[0][1] * w + vertices[1][1] * u + vertices[2][1] * v,
                    vertices[0][2] * w + vertices[1][2] * u + vertices[2][2] * v,
                ];
                assert!(norm3(subtract3(point, center)) <= radius);
            }
        }
        assert!((norm3(derived.support.outward_normal_unit) - 1.0).abs() < UNIT_EPS);
    }

    #[test]
    fn planar_triangle_derivation_requires_declared_outward_winding() {
        let vertices = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        assert_eq!(
            derive_planar_triangle_support("TRIANGLE-1", vertices, false),
            Err("TRIANGLE_OUTWARD_WINDING_NOT_ASSERTED".to_owned())
        );
    }

    #[test]
    fn planar_triangle_derivation_rejects_degenerate_faces() {
        let vertices = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        assert_eq!(
            derive_planar_triangle_support("TRIANGLE-1", vertices, true),
            Err("TRIANGLE_SUPPORT_DEGENERATE".to_owned())
        );
    }

    #[test]
    fn envelope_pass_never_claims_full_visibility_without_occlusion_evidence() {
        let corpus: SupportEnvelopeCorpus = serde_json::from_str(FIXTURE).unwrap();
        let certificate = evaluate_support_envelope(&corpus.cases[0]);
        assert_eq!(
            certificate.status,
            SupportStatus::ConditionalWithinSensingEnvelope
        );
        assert_eq!(certificate.occlusion_status, OcclusionStatus::NotAssessed);
        assert!(!certificate.full_support_visibility_certified);
        assert!(!certificate.may_count_toward_full_support_coverage);
    }
}
