// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Anderson-Rubin weak-instrument-robust confidence-set inversion for scalar linear IV.
//!
//! V1 uses the classical finite-sample F reference distribution. The caller
//! must declare the independent, homoskedastic Gaussian sampling profile and
//! separately provide an instrument-validity design audit.

#![forbid(unsafe_code)]
#![allow(clippy::needless_range_loop)]

/// Maximum number of observations accepted by one fit.
pub const MAX_OBSERVATIONS: usize = 5_000_000;
/// Maximum number of instrument columns.
pub const MAX_INSTRUMENTS: usize = 256;
const RANK_TOLERANCE_DEFAULT: f64 = 1.0e-10;
const BETA_FRACTION_MAX_ITERATIONS: usize = 300;
const BETA_FRACTION_EPSILON: f64 = 3.0e-14;
const BETA_FRACTION_MINIMUM: f64 = 1.0e-300;
const QUANTILE_ITERATIONS: usize = 120;

/// Caller-supplied status of an instrument-validity design claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssumptionStatus {
    /// The caller declares the assumption supported by the cited experimental design.
    SupportedByDesign,
    /// The assumption has not been established.
    Unknown,
    /// A control or design review contradicts the assumption.
    Violated,
}

/// Evidence reference for one caller-supplied assumption declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssumptionEvidence {
    pub status: AssumptionStatus,
    pub evidence_ref: Option<String>,
}

impl AssumptionEvidence {
    /// Creates an assumption declaration.
    #[must_use]
    pub fn new(status: AssumptionStatus, evidence_ref: Option<String>) -> Self {
        Self {
            status,
            evidence_ref,
        }
    }

    fn is_supported_and_referenced(&self) -> bool {
        self.status == AssumptionStatus::SupportedByDesign
            && self
                .evidence_ref
                .as_ref()
                .is_some_and(|reference| !reference.trim().is_empty())
    }
}

/// Statistical sampling profile required for the classical finite-sample F reference.
///
/// The booleans are caller declarations; the implementation cannot verify
/// normality, independence, homoskedasticity or randomization from the regression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassicalInferenceProfile {
    pub observations_iid: bool,
    pub structural_errors_homoskedastic: bool,
    pub structural_errors_gaussian: bool,
    pub instrument_exogeneity_and_exclusion: AssumptionEvidence,
}

impl ClassicalInferenceProfile {
    /// Whether the classical finite-sample reference conditions were declared.
    #[must_use]
    pub fn supports_classical_reference(&self) -> bool {
        self.observations_iid
            && self.structural_errors_homoskedastic
            && self.structural_errors_gaussian
    }

    /// Whether causal instrument validity was explicitly supported by a referenced design audit.
    #[must_use]
    pub fn causal_qualification(&self) -> CausalQualification {
        if self
            .instrument_exogeneity_and_exclusion
            .is_supported_and_referenced()
        {
            CausalQualification::QualifiedByCallerDesignAudit
        } else {
            CausalQualification::NotQualified
        }
    }
}

/// Causal status, based on a caller-supplied instrument-validity audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalQualification {
    /// Caller declared validity supported and supplied an evidence reference.
    ///
    /// This is not independent proof that the instrument is valid.
    QualifiedByCallerDesignAudit,
    /// Instrument exogeneity / exclusion is unknown, violated or lacks evidence.
    NotQualified,
}

/// Frozen instrument schema and confidence-set policy.
#[derive(Debug, Clone, PartialEq)]
pub struct ArDesign {
    instrument_ids: Vec<String>,
    confidence_level: f64,
    relative_rank_tolerance: f64,
    inference_profile: ClassicalInferenceProfile,
}

impl ArDesign {
    /// Creates and validates the instrument schema and classical inference profile.
    pub fn new(
        instrument_ids: Vec<String>,
        confidence_level: f64,
        relative_rank_tolerance: f64,
        inference_profile: ClassicalInferenceProfile,
    ) -> Result<Self, ArError> {
        if instrument_ids.is_empty() || instrument_ids.len() > MAX_INSTRUMENTS {
            return Err(ArError::InvalidInstrumentCount);
        }
        for id in &instrument_ids {
            if id.trim().is_empty() {
                return Err(ArError::BlankInstrumentId);
            }
        }
        for left in 0..instrument_ids.len() {
            if instrument_ids[left + 1..].contains(&instrument_ids[left]) {
                return Err(ArError::DuplicateInstrumentId);
            }
        }
        if !confidence_level.is_finite() || confidence_level <= 0.0 || confidence_level >= 1.0 {
            return Err(ArError::InvalidConfidenceLevel);
        }
        if !relative_rank_tolerance.is_finite()
            || relative_rank_tolerance <= 0.0
            || relative_rank_tolerance >= 1.0
        {
            return Err(ArError::InvalidRankTolerance);
        }
        if !inference_profile.supports_classical_reference() {
            return Err(ArError::UnsupportedClassicalReferenceProfile);
        }

        Ok(Self {
            instrument_ids,
            confidence_level,
            relative_rank_tolerance,
            inference_profile,
        })
    }

    /// Returns frozen instrument IDs in their required observation-column order.
    #[must_use]
    pub fn instrument_ids(&self) -> &[String] {
        &self.instrument_ids
    }

    /// Returns the declared confidence level.
    #[must_use]
    pub fn confidence_level(&self) -> f64 {
        self.confidence_level
    }

    /// Returns the caller-supplied sampling and instrument-validity audit.
    #[must_use]
    pub fn inference_profile(&self) -> &ClassicalInferenceProfile {
        &self.inference_profile
    }
}

/// One observation row for an endogenous source and a downstream target.
#[derive(Debug, Clone, PartialEq)]
pub struct ArObservation {
    /// Instrument values in the exact order of ArDesign::instrument_ids.
    pub instrument_values: Vec<f64>,
    /// Measured endogenous source activity.
    pub source_activity: f64,
    /// Measured target outcome.
    pub target_activity: f64,
}

/// All supported validation, numerical and inference-scope failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArError {
    InvalidInstrumentCount,
    BlankInstrumentId,
    DuplicateInstrumentId,
    InvalidConfidenceLevel,
    InvalidRankTolerance,
    UnsupportedClassicalReferenceProfile,
    TooManyObservations,
    TooFewObservations,
    InconsistentInstrumentWidth,
    NonFiniteObservation,
    RankDeficientInstruments,
    NumericalFailure,
    DegenerateNullOutcome,
    InvalidDegreesOfFreedom,
    IncompleteBetaDidNotConverge,
    InvalidQuadraticTolerance,
}

impl std::fmt::Display for ArError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInstrumentCount => {
                write!(
                    formatter,
                    "instrument count is empty or exceeds the configured limit"
                )
            }
            Self::BlankInstrumentId => write!(formatter, "instrument ID is blank"),
            Self::DuplicateInstrumentId => write!(formatter, "instrument IDs must be unique"),
            Self::InvalidConfidenceLevel => {
                write!(
                    formatter,
                    "confidence level must be finite and strictly between zero and one"
                )
            }
            Self::InvalidRankTolerance => {
                write!(
                    formatter,
                    "relative rank tolerance must be finite and between zero and one"
                )
            }
            Self::UnsupportedClassicalReferenceProfile => {
                write!(
                    formatter,
                    "classical AR F inference requires declared iid, homoskedastic Gaussian errors"
                )
            }
            Self::TooManyObservations => {
                write!(formatter, "observation count exceeds the configured limit")
            }
            Self::TooFewObservations => {
                write!(
                    formatter,
                    "too few observations for the instrument count and residual degrees of freedom"
                )
            }
            Self::InconsistentInstrumentWidth => {
                write!(
                    formatter,
                    "observation instrument width does not match the frozen schema"
                )
            }
            Self::NonFiniteObservation => {
                write!(formatter, "observation contains a non-finite value")
            }
            Self::RankDeficientInstruments => {
                write!(
                    formatter,
                    "centered instrument columns are rank deficient or numerically singular"
                )
            }
            Self::NumericalFailure => write!(
                formatter,
                "Anderson-Rubin calculation encountered a numerical failure"
            ),
            Self::DegenerateNullOutcome => {
                write!(
                    formatter,
                    "transformed outcome has zero residual and explained sum of squares at the tested coefficient"
                )
            }
            Self::InvalidDegreesOfFreedom => {
                write!(
                    formatter,
                    "F distribution degrees of freedom must be positive"
                )
            }
            Self::IncompleteBetaDidNotConverge => {
                write!(
                    formatter,
                    "regularized incomplete beta continued fraction did not converge"
                )
            }
            Self::InvalidQuadraticTolerance => {
                write!(formatter, "quadratic tolerance must be positive and finite")
            }
        }
    }
}

impl std::error::Error for ArError {}

/// Full topology of a scalar weak-IV confidence set.
///
/// All finite endpoints are included because the inverted AR test uses a
/// non-strict inequality. OutsideInterval denotes two rays with a closed
/// boundary at each finite root.
#[derive(Debug, Clone, PartialEq)]
pub enum ArConfidenceSet {
    Empty,
    AllReal,
    Point { value: f64 },
    BoundedInterval { lower: f64, upper: f64 },
    LowerRay { upper: f64 },
    UpperRay { lower: f64 },
    OutsideInterval { lower: f64, upper: f64 },
}

impl ArConfidenceSet {
    /// Returns whether a finite coefficient is included in this confidence set.
    #[must_use]
    pub fn contains(&self, coefficient: f64) -> bool {
        if !coefficient.is_finite() {
            return false;
        }
        match self {
            Self::Empty => false,
            Self::AllReal => true,
            Self::Point { value } => coefficient == *value,
            Self::BoundedInterval { lower, upper } => {
                coefficient >= *lower && coefficient <= *upper
            }
            Self::LowerRay { upper } => coefficient <= *upper,
            Self::UpperRay { lower } => coefficient >= *lower,
            Self::OutsideInterval { lower, upper } => {
                coefficient <= *lower || coefficient >= *upper
            }
        }
    }

    /// Returns the width for bounded sets only; unbounded sets return None.
    #[must_use]
    pub fn bounded_width(&self) -> Option<f64> {
        match self {
            Self::Point { .. } => Some(0.0),
            Self::BoundedInterval { lower, upper } => Some(upper - lower),
            Self::Empty
            | Self::AllReal
            | Self::LowerRay { .. }
            | Self::UpperRay { .. }
            | Self::OutsideInterval { .. } => None,
        }
    }

    /// Returns true when at least one tail is unbounded.
    #[must_use]
    pub fn is_unbounded(&self) -> bool {
        matches!(
            self,
            Self::AllReal
                | Self::LowerRay { .. }
                | Self::UpperRay { .. }
                | Self::OutsideInterval { .. }
        )
    }
}

/// AR statistic at one candidate structural coefficient.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArStatistic {
    Finite(f64),
    Infinite,
}

/// Test result and confidence-set inversion under the declared classical profile.
#[derive(Debug, Clone, PartialEq)]
pub struct ArResult {
    pub observations: usize,
    pub instrument_ids: Vec<String>,
    pub tested_coefficient: f64,
    pub statistic: ArStatistic,
    pub p_value: f64,
    pub numerator_degrees_of_freedom: usize,
    pub denominator_degrees_of_freedom: usize,
    pub critical_value: f64,
    pub confidence_level: f64,
    pub confidence_set: ArConfidenceSet,
    pub causal_qualification: CausalQualification,
}

/// Fits the Anderson-Rubin test at beta0 and inverts it at the declared confidence level.
///
/// The returned p-value and confidence set use the classical finite-sample
/// F(q, n-q-1) reference. This is not a heteroskedasticity-robust, cluster-robust,
/// or serial-correlation-robust estimator.
pub fn fit_anderson_rubin(
    observations: &[ArObservation],
    design: &ArDesign,
    tested_coefficient: f64,
) -> Result<ArResult, ArError> {
    if !tested_coefficient.is_finite() {
        return Err(ArError::NonFiniteObservation);
    }
    let prepared = prepare_design(observations, design)?;
    let q = design.instrument_ids.len();
    let n = observations.len();
    let denominator_degrees = n - q - 1;
    if denominator_degrees == 0 {
        return Err(ArError::TooFewObservations);
    }

    let numerator = transformed_explained_sum_squares(
        &prepared.projected_x,
        &prepared.projected_y,
        tested_coefficient,
    );
    let denominator = transformed_residual_sum_squares(
        &prepared.residual_x,
        &prepared.residual_y,
        tested_coefficient,
    );
    if !numerator.is_finite() || !denominator.is_finite() || numerator < 0.0 || denominator < 0.0 {
        return Err(ArError::NumericalFailure);
    }

    let statistic = if denominator == 0.0 {
        if numerator == 0.0 {
            return Err(ArError::DegenerateNullOutcome);
        }
        ArStatistic::Infinite
    } else {
        let value = (numerator / q as f64) / (denominator / denominator_degrees as f64);
        if !value.is_finite() || value < 0.0 {
            return Err(ArError::NumericalFailure);
        }
        ArStatistic::Finite(value)
    };

    let p_value = match statistic {
        ArStatistic::Finite(value) => f_survival_function(value, q, denominator_degrees)?,
        ArStatistic::Infinite => 0.0,
    };
    let critical_value = f_quantile(design.confidence_level, q, denominator_degrees)?;
    let confidence_set = invert_ar_quadratic(
        &prepared,
        q,
        denominator_degrees,
        critical_value,
        design.relative_rank_tolerance,
    )?;

    Ok(ArResult {
        observations: n,
        instrument_ids: design.instrument_ids.clone(),
        tested_coefficient,
        statistic,
        p_value,
        numerator_degrees_of_freedom: q,
        denominator_degrees_of_freedom: denominator_degrees,
        critical_value,
        confidence_level: design.confidence_level,
        confidence_set,
        causal_qualification: design.inference_profile.causal_qualification(),
    })
}

struct PreparedDesign {
    projected_x: Vec<f64>,
    projected_y: Vec<f64>,
    residual_x: Vec<f64>,
    residual_y: Vec<f64>,
}

fn prepare_design(
    observations: &[ArObservation],
    design: &ArDesign,
) -> Result<PreparedDesign, ArError> {
    let n = observations.len();
    let q = design.instrument_ids.len();
    if n > MAX_OBSERVATIONS {
        return Err(ArError::TooManyObservations);
    }
    if n <= q + 1 {
        return Err(ArError::TooFewObservations);
    }

    for observation in observations {
        if observation.instrument_values.len() != q {
            return Err(ArError::InconsistentInstrumentWidth);
        }
        if !observation.source_activity.is_finite()
            || !observation.target_activity.is_finite()
            || observation
                .instrument_values
                .iter()
                .any(|value| !value.is_finite())
        {
            return Err(ArError::NonFiniteObservation);
        }
    }

    let mean_x = observations
        .iter()
        .map(|row| row.source_activity)
        .sum::<f64>()
        / n as f64;
    let mean_y = observations
        .iter()
        .map(|row| row.target_activity)
        .sum::<f64>()
        / n as f64;
    let x: Vec<f64> = observations
        .iter()
        .map(|row| row.source_activity - mean_x)
        .collect();
    let y: Vec<f64> = observations
        .iter()
        .map(|row| row.target_activity - mean_y)
        .collect();

    let mut centered_columns = vec![vec![0.0; n]; q];
    for column in 0..q {
        let mean_z = observations
            .iter()
            .map(|row| row.instrument_values[column])
            .sum::<f64>()
            / n as f64;
        for row in 0..n {
            centered_columns[column][row] = observations[row].instrument_values[column] - mean_z;
        }
    }

    let basis = orthonormal_basis(centered_columns, design.relative_rank_tolerance)?;
    let projected_x = project(&basis, &x);
    let projected_y = project(&basis, &y);
    let residual_x: Vec<f64> = x
        .iter()
        .zip(&projected_x)
        .map(|(actual, fitted)| actual - fitted)
        .collect();
    let residual_y: Vec<f64> = y
        .iter()
        .zip(&projected_y)
        .map(|(actual, fitted)| actual - fitted)
        .collect();

    Ok(PreparedDesign {
        projected_x,
        projected_y,
        residual_x,
        residual_y,
    })
}

fn orthonormal_basis(
    mut columns: Vec<Vec<f64>>,
    relative_tolerance: f64,
) -> Result<Vec<Vec<f64>>, ArError> {
    let mut basis: Vec<Vec<f64>> = Vec::with_capacity(columns.len());
    for mut column in columns.drain(..) {
        let original_norm = stable_norm(&column);
        if !original_norm.is_finite() || original_norm <= f64::MIN_POSITIVE {
            return Err(ArError::RankDeficientInstruments);
        }
        for value in &mut column {
            *value /= original_norm;
        }

        for _ in 0..2 {
            for accepted in &basis {
                let projection = dot(accepted, &column);
                for index in 0..column.len() {
                    column[index] -= projection * accepted[index];
                }
            }
        }

        let residual_norm = stable_norm(&column);
        if !residual_norm.is_finite() || residual_norm <= relative_tolerance {
            return Err(ArError::RankDeficientInstruments);
        }
        for value in &mut column {
            *value /= residual_norm;
        }
        basis.push(column);
    }
    Ok(basis)
}

fn project(basis: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    let mut output = vec![0.0; vector.len()];
    for column in basis {
        let coefficient = dot(column, vector);
        for index in 0..vector.len() {
            output[index] += coefficient * column[index];
        }
    }
    output
}

fn transformed_explained_sum_squares(projected_x: &[f64], projected_y: &[f64], beta: f64) -> f64 {
    projected_x
        .iter()
        .zip(projected_y)
        .map(|(x, y)| (y - beta * x).powi(2))
        .sum()
}

fn transformed_residual_sum_squares(residual_x: &[f64], residual_y: &[f64], beta: f64) -> f64 {
    residual_x
        .iter()
        .zip(residual_y)
        .map(|(x, y)| (y - beta * x).powi(2))
        .sum()
}

fn invert_ar_quadratic(
    prepared: &PreparedDesign,
    q: usize,
    denominator_degrees: usize,
    critical_value: f64,
    tolerance: f64,
) -> Result<ArConfidenceSet, ArError> {
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(ArError::InvalidQuadraticTolerance);
    }
    let df = denominator_degrees as f64;
    let qf = q as f64 * critical_value;

    // Retain beta where:
    // df * ||Pz(y - beta*x)||^2 - q*Fcrit*||(I-Pz)(y-beta*x)||^2 <= 0.
    let a = df * dot(&prepared.projected_x, &prepared.projected_x)
        - qf * dot(&prepared.residual_x, &prepared.residual_x);
    let b = -2.0 * df * dot(&prepared.projected_x, &prepared.projected_y)
        + 2.0 * qf * dot(&prepared.residual_x, &prepared.residual_y);
    let c = df * dot(&prepared.projected_y, &prepared.projected_y)
        - qf * dot(&prepared.residual_y, &prepared.residual_y);

    quadratic_inequality_set(a, b, c, tolerance)
}

/// Returns the real solution set to a*beta^2 + b*beta + c <= 0.
pub fn quadratic_inequality_set(
    a: f64,
    b: f64,
    c: f64,
    relative_tolerance: f64,
) -> Result<ArConfidenceSet, ArError> {
    if !a.is_finite()
        || !b.is_finite()
        || !c.is_finite()
        || !relative_tolerance.is_finite()
        || relative_tolerance <= 0.0
    {
        return Err(ArError::InvalidQuadraticTolerance);
    }

    let scale = a.abs().max(b.abs()).max(c.abs());
    if scale == 0.0 {
        return Ok(ArConfidenceSet::AllReal);
    }
    let a = a / scale;
    let b = b / scale;
    let c = c / scale;
    let epsilon = relative_tolerance;

    if a.abs() <= epsilon {
        if b.abs() <= epsilon {
            return Ok(if c <= 0.0 {
                ArConfidenceSet::AllReal
            } else {
                ArConfidenceSet::Empty
            });
        }
        let root = -c / b;
        if !root.is_finite() {
            return Err(ArError::NumericalFailure);
        }
        return Ok(if b > 0.0 {
            ArConfidenceSet::LowerRay { upper: root }
        } else {
            ArConfidenceSet::UpperRay { lower: root }
        });
    }

    let product = 4.0 * a * c;
    let discriminant = b.mul_add(b, -product);
    if !discriminant.is_finite() {
        return Err(ArError::NumericalFailure);
    }
    let discriminant_scale = (b * b).abs() + product.abs() + 1.0;
    let discriminant_tolerance = epsilon * discriminant_scale;

    if discriminant < -discriminant_tolerance {
        return Ok(if a > 0.0 {
            ArConfidenceSet::Empty
        } else {
            ArConfidenceSet::AllReal
        });
    }

    if discriminant.abs() <= discriminant_tolerance {
        let root = -b / (2.0 * a);
        if !root.is_finite() {
            return Err(ArError::NumericalFailure);
        }
        return Ok(if a > 0.0 {
            ArConfidenceSet::Point { value: root }
        } else {
            ArConfidenceSet::AllReal
        });
    }

    let square_root = discriminant.sqrt();
    let q_value = -0.5 * (b + if b >= 0.0 { square_root } else { -square_root });
    let (first_root, second_root) = if q_value.abs() > f64::MIN_POSITIVE {
        (q_value / a, c / q_value)
    } else {
        (
            (-b - square_root) / (2.0 * a),
            (-b + square_root) / (2.0 * a),
        )
    };
    if !first_root.is_finite() || !second_root.is_finite() {
        return Err(ArError::NumericalFailure);
    }
    let lower = first_root.min(second_root);
    let upper = first_root.max(second_root);

    Ok(if a > 0.0 {
        ArConfidenceSet::BoundedInterval { lower, upper }
    } else {
        ArConfidenceSet::OutsideInterval { lower, upper }
    })
}

/// Classical F cumulative distribution function, using a regularized incomplete beta.
pub fn f_cdf(value: f64, numerator_df: usize, denominator_df: usize) -> Result<f64, ArError> {
    if numerator_df == 0 || denominator_df == 0 {
        return Err(ArError::InvalidDegreesOfFreedom);
    }
    if value.is_nan() || value < 0.0 {
        return Err(ArError::NumericalFailure);
    }
    if value == 0.0 {
        return Ok(0.0);
    }
    if value.is_infinite() {
        return Ok(1.0);
    }
    let d1 = numerator_df as f64;
    let d2 = denominator_df as f64;
    let scaled = (d1 / d2) * value;
    let x = if scaled.is_infinite() {
        1.0
    } else {
        scaled / (1.0 + scaled)
    };
    regularized_incomplete_beta(x, 0.5 * d1, 0.5 * d2)
}

/// Classical F upper-tail probability, evaluated directly to avoid 1-CDF cancellation.
pub fn f_survival_function(
    value: f64,
    numerator_df: usize,
    denominator_df: usize,
) -> Result<f64, ArError> {
    if numerator_df == 0 || denominator_df == 0 {
        return Err(ArError::InvalidDegreesOfFreedom);
    }
    if value.is_nan() || value < 0.0 {
        return Err(ArError::NumericalFailure);
    }
    if value == 0.0 {
        return Ok(1.0);
    }
    if value.is_infinite() {
        return Ok(0.0);
    }
    let d1 = numerator_df as f64;
    let d2 = denominator_df as f64;
    let scaled = (d1 / d2) * value;
    let x = if scaled.is_infinite() {
        0.0
    } else {
        1.0 / (1.0 + scaled)
    };
    regularized_incomplete_beta(x, 0.5 * d2, 0.5 * d1)
}

/// Classical F quantile for a probability in (0,1).
pub fn f_quantile(
    probability: f64,
    numerator_df: usize,
    denominator_df: usize,
) -> Result<f64, ArError> {
    if numerator_df == 0 || denominator_df == 0 {
        return Err(ArError::InvalidDegreesOfFreedom);
    }
    if !probability.is_finite() || probability <= 0.0 || probability >= 1.0 {
        return Err(ArError::InvalidConfidenceLevel);
    }

    let mut lower = 0.0;
    let mut upper = 1.0;
    let mut upper_cdf = f_cdf(upper, numerator_df, denominator_df)?;
    while upper_cdf < probability {
        upper *= 2.0;
        if !upper.is_finite() || upper > 1.0e300 {
            return Err(ArError::NumericalFailure);
        }
        upper_cdf = f_cdf(upper, numerator_df, denominator_df)?;
    }

    for _ in 0..QUANTILE_ITERATIONS {
        let middle = 0.5 * (lower + upper);
        let middle_cdf = f_cdf(middle, numerator_df, denominator_df)?;
        if middle_cdf < probability {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    Ok(0.5 * (lower + upper))
}

fn regularized_incomplete_beta(x: f64, a: f64, b: f64) -> Result<f64, ArError> {
    if !x.is_finite() || x < 0.0 || x > 1.0 || a <= 0.0 || b <= 0.0 {
        return Err(ArError::NumericalFailure);
    }
    if x == 0.0 {
        return Ok(0.0);
    }
    if x == 1.0 {
        return Ok(1.0);
    }

    let log_beta_term =
        log_gamma(a + b) - log_gamma(a) - log_gamma(b) + a * x.ln() + b * (-x).ln_1p();
    let beta_term = log_beta_term.exp();
    let result = if x < (a + 1.0) / (a + b + 2.0) {
        beta_term * beta_continued_fraction(a, b, x)? / a
    } else {
        1.0 - beta_term * beta_continued_fraction(b, a, 1.0 - x)? / b
    };
    if !result.is_finite() {
        return Err(ArError::NumericalFailure);
    }
    Ok(result.clamp(0.0, 1.0))
}

fn beta_continued_fraction(a: f64, b: f64, x: f64) -> Result<f64, ArError> {
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < BETA_FRACTION_MINIMUM {
        d = BETA_FRACTION_MINIMUM;
    }
    d = 1.0 / d;
    let mut h = d;

    for iteration in 1..=BETA_FRACTION_MAX_ITERATIONS {
        let m = iteration as f64;
        let m2 = 2.0 * m;
        let mut aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < BETA_FRACTION_MINIMUM {
            d = BETA_FRACTION_MINIMUM;
        }
        c = 1.0 + aa / c;
        if c.abs() < BETA_FRACTION_MINIMUM {
            c = BETA_FRACTION_MINIMUM;
        }
        d = 1.0 / d;
        h *= d * c;

        aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < BETA_FRACTION_MINIMUM {
            d = BETA_FRACTION_MINIMUM;
        }
        c = 1.0 + aa / c;
        if c.abs() < BETA_FRACTION_MINIMUM {
            c = BETA_FRACTION_MINIMUM;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() <= BETA_FRACTION_EPSILON {
            if h.is_finite() {
                return Ok(h);
            }
            return Err(ArError::NumericalFailure);
        }
    }
    Err(ArError::IncompleteBetaDidNotConverge)
}

fn log_gamma(value: f64) -> f64 {
    // Lanczos approximation, g=7, for the positive arguments used by the F CDF.
    const COEFFICIENTS: [f64; 9] = [
        0.999_999_999_999_809_93,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_13,
        -176.615_029_162_140_59,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_571_6e-6,
        1.505_632_735_149_311_6e-7,
    ];
    let z = value - 1.0;
    let mut x = COEFFICIENTS[0];
    for (index, coefficient) in COEFFICIENTS.iter().enumerate().skip(1) {
        x += coefficient / (z + index as f64);
    }
    let t = z + 7.5;
    0.5 * (2.0 * std::f64::consts::PI).ln() + (z + 0.5) * t.ln() - t + x.ln()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left_value, right_value)| left_value * right_value)
        .sum()
}

fn stable_norm(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |norm, value| norm.hypot(*value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> ClassicalInferenceProfile {
        ClassicalInferenceProfile {
            observations_iid: true,
            structural_errors_homoskedastic: true,
            structural_errors_gaussian: true,
            instrument_exogeneity_and_exclusion: AssumptionEvidence::new(
                AssumptionStatus::SupportedByDesign,
                Some("synthetic-randomization-and-exclusion-contract".to_owned()),
            ),
        }
    }

    fn design(ids: &[&str], confidence: f64) -> ArDesign {
        ArDesign::new(
            ids.iter().map(|id| (*id).to_owned()).collect(),
            confidence,
            RANK_TOLERANCE_DEFAULT,
            profile(),
        )
        .expect("valid AR design")
    }

    #[derive(Clone)]
    struct TestRng {
        state: u64,
        spare_normal: Option<f64>,
    }

    impl TestRng {
        fn new(seed: u64) -> Self {
            Self {
                state: seed,
                spare_normal: None,
            }
        }

        fn next_u64(&mut self) -> u64 {
            self.state ^= self.state >> 12;
            self.state ^= self.state << 25;
            self.state ^= self.state >> 27;
            self.state.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn uniform_open(&mut self) -> f64 {
            let mantissa = self.next_u64() >> 11;
            (mantissa as f64 + 0.5) / ((1_u64 << 53) as f64)
        }

        fn normal(&mut self) -> f64 {
            if let Some(value) = self.spare_normal.take() {
                return value;
            }
            let radius = (-2.0 * self.uniform_open().ln()).sqrt();
            let angle = std::f64::consts::TAU * self.uniform_open();
            self.spare_normal = Some(radius * angle.sin());
            radius * angle.cos()
        }
    }

    fn fixture(seed: u64, n: usize, instrument_strength: f64, beta: f64) -> Vec<ArObservation> {
        let mut rng = TestRng::new(seed);
        let mut rows = Vec::with_capacity(n);
        for _ in 0..n {
            let z = rng.normal();
            let confounder = rng.normal();
            let source_noise = rng.normal();
            let outcome_noise = rng.normal();
            let x = instrument_strength * z + 1.0 * confounder + source_noise;
            let y = beta * x + 1.7 * confounder + outcome_noise;
            rows.push(ArObservation {
                instrument_values: vec![z],
                source_activity: x,
                target_activity: y,
            });
        }
        rows
    }

    #[test]
    fn f_quantile_matches_known_reference_value() {
        let critical = f_quantile(0.95, 1, 120).expect("F quantile");
        assert!((critical - 3.920_124_14).abs() < 2.0e-5);
        assert!((f_cdf(critical, 1, 120).expect("F CDF") - 0.95).abs() < 1.0e-10);
    }

    #[test]
    fn f_survival_function_retains_tiny_tail_probabilities() {
        let critical = f_quantile(0.95, 1, 120).expect("F quantile");
        let upper_tail = f_survival_function(critical, 1, 120).expect("F survival");
        assert!((upper_tail - 0.05).abs() < 1.0e-10);

        // The CDF rounds to 1.0 at this extreme, but direct upper-tail evaluation
        // retains the non-zero probability needed for a meaningful p-value.
        let tiny_tail = f_survival_function(10_000.0, 1, 120).expect("tiny F survival");
        assert!(tiny_tail > 0.0);
        assert!(tiny_tail < 1.0e-100);
    }

    #[test]
    fn quadratic_inversion_preserves_bounded_interval_and_point() {
        assert_eq!(
            quadratic_inequality_set(1.0, -2.0, 0.75, 1.0e-12).expect("interval"),
            ArConfidenceSet::BoundedInterval {
                lower: 0.5,
                upper: 1.5
            }
        );
        assert_eq!(
            quadratic_inequality_set(1.0, 0.0, 0.0, 1.0e-12).expect("point"),
            ArConfidenceSet::Point { value: 0.0 }
        );
    }

    #[test]
    fn quadratic_inversion_preserves_two_rays_and_one_sided_rays() {
        assert_eq!(
            quadratic_inequality_set(-1.0, 2.0, -0.75, 1.0e-12).expect("two rays"),
            ArConfidenceSet::OutsideInterval {
                lower: 0.5,
                upper: 1.5
            }
        );
        assert_eq!(
            quadratic_inequality_set(0.0, 2.0, -4.0, 1.0e-12).expect("lower ray"),
            ArConfidenceSet::LowerRay { upper: 2.0 }
        );
        assert_eq!(
            quadratic_inequality_set(0.0, -2.0, 4.0, 1.0e-12).expect("upper ray"),
            ArConfidenceSet::UpperRay { lower: 2.0 }
        );
    }

    #[test]
    fn quadratic_inversion_preserves_empty_and_all_real_sets() {
        assert_eq!(
            quadratic_inequality_set(1.0, 0.0, 1.0, 1.0e-12).expect("empty"),
            ArConfidenceSet::Empty
        );
        assert_eq!(
            quadratic_inequality_set(-1.0, 0.0, -1.0, 1.0e-12).expect("all real"),
            ArConfidenceSet::AllReal
        );
        assert!(ArConfidenceSet::AllReal.contains(-1.0e12));
        assert!(!ArConfidenceSet::Empty.contains(0.0));
    }

    #[test]
    fn valid_iv_ar_set_contains_true_coefficient_under_confounding() {
        let rows = fixture(21, 2_000, 0.9, 1.4);
        let result =
            fit_anderson_rubin(&rows, &design(&["randomized-pulse"], 0.95), 1.4).expect("AR test");
        assert!(result.confidence_set.contains(1.4));
        assert!(result.p_value >= 0.0 && result.p_value <= 1.0);
        assert_eq!(
            result.causal_qualification,
            CausalQualification::QualifiedByCallerDesignAudit
        );
    }

    #[test]
    fn weak_instrument_does_not_force_a_narrow_conventional_interval() {
        let strong_rows = fixture(22, 2_000, 0.9, 1.4);
        let weak_rows = fixture(22, 2_000, 0.015, 1.4);
        let strong = fit_anderson_rubin(&strong_rows, &design(&["randomized-pulse"], 0.95), 1.4)
            .expect("strong AR");
        let weak = fit_anderson_rubin(&weak_rows, &design(&["randomized-pulse"], 0.95), 1.4)
            .expect("weak AR");

        assert!(strong.confidence_set.contains(1.4));
        assert!(weak.confidence_set.contains(1.4));
        match (
            strong.confidence_set.bounded_width(),
            weak.confidence_set.bounded_width(),
        ) {
            (Some(strong_width), Some(weak_width)) => assert!(weak_width > strong_width),
            (Some(_), None) => assert!(weak.confidence_set.is_unbounded()),
            (None, _) => {}
        }
    }

    #[test]
    fn multiple_independent_instruments_are_supported() {
        let mut rng = TestRng::new(23);
        let mut rows = Vec::with_capacity(3_000);
        for _ in 0..3_000 {
            let z1 = rng.normal();
            let z2 = rng.normal();
            let confounder = rng.normal();
            let x = 0.8 * z1 + 0.5 * z2 + confounder + rng.normal();
            let y = 1.1 * x + 1.3 * confounder + rng.normal();
            rows.push(ArObservation {
                instrument_values: vec![z1, z2],
                source_activity: x,
                target_activity: y,
            });
        }
        let result = fit_anderson_rubin(&rows, &design(&["pulse-a", "pulse-b"], 0.95), 1.1)
            .expect("overidentified AR test");
        assert!(result.confidence_set.contains(1.1));
        assert_eq!(result.numerator_degrees_of_freedom, 2);
    }

    #[test]
    fn instrument_column_permutation_does_not_change_the_ar_statistic() {
        let rows = fixture(24, 1_000, 0.7, 1.25);
        let original =
            fit_anderson_rubin(&rows, &design(&["pulse-a"], 0.95), 1.25).expect("original");
        let scaled_rows: Vec<ArObservation> = rows
            .iter()
            .map(|row| ArObservation {
                instrument_values: vec![100.0 * row.instrument_values[0]],
                source_activity: row.source_activity,
                target_activity: row.target_activity,
            })
            .collect();
        let scaled = fit_anderson_rubin(&scaled_rows, &design(&["pulse-a-scaled"], 0.95), 1.25)
            .expect("scaled");
        match (original.statistic, scaled.statistic) {
            (ArStatistic::Finite(left), ArStatistic::Finite(right)) => {
                assert!((left - right).abs() < 1.0e-9);
            }
            _ => panic!("expected finite AR statistics"),
        }
    }

    #[test]
    fn collinear_instruments_are_rejected() {
        let mut rng = TestRng::new(25);
        let mut rows = Vec::with_capacity(100);
        for _ in 0..100 {
            let z = rng.normal();
            let x = 0.7 * z + rng.normal();
            let y = 1.2 * x + rng.normal();
            rows.push(ArObservation {
                instrument_values: vec![z, 2.0 * z],
                source_activity: x,
                target_activity: y,
            });
        }
        assert_eq!(
            fit_anderson_rubin(&rows, &design(&["z1", "z2"], 0.95), 1.2),
            Err(ArError::RankDeficientInstruments)
        );
    }

    #[test]
    fn classical_f_reference_rejects_time_series_profile() {
        let mut unsupported = profile();
        unsupported.observations_iid = false;
        assert_eq!(
            ArDesign::new(
                vec!["z".to_owned()],
                0.95,
                RANK_TOLERANCE_DEFAULT,
                unsupported
            ),
            Err(ArError::UnsupportedClassicalReferenceProfile)
        );
    }

    #[test]
    fn unknown_instrument_validity_blocks_causal_qualification() {
        let mut assumptions = profile();
        assumptions.instrument_exogeneity_and_exclusion =
            AssumptionEvidence::new(AssumptionStatus::Unknown, None);
        let uncertain_design = ArDesign::new(
            vec!["z".to_owned()],
            0.95,
            RANK_TOLERANCE_DEFAULT,
            assumptions,
        )
        .expect("classical sampling profile is still specified");
        let rows = fixture(26, 1_000, 0.8, 1.4);
        let result = fit_anderson_rubin(&rows, &uncertain_design, 1.4).expect("AR diagnostic");
        assert_eq!(
            result.causal_qualification,
            CausalQualification::NotQualified
        );
    }

    #[test]
    fn malformed_schema_and_observations_fail_closed() {
        assert_eq!(
            ArDesign::new(vec![], 0.95, RANK_TOLERANCE_DEFAULT, profile()),
            Err(ArError::InvalidInstrumentCount)
        );
        assert_eq!(
            ArDesign::new(
                vec!["z".to_owned(), "z".to_owned()],
                0.95,
                RANK_TOLERANCE_DEFAULT,
                profile()
            ),
            Err(ArError::DuplicateInstrumentId)
        );
        let bad_rows = vec![
            ArObservation {
                instrument_values: vec![1.0, 2.0],
                source_activity: 1.0,
                target_activity: 2.0,
            };
            20
        ];
        assert_eq!(
            fit_anderson_rubin(&bad_rows, &design(&["z"], 0.95), 1.0),
            Err(ArError::InconsistentInstrumentWidth)
        );
    }
}
