// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finite-population reference for policy-dependent outcome observability.
//!
//! Candidate estimators receive only CandidateRecord values. EvaluatorRecord
//! stores latent outcomes and the true observation mechanism separately.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutcomeSupport {
    pub lower: u64,
    pub upper: u64,
}

impl OutcomeSupport {
    pub fn validate(self) -> Result<(), EvaluationError> {
        if self.lower > self.upper {
            return Err(EvaluationError::InvalidOutcomeSupport);
        }
        Ok(())
    }

    fn contains(self, value: u64) -> bool {
        (self.lower..=self.upper).contains(&value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MissingReason {
    TelemetryUnavailable,
    Nonresponse,
    PolicySuppressed,
    AdministrativeExclusion,
}

impl MissingReason {
    pub const fn id(self) -> &'static str {
        match self {
            Self::TelemetryUnavailable => "telemetry_unavailable",
            Self::Nonresponse => "nonresponse",
            Self::PolicySuppressed => "policy_suppressed",
            Self::AdministrativeExclusion => "administrative_exclusion",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MissingnessMechanism {
    Complete,
    Mcar,
    Mar,
    MnarAdverseOutcomesHidden,
    MnarBeneficialOutcomesHidden,
    Mixed,
}

impl MissingnessMechanism {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Mcar => "mcar",
            Self::Mar => "mar",
            Self::MnarAdverseOutcomesHidden => "mnar_adverse_hidden",
            Self::MnarBeneficialOutcomesHidden => "mnar_beneficial_hidden",
            Self::Mixed => "mixed",
        }
    }
}

/// An observation-only record. It deliberately contains no latent outcome or
/// true missingness-mechanism label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRecord {
    pub policy_id: String,
    pub unit_id: String,
    pub stratum_id: String,
    pub population_weight: u64,
    pub observed_outcome: Option<u64>,
    pub missing_reason: Option<MissingReason>,
    /// Known observation probability in basis points, when legitimately
    /// supplied by a declared MAR design. None is not interpreted as zero.
    pub observation_propensity_bps: Option<u32>,
}

/// Evaluator-only wrapper. Candidate estimators must be called with the
/// observation field alone, never with this value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatorRecord {
    pub observation: CandidateRecord,
    pub latent_outcome: u64,
    pub true_mechanism: MissingnessMechanism,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationAssumption {
    UnknownOrMnar,
    KnownMarPropensity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentifiedInterval {
    pub lower: f64,
    pub upper: f64,
}

impl IdentifiedInterval {
    pub fn contains(&self, value: f64) -> bool {
        value + 1e-9 >= self.lower && value <= self.upper + 1e-9
    }

    pub fn width(&self) -> f64 {
        self.upper - self.lower
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservationEstimates {
    pub complete_case_mean: Option<f64>,
    /// A deliberately naïve negative control, not a valid general estimator.
    pub missing_as_zero_mean: Option<f64>,
    /// Finite-population support bounds; no MAR/MNAR point-identification
    /// assumption is made.
    pub identified_interval: IdentifiedInterval,
    pub observation_coverage: f64,
    pub observed_units: usize,
    pub missing_units: usize,
    pub missing_by_reason: BTreeMap<MissingReason, usize>,
    pub propensity_units: usize,
    pub missing_propensity_units: usize,
    pub minimum_propensity_bps: Option<u32>,
    pub maximum_propensity_bps: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StratumEvaluation {
    pub stratum_id: String,
    pub true_mean: f64,
    pub estimates: ObservationEstimates,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyEvaluation {
    pub policy_id: String,
    pub true_mechanism: MissingnessMechanism,
    pub true_mean: f64,
    pub true_min_stratum_mean: f64,
    pub true_max_stratum_mean: f64,
    pub true_stratum_gap: f64,
    pub estimates: ObservationEstimates,
    pub strata: Vec<StratumEvaluation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobustComparison {
    LeftDominates,
    RightDominates,
    Unresolved,
}

impl RobustComparison {
    pub const fn id(self) -> &'static str {
        match self {
            Self::LeftDominates => "left_dominates",
            Self::RightDominates => "right_dominates",
            Self::Unresolved => "unresolved",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationError {
    InvalidOutcomeSupport,
    EmptyPopulation,
    MixedPolicyIds,
    EmptyIdentity,
    ZeroPopulationWeight,
    DuplicateUnit,
    OutcomeOutsideSupport,
    ObservedOutcomeMismatch,
    InvalidObservationState,
    InvalidPropensity,
    MissingPropensity,
    UnsupportedPropensityAssumption,
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EvaluationError {}

fn validate_observations(
    records: &[CandidateRecord],
    support: OutcomeSupport,
) -> Result<(), EvaluationError> {
    support.validate()?;
    if records.is_empty() {
        return Err(EvaluationError::EmptyPopulation);
    }

    let policy_id = records[0].policy_id.as_str();
    let mut unit_ids = BTreeSet::new();

    for record in records {
        if record.policy_id.trim().is_empty()
            || record.unit_id.trim().is_empty()
            || record.stratum_id.trim().is_empty()
        {
            return Err(EvaluationError::EmptyIdentity);
        }
        if record.policy_id != policy_id {
            return Err(EvaluationError::MixedPolicyIds);
        }
        if record.population_weight == 0 {
            return Err(EvaluationError::ZeroPopulationWeight);
        }
        if !unit_ids.insert(record.unit_id.as_str()) {
            return Err(EvaluationError::DuplicateUnit);
        }

        match (record.observed_outcome, record.missing_reason) {
            (Some(value), None) if support.contains(value) => {}
            (Some(_), None) => return Err(EvaluationError::OutcomeOutsideSupport),
            (None, Some(_)) => {}
            _ => return Err(EvaluationError::InvalidObservationState),
        }

        if let Some(propensity) = record.observation_propensity_bps
            && (propensity == 0 || propensity > 10_000)
        {
            return Err(EvaluationError::InvalidPropensity);
        }
    }

    Ok(())
}

/// Estimate policy value from observations only. The latent outcome and true
/// mechanism are not accepted by this API.
pub fn estimate_from_observations(
    records: &[CandidateRecord],
    support: OutcomeSupport,
) -> Result<ObservationEstimates, EvaluationError> {
    validate_observations(records, support)?;

    let total_weight: f64 = records
        .iter()
        .map(|record| record.population_weight as f64)
        .sum();
    let mut observed_weight = 0.0;
    let mut observed_total = 0.0;
    let mut missing_weight = 0.0;
    let mut missing_units = 0_usize;
    let mut observed_units = 0_usize;
    let mut missing_by_reason = BTreeMap::new();
    let mut propensity_units = 0_usize;
    let mut missing_propensity_units = 0_usize;
    let mut minimum_propensity_bps: Option<u32> = None;
    let mut maximum_propensity_bps: Option<u32> = None;

    for record in records {
        if let Some(propensity) = record.observation_propensity_bps {
            propensity_units += 1;
            minimum_propensity_bps =
                Some(minimum_propensity_bps.map_or(propensity, |v| v.min(propensity)));
            maximum_propensity_bps =
                Some(maximum_propensity_bps.map_or(propensity, |v| v.max(propensity)));
        } else {
            missing_propensity_units += 1;
        }
        let weight = record.population_weight as f64;
        match (record.observed_outcome, record.missing_reason) {
            (Some(value), None) => {
                observed_weight += weight;
                observed_total += weight * value as f64;
                observed_units += 1;
            }
            (None, Some(reason)) => {
                missing_weight += weight;
                missing_units += 1;
                *missing_by_reason.entry(reason).or_insert(0) += 1;
            }
            _ => return Err(EvaluationError::InvalidObservationState),
        }
    }

    let complete_case_mean = if observed_weight > 0.0 {
        Some(observed_total / observed_weight)
    } else {
        None
    };
    let missing_as_zero_mean = if support.contains(0) {
        Some(observed_total / total_weight)
    } else {
        None
    };
    let identified_interval = IdentifiedInterval {
        lower: (observed_total + missing_weight * support.lower as f64) / total_weight,
        upper: (observed_total + missing_weight * support.upper as f64) / total_weight,
    };

    Ok(ObservationEstimates {
        complete_case_mean,
        missing_as_zero_mean,
        identified_interval,
        observation_coverage: observed_weight / total_weight,
        observed_units,
        missing_units,
        missing_by_reason,
        propensity_units,
        missing_propensity_units,
        minimum_propensity_bps,
        maximum_propensity_bps,
    })
}

/// Horvitz-Thompson mean for a separately declared MAR design with a known,
/// positive observation propensity on every expected unit. This method must
/// not be applied to unknown or MNAR observation mechanisms.
pub fn known_mar_propensity_mean(
    records: &[CandidateRecord],
    support: OutcomeSupport,
    assumption: ObservationAssumption,
) -> Result<f64, EvaluationError> {
    if assumption != ObservationAssumption::KnownMarPropensity {
        return Err(EvaluationError::UnsupportedPropensityAssumption);
    }
    validate_observations(records, support)?;

    let total_weight: f64 = records
        .iter()
        .map(|record| record.population_weight as f64)
        .sum();
    let mut weighted_total = 0.0;

    for record in records {
        let propensity = record
            .observation_propensity_bps
            .ok_or(EvaluationError::MissingPropensity)?;
        if let Some(value) = record.observed_outcome {
            let probability = propensity as f64 / 10_000.0;
            weighted_total += record.population_weight as f64 * value as f64 / probability;
        }
    }

    Ok(weighted_total / total_weight)
}

/// Evaluator-side scoring. The candidate estimator is first called with only
/// observation records; latent values are used afterward for known-truth
/// qualification and never passed into estimator functions.
pub fn evaluate_policy(
    records: &[EvaluatorRecord],
    support: OutcomeSupport,
) -> Result<PolicyEvaluation, EvaluationError> {
    if records.is_empty() {
        return Err(EvaluationError::EmptyPopulation);
    }

    let observations: Vec<CandidateRecord> = records
        .iter()
        .map(|record| record.observation.clone())
        .collect();
    let estimates = estimate_from_observations(&observations, support)?;
    let policy_id = observations[0].policy_id.clone();

    let mechanisms: BTreeSet<MissingnessMechanism> =
        records.iter().map(|record| record.true_mechanism).collect();
    let mechanism = if mechanisms.len() == 1 {
        *mechanisms.iter().next().expect("non-empty mechanism set")
    } else {
        MissingnessMechanism::Mixed
    };
    for record in records {
        if !support.contains(record.latent_outcome) {
            return Err(EvaluationError::OutcomeOutsideSupport);
        }
        if let Some(observed) = record.observation.observed_outcome
            && observed != record.latent_outcome
        {
            return Err(EvaluationError::ObservedOutcomeMismatch);
        }
    }

    let total_weight: f64 = records
        .iter()
        .map(|record| record.observation.population_weight as f64)
        .sum();
    let weighted_truth: f64 = records
        .iter()
        .map(|record| record.observation.population_weight as f64 * record.latent_outcome as f64)
        .sum();
    let true_mean = weighted_truth / total_weight;

    let mut by_stratum: BTreeMap<&str, Vec<&EvaluatorRecord>> = BTreeMap::new();
    for record in records {
        by_stratum
            .entry(record.observation.stratum_id.as_str())
            .or_default()
            .push(record);
    }

    let mut strata = Vec::with_capacity(by_stratum.len());
    for (stratum_id, stratum_rows) in by_stratum {
        let stratum_observations: Vec<CandidateRecord> = stratum_rows
            .iter()
            .map(|record| record.observation.clone())
            .collect();
        let stratum_estimates = estimate_from_observations(&stratum_observations, support)?;
        let stratum_weight: f64 = stratum_rows
            .iter()
            .map(|record| record.observation.population_weight as f64)
            .sum();
        let stratum_truth: f64 = stratum_rows
            .iter()
            .map(|record| {
                record.observation.population_weight as f64 * record.latent_outcome as f64
            })
            .sum();

        strata.push(StratumEvaluation {
            stratum_id: stratum_id.to_owned(),
            true_mean: stratum_truth / stratum_weight,
            estimates: stratum_estimates,
        });
    }

    let true_min_stratum_mean = strata
        .iter()
        .map(|stratum| stratum.true_mean)
        .fold(f64::INFINITY, f64::min);
    let true_max_stratum_mean = strata
        .iter()
        .map(|stratum| stratum.true_mean)
        .fold(f64::NEG_INFINITY, f64::max);

    Ok(PolicyEvaluation {
        policy_id,
        true_mechanism: mechanism,
        true_mean,
        true_min_stratum_mean,
        true_max_stratum_mean,
        true_stratum_gap: true_max_stratum_mean - true_min_stratum_mean,
        estimates,
        strata,
    })
}

/// Strict interval dominance for a reward-maximization objective. Overlap
/// means the ranking is unresolved; the midpoint is never substituted.
pub fn robust_comparison(
    left: &IdentifiedInterval,
    right: &IdentifiedInterval,
) -> RobustComparison {
    if left.lower > right.upper {
        RobustComparison::LeftDominates
    } else if right.lower > left.upper {
        RobustComparison::RightDominates
    } else {
        RobustComparison::Unresolved
    }
}

/// Conservative pairwise regret upper bound when selecting selected instead
/// of competitor for a reward-maximization objective.
pub fn worst_case_regret_upper_bound(
    selected: &IdentifiedInterval,
    competitor: &IdentifiedInterval,
) -> f64 {
    (competitor.upper - selected.lower).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUPPORT: OutcomeSupport = OutcomeSupport {
        lower: 0,
        upper: 100,
    };

    fn record(input: (&str, &str, &str, u64, bool, MissingnessMechanism)) -> EvaluatorRecord {
        let (policy, unit, stratum, truth, observed, mechanism) = input;
        let (observed_outcome, missing_reason) = if observed {
            (Some(truth), None)
        } else {
            (None, Some(MissingReason::PolicySuppressed))
        };
        EvaluatorRecord {
            observation: CandidateRecord {
                policy_id: policy.to_owned(),
                unit_id: unit.to_owned(),
                stratum_id: stratum.to_owned(),
                population_weight: 1,
                observed_outcome,
                missing_reason,
                observation_propensity_bps: None,
            },
            latent_outcome: truth,
            true_mechanism: mechanism,
        }
    }

    #[test]
    fn complete_case_can_reverse_policy_ranking_when_adverse_outcomes_are_hidden() {
        let masked = vec![
            record((
                "masked",
                "u1",
                "high_need",
                100,
                true,
                MissingnessMechanism::MnarAdverseOutcomesHidden,
            )),
            record((
                "masked",
                "u2",
                "high_need",
                0,
                false,
                MissingnessMechanism::MnarAdverseOutcomesHidden,
            )),
            record((
                "masked",
                "u3",
                "general",
                0,
                false,
                MissingnessMechanism::MnarAdverseOutcomesHidden,
            )),
            record((
                "masked",
                "u4",
                "general",
                0,
                false,
                MissingnessMechanism::MnarAdverseOutcomesHidden,
            )),
        ];
        let transparent = vec![
            record((
                "transparent",
                "u1",
                "high_need",
                60,
                true,
                MissingnessMechanism::Complete,
            )),
            record((
                "transparent",
                "u2",
                "high_need",
                60,
                true,
                MissingnessMechanism::Complete,
            )),
            record((
                "transparent",
                "u3",
                "general",
                60,
                true,
                MissingnessMechanism::Complete,
            )),
            record((
                "transparent",
                "u4",
                "general",
                60,
                true,
                MissingnessMechanism::Complete,
            )),
        ];

        let a = evaluate_policy(&masked, SUPPORT).expect("masked policy");
        let b = evaluate_policy(&transparent, SUPPORT).expect("transparent policy");

        assert_eq!(a.estimates.complete_case_mean, Some(100.0));
        assert_eq!(a.true_mean, 25.0);
        assert_eq!(b.estimates.complete_case_mean, Some(60.0));
        assert_eq!(b.true_mean, 60.0);
        assert_eq!(
            robust_comparison(
                &a.estimates.identified_interval,
                &b.estimates.identified_interval
            ),
            RobustComparison::Unresolved
        );
        assert_eq!(a.true_stratum_gap, 50.0);
        assert_eq!(b.true_stratum_gap, 0.0);
        assert_eq!(a.estimates.observation_coverage, 0.25);
    }

    #[test]
    fn missing_as_zero_can_reverse_policy_ranking_when_beneficial_outcomes_are_hidden() {
        let hidden = vec![
            record((
                "hidden_successes",
                "u1",
                "high_need",
                100,
                true,
                MissingnessMechanism::MnarBeneficialOutcomesHidden,
            )),
            record((
                "hidden_successes",
                "u2",
                "high_need",
                100,
                false,
                MissingnessMechanism::MnarBeneficialOutcomesHidden,
            )),
            record((
                "hidden_successes",
                "u3",
                "general",
                100,
                false,
                MissingnessMechanism::MnarBeneficialOutcomesHidden,
            )),
            record((
                "hidden_successes",
                "u4",
                "general",
                100,
                false,
                MissingnessMechanism::MnarBeneficialOutcomesHidden,
            )),
        ];
        let steady = vec![
            record((
                "steady",
                "u1",
                "high_need",
                60,
                true,
                MissingnessMechanism::Complete,
            )),
            record((
                "steady",
                "u2",
                "high_need",
                60,
                true,
                MissingnessMechanism::Complete,
            )),
            record((
                "steady",
                "u3",
                "general",
                60,
                true,
                MissingnessMechanism::Complete,
            )),
            record((
                "steady",
                "u4",
                "general",
                60,
                true,
                MissingnessMechanism::Complete,
            )),
        ];

        let a = evaluate_policy(&hidden, SUPPORT).expect("hidden policy");
        let b = evaluate_policy(&steady, SUPPORT).expect("steady policy");

        assert_eq!(a.estimates.missing_as_zero_mean, Some(25.0));
        assert_eq!(a.true_mean, 100.0);
        assert_eq!(b.estimates.missing_as_zero_mean, Some(60.0));
        assert_eq!(b.true_mean, 60.0);
        assert_eq!(a.estimates.identified_interval.lower, 25.0);
        assert_eq!(a.estimates.identified_interval.upper, 100.0);
        assert_eq!(
            robust_comparison(
                &a.estimates.identified_interval,
                &b.estimates.identified_interval
            ),
            RobustComparison::Unresolved
        );
    }

    #[test]
    fn identified_bounds_contain_truth_for_all_frozen_mnar_fixtures() {
        let rows = [
            record((
                "p",
                "u1",
                "a",
                100,
                true,
                MissingnessMechanism::MnarAdverseOutcomesHidden,
            )),
            record((
                "p",
                "u2",
                "a",
                0,
                false,
                MissingnessMechanism::MnarAdverseOutcomesHidden,
            )),
            record((
                "p",
                "u3",
                "b",
                0,
                false,
                MissingnessMechanism::MnarAdverseOutcomesHidden,
            )),
            record((
                "p",
                "u4",
                "b",
                0,
                false,
                MissingnessMechanism::MnarAdverseOutcomesHidden,
            )),
        ];
        let result = evaluate_policy(&rows, SUPPORT).expect("evaluation");
        assert!(
            result
                .estimates
                .identified_interval
                .contains(result.true_mean)
        );
    }

    #[test]
    fn all_missing_is_full_support_not_zero_reward() {
        let rows = [
            record((
                "p",
                "u1",
                "a",
                0,
                false,
                MissingnessMechanism::MnarAdverseOutcomesHidden,
            )),
            record((
                "p",
                "u2",
                "a",
                100,
                false,
                MissingnessMechanism::MnarAdverseOutcomesHidden,
            )),
        ];
        let result = evaluate_policy(&rows, SUPPORT).expect("evaluation");
        assert_eq!(result.estimates.complete_case_mean, None);
        assert_eq!(result.estimates.missing_as_zero_mean, Some(0.0));
        assert_eq!(result.estimates.identified_interval.lower, 0.0);
        assert_eq!(result.estimates.identified_interval.upper, 100.0);
        assert_eq!(result.estimates.observation_coverage, 0.0);
    }

    #[test]
    fn observed_zero_is_not_missing() {
        let rows = [
            record((
                "p",
                "observed_zero",
                "a",
                0,
                true,
                MissingnessMechanism::Complete,
            )),
            record((
                "p",
                "missing",
                "a",
                100,
                false,
                MissingnessMechanism::MnarBeneficialOutcomesHidden,
            )),
        ];
        let observations: Vec<_> = rows.iter().map(|row| row.observation.clone()).collect();
        let estimates = estimate_from_observations(&observations, SUPPORT).expect("estimates");
        assert_eq!(estimates.observed_units, 1);
        assert_eq!(estimates.missing_units, 1);
        assert_eq!(estimates.complete_case_mean, Some(0.0));
        assert_eq!(estimates.observation_coverage, 0.5);
        assert_eq!(
            estimates
                .missing_by_reason
                .get(&MissingReason::PolicySuppressed),
            Some(&1)
        );
    }

    #[test]
    fn known_mar_propensity_positive_control_recovers_balanced_finite_fixture() {
        let mut rows = Vec::new();
        for (unit, stratum, truth, observed) in [
            ("u1", "lower", 20, true),
            ("u2", "lower", 20, false),
            ("u3", "upper", 80, true),
            ("u4", "upper", 80, false),
        ] {
            let mut row = record((
                "mar",
                unit,
                stratum,
                truth,
                observed,
                MissingnessMechanism::Mar,
            ));
            row.observation.observation_propensity_bps = Some(5_000);
            rows.push(row);
        }
        let observations: Vec<_> = rows.iter().map(|row| row.observation.clone()).collect();
        let estimate = known_mar_propensity_mean(
            &observations,
            SUPPORT,
            ObservationAssumption::KnownMarPropensity,
        )
        .expect("MAR estimate");
        let evaluation = evaluate_policy(&rows, SUPPORT).expect("MAR evaluation");
        assert_eq!(estimate, 50.0);
        assert_eq!(evaluation.true_mean, 50.0);
    }

    #[test]
    fn inverse_propensity_estimator_rejects_unknown_or_mnar_assumption() {
        let rows = [record((
            "mnar",
            "u1",
            "a",
            10,
            true,
            MissingnessMechanism::MnarAdverseOutcomesHidden,
        ))];
        let observations: Vec<_> = rows.iter().map(|row| row.observation.clone()).collect();
        assert_eq!(
            known_mar_propensity_mean(&observations, SUPPORT, ObservationAssumption::UnknownOrMnar),
            Err(EvaluationError::UnsupportedPropensityAssumption)
        );
    }

    #[test]
    fn inverse_propensity_estimator_rejects_missing_propensity_and_zero_support() {
        let mut row = record(("mar", "u1", "a", 10, true, MissingnessMechanism::Mar));
        let missing_propensity = vec![row.observation.clone()];
        assert_eq!(
            known_mar_propensity_mean(
                &missing_propensity,
                SUPPORT,
                ObservationAssumption::KnownMarPropensity
            ),
            Err(EvaluationError::MissingPropensity)
        );
        row.observation.observation_propensity_bps = Some(0);
        assert_eq!(
            estimate_from_observations(&[row.observation], SUPPORT),
            Err(EvaluationError::InvalidPropensity)
        );
    }

    #[test]
    fn duplicate_ids_and_malformed_observation_states_fail_closed() {
        let row = record(("p", "u1", "a", 10, true, MissingnessMechanism::Complete));
        let mut duplicate = row.observation.clone();
        duplicate.observed_outcome = Some(20);
        assert_eq!(
            estimate_from_observations(&[row.observation.clone(), duplicate], SUPPORT),
            Err(EvaluationError::DuplicateUnit)
        );

        let mut malformed = row.observation;
        malformed.observed_outcome = None;
        malformed.missing_reason = None;
        assert_eq!(
            estimate_from_observations(&[malformed], SUPPORT),
            Err(EvaluationError::InvalidObservationState)
        );
    }

    #[test]
    fn mixed_evaluator_mechanisms_are_preserved_as_mixed_not_rejected() {
        let rows = [
            record(("p", "u1", "a", 10, true, MissingnessMechanism::Mar)),
            record((
                "p",
                "u2",
                "a",
                20,
                false,
                MissingnessMechanism::MnarBeneficialOutcomesHidden,
            )),
        ];
        let result = evaluate_policy(&rows, SUPPORT).expect("mixed mechanism evaluation");
        assert_eq!(result.true_mechanism, MissingnessMechanism::Mixed);
        assert!(
            result
                .estimates
                .identified_interval
                .contains(result.true_mean)
        );
    }

    #[test]
    fn evaluator_rejects_observed_value_that_disagrees_with_latent_truth() {
        let mut row = record(("p", "u1", "a", 10, true, MissingnessMechanism::Complete));
        row.observation.observed_outcome = Some(11);
        assert_eq!(
            evaluate_policy(&[row], SUPPORT),
            Err(EvaluationError::ObservedOutcomeMismatch)
        );
    }

    #[test]
    fn propensity_bounds_must_have_positive_support() {
        let row = record(("p", "u1", "a", 10, true, MissingnessMechanism::Mar));
        let mut observations = row.observation;
        observations.observation_propensity_bps = Some(10_001);
        assert_eq!(
            estimate_from_observations(&[observations], SUPPORT),
            Err(EvaluationError::InvalidPropensity)
        );
    }

    #[test]
    fn regret_bound_uses_identified_sets_not_midpoints() {
        let selected = IdentifiedInterval {
            lower: 25.0,
            upper: 100.0,
        };
        let competitor = IdentifiedInterval {
            lower: 60.0,
            upper: 60.0,
        };
        assert_eq!(worst_case_regret_upper_bound(&selected, &competitor), 35.0);
        assert_eq!(
            robust_comparison(&selected, &competitor),
            RobustComparison::Unresolved
        );
    }
}
