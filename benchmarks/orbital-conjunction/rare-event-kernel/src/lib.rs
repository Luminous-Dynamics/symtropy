// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Small, deterministic mathematical kernels for the orbital rare-event
//! research programme.
//!
//! This crate deliberately has no orbit propagator, random-number generator,
//! proposal fitter, or operational collision logic. It provides:
//!
//! - strict positive-definite covariance validation;
//! - multivariate Gaussian log densities using a Cholesky factor;
//! - exact Gaussian half-space probabilities (reduced to one dimension);
//! - normalized Gaussian-mixture log densities using log-sum-exp;
//! - full-density importance log weights and weighted event indicators;
//! - a structural check for a strict covariance floor
//!   Sigma_component > alpha * Sigma_nominal;
//! - a budgeted affine-cell traversal that accounts for possible event mass
//!   outside caller-declared geometric chart boxes.
//!
//! These primitives do not establish event-mode completeness, finite-budget
//! confidence-interval coverage, physical-model adequacy, or the assumptions
//! of any particular importance-sampling theorem.

use core::fmt;

/// Outward-rounded affine geometry used by the independent coverage research.
pub mod adaptive_importance_sampling;
pub mod conditional_velocity;
pub mod coverage;
pub mod decision_gate;
pub mod decision_refinement;
pub mod lebedev;
pub mod ncdot_refinement;
pub mod ncdot_sphere;
pub mod ncdot_temporal;
pub mod normal_positive_part;
pub mod pop_col_adjustment;
pub mod pop_col_energy;
pub mod pop_convergence;
pub mod predictable_empirical_bernstein;
pub mod refinement_scheduler;
pub mod residual;
pub mod sequential;
pub mod space_time_partition;
pub mod stats;
pub mod trajectory;

pub use adaptive_importance_sampling::{
    AdaptiveImportanceDraw, AdaptiveImportanceSamplingConfidenceSequence,
    AdaptiveImportanceSamplingError, AdaptiveImportanceSamplingSummary, BoundedMeanBettingSequence,
    BoundedMeanConfidenceSequence,
};
pub use conditional_velocity::{
    ConditionalDirectionReport, ConditionalVelocityConfig, ConditionalVelocityDiagnostics,
    ConditionalVelocityDistribution, Matrix3, RelativePositionVelocityCovarianceBlocks,
    ZeroConditionalVarianceMode,
};
pub use decision_gate::{
    DecisionGateError, DecisionSufficiency, DecisionSufficiencyReport, RiskThresholdRule,
    assess_risk_threshold,
};
pub use decision_refinement::{
    DecisionRefinementDisposition, DecisionRefinementError, DecisionRefinementReport,
    plan_decision_refinement,
};
pub use lebedev::{
    AdaptiveSphereQuadratureConfig, AdaptiveSphereQuadratureReport, LebedevRule, LebedevSphereRule,
    SphereQuadratureDiagnosticsConfig, SphereQuadratureNode, SphereQuadratureReport,
    integrate_sphere_adaptive, integrate_sphere_samples, lebedev_sphere_rule,
};
pub use ncdot_refinement::{
    NcdotRefinementDecision, NcdotRefinementDisposition, NcdotRefinementPolicy,
    NcdotRefinementReason, NcdotRefinementSnapshot, decide_ncdot_refinement,
};
pub use ncdot_sphere::{
    NcdotSphereConfig, NcdotSphereDiagnosticsConfig, NcdotSphereInputs, NcdotSphereReport,
    integrate_ncdot_sphere,
};
pub use ncdot_temporal::{
    NcdotTemporalExtremum, NcdotTemporalExtremumKind, NcdotTemporalHorizon, NcdotTemporalIdentity,
    NcdotTemporalReport, NcdotTemporalSample, TemporalHorizonCoverageClaim,
    integrate_ncdot_time_series,
};
pub use normal_positive_part::{
    GaussianPositivePartBranch, GaussianPositivePartReport, gaussian_negative_part_expectation,
};
pub use pop_col_adjustment::{
    PopColAdjustmentConfig, PopColAdjustmentInput, PopColAdjustmentReport, PopColAdjustmentStatus,
    PopColFractionSearchReport, adjust_pop_col_energy_limited_center,
};
pub use pop_col_energy::{
    PopColEnergyInput, PopColEnergyReport, calculate_pop_col_energy, conditional_velocity_at_col,
};
pub use pop_convergence::{
    PeakOverlapConvergenceConfig, PeakOverlapConvergenceDecision,
    PeakOverlapConvergenceObservation, PeakOverlapConvergenceReason, PeakOverlapConvergenceStatus,
    decide_peak_overlap_convergence,
};
pub use predictable_empirical_bernstein::{
    AdaptiveImportanceSamplingPrPlEbSequence, DEFAULT_BET_FRACTION_CAP, PrPlEbConfidenceSequence,
    PrPlEbError,
};
pub use refinement_scheduler::{
    RankedRefinementCandidate, RefinementCandidate, RefinementCandidateId, RefinementScheduleError,
    RefinementScheduleReport, schedule_refinements,
};
pub use residual::{
    AffineCollisionChart, AffineCollisionComponent, AffineResidualMassConfig,
    AffineResidualMassReport, AffineUnionResidualMassConfig,
    affine_collision_residual_mass_upper_bound, affine_union_collision_residual_mass_upper_bound,
};
pub use sequential::{BernoulliConfidenceSequence, BernoulliMixtureEProcess};
pub use space_time_partition::{
    NcEnvelopeReport, NonnegativeRationalInterval, Rational, SpaceTimeCellContribution,
    SpaceTimeHorizon, SpaceTimePartition, SpaceTimePartitionError, SpaceTimeRateCell,
    SpaceTimeScope, SpherePatch, SpherePatchId, TimeSlab, TimeSlabId, verify_space_time_partition,
};
pub use stats::{WeightedEventAccumulator, WeightedEventSummary};
pub use trajectory::{
    TrajectoryContactClassification, TrajectoryContactReport, classify_polynomial_contact,
};

pub use coverage::{
    CellSide, CoverageCell, CoverageMassUpperBound, CoverageSplit, Interval,
    affine_box_image_bounds, affine_box_norm_lower_bound, affine_cell_excludes_collision_ball,
    standard_normal_box_probability_upper_bound, standard_normal_cube_tail_probability_upper_bound,
};

/// Errors returned by the analytic probability kernels.
#[derive(Debug, Clone, PartialEq)]
pub enum KernelError {
    /// A zero-dimensional vector or matrix was supplied.
    EmptyDimension,
    /// Two related vectors or matrices have incompatible dimensions.
    DimensionMismatch,
    /// A square matrix does not contain dimension squared entries.
    InvalidMatrixLength,
    /// An input contains NaN or an infinite value where it is not meaningful.
    NonFiniteValue,
    /// A covariance matrix is not symmetric within floating-point tolerance.
    NonSymmetricMatrix,
    /// A covariance matrix is not strictly positive definite.
    NotPositiveDefinite,
    /// A Gaussian-mixture component weight is not strictly positive and finite.
    InvalidMixtureWeight,
    /// Gaussian-mixture weights do not sum to one within the declared tolerance.
    MixtureWeightsNotNormalized,
    /// The covariance-floor fraction is negative or non-finite.
    InvalidCovarianceFloorFraction,
    /// An interval has its lower endpoint greater than its upper endpoint.
    InvalidInterval,
    /// A tail-bound radius is not strictly positive and finite.
    InvalidRadius,
    /// A hard-body collision radius is negative or non-finite.
    InvalidHardBodyRadius,
    /// A residual-mass traversal was configured with a zero cell budget.
    InvalidCoverageBudget,
    /// A declared chart box is not wholly contained in the finite search cube.
    CoverageChartOutsideDomain,
    /// A residual-mass event union contains no components.
    EmptyEventUnion,
    /// A chart refers to an event component outside the supplied union.
    InvalidChartComponentIndex,
    /// An interval-enclosed trajectory coordinate has no polynomial coefficients.
    InvalidPolynomial,
    /// A time-window classification was configured with a zero interval budget.
    InvalidTrajectoryBudget,
    /// A streaming sample or event-contribution counter would overflow.
    CounterOverflow,
    /// A probability is non-finite or outside the closed interval [0, 1].
    InvalidProbability,
    /// A sequential error probability is not strictly between zero and one.
    InvalidAlpha,
    /// A nonzero half-space normal has zero projected variance.
    ZeroProjectedVariance,
    /// A mathematically required intermediate exceeds the representable range.
    NumericalOverflow,
    /// The proposal log density is negative infinity, so the importance ratio
    /// is undefined at a point where a weight was requested.
    ProposalHasZeroDensity,
    /// A sphere quadrature rule has the wrong number of nodes.
    InvalidSphereQuadratureNodeCount,
    /// A sphere quadrature node is not on the unit sphere.
    InvalidSphereQuadratureNode,
    /// Sphere quadrature weights are nonpositive, non-finite or do not sum to 4π.
    InvalidSphereQuadratureWeights,
    /// A relative significant-contribution threshold lies outside [0, 1].
    InvalidSphereQuadratureThreshold,
    /// A sphere integrand contains a negative sample.
    InvalidSphereIntegrand,
    /// A sphere integrand is identically zero at all quadrature nodes.
    ZeroSphereIntegrand,
    /// Sphere quadrature absolute and relative tolerances are invalid.
    InvalidSphereQuadratureTolerance,
    /// A standard deviation for a one-dimensional Gaussian is negative.
    InvalidNormalScale,
    /// A covariance or directional-validation tolerance is negative or non-finite.
    InvalidCovarianceTolerance,
    /// A vector supplied as a sphere direction is not unit length.
    InvalidUnitDirection,
    /// An evidence identifier is blank or malformed.
    InvalidEvidenceIdentifier,
    /// An evidence digest is not canonical lowercase SHA-256 hexadecimal.
    InvalidEvidenceDigest,
    /// Event, model or target-distribution identities differ.
    EvidenceIdentityMismatch,
    /// A temporal horizon has invalid endpoints or maximum interval.
    InvalidTemporalHorizon,
    /// Fewer than two fixed-time reports were supplied for time integration.
    InsufficientTemporalSamples,
    /// Two temporal samples reuse the same stable sample identifier.
    DuplicateTemporalSample,
    /// Temporal samples use different Lebedev sphere rules.
    TemporalLebedevRuleMismatch,
    /// A temporal Ncdot report contains an invalid rate or contribution ledger.
    InvalidTemporalRate,
    /// The first or last temporal sample does not match the declared horizon.
    TemporalHorizonEndpointMismatch,
    /// Temporal epochs are duplicate or not strictly increasing.
    NonIncreasingTemporalEpochs,
    /// NASA-style Ncdot refinement thresholds or iteration policy are invalid.
    InvalidNcdotRefinementPolicy,
    /// A sampled Ncdot refinement snapshot is incomplete or internally inconsistent.
    InvalidNcdotRefinementSnapshot,
    /// A required temporal midpoint is not representable as a distinct f64 epoch.
    NcdotRefinementResolutionLimit,
    /// Central-body gravitational parameter must be finite and strictly positive.
    InvalidGravitationalParameter,
    /// CoL energy-change policy contains a negative or NaN tolerance.
    InvalidPopColAdjustmentPolicy,
    /// CoL energy-adjustment fraction or objective input is invalid.
    InvalidPopColAdjustmentInput,
    /// A CoL fraction midpoint cannot be represented distinctly in floating point.
    PopColAdjustmentResolutionLimit,
    /// PeakOverlapPos convergence policy contains invalid tolerances.
    InvalidPeakOverlapConvergenceConfig,
    /// PeakOverlapPos convergence observation is incomplete or inconsistent.
    InvalidPeakOverlapConvergenceObservation,
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyDimension => "dimension must be nonzero",
            Self::DimensionMismatch => "input dimensions do not match",
            Self::InvalidMatrixLength => "matrix must contain dimension squared entries",
            Self::NonFiniteValue => "input contains a non-finite value",
            Self::NonSymmetricMatrix => "matrix is not symmetric within tolerance",
            Self::NotPositiveDefinite => "matrix is not strictly positive definite",
            Self::InvalidMixtureWeight => "mixture weights must be strictly positive and finite",
            Self::MixtureWeightsNotNormalized => "mixture weights must sum to one",
            Self::InvalidCovarianceFloorFraction => {
                "covariance-floor fraction must be nonnegative and finite"
            }
            Self::InvalidInterval => "interval lower endpoint exceeds upper endpoint",
            Self::InvalidRadius => "radius must be strictly positive and finite",
            Self::InvalidHardBodyRadius => "hard-body radius must be nonnegative and finite",
            Self::InvalidCoverageBudget => "coverage traversal cell budget must be nonzero",
            Self::CoverageChartOutsideDomain => "coverage chart box lies outside the search cube",
            Self::EmptyEventUnion => {
                "residual-mass event union must contain at least one component"
            }
            Self::InvalidChartComponentIndex => "chart component index is outside the event union",
            Self::InvalidPolynomial => {
                "trajectory polynomial must contain at least one coefficient per axis"
            }
            Self::InvalidTrajectoryBudget => "trajectory time-interval budget must be nonzero",
            Self::CounterOverflow => "streaming observation counter overflowed",
            Self::InvalidProbability => "probability must be finite and lie in [0, 1]",
            Self::InvalidAlpha => "alpha must be finite and lie strictly between 0 and 1",
            Self::ZeroProjectedVariance => "half-space projection has zero variance",
            Self::NumericalOverflow => "numerical intermediate is outside the representable range",
            Self::ProposalHasZeroDensity => "proposal density is zero at the evaluation point",
            Self::InvalidSphereQuadratureNodeCount => {
                "sphere quadrature sample count does not match the rule"
            }
            Self::InvalidSphereQuadratureNode => "sphere quadrature node is not on the unit sphere",
            Self::InvalidSphereQuadratureWeights => {
                "sphere quadrature weights must be positive and sum to 4π"
            }
            Self::InvalidSphereQuadratureThreshold => {
                "sphere quadrature contribution threshold must lie in [0, 1]"
            }
            Self::InvalidSphereIntegrand => "sphere integrand samples must be nonnegative",
            Self::ZeroSphereIntegrand => "sphere integrand is zero at every quadrature node",
            Self::InvalidSphereQuadratureTolerance => {
                "sphere quadrature tolerances must be finite, nonnegative and not both zero"
            }
            Self::InvalidNormalScale => "normal standard deviation must be nonnegative and finite",
            Self::InvalidCovarianceTolerance => {
                "covariance and direction tolerances must be nonnegative and finite"
            }
            Self::InvalidUnitDirection => "sphere direction must have unit norm",
            Self::InvalidEvidenceIdentifier => "evidence identifier must be nonempty",
            Self::InvalidEvidenceDigest => {
                "evidence digest must be 64 lowercase hexadecimal characters"
            }
            Self::EvidenceIdentityMismatch => {
                "event, model or target-distribution identities do not match"
            }
            Self::InvalidTemporalHorizon => "temporal horizon or maximum interval is invalid",
            Self::InsufficientTemporalSamples => "at least two time samples are required",
            Self::DuplicateTemporalSample => "temporal sample identifiers must be unique",
            Self::TemporalLebedevRuleMismatch => "all time samples must use the same Lebedev rule",
            Self::InvalidTemporalRate => "temporal Ncdot rate or support report is invalid",
            Self::TemporalHorizonEndpointMismatch => {
                "temporal samples must include both declared horizon endpoints"
            }
            Self::NonIncreasingTemporalEpochs => {
                "temporal sample epochs must be strictly increasing"
            }
            Self::InvalidNcdotRefinementPolicy => {
                "NASA-style Ncdot refinement policy contains invalid thresholds"
            }
            Self::InvalidNcdotRefinementSnapshot => {
                "Ncdot refinement snapshot is incomplete or internally inconsistent"
            }
            Self::NcdotRefinementResolutionLimit => {
                "Ncdot refinement midpoint is not representable as a distinct epoch"
            }
            Self::InvalidGravitationalParameter => {
                "gravitational parameter must be finite and strictly positive"
            }
            Self::InvalidPopColAdjustmentPolicy => {
                "CoL energy-change policy must be nonnegative and not NaN"
            }
            Self::InvalidPopColAdjustmentInput => {
                "CoL energy-adjustment input or objective is invalid"
            }
            Self::PopColAdjustmentResolutionLimit => {
                "CoL fraction midpoint is not representable as a distinct value"
            }
            Self::InvalidPeakOverlapConvergenceConfig => {
                "PeakOverlapPos convergence tolerances are invalid"
            }
            Self::InvalidPeakOverlapConvergenceObservation => {
                "PeakOverlapPos convergence observation is incomplete or inconsistent"
            }
        };
        f.write_str(message)
    }
}

impl std::error::Error for KernelError {}

/// A multivariate normal distribution with a validated positive-definite
/// covariance matrix.
///
/// The covariance is stored through its lower Cholesky factor. Matrices use
/// row-major flattened storage of length dimension squared.
#[derive(Debug, Clone, PartialEq)]
pub struct Gaussian {
    mean: Vec<f64>,
    lower_cholesky: Vec<f64>,
    log_determinant: f64,
}

impl Gaussian {
    /// Construct a Gaussian from its mean and row-major covariance matrix.
    ///
    /// Construction fails closed for empty, non-finite, non-symmetric, or
    /// non-positive-definite inputs. The covariance is not silently repaired,
    /// regularized, or eigenvalue-clipped.
    pub fn new(mean: Vec<f64>, covariance: Vec<f64>) -> Result<Self, KernelError> {
        let dimension = mean.len();
        if dimension == 0 {
            return Err(KernelError::EmptyDimension);
        }
        if !all_finite(&mean) {
            return Err(KernelError::NonFiniteValue);
        }

        let lower_cholesky = cholesky_lower(&covariance, dimension)?;
        let mut log_determinant = 0.0;
        for index in 0..dimension {
            let diagonal = lower_cholesky[index * dimension + index];
            log_determinant += 2.0 * diagonal.ln();
        }
        if !log_determinant.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }

        Ok(Self {
            mean,
            lower_cholesky,
            log_determinant,
        })
    }

    /// Return the distribution dimension.
    pub fn dimension(&self) -> usize {
        self.mean.len()
    }

    /// Return the mean vector.
    pub fn mean(&self) -> &[f64] {
        &self.mean
    }

    /// Evaluate the multivariate Gaussian log density at a point.
    ///
    /// This returns a log density, not a probability mass. The Cholesky solve
    /// avoids explicit matrix inversion.
    pub fn log_pdf(&self, point: &[f64]) -> Result<f64, KernelError> {
        let dimension = self.dimension();
        if point.len() != dimension {
            return Err(KernelError::DimensionMismatch);
        }
        if !all_finite(point) {
            return Err(KernelError::NonFiniteValue);
        }

        let mut whitened = vec![0.0; dimension];
        for row in 0..dimension {
            let mut residual = point[row] - self.mean[row];
            for (column, whitened_value) in whitened.iter().enumerate().take(row) {
                residual -= self.lower_cholesky[row * dimension + column] * whitened_value;
            }
            let diagonal = self.lower_cholesky[row * dimension + row];
            whitened[row] = residual / diagonal;
        }

        let quadratic_form: f64 = whitened.iter().map(|value| value * value).sum();
        if !quadratic_form.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }

        let normalizer = dimension as f64 * core::f64::consts::TAU.ln() + self.log_determinant;
        let result = -0.5 * (normalizer + quadratic_form);
        if result.is_finite() {
            Ok(result)
        } else {
            Err(KernelError::NumericalOverflow)
        }
    }
}

/// Evaluate the log probability of a Gaussian half-space event.
///
/// For X ~ N(mean, covariance), this returns log P(normal dot X <= bound).
/// The event is reduced exactly to a one-dimensional normal projection. The
/// covariance must be strictly positive definite and the projected variance
/// must be positive.
///
/// The log-domain result remains useful when the probability itself underflows
/// to zero in f64, which is important when validating rare-event calculations.
pub fn gaussian_half_space_log_probability(
    mean: &[f64],
    covariance: &[f64],
    normal: &[f64],
    bound: f64,
) -> Result<f64, KernelError> {
    let z = gaussian_half_space_standardized_bound(mean, covariance, normal, bound)?;
    log_standard_normal_cdf(z)
}

/// Evaluate the probability of a Gaussian half-space event.
///
/// This exponentiates the log-probability API. Extremely small probabilities
/// may underflow to zero; use the log-probability API when the tail magnitude
/// itself is scientifically relevant.
pub fn gaussian_half_space_probability(
    mean: &[f64],
    covariance: &[f64],
    normal: &[f64],
    bound: f64,
) -> Result<f64, KernelError> {
    let log_probability = gaussian_half_space_log_probability(mean, covariance, normal, bound)?;
    let probability = log_probability.exp();
    if probability.is_finite() && (0.0..=1.0).contains(&probability) {
        Ok(probability)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

/// Return the log probability mass of a standard-normal interval [lower, upper].
///
/// The calculation uses the lower tail on the negative side and the survival
/// tail on the positive side to avoid subtracting two probabilities close to
/// one. Intervals crossing zero use the error function, which is well
/// conditioned around the origin.
pub fn standard_normal_interval_log_probability(
    lower: f64,
    upper: f64,
) -> Result<f64, KernelError> {
    if lower.is_nan() || upper.is_nan() {
        return Err(KernelError::NonFiniteValue);
    }
    if lower > upper {
        return Err(KernelError::InvalidInterval);
    }
    if lower == upper {
        return Ok(f64::NEG_INFINITY);
    }

    if lower >= 0.0 {
        let log_survival_lower = log_standard_normal_cdf(-lower)?;
        let log_survival_upper = log_standard_normal_cdf(-upper)?;
        log_difference_exp(log_survival_lower, log_survival_upper)
    } else if upper <= 0.0 {
        let log_cdf_upper = log_standard_normal_cdf(upper)?;
        let log_cdf_lower = log_standard_normal_cdf(lower)?;
        log_difference_exp(log_cdf_upper, log_cdf_lower)
    } else {
        let mass = 0.5
            * (libm::erf(upper / core::f64::consts::SQRT_2)
                - libm::erf(lower / core::f64::consts::SQRT_2));
        if mass.is_finite() && mass > 0.0 && mass <= 1.0 {
            Ok(mass.ln())
        } else {
            Err(KernelError::NumericalOverflow)
        }
    }
}

/// Return the log probability mass of an axis-aligned box under independent
/// standard-normal coordinates.
///
/// Each axis interval is evaluated with stable lower-tail / survival-tail
/// subtraction and the log masses are summed. This is useful for probability
/// accounting over whitened Gaussian cells. The result is a floating-point
/// value, not an outward-rounded interval enclosure.
pub fn standard_normal_box_log_probability(
    lower: &[f64],
    upper: &[f64],
) -> Result<f64, KernelError> {
    if lower.is_empty() || upper.is_empty() {
        return Err(KernelError::EmptyDimension);
    }
    if lower.len() != upper.len() {
        return Err(KernelError::DimensionMismatch);
    }

    let mut log_mass = 0.0;
    for (axis_lower, axis_upper) in lower.iter().zip(upper.iter()) {
        let axis_log_mass = standard_normal_interval_log_probability(*axis_lower, *axis_upper)?;
        if axis_log_mass == f64::NEG_INFINITY {
            return Ok(f64::NEG_INFINITY);
        }
        log_mass += axis_log_mass;
    }

    if log_mass.is_finite() {
        Ok(log_mass)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

/// Return the logarithm of the union-bound upper bound for a standard-normal
/// vector lying outside the cube [-radius, radius]^dimension.
///
/// By the union bound, the outside probability is at most
/// min(1, 2 * dimension * Phi(-radius)). This is a numerical point evaluation
/// of a mathematical upper bound; a certified residual-mass implementation
/// must outward-round the CDF and final arithmetic.
pub fn standard_normal_cube_tail_log_upper_bound(
    radius: f64,
    dimension: usize,
) -> Result<f64, KernelError> {
    if dimension == 0 {
        return Err(KernelError::EmptyDimension);
    }
    if !radius.is_finite() || radius <= 0.0 {
        return Err(KernelError::InvalidRadius);
    }

    let one_axis_log_tail = log_standard_normal_cdf(-radius)?;
    let union_log_bound = (2.0 * dimension as f64).ln() + one_axis_log_tail;
    if !union_log_bound.is_finite() {
        return if union_log_bound == f64::NEG_INFINITY {
            Ok(f64::NEG_INFINITY)
        } else {
            Err(KernelError::NumericalOverflow)
        };
    }
    Ok(union_log_bound.min(0.0))
}

fn gaussian_half_space_standardized_bound(
    mean: &[f64],
    covariance: &[f64],
    normal: &[f64],
    bound: f64,
) -> Result<f64, KernelError> {
    let dimension = mean.len();
    if dimension == 0 {
        return Err(KernelError::EmptyDimension);
    }
    if normal.len() != dimension {
        return Err(KernelError::DimensionMismatch);
    }
    if !bound.is_finite() || !all_finite(mean) || !all_finite(normal) {
        return Err(KernelError::NonFiniteValue);
    }

    // Validate covariance through the same strict Cholesky path used by
    // Gaussian::new. The Cholesky factor also gives a stable projected
    // variance without explicitly multiplying the full covariance matrix.
    let lower = cholesky_lower(covariance, dimension)?;

    if normal.iter().all(|value| *value == 0.0) {
        return if bound >= 0.0 {
            Ok(f64::INFINITY)
        } else {
            Ok(f64::NEG_INFINITY)
        };
    }

    let projected_mean: f64 = normal.iter().zip(mean.iter()).map(|(a, x)| a * x).sum();

    // If covariance = L L^T, then a^T covariance a = ||L^T a||^2.
    let mut projected_variance = 0.0;
    for column in 0..dimension {
        let mut projected = 0.0;
        for row in column..dimension {
            projected += lower[row * dimension + column] * normal[row];
        }
        projected_variance += projected * projected;
    }

    if !projected_mean.is_finite() || !projected_variance.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    if projected_variance <= 0.0 {
        return Err(KernelError::ZeroProjectedVariance);
    }

    let z = (bound - projected_mean) / projected_variance.sqrt();
    if z.is_nan() {
        Err(KernelError::NumericalOverflow)
    } else {
        Ok(z)
    }
}

fn log_standard_normal_cdf(value: f64) -> Result<f64, KernelError> {
    if value.is_nan() {
        return Err(KernelError::NonFiniteValue);
    }
    if value == f64::INFINITY {
        return Ok(0.0);
    }
    if value == f64::NEG_INFINITY {
        return Ok(f64::NEG_INFINITY);
    }
    if value <= -8.0 {
        return Ok(log_standard_normal_lower_tail(-value));
    }
    if value >= 8.0 {
        let log_upper_tail = log_standard_normal_lower_tail(value);
        return Ok((-log_upper_tail.exp()).ln_1p());
    }

    let probability = 0.5 * libm::erfc(-value / core::f64::consts::SQRT_2);
    if probability.is_finite() && probability > 0.0 && probability <= 1.0 {
        Ok(probability.ln())
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

fn log_standard_normal_lower_tail(value: f64) -> f64 {
    // Mills-ratio asymptotic expansion:
    // Phi(-x) = phi(x)/x * (1 - 1/x^2 + 3/x^4 - 15/x^6 + ...).
    // Truncate at the smallest term to avoid entering the divergent tail.
    let inverse_square = 1.0 / (value * value);
    let mut term = 1.0;
    let mut series = 1.0;
    let mut previous_magnitude = 1.0;

    for order in 1..=512 {
        term *= -((2 * order - 1) as f64) * inverse_square;
        let magnitude = term.abs();
        if magnitude >= previous_magnitude {
            break;
        }
        series += term;
        previous_magnitude = magnitude;
        if magnitude <= f64::EPSILON * series.abs() {
            break;
        }
    }

    -0.5 * value * value - 0.5 * core::f64::consts::TAU.ln() - value.ln() + series.ln()
}

fn log_difference_exp(log_larger: f64, log_smaller: f64) -> Result<f64, KernelError> {
    if log_larger.is_nan() || log_smaller.is_nan() || log_smaller > log_larger {
        return Err(KernelError::NumericalOverflow);
    }
    if log_larger == log_smaller {
        return Ok(f64::NEG_INFINITY);
    }
    if log_smaller == f64::NEG_INFINITY {
        return Ok(log_larger);
    }

    let result = log_larger + (-(log_smaller - log_larger).exp()).ln_1p();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

/// One positive-weight component in a Gaussian mixture.
#[derive(Debug, Clone, PartialEq)]
pub struct GaussianComponent {
    weight: f64,
    distribution: Gaussian,
}

impl GaussianComponent {
    /// Construct a mixture component with a strictly positive finite weight.
    pub fn new(weight: f64, distribution: Gaussian) -> Result<Self, KernelError> {
        if !weight.is_finite() || weight <= 0.0 {
            return Err(KernelError::InvalidMixtureWeight);
        }
        Ok(Self {
            weight,
            distribution,
        })
    }

    /// Return the normalized component weight.
    pub fn weight(&self) -> f64 {
        self.weight
    }

    /// Return the component distribution.
    pub fn distribution(&self) -> &Gaussian {
        &self.distribution
    }
}

/// A normalized finite mixture of multivariate Gaussian distributions.
///
/// The constructor requires weights to sum to one within 1e-12. It normalizes
/// only the residual floating-point roundoff in that accepted range; materially
/// unnormalized input is rejected.
#[derive(Debug, Clone, PartialEq)]
pub struct GaussianMixture {
    components: Vec<GaussianComponent>,
    dimension: usize,
}

impl GaussianMixture {
    /// Construct a normalized Gaussian mixture.
    pub fn new(mut components: Vec<GaussianComponent>) -> Result<Self, KernelError> {
        if components.is_empty() {
            return Err(KernelError::EmptyDimension);
        }
        let dimension = components[0].distribution.dimension();
        if components
            .iter()
            .any(|component| component.distribution.dimension() != dimension)
        {
            return Err(KernelError::DimensionMismatch);
        }

        let weight_sum: f64 = components.iter().map(|component| component.weight).sum();
        if !weight_sum.is_finite() || (weight_sum - 1.0).abs() > 1.0e-12 {
            return Err(KernelError::MixtureWeightsNotNormalized);
        }
        for component in &mut components {
            component.weight /= weight_sum;
        }

        Ok(Self {
            components,
            dimension,
        })
    }

    /// Return the number of Gaussian components.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Return the common component dimension.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Return all normalized mixture components.
    pub fn components(&self) -> &[GaussianComponent] {
        &self.components
    }

    /// Evaluate the full mixture log density.
    ///
    /// Importance sampling must use this full density in the denominator,
    /// including every defensive or coverage component.
    pub fn log_pdf(&self, point: &[f64]) -> Result<f64, KernelError> {
        if point.len() != self.dimension {
            return Err(KernelError::DimensionMismatch);
        }
        let mut terms = Vec::with_capacity(self.components.len());
        for component in &self.components {
            let component_log_pdf = component.distribution.log_pdf(point)?;
            terms.push(component.weight.ln() + component_log_pdf);
        }
        logsumexp(&terms)
    }
}

/// Compute log p(x) - log q(x), rejecting zero proposal density.
pub fn log_importance_weight(
    log_target_density: f64,
    log_proposal_density: f64,
) -> Result<f64, KernelError> {
    if log_target_density.is_nan()
        || log_target_density == f64::INFINITY
        || log_proposal_density.is_nan()
        || log_proposal_density == f64::INFINITY
    {
        return Err(KernelError::NonFiniteValue);
    }
    if log_proposal_density == f64::NEG_INFINITY {
        return Err(KernelError::ProposalHasZeroDensity);
    }
    if log_target_density == f64::NEG_INFINITY {
        return Ok(f64::NEG_INFINITY);
    }

    let weight = log_target_density - log_proposal_density;
    if weight.is_finite() {
        Ok(weight)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

/// Compute the likelihood ratio p(x) / q(x) from log densities.
///
/// Underflow to zero is retained as floating-point underflow. Overflow is
/// reported as an error; callers should retain the log weight in their
/// evidence record even when converting to a linear weight.
pub fn importance_weight(
    log_target_density: f64,
    log_proposal_density: f64,
) -> Result<f64, KernelError> {
    let log_weight = log_importance_weight(log_target_density, log_proposal_density)?;
    if log_weight == f64::NEG_INFINITY {
        return Ok(0.0);
    }
    if log_weight > f64::MAX.ln() {
        return Err(KernelError::NumericalOverflow);
    }
    let weight = log_weight.exp();
    if weight.is_finite() {
        Ok(weight)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

/// Return the importance-weighted event indicator for one evaluation draw.
///
/// A non-event contributes exactly zero. An event contributes p(x) / q(x).
/// This primitive does not aggregate draws or construct a confidence interval.
pub fn weighted_event_indicator(
    event_occurred: bool,
    log_target_density: f64,
    log_proposal_density: f64,
) -> Result<f64, KernelError> {
    if !event_occurred {
        return Ok(0.0);
    }
    importance_weight(log_target_density, log_proposal_density)
}

/// Check the strict Loewner covariance-floor inequality.
///
/// Returns success exactly when candidate - fraction * nominal is strictly
/// positive definite under the floating-point Cholesky check. For the Lorek
/// constrained-mixture condition, callers would normally set fraction to 0.5.
///
/// This function validates only the matrix inequality. It does not establish
/// that the remaining assumptions of a finite-variance theorem hold.
pub fn validate_covariance_floor(
    candidate: &[f64],
    nominal: &[f64],
    dimension: usize,
    fraction: f64,
) -> Result<(), KernelError> {
    if !fraction.is_finite() || fraction < 0.0 {
        return Err(KernelError::InvalidCovarianceFloorFraction);
    }
    if dimension == 0 {
        return Err(KernelError::EmptyDimension);
    }
    let expected_length = dimension
        .checked_mul(dimension)
        .ok_or(KernelError::InvalidMatrixLength)?;
    if candidate.len() != expected_length || nominal.len() != expected_length {
        return Err(KernelError::InvalidMatrixLength);
    }

    // The nominal covariance itself must be a valid positive-definite matrix.
    let _ = cholesky_lower(nominal, dimension)?;
    let _ = cholesky_lower(candidate, dimension)?;

    let difference: Vec<f64> = candidate
        .iter()
        .zip(nominal.iter())
        .map(|(candidate_value, nominal_value)| candidate_value - fraction * nominal_value)
        .collect();
    let _ = cholesky_lower(&difference, dimension)?;
    Ok(())
}

fn all_finite(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn cholesky_lower(matrix: &[f64], dimension: usize) -> Result<Vec<f64>, KernelError> {
    if dimension == 0 {
        return Err(KernelError::EmptyDimension);
    }
    let expected_length = dimension
        .checked_mul(dimension)
        .ok_or(KernelError::InvalidMatrixLength)?;
    if matrix.len() != expected_length {
        return Err(KernelError::InvalidMatrixLength);
    }
    if !all_finite(matrix) {
        return Err(KernelError::NonFiniteValue);
    }

    let scale = matrix
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let symmetry_tolerance = 64.0 * f64::EPSILON * scale.max(f64::MIN_POSITIVE);
    for row in 0..dimension {
        for column in (row + 1)..dimension {
            if (matrix[row * dimension + column] - matrix[column * dimension + row]).abs()
                > symmetry_tolerance
            {
                return Err(KernelError::NonSymmetricMatrix);
            }
        }
    }

    let mut lower = vec![0.0; expected_length];
    for row in 0..dimension {
        for column in 0..=row {
            let mut value = if row == column {
                matrix[row * dimension + column]
            } else {
                // The prior symmetry check bounds this adjustment to roundoff.
                0.5 * (matrix[row * dimension + column] + matrix[column * dimension + row])
            };
            for inner in 0..column {
                value -= lower[row * dimension + inner] * lower[column * dimension + inner];
            }

            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return Err(KernelError::NotPositiveDefinite);
                }
                lower[row * dimension + column] = value.sqrt();
            } else {
                let diagonal = lower[column * dimension + column];
                let entry = value / diagonal;
                if !entry.is_finite() {
                    return Err(KernelError::NumericalOverflow);
                }
                lower[row * dimension + column] = entry;
            }
        }
    }
    Ok(lower)
}

fn logsumexp(values: &[f64]) -> Result<f64, KernelError> {
    if values.is_empty() {
        return Err(KernelError::EmptyDimension);
    }
    if values
        .iter()
        .any(|value| value.is_nan() || *value == f64::INFINITY)
    {
        return Err(KernelError::NonFiniteValue);
    }
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if maximum == f64::NEG_INFINITY {
        return Ok(f64::NEG_INFINITY);
    }

    let sum: f64 = values.iter().map(|value| (value - maximum).exp()).sum();
    let result = maximum + sum.ln();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:.17e} differs from expected {expected:.17e} by more than {tolerance:.3e}"
        );
    }

    #[test]
    fn standard_normal_log_density_at_origin_is_known_answer() {
        let gaussian = Gaussian::new(vec![0.0], vec![1.0]).unwrap();
        assert_close(
            gaussian.log_pdf(&[0.0]).unwrap(),
            -0.5 * core::f64::consts::TAU.ln(),
            2.0e-15,
        );
    }

    #[test]
    fn translated_and_scaled_gaussian_log_density_is_known_answer() {
        let gaussian = Gaussian::new(vec![2.0], vec![4.0]).unwrap();
        let expected = -0.5 * core::f64::consts::TAU.ln() - 0.5 * 4.0_f64.ln();
        assert_close(gaussian.log_pdf(&[2.0]).unwrap(), expected, 2.0e-15);
    }

    #[test]
    fn standard_normal_half_space_is_one_half() {
        let probability = gaussian_half_space_probability(&[0.0], &[1.0], &[1.0], 0.0).unwrap();
        assert_close(probability, 0.5, 2.0e-16);
    }

    #[test]
    fn log_half_space_probability_retains_extreme_gaussian_tail() {
        let log_probability =
            gaussian_half_space_log_probability(&[40.0], &[1.0], &[1.0], 0.0).unwrap();
        assert_close(log_probability, -804.6084420137538, 2.0e-12);
    }

    #[test]
    fn analytic_probability_sweep_covers_target_tails_and_dimensions() {
        let tail_cases = [
            (3.090_232_306_167_813, 1.0e-3_f64),
            (4.264_890_793_923_84, 1.0e-5_f64),
            (5.199_337_582_290_662, 1.0e-7_f64),
            (5.997_807_019_601_638, 1.0e-9_f64),
        ];

        for dimension in [1_usize, 2, 3, 6, 12] {
            let mut covariance = vec![0.0; dimension * dimension];
            for (index, value) in covariance.iter_mut().enumerate() {
                if index % (dimension + 1) == 0 {
                    *value = 1.0;
                }
            }
            let mut normal = vec![0.0; dimension];
            normal[0] = 1.0;

            for (offset, expected_probability) in tail_cases {
                let mut mean = vec![0.0; dimension];
                mean[0] = offset;
                let log_probability =
                    gaussian_half_space_log_probability(&mean, &covariance, &normal, 0.0).unwrap();
                assert_close(log_probability, expected_probability.ln(), 1.0e-7);
            }
        }
    }

    #[test]
    fn log_half_space_probability_matches_ten_sigma_reference() {
        let log_probability =
            gaussian_half_space_log_probability(&[10.0], &[1.0], &[1.0], 0.0).unwrap();
        assert_close(log_probability, -53.23128515051247, 2.0e-13);
    }

    #[test]
    fn log_half_space_probability_remains_finite_at_one_hundred_sigma() {
        let log_probability =
            gaussian_half_space_log_probability(&[100.0], &[1.0], &[1.0], 0.0).unwrap();
        assert_close(log_probability, -5005.524208694205, 2.0e-10);
    }

    #[test]
    fn zero_normal_does_not_bypass_covariance_validation() {
        assert_eq!(
            gaussian_half_space_log_probability(&[0.0], &[-1.0], &[0.0], 1.0),
            Err(KernelError::NotPositiveDefinite)
        );
    }

    #[test]
    fn ordinary_half_space_probability_may_underflow_but_log_api_does_not() {
        let probability = gaussian_half_space_probability(&[40.0], &[1.0], &[1.0], 0.0).unwrap();
        assert_eq!(probability, 0.0);
    }

    #[test]
    fn zero_normal_has_deterministic_half_space_semantics() {
        assert_eq!(
            gaussian_half_space_log_probability(&[4.0], &[1.0], &[0.0], 1.0).unwrap(),
            0.0
        );
        assert_eq!(
            gaussian_half_space_log_probability(&[4.0], &[1.0], &[0.0], -1.0).unwrap(),
            f64::NEG_INFINITY
        );
    }

    #[test]
    fn standard_normal_interval_uses_stable_positive_and_negative_tails() {
        let negative = standard_normal_interval_log_probability(-41.0, -40.0).unwrap();
        let positive = standard_normal_interval_log_probability(40.0, 41.0).unwrap();
        assert_close(negative, -804.6084420137538, 2.0e-12);
        assert_close(positive, -804.6084420137538, 2.0e-12);
    }

    #[test]
    fn standard_normal_interval_crossing_zero_is_stable() {
        let log_mass = standard_normal_interval_log_probability(-1.0e-10, 1.0e-10).unwrap();
        let expected = (2.0e-10 / core::f64::consts::TAU.sqrt()).ln();
        assert_close(log_mass, expected, 2.0e-15);
    }

    #[test]
    fn standard_normal_box_mass_factorizes_in_log_space() {
        let log_mass = standard_normal_box_log_probability(
            &[-f64::INFINITY, -f64::INFINITY],
            &[f64::INFINITY, f64::INFINITY],
        )
        .unwrap();
        assert_close(log_mass, 0.0, 0.0);
    }

    #[test]
    fn standard_normal_box_retains_extreme_multidimensional_tail_mass() {
        let log_mass = standard_normal_box_log_probability(&[40.0, 40.0], &[41.0, 41.0]).unwrap();
        assert_close(log_mass, -1609.2168840275076, 4.0e-12);
    }

    #[test]
    fn standard_normal_cube_tail_union_bound_is_capped_at_one() {
        assert_eq!(
            standard_normal_cube_tail_log_upper_bound(0.01, 3).unwrap(),
            0.0
        );
    }

    #[test]
    fn standard_normal_cube_tail_union_bound_decreases_with_radius() {
        let moderate = standard_normal_cube_tail_log_upper_bound(5.0, 6).unwrap();
        let large = standard_normal_cube_tail_log_upper_bound(8.0, 6).unwrap();
        assert!(large < moderate);
    }

    #[test]
    fn standard_normal_box_rejects_dimension_mismatch() {
        assert_eq!(
            standard_normal_box_log_probability(&[-1.0, -1.0], &[1.0]),
            Err(KernelError::DimensionMismatch)
        );
    }

    #[test]
    fn standard_normal_interval_rejects_reversed_endpoints() {
        assert_eq!(
            standard_normal_interval_log_probability(1.0, -1.0),
            Err(KernelError::InvalidInterval)
        );
    }

    #[test]
    fn correlated_multivariate_half_space_reduces_to_projected_normal() {
        let probability =
            gaussian_half_space_probability(&[0.0, 0.0], &[1.0, 0.5, 0.5, 1.0], &[1.0, 1.0], 0.0)
                .unwrap();
        assert_close(probability, 0.5, 2.0e-16);
    }

    #[test]
    fn shifted_half_space_probability_matches_standard_normal_cdf() {
        let probability = gaussian_half_space_probability(&[1.0], &[4.0], &[1.0], 1.0).unwrap();
        assert_close(probability, 0.5, 2.0e-16);
    }

    #[test]
    fn mixture_log_density_uses_the_full_weighted_mixture() {
        let narrow = Gaussian::new(vec![0.0], vec![1.0]).unwrap();
        let broad = Gaussian::new(vec![0.0], vec![4.0]).unwrap();
        let mixture = GaussianMixture::new(vec![
            GaussianComponent::new(0.25, narrow).unwrap(),
            GaussianComponent::new(0.75, broad).unwrap(),
        ])
        .unwrap();

        let density =
            0.25 / core::f64::consts::TAU.sqrt() + 0.75 / (2.0 * core::f64::consts::TAU.sqrt());
        assert_close(mixture.log_pdf(&[0.0]).unwrap(), density.ln(), 2.0e-15);
    }

    #[test]
    fn mixture_rejects_non_normalized_weights_instead_of_repairing_them() {
        let first = Gaussian::new(vec![0.0], vec![1.0]).unwrap();
        let second = Gaussian::new(vec![1.0], vec![1.0]).unwrap();
        let result = GaussianMixture::new(vec![
            GaussianComponent::new(0.4, first).unwrap(),
            GaussianComponent::new(0.4, second).unwrap(),
        ]);
        assert_eq!(result, Err(KernelError::MixtureWeightsNotNormalized));
    }

    #[test]
    fn importance_weight_is_computed_from_log_density_ratio() {
        let log_weight = log_importance_weight(-3.0, -5.0).unwrap();
        assert_close(log_weight, 2.0, 0.0);
        assert_close(
            importance_weight(-3.0, -5.0).unwrap(),
            2.0_f64.exp(),
            2.0e-15,
        );
    }

    #[test]
    fn event_indicator_is_zero_for_non_events() {
        assert_eq!(weighted_event_indicator(false, -2.0, -4.0).unwrap(), 0.0);
    }

    #[test]
    fn zero_proposal_density_is_rejected_for_an_event_weight() {
        assert_eq!(
            log_importance_weight(-2.0, f64::NEG_INFINITY),
            Err(KernelError::ProposalHasZeroDensity)
        );
    }

    #[test]
    fn strict_half_nominal_covariance_floor_accepts_positive_difference() {
        validate_covariance_floor(&[0.75, 0.0, 0.0, 0.65], &[1.0, 0.0, 0.0, 1.0], 2, 0.5).unwrap();
    }

    #[test]
    fn strict_covariance_floor_rejects_equality() {
        assert_eq!(
            validate_covariance_floor(&[0.5, 0.0, 0.0, 0.75], &[1.0, 0.0, 0.0, 1.0], 2, 0.5,),
            Err(KernelError::NotPositiveDefinite)
        );
    }

    #[test]
    fn covariance_floor_rejects_indefinite_difference() {
        assert_eq!(
            validate_covariance_floor(&[0.75, 0.0, 0.0, 0.4], &[1.0, 0.0, 0.0, 1.0], 2, 0.5,),
            Err(KernelError::NotPositiveDefinite)
        );
    }

    #[test]
    fn non_positive_definite_and_asymmetric_covariances_fail_closed() {
        assert_eq!(
            Gaussian::new(vec![0.0, 0.0], vec![1.0, 2.0, 0.0, 1.0]),
            Err(KernelError::NonSymmetricMatrix)
        );
        assert_eq!(
            Gaussian::new(vec![0.0, 0.0], vec![1.0, 2.0, 2.0, 1.0]),
            Err(KernelError::NotPositiveDefinite)
        );
    }

    #[test]
    fn dimension_mismatch_fails_closed() {
        let gaussian = Gaussian::new(vec![0.0, 0.0], vec![1.0, 0.0, 0.0, 1.0]).unwrap();
        assert_eq!(
            gaussian.log_pdf(&[0.0]),
            Err(KernelError::DimensionMismatch)
        );
    }
}
