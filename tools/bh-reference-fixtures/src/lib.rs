//! Convention-safe reference fixtures for independent black-hole perturbation engines.
//!
//! This crate defines a strict exchange format for externally generated
//! Kerr/Teukolsky reference data. It does not solve the Teukolsky equation
//! or compare numerical solutions. Its first responsibility is to prevent
//! convention mismatches from being mislabeled as numerical failures.

use core::fmt;
use serde::{Deserialize, Serialize};

pub mod qnm;

pub const FIXTURE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FourierConvention {
    EMinusIOmegaT,
    EPlusIOmegaT,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadialCoordinate {
    BoyerLindquistROverM,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrequencyNormalization {
    DimensionlessMOmega,
}

/// Complex values use explicit named components, avoiding ambiguous array
/// ordering or an implicit real/imaginary versus magnitude/phase convention.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComplexValue {
    pub re: f64,
    pub im: f64,
}

impl ComplexValue {
    pub fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureProvenance {
    pub engine: String,
    pub engine_version: String,
    pub source_revision: Option<String>,
    /// SHA-256 of the producer script, notebook, or executable artifact.
    pub producer_artifact_sha256: Option<String>,
    pub generated_at_utc: String,
    /// Binary precision used for the calculation; 53 is ordinary f64.
    pub precision_bits: u16,
}

/// Problem identity: mass is in kilograms; spin and frequency are
/// dimensionless. The angular branch ID records the continuation path/label
/// used by the external engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KerrProblem {
    pub mass_kg: f64,
    pub spin_over_mass: f64,
    pub spin_weight: i32,
    pub ell: u32,
    pub m: i32,
    pub omega_m: ComplexValue,
    pub angular_eigenvalue: Option<ComplexValue>,
    pub angular_branch_id: Option<String>,
    pub source_id: Option<String>,
}

/// Producer-authored definitions are required because homogeneous radial
/// basis amplitudes can differ by normalization conventions across engines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryNormalization {
    pub in_basis: String,
    pub up_basis: String,
    pub radial_function_definition: String,
    pub derivative_definition: String,
}

/// Derivatives are with respect to r/M, not physical radius in metres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadialSample {
    pub r_over_m: f64,
    pub in_value: Option<ComplexValue>,
    pub in_derivative: Option<ComplexValue>,
    pub up_value: Option<ComplexValue>,
    pub up_derivative: Option<ComplexValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceDiagnostics {
    pub wronskian: Option<ComplexValue>,
    pub incident_amplitude: Option<ComplexValue>,
    pub reflection_amplitude: Option<ComplexValue>,
    pub transmission_amplitude: Option<ComplexValue>,
    pub phase_shift: Option<ComplexValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceData {
    pub radial_samples: Vec<RadialSample>,
    pub diagnostics: Option<ReferenceDiagnostics>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceFixture {
    pub schema_version: u32,
    pub fixture_id: String,
    pub provenance: FixtureProvenance,
    pub problem: KerrProblem,
    pub fourier_convention: FourierConvention,
    pub radial_coordinate: RadialCoordinate,
    pub frequency_normalization: FrequencyNormalization,
    pub normalization: BoundaryNormalization,
    pub data: ReferenceData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixtureValidationError {
    UnsupportedSchemaVersion(u32),
    EmptyField(&'static str),
    InvalidSha256(&'static str),
    InvalidPrecision,
    InvalidMass,
    InvalidSpin,
    InvalidModeNumbers,
    NonFiniteFrequency,
    AngularBranchMetadataIncomplete,
    NonFiniteAngularEigenvalue,
    NoRadialSamples,
    InvalidRadialSample(usize),
    RadialSamplesNotStrictlyIncreasing(usize),
    NonFiniteDiagnostic(&'static str),
}

impl fmt::Display for FixtureValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported fixture schema version {version}")
            }
            Self::EmptyField(name) => write!(f, "required field {name} is empty"),
            Self::InvalidSha256(name) => write!(f, "{name} must be 64 hexadecimal characters"),
            Self::InvalidPrecision => write!(f, "precision_bits must be at least 53"),
            Self::InvalidMass => write!(f, "mass_kg must be finite and positive"),
            Self::InvalidSpin => write!(f, "spin_over_mass must be finite and satisfy |a/M| < 1"),
            Self::InvalidModeNumbers => {
                write!(f, "mode numbers must satisfy ell >= |s| and |m| <= ell")
            }
            Self::NonFiniteFrequency => write!(f, "dimensionless frequency must be finite"),
            Self::AngularBranchMetadataIncomplete => {
                write!(
                    f,
                    "angular eigenvalue and branch ID must be supplied together"
                )
            }
            Self::NonFiniteAngularEigenvalue => write!(f, "angular eigenvalue must be finite"),
            Self::NoRadialSamples => write!(f, "at least one radial sample is required"),
            Self::InvalidRadialSample(index) => {
                write!(
                    f,
                    "radial sample {index} has invalid radius or no solution data"
                )
            }
            Self::RadialSamplesNotStrictlyIncreasing(index) => write!(
                f,
                "radial sample {index} is not strictly outside the preceding sample"
            ),
            Self::NonFiniteDiagnostic(name) => write!(f, "diagnostic {name} is non-finite"),
        }
    }
}

impl std::error::Error for FixtureValidationError {}

impl ReferenceFixture {
    /// Validate structure and physical-domain invariants before cross-engine
    /// comparison. This does not certify the producer's numerical accuracy.
    pub fn validate(&self) -> Result<(), FixtureValidationError> {
        if self.schema_version != FIXTURE_SCHEMA_VERSION {
            return Err(FixtureValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }

        require_nonempty(&self.fixture_id, "fixture_id")?;
        require_nonempty(&self.provenance.engine, "provenance.engine")?;
        require_nonempty(&self.provenance.engine_version, "provenance.engine_version")?;
        require_nonempty(
            &self.provenance.generated_at_utc,
            "provenance.generated_at_utc",
        )?;
        if self.provenance.precision_bits < 53 {
            return Err(FixtureValidationError::InvalidPrecision);
        }
        if let Some(digest) = &self.provenance.producer_artifact_sha256 {
            validate_sha256(digest, "producer_artifact_sha256")?;
        }

        if !self.problem.mass_kg.is_finite() || self.problem.mass_kg <= 0.0 {
            return Err(FixtureValidationError::InvalidMass);
        }
        if !self.problem.spin_over_mass.is_finite() || self.problem.spin_over_mass.abs() >= 1.0 {
            return Err(FixtureValidationError::InvalidSpin);
        }
        if self.problem.ell < self.problem.spin_weight.unsigned_abs()
            || self.problem.m.unsigned_abs() > self.problem.ell
        {
            return Err(FixtureValidationError::InvalidModeNumbers);
        }
        if !self.problem.omega_m.is_finite() {
            return Err(FixtureValidationError::NonFiniteFrequency);
        }

        match (
            self.problem.angular_eigenvalue,
            self.problem.angular_branch_id.as_deref(),
        ) {
            (Some(value), Some(branch)) => {
                if !value.is_finite() {
                    return Err(FixtureValidationError::NonFiniteAngularEigenvalue);
                }
                require_nonempty(branch, "angular_branch_id")?;
            }
            (None, None) => {}
            _ => return Err(FixtureValidationError::AngularBranchMetadataIncomplete),
        }

        require_nonempty(&self.normalization.in_basis, "normalization.in_basis")?;
        require_nonempty(&self.normalization.up_basis, "normalization.up_basis")?;
        require_nonempty(
            &self.normalization.radial_function_definition,
            "normalization.radial_function_definition",
        )?;
        require_nonempty(
            &self.normalization.derivative_definition,
            "normalization.derivative_definition",
        )?;

        if self.data.radial_samples.is_empty() {
            return Err(FixtureValidationError::NoRadialSamples);
        }

        let outer_horizon = 1.0 + (1.0 - self.problem.spin_over_mass.powi(2)).sqrt();
        let mut previous_radius = f64::NEG_INFINITY;
        for (index, sample) in self.data.radial_samples.iter().enumerate() {
            if !sample.r_over_m.is_finite()
                || sample.r_over_m <= outer_horizon
                || !sample_has_values(sample)
                || !sample_values_are_finite(sample)
            {
                return Err(FixtureValidationError::InvalidRadialSample(index));
            }
            if sample.r_over_m <= previous_radius {
                return Err(FixtureValidationError::RadialSamplesNotStrictlyIncreasing(
                    index,
                ));
            }
            previous_radius = sample.r_over_m;
        }

        if let Some(diagnostics) = &self.data.diagnostics {
            validate_optional_complex(diagnostics.wronskian, "wronskian")?;
            validate_optional_complex(diagnostics.incident_amplitude, "incident_amplitude")?;
            validate_optional_complex(diagnostics.reflection_amplitude, "reflection_amplitude")?;
            validate_optional_complex(
                diagnostics.transmission_amplitude,
                "transmission_amplitude",
            )?;
            validate_optional_complex(diagnostics.phase_shift, "phase_shift")?;
        }

        Ok(())
    }
}

/// Convention/identity preflight. A Matched result means only that the
/// fixtures describe the same mathematical problem, not that their numbers
/// agree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityStatus {
    Matched,
    Unmatched,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureCompatibility {
    pub status: CompatibilityStatus,
    pub differing_fields: Vec<&'static str>,
}

pub fn assess_compatibility(
    left: &ReferenceFixture,
    right: &ReferenceFixture,
) -> FixtureCompatibility {
    let mut differing_fields = Vec::new();

    if left.problem.spin_weight != right.problem.spin_weight {
        differing_fields.push("spin_weight");
    }
    if left.problem.ell != right.problem.ell {
        differing_fields.push("ell");
    }
    if left.problem.m != right.problem.m {
        differing_fields.push("m");
    }
    if left.problem.source_id != right.problem.source_id {
        differing_fields.push("source_id");
    }
    if !close(left.problem.mass_kg, right.problem.mass_kg, 1.0e-13) {
        differing_fields.push("mass_kg");
    }
    if !close(
        left.problem.spin_over_mass,
        right.problem.spin_over_mass,
        1.0e-13,
    ) {
        differing_fields.push("spin_over_mass");
    }
    if !complex_close(left.problem.omega_m, right.problem.omega_m, 1.0e-13) {
        differing_fields.push("omega_m");
    }
    if left.problem.angular_eigenvalue != right.problem.angular_eigenvalue {
        differing_fields.push("angular_eigenvalue");
    }
    if left.problem.angular_branch_id != right.problem.angular_branch_id {
        differing_fields.push("angular_branch_id");
    }
    if left.fourier_convention != right.fourier_convention {
        differing_fields.push("fourier_convention");
    }
    if left.radial_coordinate != right.radial_coordinate {
        differing_fields.push("radial_coordinate");
    }
    if left.frequency_normalization != right.frequency_normalization {
        differing_fields.push("frequency_normalization");
    }
    if left.normalization != right.normalization {
        differing_fields.push("boundary_normalization");
    }

    let status = if differing_fields.is_empty() {
        CompatibilityStatus::Matched
    } else {
        CompatibilityStatus::Unmatched
    };
    FixtureCompatibility {
        status,
        differing_fields,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadialComparisonConfig {
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
}

impl RadialComparisonConfig {
    fn validate(self) -> Result<(), RadialComparisonError> {
        if !self.absolute_tolerance.is_finite()
            || !self.relative_tolerance.is_finite()
            || self.absolute_tolerance < 0.0
            || self.relative_tolerance < 0.0
            || (self.absolute_tolerance == 0.0 && self.relative_tolerance == 0.0)
        {
            return Err(RadialComparisonError::InvalidTolerance);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadialComparisonStatus {
    Unmatched,
    Unresolved,
    NumericallyAgreeing,
    NumericallyDisagreeing,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct RadialChannelMetrics {
    pub compared_values: usize,
    pub maximum_absolute_error: f64,
    /// Maximum error divided by (absolute tolerance + relative tolerance
    /// times the larger magnitude at the compared point).
    pub maximum_scaled_error: f64,
    pub values_outside_tolerance: usize,
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadialComparisonResult {
    pub status: RadialComparisonStatus,
    pub differing_fields: Vec<String>,
    pub unresolved_reason: Option<String>,
    pub in_value: RadialChannelMetrics,
    pub in_derivative: RadialChannelMetrics,
    pub up_value: RadialChannelMetrics,
    pub up_derivative: RadialChannelMetrics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadialComparisonError {
    InvalidTolerance,
    InvalidLeftFixture(FixtureValidationError),
    InvalidRightFixture(FixtureValidationError),
}

impl fmt::Display for RadialComparisonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTolerance => write!(f, "comparison tolerances must be finite, non-negative, and not both zero"),
            Self::InvalidLeftFixture(error) => write!(f, "left radial fixture is invalid: {error}"),
            Self::InvalidRightFixture(error) => write!(f, "right radial fixture is invalid: {error}"),
        }
    }
}

impl std::error::Error for RadialComparisonError {}

/// Compare two radial fixtures only when their declared mathematical problem,
/// sample radii, and per-channel coverage match exactly. No interpolation,
/// normalization fitting, or silent sample dropping is performed.
pub fn compare_radial_fixtures(
    left: &ReferenceFixture,
    right: &ReferenceFixture,
    config: RadialComparisonConfig,
) -> Result<RadialComparisonResult, RadialComparisonError> {
    config.validate()?;
    left.validate()
        .map_err(RadialComparisonError::InvalidLeftFixture)?;
    right
        .validate()
        .map_err(RadialComparisonError::InvalidRightFixture)?;

    let compatibility = assess_compatibility(left, right);
    if compatibility.status == CompatibilityStatus::Unmatched {
        return Ok(RadialComparisonResult {
            status: RadialComparisonStatus::Unmatched,
            differing_fields: compatibility
                .differing_fields
                .into_iter()
                .map(str::to_owned)
                .collect(),
            unresolved_reason: None,
            in_value: RadialChannelMetrics::default(),
            in_derivative: RadialChannelMetrics::default(),
            up_value: RadialChannelMetrics::default(),
            up_derivative: RadialChannelMetrics::default(),
        });
    }

    if left.data.radial_samples.len() != right.data.radial_samples.len()
        || left
            .data
            .radial_samples
            .iter()
            .zip(&right.data.radial_samples)
            .any(|(left_sample, right_sample)| left_sample.r_over_m != right_sample.r_over_m)
    {
        return Ok(unresolved_radial_comparison(
            "radial sample nodes differ; interpolation is intentionally not implicit",
        ));
    }

    let mut result = RadialComparisonResult {
        status: RadialComparisonStatus::NumericallyAgreeing,
        differing_fields: Vec::new(),
        unresolved_reason: None,
        in_value: RadialChannelMetrics::default(),
        in_derivative: RadialChannelMetrics::default(),
        up_value: RadialChannelMetrics::default(),
        up_derivative: RadialChannelMetrics::default(),
    };

    for (left_sample, right_sample) in left
        .data
        .radial_samples
        .iter()
        .zip(&right.data.radial_samples)
    {
        if !compare_optional_channel(
            left_sample.in_value,
            right_sample.in_value,
            config,
            &mut result.in_value,
        ) || !compare_optional_channel(
            left_sample.in_derivative,
            right_sample.in_derivative,
            config,
            &mut result.in_derivative,
        ) || !compare_optional_channel(
            left_sample.up_value,
            right_sample.up_value,
            config,
            &mut result.up_value,
        ) || !compare_optional_channel(
            left_sample.up_derivative,
            right_sample.up_derivative,
            config,
            &mut result.up_derivative,
        ) {
            return Ok(unresolved_radial_comparison(
                "radial channel coverage differs between fixtures",
            ));
        }
    }

    let compared_values = result.in_value.compared_values
        + result.in_derivative.compared_values
        + result.up_value.compared_values
        + result.up_derivative.compared_values;
    if compared_values == 0 {
        return Ok(unresolved_radial_comparison(
            "fixtures contain no paired radial solution values",
        ));
    }

    let failures = result.in_value.values_outside_tolerance
        + result.in_derivative.values_outside_tolerance
        + result.up_value.values_outside_tolerance
        + result.up_derivative.values_outside_tolerance;
    if failures > 0 {
        result.status = RadialComparisonStatus::NumericallyDisagreeing;
    }
    Ok(result)
}

fn unresolved_radial_comparison(reason: &'static str) -> RadialComparisonResult {
    RadialComparisonResult {
        status: RadialComparisonStatus::Unresolved,
        differing_fields: Vec::new(),
        unresolved_reason: Some(reason.to_owned()),
        in_value: RadialChannelMetrics::default(),
        in_derivative: RadialChannelMetrics::default(),
        up_value: RadialChannelMetrics::default(),
        up_derivative: RadialChannelMetrics::default(),
    }
}

fn compare_optional_channel(
    left: Option<ComplexValue>,
    right: Option<ComplexValue>,
    config: RadialComparisonConfig,
    metrics: &mut RadialChannelMetrics,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            let absolute_error = (left.re - right.re).hypot(left.im - right.im);
            let reference_scale = left.re.hypot(left.im).max(right.re.hypot(right.im));
            let tolerance =
                config.absolute_tolerance + config.relative_tolerance * reference_scale;
            let scaled_error = absolute_error / tolerance;
            metrics.compared_values += 1;
            metrics.maximum_absolute_error =
                metrics.maximum_absolute_error.max(absolute_error);
            metrics.maximum_scaled_error = metrics.maximum_scaled_error.max(scaled_error);
            if scaled_error > 1.0 {
                metrics.values_outside_tolerance += 1;
            }
            true
        }
        _ => false,
    }
}

fn require_nonempty(value: &str, name: &'static str) -> Result<(), FixtureValidationError> {
    if value.trim().is_empty() {
        Err(FixtureValidationError::EmptyField(name))
    } else {
        Ok(())
    }
}

fn validate_sha256(value: &str, name: &'static str) -> Result<(), FixtureValidationError> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(FixtureValidationError::InvalidSha256(name))
    }
}

fn sample_has_values(sample: &RadialSample) -> bool {
    sample.in_value.is_some()
        || sample.in_derivative.is_some()
        || sample.up_value.is_some()
        || sample.up_derivative.is_some()
}

fn sample_values_are_finite(sample: &RadialSample) -> bool {
    [
        sample.in_value,
        sample.in_derivative,
        sample.up_value,
        sample.up_derivative,
    ]
    .into_iter()
    .flatten()
    .all(ComplexValue::is_finite)
}

fn validate_optional_complex(
    value: Option<ComplexValue>,
    name: &'static str,
) -> Result<(), FixtureValidationError> {
    if value.is_some_and(|value| !value.is_finite()) {
        Err(FixtureValidationError::NonFiniteDiagnostic(name))
    } else {
        Ok(())
    }
}

fn close(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance * left.abs().max(right.abs()).max(1.0)
}

fn complex_close(left: ComplexValue, right: ComplexValue, tolerance: f64) -> bool {
    close(left.re, right.re, tolerance) && close(left.im, right.im, tolerance)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ReferenceFixture {
        ReferenceFixture {
            schema_version: FIXTURE_SCHEMA_VERSION,
            fixture_id: "kerr-s-2-l2-m2-a03-w05".to_owned(),
            provenance: FixtureProvenance {
                engine: "external-reference".to_owned(),
                engine_version: "1.2.1".to_owned(),
                source_revision: Some("fixture-test-revision".to_owned()),
                producer_artifact_sha256: Some("a".repeat(64)),
                generated_at_utc: "2026-11-09T00:00:00Z".to_owned(),
                precision_bits: 200,
            },
            problem: KerrProblem {
                mass_kg: 1.988_47e30,
                spin_over_mass: 0.3,
                spin_weight: -2,
                ell: 2,
                m: 2,
                omega_m: ComplexValue { re: 0.5, im: 0.0 },
                angular_eigenvalue: Some(ComplexValue { re: 3.0, im: 0.0 }),
                angular_branch_id: Some("continued-from-real-frequency".to_owned()),
                source_id: None,
            },
            fourier_convention: FourierConvention::EMinusIOmegaT,
            radial_coordinate: RadialCoordinate::BoyerLindquistROverM,
            frequency_normalization: FrequencyNormalization::DimensionlessMOmega,
            normalization: BoundaryNormalization {
                in_basis: "ingoing at outer horizon".to_owned(),
                up_basis: "outgoing at future null infinity".to_owned(),
                radial_function_definition: "producer-documented Teukolsky radial field".to_owned(),
                derivative_definition: "d/dr_over_M".to_owned(),
            },
            data: ReferenceData {
                radial_samples: vec![
                    RadialSample {
                        r_over_m: 3.0,
                        in_value: Some(ComplexValue { re: 1.0, im: 0.0 }),
                        in_derivative: Some(ComplexValue { re: 0.1, im: 0.0 }),
                        up_value: None,
                        up_derivative: None,
                    },
                    RadialSample {
                        r_over_m: 4.0,
                        in_value: None,
                        in_derivative: None,
                        up_value: Some(ComplexValue { re: 0.8, im: 0.1 }),
                        up_derivative: Some(ComplexValue {
                            re: -0.02,
                            im: 0.01,
                        }),
                    },
                ],
                diagnostics: Some(ReferenceDiagnostics {
                    wronskian: Some(ComplexValue { re: 1.0, im: 0.0 }),
                    incident_amplitude: None,
                    reflection_amplitude: None,
                    transmission_amplitude: None,
                    phase_shift: None,
                }),
            },
        }
    }

    #[test]
    fn validates_a_complete_reference_fixture() {
        assert_eq!(fixture().validate(), Ok(()));
    }

    #[test]
    fn rejects_nonfinite_radial_data_and_bad_horizon_domain() {
        let mut nonfinite = fixture();
        nonfinite.data.radial_samples[0].in_value = Some(ComplexValue {
            re: f64::INFINITY,
            im: 0.0,
        });
        assert_eq!(
            nonfinite.validate(),
            Err(FixtureValidationError::InvalidRadialSample(0))
        );

        let mut inside_horizon = fixture();
        inside_horizon.data.radial_samples[0].r_over_m = 1.1;
        assert_eq!(
            inside_horizon.validate(),
            Err(FixtureValidationError::InvalidRadialSample(0))
        );
    }

    #[test]
    fn rejects_unsorted_or_duplicate_radial_nodes() {
        let mut value = fixture();
        value.data.radial_samples.swap(0, 1);
        assert_eq!(
            value.validate(),
            Err(FixtureValidationError::RadialSamplesNotStrictlyIncreasing(
                1
            ))
        );
    }

    #[test]
    fn rejects_incomplete_angular_branch_metadata() {
        let mut value = fixture();
        value.problem.angular_branch_id = None;
        assert_eq!(
            value.validate(),
            Err(FixtureValidationError::AngularBranchMetadataIncomplete)
        );
    }

    #[test]
    fn convention_mismatches_are_unmatched_not_numerical_failures() {
        let left = fixture();
        let mut right = fixture();
        right.fourier_convention = FourierConvention::EPlusIOmegaT;
        let result = assess_compatibility(&left, &right);
        assert_eq!(result.status, CompatibilityStatus::Unmatched);
        assert_eq!(result.differing_fields, vec!["fourier_convention"]);
    }

    #[test]
    fn source_identity_mismatch_is_unmatched() {
        let left = fixture();
        let mut right = fixture();
        right.problem.source_id = Some("different-orbit".to_owned());
        let result = assess_compatibility(&left, &right);
        assert_eq!(result.status, CompatibilityStatus::Unmatched);
        assert_eq!(result.differing_fields, vec!["source_id"]);
    }

    #[test]
    fn matched_problem_identity_does_not_claim_numerical_agreement() {
        let left = fixture();
        let right = fixture();
        let result = assess_compatibility(&left, &right);
        assert_eq!(result.status, CompatibilityStatus::Matched);
        assert!(result.differing_fields.is_empty());
    }

    fn comparison_config() -> RadialComparisonConfig {
        RadialComparisonConfig {
            absolute_tolerance: 1.0e-12,
            relative_tolerance: 1.0e-9,
        }
    }

    #[test]
    fn identical_radial_fixtures_agree_on_exact_common_nodes() {
        let left = fixture();
        let right = fixture();
        let comparison =
            compare_radial_fixtures(&left, &right, comparison_config()).unwrap();
        assert_eq!(
            comparison.status,
            RadialComparisonStatus::NumericallyAgreeing
        );
        assert_eq!(comparison.in_value.compared_values, 1);
        assert_eq!(comparison.in_derivative.compared_values, 1);
        assert_eq!(comparison.up_value.compared_values, 1);
        assert_eq!(comparison.up_derivative.compared_values, 1);
        assert_eq!(comparison.in_value.maximum_absolute_error, 0.0);
    }

    #[test]
    fn radial_comparison_reports_scaled_errors_and_disagreement() {
        let left = fixture();
        let mut within = fixture();
        within.data.radial_samples[0].in_value =
            Some(ComplexValue { re: 1.0 + 1.0e-10, im: 0.0 });
        let agreeing = compare_radial_fixtures(&left, &within, comparison_config()).unwrap();
        assert_eq!(
            agreeing.status,
            RadialComparisonStatus::NumericallyAgreeing
        );
        assert!(agreeing.in_value.maximum_scaled_error < 1.0);

        let mut outside = fixture();
        outside.data.radial_samples[0].in_value =
            Some(ComplexValue { re: 1.0 + 1.0e-5, im: 0.0 });
        let disagreeing = compare_radial_fixtures(&left, &outside, comparison_config()).unwrap();
        assert_eq!(
            disagreeing.status,
            RadialComparisonStatus::NumericallyDisagreeing
        );
        assert_eq!(disagreeing.in_value.values_outside_tolerance, 1);
        assert!(disagreeing.in_value.maximum_scaled_error > 1.0);
    }

    #[test]
    fn radial_comparison_abstains_on_different_nodes_or_channel_coverage() {
        let left = fixture();
        let mut different_nodes = fixture();
        different_nodes.data.radial_samples[1].r_over_m = 4.1;
        let node_result =
            compare_radial_fixtures(&left, &different_nodes, comparison_config()).unwrap();
        assert_eq!(node_result.status, RadialComparisonStatus::Unresolved);
        assert!(node_result.unresolved_reason.unwrap().contains("nodes differ"));

        let mut different_coverage = fixture();
        different_coverage.data.radial_samples[0].in_derivative = None;
        let coverage_result =
            compare_radial_fixtures(&left, &different_coverage, comparison_config()).unwrap();
        assert_eq!(coverage_result.status, RadialComparisonStatus::Unresolved);
        assert!(coverage_result
            .unresolved_reason
            .unwrap()
            .contains("coverage differs"));
    }

    #[test]
    fn radial_comparison_reports_convention_mismatch_without_comparing_values() {
        let left = fixture();
        let mut right = fixture();
        right.fourier_convention = FourierConvention::EPlusIOmegaT;
        let comparison = compare_radial_fixtures(&left, &right, comparison_config()).unwrap();
        assert_eq!(comparison.status, RadialComparisonStatus::Unmatched);
        assert_eq!(comparison.differing_fields, vec!["fourier_convention"]);
        assert_eq!(comparison.in_value.compared_values, 0);
    }

    #[test]
    fn radial_comparison_rejects_invalid_tolerances() {
        let left = fixture();
        let error = compare_radial_fixtures(
            &left,
            &left,
            RadialComparisonConfig {
                absolute_tolerance: 0.0,
                relative_tolerance: 0.0,
            },
        );
        assert_eq!(error, Err(RadialComparisonError::InvalidTolerance));
    }

    #[test]
    fn serde_roundtrip_preserves_the_fixture_contract() {
        let original = fixture();
        let encoded = serde_json::to_string(&original).unwrap();
        let decoded: ReferenceFixture = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, original);
        decoded.validate().unwrap();
    }
}
