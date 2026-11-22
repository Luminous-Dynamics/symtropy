//! Minimax ranking of experiment designs under partially identified
//! shared-source dependence.
//!
//! Each candidate has a two-observation scalar linear-Gaussian model and the
//! same prior, shared-source marginal scales, and admissible correlation set.
//! The module compares nominal rho=0 ranking with minimax ranking by minimum
//! EIG over the full admissible correlation interval.
//!
//! This is an exploratory f64 analytic ranking. It is not a directed-rounded
//! interval certificate, does not authenticate the correlation-bound receipt,
//! and does not establish that the real-world source correlation lies inside
//! the declared interval. It exposes rank reversals and the cost of assuming
//! independence.

use crate::dependence_envelope::{
    bound_two_source_dependence, TwoSourceDependenceEnvelope, TwoSourceDependenceProblem,
};
use crate::ModelError;

/// One candidate experimental design evaluated over a shared dependence set.
#[derive(Debug, Clone, PartialEq)]
pub struct DependenceRobustDesignCandidate {
    pub candidate_id: String,
    pub problem: TwoSourceDependenceProblem,
}

/// Numerical status of a dependence-envelope ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependenceRobustNumericalStatus {
    /// Analytic stationary-point search in f64; not formal interval arithmetic.
    AnalyticF64Uncertified,
}

/// One candidate's nominal and minimax utility values.
#[derive(Debug, Clone, PartialEq)]
pub struct DependenceRobustDesignScore {
    pub candidate_id: String,
    /// None when zero lies outside the declared admissible correlation interval.
    pub nominal_independence_eig_nats: Option<f64>,
    pub worst_case_eig_nats: f64,
    pub best_case_eig_nats: f64,
    pub posterior_parameter_variance_bounds: [f64; 2],
    pub worst_case_correlation: f64,
    pub best_case_correlation: f64,
    /// 1-based rank; larger utility has the better (smaller) rank.
    pub robust_rank: usize,
    /// 1-based rho=0 rank; absent if independence is outside the set.
    pub nominal_independence_rank: Option<usize>,
    pub envelope: TwoSourceDependenceEnvelope,
}

/// Ranking across candidates under a common partially identified dependence set.
#[derive(Debug, Clone, PartialEq)]
pub struct DependenceRobustDesignRanking {
    /// Reference to evidence qualifying the declared common correlation range.
    /// This identifier is not itself authenticated by the kernel.
    pub dependence_scope_receipt_id: String,
    pub correlation_bounds: [f64; 2],
    /// Sorted by robust rank, descending worst-case EIG.
    pub scores: Vec<DependenceRobustDesignScore>,
    pub nominal_independence_preferred_candidate_id: Option<String>,
    pub robust_preferred_candidate_id: String,
    pub ranking_reversed: Option<bool>,
    pub numerical_status: DependenceRobustNumericalStatus,
    /// Always false until outward-rounded arithmetic is implemented.
    pub formal_interval_arithmetic: bool,
}

/// Rank finite candidate designs by minimax EIG over a common source-correlation
/// interval, and compare with the rho=0 baseline.
///
/// The candidate family must share prior variance, both shared-source marginal
/// standard deviations, and the same correlation interval. Candidate-specific
/// detector sensitivities, source Jacobians and independent observation noise
/// may differ.
///
/// One non-empty receipt reference is required for the complete candidate
/// family. The caller must ensure that it covers the physical source definitions
/// and the admissible dependence set. Distinct experiment IDs are insufficient.
///
/// The returned rank is a minimax ordering, not a theorem-certified dominance
/// result. This implementation uses f64 arithmetic, so formal_interval_arithmetic
/// is always false.
pub fn rank_two_source_dependence_designs(
    candidates: &[DependenceRobustDesignCandidate],
    dependence_scope_receipt_id: &str,
) -> Result<DependenceRobustDesignRanking, ModelError> {
    if candidates.is_empty() {
        return Err(ModelError::InvalidParameter(
            "dependence_robust_design_candidates_must_be_nonempty",
        ));
    }
    if dependence_scope_receipt_id.trim().is_empty() {
        return Err(ModelError::InvalidParameter(
            "dependence_robust_design_scope_receipt_must_be_nonempty",
        ));
    }

    let reference = &candidates[0].problem;
    let common_bounds = reference.correlation_bounds;
    let independence_is_admissible = common_bounds[0] <= 0.0 && common_bounds[1] >= 0.0;
    let mut seen = std::collections::BTreeSet::new();
    let mut scores = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        if candidate.candidate_id.trim().is_empty()
            || !seen.insert(candidate.candidate_id.as_str())
        {
            return Err(ModelError::InvalidParameter(
                "dependence_robust_design_candidate_ids_must_be_nonempty_and_unique",
            ));
        }
        validate_common_dependence_scope(reference, &candidate.problem)?;

        let envelope = bound_two_source_dependence(&candidate.problem)?;
        let nominal_independence_eig_nats = if independence_is_admissible {
            let mut nominal_problem = candidate.problem.clone();
            nominal_problem.correlation_bounds = [0.0, 0.0];
            Some(
                bound_two_source_dependence(&nominal_problem)?
                    .expected_information_gain_nats_bounds[0],
            )
        } else {
            None
        };

        scores.push(DependenceRobustDesignScore {
            candidate_id: candidate.candidate_id.clone(),
            nominal_independence_eig_nats,
            worst_case_eig_nats: envelope.expected_information_gain_nats_bounds[0],
            best_case_eig_nats: envelope.expected_information_gain_nats_bounds[1],
            posterior_parameter_variance_bounds: envelope.posterior_parameter_variance_bounds,
            worst_case_correlation: envelope.worst_case_information_correlation,
            best_case_correlation: envelope.best_case_information_correlation,
            robust_rank: 0,
            nominal_independence_rank: None,
            envelope,
        });
    }

    let mut robust_order: Vec<usize> = (0..scores.len()).collect();
    robust_order.sort_by(|left, right| {
        scores[*right]
            .worst_case_eig_nats
            .total_cmp(&scores[*left].worst_case_eig_nats)
            .then_with(|| scores[*left].candidate_id.cmp(&scores[*right].candidate_id))
    });
    for (rank, index) in robust_order.iter().enumerate() {
        scores[*index].robust_rank = rank + 1;
    }

    let mut nominal_independence_preferred_candidate_id = None;
    let mut ranking_reversed = None;
    if independence_is_admissible {
        let mut nominal_order: Vec<usize> = (0..scores.len()).collect();
        nominal_order.sort_by(|left, right| {
            scores[*right]
                .nominal_independence_eig_nats
                .expect("nominal EIG exists when rho=0 is admissible")
                .total_cmp(
                    &scores[*left]
                        .nominal_independence_eig_nats
                        .expect("nominal EIG exists when rho=0 is admissible"),
                )
                .then_with(|| scores[*left].candidate_id.cmp(&scores[*right].candidate_id))
        });
        for (rank, index) in nominal_order.iter().enumerate() {
            scores[*index].nominal_independence_rank = Some(rank + 1);
        }
        nominal_independence_preferred_candidate_id =
            Some(scores[nominal_order[0]].candidate_id.clone());

        let mut reversed = false;
        for left in 0..scores.len() {
            for right in (left + 1)..scores.len() {
                let robust_preference =
                    scores[left].robust_rank.cmp(&scores[right].robust_rank);
                let nominal_preference = scores[left]
                    .nominal_independence_rank
                    .cmp(&scores[right].nominal_independence_rank);
                if robust_preference != nominal_preference {
                    reversed = true;
                }
            }
        }
        ranking_reversed = Some(reversed);
    }

    scores.sort_by(|left, right| left.robust_rank.cmp(&right.robust_rank));
    let robust_preferred_candidate_id = scores[0].candidate_id.clone();

    Ok(DependenceRobustDesignRanking {
        dependence_scope_receipt_id: dependence_scope_receipt_id.to_owned(),
        correlation_bounds: common_bounds,
        scores,
        nominal_independence_preferred_candidate_id,
        robust_preferred_candidate_id,
        ranking_reversed,
        numerical_status: DependenceRobustNumericalStatus::AnalyticF64Uncertified,
        formal_interval_arithmetic: false,
    })
}

fn validate_common_dependence_scope(
    reference: &TwoSourceDependenceProblem,
    candidate: &TwoSourceDependenceProblem,
) -> Result<(), ModelError> {
    let common = [
        (reference.prior_parameter_variance, candidate.prior_parameter_variance),
        (
            reference.source_standard_deviations[0],
            candidate.source_standard_deviations[0],
        ),
        (
            reference.source_standard_deviations[1],
            candidate.source_standard_deviations[1],
        ),
        (reference.correlation_bounds[0], candidate.correlation_bounds[0]),
        (reference.correlation_bounds[1], candidate.correlation_bounds[1]),
    ];
    for (reference_value, candidate_value) in common {
        let scale = reference_value.abs().max(candidate_value.abs()).max(1.0);
        if (reference_value - candidate_value).abs() > 1.0e-12 * scale {
            return Err(ModelError::InvalidParameter(
                "dependence_robust_design_candidates_must_share_prior_sources_and_bounds",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCOPE_RECEIPT: &str = "receipt:shared-calibration-correlation-range-v1";

    fn nominal_vs_robust_candidates() -> Vec<DependenceRobustDesignCandidate> {
        vec![
            DependenceRobustDesignCandidate {
                candidate_id: "nominal-winner".to_owned(),
                problem: TwoSourceDependenceProblem {
                    prior_parameter_variance: 1.0,
                    observation_sensitivities: [-2.0, -2.0],
                    independent_observation_noise_variances: [1.0, 1.0],
                    source_standard_deviations: [1.0, 1.0],
                    source_jacobians: [[-1.0, 1.0], [0.0, 2.0]],
                    correlation_bounds: [-0.75, 0.75],
                },
            },
            DependenceRobustDesignCandidate {
                candidate_id: "robust-winner".to_owned(),
                problem: TwoSourceDependenceProblem {
                    prior_parameter_variance: 1.0,
                    observation_sensitivities: [2.0, 0.5],
                    independent_observation_noise_variances: [1.0, 1.0],
                    source_standard_deviations: [1.0, 1.0],
                    source_jacobians: [[-2.0, 0.0], [0.0, 0.0]],
                    correlation_bounds: [-0.75, 0.75],
                },
            },
        ]
    }

    #[test]
    fn minimax_source_dependence_reverses_nominal_independence_ranking() {
        let ranking =
            rank_two_source_dependence_designs(&nominal_vs_robust_candidates(), SCOPE_RECEIPT)
                .expect("robust design ranking");
        assert_eq!(
            ranking.nominal_independence_preferred_candidate_id.as_deref(),
            Some("nominal-winner")
        );
        assert_eq!(ranking.robust_preferred_candidate_id, "robust-winner");
        assert_eq!(ranking.ranking_reversed, Some(true));
        assert_eq!(ranking.scores[0].candidate_id, "robust-winner");

        let nominal = ranking
            .scores
            .iter()
            .find(|score| score.candidate_id == "nominal-winner")
            .expect("nominal candidate");
        let robust = ranking
            .scores
            .iter()
            .find(|score| score.candidate_id == "robust-winner")
            .expect("robust candidate");

        assert!(
            (nominal.nominal_independence_eig_nats.unwrap() - 0.4489707966029793).abs()
                < 1.0e-12
        );
        assert!((nominal.worst_case_eig_nats - 0.3404385439840655).abs() < 1.0e-12);
        assert!(
            (robust.nominal_independence_eig_nats.unwrap() - 0.3589198965751584).abs()
                < 1.0e-12
        );
        assert!((robust.worst_case_eig_nats - 0.3589198965751584).abs() < 1.0e-12);
        assert!(!ranking.formal_interval_arithmetic);
    }

    #[test]
    fn independence_baseline_is_absent_when_zero_is_outside_dependence_set() {
        let mut candidates = nominal_vs_robust_candidates();
        for candidate in &mut candidates {
            candidate.problem.correlation_bounds = [0.2, 0.8];
        }
        let result = rank_two_source_dependence_designs(&candidates, SCOPE_RECEIPT)
            .expect("nonzero dependence interval");
        assert_eq!(result.nominal_independence_preferred_candidate_id, None);
        assert_eq!(result.ranking_reversed, None);
        assert!(result
            .scores
            .iter()
            .all(|score| score.nominal_independence_eig_nats.is_none()));
    }

    #[test]
    fn dependence_scope_requires_one_receipt_for_the_candidate_family() {
        assert_eq!(
            rank_two_source_dependence_designs(&nominal_vs_robust_candidates(), " "),
            Err(ModelError::InvalidParameter(
                "dependence_robust_design_scope_receipt_must_be_nonempty"
            ))
        );
    }

    #[test]
    fn candidate_family_must_share_prior_and_source_boundaries() {
        let mut candidates = nominal_vs_robust_candidates();
        candidates[1].problem.source_standard_deviations[0] = 1.1;
        assert_eq!(
            rank_two_source_dependence_designs(&candidates, SCOPE_RECEIPT),
            Err(ModelError::InvalidParameter(
                "dependence_robust_design_candidates_must_share_prior_sources_and_bounds"
            ))
        );
    }

    #[test]
    fn duplicate_candidate_ids_fail_closed() {
        let mut candidates = nominal_vs_robust_candidates();
        candidates[1].candidate_id = candidates[0].candidate_id.clone();
        assert_eq!(
            rank_two_source_dependence_designs(&candidates, SCOPE_RECEIPT),
            Err(ModelError::InvalidParameter(
                "dependence_robust_design_candidate_ids_must_be_nonempty_and_unique"
            ))
        );
    }
}
