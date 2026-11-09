// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Exact interference twin-world control for civilizational policy experiments.
//!
//! The observational design only records neighbor outcomes while the focal
//! neighbor is unexposed. A no-spillover world and a positive-spillover world
//! are therefore observationally equivalent under that restricted design.
//! A neighbor-exposure probe distinguishes them; a full-rollout experiment
//! does not, because direct treatment saturates the binary outcome in both
//! candidate worlds. This is a protocol fixture, not a general network causal
//! inference engine.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterferenceWorld {
    NoSpillover,
    PositiveSpillover,
}

impl InterferenceWorld {
    pub const ALL: [Self; 2] = [Self::NoSpillover, Self::PositiveSpillover];

    pub const fn name(self) -> &'static str {
        match self {
            Self::NoSpillover => "no_spillover",
            Self::PositiveSpillover => "positive_spillover",
        }
    }

    pub const fn spillover_effect(self) -> f64 {
        match self {
            Self::NoSpillover => 0.0,
            Self::PositiveSpillover => 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterferenceExperiment {
    BaselineNoExposure,
    TreatNeighborOnly,
    FullRollout,
}

impl InterferenceExperiment {
    pub const ALL: [Self; 3] = [
        Self::BaselineNoExposure,
        Self::TreatNeighborOnly,
        Self::FullRollout,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::BaselineNoExposure => "baseline_no_exposure",
            Self::TreatNeighborOnly => "treat_neighbor_only",
            Self::FullRollout => "full_rollout",
        }
    }

    const fn assignments(self) -> (bool, bool) {
        match self {
            Self::BaselineNoExposure => (false, false),
            Self::TreatNeighborOnly => (true, false),
            Self::FullRollout => (true, true),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpilloverEffectBounds {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterferenceAssessmentStatus {
    DiscriminatingExperimentAvailable,
    NoPermittedDiscriminatingExperiment,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpilloverExperimentOption {
    pub experiment: InterferenceExperiment,
    pub cost_units: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpilloverExperimentChoice {
    pub experiment: InterferenceExperiment,
    pub cost_units: f64,
    pub total_variation: f64,
    pub expected_information_gain_bits: f64,
    pub information_gain_bits_per_cost: f64,
    pub no_spillover_prediction: bool,
    pub positive_spillover_prediction: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InterferenceIdentifiabilityAssessment {
    pub observationally_equivalent: bool,
    /// The candidate effect set is {0, 1}; this interval is its convex hull.
    pub spillover_effect_bounds: SpilloverEffectBounds,
    pub status: InterferenceAssessmentStatus,
    pub selected_experiment: Option<SpilloverExperimentChoice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterferenceError {
    InvalidBudget,
    InvalidExperimentCost,
    InvalidPrior,
}

impl fmt::Display for InterferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBudget => write!(f, "experiment budget must be finite and nonnegative"),
            Self::InvalidExperimentCost => {
                write!(f, "experiment cost must be finite and nonnegative")
            }
            Self::InvalidPrior => {
                write!(f, "no-spillover prior must be finite and within [0, 1]")
            }
        }
    }
}

impl std::error::Error for InterferenceError {}

/// The observational reference samples the focal neighbor only while the
/// potentially treated source neighbor is unexposed. The returned values are
/// ordered by focal-neighbor treatment {0, 1}.
pub fn observational_neighbor_outcomes(world: InterferenceWorld) -> [bool; 2] {
    [false, true].map(|focal_treated| {
        neighbor_outcome(
            world,
            false, // source neighbor remains untreated in the observational sample
            focal_treated,
        )
    })
}

/// Binary potential outcome for the focal neighbor under an intervention.
/// The positive-spillover world is an OR exposure mapping:
/// Y_B = T_B OR T_A. The no-spillover world is Y_B = T_B.
pub fn interventional_neighbor_outcome(
    world: InterferenceWorld,
    experiment: InterferenceExperiment,
) -> bool {
    let (source_neighbor_treated, focal_neighbor_treated) = experiment.assignments();
    neighbor_outcome(world, source_neighbor_treated, focal_neighbor_treated)
}

fn neighbor_outcome(
    world: InterferenceWorld,
    source_neighbor_treated: bool,
    focal_neighbor_treated: bool,
) -> bool {
    focal_neighbor_treated
        || (world == InterferenceWorld::PositiveSpillover && source_neighbor_treated)
}

/// Exact mutual information between candidate world identity and the binary
/// focal-neighbor outcome under an explicit prior.
pub fn expected_information_gain_bits(
    experiment: InterferenceExperiment,
    no_spillover_prior: f64,
) -> Result<f64, InterferenceError> {
    if !no_spillover_prior.is_finite() || !(0.0..=1.0).contains(&no_spillover_prior) {
        return Err(InterferenceError::InvalidPrior);
    }

    let prior = [no_spillover_prior, 1.0 - no_spillover_prior];
    let outcomes = InterferenceWorld::ALL.map(|world| {
        f64::from(interventional_neighbor_outcome(world, experiment))
    });
    let marginal_one = prior[0] * outcomes[0] + prior[1] * outcomes[1];

    let mut information_gain = 0.0;
    for (world_prior, conditional_one) in prior.into_iter().zip(outcomes) {
        for outcome_is_one in [false, true] {
            let conditional_probability = if outcome_is_one {
                conditional_one
            } else {
                1.0 - conditional_one
            };
            let marginal_probability = if outcome_is_one {
                marginal_one
            } else {
                1.0 - marginal_one
            };
            let joint_probability = world_prior * conditional_probability;
            if joint_probability > 0.0 {
                let independent_probability = world_prior * marginal_probability;
                information_gain +=
                    joint_probability * (joint_probability / independent_probability).log2();
            }
        }
    }

    Ok(information_gain.max(0.0))
}

/// Select the permitted intervention with greatest expected information gain
/// per declared cost. Non-discriminating experiments are never selected.
pub fn select_spillover_experiment(
    allowed_experiments: &[SpilloverExperimentOption],
    budget_units: f64,
    no_spillover_prior: f64,
) -> Result<Option<SpilloverExperimentChoice>, InterferenceError> {
    if !budget_units.is_finite() || budget_units < 0.0 {
        return Err(InterferenceError::InvalidBudget);
    }
    if !no_spillover_prior.is_finite() || !(0.0..=1.0).contains(&no_spillover_prior) {
        return Err(InterferenceError::InvalidPrior);
    }

    let mut best: Option<SpilloverExperimentChoice> = None;
    for option in allowed_experiments {
        if !option.cost_units.is_finite() || option.cost_units < 0.0 {
            return Err(InterferenceError::InvalidExperimentCost);
        }
        if option.cost_units > budget_units {
            continue;
        }

        let no_spillover =
            interventional_neighbor_outcome(InterferenceWorld::NoSpillover, option.experiment);
        let positive_spillover =
            interventional_neighbor_outcome(InterferenceWorld::PositiveSpillover, option.experiment);
        let total_variation = f64::from(no_spillover != positive_spillover);
        if total_variation == 0.0 {
            continue;
        }

        let information_gain =
            expected_information_gain_bits(option.experiment, no_spillover_prior)?;
        let information_gain_per_cost = if option.cost_units == 0.0 {
            f64::INFINITY
        } else {
            information_gain / option.cost_units
        };
        let candidate = SpilloverExperimentChoice {
            experiment: option.experiment,
            cost_units: option.cost_units,
            total_variation,
            expected_information_gain_bits: information_gain,
            information_gain_bits_per_cost: information_gain_per_cost,
            no_spillover_prediction: no_spillover,
            positive_spillover_prediction: positive_spillover,
        };

        if best.is_none_or(|current| {
            candidate.information_gain_bits_per_cost > current.information_gain_bits_per_cost
        }) {
            best = Some(candidate);
        }
    }

    Ok(best)
}

pub fn assess_interference_identifiability(
    allowed_experiments: &[SpilloverExperimentOption],
    budget_units: f64,
    no_spillover_prior: f64,
) -> Result<InterferenceIdentifiabilityAssessment, InterferenceError> {
    let no_spillover_observations =
        observational_neighbor_outcomes(InterferenceWorld::NoSpillover);
    let positive_spillover_observations =
        observational_neighbor_outcomes(InterferenceWorld::PositiveSpillover);
    let observationally_equivalent =
        no_spillover_observations == positive_spillover_observations;

    let selected_experiment = select_spillover_experiment(
        allowed_experiments,
        budget_units,
        no_spillover_prior,
    )?;
    let status = if selected_experiment.is_some() {
        InterferenceAssessmentStatus::DiscriminatingExperimentAvailable
    } else {
        InterferenceAssessmentStatus::NoPermittedDiscriminatingExperiment
    };

    Ok(InterferenceIdentifiabilityAssessment {
        observationally_equivalent,
        spillover_effect_bounds: SpilloverEffectBounds {
            lower: InterferenceWorld::ALL
                .into_iter()
                .map(InterferenceWorld::spillover_effect)
                .fold(f64::INFINITY, f64::min),
            upper: InterferenceWorld::ALL
                .into_iter()
                .map(InterferenceWorld::spillover_effect)
                .fold(f64::NEG_INFINITY, f64::max),
        },
        status,
        selected_experiment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_observations_are_identical_under_both_interference_worlds() {
        assert_eq!(
            observational_neighbor_outcomes(InterferenceWorld::NoSpillover),
            [false, true]
        );
        assert_eq!(
            observational_neighbor_outcomes(InterferenceWorld::PositiveSpillover),
            [false, true]
        );

        let assessment =
            assess_interference_identifiability(&[], 0.0, 0.5).expect("valid assessment");
        assert!(assessment.observationally_equivalent);
        assert_eq!(assessment.spillover_effect_bounds.lower, 0.0);
        assert_eq!(assessment.spillover_effect_bounds.upper, 1.0);
        assert_eq!(
            assessment.status,
            InterferenceAssessmentStatus::NoPermittedDiscriminatingExperiment
        );
        assert_eq!(assessment.selected_experiment, None);
    }

    #[test]
    fn isolated_and_full_rollout_controls_do_not_identify_spillover() {
        for experiment in [
            InterferenceExperiment::BaselineNoExposure,
            InterferenceExperiment::FullRollout,
        ] {
            assert_eq!(
                interventional_neighbor_outcome(InterferenceWorld::NoSpillover, experiment),
                interventional_neighbor_outcome(
                    InterferenceWorld::PositiveSpillover,
                    experiment
                )
            );
            assert_eq!(expected_information_gain_bits(experiment, 0.5), Ok(0.0));
        }
    }

    #[test]
    fn neighbor_only_probe_identifies_spillover_exactly() {
        assert!(!interventional_neighbor_outcome(
            InterferenceWorld::NoSpillover,
            InterferenceExperiment::TreatNeighborOnly
        ));
        assert!(interventional_neighbor_outcome(
            InterferenceWorld::PositiveSpillover,
            InterferenceExperiment::TreatNeighborOnly
        ));
        assert_eq!(
            expected_information_gain_bits(InterferenceExperiment::TreatNeighborOnly, 0.5),
            Ok(1.0)
        );
    }

    #[test]
    fn selector_chooses_a_discriminating_probe_not_a_rollout() {
        let options = [
            SpilloverExperimentOption {
                experiment: InterferenceExperiment::FullRollout,
                cost_units: 0.1,
            },
            SpilloverExperimentOption {
                experiment: InterferenceExperiment::TreatNeighborOnly,
                cost_units: 1.0,
            },
            SpilloverExperimentOption {
                experiment: InterferenceExperiment::BaselineNoExposure,
                cost_units: 0.0,
            },
        ];
        let selected = select_spillover_experiment(&options, 1.0, 0.5)
            .unwrap()
            .unwrap();
        assert_eq!(selected.experiment, InterferenceExperiment::TreatNeighborOnly);
        assert_eq!(selected.total_variation, 1.0);
        assert_eq!(selected.expected_information_gain_bits, 1.0);
        assert_eq!(selected.information_gain_bits_per_cost, 1.0);
        assert!(!selected.no_spillover_prediction);
        assert!(selected.positive_spillover_prediction);
    }

    #[test]
    fn selector_abstains_when_only_non_discriminating_experiments_are_permitted() {
        let options = [
            SpilloverExperimentOption {
                experiment: InterferenceExperiment::BaselineNoExposure,
                cost_units: 0.0,
            },
            SpilloverExperimentOption {
                experiment: InterferenceExperiment::FullRollout,
                cost_units: 0.1,
            },
        ];
        let assessment = assess_interference_identifiability(&options, 1.0, 0.5).unwrap();
        assert!(assessment.observationally_equivalent);
        assert_eq!(
            assessment.status,
            InterferenceAssessmentStatus::NoPermittedDiscriminatingExperiment
        );
        assert_eq!(assessment.selected_experiment, None);
    }

    #[test]
    fn selector_respects_cost_budget_and_rejects_invalid_inputs() {
        let probe = [SpilloverExperimentOption {
            experiment: InterferenceExperiment::TreatNeighborOnly,
            cost_units: 1.0,
        }];
        assert_eq!(
            select_spillover_experiment(&probe, 0.5, 0.5).unwrap(),
            None
        );
        assert_eq!(
            expected_information_gain_bits(InterferenceExperiment::TreatNeighborOnly, f64::NAN),
            Err(InterferenceError::InvalidPrior)
        );
        assert_eq!(
            select_spillover_experiment(&probe, f64::NAN, 0.5),
            Err(InterferenceError::InvalidBudget)
        );
        let invalid_cost = [SpilloverExperimentOption {
            experiment: InterferenceExperiment::TreatNeighborOnly,
            cost_units: -1.0,
        }];
        assert_eq!(
            select_spillover_experiment(&invalid_cost, 2.0, 0.5),
            Err(InterferenceError::InvalidExperimentCost)
        );
    }
}
