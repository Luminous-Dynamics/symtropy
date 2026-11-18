//! Synthetic chosen-query attack against aggregate preference summaries.
//!
//! This is a red-team benchmark, not a production feature. It demonstrates
//! that direct identifier redaction and a minimum cohort size do not prevent
//! reconstruction when callers can submit arbitrary outcome vectors and read
//! exact aggregate satisfaction values.

use crate::pluralistic::{
    evaluate_group, ConsentScope, ConsentStatus, DesignAlternative, PreferenceEvent,
    PreferenceLedger, PreferenceError, CRITERION_COUNT, DecisionCriterion,
};

pub const PRIVACY_ATTACK_SCHEMA_VERSION: u32 = 1;
pub const PRIVACY_ATTACK_GENERATOR: &str = "holofrac-basis-query-reconstruction-v0";
pub const SYNTHETIC_COHORT_SIZE: usize = 5;

#[derive(Clone, Debug, PartialEq)]
pub struct RecoveredCriterion {
    pub criterion: DecisionCriterion,
    pub expected_mean_weight: f64,
    pub recovered_mean_weight: f64,
    pub absolute_error: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrivacyAttackReport {
    pub cohort_size: usize,
    pub query_count: usize,
    pub recovered: Vec<RecoveredCriterion>,
    pub max_absolute_error: f64,
    pub mean_absolute_error: f64,
    pub recovered_weight_sum: f64,
    pub reconstruction_complete: bool,
}

fn synthetic_weights() -> [[f64; CRITERION_COUNT]; SYNTHETIC_COHORT_SIZE] {
    [
        [0.40, 0.10, 0.10, 0.10, 0.10, 0.10, 0.05, 0.05],
        [0.10, 0.10, 0.05, 0.05, 0.05, 0.40, 0.10, 0.15],
        [0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.40, 0.30],
        [0.10, 0.10, 0.05, 0.05, 0.45, 0.05, 0.05, 0.15],
        [0.05, 0.15, 0.05, 0.25, 0.10, 0.05, 0.20, 0.15],
    ]
}

fn synthetic_events() -> Vec<PreferenceEvent> {
    synthetic_weights()
        .into_iter()
        .enumerate()
        .map(|(index, weights)| PreferenceEvent {
            event_id: index as u64 + 1,
            stakeholder_id: format!("synthetic:anonymous-{index}"),
            revision: 1,
            previous_event_id: None,
            effective_at_micros: 100,
            recorded_at_micros: 100,
            consent_scope: ConsentScope::AnonymousAggregate,
            consent_status: ConsentStatus::Granted,
            weights,
            veto_thresholds: [None; CRITERION_COUNT],
        })
        .collect()
}

fn expected_mean_weights() -> [f64; CRITERION_COUNT] {
    let mut means = [0.0; CRITERION_COUNT];
    for profile in synthetic_weights() {
        for (mean, weight) in means.iter_mut().zip(profile) {
            *mean += weight / SYNTHETIC_COHORT_SIZE as f64;
        }
    }
    means
}

fn basis_alternative(criterion_index: usize) -> DesignAlternative {
    let mut outcomes = [0.0; CRITERION_COUNT];
    outcomes[criterion_index] = 1.0;
    DesignAlternative {
        alternative_id: format!("chosen-query-basis-{criterion_index}"),
        outcomes,
        hard_constraints_satisfied: true,
    }
}

/// Recover the exact mean preference vector using one unit-basis alternative
/// per criterion. Each query returns one aggregate mean satisfaction value.
/// This is possible only because the current API accepts arbitrary alternatives
/// and exposes unrounded aggregate means.
pub fn run_synthetic_basis_query_attack() -> Result<PrivacyAttackReport, PreferenceError> {
    let expected = expected_mean_weights();
    let ledger = PreferenceLedger::from_events(synthetic_events())?;
    let snapshot = ledger.snapshot_as_of(100)?;

    let mut recovered = Vec::with_capacity(CRITERION_COUNT);
    for index in 0..CRITERION_COUNT {
        let alternative = basis_alternative(index);
        let report = evaluate_group(&[alternative], &snapshot)?;
        let observed = report
            .assessments
            .first()
            .ok_or(PreferenceError::NoParticipatingStakeholders)?
            .mean_satisfaction;
        recovered.push(RecoveredCriterion {
            criterion: DecisionCriterion::ALL[index],
            expected_mean_weight: expected[index],
            recovered_mean_weight: observed,
            absolute_error: (expected[index] - observed).abs(),
        });
    }

    let max_absolute_error = recovered
        .iter()
        .map(|row| row.absolute_error)
        .fold(0.0_f64, f64::max);
    let mean_absolute_error = recovered
        .iter()
        .map(|row| row.absolute_error)
        .sum::<f64>()
        / CRITERION_COUNT as f64;
    let recovered_weight_sum = recovered
        .iter()
        .map(|row| row.recovered_mean_weight)
        .sum::<f64>();

    Ok(PrivacyAttackReport {
        cohort_size: SYNTHETIC_COHORT_SIZE,
        query_count: CRITERION_COUNT,
        recovered,
        max_absolute_error,
        mean_absolute_error,
        recovered_weight_sum,
        reconstruction_complete: max_absolute_error < 1.0e-12
            && (recovered_weight_sum - 1.0).abs() < 1.0e-12,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_adaptive_basis_queries_recover_the_exact_mean_preference_vector() {
        let report = run_synthetic_basis_query_attack().unwrap();
        assert_eq!(report.cohort_size, MIN_COHORT_SIZE_FOR_TEST);
        assert_eq!(report.query_count, CRITERION_COUNT);
        assert!(report.reconstruction_complete);
        assert!(report.max_absolute_error < 1.0e-12);
        assert!((report.recovered_weight_sum - 1.0).abs() < 1.0e-12);
    }

    const MIN_COHORT_SIZE_FOR_TEST: usize = 5;

    #[test]
    fn attack_uses_only_redacted_aggregate_output() {
        let report = run_synthetic_basis_query_attack().unwrap();
        assert!(report.recovered.iter().all(|row| row.absolute_error < 1.0e-12));
        assert_eq!(
            report.recovered.iter().map(|row| row.criterion).collect::<Vec<_>>(),
            DecisionCriterion::ALL
        );
    }
}
