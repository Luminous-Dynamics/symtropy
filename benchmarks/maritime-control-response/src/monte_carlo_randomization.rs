#![forbid(unsafe_code)]

//! Deterministic Monte Carlo Fisher randomization inference for blocked trials.
//!
//! The exact small-sample implementation remains the reference. This module
//! samples uniformly from the same blocked assignment space using a fixed
//! SplitMix64 stream and a preregistered draw budget. It reports Monte Carlo
//! uncertainty separately from the causal estimator's sampling and measurement
//! uncertainty. It does not create or authenticate the real-world assignment.

use crate::maintenance_effect::{
    AssignedArm, BlockedRandomizedMaintenanceTrial, MaintenanceEffectError,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;

pub const MONTE_CARLO_RANDOMIZATION_SCHEMA: &str =
    "symtropy-marine-monte-carlo-randomization-v0.1";
pub const MONTE_CARLO_RANDOMIZATION_ALGORITHM: &str = "splitmix64_v1";
pub const MIN_MONTE_CARLO_DRAWS: usize = 100;
pub const MAX_MONTE_CARLO_DRAWS: usize = 2_000_000;
pub const MAX_MONTE_CARLO_UNITS: usize = 10_000;
pub const MAX_TOTAL_UNIT_DRAWS: usize = 100_000_000;
const VARIANCE_TOLERANCE: f64 = 1e-12;
const WILSON_95_Z: f64 = 1.959_963_984_540_054;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonteCarloRandomizationConfig {
    pub seed_u64: u64,
    pub draw_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MonteCarloRandomizationResult {
    pub schema_version: String,
    pub trial_id: String,
    pub assignment_plan_sha256: String,
    pub randomization_frame_sha256: String,
    pub algorithm: String,
    pub seed_u64: u64,
    pub draw_count: usize,
    pub observed_treatment_minus_control: f64,
    pub exceedance_count: usize,
    /// Finite-sample corrected p-value: (1 + exceedances) / (B + 1).
    pub p_value_plus_one: f64,
    /// Uncorrected Monte Carlo estimate of the randomization tail probability.
    pub tail_probability_estimate: f64,
    pub monte_carlo_standard_error: f64,
    /// Wilson 95% interval for the underlying tail probability.
    pub tail_probability_wilson_95: [f64; 2],
    pub sampled_assignments_sha256: String,
    pub allocation_constraint_violations: usize,
    pub exact_enumeration_used: bool,
    pub maintenance_authorized: bool,
    pub physical_restoration_authorized: bool,
    pub operating_restriction_clearance_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonteCarloRandomizationError {
    InvalidDrawCount(usize),
    TooManyUnits(usize),
    WorkBudgetExceeded {
        draw_count: usize,
        unit_count: usize,
        maximum_unit_draws: usize,
    },
    InvalidTrial(MaintenanceEffectError),
    MissingStratum(String),
    InvalidAssignmentSpace,
    Serialization,
}

impl fmt::Display for MonteCarloRandomizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDrawCount(count) => write!(
                formatter,
                "Monte Carlo draw count must be between {MIN_MONTE_CARLO_DRAWS} and {MAX_MONTE_CARLO_DRAWS}; got {count}"
            ),
            Self::TooManyUnits(count) => write!(
                formatter,
                "Monte Carlo blocked sampler is bounded to {MAX_MONTE_CARLO_UNITS} units; got {count}"
            ),
            Self::WorkBudgetExceeded {
                draw_count,
                unit_count,
                maximum_unit_draws,
            } => write!(
                formatter,
                "Monte Carlo work budget exceeded: {draw_count} draws × {unit_count} units > {maximum_unit_draws}"
            ),
            Self::InvalidTrial(error) => write!(formatter, "blocked trial is invalid: {error}"),
            Self::MissingStratum(id) => write!(formatter, "randomization stratum is missing: {id}"),
            Self::InvalidAssignmentSpace => {
                write!(formatter, "no admissible blocked assignment could be sampled")
            }
            Self::Serialization => write!(formatter, "failed to serialize Monte Carlo provenance"),
        }
    }
}

impl std::error::Error for MonteCarloRandomizationError {}

impl From<MaintenanceEffectError> for MonteCarloRandomizationError {
    fn from(error: MaintenanceEffectError) -> Self {
        Self::InvalidTrial(error)
    }
}

/// SplitMix64, with its constants and wrapping semantics frozen for replay.
#[derive(Clone, Copy, Debug)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    /// Uniform integer in [0, upper), using rejection to avoid modulo bias.
    fn uniform_below(&mut self, upper: usize) -> Option<usize> {
        if upper == 0 {
            return None;
        }
        let bound = upper as u64;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return Some((value % bound) as usize);
            }
        }
    }
}

fn stratum_indices(
    trial: &BlockedRandomizedMaintenanceTrial,
) -> BTreeMap<String, Vec<usize>> {
    let mut result = BTreeMap::<String, Vec<usize>>::new();
    for (index, observation) in trial.observations.iter().enumerate() {
        result
            .entry(observation.stratum_id.clone())
            .or_default()
            .push(index);
    }
    for indices in result.values_mut() {
        indices.sort_by(|left, right| {
            trial.observations[*left]
                .unit_id
                .cmp(&trial.observations[*right].unit_id)
        });
    }
    result
}

fn allocation_map(
    trial: &BlockedRandomizedMaintenanceTrial,
) -> BTreeMap<&str, (usize, usize)> {
    trial
        .stratum_allocations
        .iter()
        .map(|allocation| {
            (
                allocation.stratum_id.as_str(),
                (
                    allocation.planned_treatment_count,
                    allocation.planned_control_count,
                ),
            )
        })
        .collect()
}

fn update_len_prefixed(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

/// Hash the randomization frame in canonical ID order, independent of JSON row order.
fn randomization_frame_sha256(
    trial: &BlockedRandomizedMaintenanceTrial,
    groups: &BTreeMap<String, Vec<usize>>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"symtropy-marine-randomization-frame-v1\\0");
    update_len_prefixed(&mut hasher, &trial.trial_id);
    hasher.update(trial.assignment_plan_sha256.as_bytes());

    let allocations = allocation_map(trial);
    for (stratum_id, indices) in groups {
        update_len_prefixed(&mut hasher, stratum_id);
        let (treatment_count, control_count) = allocations[stratum_id.as_str()];
        hasher.update((treatment_count as u64).to_le_bytes());
        hasher.update((control_count as u64).to_le_bytes());
        for index in indices {
            let observation = &trial.observations[*index];
            update_len_prefixed(&mut hasher, &observation.unit_id);
            update_len_prefixed(&mut hasher, &observation.stratum_id);
            hasher.update([u8::from(observation.assigned_arm == AssignedArm::Treatment)]);
            hasher.update(observation.observed_outcome_value.to_bits().to_le_bytes());
            update_len_prefixed(&mut hasher, &observation.outcome_unit);
        }
    }
    format!("{:x}", hasher.finalize())
}

fn update_sampled_assignment_digest(
    hasher: &mut Sha256,
    trial: &BlockedRandomizedMaintenanceTrial,
    groups: &BTreeMap<String, Vec<usize>>,
    assignment: &[bool],
) {
    for indices in groups.values() {
        for index in indices {
            let observation = &trial.observations[*index];
            update_len_prefixed(hasher, &observation.unit_id);
            hasher.update([u8::from(assignment[*index])]);
        }
    }
}

fn validate_work_budget(
    unit_count: usize,
    draw_count: usize,
) -> Result<(), MonteCarloRandomizationError> {
    let total = draw_count.checked_mul(unit_count).unwrap_or(usize::MAX);
    if total > MAX_TOTAL_UNIT_DRAWS {
        return Err(MonteCarloRandomizationError::WorkBudgetExceeded {
            draw_count,
            unit_count,
            maximum_unit_draws: MAX_TOTAL_UNIT_DRAWS,
        });
    }
    Ok(())
}

fn sample_blocked_assignment(
    trial: &BlockedRandomizedMaintenanceTrial,
    groups: &BTreeMap<String, Vec<usize>>,
    rng: &mut SplitMix64,
) -> Result<Vec<bool>, MonteCarloRandomizationError> {
    let mut treatment_mask = vec![false; trial.observations.len()];
    let allocations = allocation_map(trial);
    for (stratum_id, indices) in groups {
        let Some((planned_treatment_count, _)) = allocations.get(stratum_id.as_str()) else {
            return Err(MonteCarloRandomizationError::MissingStratum(
                stratum_id.clone(),
            ));
        };
        if *planned_treatment_count > indices.len() {
            return Err(MonteCarloRandomizationError::InvalidAssignmentSpace);
        }
        let mut shuffled = indices.clone();
        let treatment_count = *planned_treatment_count;
        for position in 0..treatment_count {
            let remaining = shuffled.len() - position;
            let offset = rng
                .uniform_below(remaining)
                .ok_or(MonteCarloRandomizationError::InvalidAssignmentSpace)?;
            shuffled.swap(position, position + offset);
        }
        for index in shuffled.iter().take(treatment_count) {
            treatment_mask[*index] = true;
        }
    }
    Ok(treatment_mask)
}

fn stratified_difference_in_means(
    trial: &BlockedRandomizedMaintenanceTrial,
    groups: &BTreeMap<String, Vec<usize>>,
    treatment_mask: &[bool],
) -> Result<f64, MonteCarloRandomizationError> {
    if treatment_mask.len() != trial.observations.len() {
        return Err(MonteCarloRandomizationError::InvalidAssignmentSpace);
    }
    let allocations = allocation_map(trial);
    let total_count = trial.observations.len() as f64;
    let mut weighted_effect = 0.0;
    for (stratum_id, indices) in groups {
        let Some((planned_treatment_count, planned_control_count)) =
            allocations.get(stratum_id.as_str())
        else {
            return Err(MonteCarloRandomizationError::MissingStratum(
                stratum_id.clone(),
            ));
        };
        let mut treatment_sum = 0.0;
        let mut control_sum = 0.0;
        let mut treatment_count = 0usize;
        let mut control_count = 0usize;
        for index in indices {
            let observation = &trial.observations[*index];
            if treatment_mask[*index] {
                treatment_sum += observation.observed_outcome_value;
                treatment_count += 1;
            } else {
                control_sum += observation.observed_outcome_value;
                control_count += 1;
            }
        }
        if treatment_count != *planned_treatment_count
            || control_count != *planned_control_count
            || treatment_count == 0
            || control_count == 0
        {
            return Err(MonteCarloRandomizationError::InvalidAssignmentSpace);
        }
        let treatment_mean = treatment_sum / treatment_count as f64;
        let control_mean = control_sum / control_count as f64;
        let stratum_count = (treatment_count + control_count) as f64;
        weighted_effect += (stratum_count / total_count) * (treatment_mean - control_mean);
    }
    Ok(weighted_effect)
}

fn wilson_interval_95(successes: usize, trials: usize) -> [f64; 2] {
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = WILSON_95_Z * WILSON_95_Z;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half_width = WILSON_95_Z
        * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt())
        / denominator;
    [(center - half_width).max(0.0), (center + half_width).min(1.0)]
}

/// Estimate a two-sided Fisher randomization tail probability by fixed-budget Monte Carlo.
///
/// Treatment counts are held fixed within each predeclared stratum. The PRNG seed,
/// algorithm identifier, draw count and digest of all sampled allocations are reported.
/// A plus-one correction prevents a zero p-value. The Wilson interval describes Monte
/// Carlo uncertainty in the tail probability; it is not a confidence interval for the ATE.
pub fn estimate_monte_carlo_randomization_p_value(
    trial: &BlockedRandomizedMaintenanceTrial,
    config: MonteCarloRandomizationConfig,
) -> Result<MonteCarloRandomizationResult, MonteCarloRandomizationError> {
    trial.validate()?;
    if !(MIN_MONTE_CARLO_DRAWS..=MAX_MONTE_CARLO_DRAWS).contains(&config.draw_count) {
        return Err(MonteCarloRandomizationError::InvalidDrawCount(config.draw_count));
    }
    if trial.observations.len() > MAX_MONTE_CARLO_UNITS {
        return Err(MonteCarloRandomizationError::TooManyUnits(
            trial.observations.len(),
        ));
    }

    validate_work_budget(trial.observations.len(), config.draw_count)?;
    let groups = stratum_indices(trial);
    let frame_sha256 = randomization_frame_sha256(trial, &groups);
    let observed_mask = trial
        .observations
        .iter()
        .map(|observation| observation.assigned_arm == AssignedArm::Treatment)
        .collect::<Vec<_>>();
    let observed_statistic =
        stratified_difference_in_means(trial, &groups, &observed_mask)?;
    let mut rng = SplitMix64::new(config.seed_u64);
    let mut hasher = Sha256::new();
    hasher.update(b"symtropy-marine-monte-carlo-assignment-stream-v1\0");
    hasher.update(frame_sha256.as_bytes());
    hasher.update(trial.assignment_plan_sha256.as_bytes());
    hasher.update(MONTE_CARLO_RANDOMIZATION_ALGORITHM.as_bytes());
    hasher.update(config.seed_u64.to_le_bytes());
    hasher.update((config.draw_count as u64).to_le_bytes());

    let mut exceedances = 0usize;
    let mut allocation_constraint_violations = 0usize;
    for _ in 0..config.draw_count {
        let assignment = sample_blocked_assignment(trial, &groups, &mut rng)?;
        update_sampled_assignment_digest(&mut hasher, trial, &groups, &assignment);

        let statistic = stratified_difference_in_means(trial, &groups, &assignment)?;
        if statistic.abs() + 1e-12 >= observed_statistic.abs() {
            exceedances += 1;
        }
    }
    let tail_probability_estimate = exceedances as f64 / config.draw_count as f64;
    let p_value_plus_one =
        (exceedances as f64 + 1.0) / (config.draw_count as f64 + 1.0);
    let monte_carlo_standard_error =
        (tail_probability_estimate * (1.0 - tail_probability_estimate)
            / config.draw_count as f64)
            .sqrt();
    let sampled_assignments_sha256 = format!("{:x}", hasher.finalize());

    Ok(MonteCarloRandomizationResult {
        schema_version: MONTE_CARLO_RANDOMIZATION_SCHEMA.to_owned(),
        trial_id: trial.trial_id.clone(),
        assignment_plan_sha256: trial.assignment_plan_sha256.clone(),
        randomization_frame_sha256: frame_sha256,
        algorithm: MONTE_CARLO_RANDOMIZATION_ALGORITHM.to_owned(),
        seed_u64: config.seed_u64,
        draw_count: config.draw_count,
        observed_treatment_minus_control: observed_statistic,
        exceedance_count: exceedances,
        p_value_plus_one,
        tail_probability_estimate,
        monte_carlo_standard_error,
        tail_probability_wilson_95: wilson_interval_95(exceedances, config.draw_count),
        sampled_assignments_sha256,
        allocation_constraint_violations,
        exact_enumeration_used: false,
        maintenance_authorized: false,
        physical_restoration_authorized: false,
        operating_restriction_clearance_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maintenance_effect::{
        estimate_blocked_randomized_maintenance_effect, AssignmentMethod, OutcomeDirection,
        RandomizedUnitObservation, StratumAllocation,
    };

    fn trial() -> BlockedRandomizedMaintenanceTrial {
        let mut observations = Vec::new();
        let effects = [2.0, 4.0, 6.0];
        for (stratum_index, effect) in effects.iter().enumerate() {
            let stratum_id = format!("load-band-{stratum_index}");
            for unit_index in 0..4 {
                let unit_id = format!("asset-{stratum_index}-{unit_index}");
                let assigned_arm = if unit_index == 0 || unit_index == 3 {
                    AssignedArm::Treatment
                } else {
                    AssignedArm::Control
                };
                let baseline = 10.0 + (unit_index as f64 * 2.0);
                let outcome = baseline
                    + if assigned_arm == AssignedArm::Treatment {
                        *effect
                    } else {
                        0.0
                    };
                let digest_base = (stratum_index * 4 + unit_index) * 7 + 10;
                observations.push(RandomizedUnitObservation {
                    unit_id: unit_id.clone(),
                    stratum_id: stratum_id.clone(),
                    assigned_arm,
                    assignment_evidence_sha256: format!("{:064x}", digest_base),
                    assignment_available_at_seconds: 100.0,
                    baseline_value: baseline,
                    baseline_unit: "normalized_capability".into(),
                    baseline_observed_at_seconds: 80.0,
                    baseline_available_at_seconds: 90.0,
                    baseline_evidence_sha256: format!("{:064x}", digest_base + 1),
                    baseline_source_id: format!("baseline-source-{unit_id}"),
                    baseline_measurement_principle_id: "calibrated_shaft_torque".into(),
                    baseline_dependency_domain_id: format!("baseline-daq-{unit_id}"),
                    baseline_quality: crate::maintenance_effect::MeasurementQuality::Valid,
                    baseline_calibration_status:
                        crate::maintenance_effect::CalibrationStatus::Verified,
                    baseline_calibration_record_sha256: format!("{:064x}", digest_base + 2),
                    baseline_calibration_valid_from_seconds: 0.0,
                    baseline_calibration_valid_until_seconds: 1_000.0,
                    baseline_uncertainty_abs: 0.1,
                    observed_outcome_value: outcome,
                    outcome_unit: "normalized_capability".into(),
                    outcome_observed_at_seconds: 210.0,
                    outcome_available_at_seconds: 220.0,
                    outcome_evidence_sha256: format!("{:064x}", digest_base + 3),
                    outcome_source_id: format!("outcome-source-{unit_id}"),
                    outcome_measurement_principle_id: "calibrated_shaft_torque".into(),
                    outcome_dependency_domain_id: format!("outcome-daq-{unit_id}"),
                    outcome_quality: crate::maintenance_effect::MeasurementQuality::Valid,
                    outcome_calibration_status:
                        crate::maintenance_effect::CalibrationStatus::Verified,
                    outcome_calibration_record_sha256: format!("{:064x}", digest_base + 4),
                    outcome_calibration_valid_from_seconds: 0.0,
                    outcome_calibration_valid_until_seconds: 1_000.0,
                    outcome_uncertainty_abs: 0.25,
                });
            }
        }
        BlockedRandomizedMaintenanceTrial {
            schema_version: crate::maintenance_effect::MAINTENANCE_EFFECT_SCHEMA.into(),
            trial_id: "mc-randomization-test-v1".into(),
            assignment_method: AssignmentMethod::BlockRandomized,
            treatment_label: "repair".into(),
            control_label: "control".into(),
            outcome_metric_id: "normalized_propulsion_capability".into(),
            outcome_unit: "normalized_capability".into(),
            outcome_measurement_principle_id: "calibrated_shaft_torque".into(),
            outcome_direction: OutcomeDirection::HigherIsBetter,
            assignment_at_seconds: 100.0,
            followup_horizon_seconds: 100.0,
            analysis_as_of_seconds: 250.0,
            confidence_level: 0.95,
            assignment_plan_sha256: "c".repeat(64),
            assignment_audit_ref: "test-assignment-plan".into(),
            stratum_allocations: effects
                .iter()
                .enumerate()
                .map(|(index, _)| StratumAllocation {
                    stratum_id: format!("load-band-{index}"),
                    planned_treatment_count: 2,
                    planned_control_count: 2,
                })
                .collect(),
            observations,
        }
    }

    fn larger_trial() -> BlockedRandomizedMaintenanceTrial {
        let mut trial = trial();
        trial.stratum_allocations.push(StratumAllocation {
            stratum_id: "load-band-3".into(),
            planned_treatment_count: 3,
            planned_control_count: 2,
        });
        for index in 0..5 {
            let mut observation = trial.observations[index].clone();
            observation.unit_id = format!("large-asset-{index}");
            observation.stratum_id = "load-band-3".into();
            observation.assigned_arm = if index < 3 {
                AssignedArm::Treatment
            } else {
                AssignedArm::Control
            };
            observation.assignment_evidence_sha256 = format!("{:064x}", 1000 + index);
            observation.baseline_evidence_sha256 = format!("{:064x}", 1100 + index);
            observation.baseline_source_id = format!("baseline-large-{index}");
            observation.baseline_calibration_record_sha256 = format!("{:064x}", 1200 + index);
            observation.outcome_evidence_sha256 = format!("{:064x}", 1300 + index);
            observation.outcome_source_id = format!("outcome-large-{index}");
            observation.outcome_calibration_record_sha256 = format!("{:064x}", 1400 + index);
            trial.observations.push(observation);
        }
        trial
    }

    #[test]
    fn splitmix64_v1_matches_frozen_golden_vector() {
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(rng.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    }

    #[test]
    fn frame_and_assignment_stream_digests_ignore_input_row_order() {
        let original = trial();
        let mut reordered = original.clone();
        reordered.observations.reverse();
        reordered.stratum_allocations.reverse();
        let config = MonteCarloRandomizationConfig {
            seed_u64: 0x1234_5678,
            draw_count: 2_000,
        };
        let first = estimate_monte_carlo_randomization_p_value(&original, config).unwrap();
        let second = estimate_monte_carlo_randomization_p_value(&reordered, config).unwrap();
        assert_eq!(first.randomization_frame_sha256, second.randomization_frame_sha256);
        assert_eq!(first.sampled_assignments_sha256, second.sampled_assignments_sha256);
        assert_eq!(first.p_value_plus_one, second.p_value_plus_one);
    }

    #[test]
    fn total_unit_draw_budget_fails_closed_before_sampling() {
        assert_eq!(
            validate_work_budget(10_000, 10_001),
            Err(MonteCarloRandomizationError::WorkBudgetExceeded {
                draw_count: 10_001,
                unit_count: 10_000,
                maximum_unit_draws: MAX_TOTAL_UNIT_DRAWS,
            })
        );
        assert_eq!(validate_work_budget(10_000, 10_000), Ok(()));
    }

    #[test]
    fn partial_fisher_yates_sampler_covers_all_small_subsets_without_block_violations() {
        let trial = trial();
        let groups = stratum_indices(&trial);
        let mut rng = SplitMix64::new(2026);
        let mut counts = BTreeMap::<u8, usize>::new();
        for _ in 0..12_000 {
            let assignment = sample_blocked_assignment(&trial, &groups, &mut rng).unwrap();
            for indices in groups.values() {
                let mask = indices.iter().enumerate().fold(0u8, |mask, (position, index)| {
                    if assignment[*index] {
                        mask | (1 << position)
                    } else {
                        mask
                    }
                });
                *counts.entry(mask).or_default() += 1;
            }
            assert!(groups.values().all(|indices| {
                indices.iter().filter(|index| assignment[**index]).count() == 2
            }));
        }
        assert_eq!(counts.len(), 6);
        assert!(counts.values().all(|count| (1_700..=2_300).contains(count)));
    }

    #[test]
    fn same_seed_and_inputs_reproduce_identical_assignment_digest_and_p_value() {
        let trial = trial();
        let config = MonteCarloRandomizationConfig {
            seed_u64: 0xCAFE_BABE,
            draw_count: 2_000,
        };
        let first = estimate_monte_carlo_randomization_p_value(&trial, config).unwrap();
        let second = estimate_monte_carlo_randomization_p_value(&trial, config).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.algorithm, MONTE_CARLO_RANDOMIZATION_ALGORITHM);
        assert_eq!(first.allocation_constraint_violations, 0);
        assert!(!first.exact_enumeration_used);
    }

    #[test]
    fn different_seeds_change_the_sampled_assignment_digest() {
        let trial = trial();
        let first = estimate_monte_carlo_randomization_p_value(
            &trial,
            MonteCarloRandomizationConfig {
                seed_u64: 1,
                draw_count: 2_000,
            },
        )
        .unwrap();
        let second = estimate_monte_carlo_randomization_p_value(
            &trial,
            MonteCarloRandomizationConfig {
                seed_u64: 2,
                draw_count: 2_000,
            },
        )
        .unwrap();
        assert_ne!(
            first.sampled_assignments_sha256,
            second.sampled_assignments_sha256
        );
    }

    #[test]
    fn plus_one_correction_never_returns_zero() {
        let trial = trial();
        let mut no_effect = trial;
        for observation in &mut no_effect.observations {
            observation.observed_outcome_value = 0.0;
        }
        let result = estimate_monte_carlo_randomization_p_value(
            &no_effect,
            MonteCarloRandomizationConfig {
                seed_u64: 5,
                draw_count: 1_000,
            },
        )
        .unwrap();
        assert!(result.p_value_plus_one > 0.0);
        assert_eq!(result.p_value_plus_one, 1.0);
    }

    #[test]
    fn small_trial_monte_carlo_wilson_interval_contains_exact_tail_probability() {
        let trial = trial();
        let exact = estimate_blocked_randomized_maintenance_effect(&trial)
            .unwrap()
            .exact_randomization_p_value;
        let mc = estimate_monte_carlo_randomization_p_value(
            &trial,
            MonteCarloRandomizationConfig {
                seed_u64: 42,
                draw_count: 30_000,
            },
        )
        .unwrap();
        assert!(
            mc.tail_probability_wilson_95[0] <= exact
                && exact <= mc.tail_probability_wilson_95[1]
        );
        assert!((0.0..=1.0).contains(&mc.p_value_plus_one));
    }

    #[test]
    fn monte_carlo_sampler_supports_trials_above_the_exact_sixteen_unit_bound() {
        let trial = larger_trial();
        assert_eq!(trial.observations.len(), 17);
        let result = estimate_monte_carlo_randomization_p_value(
            &trial,
            MonteCarloRandomizationConfig {
                seed_u64: 99,
                draw_count: 1_000,
            },
        )
        .unwrap();
        assert_eq!(result.draw_count, 1_000);
        assert_eq!(result.allocation_constraint_violations, 0);
        assert!(!result.exact_enumeration_used);
    }

    #[test]
    fn invalid_draw_counts_fail_closed() {
        let trial = trial();
        for draw_count in [0, MIN_MONTE_CARLO_DRAWS - 1, MAX_MONTE_CARLO_DRAWS + 1] {
            assert_eq!(
                estimate_monte_carlo_randomization_p_value(
                    &trial,
                    MonteCarloRandomizationConfig {
                        seed_u64: 1,
                        draw_count,
                    }
                ),
                Err(MonteCarloRandomizationError::InvalidDrawCount(draw_count))
            );
        }
    }

    #[test]
    fn protocol_and_corpus_bind_deterministic_sampling_contract() {
        let protocol: serde_json::Value =
            serde_json::from_str(include_str!("../monte-carlo-randomization-v0.1.json")).unwrap();
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../monte-carlo-randomization-cases-v0.1.json"))
                .unwrap();
        assert_eq!(protocol["schema_version"], MONTE_CARLO_RANDOMIZATION_SCHEMA);
        assert_eq!(protocol["algorithm"]["id"], MONTE_CARLO_RANDOMIZATION_ALGORITHM);
        assert_eq!(protocol["algorithm"]["max_units"], MAX_MONTE_CARLO_UNITS);
        assert_eq!(corpus["cases"].as_array().unwrap().len(), 8);
    }
}
