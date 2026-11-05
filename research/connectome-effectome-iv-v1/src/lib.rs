// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Auditable scalar instrumental-variable estimation for effectome research.
//!
//! This crate estimates a scalar source-to-target response using one or more
//! declared instruments. It does not prove the instrument assumptions.

#![forbid(unsafe_code)]

/// Maximum accepted observation count.
pub const MAX_OBSERVATIONS: usize = 5_000_000;
/// Maximum number of instrument columns.
pub const MAX_INSTRUMENTS: usize = 256;
const RANK_RELATIVE_TOLERANCE: f64 = 1.0e-12;

/// Caller-declared status for one instrumental-variable assumption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssumptionStatus {
    /// The caller has a documented design argument and points to its evidence.
    SupportedByDesign,
    /// The assumption has not been adequately established.
    Unknown,
    /// The design or observed controls contradict the assumption.
    Violated,
}

/// Auditable reference supporting a caller's assumption declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssumptionEvidence {
    pub status: AssumptionStatus,
    /// Stable reference to a protocol, randomization receipt, control or review.
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

/// Instrument assumptions that must be justified by experimental design.
///
/// These declarations are not statistically proven by this crate. In
/// particular, the exclusion restriction is generally not testable from the
/// same regression used to estimate the causal effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentAssumptionAudit {
    /// Assignment is randomized / independent of potential outcomes.
    pub assignment_independence: AssumptionEvidence,
    /// Assignment influences the target only through the declared source.
    pub exclusion_restriction: AssumptionEvidence,
    /// Intervention timing, carryover and washout are specified and defensible.
    pub timing_and_carryover: AssumptionEvidence,
    /// Co-stimulation and network interference are represented or controlled.
    pub interference: AssumptionEvidence,
}

impl InstrumentAssumptionAudit {
    /// Returns assumptions lacking a supported, referenced design argument.
    #[must_use]
    pub fn unsupported_assumptions(&self) -> Vec<InstrumentAssumption> {
        let mut unsupported = Vec::new();
        if !self.assignment_independence.is_supported_and_referenced() {
            unsupported.push(InstrumentAssumption::AssignmentIndependence);
        }
        if !self.exclusion_restriction.is_supported_and_referenced() {
            unsupported.push(InstrumentAssumption::ExclusionRestriction);
        }
        if !self.timing_and_carryover.is_supported_and_referenced() {
            unsupported.push(InstrumentAssumption::TimingAndCarryover);
        }
        if !self.interference.is_supported_and_referenced() {
            unsupported.push(InstrumentAssumption::Interference);
        }
        unsupported
    }

    /// Returns a design-audit qualification, not proof of an assumption.
    #[must_use]
    pub fn qualification(&self) -> CausalQualification {
        let unsupported = self.unsupported_assumptions();
        if unsupported.is_empty() {
            CausalQualification::QualifiedByDeclaredDesignAudit
        } else {
            CausalQualification::NotQualified { unsupported }
        }
    }
}

/// An instrumental-variable assumption not supported by the design audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentAssumption {
    AssignmentIndependence,
    ExclusionRestriction,
    TimingAndCarryover,
    Interference,
}

/// Causal status assigned from caller-supplied assumptions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CausalQualification {
    /// All required assumptions have caller-supplied status and evidence references.
    ///
    /// This is not a statistical proof or a guarantee that the assumptions are true.
    QualifiedByDeclaredDesignAudit,
    /// At least one assumption remains unknown, violated or unsupported by evidence.
    NotQualified {
        unsupported: Vec<InstrumentAssumption>,
    },
}

/// Frozen schema and diagnostics policy for an instrument design.
#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentDesign {
    instrument_ids: Vec<String>,
    assumptions: InstrumentAssumptionAudit,
    minimum_first_stage_f: Option<f64>,
}

impl InstrumentDesign {
    /// Creates and validates a design schema.
    pub fn new(
        instrument_ids: Vec<String>,
        assumptions: InstrumentAssumptionAudit,
        minimum_first_stage_f: Option<f64>,
    ) -> Result<Self, IvError> {
        if instrument_ids.is_empty() || instrument_ids.len() > MAX_INSTRUMENTS {
            return Err(IvError::InvalidInstrumentCount);
        }
        for id in &instrument_ids {
            if id.trim().is_empty() {
                return Err(IvError::BlankInstrumentId);
            }
        }
        for left in 0..instrument_ids.len() {
            if instrument_ids[left + 1..].contains(&instrument_ids[left]) {
                return Err(IvError::DuplicateInstrumentId);
            }
        }
        if minimum_first_stage_f.is_some_and(|value| !value.is_finite() || value <= 0.0) {
            return Err(IvError::InvalidWeakInstrumentThreshold);
        }

        Ok(Self {
            instrument_ids,
            assumptions,
            minimum_first_stage_f,
        })
    }

    /// Returns the frozen instrument column IDs and their required order.
    #[must_use]
    pub fn instrument_ids(&self) -> &[String] {
        &self.instrument_ids
    }

    /// Returns the caller-supplied assumption audit.
    #[must_use]
    pub fn assumptions(&self) -> &InstrumentAssumptionAudit {
        &self.assumptions
    }

    /// Returns the optional caller-selected first-stage F warning threshold.
    #[must_use]
    pub fn minimum_first_stage_f(&self) -> Option<f64> {
        self.minimum_first_stage_f
    }
}

/// One row of a randomized instrument / endogenous source / target observation table.
#[derive(Debug, Clone, PartialEq)]
pub struct IvObservation {
    /// Instrument assignments / values in the frozen schema's column order.
    pub instrument_values: Vec<f64>,
    /// Measured source activity (endogenous regressor).
    pub source_activity: f64,
    /// Measured downstream target response.
    pub target_activity: f64,
}

/// Explicit estimator and validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IvError {
    InvalidInstrumentCount,
    BlankInstrumentId,
    DuplicateInstrumentId,
    InvalidWeakInstrumentThreshold,
    TooManyObservations,
    TooFewObservations,
    InconsistentInstrumentWidth,
    NonFiniteObservation,
    RankDeficientInstrumentMatrix,
    NoSourceVariance,
    UnidentifiedSecondStage,
    NumericalFailure,
}

impl std::fmt::Display for IvError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInstrumentCount => {
                write!(
                    formatter,
                    "instrument count is empty or exceeds the safety limit"
                )
            }
            Self::BlankInstrumentId => write!(formatter, "instrument ID is blank"),
            Self::DuplicateInstrumentId => write!(formatter, "instrument IDs must be unique"),
            Self::InvalidWeakInstrumentThreshold => {
                write!(
                    formatter,
                    "weak-instrument threshold must be positive and finite"
                )
            }
            Self::TooManyObservations => {
                write!(
                    formatter,
                    "observation count exceeds the configured safety limit"
                )
            }
            Self::TooFewObservations => {
                write!(formatter, "too few observations for the instrument design")
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
            Self::RankDeficientInstrumentMatrix => {
                write!(
                    formatter,
                    "instrument matrix is rank deficient or numerically singular"
                )
            }
            Self::NoSourceVariance => write!(formatter, "source activity has no sample variance"),
            Self::UnidentifiedSecondStage => {
                write!(
                    formatter,
                    "instrumented source activity does not identify the second stage"
                )
            }
            Self::NumericalFailure => {
                write!(formatter, "estimator encountered a numerical failure")
            }
        }
    }
}

impl std::error::Error for IvError {}

/// First-stage F statistic; perfect prediction is represented without infinity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FirstStageF {
    Finite(f64),
    PerfectFit,
}

/// Result from the scalar 2SLS estimator.
#[derive(Debug, Clone, PartialEq)]
pub struct IvResult {
    pub observations: usize,
    pub instrument_ids: Vec<String>,
    /// 2SLS estimate of target response per unit source activity.
    pub iv_coefficient: f64,
    /// Intercept implied by the centered second-stage fit.
    pub iv_intercept: f64,
    /// HC0 heteroskedasticity-robust standard error for the 2SLS coefficient.
    pub iv_standard_error_hc0: f64,
    /// OLS comparator, not a causal estimator under endogenous source activity.
    pub ols_coefficient: f64,
    pub ols_intercept: f64,
    /// Intercept followed by one coefficient per declared instrument.
    pub first_stage_coefficients: Vec<f64>,
    /// First-stage R-squared including an intercept.
    pub first_stage_r_squared: f64,
    /// Conventional first-stage F statistic; no robust-Wald correction is applied.
    pub first_stage_f_statistic: FirstStageF,
    /// Whether the caller's optional first-stage warning threshold was breached.
    pub weak_first_stage: bool,
    pub causal_qualification: CausalQualification,
}

impl IvResult {
    /// Returns coefficient / HC0 standard error when the standard error is positive.
    ///
    /// This diagnostic is not a p-value or a weak-IV-robust confidence interval.
    #[must_use]
    pub fn robust_t_statistic(&self) -> Option<f64> {
        if self.iv_standard_error_hc0 > 0.0 {
            Some(self.iv_coefficient / self.iv_standard_error_hc0)
        } else {
            None
        }
    }
}

/// Fits scalar two-stage least squares with an intercept and one or more instruments.
///
/// The function reports statistical diagnostics separately from the caller's
/// assumption audit. It does not prove randomization or the exclusion restriction.
pub fn fit_two_stage_least_squares(
    observations: &[IvObservation],
    design: &InstrumentDesign,
) -> Result<IvResult, IvError> {
    let n = observations.len();
    let instrument_ids = design.instrument_ids();
    let q = instrument_ids.len();

    if n > MAX_OBSERVATIONS {
        return Err(IvError::TooManyObservations);
    }
    if n <= q + 1 {
        return Err(IvError::TooFewObservations);
    }

    for observation in observations {
        if observation.instrument_values.len() != q {
            return Err(IvError::InconsistentInstrumentWidth);
        }
        if !observation.source_activity.is_finite()
            || !observation.target_activity.is_finite()
            || observation
                .instrument_values
                .iter()
                .any(|value| !value.is_finite())
        {
            return Err(IvError::NonFiniteObservation);
        }
    }

    let mean_x = observations
        .iter()
        .map(|observation| observation.source_activity)
        .sum::<f64>()
        / n as f64;
    let mean_y = observations
        .iter()
        .map(|observation| observation.target_activity)
        .sum::<f64>()
        / n as f64;
    let mut mean_z = vec![0.0; q];
    for observation in observations {
        for (index, value) in observation.instrument_values.iter().enumerate() {
            mean_z[index] += value / n as f64;
        }
    }

    let x: Vec<f64> = observations
        .iter()
        .map(|observation| observation.source_activity - mean_x)
        .collect();
    let y: Vec<f64> = observations
        .iter()
        .map(|observation| observation.target_activity - mean_y)
        .collect();
    let z: Vec<Vec<f64>> = observations
        .iter()
        .map(|observation| {
            observation
                .instrument_values
                .iter()
                .zip(&mean_z)
                .map(|(value, mean)| value - mean)
                .collect()
        })
        .collect();

    let source_sum_squares = dot(&x, &x);
    if !source_sum_squares.is_finite() || source_sum_squares <= f64::EPSILON {
        return Err(IvError::NoSourceVariance);
    }

    let mut gram = vec![vec![0.0; q]; q];
    let mut cross_x = vec![0.0; q];
    for row in 0..n {
        for col in 0..q {
            cross_x[col] += z[row][col] * x[row];
            for other in 0..q {
                gram[col][other] += z[row][col] * z[row][other];
            }
        }
    }

    let first_stage_slopes = solve_positive_definite(&gram, &cross_x)?;
    let x_hat: Vec<f64> = z.iter().map(|row| dot(row, &first_stage_slopes)).collect();
    let explained_sum_squares = dot(&x_hat, &x_hat);
    let first_stage_sse = x
        .iter()
        .zip(&x_hat)
        .map(|(actual, fitted)| (actual - fitted).powi(2))
        .sum::<f64>();
    if !explained_sum_squares.is_finite() || !first_stage_sse.is_finite() {
        return Err(IvError::NumericalFailure);
    }

    let first_stage_r_squared = (explained_sum_squares / source_sum_squares).clamp(0.0, 1.0);
    let first_stage_f_statistic = if first_stage_sse <= f64::EPSILON {
        FirstStageF::PerfectFit
    } else {
        let residual_degrees = n - q - 1;
        let statistic =
            (explained_sum_squares / q as f64) / (first_stage_sse / residual_degrees as f64);
        if !statistic.is_finite() || statistic < 0.0 {
            return Err(IvError::NumericalFailure);
        }
        FirstStageF::Finite(statistic)
    };

    let iv_denominator = dot(&x_hat, &x);
    if !iv_denominator.is_finite() || iv_denominator.abs() <= f64::EPSILON {
        return Err(IvError::UnidentifiedSecondStage);
    }
    let iv_coefficient = dot(&x_hat, &y) / iv_denominator;
    let iv_intercept = mean_y - iv_coefficient * mean_x;

    let iv_residuals: Vec<f64> = observations
        .iter()
        .map(|observation| {
            observation.target_activity
                - iv_intercept
                - iv_coefficient * observation.source_activity
        })
        .collect();
    let robust_meat = x_hat
        .iter()
        .zip(&iv_residuals)
        .map(|(fitted_x, residual)| (fitted_x * residual).powi(2))
        .sum::<f64>();
    let iv_standard_error_hc0 = (robust_meat / iv_denominator.powi(2)).sqrt();

    let ols_coefficient = dot(&x, &y) / source_sum_squares;
    let ols_intercept = mean_y - ols_coefficient * mean_x;

    if !iv_coefficient.is_finite()
        || !iv_intercept.is_finite()
        || !iv_standard_error_hc0.is_finite()
        || !ols_coefficient.is_finite()
        || !ols_intercept.is_finite()
    {
        return Err(IvError::NumericalFailure);
    }

    let weak_first_stage = match (design.minimum_first_stage_f(), first_stage_f_statistic) {
        (Some(threshold), FirstStageF::Finite(value)) => value < threshold,
        (Some(_), FirstStageF::PerfectFit) | (None, _) => false,
    };

    let mut first_stage_coefficients = Vec::with_capacity(q + 1);
    let first_stage_intercept = mean_x - dot(&mean_z, &first_stage_slopes);
    first_stage_coefficients.push(first_stage_intercept);
    first_stage_coefficients.extend(first_stage_slopes);

    Ok(IvResult {
        observations: n,
        instrument_ids: instrument_ids.to_vec(),
        iv_coefficient,
        iv_intercept,
        iv_standard_error_hc0,
        ols_coefficient,
        ols_intercept,
        first_stage_coefficients,
        first_stage_r_squared,
        first_stage_f_statistic,
        weak_first_stage,
        causal_qualification: design.assumptions().qualification(),
    })
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left_value, right_value)| left_value * right_value)
        .sum()
}

/// Solves a symmetric positive-definite linear system using Cholesky factorization.
fn solve_positive_definite(matrix: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>, IvError> {
    let size = rhs.len();
    if matrix.len() != size || matrix.iter().any(|row| row.len() != size) {
        return Err(IvError::RankDeficientInstrumentMatrix);
    }

    let scale = matrix
        .iter()
        .enumerate()
        .map(|(index, row)| row[index].abs())
        .fold(0.0_f64, f64::max);
    if !scale.is_finite() || scale <= 0.0 {
        return Err(IvError::RankDeficientInstrumentMatrix);
    }

    let mut lower = vec![vec![0.0; size]; size];
    for row in 0..size {
        for col in 0..=row {
            let mut value = matrix[row][col];
            for (left_value, right_value) in lower[row][..col].iter().zip(&lower[col][..col]) {
                value -= left_value * right_value;
            }
            if row == col {
                if !value.is_finite() || value <= scale * RANK_RELATIVE_TOLERANCE {
                    return Err(IvError::RankDeficientInstrumentMatrix);
                }
                lower[row][col] = value.sqrt();
            } else {
                lower[row][col] = value / lower[col][col];
            }
        }
    }

    let mut intermediate = vec![0.0; size];
    for row in 0..size {
        let mut value = rhs[row];
        for col in 0..row {
            value -= lower[row][col] * intermediate[col];
        }
        intermediate[row] = value / lower[row][row];
    }

    let mut solution = vec![0.0; size];
    for row in (0..size).rev() {
        let mut value = intermediate[row];
        for col in (row + 1)..size {
            value -= lower[col][row] * solution[col];
        }
        solution[row] = value / lower[row][row];
    }

    if solution.iter().any(|value| !value.is_finite()) {
        return Err(IvError::NumericalFailure);
    }
    Ok(solution)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supported_evidence(reference: &str) -> AssumptionEvidence {
        AssumptionEvidence::new(
            AssumptionStatus::SupportedByDesign,
            Some(reference.to_owned()),
        )
    }

    fn valid_audit() -> InstrumentAssumptionAudit {
        InstrumentAssumptionAudit {
            assignment_independence: supported_evidence("protocol/randomization-v1"),
            exclusion_restriction: supported_evidence("review/exclusion-v1"),
            timing_and_carryover: supported_evidence("protocol/timing-v1"),
            interference: supported_evidence("protocol/interference-v1"),
        }
    }

    fn design(ids: Vec<&str>, threshold: Option<f64>) -> InstrumentDesign {
        InstrumentDesign::new(
            ids.into_iter().map(str::to_owned).collect(),
            valid_audit(),
            threshold,
        )
        .expect("valid design")
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

    fn confounded_fixture(
        seed: u64,
        count: usize,
        instrument_effect: f64,
        direct_instrument_effect: f64,
    ) -> Vec<IvObservation> {
        let mut rng = TestRng::new(seed);
        let mut rows = Vec::with_capacity(count);
        for _ in 0..count {
            let z = rng.normal();
            let confounder = rng.normal();
            let source_noise = rng.normal();
            let target_noise = rng.normal();
            let source = instrument_effect * z + 1.4 * confounder + source_noise;
            let target =
                1.75 * source + 2.0 * confounder + direct_instrument_effect * z + target_noise;
            rows.push(IvObservation {
                instrument_values: vec![z],
                source_activity: source,
                target_activity: target,
            });
        }
        rows
    }

    #[test]
    fn valid_instrument_recovers_effect_under_hidden_confounding() {
        let rows = confounded_fixture(11, 40_000, 0.8, 0.0);
        let result =
            fit_two_stage_least_squares(&rows, &design(vec!["randomized_pulse"], Some(10.0)))
                .expect("valid IV fit");

        assert!((result.iv_coefficient - 1.75).abs() < 0.04);
        assert!((result.ols_coefficient - 1.75).abs() > 0.5);
        assert!(!result.weak_first_stage);
        assert_eq!(
            result.causal_qualification,
            CausalQualification::QualifiedByDeclaredDesignAudit
        );
        assert!(result.iv_standard_error_hc0 > 0.0);
    }

    #[test]
    fn direct_instrument_to_outcome_path_biases_iv_and_audit_blocks_qualification() {
        let rows = confounded_fixture(12, 40_000, 0.8, 0.9);
        let mut audit = valid_audit();
        audit.exclusion_restriction = AssumptionEvidence::new(
            AssumptionStatus::Violated,
            Some("negative-control/direct-light-path".to_owned()),
        );
        let invalid_design =
            InstrumentDesign::new(vec!["randomized_pulse".to_owned()], audit, Some(10.0))
                .expect("valid design");

        let result = fit_two_stage_least_squares(&rows, &invalid_design).expect("numerical IV fit");
        assert!((result.iv_coefficient - 1.75).abs() > 0.6);
        assert_eq!(
            result.causal_qualification,
            CausalQualification::NotQualified {
                unsupported: vec![InstrumentAssumption::ExclusionRestriction]
            }
        );
    }

    #[test]
    fn weak_first_stage_is_reported_without_a_universal_default_cutoff() {
        let mut rng = TestRng::new(13);
        let mut rows = Vec::with_capacity(2_000);
        for _ in 0..2_000 {
            let z = rng.normal();
            let source = rng.normal();
            let target = 1.75 * source + rng.normal();
            rows.push(IvObservation {
                instrument_values: vec![z],
                source_activity: source,
                target_activity: target,
            });
        }

        let warning =
            fit_two_stage_least_squares(&rows, &design(vec!["weak_randomizer"], Some(10.0)))
                .expect("weak-IV fit remains numerically estimable");
        assert!(warning.weak_first_stage);

        let no_cutoff = fit_two_stage_least_squares(&rows, &design(vec!["weak_randomizer"], None))
            .expect("fit without policy threshold");
        assert!(!no_cutoff.weak_first_stage);
        assert!(
            matches!(no_cutoff.first_stage_f_statistic, FirstStageF::Finite(value) if value < 10.0)
        );
    }

    #[test]
    fn multiple_independent_instruments_recover_a_confounded_effect() {
        let mut rng = TestRng::new(15);
        let mut rows = Vec::with_capacity(30_000);
        for _ in 0..30_000 {
            let z1 = rng.normal();
            let z2 = rng.normal();
            let confounder = rng.normal();
            let source = 0.9 * z1 + 0.6 * z2 + 1.2 * confounder + rng.normal();
            let target = 2.2 * source + 1.7 * confounder + rng.normal();
            rows.push(IvObservation {
                instrument_values: vec![z1, z2],
                source_activity: source,
                target_activity: target,
            });
        }

        let result =
            fit_two_stage_least_squares(&rows, &design(vec!["pulse-a", "pulse-b"], Some(10.0)))
                .expect("two-instrument IV fit");
        assert!((result.iv_coefficient - 2.2).abs() < 0.05);
        assert!(!result.weak_first_stage);
        assert_eq!(
            result.instrument_ids,
            vec!["pulse-a".to_owned(), "pulse-b".to_owned()]
        );
    }

    #[test]
    fn collinear_instrument_columns_are_rejected() {
        let rows: Vec<IvObservation> = (0..50)
            .map(|index| {
                let z = index as f64;
                IvObservation {
                    instrument_values: vec![z, 2.0 * z],
                    source_activity: 3.0 * z,
                    target_activity: 2.0 * z,
                }
            })
            .collect();
        let instrument_design = design(vec!["z1", "z2"], None);
        assert_eq!(
            fit_two_stage_least_squares(&rows, &instrument_design),
            Err(IvError::RankDeficientInstrumentMatrix)
        );
    }

    #[test]
    fn exact_known_answer_returns_perfect_first_stage() {
        let rows: Vec<IvObservation> = (0..20)
            .map(|index| {
                let z = index as f64 - 9.5;
                let x = 2.0 * z;
                IvObservation {
                    instrument_values: vec![z],
                    source_activity: x,
                    target_activity: 3.0 + 4.0 * x,
                }
            })
            .collect();
        let result =
            fit_two_stage_least_squares(&rows, &design(vec!["z"], None)).expect("exact linear fit");

        assert!((result.iv_coefficient - 4.0).abs() < 1.0e-12);
        assert!((result.iv_intercept - 3.0).abs() < 1.0e-12);
        assert_eq!(result.first_stage_f_statistic, FirstStageF::PerfectFit);
        assert!(result.iv_standard_error_hc0 < 1.0e-10);
    }

    #[test]
    fn row_order_does_not_change_estimate() {
        let rows = confounded_fixture(14, 5_000, 1.0, 0.0);
        let forward =
            fit_two_stage_least_squares(&rows, &design(vec!["z"], None)).expect("forward order");
        let mut reverse_rows = rows.clone();
        reverse_rows.reverse();
        let reverse = fit_two_stage_least_squares(&reverse_rows, &design(vec!["z"], None))
            .expect("reverse order");

        assert!((forward.iv_coefficient - reverse.iv_coefficient).abs() < 1.0e-10);
        assert!((forward.ols_coefficient - reverse.ols_coefficient).abs() < 1.0e-10);
    }

    #[test]
    fn invalid_design_schema_fails_closed() {
        assert_eq!(
            InstrumentDesign::new(vec![], valid_audit(), None),
            Err(IvError::InvalidInstrumentCount)
        );
        assert_eq!(
            InstrumentDesign::new(vec![" ".to_owned()], valid_audit(), None),
            Err(IvError::BlankInstrumentId)
        );
        assert_eq!(
            InstrumentDesign::new(
                vec!["same".to_owned(), "same".to_owned()],
                valid_audit(),
                None
            ),
            Err(IvError::DuplicateInstrumentId)
        );
        assert_eq!(
            InstrumentDesign::new(vec!["z".to_owned()], valid_audit(), Some(0.0)),
            Err(IvError::InvalidWeakInstrumentThreshold)
        );
    }

    #[test]
    fn malformed_observations_fail_closed() {
        let valid_design = design(vec!["z"], None);
        let width_mismatch: Vec<IvObservation> = (0..20)
            .map(|index| {
                let value = index as f64;
                IvObservation {
                    instrument_values: vec![value, value + 1.0],
                    source_activity: value,
                    target_activity: value,
                }
            })
            .collect();
        assert_eq!(
            fit_two_stage_least_squares(&width_mismatch, &valid_design),
            Err(IvError::InconsistentInstrumentWidth)
        );

        let rows: Vec<IvObservation> = (0..20)
            .map(|index| {
                let value = index as f64;
                IvObservation {
                    instrument_values: vec![value],
                    source_activity: value,
                    target_activity: value,
                }
            })
            .collect();
        let mut non_finite = rows;
        non_finite[0].target_activity = f64::NAN;
        assert_eq!(
            fit_two_stage_least_squares(&non_finite, &valid_design),
            Err(IvError::NonFiniteObservation)
        );
    }

    #[test]
    fn supported_status_without_evidence_does_not_qualify() {
        let mut audit = valid_audit();
        audit.exclusion_restriction =
            AssumptionEvidence::new(AssumptionStatus::SupportedByDesign, Some(" ".to_owned()));
        assert_eq!(
            audit.qualification(),
            CausalQualification::NotQualified {
                unsupported: vec![InstrumentAssumption::ExclusionRestriction]
            }
        );
    }
}
