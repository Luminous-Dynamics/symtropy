//! Spin-continuation traces for Kerr QNM identity and extremal limits.
//!
//! Each trace follows one analytic mode lineage on one declared Riemann sheet.
//! A branch-cut crossing starts a distinct trace linked to its parent so the
//! sheet transition is never silently folded into an overtone-index change.

use serde::{Deserialize, Serialize};

use crate::{
    ComplexValue, FixtureProvenance, FourierConvention,
    qnm::{
        QnmContinuationClass, QnmFrequencyCutRelation, QnmMethodApplicability, QnmModeResiduals,
        QnmRiemannSheetKind, QnmSolverMethod,
    },
};

pub const QNM_CONTINUATION_TRACE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmExtremalDampingBehavior {
    FiniteDamping,
    ZeroDamping,
    Unresolved,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KerrQnmSpinSample {
    /// Signed dimensionless Kerr spin a/M; exact extremality is represented
    /// by |a/M| == 1, not by a near-extremal extrapolation.
    pub spin_over_mass: f64,
    pub omega_m: ComplexValue,
    pub angular_eigenvalue: ComplexValue,
    pub angular_branch_id: String,
    pub angular_continuation_path_id: String,
    pub solver_method: QnmSolverMethod,
    pub method_applicability: QnmMethodApplicability,
    pub precision_bits: u16,
    pub residuals: QnmModeResiduals,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtremalLimitEvidence {
    /// Subextremal interval used for the limiting fit. The exact a/M=1
    /// sample is stored separately and is not included in this fit window.
    pub fit_window_min_abs_spin: f64,
    pub fit_window_max_abs_spin: f64,
    pub fit_sample_count: u32,
    pub estimated_damping_limit: f64,
    pub damping_limit_uncertainty: f64,
    pub damping_fit_residual: f64,
    pub damping_vs_surface_gravity_exponent: Option<f64>,
    /// Exact-extremal M omega - m/2 in the declared Fourier convention.
    pub exact_extremal_superradiant_detuning: ComplexValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KerrQnmSpinContinuationTrace {
    pub schema_version: u32,
    pub trace_id: String,
    pub mode_lineage_id: String,
    /// A branch-cut crossing is represented by a distinct trace linked to
    /// its parent; this preserves the sheet transition as an explicit event.
    pub continuation_from_trace_id: Option<String>,
    pub provenance: FixtureProvenance,
    pub spin_weight: i32,
    pub ell: u32,
    pub m: i32,
    pub fourier_convention: FourierConvention,
    pub riemann_sheet_kind: QnmRiemannSheetKind,
    pub riemann_sheet_id: String,
    pub continuation_class: QnmContinuationClass,
    pub continuation_class_label: Option<String>,
    pub frequency_cut_relation: QnmFrequencyCutRelation,
    pub extremal_damping_behavior: QnmExtremalDampingBehavior,
    pub extremal_limit_evidence: Option<ExtremalLimitEvidence>,
    /// Samples are ordered monotonically in signed a/M. At least one exact
    /// extremal endpoint is required for a finite/zero-damping claim.
    pub samples: Vec<KerrQnmSpinSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QnmContinuationValidationError {
    UnsupportedSchemaVersion(u32),
    EmptyField(&'static str),
    InvalidSha256,
    InvalidPrecision,
    InvalidModeNumbers,
    EmptySamples,
    InvalidSpinSample(usize),
    ExactExtremalSampleRequiresHeunD,
    SpinSamplesNotStrictlyMonotonic(usize),
    AngularBranchChanged(usize),
    InvalidSpectralIdentity,
    MissingExactExtremalSample,
    MissingExtremalLimitEvidence,
    InvalidExtremalLimitEvidence,
    ExtremalLimitEvidenceWithoutExactSample,
}

impl core::fmt::Display for QnmContinuationValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(
                    f,
                    "unsupported Kerr QNM continuation schema version {version}"
                )
            }
            Self::EmptyField(name) => write!(f, "required continuation field {name} is empty"),
            Self::InvalidSha256 => write!(
                f,
                "producer artifact SHA-256 must contain 64 hex characters"
            ),
            Self::InvalidPrecision => {
                write!(
                    f,
                    "continuation and sample precision must be at least 53 bits"
                )
            }
            Self::InvalidModeNumbers => {
                write!(f, "mode numbers must satisfy ell >= |s| and |m| <= ell")
            }
            Self::EmptySamples => write!(f, "at least two spin-continuation samples are required"),
            Self::InvalidSpinSample(index) => {
                write!(
                    f,
                    "spin-continuation sample {index} has invalid values or residuals"
                )
            }
            Self::ExactExtremalSampleRequiresHeunD => write!(
                f,
                "gravitational exact-extremal samples require the double-confluent Heun method"
            ),
            Self::SpinSamplesNotStrictlyMonotonic(index) => {
                write!(
                    f,
                    "spin-continuation sample {index} reverses or duplicates the spin path"
                )
            }
            Self::AngularBranchChanged(index) => {
                write!(
                    f,
                    "angular branch or continuation path changes at sample {index}"
                )
            }
            Self::InvalidSpectralIdentity => {
                write!(
                    f,
                    "Riemann-sheet or continuation-class identity is incomplete"
                )
            }
            Self::MissingExactExtremalSample => {
                write!(
                    f,
                    "a finite/zero-damping classification requires an exact |a/M|=1 sample"
                )
            }
            Self::MissingExtremalLimitEvidence => {
                write!(
                    f,
                    "finite/zero-damping classification requires a subextremal limit-fit record"
                )
            }
            Self::InvalidExtremalLimitEvidence => {
                write!(
                    f,
                    "extremal limit evidence has invalid spin window, uncertainty, or fit residual"
                )
            }
            Self::ExtremalLimitEvidenceWithoutExactSample => {
                write!(
                    f,
                    "exact-extremal limit evidence was supplied without an exact extremal sample"
                )
            }
        }
    }
}

impl std::error::Error for QnmContinuationValidationError {}

/// Compute dimensionless detuning from the signed extremal superradiant
/// bound M omega_SR = sign(a) m/2. For a/M=+1 this is m/2; for a/M=-1
/// it is -m/2. The signed convention matters for counterrotating modes.
pub fn superradiant_bound_detuning(
    omega_m: ComplexValue,
    m: i32,
    extremal_spin_over_mass: f64,
) -> ComplexValue {
    let spin_orientation = extremal_spin_over_mass.signum();
    ComplexValue {
        re: omega_m.re - 0.5 * spin_orientation * f64::from(m),
        im: omega_m.im,
    }
}

impl KerrQnmSpinContinuationTrace {
    pub fn validate(&self) -> Result<(), QnmContinuationValidationError> {
        if self.schema_version != QNM_CONTINUATION_TRACE_SCHEMA_VERSION {
            return Err(QnmContinuationValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        for (value, name) in [
            (self.trace_id.as_str(), "trace_id"),
            (self.mode_lineage_id.as_str(), "mode_lineage_id"),
            (self.provenance.engine.as_str(), "provenance.engine"),
            (
                self.provenance.engine_version.as_str(),
                "provenance.engine_version",
            ),
            (
                self.provenance.generated_at_utc.as_str(),
                "provenance.generated_at_utc",
            ),
            (self.riemann_sheet_id.as_str(), "riemann_sheet_id"),
        ] {
            if value.trim().is_empty() {
                return Err(QnmContinuationValidationError::EmptyField(name));
            }
        }
        if self.provenance.precision_bits < 53 {
            return Err(QnmContinuationValidationError::InvalidPrecision);
        }
        if let Some(digest) = &self.provenance.producer_artifact_sha256
            && (digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(QnmContinuationValidationError::InvalidSha256);
        }
        if self.spin_weight.unsigned_abs() > self.ell || self.m.unsigned_abs() > self.ell {
            return Err(QnmContinuationValidationError::InvalidModeNumbers);
        }
        if self.continuation_class == QnmContinuationClass::OtherNamed
            && self
                .continuation_class_label
                .as_deref()
                .is_none_or(str::is_empty)
        {
            return Err(QnmContinuationValidationError::InvalidSpectralIdentity);
        }
        if self.continuation_class != QnmContinuationClass::OtherNamed
            && self.continuation_class_label.is_some()
        {
            return Err(QnmContinuationValidationError::InvalidSpectralIdentity);
        }
        if self.riemann_sheet_kind == QnmRiemannSheetKind::Unresolved
            && self.riemann_sheet_id != "unresolved"
        {
            return Err(QnmContinuationValidationError::InvalidSpectralIdentity);
        }
        if self.samples.len() < 2 {
            return Err(QnmContinuationValidationError::EmptySamples);
        }

        let mut direction = 0.0_f64;
        let first_branch = &self.samples[0].angular_branch_id;
        let first_path = &self.samples[0].angular_continuation_path_id;
        for (index, sample) in self.samples.iter().enumerate() {
            if !sample.spin_over_mass.is_finite()
                || sample.spin_over_mass.abs() > 1.0
                || !sample.omega_m.is_finite()
                || !sample.angular_eigenvalue.is_finite()
                || sample.angular_branch_id.trim().is_empty()
                || sample.angular_continuation_path_id.trim().is_empty()
                || sample.precision_bits < 53
                || sample.residuals.angular_ode_relative.is_none()
                || !sample.residuals.validate()
                || sample.method_applicability == QnmMethodApplicability::KnownBreakdown
            {
                return Err(QnmContinuationValidationError::InvalidSpinSample(index));
            }
            if sample.spin_over_mass.abs() == 1.0
                && self.spin_weight != 0
                && sample.solver_method != QnmSolverMethod::ExactExtremalDoubleConfluentHeun
            {
                return Err(QnmContinuationValidationError::ExactExtremalSampleRequiresHeunD);
            }
            if sample.angular_branch_id != *first_branch
                || sample.angular_continuation_path_id != *first_path
            {
                return Err(QnmContinuationValidationError::AngularBranchChanged(index));
            }
            if index > 0 {
                let difference = sample.spin_over_mass - self.samples[index - 1].spin_over_mass;
                if difference == 0.0 || (direction != 0.0 && difference.signum() != direction) {
                    return Err(
                        QnmContinuationValidationError::SpinSamplesNotStrictlyMonotonic(index),
                    );
                }
                direction = difference.signum();
            }
        }

        let has_exact_extremal = self
            .samples
            .iter()
            .any(|sample| sample.spin_over_mass.abs() == 1.0);
        match self.extremal_damping_behavior {
            QnmExtremalDampingBehavior::FiniteDamping | QnmExtremalDampingBehavior::ZeroDamping => {
                if !has_exact_extremal {
                    return Err(QnmContinuationValidationError::MissingExactExtremalSample);
                }
                let evidence = self
                    .extremal_limit_evidence
                    .as_ref()
                    .ok_or(QnmContinuationValidationError::MissingExtremalLimitEvidence)?;
                let fit_samples = self
                    .samples
                    .iter()
                    .filter(|sample| {
                        let spin = sample.spin_over_mass.abs();
                        spin >= evidence.fit_window_min_abs_spin
                            && spin <= evidence.fit_window_max_abs_spin
                    })
                    .count();
                let exact_sample = self
                    .samples
                    .iter()
                    .find(|sample| sample.spin_over_mass.abs() == 1.0)
                    .ok_or(QnmContinuationValidationError::MissingExactExtremalSample)?;
                let expected_detuning = superradiant_bound_detuning(
                    exact_sample.omega_m,
                    self.m,
                    exact_sample.spin_over_mass,
                );
                let detuning_error = ComplexValue {
                    re: evidence.exact_extremal_superradiant_detuning.re - expected_detuning.re,
                    im: evidence.exact_extremal_superradiant_detuning.im - expected_detuning.im,
                };
                let detuning_tolerance =
                    1.0e-12 * (1.0 + expected_detuning.re.hypot(expected_detuning.im));
                if !evidence.fit_window_min_abs_spin.is_finite()
                    || !evidence.fit_window_max_abs_spin.is_finite()
                    || evidence.fit_window_min_abs_spin < 0.0
                    || evidence.fit_window_max_abs_spin <= evidence.fit_window_min_abs_spin
                    || evidence.fit_window_max_abs_spin >= 1.0
                    || evidence.fit_sample_count < 3
                    || fit_samples < evidence.fit_sample_count as usize
                    || !evidence.estimated_damping_limit.is_finite()
                    || evidence.estimated_damping_limit < 0.0
                    || !evidence.damping_limit_uncertainty.is_finite()
                    || evidence.damping_limit_uncertainty < 0.0
                    || !evidence.damping_fit_residual.is_finite()
                    || evidence.damping_fit_residual < 0.0
                    || evidence
                        .damping_vs_surface_gravity_exponent
                        .is_some_and(|value| !value.is_finite() || value < 0.0)
                    || !evidence.exact_extremal_superradiant_detuning.is_finite()
                    || detuning_error.re.hypot(detuning_error.im) > detuning_tolerance
                {
                    return Err(QnmContinuationValidationError::InvalidExtremalLimitEvidence);
                }
            }
            QnmExtremalDampingBehavior::Unresolved | QnmExtremalDampingBehavior::Ambiguous => {
                if self.extremal_limit_evidence.is_some() && !has_exact_extremal {
                    return Err(
                        QnmContinuationValidationError::ExtremalLimitEvidenceWithoutExactSample,
                    );
                }
            }
        }

        Ok(())
    }
}

/// Numerical controls for classifying a single spin-continuation trace.
/// The exponent windows are user-declared decision thresholds, not universal
/// physical constants. They must be included in every reported analysis.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmExtremalDampingAnalysisConfig {
    pub fit_samples: usize,
    pub minimum_log_surface_gravity_span: f64,
    pub zero_damping_min_exponent: f64,
    pub finite_damping_max_exponent: f64,
    pub minimum_finite_damping_rate: f64,
    pub maximum_log_fit_rms: f64,
}

impl Default for QnmExtremalDampingAnalysisConfig {
    fn default() -> Self {
        Self {
            fit_samples: 4,
            minimum_log_surface_gravity_span: 1.0,
            zero_damping_min_exponent: 0.7,
            finite_damping_max_exponent: 0.25,
            minimum_finite_damping_rate: 1.0e-5,
            maximum_log_fit_rms: 0.2,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QnmExtremalDampingAnalysis {
    pub classification: QnmExtremalDampingBehavior,
    pub observed_damping_vs_surface_gravity_exponent: Option<f64>,
    pub log_fit_rms: Option<f64>,
    pub log_surface_gravity_span: Option<f64>,
    pub exponent_standard_error: Option<f64>,
    pub exact_extremal_damping_rate: Option<f64>,
    pub closest_subextremal_damping_rate: Option<f64>,
    pub fit_samples_used: usize,
    pub producer_label_matches_analysis: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QnmExtremalDampingAnalysisError {
    InvalidTrace(QnmContinuationValidationError),
    InvalidConfiguration,
}

impl core::fmt::Display for QnmExtremalDampingAnalysisError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidTrace(error) => write!(f, "invalid QNM continuation trace: {error}"),
            Self::InvalidConfiguration => write!(
                f,
                "damping-analysis thresholds must be finite, nonnegative, ordered, and use at least three fit samples"
            ),
        }
    }
}

impl std::error::Error for QnmExtremalDampingAnalysisError {}

fn dimensionless_surface_gravity(spin_over_mass: f64) -> Option<f64> {
    let spin = spin_over_mass.abs();
    if !spin.is_finite() || spin >= 1.0 {
        return None;
    }
    let discriminant = ((1.0 - spin) * (1.0 + spin)).sqrt();
    let value = discriminant / (2.0 * (1.0 + discriminant));
    (value.is_finite() && value > 0.0).then_some(value)
}

fn dimensionless_damping_rate(
    convention: FourierConvention,
    omega_m: ComplexValue,
) -> f64 {
    match convention {
        FourierConvention::EMinusIOmegaT => -omega_m.im,
        FourierConvention::EPlusIOmegaT => omega_m.im,
    }
}

/// Fit log(damping) = intercept + exponent * log(surface gravity).
/// A slope near one indicates damping that vanishes with surface gravity;
/// a slope near zero indicates a finite damping limit. This local classifier
/// is intentionally separate from the analytic sheet and family labels.
pub fn analyze_extremal_damping(
    trace: &KerrQnmSpinContinuationTrace,
    config: QnmExtremalDampingAnalysisConfig,
) -> Result<QnmExtremalDampingAnalysis, QnmExtremalDampingAnalysisError> {
    trace
        .validate()
        .map_err(QnmExtremalDampingAnalysisError::InvalidTrace)?;

    if config.fit_samples < 3
        || !config.minimum_log_surface_gravity_span.is_finite()
        || config.minimum_log_surface_gravity_span <= 0.0
        || !config.zero_damping_min_exponent.is_finite()
        || !config.finite_damping_max_exponent.is_finite()
        || config.zero_damping_min_exponent <= config.finite_damping_max_exponent
        || config.finite_damping_max_exponent < 0.0
        || !config.minimum_finite_damping_rate.is_finite()
        || config.minimum_finite_damping_rate <= 0.0
        || !config.maximum_log_fit_rms.is_finite()
        || config.maximum_log_fit_rms <= 0.0
    {
        return Err(QnmExtremalDampingAnalysisError::InvalidConfiguration);
    }

    let exact_sample = trace
        .samples
        .iter()
        .find(|sample| sample.spin_over_mass.abs() == 1.0);
    let Some(exact_sample) = exact_sample else {
        return Ok(QnmExtremalDampingAnalysis {
            classification: QnmExtremalDampingBehavior::Unresolved,
            observed_damping_vs_surface_gravity_exponent: None,
            log_fit_rms: None,
            log_surface_gravity_span: None,
            exponent_standard_error: None,
            exact_extremal_damping_rate: None,
            closest_subextremal_damping_rate: None,
            fit_samples_used: 0,
            producer_label_matches_analysis: false,
        });
    };

    let exact_damping = dimensionless_damping_rate(trace.fourier_convention, exact_sample.omega_m);
    if !exact_damping.is_finite() || exact_damping < 0.0 {
        return Ok(QnmExtremalDampingAnalysis {
            classification: QnmExtremalDampingBehavior::Unresolved,
            observed_damping_vs_surface_gravity_exponent: None,
            log_fit_rms: None,
            log_surface_gravity_span: None,
            exponent_standard_error: None,
            exact_extremal_damping_rate: Some(exact_damping),
            closest_subextremal_damping_rate: None,
            fit_samples_used: 0,
            producer_label_matches_analysis: false,
        });
    }

    let mut subextremal = trace
        .samples
        .iter()
        .filter(|sample| sample.spin_over_mass.abs() < 1.0)
        .collect::<Vec<_>>();
    subextremal.sort_by(|left, right| {
        right
            .spin_over_mass
            .abs()
            .total_cmp(&left.spin_over_mass.abs())
    });
    subextremal.truncate(config.fit_samples);

    if subextremal.len() < config.fit_samples {
        return Ok(QnmExtremalDampingAnalysis {
            classification: QnmExtremalDampingBehavior::Unresolved,
            observed_damping_vs_surface_gravity_exponent: None,
            log_fit_rms: None,
            log_surface_gravity_span: None,
            exponent_standard_error: None,
            exact_extremal_damping_rate: Some(exact_damping),
            closest_subextremal_damping_rate: None,
            fit_samples_used: subextremal.len(),
            producer_label_matches_analysis: false,
        });
    }

    let mut points = Vec::with_capacity(subextremal.len());
    for sample in &subextremal {
        let Some(surface_gravity) = dimensionless_surface_gravity(sample.spin_over_mass) else {
            return Ok(QnmExtremalDampingAnalysis {
                classification: QnmExtremalDampingBehavior::Unresolved,
                observed_damping_vs_surface_gravity_exponent: None,
                log_fit_rms: None,
                log_surface_gravity_span: None,
                exponent_standard_error: None,
                exact_extremal_damping_rate: Some(exact_damping),
                closest_subextremal_damping_rate: None,
                fit_samples_used: subextremal.len(),
                producer_label_matches_analysis: false,
            });
        };
        let damping = dimensionless_damping_rate(trace.fourier_convention, sample.omega_m);
        if !damping.is_finite() || damping <= 0.0 {
            return Ok(QnmExtremalDampingAnalysis {
                classification: QnmExtremalDampingBehavior::Unresolved,
                observed_damping_vs_surface_gravity_exponent: None,
                log_fit_rms: None,
                log_surface_gravity_span: None,
                exponent_standard_error: None,
                exact_extremal_damping_rate: Some(exact_damping),
                closest_subextremal_damping_rate: Some(damping),
                fit_samples_used: subextremal.len(),
                producer_label_matches_analysis: false,
            });
        }
        points.push((surface_gravity.ln(), damping.ln()));
    }

    let count = points.len() as f64;
    let mean_x = points.iter().map(|point| point.0).sum::<f64>() / count;
    let mean_y = points.iter().map(|point| point.1).sum::<f64>() / count;
    let variance_x = points
        .iter()
        .map(|point| (point.0 - mean_x).powi(2))
        .sum::<f64>();
    if !variance_x.is_finite() || variance_x <= f64::MIN_POSITIVE {
        return Ok(QnmExtremalDampingAnalysis {
            classification: QnmExtremalDampingBehavior::Unresolved,
            observed_damping_vs_surface_gravity_exponent: None,
            log_fit_rms: None,
            log_surface_gravity_span: None,
            exponent_standard_error: None,
            exact_extremal_damping_rate: Some(exact_damping),
            closest_subextremal_damping_rate: None,
            fit_samples_used: points.len(),
            producer_label_matches_analysis: false,
        });
    }
    let covariance = points
        .iter()
        .map(|point| (point.0 - mean_x) * (point.1 - mean_y))
        .sum::<f64>();
    let exponent = covariance / variance_x;
    let intercept = mean_y - exponent * mean_x;
    let squared_log_residuals = points
        .iter()
        .map(|point| (point.1 - (intercept + exponent * point.0)).powi(2))
        .sum::<f64>();
    let log_fit_rms = (squared_log_residuals / count).sqrt();
    let log_surface_gravity_span = points
        .iter()
        .map(|point| point.0)
        .fold(f64::NEG_INFINITY, f64::max)
        - points
            .iter()
            .map(|point| point.0)
            .fold(f64::INFINITY, f64::min);
    let exponent_standard_error = (points.len() > 2)
        .then(|| (squared_log_residuals / (count - 2.0) / variance_x).sqrt());
    let closest_subextremal_damping = Some(dimensionless_damping_rate(
        trace.fourier_convention,
        subextremal[0].omega_m,
    ));

    let classification = if !exponent.is_finite()
        || !log_fit_rms.is_finite()
        || !log_surface_gravity_span.is_finite()
        || log_surface_gravity_span < config.minimum_log_surface_gravity_span
        || log_fit_rms > config.maximum_log_fit_rms
    {
        QnmExtremalDampingBehavior::Ambiguous
    } else if exponent >= config.zero_damping_min_exponent
        && exact_damping <= config.minimum_finite_damping_rate
    {
        QnmExtremalDampingBehavior::ZeroDamping
    } else if exponent <= config.finite_damping_max_exponent
        && exact_damping > config.minimum_finite_damping_rate
    {
        QnmExtremalDampingBehavior::FiniteDamping
    } else {
        QnmExtremalDampingBehavior::Ambiguous
    };

    Ok(QnmExtremalDampingAnalysis {
        classification,
        observed_damping_vs_surface_gravity_exponent: Some(exponent),
        log_fit_rms: Some(log_fit_rms),
        log_surface_gravity_span: Some(log_surface_gravity_span),
        exponent_standard_error,
        exact_extremal_damping_rate: Some(exact_damping),
        closest_subextremal_damping_rate: closest_subextremal_damping,
        fit_samples_used: points.len(),
        producer_label_matches_analysis: classification == trace.extremal_damping_behavior,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(m: i32, damping: QnmExtremalDampingBehavior) -> KerrQnmSpinContinuationTrace {
        let provenance = FixtureProvenance {
            engine: "heun-d-reference".to_owned(),
            engine_version: "0.1.0".to_owned(),
            source_revision: Some("pinned-heun-d-revision".to_owned()),
            producer_artifact_sha256: Some("c".repeat(64)),
            generated_at_utc: "2026-11-09T00:00:00Z".to_owned(),
            precision_bits: 200,
        };
        let spins = [0.9_f64, 0.99, 0.999, 0.9999, 1.0];
        let samples = spins
            .iter()
            .enumerate()
            .map(|(index, spin)| {
                let damping_rate = match damping {
                    QnmExtremalDampingBehavior::ZeroDamping if index < 3 => {
                        0.25 * dimensionless_surface_gravity(*spin).unwrap()
                    }
                    QnmExtremalDampingBehavior::ZeroDamping => 0.0,
                    QnmExtremalDampingBehavior::FiniteDamping => 0.1,
                    QnmExtremalDampingBehavior::Unresolved
                    | QnmExtremalDampingBehavior::Ambiguous => {
                        if index == 3 { 0.0 } else { 0.1 }
                    }
                };
                KerrQnmSpinSample {
                spin_over_mass: *spin,
                omega_m: ComplexValue {
                    re: 0.5 * f64::from(m),
                    im: -damping_rate,
                },
                angular_eigenvalue: ComplexValue { re: 3.0, im: -0.1 },
                angular_branch_id: "angular-branch-A".to_owned(),
                angular_continuation_path_id: "spin-increasing".to_owned(),
                solver_method: if *spin == 1.0 {
                    QnmSolverMethod::ExactExtremalDoubleConfluentHeun
                } else {
                    QnmSolverMethod::Leaver
                },
                method_applicability: QnmMethodApplicability::WithinDocumentedRegime,
                precision_bits: 200,
                residuals: QnmModeResiduals {
                    angular_ode_relative: Some(1.0e-40),
                    radial_ode_relative: Some(1.0e-38),
                    continued_fraction_relative: None,
                    incidence_amplitude_relative: None,
                },
            }
            })
            .collect();
        let evidence = matches!(
            damping,
            QnmExtremalDampingBehavior::FiniteDamping | QnmExtremalDampingBehavior::ZeroDamping
        )
        .then_some(ExtremalLimitEvidence {
            fit_window_min_abs_spin: 0.9,
            fit_window_max_abs_spin: 0.9999,
            fit_sample_count: 4,
            estimated_damping_limit: if damping == QnmExtremalDampingBehavior::ZeroDamping {
                0.0
            } else {
                0.1
            },
            damping_limit_uncertainty: 1.0e-5,
            damping_fit_residual: 1.0e-8,
            damping_vs_surface_gravity_exponent: Some(0.5),
            exact_extremal_superradiant_detuning: ComplexValue { re: 0.0, im: 0.0 },
        });
        KerrQnmSpinContinuationTrace {
            schema_version: QNM_CONTINUATION_TRACE_SCHEMA_VERSION,
            trace_id: format!("trace-m{m}"),
            mode_lineage_id: format!("class-i-m{m}-unconventional"),
            continuation_from_trace_id: None,
            provenance,
            spin_weight: -2,
            ell: 2,
            m,
            fourier_convention: FourierConvention::EMinusIOmegaT,
            riemann_sheet_kind: QnmRiemannSheetKind::Unconventional,
            riemann_sheet_id: "unconventional-class-i".to_owned(),
            continuation_class: QnmContinuationClass::ClassI,
            continuation_class_label: None,
            frequency_cut_relation: QnmFrequencyCutRelation::NegativeRealSide,
            extremal_damping_behavior: damping,
            extremal_limit_evidence: evidence,
            samples,
        }
    }

    #[test]
    fn accepts_exact_extremal_negative_m_unconventional_zero_damping_trace() {
        let value = trace(-2, QnmExtremalDampingBehavior::ZeroDamping);
        assert_eq!(value.validate(), Ok(()));
        let exact = value.samples.last().unwrap().omega_m;
        assert_eq!(
            superradiant_bound_detuning(exact, value.m, 1.0),
            ComplexValue { re: 0.0, im: 0.0 }
        );
    }

    #[test]
    fn classifies_surface_gravity_scaling_for_zero_and_finite_damping() {
        let zero_trace = trace(-2, QnmExtremalDampingBehavior::ZeroDamping);
        let zero = analyze_extremal_damping(
            &zero_trace,
            QnmExtremalDampingAnalysisConfig::default(),
        )
        .unwrap();
        assert_eq!(zero.classification, QnmExtremalDampingBehavior::ZeroDamping);
        assert!((zero.observed_damping_vs_surface_gravity_exponent.unwrap() - 1.0).abs() < 1.0e-12);
        assert!(zero.producer_label_matches_analysis);

        let finite_trace = trace(2, QnmExtremalDampingBehavior::FiniteDamping);
        let finite = analyze_extremal_damping(
            &finite_trace,
            QnmExtremalDampingAnalysisConfig::default(),
        )
        .unwrap();
        assert_eq!(
            finite.classification,
            QnmExtremalDampingBehavior::FiniteDamping
        );
        assert!(finite.observed_damping_vs_surface_gravity_exponent.unwrap().abs() < 1.0e-12);
        assert!(finite.producer_label_matches_analysis);
    }

    #[test]
    fn leaves_intermediate_damping_scaling_ambiguous() {
        let mut value = trace(2, QnmExtremalDampingBehavior::ZeroDamping);
        for sample in value.samples.iter_mut().take(3) {
            let gravity = dimensionless_surface_gravity(sample.spin_over_mass).unwrap();
            sample.omega_m.im = -0.1 * gravity.sqrt();
        }
        let result = analyze_extremal_damping(
            &value,
            QnmExtremalDampingAnalysisConfig::default(),
        )
        .unwrap();
        assert_eq!(result.classification, QnmExtremalDampingBehavior::Ambiguous);
        assert!(!result.producer_label_matches_analysis);
    }

    #[test]
    fn requires_exact_extremal_sample_for_damping_claim() {
        let mut value = trace(2, QnmExtremalDampingBehavior::FiniteDamping);
        value.samples.pop();
        assert_eq!(
            value.validate(),
            Err(QnmContinuationValidationError::MissingExactExtremalSample)
        );
    }

    #[test]
    fn rejects_wrong_signed_superradiant_detuning() {
        let mut value = trace(-2, QnmExtremalDampingBehavior::ZeroDamping);
        value
            .extremal_limit_evidence
            .as_mut()
            .unwrap()
            .exact_extremal_superradiant_detuning = ComplexValue { re: 2.0, im: 0.0 };
        assert_eq!(
            value.validate(),
            Err(QnmContinuationValidationError::InvalidExtremalLimitEvidence)
        );
    }

    #[test]
    fn rejects_spin_path_reversal_and_angular_branch_changes() {
        let mut reversed = trace(2, QnmExtremalDampingBehavior::ZeroDamping);
        reversed.samples[2].spin_over_mass = 0.95;
        assert!(matches!(
            reversed.validate(),
            Err(QnmContinuationValidationError::SpinSamplesNotStrictlyMonotonic(_))
        ));

        let mut changed_branch = trace(2, QnmExtremalDampingBehavior::ZeroDamping);
        changed_branch.samples[2].angular_branch_id = "different-branch".to_owned();
        assert_eq!(
            changed_branch.validate(),
            Err(QnmContinuationValidationError::AngularBranchChanged(2))
        );
    }

    #[test]
    fn superradiant_bound_uses_signed_m() {
        let positive_spin_detuning = superradiant_bound_detuning(
            ComplexValue {
                re: -1.0,
                im: -1.0e-12,
            },
            -2,
            1.0,
        );
        assert_eq!(
            positive_spin_detuning,
            ComplexValue {
                re: 0.0,
                im: -1.0e-12,
            }
        );

        let negative_spin_detuning = superradiant_bound_detuning(
            ComplexValue {
                re: 1.0,
                im: -1.0e-12,
            },
            -2,
            -1.0,
        );
        assert_eq!(
            negative_spin_detuning,
            ComplexValue {
                re: 0.0,
                im: -1.0e-12,
            }
        );
    }
}
