#![forbid(unsafe_code)]

use orbital_gsd_kcoverage_bench::{
    CandidateView, GsdCase, GsdReceipt, SurfaceSupport, evaluate_bytes as evaluate_gsd,
    surface_supports,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const SUPPORT_RECEIPT_SCHEMA: &str = "luminous.orbital-surface-support.receipt.v0.1";
pub const SUPPORT_BENCHMARK_ID: &str = "ORBITAL-SURFACE-SUPPORT-VOXEL-001";
const SENSOR_POSITION_QUANTIZATION_MARGIN_M: f64 = 1.0e-6;
const OPTICAL_AXIS_QUANTIZATION_MARGIN_RAD: f64 = 2.0e-6;
const BOUND_EPS: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SupportDisposition {
    CertifiedVisible,
    Indeterminate,
    CentreNotVisible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SupportCertificate {
    pub support_id: String,
    pub feature_id: String,
    pub source_cell: [usize; 3],
    pub view_id: String,
    pub centre_visible_in_upstream_model: bool,
    pub disposition: SupportDisposition,
    pub reason_codes: Vec<String>,
    pub support_radius_um_upper: Option<i64>,
    pub centre_distance_um: Option<i64>,
    pub minimum_distance_um_lower: Option<i64>,
    pub maximum_distance_um_upper: Option<i64>,
    pub direction_variation_microrad_upper: Option<i64>,
    pub centre_fov_microrad: Option<i64>,
    pub support_fov_microrad_upper: Option<i64>,
    pub centre_incidence_microrad: Option<i64>,
    pub support_incidence_microrad_upper: Option<i64>,
    pub gsd_microm_per_pixel_upper: Option<i64>,
    pub derived_slant_range_um_upper_limit: i64,
    pub occlusion_disposition: String,
    pub occlusion_candidate_cells: Vec<[usize; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SupportReceipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub source_case_id: String,
    pub coordinate_frame_id: String,
    pub source_input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub source_candidate_view_count: usize,
    pub source_surface_support_count: usize,
    pub candidate_support_pair_count: usize,
    pub centre_visible_pair_count: usize,
    pub centre_not_visible_pair_count: usize,
    pub certified_visible_pair_count: usize,
    pub indeterminate_pair_count: usize,
    pub authority_granted: bool,
    pub structural_acceptance_authorized: bool,
    pub flight_authorized: bool,
    pub physical_validation: String,
    pub certificates: Vec<SupportCertificate>,
}

#[derive(Debug, Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn from_array(values: [f64; 3]) -> Self {
        Self {
            x: values[0],
            y: values[1],
            z: values[2],
        }
    }
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
    fn norm(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    fn normalized(self) -> Option<Self> {
        let norm = self.norm();
        (norm.is_finite() && norm > 1.0e-12).then(|| Self {
            x: self.x / norm,
            y: self.y / norm,
            z: self.z / norm,
        })
    }
    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    fn angle(self, other: Self) -> Option<f64> {
        let left = self.normalized()?;
        let right = other.normalized()?;
        Some(left.dot(right).clamp(-1.0, 1.0).acos())
    }
    fn array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

#[derive(Debug, Clone, Copy)]
struct Aabb {
    min: [f64; 3],
    max: [f64; 3],
}

impl Aabb {
    fn ray_enclosure(sensor: Vec3, support_centre: Vec3, half_extents: Vec3) -> Self {
        let centre = support_centre.array();
        let sensor = sensor.array();
        let half = half_extents.array();
        let mut min = [0.0; 3];
        let mut max = [0.0; 3];
        for axis in 0..3 {
            min[axis] = sensor[axis].min(centre[axis] - half[axis]);
            max[axis] = sensor[axis].max(centre[axis] + half[axis]);
        }
        Self { min, max }
    }
    fn intersects(self, other: Self) -> bool {
        (0..3).all(|axis| {
            self.max[axis] + BOUND_EPS >= other.min[axis]
                && other.max[axis] + BOUND_EPS >= self.min[axis]
        })
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn round_scaled(value: f64, scale: f64) -> Option<i64> {
    let scaled = value * scale;
    (scaled.is_finite() && scaled >= i64::MIN as f64 && scaled <= i64::MAX as f64)
        .then(|| scaled.round() as i64)
}
fn floor_scaled(value: f64, scale: f64) -> Option<i64> {
    let scaled = value * scale;
    (scaled.is_finite() && scaled >= i64::MIN as f64 && scaled <= i64::MAX as f64)
        .then(|| scaled.floor() as i64)
}
fn ceil_scaled(value: f64, scale: f64) -> Option<i64> {
    let scaled = value * scale;
    (scaled.is_finite() && scaled >= i64::MIN as f64 && scaled <= i64::MAX as f64)
        .then(|| scaled.ceil() as i64)
}

fn occlusion_aabb_candidates(
    case: &GsdCase,
    support: &SurfaceSupport,
    sensor: Vec3,
) -> Vec<[usize; 3]> {
    let centre = Vec3::from_array([
        support.centre_um[0] as f64 / 1.0e6,
        support.centre_um[1] as f64 / 1.0e6,
        support.centre_um[2] as f64 / 1.0e6,
    ]);
    let half = Vec3::from_array([
        support.half_extents_um[0] as f64 / 1.0e6,
        support.half_extents_um[1] as f64 / 1.0e6,
        support.half_extents_um[2] as f64 / 1.0e6,
    ]);
    let enclosure = Aabb::ray_enclosure(sensor, centre, half);
    let origin = case.grid.origin_m;
    let voxel = case.grid.voxel_size_m;
    let mut occupied = case.grid.occupied_cells.clone();
    occupied.sort_unstable();
    occupied
        .into_iter()
        .filter(|cell| *cell != support.cell)
        .filter(|cell| {
            let min = [
                origin[0] + cell[0] as f64 * voxel,
                origin[1] + cell[1] as f64 * voxel,
                origin[2] + cell[2] as f64 * voxel,
            ];
            let max = [min[0] + voxel, min[1] + voxel, min[2] + voxel];
            enclosure.intersects(Aabb { min, max })
        })
        .collect()
}

fn centre_not_visible(
    support: &SurfaceSupport,
    candidate: &CandidateView,
    slant_limit_um: i64,
) -> SupportCertificate {
    SupportCertificate {
        support_id: support.support_id.clone(),
        feature_id: support.feature_id.clone(),
        source_cell: support.cell,
        view_id: candidate.view_id.clone(),
        centre_visible_in_upstream_model: false,
        disposition: SupportDisposition::CentreNotVisible,
        reason_codes: vec!["UPSTREAM_CENTRE_VISIBILITY_FALSE".to_owned()],
        support_radius_um_upper: None,
        centre_distance_um: None,
        minimum_distance_um_lower: None,
        maximum_distance_um_upper: None,
        direction_variation_microrad_upper: None,
        centre_fov_microrad: None,
        support_fov_microrad_upper: None,
        centre_incidence_microrad: None,
        support_incidence_microrad_upper: None,
        gsd_microm_per_pixel_upper: None,
        derived_slant_range_um_upper_limit: slant_limit_um,
        occlusion_disposition: "NOT_EVALUATED_CENTRE_NOT_VISIBLE".to_owned(),
        occlusion_candidate_cells: Vec::new(),
    }
}

fn evaluate_pair(
    case: &GsdCase,
    receipt: &GsdReceipt,
    support: &SurfaceSupport,
    candidate: &CandidateView,
) -> SupportCertificate {
    let slant_limit_um = receipt.slant_range_max_um;
    if !candidate.visible_feature_ids.contains(&support.feature_id) {
        return centre_not_visible(support, candidate, slant_limit_um);
    }

    let centre = Vec3::from_array([
        support.centre_um[0] as f64 / 1.0e6,
        support.centre_um[1] as f64 / 1.0e6,
        support.centre_um[2] as f64 / 1.0e6,
    ]);
    let half = Vec3::from_array([
        support.half_extents_um[0] as f64 / 1.0e6,
        support.half_extents_um[1] as f64 / 1.0e6,
        support.half_extents_um[2] as f64 / 1.0e6,
    ]);
    let radius = half.norm() + SENSOR_POSITION_QUANTIZATION_MARGIN_M;
    let sensor = Vec3::from_array([
        candidate.position_um[0] as f64 / 1.0e6,
        candidate.position_um[1] as f64 / 1.0e6,
        candidate.position_um[2] as f64 / 1.0e6,
    ]);
    let optical_axis = Vec3::from_array([
        candidate.optical_axis_q1e6[0] as f64 / 1.0e6,
        candidate.optical_axis_q1e6[1] as f64 / 1.0e6,
        candidate.optical_axis_q1e6[2] as f64 / 1.0e6,
    ]);
    let normal = Vec3::from_array([
        support.outward_normal_q1e6[0] as f64 / 1.0e6,
        support.outward_normal_q1e6[1] as f64 / 1.0e6,
        support.outward_normal_q1e6[2] as f64 / 1.0e6,
    ]);
    let normal_cone = support.normal_cone_half_angle_microrad as f64 / 1.0e6;
    let to_support = centre.sub(sensor);
    let distance = to_support.norm();
    let mut reasons = Vec::new();

    let mut distance_min = None;
    let mut distance_max = None;
    let mut direction_delta = None;
    let mut centre_fov = None;
    let mut upper_fov = None;
    let mut centre_incidence = None;
    let mut upper_incidence = None;
    let mut gsd_nano_upper = None;
    let radius_um = ceil_scaled(radius, 1.0e6);

    if !radius.is_finite()
        || !distance.is_finite()
        || radius <= 0.0
        || !normal_cone.is_finite()
        || normal_cone < 0.0
        || normal_cone > std::f64::consts::PI
    {
        reasons.push("SUPPORT_GEOMETRY_BOUND_INVALID".to_owned());
    } else if distance <= radius + BOUND_EPS {
        reasons.push("SENSOR_NOT_OUTSIDE_SUPPORT_BOUND".to_owned());
    } else {
        let delta = (radius / distance).clamp(0.0, 1.0).asin();
        let d_min = distance - radius;
        let d_max = distance + radius;
        let eta0 = optical_axis.angle(to_support);
        let alpha0 = normal.angle(sensor.sub(centre));
        if let (Some(eta0), Some(alpha0)) = (eta0, alpha0) {
            let eta_upper = eta0 + delta + OPTICAL_AXIS_QUANTIZATION_MARGIN_RAD;
            let alpha_upper = alpha0 + delta + normal_cone;
            let slant_limit = slant_limit_um as f64 / 1.0e6 - SENSOR_POSITION_QUANTIZATION_MARGIN_M;

            distance_min = floor_scaled(d_min, 1.0e6);
            distance_max = ceil_scaled(d_max, 1.0e6);
            direction_delta = ceil_scaled(delta, 1.0e6);
            centre_fov = round_scaled(eta0, 1.0e6);
            upper_fov = ceil_scaled(eta_upper, 1.0e6);
            centre_incidence = round_scaled(alpha0, 1.0e6);
            upper_incidence = ceil_scaled(alpha_upper, 1.0e6);

            if d_max > slant_limit + BOUND_EPS {
                reasons.push("SUPPORT_RANGE_UPPER_BOUND_EXCEEDS_LIMIT".to_owned());
            }
            if eta_upper > case.camera.horizontal_fov_rad * 0.5 + BOUND_EPS {
                reasons.push("SUPPORT_FOV_UPPER_BOUND_EXCEEDS_LIMIT".to_owned());
            }
            if alpha_upper >= std::f64::consts::FRAC_PI_2 - BOUND_EPS {
                reasons.push("SUPPORT_INCIDENCE_BOUND_REACHES_GRAZING".to_owned());
            } else if alpha_upper > case.camera.max_oblique_angle_rad + BOUND_EPS {
                reasons.push("SUPPORT_INCIDENCE_UPPER_BOUND_EXCEEDS_LIMIT".to_owned());
            } else {
                let gsd_upper = d_max * (2.0 * (case.camera.horizontal_fov_rad * 0.5).tan())
                    / (f64::from(case.camera.horizontal_pixel_count) * alpha_upper.cos());
                gsd_nano_upper = ceil_scaled(gsd_upper, 1.0e9);
                if !gsd_upper.is_finite() || gsd_upper > case.camera.gsd_max_m_per_pixel + BOUND_EPS
                {
                    reasons.push("SUPPORT_GSD_UPPER_BOUND_EXCEEDS_LIMIT".to_owned());
                }
            }
        } else {
            reasons.push("SUPPORT_DIRECTION_OR_NORMAL_UNDEFINED".to_owned());
        }
    }

    let mut occlusion_cells = Vec::new();
    let occlusion_disposition;
    if reasons.is_empty() {
        occlusion_cells = occlusion_aabb_candidates(case, support, sensor);
        if occlusion_cells.is_empty() {
            occlusion_disposition = "CLEAR_CERTIFIED_BY_RAY_ENCLOSURE_AABB_DISJOINTNESS".to_owned();
        } else {
            reasons.push("OCCLUSION_CONSERVATIVE_ENCLOSURE_OVERLAP".to_owned());
            occlusion_disposition = "INDETERMINATE_CONSERVATIVE_ENCLOSURE_OVERLAP".to_owned();
        }
    } else {
        occlusion_disposition = "NOT_EVALUATED_GEOMETRY_BOUND_INDETERMINATE".to_owned();
    }

    SupportCertificate {
        support_id: support.support_id.clone(),
        feature_id: support.feature_id.clone(),
        source_cell: support.cell,
        view_id: candidate.view_id.clone(),
        centre_visible_in_upstream_model: true,
        disposition: if reasons.is_empty() {
            SupportDisposition::CertifiedVisible
        } else {
            SupportDisposition::Indeterminate
        },
        reason_codes: reasons,
        support_radius_um_upper: radius_um,
        centre_distance_um: round_scaled(distance, 1.0e6),
        minimum_distance_um_lower: distance_min,
        maximum_distance_um_upper: distance_max,
        direction_variation_microrad_upper: direction_delta,
        centre_fov_microrad: centre_fov,
        support_fov_microrad_upper: upper_fov,
        centre_incidence_microrad: centre_incidence,
        support_incidence_microrad_upper: upper_incidence,
        gsd_microm_per_pixel_upper: gsd_nano_upper.map(|value| (value + 999) / 1000),
        derived_slant_range_um_upper_limit: slant_limit_um,
        occlusion_disposition,
        occlusion_candidate_cells: occlusion_cells,
    }
}

pub fn evaluate_bytes(
    source_case_bytes: &[u8],
    source_commit: &str,
) -> Result<SupportReceipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let case: GsdCase = serde_json::from_slice(source_case_bytes)
        .map_err(|error| format!("SOURCE_CASE_PARSE_FAILED:{error}"))?;
    let source_receipt = evaluate_gsd(source_case_bytes, source_commit)?;
    let supports = surface_supports(&case)?;
    if supports.is_empty() || source_receipt.candidate_views.is_empty() {
        return Err("SOURCE_GEOMETRY_HAS_NO_SUPPORTS_OR_CANDIDATES".to_owned());
    }
    let feature_ids: BTreeSet<&str> = source_receipt
        .surface_features
        .iter()
        .map(|feature| feature.feature_id.as_str())
        .collect();
    if supports
        .iter()
        .any(|support| !feature_ids.contains(support.feature_id.as_str()))
    {
        return Err("SUPPORT_FEATURE_ID_NOT_PRESENT_IN_SOURCE_RECEIPT".to_owned());
    }
    let pair_count = source_receipt
        .candidate_views
        .len()
        .checked_mul(supports.len())
        .ok_or_else(|| "SUPPORT_PAIR_COUNT_OVERFLOW".to_owned())?;
    if pair_count > 500_000 {
        return Err("SUPPORT_PAIR_LIMIT_EXCEEDED".to_owned());
    }
    let mut certificates = Vec::with_capacity(pair_count);
    for candidate in &source_receipt.candidate_views {
        for support in &supports {
            certificates.push(evaluate_pair(&case, &source_receipt, support, candidate));
        }
    }
    let centre_visible_pair_count = certificates
        .iter()
        .filter(|row| row.centre_visible_in_upstream_model)
        .count();
    let centre_not_visible_pair_count = certificates.len() - centre_visible_pair_count;
    let certified_visible_pair_count = certificates
        .iter()
        .filter(|row| row.disposition == SupportDisposition::CertifiedVisible)
        .count();
    let indeterminate_pair_count = certificates
        .iter()
        .filter(|row| row.disposition == SupportDisposition::Indeterminate)
        .count();
    if centre_visible_pair_count + centre_not_visible_pair_count != pair_count
        || certified_visible_pair_count + indeterminate_pair_count != centre_visible_pair_count
    {
        return Err("SUPPORT_RECEIPT_ACCOUNTING_INVARIANT_FAILED".to_owned());
    }

    Ok(SupportReceipt {
        schema: SUPPORT_RECEIPT_SCHEMA.to_owned(),
        benchmark_id: SUPPORT_BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        source_case_id: case.case_id,
        coordinate_frame_id: case.coordinate_frame_id,
        source_input_sha256: sha256(source_case_bytes),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling:
            "synthetic_3d_voxel_surface_support_envelope_and_conservative_occlusion_only".to_owned(),
        source_candidate_view_count: source_receipt.candidate_views.len(),
        source_surface_support_count: supports.len(),
        candidate_support_pair_count: pair_count,
        centre_visible_pair_count,
        centre_not_visible_pair_count,
        certified_visible_pair_count,
        indeterminate_pair_count,
        authority_granted: false,
        structural_acceptance_authorized: false,
        flight_authorized: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        certificates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &[u8] = include_bytes!(
        "../../../docs/research/orbital-gsd-kcoverage/fixtures/voxel-visibility-v0_1.json"
    );

    #[test]
    fn support_receipt_is_deterministic_and_never_upgrades_centre_only_pairs() {
        let first = evaluate_bytes(SOURCE, &"a".repeat(40)).unwrap();
        let second = evaluate_bytes(SOURCE, &"a".repeat(40)).unwrap();
        assert_eq!(first, second);
        assert!(first.source_surface_support_count > 0);
        assert!(first.centre_visible_pair_count > 0);
        assert_eq!(
            first.centre_visible_pair_count + first.centre_not_visible_pair_count,
            first.candidate_support_pair_count
        );
        assert_eq!(
            first.certified_visible_pair_count + first.indeterminate_pair_count,
            first.centre_visible_pair_count
        );
        assert!(first.certificates.iter().all(|row| {
            row.disposition != SupportDisposition::CertifiedVisible
                || (row.centre_visible_in_upstream_model
                    && row.reason_codes.is_empty()
                    && row.occlusion_candidate_cells.is_empty()
                    && row.occlusion_disposition
                        == "CLEAR_CERTIFIED_BY_RAY_ENCLOSURE_AABB_DISJOINTNESS")
        }));
        assert!(!first.authority_granted);
        assert_eq!(first.physical_validation, "NOT_ASSESSED");
    }

    #[test]
    fn conservative_ray_enclosure_overlap_includes_boundary_contact() {
        let left = Aabb {
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        };
        let touching = Aabb {
            min: [1.0, 0.5, 0.5],
            max: [2.0, 0.75, 0.75],
        };
        assert!(left.intersects(touching));
    }

    #[test]
    fn support_ball_radius_contains_every_declared_box_corner() {
        let half = Vec3 {
            x: 0.05,
            y: 0.05,
            z: 0.0,
        };
        let radius = half.norm();
        for x in [-0.05, 0.05] {
            for y in [-0.05, 0.05] {
                assert!((Vec3 { x, y, z: 0.0 }).norm() <= radius + 1.0e-12);
            }
        }
    }

    #[test]
    fn widening_support_never_reduces_the_direction_bound() {
        let distance = 0.5_f64;
        let narrow = (0.04_f64 / distance).asin();
        let wide = (0.08_f64 / distance).asin();
        assert!(wide >= narrow);
    }

    #[test]
    fn widening_normal_cone_never_reduces_the_incidence_bound() {
        let centre_incidence = 0.2_f64;
        let delta = 0.1_f64;
        assert!(centre_incidence + delta + 0.15 >= centre_incidence + delta + 0.05);
    }
}
