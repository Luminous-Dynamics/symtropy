// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Small known-truth model-set diagnostics for synthetic cohort-structure counterexamples.
//!
//! These routines test whether aggregate historical fit can conceal structural
//! disagreement or a common model omission. They are not calibrated ecological models,
//! a general model-selection algorithm, or a substitute for blind evaluation.

use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum ModelSetError {
    EmptyCandidateSet,
    EmptyModelIdentity,
    DuplicateModelIdentity,
    InvalidHistoricalObservation,
    InvalidHistoricalPrediction,
    InvalidCohortState,
    InvalidThreshold,
}

impl fmt::Display for ModelSetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ModelSetError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CohortState {
    pub small_live_biomass: f64,
    pub large_live_biomass: f64,
}

impl CohortState {
    pub fn validate(self) -> Result<(), ModelSetError> {
        if !self.small_live_biomass.is_finite()
            || !self.large_live_biomass.is_finite()
            || self.small_live_biomass < 0.0
            || self.large_live_biomass < 0.0
        {
            return Err(ModelSetError::InvalidCohortState);
        }
        Ok(())
    }

    pub fn total_biomass(self) -> f64 {
        self.small_live_biomass + self.large_live_biomass
    }

    /// L1 difference over the two declared cohort classes.
    pub fn cohort_distance(self, other: Self) -> f64 {
        (self.small_live_biomass - other.small_live_biomass).abs()
            + (self.large_live_biomass - other.large_live_biomass).abs()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateForecast {
    pub model_id: String,
    pub historical_total_biomass: f64,
    pub predicted_validation_state: CohortState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdequacyDisposition {
    StructuralDisagreement,
    CommonStructuralOmission,
    HistoryFitFailure,
    NoFailureDetectedOnThisSyntheticCase,
}

impl AdequacyDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StructuralDisagreement => "STRUCTURAL_DISAGREEMENT_REQUIRES_DISCRIMINATING_EVIDENCE",
            Self::CommonStructuralOmission => "COMMON_STRUCTURAL_OMISSION_DETECTED_ON_VALIDATION",
            Self::HistoryFitFailure => "CANDIDATE_HISTORY_FIT_INVALID",
            Self::NoFailureDetectedOnThisSyntheticCase => {
                "NO_FAILURE_DETECTED_ON_THIS_SYNTHETIC_CASE"
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KnownTruthScenario {
    pub observed_historical_total_biomass: f64,
    pub observed_validation_state: CohortState,
    pub historical_fit_tolerance: f64,
    pub structural_disagreement_threshold: f64,
    pub common_omission_residual_threshold: f64,
    pub candidates: Vec<CandidateForecast>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownTruthResult {
    pub disposition: AdequacyDisposition,
    pub candidate_count: usize,
    pub all_candidates_fit_history: bool,
    pub maximum_pairwise_cohort_span: f64,
    pub minimum_validation_residual: f64,
    pub maximum_validation_residual: f64,
}

/// Evaluate a public known-truth counterexample.
///
/// Precedence: invalid historical fit; then candidate structural disagreement; then a
/// common omission when all candidates are close to one another but far from validation
/// truth; otherwise no failure is detected in this case. A no-failure result is not a
/// model-set adequacy certificate.
pub fn evaluate_known_truth_scenario(
    scenario: &KnownTruthScenario,
) -> Result<KnownTruthResult, ModelSetError> {
    if scenario.candidates.is_empty() {
        return Err(ModelSetError::EmptyCandidateSet);
    }
    if !scenario.observed_historical_total_biomass.is_finite()
        || scenario.observed_historical_total_biomass < 0.0
    {
        return Err(ModelSetError::InvalidHistoricalObservation);
    }
    scenario.observed_validation_state.validate()?;
    if !scenario.historical_fit_tolerance.is_finite()
        || scenario.historical_fit_tolerance < 0.0
        || !scenario.structural_disagreement_threshold.is_finite()
        || scenario.structural_disagreement_threshold < 0.0
        || !scenario.common_omission_residual_threshold.is_finite()
        || scenario.common_omission_residual_threshold < 0.0
    {
        return Err(ModelSetError::InvalidThreshold);
    }

    let mut model_ids = BTreeSet::new();
    let mut all_candidates_fit_history = true;
    let mut minimum_validation_residual = f64::INFINITY;
    let mut maximum_validation_residual: f64 = 0.0;
    let mut maximum_pairwise_cohort_span: f64 = 0.0;

    for candidate in &scenario.candidates {
        if candidate.model_id.trim().is_empty() {
            return Err(ModelSetError::EmptyModelIdentity);
        }
        if !model_ids.insert(candidate.model_id.as_str()) {
            return Err(ModelSetError::DuplicateModelIdentity);
        }
        if !candidate.historical_total_biomass.is_finite()
            || candidate.historical_total_biomass < 0.0
        {
            return Err(ModelSetError::InvalidHistoricalPrediction);
        }
        candidate.predicted_validation_state.validate()?;

        let history_error = (candidate.historical_total_biomass
            - scenario.observed_historical_total_biomass)
            .abs();
        all_candidates_fit_history &= history_error <= scenario.historical_fit_tolerance;

        let validation_residual = candidate
            .predicted_validation_state
            .cohort_distance(scenario.observed_validation_state);
        minimum_validation_residual =
            minimum_validation_residual.min(validation_residual);
        maximum_validation_residual =
            maximum_validation_residual.max(validation_residual);
    }

    for left_index in 0..scenario.candidates.len() {
        for right in scenario.candidates.iter().skip(left_index + 1) {
            let left = &scenario.candidates[left_index];
            maximum_pairwise_cohort_span = maximum_pairwise_cohort_span.max(
                left.predicted_validation_state
                    .cohort_distance(right.predicted_validation_state),
            );
        }
    }

    let disposition = if !all_candidates_fit_history {
        AdequacyDisposition::HistoryFitFailure
    } else if maximum_pairwise_cohort_span > scenario.structural_disagreement_threshold {
        AdequacyDisposition::StructuralDisagreement
    } else if minimum_validation_residual > scenario.common_omission_residual_threshold {
        AdequacyDisposition::CommonStructuralOmission
    } else {
        AdequacyDisposition::NoFailureDetectedOnThisSyntheticCase
    };

    Ok(KnownTruthResult {
        disposition,
        candidate_count: scenario.candidates.len(),
        all_candidates_fit_history,
        maximum_pairwise_cohort_span,
        minimum_validation_residual,
        maximum_validation_residual,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(small: f64, large: f64) -> CohortState {
        CohortState {
            small_live_biomass: small,
            large_live_biomass: large,
        }
    }

    fn candidate(id: &str, historical: f64, future: CohortState) -> CandidateForecast {
        CandidateForecast {
            model_id: id.to_string(),
            historical_total_biomass: historical,
            predicted_validation_state: future,
        }
    }

    fn scenario(candidates: Vec<CandidateForecast>, observed: CohortState) -> KnownTruthScenario {
        KnownTruthScenario {
            observed_historical_total_biomass: 100.0,
            observed_validation_state: observed,
            historical_fit_tolerance: 0.01,
            structural_disagreement_threshold: 5.0,
            common_omission_residual_threshold: 5.0,
            candidates,
        }
    }

    #[test]
    fn equal_aggregate_forecasts_can_hide_size_structure_disagreement() {
        let result = evaluate_known_truth_scenario(&scenario(
            vec![
                candidate("growth-small", 100.0, state(40.0, 60.0)),
                candidate("mortality-small", 100.0, state(60.0, 40.0)),
            ],
            state(50.0, 50.0),
        ))
        .expect("known-truth cohort case");
        assert_eq!(result.disposition, AdequacyDisposition::StructuralDisagreement);
        assert_eq!(result.maximum_pairwise_cohort_span, 40.0);
        assert_eq!(result.minimum_validation_residual, 20.0);
    }

    #[test]
    fn recruitment_growth_aliasing_is_visible_in_cohort_space() {
        let result = evaluate_known_truth_scenario(&scenario(
            vec![
                candidate("recruitment-pulse", 100.0, state(20.0, 90.0)),
                candidate("large-cohort-growth", 100.0, state(10.0, 100.0)),
            ],
            state(15.0, 95.0),
        ))
        .expect("recruitment-growth counterexample");
        assert_eq!(result.disposition, AdequacyDisposition::StructuralDisagreement);
        assert_eq!(result.maximum_pairwise_cohort_span, 20.0);
    }

    #[test]
    fn common_delayed_response_omission_is_detected_by_validation() {
        let result = evaluate_known_truth_scenario(&scenario(
            vec![
                candidate("instantaneous-model-a", 100.0, state(80.0, 20.0)),
                candidate("instantaneous-model-b", 100.0, state(80.0, 20.0)),
            ],
            state(100.0, 0.0),
        ))
        .expect("delayed-response counterexample");
        assert_eq!(result.disposition, AdequacyDisposition::CommonStructuralOmission);
        assert_eq!(result.maximum_pairwise_cohort_span, 0.0);
        assert_eq!(result.minimum_validation_residual, 40.0);
    }

    #[test]
    fn matching_control_does_not_report_adequacy_as_proven() {
        let result = evaluate_known_truth_scenario(&scenario(
            vec![
                candidate("matched-a", 100.0, state(45.0, 55.0)),
                candidate("matched-b", 100.0, state(45.0, 55.0)),
            ],
            state(45.0, 55.0),
        ))
        .expect("matched control");
        assert_eq!(
            result.disposition,
            AdequacyDisposition::NoFailureDetectedOnThisSyntheticCase
        );
        assert!(result.all_candidates_fit_history);
    }

    #[test]
    fn candidate_history_fit_failure_is_not_mislabelled_as_omission() {
        let result = evaluate_known_truth_scenario(&scenario(
            vec![
                candidate("bad-history", 90.0, state(45.0, 55.0)),
                candidate("good-history", 100.0, state(45.0, 55.0)),
            ],
            state(45.0, 55.0),
        ))
        .expect("history fit failure control");
        assert_eq!(result.disposition, AdequacyDisposition::HistoryFitFailure);
        assert!(!result.all_candidates_fit_history);
    }

    #[test]
    fn duplicate_candidate_identity_is_rejected() {
        let result = evaluate_known_truth_scenario(&scenario(
            vec![
                candidate("same-id", 100.0, state(45.0, 55.0)),
                candidate("same-id", 100.0, state(45.0, 55.0)),
            ],
            state(45.0, 55.0),
        ));
        assert_eq!(result, Err(ModelSetError::DuplicateModelIdentity));
    }

    #[test]
    fn negative_or_non_finite_cohort_biomass_is_rejected() {
        let result = evaluate_known_truth_scenario(&scenario(
            vec![candidate("invalid", 100.0, state(-1.0, 101.0))],
            state(45.0, 55.0),
        ));
        assert_eq!(result, Err(ModelSetError::InvalidCohortState));
    }

    #[test]
    fn empty_model_set_is_rejected() {
        let result = evaluate_known_truth_scenario(&scenario(Vec::new(), state(45.0, 55.0)));
        assert_eq!(result, Err(ModelSetError::EmptyCandidateSet));
    }
}
