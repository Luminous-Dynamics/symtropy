//! Two-parameter weighted identifiability reference kernel.
//!
//! This is a manufactured linear-algebra benchmark, not an EHD parameter
//! estimator. It supports diagonal observation covariance and two parameters.
//! Correlated covariance, nonlinear profile likelihood and model discrimination
//! remain separate future work and are never silently approximated here.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentificationDisposition {
    Identifiable,
    NotIdentifiable,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdentificationReport {
    pub information_matrix: [[f64; 2]; 2],
    pub eigenvalues: [f64; 2],
    pub singular_values: [f64; 2],
    pub numerical_rank: usize,
    pub relative_rank_tolerance: f64,
    pub condition_number: Option<f64>,
    pub weakest_parameter_direction_scaled: [f64; 2],
    pub disposition: IdentificationDisposition,
    /// Physical-coordinate parameter estimate; absent unless full rank.
    pub parameter_estimate: Option<[f64; 2]>,
    pub weighted_residual_sum_squares: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IdentifiabilityError {
    EmptyDesign,
    LengthMismatch,
    NonFiniteDesign { row: usize, column: usize },
    NonFiniteObservation { row: usize },
    InvalidVariance { row: usize },
    InvalidParameterScale { parameter: usize },
    InvalidRankTolerance,
    NonFiniteDerivedValue,
    SingularSolveAfterRankCheck,
    InvalidCandidateCount,
    InvalidObservationBudget,
    SearchSpaceLimitExceeded { limit: usize },
    NoIdentifiableDesign,
}

impl fmt::Display for IdentifiabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDesign => write!(f, "identifiability design must contain at least one row"),
            Self::LengthMismatch => write!(f, "design, observations and variances must have equal row counts"),
            Self::NonFiniteDesign { row, column } => write!(f, "design row {row} column {column} is non-finite"),
            Self::NonFiniteObservation { row } => write!(f, "observation row {row} is non-finite"),
            Self::InvalidVariance { row } => write!(f, "observation variance at row {row} must be finite and positive"),
            Self::InvalidParameterScale { parameter } => write!(f, "parameter scale {parameter} must be finite and positive"),
            Self::InvalidRankTolerance => write!(f, "relative rank tolerance must be finite and in (0, 1)"),
            Self::NonFiniteDerivedValue => write!(f, "identifiability calculation produced a non-finite value"),
            Self::SingularSolveAfterRankCheck => write!(f, "normal-equation solve was singular after full-rank classification"),
            Self::InvalidCandidateCount => write!(f, "design tournament requires at least one candidate condition"),
            Self::InvalidObservationBudget => write!(f, "design tournament observation budget must be between 1 and 32"),
            Self::SearchSpaceLimitExceeded { limit } => write!(f, "design tournament search exceeded its evaluation limit of {limit}"),
            Self::NoIdentifiableDesign => write!(f, "no full-rank design exists within the candidate set and observation budget"),
        }
    }
}

impl Error for IdentifiabilityError {}

/// Analyse a two-parameter design with independent observation errors.
///
/// Design columns are derivatives with respect to physical parameters. Each
/// column is multiplied by its declared parameter scale before rank analysis.
/// The observation covariance is diagonal; correlated errors are unsupported.
pub fn analyse_two_parameter_design(
    design_rows: &[[f64; 2]],
    observations: &[f64],
    observation_variances: &[f64],
    parameter_scales: [f64; 2],
    relative_rank_tolerance: f64,
) -> Result<IdentificationReport, IdentifiabilityError> {
    let n = design_rows.len();
    if n == 0 {
        return Err(IdentifiabilityError::EmptyDesign);
    }
    if observations.len() != n || observation_variances.len() != n {
        return Err(IdentifiabilityError::LengthMismatch);
    }
    if !relative_rank_tolerance.is_finite()
        || relative_rank_tolerance <= 0.0
        || relative_rank_tolerance >= 1.0
    {
        return Err(IdentifiabilityError::InvalidRankTolerance);
    }
    for (parameter, scale) in parameter_scales.iter().enumerate() {
        if !scale.is_finite() || *scale <= 0.0 {
            return Err(IdentifiabilityError::InvalidParameterScale { parameter });
        }
    }

    let mut g00 = 0.0_f64;
    let mut g01 = 0.0_f64;
    let mut g11 = 0.0_f64;
    let mut r0 = 0.0_f64;
    let mut r1 = 0.0_f64;

    for row in 0..n {
        for column in 0..2 {
            if !design_rows[row][column].is_finite() {
                return Err(IdentifiabilityError::NonFiniteDesign { row, column });
            }
        }
        if !observations[row].is_finite() {
            return Err(IdentifiabilityError::NonFiniteObservation { row });
        }
        let variance = observation_variances[row];
        if !variance.is_finite() || variance <= 0.0 {
            return Err(IdentifiabilityError::InvalidVariance { row });
        }

        let weight = 1.0 / variance;
        let x0 = design_rows[row][0] * parameter_scales[0];
        let x1 = design_rows[row][1] * parameter_scales[1];
        g00 = (weight * x0).mul_add(x0, g00);
        g01 = (weight * x0).mul_add(x1, g01);
        g11 = (weight * x1).mul_add(x1, g11);
        r0 = (weight * x0).mul_add(observations[row], r0);
        r1 = (weight * x1).mul_add(observations[row], r1);
    }

    if ![g00, g01, g11, r0, r1].iter().all(|value| value.is_finite()) {
        return Err(IdentifiabilityError::NonFiniteDerivedValue);
    }

    // One-sided Jacobi rotation on the two weighted design columns. This
    // computes the singular values from the rotated columns directly instead
    // of subtracting nearly equal eigenvalues of X^T X (which squares the
    // condition number and can erase a small singular direction).
    let tau = if g01 == 0.0 {
        0.0
    } else {
        (g11 - g00) / (2.0 * g01)
    };
    let tangent = if g01 == 0.0 {
        0.0
    } else {
        let sign = if tau.is_sign_negative() { -1.0 } else { 1.0 };
        sign / (tau.abs() + tau.hypot(1.0))
    };
    let cosine = 1.0 / tangent.hypot(1.0);
    let sine = tangent * cosine;

    let mut rotated_norm_0 = 0.0_f64;
    let mut rotated_norm_1 = 0.0_f64;
    for row in 0..n {
        let inverse_sigma = 1.0 / observation_variances[row].sqrt();
        let x0 = design_rows[row][0] * parameter_scales[0] * inverse_sigma;
        let x1 = design_rows[row][1] * parameter_scales[1] * inverse_sigma;
        let rotated_0 = cosine.mul_add(x0, -sine * x1);
        let rotated_1 = sine.mul_add(x0, cosine * x1);
        rotated_norm_0 = rotated_0.mul_add(rotated_0, rotated_norm_0);
        rotated_norm_1 = rotated_1.mul_add(rotated_1, rotated_norm_1);
    }
    if !rotated_norm_0.is_finite() || !rotated_norm_1.is_finite() {
        return Err(IdentifiabilityError::NonFiniteDerivedValue);
    }
    let lambda_max = rotated_norm_0.max(rotated_norm_1);
    let lambda_min = rotated_norm_0.min(rotated_norm_1);
    let eigenvalues = [lambda_max, lambda_min];
    let singular_values = [lambda_max.sqrt(), lambda_min.sqrt()];
    let weakest_parameter_direction_scaled = if rotated_norm_0 <= rotated_norm_1 {
        [cosine, -sine]
    } else {
        [sine, cosine]
    };
    let threshold = relative_rank_tolerance * singular_values[0];
    let numerical_rank = singular_values
        .iter()
        .filter(|value| **value > threshold)
        .count();

    let (condition_number, disposition, parameter_estimate, weighted_residual_sum_squares) =
        if numerical_rank == 2 {
            // Solve in the symmetric eigensystem rather than invert the Gram
            // matrix by determinant subtraction, which loses precision near
            // the declared rank threshold.
            if lambda_min <= 0.0 || lambda_max <= 0.0 {
                return Err(IdentifiabilityError::SingularSolveAfterRankCheck);
            }
            let v_min = weakest_parameter_direction_scaled;
            let v_max = [-v_min[1], v_min[0]];
            let projection_min = v_min[0] * r0 + v_min[1] * r1;
            let projection_max = v_max[0] * r0 + v_max[1] * r1;
            let phi0 = v_min[0] * (projection_min / lambda_min)
                + v_max[0] * (projection_max / lambda_max);
            let phi1 = v_min[1] * (projection_min / lambda_min)
                + v_max[1] * (projection_max / lambda_max);
            let estimate = [
                parameter_scales[0] * phi0,
                parameter_scales[1] * phi1,
            ];
            if !estimate.iter().all(|value| value.is_finite()) {
                return Err(IdentifiabilityError::NonFiniteDerivedValue);
            }

            let mut residual_sum = 0.0_f64;
            for row in 0..n {
                let predicted =
                    design_rows[row][0] * estimate[0] + design_rows[row][1] * estimate[1];
                let residual = observations[row] - predicted;
                residual_sum += residual * residual / observation_variances[row];
            }
            if !residual_sum.is_finite() {
                return Err(IdentifiabilityError::NonFiniteDerivedValue);
            }
            (
                Some(singular_values[0] / singular_values[1]),
                IdentificationDisposition::Identifiable,
                Some(estimate),
                Some(residual_sum),
            )
        } else {
            (None, IdentificationDisposition::NotIdentifiable, None, None)
        };

    Ok(IdentificationReport {
        information_matrix: [[g00, g01], [g01, g11]],
        eigenvalues,
        singular_values,
        numerical_rank,
        relative_rank_tolerance,
        condition_number,
        weakest_parameter_direction_scaled,
        disposition,
        parameter_estimate,
        weighted_residual_sum_squares,
    })
}

/// Manufactured negative/positive reference pair used by EHD-OED-001.
pub fn reference_self_check() -> Result<(), IdentifiabilityError> {
    let repeated_design = [[1.0, 1.0], [1.0, 1.0], [1.0, 1.0]];
    let repeated_observations = [2.5, 2.5, 2.5];
    let unit_variances = [1.0, 1.0, 1.0];
    let repeated = analyse_two_parameter_design(
        &repeated_design, &repeated_observations, &unit_variances, [1.0, 1.0], 1.0e-12,
    )?;
    if repeated.information_matrix != [[3.0, 3.0], [3.0, 3.0]]
        || (repeated.eigenvalues[0] - 6.0).abs() > 1.0e-12
        || repeated.eigenvalues[1].abs() > 1.0e-12
        || repeated.numerical_rank != 1
        || repeated.disposition != IdentificationDisposition::NotIdentifiable
        || repeated.parameter_estimate.is_some()
        || (repeated.weakest_parameter_direction_scaled[0].abs()
            - std::f64::consts::FRAC_1_SQRT_2).abs() > 1.0e-12
        || (repeated.weakest_parameter_direction_scaled[1].abs()
            - std::f64::consts::FRAC_1_SQRT_2).abs() > 1.0e-12
    {
        return Err(IdentifiabilityError::NonFiniteDerivedValue);
    }

    let balanced_design = [[-1.0, 1.0], [0.0, 1.0], [1.0, 1.0]];
    let balanced_observations = [-1.5, 0.5, 2.5];
    let balanced = analyse_two_parameter_design(
        &balanced_design, &balanced_observations, &unit_variances, [1.0, 1.0], 1.0e-12,
    )?;
    if balanced.information_matrix != [[2.0, 0.0], [0.0, 3.0]]
        || balanced.eigenvalues != [3.0, 2.0]
        || balanced.numerical_rank != 2
        || balanced.disposition != IdentificationDisposition::Identifiable
        || balanced.parameter_estimate.is_none_or(|estimate| {
            (estimate[0] - 2.0).abs() > 1.0e-12 || (estimate[1] - 0.5).abs() > 1.0e-12
        })
        || balanced.weighted_residual_sum_squares.is_none_or(|value| value > 1.0e-24)
    {
        return Err(IdentifiabilityError::NonFiniteDerivedValue);
    }
    Ok(())
}


/// One admissible candidate observation condition in a manufactured design
/// tournament. This is a sensitivity row and variance, not a physical EHD
/// voltage/current operating point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateCondition {
    pub sensitivity: [f64; 2],
    pub observation_variance: f64,
}

/// A full-rank design score at a fixed observation budget.
#[derive(Clone, Debug, PartialEq)]
pub struct DesignScore {
    pub candidate_indices: Vec<usize>,
    pub information_matrix: [[f64; 2]; 2],
    pub singular_values: [f64; 2],
    pub minimum_information_eigenvalue: f64,
    pub log_determinant_information: f64,
    pub condition_number: f64,
}

/// D-optimal and E-optimal selections evaluated on the same candidate set
/// and identical observation budget.
#[derive(Clone, Debug, PartialEq)]
pub struct DesignTournament {
    pub observation_budget: usize,
    pub evaluated_design_count: usize,
    pub identifiable_design_count: usize,
    pub d_optimal: DesignScore,
    pub e_optimal: DesignScore,
}

fn enumerate_designs(
    candidate_count: usize,
    start_index: usize,
    remaining: usize,
    current: &mut Vec<usize>,
    output: &mut Vec<Vec<usize>>,
    max_evaluations: usize,
) -> Result<(), IdentifiabilityError> {
    if remaining == 0 {
        if output.len() >= max_evaluations {
            return Err(IdentifiabilityError::SearchSpaceLimitExceeded {
                limit: max_evaluations,
            });
        }
        output.push(current.clone());
        return Ok(());
    }
    for index in start_index..candidate_count {
        current.push(index);
        enumerate_designs(
            candidate_count,
            index,
            remaining - 1,
            current,
            output,
            max_evaluations,
        )?;
        current.pop();
    }
    Ok(())
}

fn score_candidate_design(
    indices: &[usize],
    candidates: &[CandidateCondition],
    parameter_scales: [f64; 2],
    relative_rank_tolerance: f64,
) -> Result<Option<DesignScore>, IdentifiabilityError> {
    let rows: Vec<[f64; 2]> = indices
        .iter()
        .map(|index| candidates[*index].sensitivity)
        .collect();
    let variances: Vec<f64> = indices
        .iter()
        .map(|index| candidates[*index].observation_variance)
        .collect();
    let observations = vec![0.0; indices.len()];
    let report = analyse_two_parameter_design(
        &rows,
        &observations,
        &variances,
        parameter_scales,
        relative_rank_tolerance,
    )?;
    if report.numerical_rank != 2 {
        return Ok(None);
    }
    let sigma_max = report.singular_values[0];
    let sigma_min = report.singular_values[1];
    let minimum_information_eigenvalue = sigma_min * sigma_min;
    let log_determinant_information =
        2.0 * sigma_max.ln() + 2.0 * sigma_min.ln();
    let condition_number = sigma_max / sigma_min;
    if !minimum_information_eigenvalue.is_finite()
        || !log_determinant_information.is_finite()
        || !condition_number.is_finite()
    {
        return Err(IdentifiabilityError::NonFiniteDerivedValue);
    }
    Ok(Some(DesignScore {
        candidate_indices: indices.to_vec(),
        information_matrix: report.information_matrix,
        singular_values: report.singular_values,
        minimum_information_eigenvalue,
        log_determinant_information,
        condition_number,
    }))
}

fn is_better_d(candidate: &DesignScore, incumbent: &DesignScore) -> bool {
    candidate
        .log_determinant_information
        .total_cmp(&incumbent.log_determinant_information)
        .is_gt()
        || (candidate.log_determinant_information
            .total_cmp(&incumbent.log_determinant_information)
            .is_eq()
            && candidate.candidate_indices < incumbent.candidate_indices)
}

fn is_better_e(candidate: &DesignScore, incumbent: &DesignScore) -> bool {
    // Maximizing the smallest singular value is exactly equivalent to
    // maximizing the smallest information eigenvalue, while avoiding
    // underflow from squaring an extremely small but rank-resolvable value.
    candidate.singular_values[1]
        .total_cmp(&incumbent.singular_values[1])
        .is_gt()
        || (candidate.singular_values[1]
            .total_cmp(&incumbent.singular_values[1])
            .is_eq()
            && candidate.candidate_indices < incumbent.candidate_indices)
}

/// Exhaustively compare D-optimal and E-optimal designs for a small discrete
/// candidate set at a fixed observation count. Each selected observation has
/// unit resource cost; heteroscedastic measurement variance is supported.
///
/// D-optimal maximizes log(det(F)); E-optimal maximizes the weakest eigenvalue
/// of F. Rank-deficient designs are excluded from both selections. The search
/// is deterministic, lexicographically tie-broken and bounded by a fixed cap.
/// Hard recursion bound for this small exhaustive reference tournament.
pub const MAX_DISCRETE_OBSERVATION_BUDGET: usize = 32;

pub fn select_discrete_designs(
    candidates: &[CandidateCondition],
    observation_budget: usize,
    parameter_scales: [f64; 2],
    relative_rank_tolerance: f64,
    max_evaluations: usize,
) -> Result<DesignTournament, IdentifiabilityError> {
    if candidates.is_empty() {
        return Err(IdentifiabilityError::InvalidCandidateCount);
    }
    if observation_budget == 0 || observation_budget > MAX_DISCRETE_OBSERVATION_BUDGET {
        return Err(IdentifiabilityError::InvalidObservationBudget);
    }
    if max_evaluations == 0 {
        return Err(IdentifiabilityError::SearchSpaceLimitExceeded { limit: 0 });
    }
    for (index, candidate) in candidates.iter().enumerate() {
        for column in 0..2 {
            if !candidate.sensitivity[column].is_finite() {
                return Err(IdentifiabilityError::NonFiniteDesign {
                    row: index,
                    column,
                });
            }
        }
        if !candidate.observation_variance.is_finite()
            || candidate.observation_variance <= 0.0
        {
            return Err(IdentifiabilityError::InvalidVariance { row: index });
        }
    }

    let mut designs = Vec::new();
    enumerate_designs(
        candidates.len(),
        0,
        observation_budget,
        &mut Vec::with_capacity(observation_budget),
        &mut designs,
        max_evaluations,
    )?;

    let mut d_best: Option<DesignScore> = None;
    let mut e_best: Option<DesignScore> = None;
    let mut identifiable_design_count = 0usize;
    for indices in &designs {
        if let Some(score) = score_candidate_design(
            indices,
            candidates,
            parameter_scales,
            relative_rank_tolerance,
        )? {
            identifiable_design_count += 1;
            if d_best.as_ref().is_none_or(|best| is_better_d(&score, best)) {
                d_best = Some(score.clone());
            }
            if e_best.as_ref().is_none_or(|best| is_better_e(&score, best)) {
                e_best = Some(score);
            }
        }
    }

    let d_optimal = d_best.ok_or(IdentifiabilityError::NoIdentifiableDesign)?;
    let e_optimal = e_best.ok_or(IdentifiabilityError::NoIdentifiableDesign)?;
    Ok(DesignTournament {
        observation_budget,
        evaluated_design_count: designs.len(),
        identifiable_design_count,
        d_optimal,
        e_optimal,
    })
}

/// A manufactured design where D-optimality and E-optimality select different
/// allocations at the same three-observation budget.
pub fn tournament_reference_case() -> [CandidateCondition; 4] {
    [
        CandidateCondition { sensitivity: [-3.0, -3.0], observation_variance: 1.0 },
        CandidateCondition { sensitivity: [-3.0, -2.0], observation_variance: 1.0 },
        CandidateCondition { sensitivity: [-3.0, -1.0], observation_variance: 1.0 },
        CandidateCondition { sensitivity: [-2.0, 0.0], observation_variance: 1.0 },
    ]
}

pub fn tournament_reference_self_check() -> Result<(), IdentifiabilityError> {
    let result = select_discrete_designs(
        &tournament_reference_case(),
        3,
        [1.0, 1.0],
        1.0e-12,
        100,
    )?;
    if result.evaluated_design_count != 20
        || result.identifiable_design_count == 0
        || result.d_optimal.candidate_indices != [0, 2, 3]
        || result.e_optimal.candidate_indices != [0, 3, 3]
        || (result.d_optimal.log_determinant_information - 76.0_f64.ln()).abs() > 1.0e-12
        || (result.e_optimal.log_determinant_information - 72.0_f64.ln()).abs() > 1.0e-12
        || (result.d_optimal.minimum_information_eigenvalue - 2.583592135001262).abs() > 1.0e-10
        || (result.e_optimal.minimum_information_eigenvalue - 3.1511421982038947).abs() > 1.0e-10
        || result.e_optimal.minimum_information_eigenvalue
            <= result.d_optimal.minimum_information_eigenvalue
    {
        return Err(IdentifiabilityError::NonFiniteDerivedValue);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_variances() -> [f64; 3] { [1.0, 1.0, 1.0] }

    #[test]
    fn repeated_identical_conditions_do_not_repair_rank_deficiency() {
        let design = [[1.0, 1.0], [1.0, 1.0], [1.0, 1.0]];
        let observations = [2.5, 2.5, 2.5];
        let report = analyse_two_parameter_design(
            &design, &observations, &unit_variances(), [1.0, 1.0], 1.0e-12,
        ).unwrap();
        assert_eq!(report.numerical_rank, 1);
        assert_eq!(report.disposition, IdentificationDisposition::NotIdentifiable);
        assert!(report.parameter_estimate.is_none());
        assert!((report.eigenvalues[0] - 6.0).abs() < 1.0e-12);
        assert!(report.eigenvalues[1].abs() < 1.0e-12);
    }

    #[test]
    fn balanced_conditions_identify_both_manufactured_parameters() {
        let design = [[-1.0, 1.0], [0.0, 1.0], [1.0, 1.0]];
        let observations = [-1.5, 0.5, 2.5];
        let report = analyse_two_parameter_design(
            &design, &observations, &unit_variances(), [1.0, 1.0], 1.0e-12,
        ).unwrap();
        assert_eq!(report.numerical_rank, 2);
        assert_eq!(report.disposition, IdentificationDisposition::Identifiable);
        let estimate = report.parameter_estimate.unwrap();
        assert!((estimate[0] - 2.0).abs() < 1.0e-12);
        assert!((estimate[1] - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn zero_or_negative_variance_is_rejected() {
        let design = [[1.0, 1.0]];
        let observations = [2.0];
        let variances = [0.0];
        assert_eq!(
            analyse_two_parameter_design(&design, &observations, &variances, [1.0, 1.0], 1.0e-12).unwrap_err(),
            IdentifiabilityError::InvalidVariance { row: 0 }
        );
    }

    #[test]
    fn invalid_rank_tolerance_is_rejected() {
        let design = [[1.0, 1.0]];
        let observations = [2.0];
        let variances = [1.0];
        assert_eq!(
            analyse_two_parameter_design(&design, &observations, &variances, [1.0, 1.0], 0.0).unwrap_err(),
            IdentifiabilityError::InvalidRankTolerance
        );
    }

    #[test]
    fn non_finite_input_is_rejected() {
        let design = [[f64::NAN, 1.0]];
        let observations = [2.0];
        let variances = [1.0];
        assert!(matches!(
            analyse_two_parameter_design(&design, &observations, &variances, [1.0, 1.0], 1.0e-12),
            Err(IdentifiabilityError::NonFiniteDesign { row: 0, column: 0 })
        ));
    }

    #[test]
    fn parameter_scaling_changes_information_in_declared_coordinates() {
        let design = [[1.0, 1.0], [-1.0, 1.0]];
        let observations = [2.0, 0.0];
        let variances = [1.0, 1.0];
        let unit = analyse_two_parameter_design(&design, &observations, &variances, [1.0, 1.0], 1.0e-12).unwrap();
        let scaled = analyse_two_parameter_design(&design, &observations, &variances, [10.0, 1.0], 1.0e-12).unwrap();
        assert_ne!(unit.information_matrix, scaled.information_matrix);
        assert_eq!(unit.numerical_rank, scaled.numerical_rank);
    }

    #[test]
    fn independent_variance_weights_are_applied() {
        let design = [[1.0, 1.0], [-1.0, 1.0]];
        let observations = [2.0, 0.0];
        let variances = [4.0, 1.0];
        let report = analyse_two_parameter_design(&design, &observations, &variances, [1.0, 1.0], 1.0e-12).unwrap();
        assert_eq!(report.information_matrix, [[1.25, -0.75], [-0.75, 1.25]]);
    }

    #[test]
    fn jacobi_spectrum_retains_a_small_but_resolvable_singular_direction() {
        let design = [[1.0, 0.0], [1.0, 1.0e-9]];
        let observations = [1.0, 1.0];
        let variances = [1.0, 1.0];
        let report = analyse_two_parameter_design(
            &design, &observations, &variances, [1.0, 1.0], 1.0e-12,
        ).unwrap();
        assert_eq!(report.numerical_rank, 2);
        assert!(report.singular_values[1] > 1.0e-12 * report.singular_values[0]);
        assert!(report.condition_number.unwrap() > 1.0e8);
    }
}

    #[test]
    fn d_and_e_optimal_designs_can_differ_at_the_same_budget() {
        let result = select_discrete_designs(
            &tournament_reference_case(), 3, [1.0, 1.0], 1.0e-12, 100,
        ).unwrap();
        assert_eq!(result.evaluated_design_count, 20);
        assert_eq!(result.d_optimal.candidate_indices, vec![0, 2, 3]);
        assert_eq!(result.e_optimal.candidate_indices, vec![0, 3, 3]);
        assert!(result.d_optimal.log_determinant_information
            > result.e_optimal.log_determinant_information);
        assert!(result.e_optimal.minimum_information_eigenvalue
            > result.d_optimal.minimum_information_eigenvalue);
    }

    #[test]
    fn design_tournament_rejects_search_space_over_limit() {
        assert_eq!(
            select_discrete_designs(
                &tournament_reference_case(), 3, [1.0, 1.0], 1.0e-12, 19,
            ).unwrap_err(),
            IdentifiabilityError::SearchSpaceLimitExceeded { limit: 19 }
        );
    }

    #[test]
    fn design_tournament_rejects_all_rank_deficient_allocations() {
        let candidates = [
            CandidateCondition { sensitivity: [1.0, 1.0], observation_variance: 1.0 },
            CandidateCondition { sensitivity: [2.0, 2.0], observation_variance: 1.0 },
        ];
        assert_eq!(
            select_discrete_designs(&candidates, 3, [1.0, 1.0], 1.0e-12, 20).unwrap_err(),
            IdentifiabilityError::NoIdentifiableDesign
        );
    }

    #[test]
    fn design_tournament_rejects_observation_budget_above_recursion_bound() {
        assert_eq!(
            select_discrete_designs(
                &tournament_reference_case(),
                MAX_DISCRETE_OBSERVATION_BUDGET + 1,
                [1.0, 1.0],
                1.0e-12,
                usize::MAX,
            ).unwrap_err(),
            IdentifiabilityError::InvalidObservationBudget
        );
    }

    #[test]
    fn design_tournament_rejects_nonpositive_candidate_variance() {
        let candidates = [
            CandidateCondition { sensitivity: [1.0, 0.0], observation_variance: 1.0 },
            CandidateCondition { sensitivity: [0.0, 1.0], observation_variance: 0.0 },
        ];
        assert_eq!(
            select_discrete_designs(&candidates, 2, [1.0, 1.0], 1.0e-12, 10).unwrap_err(),
            IdentifiabilityError::InvalidVariance { row: 1 }
        );
    }

