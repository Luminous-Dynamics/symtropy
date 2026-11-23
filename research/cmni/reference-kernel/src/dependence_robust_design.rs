//! Maximin experiment ranking under interval uncertainty in shared-source correlation.
//!
//! Unlike a nominal design that plugs in one cross-source correlation, this
//! module ranks candidates by the minimum linear-Gaussian EIG over the full
//! declared correlation interval. It also reports midpoint and best-case EIG
//! so the robustness trade-off is visible.
//!
//! This uses the analytic extremum calculation in dependence_envelope.rs.
//! Extrema are evaluated in f64 and are not directed-rounded or formally
//! machine-certified. A shared receipt ID must qualify the source pair and
//! correlation interval across every candidate; receipt IDs are references,
//! not self-authenticating proofs.

use crate::{bound_two_source_dependence, ModelError, TwoSourceDependenceProblem};

/// A candidate experiment evaluated against one shared pair of uncertain sources.
#[derive(Debug, Clone, PartialEq)]
pub struct DependenceRobustDesignCandidate {
    pub candidate_id: String,
    /// Ordered source IDs. The ordering matches the source-Jacobian columns.
    pub source_pair_ids: [String; 2],
    /// One receipt covering the joint source-pair correlation model and scope.
    pub joint_model_receipt_id: String,
    pub problem: TwoSourceDependenceProblem,
}

/// Numerical robust utility and nominal comparator for one candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct DependenceRobustDesignScore {
    pub candidate_id: String,
    /// EIG at the midpoint of the declared correlation interval.
    pub nominal_midpoint_eig_nats: f64,
    /// Minimum EIG over the entire declared correlation interval.
    pub worst_case_eig_nats: f64,
    /// Maximum EIG over the entire declared correlation interval.
    pub best_case_eig_nats: f64,
    pub worst_case_correlation: f64,
    pub best_case_correlation: f64,
    /// One-based rank by midpoint EIG, with deterministic ID tie-break.
    pub nominal_rank: usize,
    /// One-based rank by worst-case EIG, with deterministic ID tie-break.
    pub robust_rank: usize,
}

/// Maximin ranking with its common source/dependence scope.
#[derive(Debug, Clone, PartialEq)]
pub struct NumericalDependenceRobustRanking {
    pub source_pair_ids: [String; 2],
    pub correlation_bounds: [f64; 2],
    pub joint_model_receipt_id: String,
    /// Scores are ordered by descending worst-case EIG.
    pub scores: Vec<DependenceRobustDesignScore>,
    /// True is deliberately not claimed: computations use ordinary f64.
    pub machine_certified_enclosure: bool,
}

/// Rank experiment candidates by worst-case linear-Gaussian EIG.
///
/// Candidates must refer to the same ordered source pair, the same source
/// standard deviations, the same prior parameter variance, the same correlation
/// interval, and the same non-empty joint-model receipt. This prevents a design
/// comparison from silently changing the uncertainty set between candidates.
///
/// The nominal comparator uses the interval midpoint. The robust comparator
/// uses the analytical minimum over the full closed interval. The underlying
/// extrema are global for the two-observation/one-parameter model, up to f64
/// arithmetic, but are not formal interval-arithmetic certificates.
pub fn rank_two_source_dependence_robust_designs(
    candidates: &[DependenceRobustDesignCandidate],
) -> Result<NumericalDependenceRobustRanking, ModelError> {
    if candidates.is_empty() {
        return Err(ModelError::InvalidParameter(
            "dependence_robust_candidate_set_must_be_nonempty",
        ));
    }

    let first = &candidates[0];
    validate_scope_identifiers(first)?;
    let source_pair_ids = first.source_pair_ids.clone();
    let receipt_id = first.joint_model_receipt_id.clone();
    let correlation_bounds = first.problem.correlation_bounds;
    let source_standard_deviations = first.problem.source_standard_deviations;
    let prior_variance = first.problem.prior_parameter_variance;

    let mut seen_ids = std::collections::BTreeSet::new();
    let midpoint = correlation_bounds[0] + (correlation_bounds[1] - correlation_bounds[0]) * 0.5;
    let mut scores = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        validate_scope_identifiers(candidate)?;
        if !seen_ids.insert(candidate.candidate_id.as_str()) {
            return Err(ModelError::InvalidParameter(
                "dependence_robust_candidate_ids_must_be_unique",
            ));
        }
        if candidate.source_pair_ids != source_pair_ids {
            return Err(ModelError::InvalidParameter(
                "dependence_robust_source_pair_scope_mismatch",
            ));
        }
        if candidate.joint_model_receipt_id != receipt_id {
            return Err(ModelError::InvalidParameter(
                "dependence_robust_joint_receipt_scope_mismatch",
            ));
        }
        if candidate.problem.correlation_bounds != correlation_bounds {
            return Err(ModelError::InvalidParameter(
                "dependence_robust_correlation_interval_mismatch",
            ));
        }
        if candidate.problem.source_standard_deviations != source_standard_deviations {
            return Err(ModelError::InvalidParameter(
                "dependence_robust_source_marginal_scale_mismatch",
            ));
        }
        if candidate.problem.prior_parameter_variance != prior_variance {
            return Err(ModelError::InvalidParameter(
                "dependence_robust_prior_variance_mismatch",
            ));
        }

        let envelope = bound_two_source_dependence(&candidate.problem)?;
        let mut midpoint_problem = candidate.problem.clone();
        midpoint_problem.correlation_bounds = [midpoint, midpoint];
        let midpoint_envelope = bound_two_source_dependence(&midpoint_problem)?;

        scores.push(DependenceRobustDesignScore {
            candidate_id: candidate.candidate_id.clone(),
            nominal_midpoint_eig_nats: midpoint_envelope.expected_information_gain_nats_bounds[0],
            worst_case_eig_nats: envelope.expected_information_gain_nats_bounds[0],
            best_case_eig_nats: envelope.expected_information_gain_nats_bounds[1],
            worst_case_correlation: envelope.worst_case_information_correlation,
            best_case_correlation: envelope.best_case_information_correlation,
            nominal_rank: 0,
            robust_rank: 0,
        });
    }

    let mut nominal_order: Vec<usize> = (0..scores.len()).collect();
    nominal_order.sort_by(|left, right| {
        scores[*right]
            .nominal_midpoint_eig_nats
            .total_cmp(&scores[*left].nominal_midpoint_eig_nats)
            .then_with(|| scores[*left].candidate_id.cmp(&scores[*right].candidate_id))
    });
    for (rank, index) in nominal_order.into_iter().enumerate() {
        scores[index].nominal_rank = rank + 1;
    }

    let mut robust_order: Vec<usize> = (0..scores.len()).collect();
    robust_order.sort_by(|left, right| {
        scores[*right]
            .worst_case_eig_nats
            .total_cmp(&scores[*left].worst_case_eig_nats)
            .then_with(|| scores[*left].candidate_id.cmp(&scores[*right].candidate_id))
    });
    for (rank, index) in robust_order.iter().copied().enumerate() {
        scores[index].robust_rank = rank + 1;
    }
    scores.sort_by_key(|score| score.robust_rank);

    Ok(NumericalDependenceRobustRanking {
        source_pair_ids,
        correlation_bounds,
        joint_model_receipt_id: receipt_id,
        scores,
        machine_certified_enclosure: false,
    })
}

fn validate_scope_identifiers(
    candidate: &DependenceRobustDesignCandidate,
) -> Result<(), ModelError> {
    if candidate.candidate_id.trim().is_empty()
        || candidate
            .source_pair_ids
            .iter()
            .any(|source_id| source_id.trim().is_empty())
        || candidate.source_pair_ids[0] == candidate.source_pair_ids[1]
        || candidate.joint_model_receipt_id.trim().is_empty()
    {
        return Err(ModelError::InvalidParameter(
            "dependence_robust_scope_identifiers_must_be_valid",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECEIPT: &str = "receipt:detector-reference-correlation-v1";
    const SOURCE_PAIR: [&str; 2] = ["detector-calibration", "reference-evaluation"];

    fn candidate_a() -> DependenceRobustDesignCandidate {
        DependenceRobustDesignCandidate {
            candidate_id: "nominal-favourite".to_owned(),
            source_pair_ids: [SOURCE_PAIR[0].to_owned(), SOURCE_PAIR[1].to_owned()],
            joint_model_receipt_id: RECEIPT.to_owned(),
            problem: TwoSourceDependenceProblem {
                prior_parameter_variance: 1.0,
                observation_sensitivities: [3.0, 3.0],
                independent_observation_noise_variances: [0.25, 0.25],
                source_standard_deviations: [1.0, 1.0],
                source_jacobians: [[-1.0, 0.0], [0.0, -1.0]],
                correlation_bounds: [-0.75, 0.75],
            },
        }
    }

    fn candidate_b() -> DependenceRobustDesignCandidate {
        DependenceRobustDesignCandidate {
            candidate_id: "dependence-stable".to_owned(),
            source_pair_ids: [SOURCE_PAIR[0].to_owned(), SOURCE_PAIR[1].to_owned()],
            joint_model_receipt_id: RECEIPT.to_owned(),
            problem: TwoSourceDependenceProblem {
                prior_parameter_variance: 1.0,
                observation_sensitivities: [1.0, 3.0],
                independent_observation_noise_variances: [0.25, 0.25],
                source_standard_deviations: [1.0, 1.0],
                source_jacobians: [[-1.0, 0.0], [-1.0, 0.0]],
                correlation_bounds: [-0.75, 0.75],
            },
        }
    }

    #[test]
    fn maximin_ranking_reverses_a_midpoint_only_design_choice() {
        let ranking = rank_two_source_dependence_robust_designs(&[candidate_a(), candidate_b()])
            .expect("robust design ranking");

        assert_eq!(ranking.scores[0].candidate_id, "dependence-stable");
        let nominal_favourite = ranking
            .scores
            .iter()
            .find(|score| score.candidate_id == "nominal-favourite")
            .expect("nominal candidate");
        let dependence_stable = &ranking.scores[0];

        assert_eq!(nominal_favourite.nominal_rank, 1);
        assert_eq!(dependence_stable.nominal_rank, 2);
        assert_eq!(dependence_stable.robust_rank, 1);
        assert_eq!(nominal_favourite.robust_rank, 2);
        assert!((nominal_favourite.nominal_midpoint_eig_nats - 1.3671837547097918).abs() < 1.0e-12);
        assert!((nominal_favourite.worst_case_eig_nats - 0.5 * 10.0_f64.ln()).abs() < 1.0e-12);
        assert!(
            (dependence_stable.worst_case_eig_nats - 0.5 * (113.0_f64 / 9.0).ln()).abs() < 1.0e-12
        );
        assert!(dependence_stable.worst_case_eig_nats > nominal_favourite.worst_case_eig_nats);
        assert!(!ranking.machine_certified_enclosure);
    }

    #[test]
    fn source_receipt_interval_and_source_order_must_match() {
        let mut mismatched_receipt = candidate_b();
        mismatched_receipt.joint_model_receipt_id = "receipt:other-scope".to_owned();
        assert_eq!(
            rank_two_source_dependence_robust_designs(&[candidate_a(), mismatched_receipt]),
            Err(ModelError::InvalidParameter(
                "dependence_robust_joint_receipt_scope_mismatch"
            ))
        );

        let mut mismatched_interval = candidate_b();
        mismatched_interval.problem.correlation_bounds = [-0.5, 0.75];
        assert_eq!(
            rank_two_source_dependence_robust_designs(&[candidate_a(), mismatched_interval]),
            Err(ModelError::InvalidParameter(
                "dependence_robust_correlation_interval_mismatch"
            ))
        );

        let mut reversed_sources = candidate_b();
        reversed_sources.source_pair_ids.reverse();
        assert_eq!(
            rank_two_source_dependence_robust_designs(&[candidate_a(), reversed_sources]),
            Err(ModelError::InvalidParameter(
                "dependence_robust_source_pair_scope_mismatch"
            ))
        );
    }

    #[test]
    fn duplicate_or_invalid_candidate_scope_is_rejected() {
        assert_eq!(
            rank_two_source_dependence_robust_designs(&[]),
            Err(ModelError::InvalidParameter(
                "dependence_robust_candidate_set_must_be_nonempty"
            ))
        );
        assert_eq!(
            rank_two_source_dependence_robust_designs(&[candidate_a(), candidate_a()]),
            Err(ModelError::InvalidParameter(
                "dependence_robust_candidate_ids_must_be_unique"
            ))
        );
        let mut invalid = candidate_a();
        invalid.source_pair_ids[1] = invalid.source_pair_ids[0].clone();
        assert_eq!(
            rank_two_source_dependence_robust_designs(&[invalid]),
            Err(ModelError::InvalidParameter(
                "dependence_robust_scope_identifiers_must_be_valid"
            ))
        );
    }
}
