//! Exact finite-hypothesis Bayesian updates for locomotion research.
//!
//! This crate is deliberately small: it is an arithmetic reference, not a
//! general POMDP solver, calibration framework, sensor fusion engine, or
//! physical safety certificate.
//!
//! Probabilities are represented by non-negative integer weights. A belief
//! distribution is the normalized ratio of each weight to the checked sum of
//! all weights. Updates use exact integer multiplication and fail explicitly
//! on overflow. Likelihoods supplied for one observation must already encode
//! the joint observation model; correlated sensor channels must not be
//! multiplied as if conditionally independent without evidence.
//!
//! Robust probe filtering is a separate operation. Candidate probes are
//! intersected with an externally supplied set of robustly admissible action
//! identifiers. This crate does not generate or validate those certificates.

use std::collections::{BTreeMap, BTreeSet};

/// A finite discrete belief represented by canonical, unnormalized weights.
///
/// Zero-weight hypotheses are retained as explicit zero-probability entries.
/// The common divisor of positive weights is removed on construction/update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscreteBelief {
    weights: BTreeMap<String, u128>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeliefError {
    EmptyHypothesisSet,
    EmptyHypothesisId,
    UntrimmedHypothesisId(String),
    DuplicateHypothesisId(String),
    ZeroPriorMass,
    EmptyLikelihoodSet,
    EmptyLikelihoodHypothesisId,
    UntrimmedLikelihoodHypothesisId(String),
    DuplicateLikelihoodHypothesisId(String),
    UnknownLikelihoodHypothesis(String),
    MissingLikelihoodHypothesis(String),
    ZeroEvidenceMass,
    WeightOverflow,
    ProbabilityMassOverflow,
    EmptyProbeId,
    UntrimmedProbeId(String),
    DuplicateProbeId(String),
    InvalidAdmissibleProbeId(String),
}

impl DiscreteBelief {
    /// Construct a belief from finite non-negative integer weights.
    ///
    /// The input order does not affect the canonical result. The sum of
    /// reduced weights must fit in u128, so exact probabilities can be
    /// returned without silently overflowing their denominator.
    pub fn from_weights(entries: Vec<(String, u128)>) -> Result<Self, BeliefError> {
        if entries.is_empty() {
            return Err(BeliefError::EmptyHypothesisSet);
        }

        let mut weights = BTreeMap::new();
        for (hypothesis, weight) in entries {
            if hypothesis.trim().is_empty() {
                return Err(BeliefError::EmptyHypothesisId);
            }
            if hypothesis.trim() != hypothesis {
                return Err(BeliefError::UntrimmedHypothesisId(hypothesis));
            }
            if weights.insert(hypothesis.clone(), weight).is_some() {
                return Err(BeliefError::DuplicateHypothesisId(hypothesis));
            }
        }

        normalize_weights(&mut weights)?;
        if weights.values().all(|weight| *weight == 0) {
            return Err(BeliefError::ZeroPriorMass);
        }
        checked_total(&weights)?;
        Ok(Self { weights })
    }

    /// Return the canonical hypothesis-to-weight map.
    pub fn weights(&self) -> &BTreeMap<String, u128> {
        &self.weights
    }

    /// Return the checked sum of canonical weights.
    pub fn total_weight(&self) -> Result<u128, BeliefError> {
        checked_total(&self.weights)
    }

    /// Return an exact reduced probability (numerator, denominator).
    ///
    /// None means that the hypothesis identifier is not in this belief.
    /// A zero-probability hypothesis is represented as Some((0, 1)).
    pub fn probability(&self, hypothesis: &str) -> Result<Option<(u128, u128)>, BeliefError> {
        let Some(weight) = self.weights.get(hypothesis).copied() else {
            return Ok(None);
        };

        if weight == 0 {
            return Ok(Some((0, 1)));
        }

        let total = self.total_weight()?;
        let divisor = greatest_common_divisor(weight, total);
        Ok(Some((weight / divisor, total / divisor)))
    }

    /// Apply one observation using an explicit likelihood weight per
    /// hypothesis. The likelihood weights may be proportional values; their
    /// common factor is removed before multiplication.
    ///
    /// Missing hypotheses, unknown hypotheses, duplicate identifiers,
    /// impossible evidence and arithmetic overflow all fail closed.
    pub fn update(&self, likelihoods: Vec<(String, u128)>) -> Result<Self, BeliefError> {
        if likelihoods.is_empty() {
            return Err(BeliefError::EmptyLikelihoodSet);
        }

        let mut likelihood_map = BTreeMap::new();
        for (hypothesis, likelihood) in likelihoods {
            if hypothesis.trim().is_empty() {
                return Err(BeliefError::EmptyLikelihoodHypothesisId);
            }
            if hypothesis.trim() != hypothesis {
                return Err(BeliefError::UntrimmedLikelihoodHypothesisId(hypothesis));
            }
            if !self.weights.contains_key(&hypothesis) {
                return Err(BeliefError::UnknownLikelihoodHypothesis(hypothesis));
            }
            if likelihood_map
                .insert(hypothesis.clone(), likelihood)
                .is_some()
            {
                return Err(BeliefError::DuplicateLikelihoodHypothesisId(hypothesis));
            }
        }

        for hypothesis in self.weights.keys() {
            if !likelihood_map.contains_key(hypothesis) {
                return Err(BeliefError::MissingLikelihoodHypothesis(hypothesis.clone()));
            }
        }

        if likelihood_map.values().all(|likelihood| *likelihood == 0) {
            return Err(BeliefError::ZeroEvidenceMass);
        }
        normalize_weights(&mut likelihood_map)?;

        let mut posterior = BTreeMap::new();
        for (hypothesis, prior_weight) in &self.weights {
            let likelihood = likelihood_map[hypothesis];
            let product = prior_weight
                .checked_mul(likelihood)
                .ok_or(BeliefError::WeightOverflow)?;
            posterior.insert(hypothesis.clone(), product);
        }

        if posterior.values().all(|weight| *weight == 0) {
            return Err(BeliefError::ZeroEvidenceMass);
        }
        normalize_weights(&mut posterior)?;
        checked_total(&posterior)?;
        Ok(Self { weights: posterior })
    }
}

/// Return the deterministic intersection of candidate probes and a separately
/// computed set of robustly admissible action identifiers.
///
/// This function cannot create a robust certificate. Callers must obtain
/// robustly_admissible from the independent viability assurance path. A
/// candidate not present in that set is always filtered out.
pub fn robustly_admissible_probe_ids(
    candidates: &[String],
    robustly_admissible: &BTreeSet<String>,
) -> Result<BTreeSet<String>, BeliefError> {
    let mut candidate_ids = BTreeSet::new();
    for candidate in candidates {
        if candidate.trim().is_empty() {
            return Err(BeliefError::EmptyProbeId);
        }
        if candidate.trim() != candidate {
            return Err(BeliefError::UntrimmedProbeId(candidate.clone()));
        }
        if !candidate_ids.insert(candidate.clone()) {
            return Err(BeliefError::DuplicateProbeId(candidate.clone()));
        }
    }

    for admitted in robustly_admissible {
        if admitted.trim().is_empty() || admitted.trim() != admitted {
            return Err(BeliefError::InvalidAdmissibleProbeId(admitted.clone()));
        }
    }

    Ok(candidate_ids
        .intersection(robustly_admissible)
        .cloned()
        .collect())
}

fn normalize_weights(weights: &mut BTreeMap<String, u128>) -> Result<(), BeliefError> {
    let divisor = weights
        .values()
        .copied()
        .filter(|weight| *weight > 0)
        .reduce(greatest_common_divisor)
        .ok_or(BeliefError::ZeroPriorMass)?;

    for weight in weights.values_mut() {
        *weight /= divisor;
    }
    Ok(())
}

fn checked_total(weights: &BTreeMap<String, u128>) -> Result<u128, BeliefError> {
    weights.values().try_fold(0_u128, |total, weight| {
        total
            .checked_add(*weight)
            .ok_or(BeliefError::ProbabilityMassOverflow)
    })
}

fn greatest_common_divisor(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair_belief() -> DiscreteBelief {
        DiscreteBelief::from_weights(vec![("firm".to_owned(), 2), ("loose".to_owned(), 1)]).unwrap()
    }

    #[test]
    fn exact_bayes_update_matches_hand_calculated_posterior() {
        let prior = pair_belief();
        let posterior = prior
            .update(vec![("firm".to_owned(), 3), ("loose".to_owned(), 1)])
            .unwrap();

        assert_eq!(posterior.weights()["firm"], 6);
        assert_eq!(posterior.weights()["loose"], 1);
        assert_eq!(posterior.probability("firm").unwrap(), Some((6, 7)));
        assert_eq!(posterior.probability("loose").unwrap(), Some((1, 7)));
    }

    #[test]
    fn common_weight_scaling_does_not_change_the_belief() {
        let left =
            DiscreteBelief::from_weights(vec![("firm".to_owned(), 2), ("loose".to_owned(), 1)])
                .unwrap();
        let right =
            DiscreteBelief::from_weights(vec![("firm".to_owned(), 200), ("loose".to_owned(), 100)])
                .unwrap();

        assert_eq!(left, right);
    }

    #[test]
    fn input_permutation_preserves_canonical_posterior() {
        let prior = pair_belief();
        let left = prior
            .update(vec![("firm".to_owned(), 9), ("loose".to_owned(), 2)])
            .unwrap();
        let right = prior
            .update(vec![("loose".to_owned(), 2), ("firm".to_owned(), 9)])
            .unwrap();

        assert_eq!(left, right);
    }

    #[test]
    fn zero_likelihood_removes_hypothesis_mass_exactly() {
        let posterior = pair_belief()
            .update(vec![("firm".to_owned(), 1), ("loose".to_owned(), 0)])
            .unwrap();

        assert_eq!(posterior.probability("firm").unwrap(), Some((1, 1)));
        assert_eq!(posterior.probability("loose").unwrap(), Some((0, 1)));
    }

    #[test]
    fn impossible_observation_is_not_silently_renormalized() {
        let result = pair_belief().update(vec![("firm".to_owned(), 0), ("loose".to_owned(), 0)]);

        assert_eq!(result, Err(BeliefError::ZeroEvidenceMass));
    }

    #[test]
    fn missing_likelihood_hypothesis_is_rejected() {
        let result = pair_belief().update(vec![("firm".to_owned(), 1)]);
        assert_eq!(
            result,
            Err(BeliefError::MissingLikelihoodHypothesis("loose".to_owned()))
        );
    }

    #[test]
    fn unknown_likelihood_hypothesis_is_rejected() {
        let result = pair_belief().update(vec![
            ("firm".to_owned(), 1),
            ("loose".to_owned(), 1),
            ("unknown".to_owned(), 1),
        ]);
        assert_eq!(
            result,
            Err(BeliefError::UnknownLikelihoodHypothesis(
                "unknown".to_owned()
            ))
        );
    }

    #[test]
    fn duplicate_hypotheses_are_rejected() {
        let result =
            DiscreteBelief::from_weights(vec![("firm".to_owned(), 1), ("firm".to_owned(), 2)]);
        assert_eq!(
            result,
            Err(BeliefError::DuplicateHypothesisId("firm".to_owned()))
        );
    }

    #[test]
    fn zero_prior_mass_is_rejected() {
        let result =
            DiscreteBelief::from_weights(vec![("firm".to_owned(), 0), ("loose".to_owned(), 0)]);
        assert_eq!(result, Err(BeliefError::ZeroPriorMass));
    }

    #[test]
    fn arithmetic_overflow_returns_an_error() {
        let large = (u128::MAX / 2) - 10;
        let slightly_smaller = (u128::MAX / 2) - 20;
        let prior = DiscreteBelief::from_weights(vec![
            ("firm".to_owned(), large),
            ("loose".to_owned(), slightly_smaller),
        ])
        .unwrap();

        let result = prior.update(vec![
            ("firm".to_owned(), large),
            ("loose".to_owned(), slightly_smaller),
        ]);
        assert_eq!(result, Err(BeliefError::WeightOverflow));
    }

    #[test]
    fn robust_probe_filter_never_admits_unlisted_candidate() {
        let candidates = vec![
            "probe-left".to_owned(),
            "probe-right".to_owned(),
            "probe-risky".to_owned(),
        ];
        let robustly_admissible =
            BTreeSet::from(["probe-left".to_owned(), "probe-right".to_owned()]);

        assert_eq!(
            robustly_admissible_probe_ids(&candidates, &robustly_admissible).unwrap(),
            robustly_admissible
        );
    }

    #[test]
    fn empty_robust_admissible_set_filters_every_probe() {
        let candidates = vec!["probe".to_owned()];
        assert!(
            robustly_admissible_probe_ids(&candidates, &BTreeSet::new())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn duplicate_probe_candidate_is_rejected() {
        let candidates = vec!["probe".to_owned(), "probe".to_owned()];
        let allowed = BTreeSet::from(["probe".to_owned()]);
        assert_eq!(
            robustly_admissible_probe_ids(&candidates, &allowed),
            Err(BeliefError::DuplicateProbeId("probe".to_owned()))
        );
    }
}
