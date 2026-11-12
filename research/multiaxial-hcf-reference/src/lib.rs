//! Bounded stress-based multiaxial HCF reference.
//!
//! This crate evaluates the Findley critical-plane parameter on one declared
//! stabilized stress-tensor cycle. It does not count arbitrary variable-
//! amplitude tensor paths, fit S-N data, predict fatigue life, or validate
//! the physical correctness of the supplied FEA/measurement stress history.

use std::f64::consts::{PI, TAU};
use std::fmt;

const MIN_SAMPLES: usize = 8;
const MAX_SAMPLES: usize = 512;
const MAX_PLANES: usize = 16_384;
const MAX_PAIR_COMPARISONS: u128 = 100_000_000;
const DIGEST_HEX_LENGTH: usize = 64;

/// Evidence class of the source stress field. The derived Findley parameter
/// is always a model estimate; this crate never promotes it to measured data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    Synthetic,
    Measured,
    ModelEstimate,
}

/// Minimal evidence reference. The digest is syntax-checked, not authenticated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRef {
    pub evidence_id: String,
    pub sha256: String,
    pub class: EvidenceClass,
}

/// Symmetric Cauchy stress tensor in megapascals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StressTensorMpa {
    pub sigma_xx: f64,
    pub sigma_yy: f64,
    pub sigma_zz: f64,
    pub tau_xy: f64,
    pub tau_yz: f64,
    pub tau_zx: f64,
}

impl StressTensorMpa {
    pub fn validate(self) -> Result<(), HcfError> {
        let values = [
            self.sigma_xx,
            self.sigma_yy,
            self.sigma_zz,
            self.tau_xy,
            self.tau_yz,
            self.tau_zx,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(HcfError::NonFiniteStress);
        }
        Ok(())
    }

    fn multiply_vector(self, vector: [f64; 3]) -> [f64; 3] {
        [
            self.sigma_xx * vector[0] + self.tau_xy * vector[1] + self.tau_zx * vector[2],
            self.tau_xy * vector[0] + self.sigma_yy * vector[1] + self.tau_yz * vector[2],
            self.tau_zx * vector[0] + self.tau_yz * vector[1] + self.sigma_zz * vector[2],
        ]
    }

    /// Rotate the stress tensor by R * sigma * R^T, where R is an orthogonal
    /// 3x3 basis rotation. This is supplied for metamorphic reference tests
    /// and coordinate-system adapters.
    pub fn rotated(self, rotation: [[f64; 3]; 3]) -> Result<Self, HcfError> {
        self.validate()?;
        validate_rotation(rotation)?;

        let matrix = [
            [self.sigma_xx, self.tau_xy, self.tau_zx],
            [self.tau_xy, self.sigma_yy, self.tau_yz],
            [self.tau_zx, self.tau_yz, self.sigma_zz],
        ];
        let mut intermediate = [[0.0; 3]; 3];
        let mut result = [[0.0; 3]; 3];

        for row in 0..3 {
            for column in 0..3 {
                for inner in 0..3 {
                    intermediate[row][column] += rotation[row][inner] * matrix[inner][column];
                }
            }
        }
        for row in 0..3 {
            for column in 0..3 {
                for inner in 0..3 {
                    result[row][column] += intermediate[row][inner] * rotation[column][inner];
                }
            }
        }

        Ok(Self {
            sigma_xx: result[0][0],
            sigma_yy: result[1][1],
            sigma_zz: result[2][2],
            tau_xy: 0.5 * (result[0][1] + result[1][0]),
            tau_yz: 0.5 * (result[1][2] + result[2][1]),
            tau_zx: 0.5 * (result[2][0] + result[0][2]),
        })
    }
}

/// One chronological sample from a single stabilized cycle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StressSample {
    /// Phase in radians in [0, 2*pi).
    pub phase_rad: f64,
    pub stress: StressTensorMpa,
}

/// Source stress history from one local material point or declared FEA point.
#[derive(Clone, Debug, PartialEq)]
pub struct StressHistory {
    pub case_id: String,
    pub material_grade_id: String,
    pub specimen_id: String,
    pub evidence: EvidenceRef,
    /// Samples must be chronologically ordered by phase.
    pub samples: Vec<StressSample>,
}

/// Deterministic angular plane-search and HCF applicability settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FindleyProfile {
    pub profile_id: &'static str,
    pub version: &'static str,
    /// Material sensitivity to maximum normal stress (dimensionless).
    /// This must be calibrated; it is never inferred by this crate.
    pub normal_stress_sensitivity: f64,
    /// Number of equal polar intervals over the normal hemisphere [0, pi/2].
    /// Must be even so the equator is represented.
    pub polar_steps: usize,
    /// Number of azimuth samples over [0, 2*pi).
    pub azimuth_steps: usize,
    /// Maximum permitted circular phase gap between adjacent samples, radians.
    pub max_phase_gap_rad: f64,
}

/// Bounded Findley result for one critical material plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CriticalPlaneResult {
    /// Undirected plane normal in the input coordinate frame.
    pub plane_normal: [f64; 3],
    /// Half the diameter of the in-plane shear-traction path, in MPa.
    pub shear_amplitude_mpa: f64,
    /// Maximum normal stress on the plane over the sampled cycle, in MPa.
    pub maximum_normal_stress_mpa: f64,
    /// Findley parameter = shear amplitude + k * maximum normal stress, MPa.
    pub findley_parameter_mpa: f64,
    /// 4*convex-hull-area/(pi*diameter^2); diagnostic only, [0,1].
    /// Zero denotes a line-like shear path; values near one are circle-like.
    pub shear_path_shape_index: f64,
}

/// Reproducibility and resolution metadata accompanying a result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SearchDiagnostics {
    pub evaluated_planes: usize,
    pub shear_point_pair_comparisons: u128,
    pub polar_step_rad: f64,
    pub azimuth_step_rad: f64,
    pub maximum_observed_phase_gap_rad: f64,
}

/// Evidence-bound result; output class remains ModelEstimate.
#[derive(Clone, Debug, PartialEq)]
pub struct FindleyReport {
    pub case_id: String,
    pub material_grade_id: String,
    pub specimen_id: String,
    pub source_evidence: EvidenceRef,
    pub output_evidence_class: EvidenceClass,
    pub method_profile_id: String,
    pub method_version: String,
    pub normal_stress_sensitivity: f64,
    pub critical_plane: CriticalPlaneResult,
    pub diagnostics: SearchDiagnostics,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HcfError {
    EmptyIdentity(&'static str),
    InvalidDigest,
    NonFiniteStress,
    NonFinitePhase,
    PhaseOutsideCycle,
    PhaseOrderNotStrict,
    TooFewSamples,
    TooManySamples,
    PhaseGapTooLarge,
    InvalidProfileIdentity,
    InvalidMaterialParameter,
    InvalidPlaneSearch,
    SearchBudgetExceeded,
    InvalidRotation,
    NoFiniteCandidate,
}

impl fmt::Display for HcfError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for HcfError {}

impl StressHistory {
    fn validate(&self, maximum_phase_gap_rad: f64) -> Result<f64, HcfError> {
        for (value, field) in [
            (self.case_id.as_str(), "case_id"),
            (self.material_grade_id.as_str(), "material_grade_id"),
            (self.specimen_id.as_str(), "specimen_id"),
            (self.evidence.evidence_id.as_str(), "evidence_id"),
        ] {
            if value.trim().is_empty() {
                return Err(HcfError::EmptyIdentity(field));
            }
        }
        validate_digest(&self.evidence.sha256)?;

        if self.samples.len() < MIN_SAMPLES {
            return Err(HcfError::TooFewSamples);
        }
        if self.samples.len() > MAX_SAMPLES {
            return Err(HcfError::TooManySamples);
        }

        let mut previous_phase = None;
        for sample in &self.samples {
            if !sample.phase_rad.is_finite() {
                return Err(HcfError::NonFinitePhase);
            }
            if !(0.0..TAU).contains(&sample.phase_rad) {
                return Err(HcfError::PhaseOutsideCycle);
            }
            if previous_phase.is_some_and(|previous| sample.phase_rad <= previous) {
                return Err(HcfError::PhaseOrderNotStrict);
            }
            sample.stress.validate()?;
            previous_phase = Some(sample.phase_rad);
        }

        let first = self
            .samples
            .first()
            .expect("sample count validated")
            .phase_rad;
        let last = self
            .samples
            .last()
            .expect("sample count validated")
            .phase_rad;
        let wrap_gap = TAU - (last - first);
        let mut maximum_gap = wrap_gap;
        for pair in self.samples.windows(2) {
            maximum_gap = maximum_gap.max(pair[1].phase_rad - pair[0].phase_rad);
        }
        if maximum_gap > maximum_phase_gap_rad {
            return Err(HcfError::PhaseGapTooLarge);
        }
        Ok(maximum_gap)
    }
}

impl FindleyProfile {
    fn validate(self) -> Result<(), HcfError> {
        if self.profile_id.trim().is_empty() || self.version.trim().is_empty() {
            return Err(HcfError::InvalidProfileIdentity);
        }
        if !self.normal_stress_sensitivity.is_finite() || self.normal_stress_sensitivity < 0.0 {
            return Err(HcfError::InvalidMaterialParameter);
        }
        if self.polar_steps < 2
            || !self.polar_steps.is_multiple_of(2)
            || self.azimuth_steps < 4
            || self.polar_steps > 90
            || self.azimuth_steps > 180
            || !self.max_phase_gap_rad.is_finite()
            || self.max_phase_gap_rad <= 0.0
            || self.max_phase_gap_rad > PI
        {
            return Err(HcfError::InvalidPlaneSearch);
        }

        let planes = (self.polar_steps + 1)
            .checked_mul(self.azimuth_steps)
            .ok_or(HcfError::InvalidPlaneSearch)?;
        if planes > MAX_PLANES {
            return Err(HcfError::InvalidPlaneSearch);
        }
        Ok(())
    }
}

/// Evaluate the Findley parameter over a deterministic grid of undirected
/// material planes on one validated, stabilized HCF stress cycle.
///
/// For each plane, the in-plane shear path is represented in an orthonormal
/// local basis. Its shear amplitude is half the maximum Euclidean distance
/// between any two sampled shear-traction points. Equivalently, it is the
/// maximum half-range over all in-plane resolved shear directions for this
/// discrete path. This explicit path-amplitude convention is a bounded
/// numerical reference; it is not a universal cycle-counting law.
pub fn evaluate_findley(
    history: &StressHistory,
    profile: FindleyProfile,
) -> Result<FindleyReport, HcfError> {
    profile.validate()?;
    let maximum_phase_gap = history.validate(profile.max_phase_gap_rad)?;

    let plane_count = (profile.polar_steps + 1) * profile.azimuth_steps;
    let sample_count = history.samples.len() as u128;
    let pairs_per_plane = sample_count
        .checked_mul(sample_count - 1)
        .and_then(|value| value.checked_div(2))
        .ok_or(HcfError::SearchBudgetExceeded)?;
    let pair_comparisons = (plane_count as u128)
        .checked_mul(pairs_per_plane)
        .ok_or(HcfError::SearchBudgetExceeded)?;
    if pair_comparisons > MAX_PAIR_COMPARISONS {
        return Err(HcfError::SearchBudgetExceeded);
    }

    let mut best: Option<CriticalPlaneResult> = None;
    let mut examined = 0usize;
    for polar_index in 0..=profile.polar_steps {
        let theta = (PI / 2.0) * (polar_index as f64) / (profile.polar_steps as f64);
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for azimuth_index in 0..profile.azimuth_steps {
            let phi = TAU * (azimuth_index as f64) / (profile.azimuth_steps as f64);
            let normal = [sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta];
            let candidate =
                evaluate_plane(&history.samples, normal, profile.normal_stress_sensitivity)?;
            examined += 1;

            let should_replace = best.is_none_or(|current| {
                candidate.findley_parameter_mpa > current.findley_parameter_mpa
            });
            if should_replace {
                best = Some(candidate);
            }
        }
    }

    let critical_plane = best.ok_or(HcfError::NoFiniteCandidate)?;
    Ok(FindleyReport {
        case_id: history.case_id.clone(),
        material_grade_id: history.material_grade_id.clone(),
        specimen_id: history.specimen_id.clone(),
        source_evidence: history.evidence.clone(),
        output_evidence_class: EvidenceClass::ModelEstimate,
        method_profile_id: profile.profile_id.to_owned(),
        method_version: profile.version.to_owned(),
        normal_stress_sensitivity: profile.normal_stress_sensitivity,
        critical_plane,
        diagnostics: SearchDiagnostics {
            evaluated_planes: examined,
            shear_point_pair_comparisons: pair_comparisons,
            polar_step_rad: (PI / 2.0) / (profile.polar_steps as f64),
            azimuth_step_rad: TAU / (profile.azimuth_steps as f64),
            maximum_observed_phase_gap_rad: maximum_phase_gap,
        },
    })
}

fn evaluate_plane(
    samples: &[StressSample],
    normal: [f64; 3],
    sensitivity: f64,
) -> Result<CriticalPlaneResult, HcfError> {
    let normal = normalize(normal).ok_or(HcfError::NoFiniteCandidate)?;
    let reference = if normal[0].abs() < 0.8 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let basis_one = normalize(cross(normal, reference)).ok_or(HcfError::NoFiniteCandidate)?;
    let basis_two = cross(normal, basis_one);

    let mut shear_points = Vec::with_capacity(samples.len());
    let mut maximum_normal = f64::NEG_INFINITY;
    for sample in samples {
        let traction = sample.stress.multiply_vector(normal);
        let normal_stress = dot(normal, traction);
        maximum_normal = maximum_normal.max(normal_stress);
        let shear = subtract(traction, scale(normal, normal_stress));
        shear_points.push([dot(shear, basis_one), dot(shear, basis_two)]);
    }

    let hull = convex_hull(shear_points);
    let diameter = hull_diameter(&hull);
    let shear_amplitude = 0.5 * diameter;
    let hull_area = polygon_area(&hull);
    let shape_index = if diameter > f64::EPSILON {
        (4.0 * hull_area / (PI * diameter * diameter)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let parameter = shear_amplitude + sensitivity * maximum_normal;
    if !parameter.is_finite() || !maximum_normal.is_finite() {
        return Err(HcfError::NoFiniteCandidate);
    }

    Ok(CriticalPlaneResult {
        plane_normal: normal,
        shear_amplitude_mpa: shear_amplitude,
        maximum_normal_stress_mpa: maximum_normal,
        findley_parameter_mpa: parameter,
        shear_path_shape_index: shape_index,
    })
}

fn convex_hull(mut points: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    points.sort_by(|left, right| {
        left[0]
            .total_cmp(&right[0])
            .then_with(|| left[1].total_cmp(&right[1]))
    });
    points.dedup_by(|left, right| left[0] == right[0] && left[1] == right[1]);
    if points.len() <= 2 {
        return points;
    }

    let mut lower = Vec::with_capacity(points.len());
    for point in &points {
        while lower.len() >= 2
            && cross_2d(lower[lower.len() - 2], lower[lower.len() - 1], *point) <= 0.0
        {
            lower.pop();
        }
        lower.push(*point);
    }

    let mut upper = Vec::with_capacity(points.len());
    for point in points.iter().rev() {
        while upper.len() >= 2
            && cross_2d(upper[upper.len() - 2], upper[upper.len() - 1], *point) <= 0.0
        {
            upper.pop();
        }
        upper.push(*point);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn hull_diameter(hull: &[[f64; 2]]) -> f64 {
    let mut maximum_squared = 0.0_f64;
    for left in 0..hull.len() {
        for right in (left + 1)..hull.len() {
            let dx = hull[left][0] - hull[right][0];
            let dy = hull[left][1] - hull[right][1];
            maximum_squared = maximum_squared.max(dx * dx + dy * dy);
        }
    }
    maximum_squared.sqrt()
}

fn polygon_area(hull: &[[f64; 2]]) -> f64 {
    if hull.len() < 3 {
        return 0.0;
    }
    let mut twice_area = 0.0;
    for index in 0..hull.len() {
        let next = (index + 1) % hull.len();
        twice_area += hull[index][0] * hull[next][1] - hull[next][0] * hull[index][1];
    }
    0.5 * twice_area.abs()
}

fn cross_2d(origin: [f64; 2], left: [f64; 2], right: [f64; 2]) -> f64 {
    (left[0] - origin[0]) * (right[1] - origin[1]) - (left[1] - origin[1]) * (right[0] - origin[0])
}

fn validate_digest(digest: &str) -> Result<(), HcfError> {
    if digest.len() != DIGEST_HEX_LENGTH || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HcfError::InvalidDigest);
    }
    Ok(())
}

fn validate_rotation(rotation: [[f64; 3]; 3]) -> Result<(), HcfError> {
    if rotation.iter().flatten().any(|value| !value.is_finite()) {
        return Err(HcfError::InvalidRotation);
    }
    let tolerance = 1.0e-10;
    for row in 0..3 {
        for column in 0..3 {
            let product = (0..3)
                .map(|index| rotation[row][index] * rotation[column][index])
                .sum::<f64>();
            let expected = if row == column { 1.0 } else { 0.0 };
            if (product - expected).abs() > tolerance {
                return Err(HcfError::InvalidRotation);
            }
        }
    }
    let determinant = rotation[0][0]
        * (rotation[1][1] * rotation[2][2] - rotation[1][2] * rotation[2][1])
        - rotation[0][1] * (rotation[1][0] * rotation[2][2] - rotation[1][2] * rotation[2][0])
        + rotation[0][2] * (rotation[1][0] * rotation[2][1] - rotation[1][1] * rotation[2][0]);
    if (determinant - 1.0).abs() > tolerance {
        return Err(HcfError::InvalidRotation);
    }
    Ok(())
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

fn normalize(vector: [f64; 3]) -> Option<[f64; 3]> {
    let magnitude = dot(vector, vector).sqrt();
    if !magnitude.is_finite() || magnitude <= f64::EPSILON {
        return None;
    }
    Some(scale(vector, 1.0 / magnitude))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_COUNT: usize = 64;
    const MAX_GAP: f64 = 0.15;

    fn evidence() -> EvidenceRef {
        EvidenceRef {
            evidence_id: "stress-field-001".into(),
            sha256: "a".repeat(64),
            class: EvidenceClass::Synthetic,
        }
    }

    fn history(case_id: &str, stress_at_phase: impl Fn(f64) -> StressTensorMpa) -> StressHistory {
        let samples = (0..SAMPLE_COUNT)
            .map(|index| {
                let phase = TAU * (index as f64) / (SAMPLE_COUNT as f64);
                StressSample {
                    phase_rad: phase,
                    stress: stress_at_phase(phase),
                }
            })
            .collect();
        StressHistory {
            case_id: case_id.into(),
            material_grade_id: "42CrMo4+QT".into(),
            specimen_id: "synthetic-specimen-001".into(),
            evidence: evidence(),
            samples,
        }
    }

    fn profile(sensitivity: f64) -> FindleyProfile {
        FindleyProfile {
            profile_id: "findley-stress-hcf",
            version: "0.1.0",
            normal_stress_sensitivity: sensitivity,
            polar_steps: 36,
            azimuth_steps: 72,
            max_phase_gap_rad: MAX_GAP,
        }
    }

    fn zero_stress() -> StressTensorMpa {
        StressTensorMpa {
            sigma_xx: 0.0,
            sigma_yy: 0.0,
            sigma_zz: 0.0,
            tau_xy: 0.0,
            tau_yz: 0.0,
            tau_zx: 0.0,
        }
    }

    #[test]
    fn uniaxial_findley_parameter_matches_closed_form_with_grid_tolerance() {
        let amplitude = 100.0;
        let sensitivity = 0.3;
        let h = history("uniaxial", |phase| StressTensorMpa {
            sigma_xx: amplitude * phase.sin(),
            ..zero_stress()
        });
        let report = evaluate_findley(&h, profile(sensitivity)).unwrap();

        let expected = amplitude * ((1.0 + sensitivity * sensitivity).sqrt() + sensitivity) / 2.0;
        assert!((report.critical_plane.findley_parameter_mpa - expected).abs() < 0.5);
        assert_eq!(report.output_evidence_class, EvidenceClass::ModelEstimate);
        assert_eq!(report.diagnostics.evaluated_planes, 37 * 72);
    }

    #[test]
    fn pure_torsion_findley_parameter_matches_closed_form_with_grid_tolerance() {
        let amplitude = 80.0;
        let sensitivity = 0.25;
        let h = history("pure-torsion", |phase| StressTensorMpa {
            tau_xy: amplitude * phase.sin(),
            ..zero_stress()
        });
        let report = evaluate_findley(&h, profile(sensitivity)).unwrap();

        let expected = amplitude * (1.0 + sensitivity * sensitivity).sqrt();
        assert!((report.critical_plane.findley_parameter_mpa - expected).abs() < 0.8);
    }

    #[test]
    fn in_phase_and_quadrature_histories_have_distinct_shear_path_shapes() {
        let axial_amplitude = 100.0;
        let torsion_amplitude = 60.0;
        let in_phase = history("in-phase", |phase| StressTensorMpa {
            sigma_xx: axial_amplitude * phase.sin(),
            tau_xy: torsion_amplitude * phase.sin(),
            ..zero_stress()
        });
        let quadrature = history("quadrature", |phase| StressTensorMpa {
            sigma_xx: axial_amplitude * phase.sin(),
            tau_xy: torsion_amplitude * (phase + PI / 2.0).sin(),
            ..zero_stress()
        });

        let in_phase_report = evaluate_findley(&in_phase, profile(0.3)).unwrap();
        let quadrature_report = evaluate_findley(&quadrature, profile(0.3)).unwrap();

        assert!(in_phase_report.critical_plane.shear_path_shape_index < 1.0e-8);
        assert!(quadrature_report.critical_plane.shear_path_shape_index > 0.05);
    }

    #[test]
    fn phase_origin_shift_preserves_the_discrete_path_parameter() {
        let original = history("phase-origin", |phase| StressTensorMpa {
            sigma_xx: 90.0 * phase.sin(),
            tau_xy: 55.0 * (phase + PI / 2.0).sin(),
            ..zero_stress()
        });
        let shift = 1.13;
        let mut shifted = original.clone();
        for sample in &mut shifted.samples {
            sample.phase_rad = (sample.phase_rad + shift) % TAU;
        }
        shifted
            .samples
            .sort_by(|a, b| a.phase_rad.total_cmp(&b.phase_rad));

        let first = evaluate_findley(&original, profile(0.28)).unwrap();
        let second = evaluate_findley(&shifted, profile(0.28)).unwrap();
        assert!(
            (first.critical_plane.findley_parameter_mpa
                - second.critical_plane.findley_parameter_mpa)
                .abs()
                < 1.0e-10
        );
    }

    #[test]
    fn orthogonal_coordinate_rotation_preserves_the_parameter() {
        let original = history("rotation-invariance", |phase| StressTensorMpa {
            sigma_xx: 100.0 * phase.sin(),
            tau_xy: 60.0 * (phase + PI / 2.0).sin(),
            sigma_yy: 12.0 * (phase - 0.3).sin(),
            ..zero_stress()
        });
        let rotation = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let mut rotated = original.clone();
        for sample in &mut rotated.samples {
            sample.stress = sample.stress.rotated(rotation).unwrap();
        }

        let first = evaluate_findley(&original, profile(0.3)).unwrap();
        let second = evaluate_findley(&rotated, profile(0.3)).unwrap();
        assert!(
            (first.critical_plane.findley_parameter_mpa
                - second.critical_plane.findley_parameter_mpa)
                .abs()
                < 1.0e-8
        );
    }

    #[test]
    fn phase_gap_above_profile_limit_is_rejected() {
        let mut h = history("bad-phase-gap", |_| zero_stress());
        h.samples[20].phase_rad += 0.2;
        h.samples
            .sort_by(|a, b| a.phase_rad.total_cmp(&b.phase_rad));
        assert_eq!(
            evaluate_findley(&h, profile(0.3)),
            Err(HcfError::PhaseGapTooLarge)
        );
    }

    #[test]
    fn non_finite_stress_is_rejected() {
        let mut h = history("bad-stress", |_| zero_stress());
        h.samples[3].stress.sigma_xx = f64::NAN;
        assert_eq!(
            evaluate_findley(&h, profile(0.3)),
            Err(HcfError::NonFiniteStress)
        );
    }

    #[test]
    fn duplicate_or_unordered_phase_is_rejected() {
        let mut h = history("bad-order", |_| zero_stress());
        h.samples[5].phase_rad = h.samples[4].phase_rad;
        assert_eq!(
            evaluate_findley(&h, profile(0.3)),
            Err(HcfError::PhaseOrderNotStrict)
        );
    }

    #[test]
    fn material_parameter_must_be_finite_and_non_negative() {
        let h = history("bad-k", |_| zero_stress());
        let mut p = profile(0.3);
        p.normal_stress_sensitivity = f64::NAN;
        assert_eq!(
            evaluate_findley(&h, p),
            Err(HcfError::InvalidMaterialParameter)
        );
    }

    #[test]
    fn malformed_source_digest_is_rejected() {
        let mut h = history("bad-digest", |_| zero_stress());
        h.evidence.sha256 = "not-a-digest".into();
        assert_eq!(
            evaluate_findley(&h, profile(0.3)),
            Err(HcfError::InvalidDigest)
        );
    }

    #[test]
    fn undersampled_cycle_is_rejected() {
        let mut h = history("undersampled", |_| zero_stress());
        h.samples.truncate(4);
        assert_eq!(
            evaluate_findley(&h, profile(0.3)),
            Err(HcfError::TooFewSamples)
        );
    }

    #[test]
    fn empty_shear_path_has_zero_amplitude_and_zero_shape_index() {
        let h = history("zero-stress", |_| zero_stress());
        let report = evaluate_findley(&h, profile(0.3)).unwrap();
        assert_eq!(report.critical_plane.shear_amplitude_mpa, 0.0);
        assert_eq!(report.critical_plane.shear_path_shape_index, 0.0);
        assert_eq!(report.critical_plane.findley_parameter_mpa, 0.0);
    }

    #[test]
    fn profile_limits_prevent_unbounded_search_work() {
        let h = history("bounded-search", |_| zero_stress());
        let mut p = profile(0.3);
        p.polar_steps = 92;
        assert_eq!(evaluate_findley(&h, p), Err(HcfError::InvalidPlaneSearch));
    }
}
