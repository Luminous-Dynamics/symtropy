//! Sheet-scoped argument-principle QNM contour-count fixtures.
//!
//! The fixture stores the complex characteristic-function values sampled on
//! a declared circular contour, on one fixed angular branch and Riemann
//! sheet. Two independent sampling resolutions must produce the same winding
//! count. Enumerated roots are reconciled against zeros minus known poles.
//!
//! This is a numerical completeness check in a bounded domain, not a proof of
//! global spectral completeness or an interval-arithmetic certificate.

use serde::{Deserialize, Serialize};

use crate::{
    AngularEigenvalueConvention, ComplexValue, FixtureProvenance, FourierConvention,
    FrequencyNormalization,
    qnm::{QnmFrequencyCutRelation, QnmRiemannSheetKind},
};

pub const QNM_CONTOUR_COUNT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComplexFrequencyContour {
    pub center_m: ComplexValue,
    pub radius_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmContourSample {
    pub omega_m: ComplexValue,
    pub characteristic_value: ComplexValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmContourResolution {
    pub resolution_id: String,
    /// Samples ordered counter-clockwise around the contour. The first point
    /// is not repeated at the end; the final-to-first segment closes the grid.
    pub samples: Vec<QnmContourSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmContourRoot {
    pub mode_lineage_id: String,
    pub omega_m: ComplexValue,
    pub multiplicity: u32,
    /// Relative residual of the independently evaluated characteristic
    /// function at the reported root.
    pub relative_characteristic_residual: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmContourPole {
    pub pole_id: String,
    pub omega_m: ComplexValue,
    pub multiplicity: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmContourAnalysisConfig {
    pub maximum_relative_radius_error: f64,
    pub maximum_frequency_angle_step_rad: f64,
    pub maximum_characteristic_phase_step_rad: f64,
    pub winding_integer_tolerance: f64,
    pub minimum_characteristic_magnitude: f64,
    pub maximum_root_relative_residual: f64,
    pub minimum_relative_boundary_margin: f64,
}

impl Default for QnmContourAnalysisConfig {
    fn default() -> Self {
        Self {
            maximum_relative_radius_error: 1.0e-8,
            maximum_frequency_angle_step_rad: core::f64::consts::FRAC_PI_8,
            maximum_characteristic_phase_step_rad: core::f64::consts::FRAC_PI_8,
            winding_integer_tolerance: 1.0e-6,
            minimum_characteristic_magnitude: 1.0e-14,
            maximum_root_relative_residual: 1.0e-10,
            minimum_relative_boundary_margin: 1.0e-8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmContourCountFixture {
    pub schema_version: u32,
    pub contour_id: String,
    pub characteristic_function_id: String,
    pub provenance: FixtureProvenance,
    pub spin_over_mass: f64,
    pub spin_weight: i32,
    pub ell: u32,
    pub m: i32,
    pub angular_eigenvalue_convention: AngularEigenvalueConvention,
    pub angular_eigenvalue_definition: Option<String>,
    pub angular_branch_id: String,
    pub riemann_sheet_kind: QnmRiemannSheetKind,
    pub riemann_sheet_id: String,
    /// A contour must sit on one declared side/chart of the frequency cut.
    /// A contour that intersects the cut must be split into sheet-specific
    /// pieces rather than treated as a single analytic loop.
    pub frequency_cut_relation: QnmFrequencyCutRelation,
    pub fourier_convention: FourierConvention,
    pub frequency_normalization: FrequencyNormalization,
    pub contour: ComplexFrequencyContour,
    pub poles_inside_contour: Vec<QnmContourPole>,
    pub coarse_resolution: QnmContourResolution,
    pub fine_resolution: QnmContourResolution,
    pub enumerated_roots: Vec<QnmContourRoot>,
    pub analysis_config: QnmContourAnalysisConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QnmContourValidationError {
    UnsupportedSchemaVersion(u32),
    EmptyField(&'static str),
    InvalidSha256,
    InvalidPrecision,
    InvalidSpin,
    InvalidModeNumbers,
    InvalidAngularConvention,
    InvalidSpectralSheet,
    FrequencyCutNotSeparated,
    InvalidContour,
    InvalidResolution(&'static str),
    DuplicateModeLineage(String),
    DuplicatePoleId(String),
    InvalidRoot(usize),
    InvalidPole(usize),
    InvalidConfiguration,
}

impl core::fmt::Display for QnmContourValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported QNM contour-count schema version {version}")
            }
            Self::EmptyField(name) => write!(f, "required contour-count field {name} is empty"),
            Self::InvalidSha256 => write!(f, "producer artifact SHA-256 must contain 64 hex characters"),
            Self::InvalidPrecision => write!(f, "contour producer precision must be at least 53 bits"),
            Self::InvalidSpin => write!(f, "spin_over_mass must be finite and satisfy |a/M| <= 1"),
            Self::InvalidModeNumbers => write!(f, "mode numbers must satisfy ell >= |s| and |m| <= ell"),
            Self::InvalidAngularConvention => {
                write!(f, "producer-defined angular eigenvalue convention requires a definition")
            }
            Self::InvalidSpectralSheet => {
                write!(f, "contour count requires an explicitly named analytic sheet and angular branch")
            }
            Self::FrequencyCutNotSeparated => {
                write!(f, "a contour count cannot be treated as one analytic loop when it lies on or crosses an unresolved cut")
            }
            Self::InvalidContour => write!(f, "complex-frequency contour center and radius are invalid"),
            Self::InvalidResolution(name) => {
                write!(f, "{name} contour resolution requires at least 16 finite samples")
            }
            Self::DuplicateModeLineage(id) => {
                write!(f, "mode lineage {id} occurs more than once in the enumerated root list")
            }
            Self::DuplicatePoleId(id) => write!(f, "pole ID {id} occurs more than once"),
            Self::InvalidRoot(index) => {
                write!(f, "enumerated QNM root {index} has invalid frequency, multiplicity, or residual")
            }
            Self::InvalidPole(index) => {
                write!(f, "known characteristic-function pole {index} is invalid")
            }
            Self::InvalidConfiguration => {
                write!(f, "contour-analysis tolerances are invalid")
            }
        }
    }
}

impl std::error::Error for QnmContourValidationError {}

impl QnmContourCountFixture {
    pub fn validate(&self) -> Result<(), QnmContourValidationError> {
        if self.schema_version != QNM_CONTOUR_COUNT_SCHEMA_VERSION {
            return Err(QnmContourValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        for (value, name) in [
            (self.contour_id.as_str(), "contour_id"),
            (
                self.characteristic_function_id.as_str(),
                "characteristic_function_id",
            ),
            (self.provenance.engine.as_str(), "provenance.engine"),
            (
                self.provenance.engine_version.as_str(),
                "provenance.engine_version",
            ),
            (
                self.provenance.generated_at_utc.as_str(),
                "provenance.generated_at_utc",
            ),
            (self.angular_branch_id.as_str(), "angular_branch_id"),
            (self.riemann_sheet_id.as_str(), "riemann_sheet_id"),
            (
                self.coarse_resolution.resolution_id.as_str(),
                "coarse_resolution.resolution_id",
            ),
            (
                self.fine_resolution.resolution_id.as_str(),
                "fine_resolution.resolution_id",
            ),
        ] {
            if value.trim().is_empty() {
                return Err(QnmContourValidationError::EmptyField(name));
            }
        }
        if self.provenance.precision_bits < 53 {
            return Err(QnmContourValidationError::InvalidPrecision);
        }
        if let Some(digest) = &self.provenance.producer_artifact_sha256
            && (digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(QnmContourValidationError::InvalidSha256);
        }
        if !self.spin_over_mass.is_finite() || self.spin_over_mass.abs() > 1.0 {
            return Err(QnmContourValidationError::InvalidSpin);
        }
        if self.ell < self.spin_weight.unsigned_abs() || self.m.unsigned_abs() > self.ell {
            return Err(QnmContourValidationError::InvalidModeNumbers);
        }
        if self.angular_eigenvalue_convention == AngularEigenvalueConvention::Other
            && self
                .angular_eigenvalue_definition
                .as_deref()
                .is_none_or(str::is_empty)
        {
            return Err(QnmContourValidationError::InvalidAngularConvention);
        }
        if self.angular_branch_id.trim().is_empty()
            || self.riemann_sheet_id.trim().is_empty()
            || self.riemann_sheet_kind == QnmRiemannSheetKind::Unresolved
            || self.riemann_sheet_id == "unresolved"
        {
            return Err(QnmContourValidationError::InvalidSpectralSheet);
        }
        if matches!(
            self.frequency_cut_relation,
            QnmFrequencyCutRelation::OnCut
                | QnmFrequencyCutRelation::Unresolved
                | QnmFrequencyCutRelation::NotApplicable
        ) {
            return Err(QnmContourValidationError::FrequencyCutNotSeparated);
        }
        if !self.contour.center_m.is_finite()
            || !self.contour.radius_m.is_finite()
            || self.contour.radius_m <= 0.0
        {
            return Err(QnmContourValidationError::InvalidContour);
        }
        if self.coarse_resolution.samples.len() < 16 {
            return Err(QnmContourValidationError::InvalidResolution(
                "coarse_resolution",
            ));
        }
        if self.fine_resolution.samples.len() < 16
            || self.fine_resolution.samples.len()
                < self.coarse_resolution.samples.len().saturating_mul(2)
        {
            return Err(QnmContourValidationError::InvalidResolution(
                "fine_resolution",
            ));
        }
        if self
            .coarse_resolution
            .samples
            .iter()
            .chain(&self.fine_resolution.samples)
            .any(|sample| {
                !sample.omega_m.is_finite()
                    || !sample.characteristic_value.is_finite()
            })
        {
            return Err(QnmContourValidationError::InvalidResolution(
                "non_finite_contour_sample",
            ));
        }

        let mut lineages = std::collections::BTreeSet::new();
        for (index, root) in self.enumerated_roots.iter().enumerate() {
            if root.mode_lineage_id.trim().is_empty()
                || !lineages.insert(root.mode_lineage_id.as_str())
            {
                return Err(QnmContourValidationError::DuplicateModeLineage(
                    root.mode_lineage_id.clone(),
                ));
            }
            if !root.omega_m.is_finite()
                || root.multiplicity == 0
                || !root.relative_characteristic_residual.is_finite()
                || root.relative_characteristic_residual < 0.0
            {
                return Err(QnmContourValidationError::InvalidRoot(index));
            }
        }

        let mut poles = std::collections::BTreeSet::new();
        for (index, pole) in self.poles_inside_contour.iter().enumerate() {
            if pole.pole_id.trim().is_empty() || !poles.insert(pole.pole_id.as_str()) {
                return Err(QnmContourValidationError::DuplicatePoleId(
                    pole.pole_id.clone(),
                ));
            }
            if !pole.omega_m.is_finite() || pole.multiplicity == 0 {
                return Err(QnmContourValidationError::InvalidPole(index));
            }
        }

        let config = self.analysis_config;
        if !config.maximum_relative_radius_error.is_finite()
            || config.maximum_relative_radius_error <= 0.0
            || !config.maximum_frequency_angle_step_rad.is_finite()
            || config.maximum_frequency_angle_step_rad <= 0.0
            || config.maximum_frequency_angle_step_rad >= core::f64::consts::PI
            || !config.maximum_characteristic_phase_step_rad.is_finite()
            || config.maximum_characteristic_phase_step_rad <= 0.0
            || config.maximum_characteristic_phase_step_rad >= core::f64::consts::PI
            || !config.winding_integer_tolerance.is_finite()
            || config.winding_integer_tolerance <= 0.0
            || config.winding_integer_tolerance >= 0.5
            || !config.minimum_characteristic_magnitude.is_finite()
            || config.minimum_characteristic_magnitude <= 0.0
            || !config.maximum_root_relative_residual.is_finite()
            || config.maximum_root_relative_residual <= 0.0
            || !config.minimum_relative_boundary_margin.is_finite()
            || config.minimum_relative_boundary_margin <= 0.0
            || config.minimum_relative_boundary_margin >= 1.0
        {
            return Err(QnmContourValidationError::InvalidConfiguration);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmContourCountClassification {
    CompleteWithinDomain,
    IncompleteOrInconsistent,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmContourResolutionAnalysis {
    pub resolution_id: String,
    pub winding_number: Option<i32>,
    pub maximum_frequency_angle_step_rad: Option<f64>,
    pub maximum_characteristic_phase_step_rad: Option<f64>,
    pub minimum_characteristic_magnitude: Option<f64>,
    pub phase_closure_error_rad: Option<f64>,
    pub unresolved_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmContourCountAnalysis {
    pub classification: QnmContourCountClassification,
    pub coarse: QnmContourResolutionAnalysis,
    pub fine: QnmContourResolutionAnalysis,
    pub poles_inside_count: u32,
    pub expected_zero_count: Option<u32>,
    pub enumerated_zero_multiplicity: u32,
    pub count_matches: bool,
    pub claim_scope: String,
}

fn complex_norm(value: ComplexValue) -> f64 {
    value.re.hypot(value.im)
}

fn phase_difference(previous: ComplexValue, next: ComplexValue) -> f64 {
    let dot = previous.re * next.re + previous.im * next.im;
    let cross = next.im * previous.re - next.re * previous.im;
    cross.atan2(dot)
}

fn analyze_resolution(
    resolution: &QnmContourResolution,
    contour: ComplexFrequencyContour,
    config: QnmContourAnalysisConfig,
) -> QnmContourResolutionAnalysis {
    let unresolved = |reason: &str,
                      frequency_step: Option<f64>,
                      phase_step: Option<f64>,
                      minimum_magnitude: Option<f64>,
                      closure: Option<f64>| {
        QnmContourResolutionAnalysis {
            resolution_id: resolution.resolution_id.clone(),
            winding_number: None,
            maximum_frequency_angle_step_rad: frequency_step,
            maximum_characteristic_phase_step_rad: phase_step,
            minimum_characteristic_magnitude: minimum_magnitude,
            phase_closure_error_rad: closure,
            unresolved_reason: Some(reason.to_owned()),
        }
    };

    let mut angles = Vec::with_capacity(resolution.samples.len());
    let mut minimum_magnitude = f64::INFINITY;
    for sample in &resolution.samples {
        let displacement = ComplexValue {
            re: sample.omega_m.re - contour.center_m.re,
            im: sample.omega_m.im - contour.center_m.im,
        };
        let radius = complex_norm(displacement);
        if !radius.is_finite()
            || (radius - contour.radius_m).abs()
                > config.maximum_relative_radius_error * contour.radius_m
        {
            return unresolved(
                "frequency sample is outside the declared circular contour tolerance",
                None,
                None,
                None,
                None,
            );
        }
        angles.push(displacement.im.atan2(displacement.re));
        let magnitude = complex_norm(sample.characteristic_value);
        minimum_magnitude = minimum_magnitude.min(magnitude);
    }

    if minimum_magnitude <= config.minimum_characteristic_magnitude {
        return unresolved(
            "characteristic function approaches zero on the sampled contour",
            None,
            None,
            Some(minimum_magnitude),
            None,
        );
    }

    let mut total_frequency_angle = 0.0;
    let mut maximum_frequency_step: f64 = 0.0;
    let mut maximum_phase_step: f64 = 0.0;
    let mut total_phase = 0.0;
    for index in 0..resolution.samples.len() {
        let next_index = (index + 1) % resolution.samples.len();
        let frequency_step = (angles[next_index] - angles[index]).rem_euclid(core::f64::consts::TAU);
        if frequency_step <= f64::EPSILON {
            return unresolved(
                "frequency contour contains a duplicate or reversed sample",
                Some(maximum_frequency_step),
                None,
                Some(minimum_magnitude),
                None,
            );
        }
        total_frequency_angle += frequency_step;
        maximum_frequency_step = maximum_frequency_step.max(frequency_step);

        let phase_step = phase_difference(
            resolution.samples[index].characteristic_value,
            resolution.samples[next_index].characteristic_value,
        );
        if !phase_step.is_finite() {
            return unresolved(
                "characteristic phase increment is non-finite",
                Some(maximum_frequency_step),
                None,
                Some(minimum_magnitude),
                None,
            );
        }
        total_phase += phase_step;
        maximum_phase_step = maximum_phase_step.max(phase_step.abs());
    }

    let frequency_closure_error = (total_frequency_angle - core::f64::consts::TAU).abs();
    if frequency_closure_error > 1.0e-8 {
        return unresolved(
            "frequency samples do not make exactly one counter-clockwise contour",
            Some(maximum_frequency_step),
            Some(maximum_phase_step),
            Some(minimum_magnitude),
            Some(frequency_closure_error),
        );
    }
    if maximum_frequency_step > config.maximum_frequency_angle_step_rad {
        return unresolved(
            "frequency contour sampling is too coarse",
            Some(maximum_frequency_step),
            Some(maximum_phase_step),
            Some(minimum_magnitude),
            Some(frequency_closure_error),
        );
    }
    if maximum_phase_step > config.maximum_characteristic_phase_step_rad {
        return unresolved(
            "characteristic-function phase sampling is too coarse to trust the winding",
            Some(maximum_frequency_step),
            Some(maximum_phase_step),
            Some(minimum_magnitude),
            Some(frequency_closure_error),
        );
    }

    let winding_float = total_phase / core::f64::consts::TAU;
    let rounded = winding_float.round();
    let phase_closure_error = (winding_float - rounded).abs() * core::f64::consts::TAU;
    if !rounded.is_finite()
        || rounded < f64::from(i32::MIN)
        || rounded > f64::from(i32::MAX)
        || (winding_float - rounded).abs() > config.winding_integer_tolerance
    {
        return unresolved(
            "characteristic-function phase does not close at an integer winding number",
            Some(maximum_frequency_step),
            Some(maximum_phase_step),
            Some(minimum_magnitude),
            Some(phase_closure_error),
        );
    }

    QnmContourResolutionAnalysis {
        resolution_id: resolution.resolution_id.clone(),
        winding_number: Some(rounded as i32),
        maximum_frequency_angle_step_rad: Some(maximum_frequency_step),
        maximum_characteristic_phase_step_rad: Some(maximum_phase_step),
        minimum_characteristic_magnitude: Some(minimum_magnitude),
        phase_closure_error_rad: Some(phase_closure_error),
        unresolved_reason: None,
    }
}

/// Recompute the argument-principle count at two independently sampled
/// resolutions and reconcile zeros minus known poles with the explicit root
/// list. Agreement establishes completeness only for the declared circular
/// domain, angular branch and Riemann sheet. It is not a global or interval
/// proof; all sampled phase increments must remain below the declared limits.
pub fn analyze_qnm_contour_count(
    fixture: &QnmContourCountFixture,
) -> Result<QnmContourCountAnalysis, QnmContourValidationError> {
    fixture.validate()?;
    let config = fixture.analysis_config;
    let coarse = analyze_resolution(&fixture.coarse_resolution, fixture.contour, config);
    let fine = analyze_resolution(&fixture.fine_resolution, fixture.contour, config);
    let poles_inside_count = fixture
        .poles_inside_contour
        .iter()
        .map(|pole| pole.multiplicity)
        .sum::<u32>();
    let enumerated_zero_multiplicity = fixture
        .enumerated_roots
        .iter()
        .map(|root| root.multiplicity)
        .sum::<u32>();

    let unresolved = |coarse: QnmContourResolutionAnalysis,
                      fine: QnmContourResolutionAnalysis,
                      reason: &str| {
        QnmContourCountAnalysis {
            classification: QnmContourCountClassification::Unresolved,
            coarse,
            fine,
            poles_inside_count,
            expected_zero_count: None,
            enumerated_zero_multiplicity,
            count_matches: false,
            claim_scope: reason.to_owned(),
        }
    };

    let (Some(coarse_winding), Some(fine_winding)) =
        (coarse.winding_number, fine.winding_number)
    else {
        return Ok(unresolved(
            coarse,
            fine,
            "one or both contour resolutions did not establish a reliable winding number",
        ));
    };
    if coarse_winding != fine_winding {
        return Ok(unresolved(
            coarse,
            fine,
            "coarse and fine contour grids disagree on the winding number",
        ));
    }

    let expected_zeros_i64 = i64::from(fine_winding) + i64::from(poles_inside_count);
    if expected_zeros_i64 < 0 || expected_zeros_i64 > i64::from(u32::MAX) {
        return Ok(unresolved(
            coarse,
            fine,
            "winding plus known pole count is outside the supported nonnegative zero-count range",
        ));
    }
    let expected_zero_count = expected_zeros_i64 as u32;

    for root in &fixture.enumerated_roots {
        let distance = complex_norm(ComplexValue {
            re: root.omega_m.re - fixture.contour.center_m.re,
            im: root.omega_m.im - fixture.contour.center_m.im,
        });
        let margin = fixture.contour.radius_m - distance;
        if margin <= config.minimum_relative_boundary_margin * fixture.contour.radius_m {
            return Ok(unresolved(
                coarse,
                fine,
                "an enumerated root lies outside or too close to the contour boundary",
            ));
        }
        if root.relative_characteristic_residual > config.maximum_root_relative_residual {
            return Ok(unresolved(
                coarse,
                fine,
                "an enumerated root exceeds the configured characteristic residual",
            ));
        }
    }
    for pole in &fixture.poles_inside_contour {
        let distance = complex_norm(ComplexValue {
            re: pole.omega_m.re - fixture.contour.center_m.re,
            im: pole.omega_m.im - fixture.contour.center_m.im,
        });
        let margin = fixture.contour.radius_m - distance;
        if margin <= config.minimum_relative_boundary_margin * fixture.contour.radius_m {
            return Ok(unresolved(
                coarse,
                fine,
                "a known characteristic pole lies outside or too close to the contour boundary",
            ));
        }
    }

    let count_matches = expected_zero_count == enumerated_zero_multiplicity;
    Ok(QnmContourCountAnalysis {
        classification: if count_matches {
            QnmContourCountClassification::CompleteWithinDomain
        } else {
            QnmContourCountClassification::IncompleteOrInconsistent
        },
        coarse,
        fine,
        poles_inside_count,
        expected_zero_count: Some(expected_zero_count),
        enumerated_zero_multiplicity,
        count_matches,
        claim_scope: "bounded circular frequency domain on the recorded angular branch and Riemann sheet only; not a global completeness or interval-arithmetic proof".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn multiply(left: ComplexValue, right: ComplexValue) -> ComplexValue {
        ComplexValue {
            re: left.re * right.re - left.im * right.im,
            im: left.re * right.im + left.im * right.re,
        }
    }

    fn subtract(left: ComplexValue, right: ComplexValue) -> ComplexValue {
        ComplexValue {
            re: left.re - right.re,
            im: left.im - right.im,
        }
    }

    fn divide(left: ComplexValue, right: ComplexValue) -> ComplexValue {
        let denominator = right.re * right.re + right.im * right.im;
        ComplexValue {
            re: (left.re * right.re + left.im * right.im) / denominator,
            im: (left.im * right.re - left.re * right.im) / denominator,
        }
    }

    fn circle_samples(
        center: ComplexValue,
        radius: f64,
        samples: usize,
        zero: ComplexValue,
        pole: Option<ComplexValue>,
    ) -> Vec<QnmContourSample> {
        (0..samples)
            .map(|index| {
                let angle = core::f64::consts::TAU * index as f64 / samples as f64;
                let omega = ComplexValue {
                    re: center.re + radius * angle.cos(),
                    im: center.im + radius * angle.sin(),
                };
                let numerator = subtract(omega, zero);
                let characteristic_value = if let Some(pole) = pole {
                    divide(numerator, subtract(omega, pole))
                } else {
                    numerator
                };
                QnmContourSample {
                    omega_m: omega,
                    characteristic_value,
                }
            })
            .collect()
    }

    fn fixture(pole: Option<ComplexValue>, enumerate_root: bool) -> QnmContourCountFixture {
        let center = ComplexValue { re: 0.0, im: 0.0 };
        let zero = ComplexValue { re: 0.2, im: 0.1 };
        let radius = 1.0;
        QnmContourCountFixture {
            schema_version: QNM_CONTOUR_COUNT_SCHEMA_VERSION,
            contour_id: "unit-circle-a07-sheet-conventional".to_owned(),
            characteristic_function_id: "ingoing-incidence-amplitude".to_owned(),
            provenance: FixtureProvenance {
                engine: "synthetic-argument-principle-test".to_owned(),
                engine_version: "0.1.0".to_owned(),
                source_revision: Some("test-revision".to_owned()),
                producer_artifact_sha256: None,
                generated_at_utc: "2026-11-09T00:00:00Z".to_owned(),
                precision_bits: 128,
            },
            spin_over_mass: 0.7,
            spin_weight: -2,
            ell: 2,
            m: 2,
            angular_eigenvalue_convention: AngularEigenvalueConvention::SpinWeightedSpheroidalA,
            angular_eigenvalue_definition: None,
            angular_branch_id: "s-2-l2-m2-angular-branch".to_owned(),
            riemann_sheet_kind: QnmRiemannSheetKind::Conventional,
            riemann_sheet_id: "conventional-lower-half-plane".to_owned(),
            frequency_cut_relation: QnmFrequencyCutRelation::OffCut,
            fourier_convention: FourierConvention::EMinusIOmegaT,
            frequency_normalization: FrequencyNormalization::DimensionlessMOmega,
            contour: ComplexFrequencyContour {
                center_m: center,
                radius_m: radius,
            },
            poles_inside_contour: pole
                .map(|value| {
                    vec![QnmContourPole {
                        pole_id: "pole-0".to_owned(),
                        omega_m: value,
                        multiplicity: 1,
                    }]
                })
                .unwrap_or_default(),
            coarse_resolution: QnmContourResolution {
                resolution_id: "coarse-32".to_owned(),
                samples: circle_samples(center, radius, 32, zero, pole),
            },
            fine_resolution: QnmContourResolution {
                resolution_id: "fine-64".to_owned(),
                samples: circle_samples(center, radius, 64, zero, pole),
            },
            enumerated_roots: if enumerate_root {
                vec![QnmContourRoot {
                    mode_lineage_id: "mode-lineage-zero-0".to_owned(),
                    omega_m: zero,
                    multiplicity: 1,
                    relative_characteristic_residual: 1.0e-15,
                }]
            } else {
                Vec::new()
            },
            analysis_config: QnmContourAnalysisConfig::default(),
        }
    }

    #[test]
    fn reconciles_two_resolution_winding_with_enumerated_zero_count() {
        let result = analyze_qnm_contour_count(&fixture(None, true)).unwrap();
        assert_eq!(
            result.classification,
            QnmContourCountClassification::CompleteWithinDomain
        );
        assert_eq!(result.coarse.winding_number, Some(1));
        assert_eq!(result.fine.winding_number, Some(1));
        assert_eq!(result.expected_zero_count, Some(1));
        assert!(result.count_matches);
    }

    #[test]
    fn corrects_argument_principle_count_for_known_characteristic_poles() {
        let pole = Some(ComplexValue { re: -0.2, im: -0.1 });
        let result = analyze_qnm_contour_count(&fixture(pole, true)).unwrap();
        assert_eq!(result.coarse.winding_number, Some(0));
        assert_eq!(result.fine.winding_number, Some(0));
        assert_eq!(result.poles_inside_count, 1);
        assert_eq!(result.expected_zero_count, Some(1));
        assert_eq!(
            result.classification,
            QnmContourCountClassification::CompleteWithinDomain
        );
    }

    #[test]
    fn detects_incomplete_root_enumeration() {
        let result = analyze_qnm_contour_count(&fixture(None, false)).unwrap();
        assert_eq!(
            result.classification,
            QnmContourCountClassification::IncompleteOrInconsistent
        );
        assert!(!result.count_matches);
    }

    #[test]
    fn rejects_contours_on_unresolved_frequency_cuts() {
        let mut value = fixture(None, true);
        value.frequency_cut_relation = QnmFrequencyCutRelation::OnCut;
        assert_eq!(
            value.validate(),
            Err(QnmContourValidationError::FrequencyCutNotSeparated)
        );
    }
}
