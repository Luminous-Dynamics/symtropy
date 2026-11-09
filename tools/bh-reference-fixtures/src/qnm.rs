//! Typed Kerr QNM sequence fixtures.
//!
//! A solver's overtone index is metadata, not proof of family identity.
//! Prolate/oblate labels therefore carry explicit multi-mode asymptotic
//! evidence, while ambiguous or unresolved sequences remain representable.

use serde::{Deserialize, Serialize};

use crate::{
    AngularEigenvalueConvention, ComplexValue, FixtureProvenance, FourierConvention,
    FrequencyNormalization,
};

pub const QNM_SEQUENCE_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmSolverMethod {
    Leaver,
    IncidenceAmplitude,
    SpheroidalEigenvalue,
    SpectralTwoDimensional,
    SpectralOneDimensionalSchwarzschild,
    InterpolationFromPythonQnmCache,
    LargeLAsymptoticSchwarzschild,
    LargeNAsymptoticSchwarzschild,
    KerrNormalProlateGenusOneWkb,
    PotentialPeakPadeWkb,
    PotentialPeakBorelPadeWkb,
    ExactExtremalWkb,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmFamilyClassification {
    Prolate,
    Oblate,
    Unresolved,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmModeResiduals {
    pub angular_ode_relative: Option<f64>,
    pub radial_ode_relative: Option<f64>,
    pub continued_fraction_relative: Option<f64>,
    pub incidence_amplitude_relative: Option<f64>,
}

impl QnmModeResiduals {
    fn validate(&self) -> bool {
        let residuals = [
            self.angular_ode_relative,
            self.radial_ode_relative,
            self.continued_fraction_relative,
            self.incidence_amplitude_relative,
        ];
        residuals
            .into_iter()
            .flatten()
            .all(|value| value.is_finite() && value >= 0.0)
            && residuals.into_iter().any(|value| value.is_some())
    }
}

/// Multi-mode asymptotic summary used to justify a prolate or oblate label.
///
/// angular_scaling_ratio is compared with the explicitly recorded
/// expected_angular_scaling_limit. For the normal prolate branch in Hatsuda
/// (2026), that limit is eta = 2 Lbar + 1, not universally 1; Lbar is the
/// equatorial oscillator level. An oblate-family convention may use a
/// different limit and must state it explicitly. Frequency spacings are
/// expressed as dimensionless M delta-omega.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmFamilyEvidence {
    pub angular_scaling_ratio: ComplexValue,
    pub expected_angular_scaling_limit: ComplexValue,
    pub observed_frequency_spacing_m: ComplexValue,
    pub predicted_frequency_spacing_m: ComplexValue,
    pub angular_scaling_relative_residual: f64,
    pub frequency_spacing_relative_residual: f64,
    pub consecutive_modes_used: u32,
}

/// Independently predicted asymptotic frequency spacings. These are inputs
/// from the relevant optical/WKB reference, not estimates fitted from the
/// mode sequence being classified.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmFamilyAsymptoticTargets {
    pub prolate_spacing_m: ComplexValue,
    pub oblate_spacing_m: ComplexValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmFamilyAnalysisConfig {
    /// Number of consecutive sequence modes used in the asymptotic window.
    /// At least three are required so angular scaling and adjacent spacings
    /// are not inferred from one isolated mode.
    pub minimum_consecutive_modes: usize,
    pub maximum_angular_residual: f64,
    pub maximum_frequency_spacing_residual: f64,
    pub maximum_angular_drift: f64,
    pub maximum_frequency_spacing_drift: f64,
    pub maximum_solver_residual: f64,
}

impl Default for QnmFamilyAnalysisConfig {
    fn default() -> Self {
        Self {
            minimum_consecutive_modes: 4,
            maximum_angular_residual: 5.0e-2,
            maximum_frequency_spacing_residual: 5.0e-2,
            maximum_angular_drift: 5.0e-2,
            maximum_frequency_spacing_drift: 5.0e-2,
            maximum_solver_residual: 1.0e-8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmFamilyCandidateDiagnostics {
    pub expected_angular_scaling_limit: ComplexValue,
    pub observed_angular_scaling_limit: ComplexValue,
    pub expected_frequency_spacing_m: ComplexValue,
    pub observed_frequency_spacing_m: ComplexValue,
    pub maximum_angular_residual: f64,
    pub maximum_frequency_spacing_residual: f64,
    pub maximum_angular_drift: f64,
    pub maximum_frequency_spacing_drift: f64,
    pub consecutive_modes_used: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmFamilyAnalysis {
    pub classification: QnmFamilyClassification,
    pub prolate: Option<QnmFamilyCandidateDiagnostics>,
    pub oblate: Option<QnmFamilyCandidateDiagnostics>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QnmFamilyAnalysisError {
    InvalidSequence(QnmSequenceValidationError),
    UnsupportedAngularEigenvalueConvention,
    InvalidConfiguration,
}

impl core::fmt::Display for QnmFamilyAnalysisError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSequence(error) => write!(f, "invalid QNM sequence: {error}"),
            Self::UnsupportedAngularEigenvalueConvention => write!(
                f,
                "family analysis requires the spin-weighted spheroidal A eigenvalue convention"
            ),
            Self::InvalidConfiguration => write!(
                f,
                "family analysis targets and tolerances must be finite, positive where required, and use at least three modes"
            ),
        }
    }
}

impl std::error::Error for QnmFamilyAnalysisError {}

fn complex_add(left: ComplexValue, right: ComplexValue) -> ComplexValue {
    ComplexValue {
        re: left.re + right.re,
        im: left.im + right.im,
    }
}

fn complex_sub(left: ComplexValue, right: ComplexValue) -> ComplexValue {
    ComplexValue {
        re: left.re - right.re,
        im: left.im - right.im,
    }
}

fn complex_mul(left: ComplexValue, right: ComplexValue) -> ComplexValue {
    ComplexValue {
        re: left.re * right.re - left.im * right.im,
        im: left.re * right.im + left.im * right.re,
    }
}

fn complex_div(left: ComplexValue, right: ComplexValue) -> Option<ComplexValue> {
    let denominator = right.re * right.re + right.im * right.im;
    if !denominator.is_finite() || denominator <= f64::MIN_POSITIVE {
        return None;
    }
    let value = ComplexValue {
        re: (left.re * right.re + left.im * right.im) / denominator,
        im: (left.im * right.re - left.re * right.im) / denominator,
    };
    value.is_finite().then_some(value)
}

fn complex_norm(value: ComplexValue) -> f64 {
    value.re.hypot(value.im)
}

fn complex_mean(values: &[ComplexValue]) -> Option<ComplexValue> {
    if values.is_empty() {
        return None;
    }
    let sum = values
        .iter()
        .copied()
        .fold(ComplexValue { re: 0.0, im: 0.0 }, complex_add);
    let divisor = values.len() as f64;
    Some(ComplexValue {
        re: sum.re / divisor,
        im: sum.im / divisor,
    })
}

fn scaled_complex_error(value: ComplexValue, expected: ComplexValue) -> f64 {
    complex_norm(complex_sub(value, expected)) / complex_norm(expected).max(f64::MIN_POSITIVE)
}

fn mode_has_qualified_residuals(mode: &QnmModeRecord, maximum_solver_residual: f64) -> bool {
    let Some(residuals) = &mode.residuals else {
        return false;
    };
    let Some(angular) = residuals.angular_ode_relative else {
        return false;
    };
    let radial_or_root = residuals
        .radial_ode_relative
        .or(residuals.continued_fraction_relative)
        .or(residuals.incidence_amplitude_relative);
    angular <= maximum_solver_residual
        && radial_or_root.is_some_and(|value| value <= maximum_solver_residual)
        && mode.method_applicability != QnmMethodApplicability::KnownBreakdown
        && mode.method_applicability != QnmMethodApplicability::Unresolved
}

/// Compute family diagnostics from the mode sequence itself.
///
/// This routine does not trust the producer's family label, overtone index,
/// or stored family evidence. It uses sequence position, angular eigenvalues,
/// complex frequency spacings, branch continuity and per-mode residuals.
/// Predicted spacings are supplied independently by the corresponding
/// optical/WKB reference. A candidate passes only if both its angular
/// asymptotic scaling and frequency spacing satisfy the caller's tolerances
/// throughout the final consecutive-mode window.
///
/// For the prolate normal angular branch, A/(i chi M omega) tends to
/// eta = 2 Lbar + 1. For the oblate branch, A/(-chi^2 (M omega)^2) tends to
/// one. Only the typed spin-weighted spheroidal A convention is accepted;
/// a radial Teukolsky lambda is not silently substituted.
pub fn analyze_qnm_family_sequence(
    sequence: &KerrQnmSequenceFixture,
    targets: QnmFamilyAsymptoticTargets,
    config: QnmFamilyAnalysisConfig,
) -> Result<QnmFamilyAnalysis, QnmFamilyAnalysisError> {
    sequence
        .validate()
        .map_err(QnmFamilyAnalysisError::InvalidSequence)?;

    let tolerances = [
        config.maximum_angular_residual,
        config.maximum_frequency_spacing_residual,
        config.maximum_angular_drift,
        config.maximum_frequency_spacing_drift,
        config.maximum_solver_residual,
    ];
    if config.minimum_consecutive_modes < 3
        || sequence.modes.len() < config.minimum_consecutive_modes
        || tolerances
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        || !targets.prolate_spacing_m.is_finite()
        || !targets.oblate_spacing_m.is_finite()
        || complex_norm(targets.prolate_spacing_m) <= f64::MIN_POSITIVE
        || complex_norm(targets.oblate_spacing_m) <= f64::MIN_POSITIVE
    {
        return Err(QnmFamilyAnalysisError::InvalidConfiguration);
    }
    if sequence.angular_eigenvalue_convention
        != AngularEigenvalueConvention::SpinWeightedSpheroidalA
    {
        return Err(QnmFamilyAnalysisError::UnsupportedAngularEigenvalueConvention);
    }

    let window = &sequence.modes[sequence.modes.len() - config.minimum_consecutive_modes..];
    let first_branch = &window[0].angular_branch_id;
    let first_path = &window[0].angular_continuation_path_id;
    let branch_is_continuous = window.iter().all(|mode| {
        mode.angular_branch_id == *first_branch
            && mode.angular_continuation_path_id == *first_path
            && mode_has_qualified_residuals(mode, config.maximum_solver_residual)
    });

    if !branch_is_continuous {
        return Ok(QnmFamilyAnalysis {
            classification: QnmFamilyClassification::Unresolved,
            prolate: None,
            oblate: None,
            reason: Some(
                "angular branch/continuation path changed, method is outside its declared regime, or a mode lacks qualified angular and radial/root residuals"
                    .to_owned(),
            ),
        });
    }

    let spin = sequence.spin_over_mass;
    let oscillator_level = sequence.normal_prolate_oscillator_level;
    let expected_prolate_angular = oscillator_level.map(|level| ComplexValue {
        re: 2.0 * f64::from(level) + 1.0,
        im: 0.0,
    });
    let expected_oblate_angular = ComplexValue { re: 1.0, im: 0.0 };

    let prolate = expected_prolate_angular.and_then(|expected| {
        let ratios = window
            .iter()
            .map(|mode| {
                let denominator = complex_mul(ComplexValue { re: 0.0, im: spin }, mode.omega_m);
                complex_div(mode.angular_eigenvalue, denominator)
            })
            .collect::<Option<Vec<_>>>()?;
        let spacings = window
            .windows(2)
            .map(|pair| complex_sub(pair[1].omega_m, pair[0].omega_m))
            .collect::<Vec<_>>();
        candidate_diagnostics(
            &ratios,
            &spacings,
            expected,
            targets.prolate_spacing_m,
            config,
        )
    });

    let oblate = (spin > 0.0)
        .then(|| {
            let ratios = window
                .iter()
                .map(|mode| {
                    let omega_squared = complex_mul(mode.omega_m, mode.omega_m);
                    let denominator = ComplexValue {
                        re: -spin * spin * omega_squared.re,
                        im: -spin * spin * omega_squared.im,
                    };
                    complex_div(mode.angular_eigenvalue, denominator)
                })
                .collect::<Option<Vec<_>>>()?;
            let spacings = window
                .windows(2)
                .map(|pair| complex_sub(pair[1].omega_m, pair[0].omega_m))
                .collect::<Vec<_>>();
            candidate_diagnostics(
                &ratios,
                &spacings,
                expected_oblate_angular,
                targets.oblate_spacing_m,
                config,
            )
        })
        .flatten();

    let classification = match (prolate.is_some(), oblate.is_some()) {
        (true, false) => QnmFamilyClassification::Prolate,
        (false, true) => QnmFamilyClassification::Oblate,
        (true, true) => QnmFamilyClassification::Ambiguous,
        (false, false) => QnmFamilyClassification::Unresolved,
    };
    let reason = match classification {
        QnmFamilyClassification::Prolate => None,
        QnmFamilyClassification::Oblate => None,
        QnmFamilyClassification::Ambiguous => Some(
            "both asymptotic family models satisfy the supplied tolerances; narrow the regime or report ambiguity"
                .to_owned(),
        ),
        QnmFamilyClassification::Unresolved => Some(
            "neither family satisfies both angular-scaling and frequency-spacing tolerances"
                .to_owned(),
        ),
    };

    Ok(QnmFamilyAnalysis {
        classification,
        prolate,
        oblate,
        reason,
    })
}

fn candidate_diagnostics(
    angular_ratios: &[ComplexValue],
    frequency_spacings: &[ComplexValue],
    expected_angular: ComplexValue,
    expected_spacing: ComplexValue,
    config: QnmFamilyAnalysisConfig,
) -> Option<QnmFamilyCandidateDiagnostics> {
    if angular_ratios.len() < config.minimum_consecutive_modes
        || frequency_spacings.len() + 1 < config.minimum_consecutive_modes
    {
        return None;
    }
    let observed_angular = complex_mean(angular_ratios)?;
    let observed_spacing = complex_mean(frequency_spacings)?;
    let maximum_angular_residual = angular_ratios
        .iter()
        .map(|value| scaled_complex_error(*value, expected_angular))
        .fold(0.0, f64::max);
    let maximum_spacing_residual = frequency_spacings
        .iter()
        .map(|value| scaled_complex_error(*value, expected_spacing))
        .fold(0.0, f64::max);
    let maximum_angular_drift = angular_ratios
        .windows(2)
        .map(|pair| scaled_complex_error(pair[1], pair[0]))
        .fold(0.0, f64::max);
    let maximum_frequency_spacing_drift = frequency_spacings
        .windows(2)
        .map(|pair| scaled_complex_error(pair[1], pair[0]))
        .fold(0.0, f64::max);

    let accepted = maximum_angular_residual <= config.maximum_angular_residual
        && maximum_spacing_residual <= config.maximum_frequency_spacing_residual
        && maximum_angular_drift <= config.maximum_angular_drift
        && maximum_frequency_spacing_drift <= config.maximum_frequency_spacing_drift;
    accepted.then_some(QnmFamilyCandidateDiagnostics {
        expected_angular_scaling_limit: expected_angular,
        observed_angular_scaling_limit: observed_angular,
        expected_frequency_spacing_m: expected_spacing,
        observed_frequency_spacing_m: observed_spacing,
        maximum_angular_residual,
        maximum_frequency_spacing_residual: maximum_spacing_residual,
        maximum_angular_drift,
        maximum_frequency_spacing_drift,
        consecutive_modes_used: angular_ratios.len(),
    })
}

/// One computed mode. sequence_position preserves the order in the
/// continuation/analysis sequence. overtone_index is optional because a
/// unique traditional n label may be unavailable near crossings or in
/// nonstandard high-overtone sectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmInitialGuessSource {
    ExplicitIndependent,
    BhptInterpolationTable,
    PythonQnmCache,
    AsymptoticFallback,
    PreviousSpinContinuation,
    PreviousOvertoneContinuation,
    NoInitialGuessRequired,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmMethodApplicability {
    WithinDocumentedRegime,
    Extrapolated,
    KnownBreakdown,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmRiemannSheetKind {
    Conventional,
    Unconventional,
    OtherNamed,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmContinuationClass {
    ClassI,
    ClassII,
    OtherNamed,
    NotApplicable,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmFrequencyCutRelation {
    OffCut,
    PositiveRealSide,
    NegativeRealSide,
    OnCut,
    NotApplicable,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmModeRecord {
    /// Stable identity of this analytic mode lineage; not inferred from n.
    pub mode_lineage_id: String,
    pub riemann_sheet_kind: QnmRiemannSheetKind,
    /// Producer-defined sheet/chart identifier.
    pub riemann_sheet_id: String,
    pub continuation_class: QnmContinuationClass,
    pub continuation_class_label: Option<String>,
    pub frequency_cut_relation: QnmFrequencyCutRelation,
    pub sequence_position: u32,
    pub overtone_index: Option<u32>,
    pub omega_m: ComplexValue,
    pub angular_eigenvalue: ComplexValue,
    pub angular_branch_id: String,
    /// Producer-defined ID for the path used to continue the angular branch.
    pub angular_continuation_path_id: Option<String>,
    pub solver_method: QnmSolverMethod,
    pub method_applicability: QnmMethodApplicability,
    pub initial_guess: Option<ComplexValue>,
    /// Records shared seed-table or continuation dependencies between methods.
    pub initial_guess_source: QnmInitialGuessSource,
    pub precision_bits: u16,
    pub residuals: Option<QnmModeResiduals>,
    /// Hatsuda's normal-prolate WKB integer N. It is not generally the
    /// conventional overtone index n.
    pub wkb_quantization_index: Option<u32>,
    pub wkb_truncation_order: Option<u32>,
    pub wkb_optimal_truncation_order: Option<u32>,
    /// Higher quantum periods in the selected genus-one cycle basis.
    pub wkb_cycle_period_j0: Option<ComplexValue>,
    pub wkb_cycle_period_j1: Option<ComplexValue>,
    pub wkb_quantization_residual: Option<f64>,
    pub wkb_truncation_error_estimate: Option<f64>,
    /// Explicit cycle label; the spectral curve alone does not select it.
    pub wkb_cycle_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KerrQnmSequenceFixture {
    pub schema_version: u32,
    pub sequence_id: String,
    pub provenance: FixtureProvenance,
    pub spin_over_mass: f64,
    pub spin_weight: i32,
    pub ell: u32,
    pub m: i32,
    pub angular_eigenvalue_convention: AngularEigenvalueConvention,
    pub angular_eigenvalue_definition: Option<String>,
    pub fourier_convention: FourierConvention,
    pub frequency_normalization: FrequencyNormalization,
    pub family_classification: QnmFamilyClassification,
    /// Equatorial oscillator level Lbar for the normal prolate angular branch.
    /// For scalar fields Lbar = ell - |m|; for nonzero spin weight the mapping
    /// must be established by branch continuation and is not inferred here.
    pub normal_prolate_oscillator_level: Option<u32>,
    pub family_evidence: Option<QnmFamilyEvidence>,
    pub modes: Vec<QnmModeRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QnmSequenceValidationError {
    UnsupportedSchemaVersion(u32),
    EmptyField(&'static str),
    InvalidPrecision,
    InvalidSha256,
    InvalidSpin,
    InvalidModeNumbers,
    AngularEigenvalueConventionIncomplete,
    EmptySequence,
    SequencePositionsNotStrictlyIncreasing(usize),
    DuplicateOvertoneIndex(u32),
    NonFiniteModeValue(usize),
    InvalidSpectralIdentity(usize),
    InconsistentInitialGuessMetadata(usize),
    InvalidModeResiduals(usize),
    InvalidFamilyEvidence,
    MissingFamilyEvidence,
    MissingWkbMetadata(usize),
    InvalidWkbMetadata(usize),
    InvalidWkbSpinDomain(usize),
    ExtremalFixtureRequiresScalarExactWkb,
}

impl core::fmt::Display for QnmSequenceValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported QNM sequence schema version {version}")
            }
            Self::EmptyField(name) => write!(f, "required QNM sequence field {name} is empty"),
            Self::InvalidPrecision => write!(f, "all QNM modes require precision_bits >= 53"),
            Self::InvalidSha256 => write!(
                f,
                "producer artifact SHA-256 must contain 64 hex characters"
            ),
            Self::InvalidSpin => write!(f, "spin_over_mass must be finite and satisfy |a/M| < 1"),
            Self::InvalidModeNumbers => {
                write!(f, "mode numbers must satisfy ell >= |s| and |m| <= ell")
            }
            Self::AngularEigenvalueConventionIncomplete => {
                write!(
                    f,
                    "producer-defined angular eigenvalue convention requires a definition"
                )
            }
            Self::EmptySequence => write!(f, "at least one QNM mode is required"),
            Self::SequencePositionsNotStrictlyIncreasing(index) => {
                write!(
                    f,
                    "sequence position at mode {index} is not strictly increasing"
                )
            }
            Self::DuplicateOvertoneIndex(index) => {
                write!(
                    f,
                    "overtone index {index} occurs more than once in the sequence"
                )
            }
            Self::NonFiniteModeValue(index) => {
                write!(
                    f,
                    "QNM mode {index} has a non-finite frequency/eigenvalue/guess"
                )
            }
            Self::InvalidSpectralIdentity(index) => write!(
                f,
                "QNM mode {index} has incomplete or inconsistent Riemann-sheet/continuation identity"
            ),
            Self::InconsistentInitialGuessMetadata(index) => {
                write!(
                    f,
                    "QNM mode {index} supplies an initial guess but marks it as not required"
                )
            }
            Self::InvalidModeResiduals(index) => {
                write!(
                    f,
                    "QNM mode {index} has invalid or absent residual metadata"
                )
            }
            Self::InvalidFamilyEvidence => {
                write!(
                    f,
                    "family evidence contains invalid ratios, spacings, or residuals"
                )
            }
            Self::MissingFamilyEvidence => {
                write!(
                    f,
                    "prolate/oblate classification requires multi-mode asymptotic evidence"
                )
            }
            Self::MissingWkbMetadata(index) => {
                write!(
                    f,
                    "QNM mode {index} requires WKB index, truncation order, branch path and cycle metadata"
                )
            }
            Self::InvalidWkbMetadata(index) => {
                write!(
                    f,
                    "QNM mode {index} has non-finite or negative WKB periods/residual/error estimate"
                )
            }
            Self::InvalidWkbSpinDomain(index) => {
                write!(
                    f,
                    "QNM mode {index} uses the fixed-spin normal-prolate WKB method outside 0 < a/M < 1"
                )
            }
            Self::ExtremalFixtureRequiresScalarExactWkb => {
                write!(
                    f,
                    "exact extremality is represented only by a scalar exact-WKB fixture"
                )
            }
        }
    }
}

impl std::error::Error for QnmSequenceValidationError {}

impl KerrQnmSequenceFixture {
    /// Return the dimensionless small-spin parameter governing the
    /// non-uniformity of the normal-prolate genus-one expansion,
    /// chi^(-4/3) / (N + 1/2). The asymptotic regime requires this parameter
    /// to be small; this accessor does not impose a universal cutoff.
    pub fn normal_prolate_wkb_small_spin_parameter(&self, mode: &QnmModeRecord) -> Option<f64> {
        if mode.solver_method != QnmSolverMethod::KerrNormalProlateGenusOneWkb
            || !self.spin_over_mass.is_finite()
            || self.spin_over_mass <= 0.0
            || self.spin_over_mass >= 1.0
        {
            return None;
        }
        let quantization_index = mode.wkb_quantization_index?;
        let parameter =
            self.spin_over_mass.powf(-4.0 / 3.0) / (f64::from(quantization_index) + 0.5);
        parameter.is_finite().then_some(parameter)
    }

    /// Validate sequence structure and evidence presence. This does not prove
    /// that the producer's family label is mathematically correct.
    pub fn validate(&self) -> Result<(), QnmSequenceValidationError> {
        if self.schema_version != QNM_SEQUENCE_SCHEMA_VERSION {
            return Err(QnmSequenceValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.sequence_id.trim().is_empty() {
            return Err(QnmSequenceValidationError::EmptyField("sequence_id"));
        }
        if self.provenance.engine.trim().is_empty()
            || self.provenance.engine_version.trim().is_empty()
            || self.provenance.generated_at_utc.trim().is_empty()
        {
            return Err(QnmSequenceValidationError::EmptyField("provenance"));
        }
        if self.provenance.precision_bits < 53 {
            return Err(QnmSequenceValidationError::InvalidPrecision);
        }
        if let Some(digest) = &self.provenance.producer_artifact_sha256
            && (digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(QnmSequenceValidationError::InvalidSha256);
        }
        if !self.spin_over_mass.is_finite() || self.spin_over_mass.abs() > 1.0 {
            return Err(QnmSequenceValidationError::InvalidSpin);
        }
        if self.spin_over_mass.abs() == 1.0
            && (self.spin_weight != 0
                || self
                    .modes
                    .iter()
                    .any(|mode| mode.solver_method != QnmSolverMethod::ExactExtremalWkb))
        {
            return Err(QnmSequenceValidationError::ExtremalFixtureRequiresScalarExactWkb);
        }
        if self.ell < self.spin_weight.unsigned_abs() || self.m.unsigned_abs() > self.ell {
            return Err(QnmSequenceValidationError::InvalidModeNumbers);
        }
        if self.angular_eigenvalue_convention == AngularEigenvalueConvention::Other
            && self
                .angular_eigenvalue_definition
                .as_deref()
                .is_none_or(|definition| definition.trim().is_empty())
        {
            return Err(QnmSequenceValidationError::AngularEigenvalueConventionIncomplete);
        }

        if self.modes.is_empty() {
            return Err(QnmSequenceValidationError::EmptySequence);
        }

        let mut previous_position = None;
        let mut overtone_indices = std::collections::BTreeSet::new();
        for (index, mode) in self.modes.iter().enumerate() {
            if previous_position.is_some_and(|previous| mode.sequence_position <= previous) {
                return Err(
                    QnmSequenceValidationError::SequencePositionsNotStrictlyIncreasing(index),
                );
            }
            previous_position = Some(mode.sequence_position);

            if let Some(overtone) = mode.overtone_index
                && !overtone_indices.insert(overtone)
            {
                return Err(QnmSequenceValidationError::DuplicateOvertoneIndex(overtone));
            }
            if !mode.omega_m.is_finite()
                || !mode.angular_eigenvalue.is_finite()
                || mode.initial_guess.is_some_and(|guess| !guess.is_finite())
                || mode.angular_branch_id.trim().is_empty()
            {
                return Err(QnmSequenceValidationError::NonFiniteModeValue(index));
            }
            if mode.mode_lineage_id.trim().is_empty()
                || mode.riemann_sheet_id.trim().is_empty()
                || (mode.continuation_class == QnmContinuationClass::OtherNamed
                    && mode
                        .continuation_class_label
                        .as_deref()
                        .is_none_or(str::is_empty))
                || (mode.continuation_class != QnmContinuationClass::OtherNamed
                    && mode.continuation_class_label.is_some())
                || (mode.riemann_sheet_kind == QnmRiemannSheetKind::Unresolved
                    && mode.riemann_sheet_id != "unresolved")
            {
                return Err(QnmSequenceValidationError::InvalidSpectralIdentity(index));
            }
            if mode.initial_guess.is_some()
                && mode.initial_guess_source == QnmInitialGuessSource::NoInitialGuessRequired
            {
                return Err(QnmSequenceValidationError::InconsistentInitialGuessMetadata(index));
            }
            if mode.precision_bits < 53
                || mode
                    .residuals
                    .as_ref()
                    .is_none_or(|residuals| !residuals.validate())
            {
                return Err(QnmSequenceValidationError::InvalidModeResiduals(index));
            }
            if mode.solver_method == QnmSolverMethod::KerrNormalProlateGenusOneWkb {
                if self.spin_over_mass <= 0.0 || self.spin_over_mass >= 1.0 {
                    return Err(QnmSequenceValidationError::InvalidWkbSpinDomain(index));
                }
                if mode.wkb_quantization_index.is_none()
                    || mode.wkb_truncation_order.is_none()
                    || mode.wkb_cycle_id.as_deref().is_none_or(str::is_empty)
                    || mode
                        .angular_continuation_path_id
                        .as_deref()
                        .is_none_or(str::is_empty)
                    || self.family_classification != QnmFamilyClassification::Prolate
                    || self.normal_prolate_oscillator_level.is_none()
                {
                    return Err(QnmSequenceValidationError::MissingWkbMetadata(index));
                }
                if mode
                    .wkb_cycle_period_j0
                    .is_none_or(|value| !value.is_finite())
                    || mode
                        .wkb_cycle_period_j1
                        .is_none_or(|value| !value.is_finite())
                    || mode
                        .wkb_quantization_residual
                        .is_none_or(|value| !value.is_finite() || value < 0.0)
                    || mode
                        .wkb_truncation_error_estimate
                        .is_none_or(|value| !value.is_finite() || value < 0.0)
                {
                    return Err(QnmSequenceValidationError::InvalidWkbMetadata(index));
                }
            }
        }

        match self.family_classification {
            QnmFamilyClassification::Prolate | QnmFamilyClassification::Oblate => {
                let evidence = self
                    .family_evidence
                    .as_ref()
                    .ok_or(QnmSequenceValidationError::MissingFamilyEvidence)?;
                if !evidence.angular_scaling_ratio.is_finite()
                    || !evidence.expected_angular_scaling_limit.is_finite()
                    || !evidence.observed_frequency_spacing_m.is_finite()
                    || !evidence.predicted_frequency_spacing_m.is_finite()
                    || !evidence.angular_scaling_relative_residual.is_finite()
                    || evidence.angular_scaling_relative_residual < 0.0
                    || !evidence.frequency_spacing_relative_residual.is_finite()
                    || evidence.frequency_spacing_relative_residual < 0.0
                    || evidence.consecutive_modes_used < 2
                    || self.modes.len() < evidence.consecutive_modes_used as usize
                {
                    return Err(QnmSequenceValidationError::InvalidFamilyEvidence);
                }
            }
            QnmFamilyClassification::Unresolved | QnmFamilyClassification::Ambiguous => {
                if let Some(evidence) = &self.family_evidence
                    && (!evidence.angular_scaling_ratio.is_finite()
                        || !evidence.expected_angular_scaling_limit.is_finite()
                        || !evidence.observed_frequency_spacing_m.is_finite()
                        || !evidence.predicted_frequency_spacing_m.is_finite()
                        || !evidence.angular_scaling_relative_residual.is_finite()
                        || evidence.angular_scaling_relative_residual < 0.0
                        || !evidence.frequency_spacing_relative_residual.is_finite()
                        || evidence.frequency_spacing_relative_residual < 0.0)
                {
                    return Err(QnmSequenceValidationError::InvalidFamilyEvidence);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequence(classification: QnmFamilyClassification) -> KerrQnmSequenceFixture {
        let provenance = FixtureProvenance {
            engine: "bhpt-qnm".to_owned(),
            engine_version: "0.4.1".to_owned(),
            source_revision: Some("pinned-source".to_owned()),
            producer_artifact_sha256: Some("b".repeat(64)),
            generated_at_utc: "2026-11-09T00:00:00Z".to_owned(),
            precision_bits: 128,
        };
        let evidence = (classification == QnmFamilyClassification::Prolate
            || classification == QnmFamilyClassification::Oblate)
            .then_some(QnmFamilyEvidence {
                angular_scaling_ratio: ComplexValue { re: 0.99, im: 0.01 },
                expected_angular_scaling_limit: ComplexValue { re: 1.0, im: 0.0 },
                observed_frequency_spacing_m: ComplexValue {
                    re: 0.25,
                    im: -0.01,
                },
                predicted_frequency_spacing_m: ComplexValue { re: 0.25, im: 0.0 },
                angular_scaling_relative_residual: 0.02,
                frequency_spacing_relative_residual: 0.03,
                consecutive_modes_used: 3,
            });
        let modes = (0..3)
            .map(|position| QnmModeRecord {
                mode_lineage_id: format!("test-mode-lineage-{position}"),
                riemann_sheet_kind: QnmRiemannSheetKind::Conventional,
                riemann_sheet_id: "conventional".to_owned(),
                continuation_class: QnmContinuationClass::NotApplicable,
                continuation_class_label: None,
                frequency_cut_relation: QnmFrequencyCutRelation::OffCut,
                sequence_position: position,
                overtone_index: Some(position),
                omega_m: ComplexValue {
                    re: 0.5,
                    im: -0.08 - f64::from(position) * 0.25,
                },
                angular_eigenvalue: ComplexValue {
                    re: 3.0 + f64::from(position),
                    im: -0.1,
                },
                angular_branch_id: "continued-angular-branch".to_owned(),
                angular_continuation_path_id: None,
                solver_method: QnmSolverMethod::Leaver,
                method_applicability: QnmMethodApplicability::WithinDocumentedRegime,
                initial_guess: Some(ComplexValue { re: 0.5, im: -0.1 }),
                initial_guess_source: QnmInitialGuessSource::ExplicitIndependent,
                precision_bits: 128,
                residuals: Some(QnmModeResiduals {
                    angular_ode_relative: Some(1.0e-14),
                    radial_ode_relative: Some(1.0e-13),
                    continued_fraction_relative: Some(1.0e-15),
                    incidence_amplitude_relative: None,
                }),
                wkb_quantization_index: None,
                wkb_truncation_order: None,
                wkb_optimal_truncation_order: None,
                wkb_cycle_period_j0: None,
                wkb_cycle_period_j1: None,
                wkb_quantization_residual: None,
                wkb_truncation_error_estimate: None,
                wkb_cycle_id: None,
            })
            .collect();

        KerrQnmSequenceFixture {
            schema_version: QNM_SEQUENCE_SCHEMA_VERSION,
            sequence_id: "kerr-a03-s0-l2-m2".to_owned(),
            provenance,
            spin_over_mass: 0.3,
            spin_weight: 0,
            ell: 2,
            m: 2,
            angular_eigenvalue_convention: AngularEigenvalueConvention::SpinWeightedSpheroidalA,
            angular_eigenvalue_definition: None,
            fourier_convention: FourierConvention::EMinusIOmegaT,
            frequency_normalization: FrequencyNormalization::DimensionlessMOmega,
            family_classification: classification,
            normal_prolate_oscillator_level: None,
            family_evidence: evidence,
            modes,
        }
    }

    #[test]
    fn producer_defined_qnm_angular_eigenvalue_requires_a_definition() {
        let mut value = sequence(QnmFamilyClassification::Unresolved);
        value.angular_eigenvalue_convention = AngularEigenvalueConvention::Other;
        assert_eq!(
            value.validate(),
            Err(QnmSequenceValidationError::AngularEigenvalueConventionIncomplete)
        );
        value.angular_eigenvalue_definition =
            Some("producer-defined angular separation constant".to_owned());
        assert_eq!(value.validate(), Ok(()));
    }

    #[test]
    fn method_breakdown_is_preserved_as_metadata_not_silently_relabelled() {
        let mut value = sequence(QnmFamilyClassification::Unresolved);
        value.spin_over_mass = 0.999;
        value.modes[0].solver_method = QnmSolverMethod::PotentialPeakPadeWkb;
        value.modes[0].method_applicability = QnmMethodApplicability::KnownBreakdown;
        assert_eq!(value.validate(), Ok(()));
        assert_eq!(
            value.modes[0].method_applicability,
            QnmMethodApplicability::KnownBreakdown
        );
    }

    #[test]
    fn producer_defined_qnm_angular_eigenvalue_rejects_whitespace_definition() {
        let mut value = sequence(QnmFamilyClassification::Unresolved);
        value.angular_eigenvalue_convention = AngularEigenvalueConvention::Other;
        value.angular_eigenvalue_definition = Some("   ".to_owned());
        assert_eq!(
            value.validate(),
            Err(QnmSequenceValidationError::AngularEigenvalueConventionIncomplete)
        );
    }

    #[test]
    fn qnm_mode_identity_keeps_riemann_sheet_and_continuation_class_orthogonal_to_family() {
        let mut value = sequence(QnmFamilyClassification::Unresolved);
        value.modes[0].riemann_sheet_kind = QnmRiemannSheetKind::Unconventional;
        value.modes[0].riemann_sheet_id = "unconventional-lower-half-plane".to_owned();
        value.modes[0].continuation_class = QnmContinuationClass::ClassI;
        value.modes[0].frequency_cut_relation = QnmFrequencyCutRelation::NegativeRealSide;
        assert_eq!(value.validate(), Ok(()));
        assert_eq!(
            value.family_classification,
            QnmFamilyClassification::Unresolved
        );
    }

    #[test]
    fn qnm_mode_identity_rejects_missing_sheet_and_bad_named_class() {
        let mut missing_sheet = sequence(QnmFamilyClassification::Unresolved);
        missing_sheet.modes[0].riemann_sheet_id.clear();
        assert_eq!(
            missing_sheet.validate(),
            Err(QnmSequenceValidationError::InvalidSpectralIdentity(0))
        );

        let mut bad_named_class = sequence(QnmFamilyClassification::Unresolved);
        bad_named_class.modes[0].continuation_class = QnmContinuationClass::OtherNamed;
        bad_named_class.modes[0].continuation_class_label = None;
        assert_eq!(
            bad_named_class.validate(),
            Err(QnmSequenceValidationError::InvalidSpectralIdentity(0))
        );
    }

    #[test]
    fn unresolved_qnm_sequence_is_valid_without_a_forced_family_label() {
        assert_eq!(
            sequence(QnmFamilyClassification::Unresolved).validate(),
            Ok(())
        );
    }

    #[test]
    fn nonzero_spin_weight_does_not_force_an_unverified_prolate_oscillator_label() {
        let mut value = sequence(QnmFamilyClassification::Ambiguous);
        value.spin_weight = -2;
        assert_eq!(value.normal_prolate_oscillator_level, None);
        assert_eq!(value.validate(), Ok(()));
    }

    #[test]
    fn prolate_and_oblate_labels_require_multi_mode_evidence() {
        assert_eq!(
            sequence(QnmFamilyClassification::Prolate).validate(),
            Ok(())
        );
        assert_eq!(sequence(QnmFamilyClassification::Oblate).validate(), Ok(()));

        let mut missing = sequence(QnmFamilyClassification::Prolate);
        missing.family_evidence = None;
        assert_eq!(
            missing.validate(),
            Err(QnmSequenceValidationError::MissingFamilyEvidence)
        );
    }

    #[test]
    fn rejects_duplicate_overtone_labels_and_bad_residuals() {
        let mut duplicate = sequence(QnmFamilyClassification::Unresolved);
        duplicate.modes[1].overtone_index = Some(0);
        assert_eq!(
            duplicate.validate(),
            Err(QnmSequenceValidationError::DuplicateOvertoneIndex(0))
        );

        let mut invalid_residual = sequence(QnmFamilyClassification::Unresolved);
        invalid_residual.modes[0]
            .residuals
            .as_mut()
            .unwrap()
            .radial_ode_relative = Some(-1.0);
        assert_eq!(
            invalid_residual.validate(),
            Err(QnmSequenceValidationError::InvalidModeResiduals(0))
        );
    }

    #[test]
    fn initial_guess_source_cannot_claim_no_guess_when_a_value_is_present() {
        let mut missing = sequence(QnmFamilyClassification::Unresolved);
        missing.modes[0].initial_guess_source = QnmInitialGuessSource::NoInitialGuessRequired;
        assert_eq!(
            missing.validate(),
            Err(QnmSequenceValidationError::InconsistentInitialGuessMetadata(0))
        );

        let mut shared = sequence(QnmFamilyClassification::Unresolved);
        shared.modes[0].initial_guess_source = QnmInitialGuessSource::BhptInterpolationTable;
        assert_eq!(shared.validate(), Ok(()));

        let mut unknown = sequence(QnmFamilyClassification::Unresolved);
        unknown.modes[0].initial_guess_source = QnmInitialGuessSource::Unknown;
        assert_eq!(unknown.validate(), Ok(()));
    }

    #[test]
    fn qnm_solver_methods_preserve_interpolation_and_schwarzschild_only_methods() {
        let mut interpolation = sequence(QnmFamilyClassification::Unresolved);
        interpolation.modes[0].solver_method = QnmSolverMethod::InterpolationFromPythonQnmCache;
        interpolation.modes[0].initial_guess = None;
        interpolation.modes[0].initial_guess_source = QnmInitialGuessSource::PythonQnmCache;
        assert_eq!(interpolation.validate(), Ok(()));

        let mut asymptotic = sequence(QnmFamilyClassification::Unresolved);
        asymptotic.modes[0].solver_method = QnmSolverMethod::LargeNAsymptoticSchwarzschild;
        assert_eq!(asymptotic.validate(), Ok(()));
    }

    #[test]
    fn accepts_unlabeled_overtone_when_sequence_position_is_preserved() {
        let mut value = sequence(QnmFamilyClassification::Ambiguous);
        value.modes[1].overtone_index = None;
        assert_eq!(value.validate(), Ok(()));
    }

    #[test]
    fn genus_one_prolate_wkb_preserves_quantization_index_separately_from_overtone() {
        let mut value = sequence(QnmFamilyClassification::Prolate);
        value.spin_weight = 0;
        value.normal_prolate_oscillator_level = Some(0);
        let mode = &mut value.modes[0];
        mode.angular_continuation_path_id =
            Some("schwarzschild-to-negative-imaginary-c-one-sided".to_owned());
        mode.solver_method = QnmSolverMethod::KerrNormalProlateGenusOneWkb;
        mode.overtone_index = Some(7);
        mode.wkb_quantization_index = Some(120);
        mode.wkb_truncation_order = Some(47);
        mode.wkb_optimal_truncation_order = Some(47);
        mode.wkb_cycle_period_j0 = Some(ComplexValue { re: 1.2, im: -0.3 });
        mode.wkb_cycle_period_j1 = Some(ComplexValue { re: 0.7, im: 0.1 });
        mode.wkb_quantization_residual = Some(1.0e-12);
        mode.wkb_truncation_error_estimate = Some(1.0e-8);
        mode.wkb_cycle_id = Some("outer-scattering-cycle".to_owned());
        assert_eq!(mode.overtone_index, Some(7));
        assert_eq!(mode.wkb_quantization_index, Some(120));
        assert_eq!(value.validate(), Ok(()));
    }

    #[test]
    fn genus_one_wkb_rejects_oblate_or_missing_cycle_metadata() {
        let mut wrong_family = sequence(QnmFamilyClassification::Oblate);
        wrong_family.modes[0].solver_method = QnmSolverMethod::KerrNormalProlateGenusOneWkb;
        wrong_family.modes[0].wkb_quantization_index = Some(120);
        wrong_family.modes[0].wkb_truncation_order = Some(47);
        wrong_family.modes[0].angular_continuation_path_id = Some("path".to_owned());
        wrong_family.modes[0].wkb_cycle_period_j0 = Some(ComplexValue { re: 1.0, im: 0.0 });
        wrong_family.modes[0].wkb_cycle_period_j1 = Some(ComplexValue { re: 1.0, im: 0.0 });
        wrong_family.modes[0].wkb_quantization_residual = Some(1.0e-12);
        wrong_family.modes[0].wkb_truncation_error_estimate = Some(1.0e-8);
        wrong_family.modes[0].wkb_cycle_id = Some("outer".to_owned());
        assert_eq!(
            wrong_family.validate(),
            Err(QnmSequenceValidationError::MissingWkbMetadata(0))
        );

        let mut missing_cycle = sequence(QnmFamilyClassification::Prolate);
        missing_cycle.spin_weight = 0;
        missing_cycle.normal_prolate_oscillator_level = Some(0);
        missing_cycle.modes[0].solver_method = QnmSolverMethod::KerrNormalProlateGenusOneWkb;
        missing_cycle.modes[0].wkb_quantization_index = Some(120);
        missing_cycle.modes[0].wkb_truncation_order = Some(47);
        missing_cycle.modes[0].angular_continuation_path_id = Some("path".to_owned());
        missing_cycle.modes[0].wkb_cycle_period_j0 = Some(ComplexValue { re: 1.0, im: 0.0 });
        missing_cycle.modes[0].wkb_cycle_period_j1 = Some(ComplexValue { re: 1.0, im: 0.0 });
        missing_cycle.modes[0].wkb_quantization_residual = Some(1.0e-12);
        missing_cycle.modes[0].wkb_truncation_error_estimate = Some(1.0e-8);
        assert_eq!(
            missing_cycle.validate(),
            Err(QnmSequenceValidationError::MissingWkbMetadata(0))
        );
    }

    #[test]
    fn genus_one_wkb_rejects_zero_or_negative_spin_domain() {
        let mut zero_spin = sequence(QnmFamilyClassification::Prolate);
        zero_spin.spin_over_mass = 0.0;
        zero_spin.spin_weight = 0;
        zero_spin.normal_prolate_oscillator_level = Some(0);
        zero_spin.modes[0].solver_method = QnmSolverMethod::KerrNormalProlateGenusOneWkb;
        assert_eq!(
            zero_spin.validate(),
            Err(QnmSequenceValidationError::InvalidWkbSpinDomain(0))
        );

        let mut negative_spin = zero_spin;
        negative_spin.spin_over_mass = -0.7;
        assert_eq!(
            negative_spin.validate(),
            Err(QnmSequenceValidationError::InvalidWkbSpinDomain(0))
        );
    }

    #[test]
    fn normal_prolate_wkb_small_spin_parameter_uses_quantization_index_n_not_overtone_n() {
        let mut value = sequence(QnmFamilyClassification::Prolate);
        value.spin_weight = 0;
        value.normal_prolate_oscillator_level = Some(0);
        {
            let mode = &mut value.modes[0];
            mode.solver_method = QnmSolverMethod::KerrNormalProlateGenusOneWkb;
            mode.wkb_quantization_index = Some(120);
            mode.overtone_index = Some(7);
        }
        let parameter = value
            .normal_prolate_wkb_small_spin_parameter(&value.modes[0])
            .unwrap();
        let expected = 0.3_f64.powf(-4.0 / 3.0) / 120.5;
        assert!((parameter - expected).abs() <= 4.0 * f64::EPSILON * expected);
        assert!(parameter < 1.0);
    }

    #[test]
    fn genus_one_wkb_requires_finite_periods_and_error_metadata() {
        let mut value = sequence(QnmFamilyClassification::Prolate);
        value.spin_weight = 0;
        value.normal_prolate_oscillator_level = Some(0);
        let mode = &mut value.modes[0];
        mode.solver_method = QnmSolverMethod::KerrNormalProlateGenusOneWkb;
        mode.angular_continuation_path_id = Some("branch-path".to_owned());
        mode.wkb_quantization_index = Some(120);
        mode.wkb_truncation_order = Some(47);
        mode.wkb_cycle_id = Some("outer".to_owned());
        mode.wkb_cycle_period_j0 = Some(ComplexValue {
            re: f64::NAN,
            im: 0.0,
        });
        mode.wkb_cycle_period_j1 = Some(ComplexValue { re: 1.0, im: 0.0 });
        mode.wkb_quantization_residual = Some(1.0e-12);
        mode.wkb_truncation_error_estimate = Some(1.0e-8);
        assert_eq!(
            value.validate(),
            Err(QnmSequenceValidationError::InvalidWkbMetadata(0))
        );
    }

    #[test]
    fn exact_extremal_wkb_fixture_is_scalar_only() {
        let mut scalar = sequence(QnmFamilyClassification::Unresolved);
        scalar.spin_over_mass = 1.0;
        for mode in &mut scalar.modes {
            mode.solver_method = QnmSolverMethod::ExactExtremalWkb;
        }
        assert_eq!(scalar.validate(), Ok(()));

        let mut gravitational = scalar.clone();
        gravitational.spin_weight = -2;
        assert_eq!(
            gravitational.validate(),
            Err(QnmSequenceValidationError::ExtremalFixtureRequiresScalarExactWkb)
        );
    }

    fn synthetic_analysis_config() -> QnmFamilyAnalysisConfig {
        QnmFamilyAnalysisConfig {
            minimum_consecutive_modes: 3,
            maximum_angular_residual: 5.0e-2,
            maximum_frequency_spacing_residual: 5.0e-2,
            maximum_angular_drift: 5.0e-2,
            maximum_frequency_spacing_drift: 5.0e-2,
            maximum_solver_residual: 1.0e-8,
        }
    }

    fn synthetic_asymptotic_sequence(family: QnmFamilyClassification) -> KerrQnmSequenceFixture {
        let mut value = sequence(QnmFamilyClassification::Unresolved);
        value.spin_over_mass = 0.7;
        value.sequence_id = "synthetic-kerr-a07-s0-l2-m1".to_owned();
        value.m = 1;
        value.normal_prolate_oscillator_level = Some(1);
        for (index, mode) in value.modes.iter_mut().enumerate() {
            mode.overtone_index = None;
            mode.omega_m = ComplexValue {
                re: 0.4,
                im: -0.2 - 0.5 * index as f64,
            };
            let angular_factor = match family {
                QnmFamilyClassification::Prolate => ComplexValue {
                    re: 0.0,
                    im: value.spin_over_mass,
                },
                QnmFamilyClassification::Oblate => ComplexValue {
                    re: -value.spin_over_mass.powi(2),
                    im: 0.0,
                },
                QnmFamilyClassification::Unresolved | QnmFamilyClassification::Ambiguous => {
                    unreachable!("synthetic helper requires a concrete family")
                }
            };
            let angular_target = match family {
                QnmFamilyClassification::Prolate => ComplexValue { re: 3.0, im: 0.0 },
                QnmFamilyClassification::Oblate => ComplexValue { re: 1.0, im: 0.0 },
                QnmFamilyClassification::Unresolved | QnmFamilyClassification::Ambiguous => {
                    unreachable!("synthetic helper requires a concrete family")
                }
            };
            let angular_base = match family {
                QnmFamilyClassification::Prolate => complex_mul(angular_factor, mode.omega_m),
                QnmFamilyClassification::Oblate => {
                    complex_mul(angular_factor, complex_mul(mode.omega_m, mode.omega_m))
                }
                QnmFamilyClassification::Unresolved | QnmFamilyClassification::Ambiguous => {
                    unreachable!("synthetic helper requires a concrete family")
                }
            };
            mode.angular_eigenvalue = complex_mul(angular_base, angular_target);
        }
        value
    }

    fn synthetic_family_targets() -> QnmFamilyAsymptoticTargets {
        QnmFamilyAsymptoticTargets {
            prolate_spacing_m: ComplexValue { re: 0.0, im: -0.5 },
            oblate_spacing_m: ComplexValue { re: 0.0, im: -0.5 },
        }
    }

    #[test]
    fn sequence_analysis_recomputes_prolate_family_without_trusting_overtone_labels() {
        let mut value = synthetic_asymptotic_sequence(QnmFamilyClassification::Prolate);
        let targets = synthetic_family_targets();
        let result =
            analyze_qnm_family_sequence(&value, targets, synthetic_analysis_config()).unwrap();
        assert_eq!(result.classification, QnmFamilyClassification::Prolate);
        assert!(result.prolate.is_some());
        assert!(result.oblate.is_none());

        // Traditional n labels are absent. The analyser uses sequence order
        // and the complex angular/frequency data, not the producer's labels.
        for mode in &mut value.modes {
            mode.overtone_index = None;
        }
        let without_overtone_labels =
            analyze_qnm_family_sequence(&value, targets, synthetic_analysis_config()).unwrap();
        assert_eq!(
            without_overtone_labels.classification,
            QnmFamilyClassification::Prolate
        );
    }

    #[test]
    fn sequence_analysis_recomputes_oblate_family_from_angular_scaling_and_spacing() {
        let value = synthetic_asymptotic_sequence(QnmFamilyClassification::Oblate);
        let result = analyze_qnm_family_sequence(
            &value,
            synthetic_family_targets(),
            synthetic_analysis_config(),
        )
        .unwrap();
        assert_eq!(result.classification, QnmFamilyClassification::Oblate);
        assert!(result.prolate.is_none());
        assert!(result.oblate.is_some());
    }

    #[test]
    fn sequence_analysis_abstains_on_wrong_spacing_or_discontinuous_angular_branch() {
        let mut value = synthetic_asymptotic_sequence(QnmFamilyClassification::Prolate);
        let mut wrong_target = synthetic_family_targets();
        wrong_target.prolate_spacing_m = ComplexValue { re: 0.0, im: -0.3 };
        let wrong_spacing =
            analyze_qnm_family_sequence(&value, wrong_target, synthetic_analysis_config()).unwrap();
        assert_eq!(
            wrong_spacing.classification,
            QnmFamilyClassification::Unresolved
        );

        value.modes[1].angular_branch_id = "branch-after-crossing".to_owned();
        let changed_branch = analyze_qnm_family_sequence(
            &value,
            synthetic_family_targets(),
            synthetic_analysis_config(),
        )
        .unwrap();
        assert_eq!(
            changed_branch.classification,
            QnmFamilyClassification::Unresolved
        );
        assert!(changed_branch.reason.is_some());
    }

    #[test]
    fn sequence_analysis_abstains_for_method_breakdown_and_unsupported_angular_convention() {
        let mut breakdown = synthetic_asymptotic_sequence(QnmFamilyClassification::Prolate);
        breakdown.modes[1].method_applicability = QnmMethodApplicability::KnownBreakdown;
        let result = analyze_qnm_family_sequence(
            &breakdown,
            synthetic_family_targets(),
            synthetic_analysis_config(),
        )
        .unwrap();
        assert_eq!(result.classification, QnmFamilyClassification::Unresolved);

        let mut unsupported = synthetic_asymptotic_sequence(QnmFamilyClassification::Prolate);
        unsupported.angular_eigenvalue_convention =
            AngularEigenvalueConvention::TeukolskyRadialLambda;
        assert_eq!(
            analyze_qnm_family_sequence(
                &unsupported,
                synthetic_family_targets(),
                synthetic_analysis_config()
            ),
            Err(QnmFamilyAnalysisError::UnsupportedAngularEigenvalueConvention)
        );
    }

    #[test]
    fn sequence_analysis_reports_ambiguity_when_both_models_pass_declared_tolerances() {
        let value = synthetic_asymptotic_sequence(QnmFamilyClassification::Prolate);
        let config = QnmFamilyAnalysisConfig {
            maximum_angular_residual: 1.0e6,
            maximum_frequency_spacing_residual: 1.0e6,
            maximum_angular_drift: 1.0e6,
            maximum_frequency_spacing_drift: 1.0e6,
            ..synthetic_analysis_config()
        };
        let result =
            analyze_qnm_family_sequence(&value, synthetic_family_targets(), config).unwrap();
        assert_eq!(result.classification, QnmFamilyClassification::Ambiguous);
        assert!(result.prolate.is_some());
        assert!(result.oblate.is_some());
    }

    #[test]
    fn serde_roundtrip_preserves_sequence_and_family_evidence() {
        let original = sequence(QnmFamilyClassification::Prolate);
        let encoded = serde_json::to_string(&original).unwrap();
        let decoded: KerrQnmSequenceFixture = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, original);
        decoded.validate().unwrap();
    }
}
