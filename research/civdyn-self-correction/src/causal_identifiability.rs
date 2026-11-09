// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact observational-equivalence control for causal identifiability.
//!
//! This deliberately small binary SCM pair demonstrates that identical
//! observational distributions need not imply identical intervention effects.
//! It is a protocol fixture, not a general causal-discovery engine.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwinWorld {
    CommonCause,
    DirectTreatmentEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intervention {
    SetTreatmentZero,
    SetTreatmentOne,
}

impl Intervention {
    pub const ALL: [Self; 2] = [Self::SetTreatmentZero, Self::SetTreatmentOne];

    pub const fn name(self) -> &'static str {
        match self {
            Self::SetTreatmentZero => "do_treatment_0",
            Self::SetTreatmentOne => "do_treatment_1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryJointDistribution {
    /// Cell order is (treatment=0,outcome=0), (0,1), (1,0), (1,1).
    cell_counts: [u8; 4],
    denominator: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryOutcomeDistribution {
    outcome_one_count: u8,
    denominator: u8,
}

impl BinaryJointDistribution {
    pub fn new(cell_counts: [u8; 4], denominator: u8) -> Result<Self, AssessmentError> {
        let total: u16 = cell_counts.iter().map(|count| u16::from(*count)).sum();
        if denominator == 0 || total != u16::from(denominator) {
            return Err(AssessmentError::InvalidDistribution);
        }
        Ok(Self {
            cell_counts,
            denominator,
        })
    }

    pub const fn cell_counts(self) -> [u8; 4] {
        self.cell_counts
    }

    pub const fn denominator(self) -> u8 {
        self.denominator
    }
}

impl BinaryOutcomeDistribution {
    pub fn new(outcome_one_count: u8, denominator: u8) -> Result<Self, AssessmentError> {
        if denominator == 0 || outcome_one_count > denominator {
            return Err(AssessmentError::InvalidDistribution);
        }
        Ok(Self {
            outcome_one_count,
            denominator,
        })
    }

    pub const fn outcome_one_count(self) -> u8 {
        self.outcome_one_count
    }

    pub const fn denominator(self) -> u8 {
        self.denominator
    }

    pub fn probability_one(self) -> f64 {
        f64::from(self.outcome_one_count) / f64::from(self.denominator)
    }

    pub fn total_variation(self, other: Self) -> f64 {
        (self.probability_one() - other.probability_one()).abs()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectBounds {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentStatus {
    DiscriminatingExperimentAvailable,
    NoPermittedDiscriminatingExperiment,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExperimentOption {
    pub intervention: Intervention,
    pub cost_units: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExperimentChoice {
    pub intervention: Intervention,
    pub cost_units: f64,
    pub total_variation: f64,
    pub common_cause_prediction: BinaryOutcomeDistribution,
    pub direct_effect_prediction: BinaryOutcomeDistribution,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IdentifiabilityAssessment {
    pub observationally_equivalent: bool,
    /// The candidate effect set is {0, 1}; this interval is its convex hull.
    pub effect_bounds: EffectBounds,
    pub status: AssessmentStatus,
    pub selected_experiment: Option<ExperimentChoice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentError {
    InvalidBudget,
    InvalidExperimentCost,
    InvalidDistribution,
    InvalidPrior,
    NoFeasiblePolicy,
}

impl fmt::Display for AssessmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBudget => write!(f, "experiment budget must be finite and nonnegative"),
            Self::InvalidExperimentCost => {
                write!(f, "experiment cost must be finite and nonnegative")
            }
            Self::InvalidDistribution => {
                write!(
                    f,
                    "binary probability counts must have a positive denominator and valid mass"
                )
            }
            Self::InvalidPrior => {
                write!(f, "candidate-world prior must be finite and within [0, 1]")
            }
            Self::NoFeasiblePolicy => {
                write!(f, "no policy is feasible under the declared budget")
            }
        }
    }
}

impl std::error::Error for AssessmentError {}

/// Exact observational joint distribution for the two candidate SCMs.
///
/// CommonCause: U ~ Bernoulli(1/2), treatment = U, outcome = U.
/// DirectTreatmentEffect: treatment ~ Bernoulli(1/2), outcome = treatment.
pub fn observational_distribution(world: TwinWorld) -> BinaryJointDistribution {
    match world {
        TwinWorld::CommonCause | TwinWorld::DirectTreatmentEffect => BinaryJointDistribution {
            cell_counts: [1, 0, 0, 1],
            denominator: 2,
        },
    }
}

/// Exact outcome distribution after setting treatment to a fixed value.
pub fn interventional_distribution(
    world: TwinWorld,
    intervention: Intervention,
) -> BinaryOutcomeDistribution {
    match (world, intervention) {
        (TwinWorld::CommonCause, _) => BinaryOutcomeDistribution {
            outcome_one_count: 1,
            denominator: 2,
        },
        (TwinWorld::DirectTreatmentEffect, Intervention::SetTreatmentZero) => {
            BinaryOutcomeDistribution {
                outcome_one_count: 0,
                denominator: 1,
            }
        }
        (TwinWorld::DirectTreatmentEffect, Intervention::SetTreatmentOne) => {
            BinaryOutcomeDistribution {
                outcome_one_count: 1,
                denominator: 1,
            }
        }
    }
}

pub fn causal_effect(world: TwinWorld) -> f64 {
    match world {
        TwinWorld::CommonCause => 0.0,
        TwinWorld::DirectTreatmentEffect => 1.0,
    }
}

/// Select the permitted experiment with greatest exact TV separation per cost.
///
/// The models in this reference remain hypotheses, not observations of the
/// hidden truth. Selecting an experiment does not itself identify the world.
pub fn assess_identifiability(
    allowed_experiments: &[ExperimentOption],
    budget_units: f64,
) -> Result<IdentifiabilityAssessment, AssessmentError> {
    if !budget_units.is_finite() || budget_units < 0.0 {
        return Err(AssessmentError::InvalidBudget);
    }

    let common_observational = observational_distribution(TwinWorld::CommonCause);
    let direct_observational = observational_distribution(TwinWorld::DirectTreatmentEffect);
    let observationally_equivalent = common_observational == direct_observational;

    let mut best: Option<(f64, ExperimentChoice)> = None;
    for option in allowed_experiments {
        if !option.cost_units.is_finite() || option.cost_units < 0.0 {
            return Err(AssessmentError::InvalidExperimentCost);
        }
        if option.cost_units > budget_units {
            continue;
        }

        let common_prediction =
            interventional_distribution(TwinWorld::CommonCause, option.intervention);
        let direct_prediction =
            interventional_distribution(TwinWorld::DirectTreatmentEffect, option.intervention);
        let total_variation = common_prediction.total_variation(direct_prediction);
        if total_variation <= 0.0 {
            continue;
        }

        let efficiency = if option.cost_units == 0.0 {
            f64::INFINITY
        } else {
            total_variation / option.cost_units
        };
        let choice = ExperimentChoice {
            intervention: option.intervention,
            cost_units: option.cost_units,
            total_variation,
            common_cause_prediction: common_prediction,
            direct_effect_prediction: direct_prediction,
        };

        if best.is_none_or(|(best_efficiency, _)| efficiency > best_efficiency) {
            best = Some((efficiency, choice));
        }
    }

    let selected_experiment = best.map(|(_, choice)| choice);
    let status = if selected_experiment.is_some() {
        AssessmentStatus::DiscriminatingExperimentAvailable
    } else {
        AssessmentStatus::NoPermittedDiscriminatingExperiment
    };

    Ok(IdentifiabilityAssessment {
        observationally_equivalent,
        effect_bounds: EffectBounds {
            lower: causal_effect(TwinWorld::CommonCause)
                .min(causal_effect(TwinWorld::DirectTreatmentEffect)),
            upper: causal_effect(TwinWorld::CommonCause)
                .max(causal_effect(TwinWorld::DirectTreatmentEffect)),
        },
        status,
        selected_experiment,
    })
}

/// Expected-information-gain choice for a single permitted intervention.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InformationGainChoice {
    pub intervention: Intervention,
    pub cost_units: f64,
    pub expected_information_gain_bits: f64,
    pub information_gain_bits_per_cost: f64,
}

/// Compute mutual information between the candidate SCM identity and the
/// binary outcome under one intervention.
///
/// The prior is explicit because information gain depends on it. This function
/// assumes the two-model candidate set in this exact binary fixture.
pub fn expected_information_gain_bits(
    intervention: Intervention,
    common_cause_prior: f64,
) -> Result<f64, AssessmentError> {
    if !common_cause_prior.is_finite() || !(0.0..=1.0).contains(&common_cause_prior) {
        return Err(AssessmentError::InvalidPrior);
    }

    let prior = [common_cause_prior, 1.0 - common_cause_prior];
    let common_prediction =
        interventional_distribution(TwinWorld::CommonCause, intervention).probability_one();
    let direct_prediction =
        interventional_distribution(TwinWorld::DirectTreatmentEffect, intervention)
            .probability_one();
    let conditional_one = [common_prediction, direct_prediction];
    let marginal_one = prior[0] * conditional_one[0] + prior[1] * conditional_one[1];

    let mut information_gain = 0.0;
    for (world_prior, conditional_probability_one) in prior.into_iter().zip(conditional_one) {
        for outcome_is_one in [false, true] {
            let conditional_probability = if outcome_is_one {
                conditional_probability_one
            } else {
                1.0 - conditional_probability_one
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
/// per cost under an explicit prior over the two candidate SCMs.
pub fn select_expected_information_gain_experiment(
    allowed_experiments: &[ExperimentOption],
    budget_units: f64,
    common_cause_prior: f64,
) -> Result<Option<InformationGainChoice>, AssessmentError> {
    if !budget_units.is_finite() || budget_units < 0.0 {
        return Err(AssessmentError::InvalidBudget);
    }
    if !common_cause_prior.is_finite() || !(0.0..=1.0).contains(&common_cause_prior) {
        return Err(AssessmentError::InvalidPrior);
    }

    let mut best: Option<InformationGainChoice> = None;
    for option in allowed_experiments {
        if !option.cost_units.is_finite() || option.cost_units < 0.0 {
            return Err(AssessmentError::InvalidExperimentCost);
        }
        if option.cost_units > budget_units {
            continue;
        }

        let information_gain =
            expected_information_gain_bits(option.intervention, common_cause_prior)?;
        if information_gain <= 0.0 {
            continue;
        }
        let information_gain_per_cost = if option.cost_units == 0.0 {
            f64::INFINITY
        } else {
            information_gain / option.cost_units
        };
        let candidate = InformationGainChoice {
            intervention: option.intervention,
            cost_units: option.cost_units,
            expected_information_gain_bits: information_gain,
            information_gain_bits_per_cost: information_gain_per_cost,
        };

        if best.is_none_or(|current| {
            candidate.information_gain_bits_per_cost > current.information_gain_bits_per_cost
        }) {
            best = Some(candidate);
        }
    }

    Ok(best)
}

/// Policy evaluation under the same unresolved candidate-model set.
///
/// The synthetic utility is expected outcome success probability minus the
/// declared cost in normalized reward units. This is an explicit toy objective,
/// not a universal civilizational welfare function.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RobustPolicyChoice {
    pub intervention: Intervention,
    pub cost_units: f64,
    pub common_cause_utility: f64,
    pub direct_effect_utility: f64,
    pub worst_case_utility: f64,
    pub maximum_regret: f64,
    /// True if this action is optimal (ties permitted) in every candidate SCM.
    pub optimal_in_every_candidate_world: bool,
}

/// Select the minimax-regret policy across the unresolved twin SCMs.
///
/// This function can identify a robust action even when the causal effect
/// remains set-identified rather than point-identified. A feasible action is
/// not automatically safe in a real deployment; the utility and action set
/// are declared synthetic benchmark inputs.
pub fn select_minimax_regret_policy(
    policies: &[ExperimentOption],
    budget_units: f64,
) -> Result<RobustPolicyChoice, AssessmentError> {
    if !budget_units.is_finite() || budget_units < 0.0 {
        return Err(AssessmentError::InvalidBudget);
    }

    let mut feasible = Vec::new();
    for policy in policies {
        if !policy.cost_units.is_finite() || policy.cost_units < 0.0 {
            return Err(AssessmentError::InvalidExperimentCost);
        }
        if policy.cost_units <= budget_units {
            let common = interventional_distribution(TwinWorld::CommonCause, policy.intervention)
                .probability_one()
                - policy.cost_units;
            let direct =
                interventional_distribution(TwinWorld::DirectTreatmentEffect, policy.intervention)
                    .probability_one()
                    - policy.cost_units;
            feasible.push((policy.intervention, policy.cost_units, common, direct));
        }
    }

    if feasible.is_empty() {
        return Err(AssessmentError::NoFeasiblePolicy);
    }

    let best_common = feasible
        .iter()
        .map(|(_, _, utility, _)| *utility)
        .fold(f64::NEG_INFINITY, f64::max);
    let best_direct = feasible
        .iter()
        .map(|(_, _, _, utility)| *utility)
        .fold(f64::NEG_INFINITY, f64::max);

    let mut selected: Option<RobustPolicyChoice> = None;
    for (intervention, cost_units, common, direct) in feasible {
        let common_regret = (best_common - common).max(0.0);
        let direct_regret = (best_direct - direct).max(0.0);
        let maximum_regret = common_regret.max(direct_regret);
        let worst_case_utility = common.min(direct);
        let optimal_in_every_candidate_world = common_regret <= 1e-12 && direct_regret <= 1e-12;

        let candidate = RobustPolicyChoice {
            intervention,
            cost_units,
            common_cause_utility: common,
            direct_effect_utility: direct,
            worst_case_utility,
            maximum_regret,
            optimal_in_every_candidate_world,
        };

        let replace = selected.is_none_or(|current| {
            candidate.maximum_regret < current.maximum_regret - 1e-12
                || ((candidate.maximum_regret - current.maximum_regret).abs() <= 1e-12
                    && candidate.worst_case_utility > current.worst_case_utility + 1e-12)
        });
        if replace {
            selected = Some(candidate);
        }
    }

    selected.ok_or(AssessmentError::NoFeasiblePolicy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_distribution_constructors_reject_invalid_probability_mass() {
        assert_eq!(
            BinaryOutcomeDistribution::new(1, 0),
            Err(AssessmentError::InvalidDistribution)
        );
        assert_eq!(
            BinaryOutcomeDistribution::new(2, 1),
            Err(AssessmentError::InvalidDistribution)
        );
        assert_eq!(
            BinaryJointDistribution::new([1, 0, 0, 0], 2),
            Err(AssessmentError::InvalidDistribution)
        );
        assert_eq!(
            BinaryJointDistribution::new([1, 0, 0, 1], 2)
                .unwrap()
                .cell_counts(),
            [1, 0, 0, 1]
        );
    }

    #[test]
    fn observationally_equivalent_twins_have_different_causal_effects() {
        let common = observational_distribution(TwinWorld::CommonCause);
        let direct = observational_distribution(TwinWorld::DirectTreatmentEffect);

        assert_eq!(common, direct);
        assert_eq!(common.cell_counts, [1, 0, 0, 1]);
        assert_eq!(common.denominator, 2);
        assert_eq!(causal_effect(TwinWorld::CommonCause), 0.0);
        assert_eq!(causal_effect(TwinWorld::DirectTreatmentEffect), 1.0);
    }

    #[test]
    fn intervention_discriminates_the_observationally_equivalent_worlds() {
        for intervention in Intervention::ALL {
            let common = interventional_distribution(TwinWorld::CommonCause, intervention);
            let direct =
                interventional_distribution(TwinWorld::DirectTreatmentEffect, intervention);
            assert_eq!(common.total_variation(direct), 0.5);
        }
    }

    #[test]
    fn impossible_identification_returns_the_effect_set_and_abstains() {
        let assessment = assess_identifiability(&[], 10.0).unwrap();

        assert!(assessment.observationally_equivalent);
        assert_eq!(assessment.effect_bounds.lower, 0.0);
        assert_eq!(assessment.effect_bounds.upper, 1.0);
        assert_eq!(
            assessment.status,
            AssessmentStatus::NoPermittedDiscriminatingExperiment
        );
        assert_eq!(assessment.selected_experiment, None);
    }

    #[test]
    fn over_budget_experiments_are_not_selected() {
        let allowed = [ExperimentOption {
            intervention: Intervention::SetTreatmentOne,
            cost_units: 2.0,
        }];
        let assessment = assess_identifiability(&allowed, 1.0).unwrap();

        assert_eq!(
            assessment.status,
            AssessmentStatus::NoPermittedDiscriminatingExperiment
        );
        assert_eq!(assessment.selected_experiment, None);
    }

    #[test]
    fn experiment_selection_maximizes_discrimination_per_cost() {
        let allowed = [
            ExperimentOption {
                intervention: Intervention::SetTreatmentZero,
                cost_units: 2.0,
            },
            ExperimentOption {
                intervention: Intervention::SetTreatmentOne,
                cost_units: 1.0,
            },
        ];
        let assessment = assess_identifiability(&allowed, 2.0).unwrap();
        let selected = assessment.selected_experiment.unwrap();

        assert_eq!(selected.intervention, Intervention::SetTreatmentOne);
        assert_eq!(selected.cost_units, 1.0);
        assert_eq!(selected.total_variation, 0.5);
        assert_eq!(
            assessment.status,
            AssessmentStatus::DiscriminatingExperimentAvailable
        );
    }

    #[test]
    fn intervention_information_gain_is_exact_and_prior_sensitive() {
        let zero = expected_information_gain_bits(Intervention::SetTreatmentZero, 0.5).unwrap();
        let one = expected_information_gain_bits(Intervention::SetTreatmentOne, 0.5).unwrap();

        assert!((zero - 0.31127812445913283).abs() < 1e-12);
        assert!((one - 0.31127812445913283).abs() < 1e-12);
        assert_eq!(
            expected_information_gain_bits(Intervention::SetTreatmentZero, 0.0).unwrap(),
            0.0
        );
        assert_eq!(
            expected_information_gain_bits(Intervention::SetTreatmentOne, 1.0).unwrap(),
            0.0
        );
        assert_eq!(
            expected_information_gain_bits(Intervention::SetTreatmentOne, f64::NAN),
            Err(AssessmentError::InvalidPrior)
        );
    }

    #[test]
    fn information_gain_selector_uses_declared_prior_and_native_cost() {
        let experiments = [
            ExperimentOption {
                intervention: Intervention::SetTreatmentZero,
                cost_units: 2.0,
            },
            ExperimentOption {
                intervention: Intervention::SetTreatmentOne,
                cost_units: 1.0,
            },
        ];
        let selected = select_expected_information_gain_experiment(&experiments, 2.0, 0.5)
            .unwrap()
            .unwrap();

        assert_eq!(selected.intervention, Intervention::SetTreatmentOne);
        assert!((selected.expected_information_gain_bits - 0.31127812445913283).abs() < 1e-12);
        assert!((selected.information_gain_bits_per_cost - 0.31127812445913283).abs() < 1e-12);
        assert_eq!(
            select_expected_information_gain_experiment(&experiments, 2.0, 0.0).unwrap(),
            None
        );
    }

    #[test]
    fn policy_can_be_identified_as_robustly_optimal_without_point_identifying_causality() {
        let policies = [
            ExperimentOption {
                intervention: Intervention::SetTreatmentZero,
                cost_units: 0.0,
            },
            ExperimentOption {
                intervention: Intervention::SetTreatmentOne,
                cost_units: 0.0,
            },
        ];

        let causal = assess_identifiability(&policies, 0.0).unwrap();
        let policy = select_minimax_regret_policy(&policies, 0.0).unwrap();

        assert_eq!(causal.effect_bounds.lower, 0.0);
        assert_eq!(causal.effect_bounds.upper, 1.0);
        assert!(causal.observationally_equivalent);
        assert_eq!(policy.intervention, Intervention::SetTreatmentOne);
        assert_eq!(policy.common_cause_utility, 0.5);
        assert_eq!(policy.direct_effect_utility, 1.0);
        assert_eq!(policy.maximum_regret, 0.0);
        assert!(policy.optimal_in_every_candidate_world);
    }

    #[test]
    fn minimax_regret_reports_cost_sensitive_policy_tradeoffs() {
        let policies = [
            ExperimentOption {
                intervention: Intervention::SetTreatmentZero,
                cost_units: 0.0,
            },
            ExperimentOption {
                intervention: Intervention::SetTreatmentOne,
                cost_units: 0.25,
            },
        ];

        let policy = select_minimax_regret_policy(&policies, 0.25).unwrap();

        assert_eq!(policy.intervention, Intervention::SetTreatmentOne);
        assert_eq!(policy.common_cause_utility, 0.25);
        assert_eq!(policy.direct_effect_utility, 0.75);
        assert_eq!(policy.worst_case_utility, 0.25);
        assert_eq!(policy.maximum_regret, 0.25);
        assert!(!policy.optimal_in_every_candidate_world);
    }

    #[test]
    fn minimax_regret_fails_closed_without_a_feasible_policy() {
        let policies = [ExperimentOption {
            intervention: Intervention::SetTreatmentOne,
            cost_units: 2.0,
        }];

        assert_eq!(
            select_minimax_regret_policy(&policies, 1.0),
            Err(AssessmentError::NoFeasiblePolicy)
        );
    }

    #[test]
    fn invalid_budgets_and_costs_fail_closed() {
        assert_eq!(
            assess_identifiability(&[], f64::NAN),
            Err(AssessmentError::InvalidBudget)
        );
        let invalid_cost = [ExperimentOption {
            intervention: Intervention::SetTreatmentZero,
            cost_units: -1.0,
        }];
        assert_eq!(
            assess_identifiability(&invalid_cost, 5.0),
            Err(AssessmentError::InvalidExperimentCost)
        );
    }
}
