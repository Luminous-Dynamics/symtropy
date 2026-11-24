// SPDX-License-Identifier: AGPL-3.0-or-later
//! Perturbation-aware rank stability for inertia-whitened contact Jacobians.
//!
//! The input rows are S = J_n L^-T from the nominal SPD mass metric. Given
//! bounded whitened perturbations H = L^-1 ΔM L^-T and D = ΔJ_n L^-T,
//! this crate bounds the perturbed symmetric-root whitening:
//!
//!   S_tilde = (S + D) (I + H)^(-1/2)
//!
//! This is a spectral row-rank / subspace diagnostic only. It does not certify
//! that a unilateral contact inequality or friction-cone row is safe to omit.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const SCHEMA_VERSION: &str = "inertial-contact-rank-stability-v0.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StabilityErrorKind {
    EmptyCandidates,
    EmptyGeneralizedDimension,
    DimensionMismatch,
    InvalidIdentity,
    DuplicateContactId,
    NonFiniteInput,
    InvalidUncertainty,
    InvalidTolerance,
    UnderestimatedNominalNorm,
    NonFiniteEvaluation,
    JacobiDidNotConverge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StabilityError {
    pub kind: StabilityErrorKind,
    pub reason: String,
}

impl StabilityError {
    fn new(kind: StabilityErrorKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            reason: reason.into(),
        }
    }
}

impl Display for StabilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl Error for StabilityError {}

#[derive(Clone, Debug, PartialEq)]
pub struct RankStabilityInput {
    pub candidate_set_digest: String,
    pub mass_properties_digest: String,
    pub uncertainty_profile_digest: String,
    /// Stable IDs in the same order as the whitened matrix rows.
    pub contact_ids: Vec<String>,
    /// Nominal whitened candidate rows S = J_n L^-T.
    pub whitened_rows: Vec<Vec<f64>>,
    /// Bound rho >= ||L^-1 ΔM L^-T||_2, with 0 <= rho < 1.
    pub relative_mass_perturbation_bound: f64,
    /// Bound delta >= ||ΔJ_n L^-T||_2.
    pub whitened_jacobian_perturbation_bound: f64,
    /// Certified or conservative upper bound on ||S||_2.
    pub nominal_operator_norm_upper: f64,
    /// Fixed absolute singular-value rank floor.
    pub absolute_rank_floor: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StabilityConfig {
    /// Relative orthogonality threshold for one-sided Jacobi column pairs.
    pub jacobi_relative_tolerance: f64,
    /// Maximum complete pair sweeps.
    pub maximum_jacobi_sweeps: u64,
}

impl Default for StabilityConfig {
    fn default() -> Self {
        Self {
            jacobi_relative_tolerance: 1.0e-12,
            maximum_jacobi_sweeps: 64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SingularDirectionDisposition {
    RobustlyAboveRankFloor,
    RobustlyBelowRankFloor,
    UncertaintyCrossesRankFloor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SingularValueInterval {
    pub index: usize,
    pub nominal_value: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub disposition: SingularDirectionDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RankStabilityStatus {
    RobustRankDetermined,
    Inconclusive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasisIdStability {
    /// Singular-value separation alone does not prove the greedy pivot-ID sequence.
    NotCertifiedBySpectralEnvelope,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RankStabilityReport {
    pub schema_version: String,
    pub candidate_set_digest: String,
    pub mass_properties_digest: String,
    pub uncertainty_profile_digest: String,
    pub candidate_count: usize,
    pub generalized_dof: usize,
    pub singular_value_count: usize,
    pub nominal_spectral_rank: usize,
    pub robust_spectral_rank: Option<usize>,
    pub status: RankStabilityStatus,
    pub nominal_operator_norm: f64,
    pub nominal_operator_norm_upper: f64,
    pub relative_mass_perturbation_bound: f64,
    pub whitened_jacobian_perturbation_bound: f64,
    pub whitened_operator_perturbation_bound: f64,
    pub absolute_rank_floor: f64,
    pub singular_value_intervals: Vec<SingularValueInterval>,
    pub jacobi_sweeps_used: u64,
    pub final_maximum_column_correlation: f64,
    pub basis_id_stability: BasisIdStability,
    /// This stage only certifies numerical row-space properties.
    pub physical_omission_certified: bool,
}

fn finite_matrix(matrix: &[Vec<f64>]) -> bool {
    matrix
        .iter()
        .flat_map(|row| row.iter())
        .all(|value| value.is_finite())
}

fn stable_norm(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |norm, value| norm.hypot(*value))
}

fn dot(left: &[f64], right: &[f64]) -> Result<f64, StabilityError> {
    let mut total = 0.0_f64;
    for (left_value, right_value) in left.iter().zip(right) {
        total += left_value * right_value;
        if !total.is_finite() {
            return Err(StabilityError::new(
                StabilityErrorKind::NonFiniteEvaluation,
                "Jacobi dot product overflowed",
            ));
        }
    }
    Ok(total)
}

fn transpose(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = matrix.len();
    let columns = matrix.first().map_or(0, Vec::len);
    let mut result = vec![vec![0.0_f64; rows]; columns];
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            result[column_index][row_index] = *value;
        }
    }
    result
}

fn validate_input(
    input: &RankStabilityInput,
    config: StabilityConfig,
) -> Result<(), StabilityError> {
    if input.candidate_set_digest.trim().is_empty()
        || input.mass_properties_digest.trim().is_empty()
        || input.uncertainty_profile_digest.trim().is_empty()
    {
        return Err(StabilityError::new(
            StabilityErrorKind::InvalidIdentity,
            "candidate, mass-properties and uncertainty-profile digests must be non-empty",
        ));
    }

    if input.contact_ids.is_empty() || input.whitened_rows.is_empty() {
        return Err(StabilityError::new(
            StabilityErrorKind::EmptyCandidates,
            "at least one candidate contact row is required",
        ));
    }
    if input.whitened_rows.len() != input.contact_ids.len() {
        return Err(StabilityError::new(
            StabilityErrorKind::DimensionMismatch,
            "whitened row count must match contact ID count",
        ));
    }

    let dof = input.whitened_rows[0].len();
    if dof == 0 {
        return Err(StabilityError::new(
            StabilityErrorKind::EmptyGeneralizedDimension,
            "whitened rows must have a non-zero generalized dimension",
        ));
    }
    if input.whitened_rows.iter().any(|row| row.len() != dof) {
        return Err(StabilityError::new(
            StabilityErrorKind::DimensionMismatch,
            "whitened candidate rows must have identical dimensions",
        ));
    }
    if !finite_matrix(&input.whitened_rows) {
        return Err(StabilityError::new(
            StabilityErrorKind::NonFiniteInput,
            "whitened contact rows must be finite",
        ));
    }

    let mut ids = BTreeSet::new();
    for id in &input.contact_ids {
        if id.trim().is_empty() || !ids.insert(id.as_str()) {
            return Err(StabilityError::new(
                if id.trim().is_empty() {
                    StabilityErrorKind::InvalidIdentity
                } else {
                    StabilityErrorKind::DuplicateContactId
                },
                "contact IDs must be non-empty and unique",
            ));
        }
    }

    if !input.relative_mass_perturbation_bound.is_finite()
        || input.relative_mass_perturbation_bound < 0.0
        || input.relative_mass_perturbation_bound >= 1.0
        || !input.whitened_jacobian_perturbation_bound.is_finite()
        || input.whitened_jacobian_perturbation_bound < 0.0
        || !input.nominal_operator_norm_upper.is_finite()
        || input.nominal_operator_norm_upper < 0.0
        || !input.absolute_rank_floor.is_finite()
        || input.absolute_rank_floor <= 0.0
    {
        return Err(StabilityError::new(
            StabilityErrorKind::InvalidUncertainty,
            "rho must be in [0,1), delta and norm upper bound non-negative, and rank floor positive",
        ));
    }
    if !config.jacobi_relative_tolerance.is_finite()
        || config.jacobi_relative_tolerance <= 0.0
        || config.jacobi_relative_tolerance >= 1.0
        || config.maximum_jacobi_sweeps == 0
    {
        return Err(StabilityError::new(
            StabilityErrorKind::InvalidTolerance,
            "Jacobi tolerance must be in (0,1) and sweep budget must be positive",
        ));
    }
    Ok(())
}

/// One-sided Jacobi SVD singular values for a rectangular matrix.
///
/// The smaller dimension is made the column dimension. The input is scaled by
/// its largest absolute entry before rotations, reducing overflow risk in the
/// pairwise Gram quantities. This is a deterministic small-dense reference.
fn jacobi_singular_values(
    input: &[Vec<f64>],
    relative_tolerance: f64,
    maximum_sweeps: u64,
) -> Result<(Vec<f64>, u64, f64), StabilityError> {
    let rows = input.len();
    let columns = input[0].len();
    let mut work = if rows >= columns {
        input.to_vec()
    } else {
        transpose(input)
    };

    let scale = work
        .iter()
        .flat_map(|row| row.iter())
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    if scale == 0.0 {
        return Ok((vec![0.0; work[0].len()], 0, 0.0));
    }
    for row in &mut work {
        for value in row {
            *value /= scale;
        }
    }

    let column_count = work[0].len();
    let mut final_correlation = 0.0_f64;
    let mut sweeps_used = 0_u64;
    let mut converged = column_count <= 1;

    for sweep in 0..maximum_sweeps {
        let mut maximum_correlation = 0.0_f64;
        for p in 0..column_count {
            for q in (p + 1)..column_count {
                let mut column_p = Vec::with_capacity(work.len());
                let mut column_q = Vec::with_capacity(work.len());
                for row in &work {
                    column_p.push(row[p]);
                    column_q.push(row[q]);
                }
                let norm_p = stable_norm(&column_p);
                let norm_q = stable_norm(&column_q);
                if norm_p == 0.0 || norm_q == 0.0 {
                    continue;
                }

                let normalized_p = column_p
                    .iter()
                    .map(|value| value / norm_p)
                    .collect::<Vec<_>>();
                let normalized_q = column_q
                    .iter()
                    .map(|value| value / norm_q)
                    .collect::<Vec<_>>();
                let correlation = dot(&normalized_p, &normalized_q)?.abs();
                if !correlation.is_finite() {
                    return Err(StabilityError::new(
                        StabilityErrorKind::NonFiniteEvaluation,
                        "normalized Jacobi column correlation is non-finite",
                    ));
                }
                maximum_correlation = maximum_correlation.max(correlation);
                if correlation <= relative_tolerance {
                    continue;
                }

                let alpha = dot(&column_p, &column_p)?;
                let beta = dot(&column_q, &column_q)?;
                let gamma = dot(&column_p, &column_q)?;
                if gamma == 0.0 {
                    continue;
                }
                let difference = beta - alpha;
                let denominator = 2.0 * gamma;
                let zeta = difference / denominator;
                let tangent = if zeta.is_finite() {
                    if zeta == 0.0 {
                        1.0
                    } else if zeta.abs() > 1.0e150 {
                        0.5 / zeta
                    } else {
                        zeta.signum() / (zeta.abs() + zeta.hypot(1.0))
                    }
                } else {
                    0.0
                };
                let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
                let sine = cosine * tangent;
                if !cosine.is_finite() || !sine.is_finite() {
                    return Err(StabilityError::new(
                        StabilityErrorKind::NonFiniteEvaluation,
                        "Jacobi rotation is non-finite",
                    ));
                }

                for row in &mut work {
                    let old_p = row[p];
                    let old_q = row[q];
                    row[p] = cosine * old_p - sine * old_q;
                    row[q] = sine * old_p + cosine * old_q;
                }
            }
        }

        sweeps_used = sweep + 1;
        final_correlation = maximum_correlation;
        if maximum_correlation <= relative_tolerance {
            converged = true;
            break;
        }
    }

    if !converged {
        return Err(StabilityError::new(
            StabilityErrorKind::JacobiDidNotConverge,
            "one-sided Jacobi SVD did not reach the declared column-orthogonality tolerance",
        ));
    }

    let mut singular_values = Vec::with_capacity(column_count);
    for column in 0..column_count {
        let values = work.iter().map(|row| row[column]).collect::<Vec<_>>();
        let singular_value = stable_norm(&values) * scale;
        if !singular_value.is_finite() {
            return Err(StabilityError::new(
                StabilityErrorKind::NonFiniteEvaluation,
                "singular-value rescaling overflowed",
            ));
        }
        singular_values.push(singular_value);
    }
    singular_values.sort_by(|left, right| right.total_cmp(left));
    Ok((singular_values, sweeps_used, final_correlation))
}

/// Compute the perturbation bound for the symmetrically whitened candidate rows.
///
/// epsilon_S = delta / sqrt(1-rho)
///           + y_norm * ((1-rho)^(-1/2) - 1).
pub fn whitened_operator_perturbation_bound(
    relative_mass_bound: f64,
    whitened_jacobian_bound: f64,
    nominal_operator_norm_upper: f64,
) -> Result<f64, StabilityError> {
    if !relative_mass_bound.is_finite()
        || relative_mass_bound < 0.0
        || relative_mass_bound >= 1.0
        || !whitened_jacobian_bound.is_finite()
        || whitened_jacobian_bound < 0.0
        || !nominal_operator_norm_upper.is_finite()
        || nominal_operator_norm_upper < 0.0
    {
        return Err(StabilityError::new(
            StabilityErrorKind::InvalidUncertainty,
            "perturbation inputs violate rho/delta/norm-bound domains",
        ));
    }

    let inverse_sqrt_lower = 1.0 / (1.0 - relative_mass_bound).sqrt();
    let delta_term = whitened_jacobian_bound * inverse_sqrt_lower;
    let mass_term = nominal_operator_norm_upper * (inverse_sqrt_lower - 1.0);
    let bound = delta_term + mass_term;
    if !bound.is_finite() {
        return Err(StabilityError::new(
            StabilityErrorKind::NonFiniteEvaluation,
            "whitened operator perturbation bound overflowed",
        ));
    }
    Ok(bound)
}

pub fn certify_rank_stability(
    input: &RankStabilityInput,
    config: StabilityConfig,
) -> Result<RankStabilityReport, StabilityError> {
    validate_input(input, config)?;
    let (singular_values, sweeps, final_correlation) = jacobi_singular_values(
        &input.whitened_rows,
        config.jacobi_relative_tolerance,
        config.maximum_jacobi_sweeps,
    )?;

    let nominal_operator_norm = singular_values.first().copied().unwrap_or(0.0);
    if nominal_operator_norm > input.nominal_operator_norm_upper {
        return Err(StabilityError::new(
            StabilityErrorKind::UnderestimatedNominalNorm,
            "declared nominal operator norm upper bound is below the computed largest singular value",
        ));
    }

    let epsilon = whitened_operator_perturbation_bound(
        input.relative_mass_perturbation_bound,
        input.whitened_jacobian_perturbation_bound,
        input.nominal_operator_norm_upper,
    )?;

    let mut intervals = Vec::with_capacity(singular_values.len());
    let mut robust_rank = 0_usize;
    let mut nominal_rank = 0_usize;
    let mut inconclusive = false;

    for (index, singular_value) in singular_values.iter().copied().enumerate() {
        let lower = (singular_value - epsilon).max(0.0);
        let upper = singular_value + epsilon;
        if !upper.is_finite() {
            return Err(StabilityError::new(
                StabilityErrorKind::NonFiniteEvaluation,
                "singular-value upper interval overflowed",
            ));
        }
        let disposition = if lower > input.absolute_rank_floor {
            robust_rank += 1;
            SingularDirectionDisposition::RobustlyAboveRankFloor
        } else if upper < input.absolute_rank_floor {
            SingularDirectionDisposition::RobustlyBelowRankFloor
        } else {
            inconclusive = true;
            SingularDirectionDisposition::UncertaintyCrossesRankFloor
        };
        if singular_value > input.absolute_rank_floor {
            nominal_rank += 1;
        }
        intervals.push(SingularValueInterval {
            index,
            nominal_value: singular_value,
            lower_bound: lower,
            upper_bound: upper,
            disposition,
        });
    }

    Ok(RankStabilityReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        candidate_set_digest: input.candidate_set_digest.clone(),
        mass_properties_digest: input.mass_properties_digest.clone(),
        uncertainty_profile_digest: input.uncertainty_profile_digest.clone(),
        candidate_count: input.contact_ids.len(),
        generalized_dof: input.whitened_rows[0].len(),
        singular_value_count: singular_values.len(),
        nominal_spectral_rank: nominal_rank,
        robust_spectral_rank: if inconclusive {
            None
        } else {
            Some(robust_rank)
        },
        status: if inconclusive {
            RankStabilityStatus::Inconclusive
        } else {
            RankStabilityStatus::RobustRankDetermined
        },
        nominal_operator_norm,
        nominal_operator_norm_upper: input.nominal_operator_norm_upper,
        relative_mass_perturbation_bound: input.relative_mass_perturbation_bound,
        whitened_jacobian_perturbation_bound: input.whitened_jacobian_perturbation_bound,
        whitened_operator_perturbation_bound: epsilon,
        absolute_rank_floor: input.absolute_rank_floor,
        singular_value_intervals: intervals,
        jacobi_sweeps_used: sweeps,
        final_maximum_column_correlation: final_correlation,
        basis_id_stability: BasisIdStability::NotCertifiedBySpectralEnvelope,
        physical_omission_certified: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(
        ids: &[&str],
        rows: Vec<Vec<f64>>,
        rho: f64,
        delta: f64,
        y_upper: f64,
        rank_floor: f64,
    ) -> RankStabilityInput {
        RankStabilityInput {
            candidate_set_digest: "candidate-set-rank-stability-001".to_owned(),
            mass_properties_digest: "mass-properties-rank-stability-001".to_owned(),
            uncertainty_profile_digest: "uncertainty-profile-rank-stability-001".to_owned(),
            contact_ids: ids.iter().map(|id| (*id).to_owned()).collect(),
            whitened_rows: rows,
            relative_mass_perturbation_bound: rho,
            whitened_jacobian_perturbation_bound: delta,
            nominal_operator_norm_upper: y_upper,
            absolute_rank_floor: rank_floor,
        }
    }

    fn square_config() -> StabilityConfig {
        StabilityConfig {
            jacobi_relative_tolerance: 1.0e-12,
            maximum_jacobi_sweeps: 64,
        }
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "expected {left} ~= {right} within {tolerance}"
        );
    }

    #[test]
    fn exact_diagonal_spectrum_has_robust_rank_two() {
        let problem = input(
            &["a", "b"],
            vec![vec![3.0, 0.0], vec![0.0, 1.0]],
            0.0,
            0.0,
            3.0,
            0.5,
        );
        let report = certify_rank_stability(&problem, square_config()).expect("rank report");
        assert_eq!(report.singular_value_count, 2);
        assert_eq!(report.nominal_spectral_rank, 2);
        assert_eq!(report.robust_spectral_rank, Some(2));
        assert_eq!(report.status, RankStabilityStatus::RobustRankDetermined);
        close(
            report.singular_value_intervals[0].nominal_value,
            3.0,
            1.0e-12,
        );
        close(
            report.singular_value_intervals[1].nominal_value,
            1.0,
            1.0e-12,
        );
        assert!(!report.physical_omission_certified);
    }

    #[test]
    fn exact_zero_rank_is_robust_when_uncertainty_is_zero() {
        let problem = input(
            &["a", "b"],
            vec![vec![0.0, 0.0], vec![0.0, 0.0]],
            0.0,
            0.0,
            0.0,
            1.0e-8,
        );
        let report = certify_rank_stability(&problem, square_config()).expect("zero rank");
        assert_eq!(report.nominal_spectral_rank, 0);
        assert_eq!(report.robust_spectral_rank, Some(0));
        assert_eq!(report.status, RankStabilityStatus::RobustRankDetermined);
    }

    #[test]
    fn uncertainty_crossing_fixed_rank_floor_is_inconclusive() {
        let problem = input(
            &["a", "b"],
            vec![vec![2.0, 0.0], vec![0.0, 1.0]],
            0.0,
            0.2,
            2.0,
            0.9,
        );
        let report = certify_rank_stability(&problem, square_config()).expect("interval");
        assert_eq!(report.nominal_spectral_rank, 2);
        assert_eq!(report.robust_spectral_rank, None);
        assert_eq!(report.status, RankStabilityStatus::Inconclusive);
        assert_eq!(
            report.singular_value_intervals[1].disposition,
            SingularDirectionDisposition::UncertaintyCrossesRankFloor
        );
    }

    #[test]
    fn perturbation_bound_matches_closed_form() {
        let bound = whitened_operator_perturbation_bound(0.2, 0.1, 3.0).expect("finite envelope");
        let inverse_root = 1.0 / 0.8_f64.sqrt();
        let expected = 0.1 * inverse_root + 3.0 * (inverse_root - 1.0);
        close(bound, expected, 1.0e-12);
    }

    #[test]
    fn one_sided_jacobi_recovers_non_diagonal_known_spectrum() {
        let problem = input(
            &["a", "b"],
            vec![vec![1.0, 1.0], vec![0.0, 1.0]],
            0.0,
            0.0,
            1.7,
            0.1,
        );
        let report = certify_rank_stability(&problem, square_config()).expect("Jacobi SVD");
        let root_five = 5.0_f64.sqrt();
        let expected_max = ((3.0 + root_five) / 2.0).sqrt();
        let expected_min = ((3.0 - root_five) / 2.0).sqrt();
        close(
            report.singular_value_intervals[0].nominal_value,
            expected_max,
            1.0e-10,
        );
        close(
            report.singular_value_intervals[1].nominal_value,
            expected_min,
            1.0e-10,
        );
        assert_eq!(report.robust_spectral_rank, Some(2));
    }

    #[test]
    fn dense_rectangular_matrix_returns_sorted_singular_values() {
        let problem = input(
            &["a", "b"],
            vec![vec![3.0, 0.0, 0.0], vec![0.0, 4.0, 0.0]],
            0.0,
            0.0,
            4.0,
            0.5,
        );
        let report = certify_rank_stability(&problem, square_config()).expect("wide matrix");
        assert_eq!(report.singular_value_count, 2);
        close(
            report.singular_value_intervals[0].nominal_value,
            4.0,
            1.0e-12,
        );
        close(
            report.singular_value_intervals[1].nominal_value,
            3.0,
            1.0e-12,
        );
        assert_eq!(report.robust_spectral_rank, Some(2));
    }

    #[test]
    fn orthogonal_row_rebasing_preserves_singular_spectrum() {
        let base = vec![vec![3.0, 0.0], vec![0.0, 1.0]];
        let angle: f64 = 0.37;
        let cosine = angle.cos();
        let sine = angle.sin();
        let rotated = vec![
            vec![
                cosine * base[0][0] - sine * base[1][0],
                cosine * base[0][1] - sine * base[1][1],
            ],
            vec![
                sine * base[0][0] + cosine * base[1][0],
                sine * base[0][1] + cosine * base[1][1],
            ],
        ];
        let a = certify_rank_stability(
            &input(&["a", "b"], base, 0.0, 0.0, 3.0, 0.5),
            square_config(),
        )
        .expect("base");
        let b = certify_rank_stability(
            &input(&["a", "b"], rotated, 0.0, 0.0, 3.0, 0.5),
            square_config(),
        )
        .expect("rotated");
        for (left, right) in a
            .singular_value_intervals
            .iter()
            .zip(&b.singular_value_intervals)
        {
            close(left.nominal_value, right.nominal_value, 1.0e-12);
        }
    }

    #[test]
    fn understated_nominal_norm_bound_is_rejected() {
        let problem = input(
            &["a", "b"],
            vec![vec![3.0, 0.0], vec![0.0, 1.0]],
            0.0,
            0.0,
            2.9,
            0.5,
        );
        assert_eq!(
            certify_rank_stability(&problem, square_config())
                .expect_err("understated norm")
                .kind,
            StabilityErrorKind::UnderestimatedNominalNorm
        );
    }

    #[test]
    fn invalid_uncertainty_and_rank_floor_are_rejected() {
        for rho in [-0.1, 1.0, f64::NAN] {
            let problem = input(&["a"], vec![vec![1.0]], rho, 0.0, 1.0, 0.1);
            assert_eq!(
                certify_rank_stability(&problem, square_config())
                    .expect_err("invalid rho")
                    .kind,
                StabilityErrorKind::InvalidUncertainty
            );
        }
        let problem = input(&["a"], vec![vec![1.0]], 0.0, 0.0, 1.0, 0.0);
        assert_eq!(
            certify_rank_stability(&problem, square_config())
                .expect_err("zero rank floor")
                .kind,
            StabilityErrorKind::InvalidUncertainty
        );
    }

    #[test]
    fn duplicate_contact_ids_and_nonfinite_rows_are_rejected() {
        let duplicate = input(&["a", "a"], vec![vec![1.0], vec![1.0]], 0.0, 0.0, 1.0, 0.1);
        assert_eq!(
            certify_rank_stability(&duplicate, square_config())
                .expect_err("duplicate ID")
                .kind,
            StabilityErrorKind::DuplicateContactId
        );
        let nonfinite = input(&["a"], vec![vec![f64::INFINITY]], 0.0, 0.0, 1.0, 0.1);
        assert_eq!(
            certify_rank_stability(&nonfinite, square_config())
                .expect_err("nonfinite")
                .kind,
            StabilityErrorKind::NonFiniteInput
        );
    }

    #[test]
    fn invalid_jacobi_profile_is_rejected() {
        let problem = input(&["a"], vec![vec![1.0]], 0.0, 0.0, 1.0, 0.1);
        let bad = StabilityConfig {
            maximum_jacobi_sweeps: 0,
            ..square_config()
        };
        assert_eq!(
            certify_rank_stability(&problem, bad)
                .expect_err("zero sweeps")
                .kind,
            StabilityErrorKind::InvalidTolerance
        );
    }
}
