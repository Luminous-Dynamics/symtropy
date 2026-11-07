// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Paired, common-random-number effect summaries for declared model outcomes.
//!
//! These are paired contrasts in a specified model experiment, not estimates
//! of real-world causal effects. Structural path evidence is checked separately.

use core::fmt;
use std::collections::BTreeMap;

pub const MIN_PAIRED_RUNS: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingDesign {
    CommonRandomNumbers,
    MatchedObservational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedResponse {
    Positive,
    Negative,
    Equivalent,
    Exploratory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectStatus {
    StructurallyDisconnected,
    MovementWithoutDeclaredPath,
    DirectionallyConsistentWithDeclaredPath,
    DirectionContradicted,
    EquivalentWithinMargin,
    MaterialEffectAgainstEquivalenceContract,
    DetectableMovement,
    NoDetectedEffect,
    Indeterminate,
    DescriptiveOnly,
}

impl EffectStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::StructurallyDisconnected => "structurally_disconnected",
            Self::MovementWithoutDeclaredPath => "movement_without_declared_path",
            Self::DirectionallyConsistentWithDeclaredPath => {
                "directionally_consistent_with_declared_path"
            }
            Self::DirectionContradicted => "direction_contradicted",
            Self::EquivalentWithinMargin => "equivalent_within_margin",
            Self::MaterialEffectAgainstEquivalenceContract => {
                "material_effect_against_equivalence_contract"
            }
            Self::DetectableMovement => "detectable_movement",
            Self::NoDetectedEffect => "no_detected_effect",
            Self::Indeterminate => "indeterminate",
            Self::DescriptiveOnly => "descriptive_only",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunObservation {
    pub seed: u64,
    pub values: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContrastDataset {
    pub design: PairingDesign,
    pub control: Vec<RunObservation>,
    pub treatment: Vec<RunObservation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContrastContract {
    pub outcome_variable_id: String,
    pub expected_response: ExpectedResponse,
    /// Predeclared absolute equivalence margin in the outcome's native unit.
    pub equivalence_margin: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairedEffectResult {
    pub outcome_variable_id: String,
    pub pairing_design: PairingDesign,
    pub pair_count: usize,
    pub has_declared_path: bool,
    pub confirmatory_eligible: bool,
    pub mean_control: f64,
    pub mean_treatment: f64,
    pub mean_paired_difference: f64,
    pub paired_difference_sd: f64,
    pub ci95_lower: f64,
    pub ci95_upper: f64,
    pub expected_response: ExpectedResponse,
    pub equivalence_margin: f64,
    pub status: EffectStatus,
    /// True only for a predeclared control under common random numbers.
    pub control_gate_passed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContrastError {
    EmptyOutcomeId,
    InvalidEquivalenceMargin(f64),
    TooFewPairs {
        actual: usize,
        minimum: usize,
    },
    DuplicateSeed {
        arm: &'static str,
        seed: u64,
    },
    SeedSetMismatch {
        seed: u64,
        arm: &'static str,
    },
    MissingOutcome {
        arm: &'static str,
        seed: u64,
        variable_id: String,
    },
    NonFiniteValue {
        arm: &'static str,
        seed: u64,
        variable_id: String,
        value: f64,
    },
}

impl fmt::Display for ContrastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyOutcomeId => write!(f, "outcome variable ID must not be empty"),
            Self::InvalidEquivalenceMargin(value) => write!(
                f,
                "equivalence margin must be finite and non-negative, got {value}"
            ),
            Self::TooFewPairs { actual, minimum } => {
                write!(f, "paired run count {actual} is below minimum {minimum}")
            }
            Self::DuplicateSeed { arm, seed } => {
                write!(f, "{arm} arm contains duplicate seed {seed}")
            }
            Self::SeedSetMismatch { seed, arm } => {
                write!(f, "seed {seed} is missing from {arm} arm")
            }
            Self::MissingOutcome {
                arm,
                seed,
                variable_id,
            } => write!(f, "{arm} seed {seed} is missing outcome {variable_id}"),
            Self::NonFiniteValue {
                arm,
                seed,
                variable_id,
                value,
            } => write!(
                f,
                "{arm} seed {seed} outcome {variable_id} is non-finite: {value}"
            ),
        }
    }
}

impl std::error::Error for ContrastError {}

pub fn evaluate_paired_contrast(
    dataset: &ContrastDataset,
    contract: &ContrastContract,
    has_declared_path: bool,
) -> Result<PairedEffectResult, ContrastError> {
    if contract.outcome_variable_id.trim().is_empty() {
        return Err(ContrastError::EmptyOutcomeId);
    }
    if !contract.equivalence_margin.is_finite() || contract.equivalence_margin < 0.0 {
        return Err(ContrastError::InvalidEquivalenceMargin(
            contract.equivalence_margin,
        ));
    }

    let control = index_arm("control", &dataset.control, &contract.outcome_variable_id)?;
    let treatment = index_arm(
        "treatment",
        &dataset.treatment,
        &contract.outcome_variable_id,
    )?;
    if control.len() < MIN_PAIRED_RUNS {
        return Err(ContrastError::TooFewPairs {
            actual: control.len(),
            minimum: MIN_PAIRED_RUNS,
        });
    }
    for seed in control.keys() {
        if !treatment.contains_key(seed) {
            return Err(ContrastError::SeedSetMismatch {
                seed: *seed,
                arm: "treatment",
            });
        }
    }
    for seed in treatment.keys() {
        if !control.contains_key(seed) {
            return Err(ContrastError::SeedSetMismatch {
                seed: *seed,
                arm: "control",
            });
        }
    }

    let mut differences = Vec::with_capacity(control.len());
    let mut control_values = Vec::with_capacity(control.len());
    let mut treatment_values = Vec::with_capacity(control.len());
    for (seed, control_value) in &control {
        let treatment_value = treatment[seed];
        control_values.push(*control_value);
        treatment_values.push(treatment_value);
        differences.push(treatment_value - control_value);
    }

    let pair_count = differences.len();
    let mean_control = mean(&control_values);
    let mean_treatment = mean(&treatment_values);
    let mean_paired_difference = mean(&differences);
    let paired_difference_sd = sample_sd(&differences, mean_paired_difference);
    let half_width =
        t_critical_95(pair_count - 1) * paired_difference_sd / (pair_count as f64).sqrt();
    let ci95_lower = mean_paired_difference - half_width;
    let ci95_upper = mean_paired_difference + half_width;

    let confirmatory_eligible = dataset.design == PairingDesign::CommonRandomNumbers;
    let status = if !confirmatory_eligible {
        EffectStatus::DescriptiveOnly
    } else if !has_declared_path {
        if ci95_lower > 0.0 || ci95_upper < 0.0 {
            EffectStatus::MovementWithoutDeclaredPath
        } else {
            EffectStatus::StructurallyDisconnected
        }
    } else {
        classify_effect(
            contract.expected_response,
            contract.equivalence_margin,
            ci95_lower,
            ci95_upper,
        )
    };
    let control_gate_passed = confirmatory_eligible
        && match contract.expected_response {
            ExpectedResponse::Positive => ci95_lower > 0.0,
            ExpectedResponse::Negative => ci95_upper < 0.0,
            ExpectedResponse::Equivalent => {
                ci95_lower >= -contract.equivalence_margin
                    && ci95_upper <= contract.equivalence_margin
            }
            ExpectedResponse::Exploratory => ci95_lower > 0.0 || ci95_upper < 0.0,
        }
        && has_declared_path;

    Ok(PairedEffectResult {
        outcome_variable_id: contract.outcome_variable_id.clone(),
        pairing_design: dataset.design,
        pair_count,
        has_declared_path,
        confirmatory_eligible,
        mean_control,
        mean_treatment,
        mean_paired_difference,
        paired_difference_sd,
        ci95_lower,
        ci95_upper,
        expected_response: contract.expected_response,
        equivalence_margin: contract.equivalence_margin,
        status,
        control_gate_passed,
    })
}

fn index_arm(
    arm: &'static str,
    observations: &[RunObservation],
    outcome_id: &str,
) -> Result<BTreeMap<u64, f64>, ContrastError> {
    let mut indexed = BTreeMap::new();
    for observation in observations {
        let value =
            observation
                .values
                .get(outcome_id)
                .ok_or_else(|| ContrastError::MissingOutcome {
                    arm,
                    seed: observation.seed,
                    variable_id: outcome_id.into(),
                })?;
        if !value.is_finite() {
            return Err(ContrastError::NonFiniteValue {
                arm,
                seed: observation.seed,
                variable_id: outcome_id.into(),
                value: *value,
            });
        }
        if indexed.insert(observation.seed, *value).is_some() {
            return Err(ContrastError::DuplicateSeed {
                arm,
                seed: observation.seed,
            });
        }
    }
    Ok(indexed)
}

fn classify_effect(
    expected: ExpectedResponse,
    margin: f64,
    lower: f64,
    upper: f64,
) -> EffectStatus {
    let excludes_zero = lower > 0.0 || upper < 0.0;
    let equivalent = lower >= -margin && upper <= margin;
    match expected {
        ExpectedResponse::Positive => {
            if lower > 0.0 {
                EffectStatus::DirectionallyConsistentWithDeclaredPath
            } else if upper < 0.0 {
                EffectStatus::DirectionContradicted
            } else if equivalent {
                EffectStatus::EquivalentWithinMargin
            } else {
                EffectStatus::Indeterminate
            }
        }
        ExpectedResponse::Negative => {
            if upper < 0.0 {
                EffectStatus::DirectionallyConsistentWithDeclaredPath
            } else if lower > 0.0 {
                EffectStatus::DirectionContradicted
            } else if equivalent {
                EffectStatus::EquivalentWithinMargin
            } else {
                EffectStatus::Indeterminate
            }
        }
        ExpectedResponse::Equivalent => {
            if equivalent {
                EffectStatus::EquivalentWithinMargin
            } else if excludes_zero {
                EffectStatus::MaterialEffectAgainstEquivalenceContract
            } else {
                EffectStatus::Indeterminate
            }
        }
        ExpectedResponse::Exploratory => {
            if excludes_zero {
                EffectStatus::DetectableMovement
            } else {
                EffectStatus::NoDetectedEffect
            }
        }
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn sample_sd(values: &[f64], average: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    (values
        .iter()
        .map(|value| (value - average).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64)
        .sqrt()
}

/// Conservative two-sided 95% Student-t critical values, rounded upward by df bucket.
fn t_critical_95(degrees_of_freedom: usize) -> f64 {
    match degrees_of_freedom {
        0..=29 => 2.05,
        30..=39 => 2.043,
        40..=59 => 2.022,
        60..=119 => 2.001,
        _ => 1.981,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dataset(difference: impl Fn(usize) -> f64, design: PairingDesign) -> ContrastDataset {
        let mut control = Vec::new();
        let mut treatment = Vec::new();
        for seed in 0..MIN_PAIRED_RUNS as u64 {
            let delta = difference(seed as usize);
            let base = seed as f64 * 0.01;
            control.push(RunObservation {
                seed,
                values: BTreeMap::from([("service".into(), base)]),
            });
            treatment.push(RunObservation {
                seed,
                values: BTreeMap::from([("service".into(), base + delta)]),
            });
        }
        ContrastDataset {
            design,
            control,
            treatment,
        }
    }

    fn contract(expected_response: ExpectedResponse, margin: f64) -> ContrastContract {
        ContrastContract {
            outcome_variable_id: "service".into(),
            expected_response,
            equivalence_margin: margin,
        }
    }

    #[test]
    fn positive_control_requires_interval_excluding_zero() {
        let input = dataset(
            |index| 1.0 + (index % 3) as f64 * 0.01,
            PairingDesign::CommonRandomNumbers,
        );
        let result =
            evaluate_paired_contrast(&input, &contract(ExpectedResponse::Positive, 0.05), true)
                .unwrap();
        assert_eq!(
            result.status,
            EffectStatus::DirectionallyConsistentWithDeclaredPath
        );
        assert!(result.control_gate_passed);
        assert_eq!(result.pair_count, 30);
    }

    #[test]
    fn negative_control_requires_entire_interval_inside_equivalence_margin() {
        let input = dataset(
            |index| if index % 2 == 0 { 0.002 } else { -0.002 },
            PairingDesign::CommonRandomNumbers,
        );
        let result =
            evaluate_paired_contrast(&input, &contract(ExpectedResponse::Equivalent, 0.01), true)
                .unwrap();
        assert_eq!(result.status, EffectStatus::EquivalentWithinMargin);
        assert!(result.control_gate_passed);
        assert!(result.ci95_lower >= -0.01);
        assert!(result.ci95_upper <= 0.01);
    }

    #[test]
    fn failing_to_reject_zero_is_not_an_equivalence_pass() {
        let input = dataset(
            |index| if index % 2 == 0 { 0.4 } else { -0.4 },
            PairingDesign::CommonRandomNumbers,
        );
        let result =
            evaluate_paired_contrast(&input, &contract(ExpectedResponse::Equivalent, 0.01), true)
                .unwrap();
        assert_eq!(result.status, EffectStatus::Indeterminate);
        assert!(!result.control_gate_passed);
    }

    #[test]
    fn disconnected_effect_is_not_credited_to_the_declared_path() {
        let input = dataset(|_| 0.8, PairingDesign::CommonRandomNumbers);
        let result =
            evaluate_paired_contrast(&input, &contract(ExpectedResponse::Positive, 0.05), false)
                .unwrap();
        assert_eq!(result.status, EffectStatus::MovementWithoutDeclaredPath);
        assert!(!result.control_gate_passed);
    }

    #[test]
    fn common_random_numbers_are_required_for_confirmatory_gate() {
        let input = dataset(|_| 1.0, PairingDesign::MatchedObservational);
        let result =
            evaluate_paired_contrast(&input, &contract(ExpectedResponse::Positive, 0.05), true)
                .unwrap();
        assert_eq!(result.status, EffectStatus::DescriptiveOnly);
        assert!(!result.confirmatory_eligible);
        assert!(!result.control_gate_passed);
    }

    #[test]
    fn wrong_direction_fails_the_positive_control() {
        let input = dataset(|_| -1.0, PairingDesign::CommonRandomNumbers);
        let result =
            evaluate_paired_contrast(&input, &contract(ExpectedResponse::Positive, 0.05), true)
                .unwrap();
        assert_eq!(result.status, EffectStatus::DirectionContradicted);
        assert!(!result.control_gate_passed);
    }

    #[test]
    fn missing_pairs_and_duplicate_seeds_fail_closed() {
        let mut too_few = dataset(|_| 1.0, PairingDesign::CommonRandomNumbers);
        too_few.treatment.pop();
        assert!(matches!(
            evaluate_paired_contrast(&too_few, &contract(ExpectedResponse::Positive, 0.05), true),
            Err(ContrastError::SeedSetMismatch { .. })
        ));

        let mut duplicate = dataset(|_| 1.0, PairingDesign::CommonRandomNumbers);
        duplicate.treatment.push(duplicate.treatment[0].clone());
        assert!(matches!(
            evaluate_paired_contrast(
                &duplicate,
                &contract(ExpectedResponse::Positive, 0.05),
                true
            ),
            Err(ContrastError::DuplicateSeed { .. })
        ));
    }
}
