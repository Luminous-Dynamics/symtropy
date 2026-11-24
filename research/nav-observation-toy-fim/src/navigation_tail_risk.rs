//! Exact discrete tail-risk evaluation for finite navigation mission outcomes.
//!
//! This module evaluates a fixed policy's finite loss distribution. It does
//! not optimize a policy tree, estimate physical probabilities, or grant
//! navigation authority. Persistent-model semantics are represented by
//! evaluating each complete mission-model distribution separately and taking
//! the outer maximum; models are never re-sampled inside a distribution.

use core::cmp::Ordering;

use crate::navigation_temporal_mission_minimax::{
    ActionOutcome, ObservableBranchChoice, ObservablePolicy, TemporalMinimaxProblem,
    TerminalDecision, TimedJointAction, PROBABILITY_SCALE,
};

pub const PROBABILITY_SCALE_PPM: u128 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LossMass {
    /// Terminal mission loss in caller-defined, non-negative integer units.
    pub loss: u64,
    /// Non-negative integer probability mass. All entries in one distribution
    /// share an arbitrary common denominator; normalization is unnecessary.
    pub mass: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactRisk {
    numerator: u128,
    denominator: u128,
}

impl ExactRisk {
    fn new(numerator: u128, denominator: u128) -> Result<Self, TailRiskError> {
        if denominator == 0 {
            return Err(TailRiskError::ZeroTotalMass);
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub const fn numerator(self) -> u128 {
        self.numerator
    }

    pub const fn denominator(self) -> u128 {
        self.denominator
    }

    /// Compare two non-negative exact fractions without cross multiplication,
    /// avoiding overflow even when both fractions are close to u128 limits.
    pub fn compare(self, other: Self) -> Ordering {
        compare_fractions(
            self.numerator,
            self.denominator,
            other.numerator,
            other.denominator,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TailRiskError {
    EmptyDistribution,
    ZeroTotalMass,
    InvalidAlpha(u32),
    DuplicateModelId(u64),
    MissingAction(u64),
    InvalidPolicyTree,
    ArithmeticOverflow,
}

impl core::fmt::Display for TailRiskError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyDistribution => write!(f, "loss distribution must not be empty"),
            Self::ZeroTotalMass => write!(f, "loss distribution must have positive total mass"),
            Self::InvalidAlpha(alpha) => write!(
                f,
                "CVaR alpha must be below one million PPM, got {alpha}"
            ),
            Self::DuplicateModelId(id) => write!(f, "mission model identity is duplicated: {id}"),
            Self::MissingAction(id) => write!(f, "policy references an unavailable action: {id}"),
            Self::InvalidPolicyTree => write!(f, "observable policy tree is incomplete or inconsistent"),
            Self::ArithmeticOverflow => write!(f, "checked exact tail-risk arithmetic overflowed"),
        }
    }
}

impl std::error::Error for TailRiskError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelLossDistribution<'a> {
    /// Stable identity of one complete mission-persistent model.
    pub model_id: u64,
    /// Complete stochastic mission-path loss distribution under one fixed
    /// model and one fixed observable policy.
    pub outcomes: &'a [LossMass],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelTailRisk {
    pub model_id: u64,
    pub cvar: ExactRisk,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RobustTailRisk {
    /// Sorted by model identity for deterministic reporting.
    pub per_model: Vec<ModelTailRisk>,
    /// Worst conditional CVaR over complete mission-persistent models.
    pub worst_case_model_id: u64,
    pub worst_case_cvar: ExactRisk,
    pub alpha_ppm: u32,
}

/// Exact expected loss of a finite discrete distribution.
///
/// mass is interpreted as an arbitrary non-negative integer weight. The
/// resulting fraction is reduced, so all distributions can use their own
/// common integer denominator.
pub fn expected_loss(outcomes: &[LossMass]) -> Result<ExactRisk, TailRiskError> {
    let total_mass = total_mass(outcomes)?;
    let mut weighted_loss = 0_u128;
    for outcome in outcomes {
        let contribution = outcome
            .mass
            .checked_mul(outcome.loss as u128)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
        weighted_loss = weighted_loss
            .checked_add(contribution)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
    }
    ExactRisk::new(weighted_loss, total_mass)
}

/// Exact upper-tail conditional value at risk for a discrete loss distribution.
///
/// alpha_ppm is the confidence level in parts per million: 950_000 means
/// CVaR_0.95. A fractional atom at the quantile is included exactly, rather
/// than rounding the tail count to a whole observation.
///
/// The returned fraction is the average loss in the upper (1 - alpha) tail.
/// All arithmetic is integer and checked. The function assumes the supplied
/// weights represent a valid finite distribution with one common denominator;
/// it does not validate the physical or statistical provenance of those
/// probabilities.
pub fn cvar(
    outcomes: &[LossMass],
    alpha_ppm: u32,
) -> Result<ExactRisk, TailRiskError> {
    if alpha_ppm as u128 >= PROBABILITY_SCALE_PPM {
        return Err(TailRiskError::InvalidAlpha(alpha_ppm));
    }
    let total_mass = total_mass(outcomes)?;
    let tail_ppm = PROBABILITY_SCALE_PPM - alpha_ppm as u128;
    let target_scaled_mass = total_mass
        .checked_mul(tail_ppm)
        .ok_or(TailRiskError::ArithmeticOverflow)?;

    // Sorting a copy keeps caller ordering and data unchanged.
    let mut descending = outcomes.to_vec();
    descending.sort_unstable_by(|a, b| {
        b.loss.cmp(&a.loss).then_with(|| b.mass.cmp(&a.mass))
    });

    // A mass unit represents 1 / total_mass of the distribution. Multiplying
    // each atom by one million gives a common grid on which the upper-tail
    // boundary may split an atom exactly.
    let mut remaining = target_scaled_mass;
    let mut tail_loss_numerator = 0_u128;
    for outcome in descending {
        if remaining == 0 {
            break;
        }
        let atom_scaled_mass = outcome
            .mass
            .checked_mul(PROBABILITY_SCALE_PPM)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
        let included_mass = remaining.min(atom_scaled_mass);
        let contribution = included_mass
            .checked_mul(outcome.loss as u128)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
        tail_loss_numerator = tail_loss_numerator
            .checked_add(contribution)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
        remaining -= included_mass;
    }

    if remaining != 0 {
        return Err(TailRiskError::ArithmeticOverflow);
    }
    ExactRisk::new(tail_loss_numerator, target_scaled_mass)
}

/// Evaluate a fixed observable policy under each complete persistent mission
/// model, then return the worst model-conditional CVaR.
///
/// This is the correct outer/inner decomposition for the stated research
/// objective: the model is fixed outside the stochastic path distribution.
/// It is not a policy optimizer and makes no assumption that epistemic model
/// identities have a qualified probability distribution.
pub fn worst_case_cvar(
    models: &[ModelLossDistribution<'_>],
    alpha_ppm: u32,
) -> Result<RobustTailRisk, TailRiskError> {
    if models.is_empty() {
        return Err(TailRiskError::EmptyDistribution);
    }
    if alpha_ppm as u128 >= PROBABILITY_SCALE_PPM {
        return Err(TailRiskError::InvalidAlpha(alpha_ppm));
    }

    let mut ordered = models.to_vec();
    ordered.sort_unstable_by_key(|model| model.model_id);
    for pair in ordered.windows(2) {
        if pair[0].model_id == pair[1].model_id {
            return Err(TailRiskError::DuplicateModelId(pair[0].model_id));
        }
    }

    let mut per_model = Vec::with_capacity(ordered.len());
    let mut worst_case: Option<ModelTailRisk> = None;
    for model in ordered {
        let value = ModelTailRisk {
            model_id: model.model_id,
            cvar: cvar(model.outcomes, alpha_ppm)?,
        };
        let replace = match worst_case {
            None => true,
            Some(current) => {
                value.cvar.compare(current.cvar) == Ordering::Greater
                    || (value.cvar == current.cvar && value.model_id < current.model_id)
            }
        };
        if replace {
            worst_case = Some(value);
        }
        per_model.push(value);
    }

    let worst = worst_case.ok_or(TailRiskError::EmptyDistribution)?;
    Ok(RobustTailRisk {
        per_model,
        worst_case_model_id: worst.model_id,
        worst_case_cvar: worst.cvar,
        alpha_ppm,
    })
}

fn total_mass(outcomes: &[LossMass]) -> Result<u128, TailRiskError> {
    if outcomes.is_empty() {
        return Err(TailRiskError::EmptyDistribution);
    }
    let mut total = 0_u128;
    for outcome in outcomes {
        total = total
            .checked_add(outcome.mass)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
    }
    if total == 0 {
        return Err(TailRiskError::ZeroTotalMass);
    }
    Ok(total)
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

fn compare_fractions(
    mut left_numerator: u128,
    mut left_denominator: u128,
    mut right_numerator: u128,
    mut right_denominator: u128,
) -> Ordering {
    let mut reverse = false;
    loop {
        let left_integer = left_numerator / left_denominator;
        let right_integer = right_numerator / right_denominator;
        if left_integer != right_integer {
            let result = left_integer.cmp(&right_integer);
            return if reverse { result.reverse() } else { result };
        }

        let left_remainder = left_numerator % left_denominator;
        let right_remainder = right_numerator % right_denominator;
        match (left_remainder == 0, right_remainder == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => {
                return if reverse { Ordering::Greater } else { Ordering::Less };
            }
            (false, true) => {
                return if reverse { Ordering::Less } else { Ordering::Greater };
            }
            (false, false) => {
                left_numerator = left_denominator;
                left_denominator = left_remainder;
                right_numerator = right_denominator;
                right_denominator = right_remainder;
                reverse = !reverse;
            }
        }
    }
}



/// Reconstruct complete mission-path loss distributions for a fixed temporal
/// mission-minimax policy.
///
/// This is an independent evaluator-side propagation of the public model and
/// policy fields. It reconstructs all observable outcome branches and splits
/// each terminal state mass into its realized loss atom. The resulting mass
/// denominator is PROBABILITY_SCALE_PPM^5 for every model, matching the
/// finite oracle's two-stage integer probability scale.
///
/// The two candidate mission models are evaluated separately and remain fixed
/// through each complete path. This function does not optimize the policy and
/// must not be exposed to the controller's observation history.
pub fn mission_path_loss_distributions(
    problem: &TemporalMinimaxProblem,
    actions: &[TimedJointAction],
    policy: &ObservablePolicy,
) -> Result<[Vec<LossMass>; 2], TailRiskError> {
    for model in 0..2 {
        let total = problem.prior_by_model_ppm[model]
            .iter()
            .try_fold(0_u64, |sum, value| sum.checked_add(*value));
        if total != Some(PROBABILITY_SCALE) {
            return Err(TailRiskError::InvalidPolicyTree);
        }
    }

    for (index, action) in actions.iter().enumerate() {
        if actions[..index]
            .iter()
            .any(|prior| prior.action_id == action.action_id)
        {
            return Err(TailRiskError::InvalidPolicyTree);
        }
    }

    let padding_one_action = scale_power(2)?;
    let padding_no_action = scale_power(4)?;
    let expected_total_mass = scale_power(5)?;
    let mut distributions: [Vec<LossMass>; 2] = [Vec::new(), Vec::new()];

    let Some(first_action_id) = policy.first_action_id else {
        if !policy.branches.is_empty() {
            return Err(TailRiskError::InvalidPolicyTree);
        }
        let decision = policy
            .root_terminal_decision
            .ok_or(TailRiskError::InvalidPolicyTree)?;
        for model in 0..2 {
            append_terminal_state_masses(
                &mut distributions[model],
                problem.prior_by_model_ppm[model].map(u128::from),
                decision,
                problem,
                padding_no_action,
            )?;
        }
        for distribution in &distributions {
            if total_mass(distribution)? != expected_total_mass {
                return Err(TailRiskError::InvalidPolicyTree);
            }
        }
        return Ok(distributions);
    };

    if policy.root_terminal_decision.is_some() || policy.branches.len() != 3 {
        return Err(TailRiskError::InvalidPolicyTree);
    }
    let first_action = actions
        .iter()
        .find(|action| action.action_id == first_action_id)
        .ok_or(TailRiskError::MissingAction(first_action_id))?;

    let mut seen = [false; 3];
    for branch in &policy.branches {
        let first_index = outcome_index(branch.first_outcome);
        if seen[first_index] {
            return Err(TailRiskError::InvalidPolicyTree);
        }
        seen[first_index] = true;

        for model in 0..2 {
            let initial = problem.prior_by_model_ppm[model].map(u128::from);
            let first_outcomes = propagate_action(model, initial, first_action)?;
            let first_state_mass = first_outcomes[first_index];

            match &branch.choice {
                ObservableBranchChoice::Stop { terminal_decision } => {
                    append_terminal_state_masses(
                        &mut distributions[model],
                        first_state_mass,
                        *terminal_decision,
                        problem,
                        padding_one_action,
                    )?;
                }
                ObservableBranchChoice::Acquire {
                    second_action_id,
                    terminal_decisions,
                } => {
                    if *second_action_id == first_action_id {
                        return Err(TailRiskError::InvalidPolicyTree);
                    }
                    let second_action = actions
                        .iter()
                        .find(|action| action.action_id == *second_action_id)
                        .ok_or(TailRiskError::MissingAction(*second_action_id))?;
                    let second_outcomes = propagate_action(model, first_state_mass, second_action)?;
                    for second_outcome_index in 0..3 {
                        append_terminal_state_masses(
                            &mut distributions[model],
                            second_outcomes[second_outcome_index],
                            terminal_decisions[second_outcome_index],
                            problem,
                            1,
                        )?;
                    }
                }
            }
        }
    }

    if seen.iter().any(|present| !present) {
        return Err(TailRiskError::InvalidPolicyTree);
    }
    for distribution in &distributions {
        if total_mass(distribution)? != expected_total_mass {
            return Err(TailRiskError::InvalidPolicyTree);
        }
    }
    Ok(distributions)
}

fn outcome_index(outcome: ActionOutcome) -> usize {
    match outcome {
        ActionOutcome::NoObservation => 0,
        ActionOutcome::ObservedZero => 1,
        ActionOutcome::ObservedOne => 2,
    }
}

fn scale_power(exponent: u8) -> Result<u128, TailRiskError> {
    let mut value = 1_u128;
    for _ in 0..exponent {
        value = value
            .checked_mul(PROBABILITY_SCALE_PPM)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
    }
    Ok(value)
}

/// Returns post-use state mass for no-observation, observed-zero and
/// observed-one. State indices are nominal/inside, nominal/outside,
/// shared-cause/inside, shared-cause/outside.
fn propagate_action(
    model: usize,
    mass: [u128; 4],
    action: &TimedJointAction,
) -> Result<[[u128; 4]; 3], TailRiskError> {
    let scale = PROBABILITY_SCALE_PPM;
    let mut outcomes = [[0_u128; 4]; 3];
    for outcome_index in 0..3 {
        let likelihood_for_state = |state: usize| {
            let likelihoods = action.likelihoods[model][state];
            match outcome_index {
                0 => likelihoods.no_observation_ppm,
                1 => likelihoods.observed_zero_ppm,
                _ => likelihoods.observed_one_ppm,
            }
        };

        let mut capture = [0_u128; 4];
        for state in 0..4 {
            capture[state] = mass[state]
                .checked_mul(likelihood_for_state(state) as u128)
                .ok_or(TailRiskError::ArithmeticOverflow)?;
        }

        for cause in 0..2 {
            let inside = cause * 2;
            let outside = inside + 1;
            let transition = action.transitions[model][cause];
            let inside_to_outside = transition.inside_to_outside_ppm as u128;
            let outside_to_inside = transition.outside_to_inside_ppm as u128;
            if inside_to_outside > scale || outside_to_inside > scale {
                return Err(TailRiskError::InvalidPolicyTree);
            }
            let inside_to_inside = scale - inside_to_outside;
            let outside_to_outside = scale - outside_to_inside;

            outcomes[outcome_index][inside] = capture[inside]
                .checked_mul(inside_to_inside)
                .and_then(|value| value.checked_add(
                    capture[outside].checked_mul(outside_to_inside)?,
                ))
                .ok_or(TailRiskError::ArithmeticOverflow)?;
            outcomes[outcome_index][outside] = capture[inside]
                .checked_mul(inside_to_outside)
                .and_then(|value| value.checked_add(
                    capture[outside].checked_mul(outside_to_outside)?,
                ))
                .ok_or(TailRiskError::ArithmeticOverflow)?;
        }
    }
    Ok(outcomes)
}

fn append_terminal_state_masses(
    distribution: &mut Vec<LossMass>,
    state_mass: [u128; 4],
    decision: TerminalDecision,
    problem: &TemporalMinimaxProblem,
    multiplier: u128,
) -> Result<(), TailRiskError> {
    for (state_index, mass) in state_mass.into_iter().enumerate() {
        if mass == 0 {
            continue;
        }
        let loss = match (state_index % 2, decision) {
            (0, TerminalDecision::DeclareDegraded) => problem.false_degraded_loss,
            (1, TerminalDecision::DeclareSafe) => problem.false_safe_loss,
            _ => 0,
        };
        let scaled_mass = mass
            .checked_mul(multiplier)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
        if let Some(existing) = distribution.iter_mut().find(|item| item.loss == loss) {
            existing.mass = existing
                .mass
                .checked_add(scaled_mass)
                .ok_or(TailRiskError::ArithmeticOverflow)?;
        } else {
            distribution.push(LossMass {
                loss,
                mass: scaled_mass,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn risk(numerator: u128, denominator: u128) -> ExactRisk {
        ExactRisk::new(numerator, denominator).unwrap()
    }

    #[test]
    fn cvar_includes_fractional_atom_at_tail_boundary_exactly() {
        let outcomes = [
            LossMass { loss: 100, mass: 1 },
            LossMass { loss: 0, mass: 99 },
        ];
        assert_eq!(expected_loss(&outcomes).unwrap(), risk(1, 1));
        assert_eq!(cvar(&outcomes, 950_000).unwrap(), risk(20, 1));
    }

    #[test]
    fn cvar_equals_expectation_at_alpha_zero() {
        let outcomes = [
            LossMass { loss: 10, mass: 2 },
            LossMass { loss: 3, mass: 5 },
            LossMass { loss: 0, mass: 3 },
        ];
        assert_eq!(cvar(&outcomes, 0).unwrap(), expected_loss(&outcomes).unwrap());
    }

    #[test]
    fn cvar_of_constant_loss_equals_that_loss() {
        let outcomes = [
            LossMass { loss: 7, mass: 13 },
            LossMass { loss: 7, mass: 2 },
            LossMass { loss: 7, mass: 85 },
        ];
        assert_eq!(cvar(&outcomes, 950_000).unwrap(), risk(7, 1));
    }

    #[test]
    fn cvar_is_monotone_in_confidence_level() {
        let outcomes = [
            LossMass { loss: 10, mass: 1 },
            LossMass { loss: 1, mass: 9 },
            LossMass { loss: 0, mass: 90 },
        ];
        let levels = [0, 500_000, 900_000, 950_000, 990_000, 999_000];
        let values = levels.map(|alpha| cvar(&outcomes, alpha).unwrap());
        assert!(values.windows(2).all(|pair| pair[0].compare(pair[1]) != Ordering::Greater));
    }

    #[test]
    fn cvar_obeys_positive_translation_and_scaling() {
        let base = [
            LossMass { loss: 8, mass: 1 },
            LossMass { loss: 2, mass: 3 },
            LossMass { loss: 1, mass: 6 },
        ];
        let transformed = [
            LossMass { loss: 26, mass: 1 },
            LossMass { loss: 14, mass: 3 },
            LossMass { loss: 12, mass: 6 },
        ];
        let expected = cvar(&base, 800_000).unwrap();
        let actual = cvar(&transformed, 800_000).unwrap();
        assert_eq!(
            actual,
            risk(expected.numerator * 2 + 10 * expected.denominator, expected.denominator)
        );
    }

    #[test]
    fn outcome_order_does_not_change_exact_tail_risk() {
        let forward = [
            LossMass { loss: 9, mass: 3 },
            LossMass { loss: 4, mass: 11 },
            LossMass { loss: 0, mass: 7 },
        ];
        let reverse = [
            LossMass { loss: 0, mass: 7 },
            LossMass { loss: 4, mass: 11 },
            LossMass { loss: 9, mass: 3 },
        ];
        assert_eq!(cvar(&forward, 900_000), cvar(&reverse, 900_000));
    }

    #[test]
    fn model_set_uses_outer_maximum_of_model_conditional_cvar() {
        let nominal = [LossMass { loss: 1, mass: 100 }];
        let faulted = [
            LossMass { loss: 10, mass: 1 },
            LossMass { loss: 0, mass: 99 },
        ];
        let result = worst_case_cvar(
            &[
                ModelLossDistribution { model_id: 22, outcomes: &faulted },
                ModelLossDistribution { model_id: 11, outcomes: &nominal },
            ],
            950_000,
        )
        .unwrap();
        assert_eq!(result.worst_case_model_id, 22);
        assert_eq!(result.worst_case_cvar, risk(2, 1));
        assert_eq!(result.per_model.iter().map(|item| item.model_id).collect::<Vec<_>>(), [11, 22]);
    }

    #[test]
    fn adding_a_persistent_model_cannot_reduce_worst_case_cvar() {
        let a = [LossMass { loss: 4, mass: 100 }];
        let b = [LossMass { loss: 8, mass: 100 }];
        let c = [LossMass { loss: 12, mass: 100 }];
        let before = worst_case_cvar(
            &[
                ModelLossDistribution { model_id: 1, outcomes: &a },
                ModelLossDistribution { model_id: 2, outcomes: &b },
            ],
            990_000,
        )
        .unwrap();
        let after = worst_case_cvar(
            &[
                ModelLossDistribution { model_id: 1, outcomes: &a },
                ModelLossDistribution { model_id: 2, outcomes: &b },
                ModelLossDistribution { model_id: 3, outcomes: &c },
            ],
            990_000,
        )
        .unwrap();
        assert_ne!(after.worst_case_cvar.compare(before.worst_case_cvar), Ordering::Less);
    }

    #[test]
    fn fraction_comparison_avoids_cross_product_overflow() {
        let left = ExactRisk { numerator: u128::MAX - 1, denominator: u128::MAX };
        let right = ExactRisk { numerator: u128::MAX - 2, denominator: u128::MAX - 1 };
        assert_eq!(left.compare(right), Ordering::Greater);
    }

    #[test]
    fn rejects_empty_zero_mass_and_alpha_one() {
        assert_eq!(cvar(&[], 950_000), Err(TailRiskError::EmptyDistribution));
        assert_eq!(
            cvar(&[LossMass { loss: 1, mass: 0 }], 950_000),
            Err(TailRiskError::ZeroTotalMass)
        );
        assert_eq!(
            cvar(&[LossMass { loss: 1, mass: 1 }], 1_000_000),
            Err(TailRiskError::InvalidAlpha(1_000_000))
        );
    }

    #[test]
    fn rejects_overflow_in_total_mass_and_loss_products() {
        assert_eq!(
            expected_loss(&[
                LossMass { loss: 0, mass: u128::MAX },
                LossMass { loss: 0, mass: 1 },
            ]),
            Err(TailRiskError::ArithmeticOverflow)
        );
        assert_eq!(
            expected_loss(&[LossMass { loss: u64::MAX, mass: u128::MAX }]),
            Err(TailRiskError::ArithmeticOverflow)
        );
    }

    #[test]
    fn independently_expanded_integer_oracle_matches_cvar() {
        fn expanded_oracle(outcomes: &[LossMass], alpha_ppm: u32) -> ExactRisk {
            let mut expanded = Vec::new();
            for outcome in outcomes {
                for _ in 0..outcome.mass {
                    expanded.push(outcome.loss);
                }
            }
            expanded.sort_unstable_by(|a, b| b.cmp(a));
            let tail = (expanded.len() as u128)
                * (PROBABILITY_SCALE_PPM - alpha_ppm as u128);
            let mut left = tail;
            let mut numerator = 0_u128;
            for loss in expanded {
                if left == 0 {
                    break;
                }
                let included = left.min(PROBABILITY_SCALE_PPM);
                numerator += included * loss as u128;
                left -= included;
            }
            ExactRisk::new(numerator, tail).unwrap()
        }

        let cases: [&[LossMass]; 4] = [
            &[
                LossMass { loss: 7, mass: 1 },
                LossMass { loss: 3, mass: 2 },
                LossMass { loss: 0, mass: 4 },
            ],
            &[
                LossMass { loss: 100, mass: 1 },
                LossMass { loss: 0, mass: 99 },
            ],
            &[
                LossMass { loss: 4, mass: 3 },
                LossMass { loss: 4, mass: 2 },
                LossMass { loss: 1, mass: 2 },
            ],
            &[
                LossMass { loss: 9, mass: 1 },
                LossMass { loss: 8, mass: 1 },
                LossMass { loss: 7, mass: 1 },
            ],
        ];
        for case in cases {
            for alpha in [0, 250_000, 500_000, 750_000, 900_000, 950_000, 999_000] {
                assert_eq!(cvar(case, alpha).unwrap(), expanded_oracle(case, alpha));
            }
        }
    }

    #[test]
    fn robust_model_ties_choose_smallest_identity_deterministically() {
        let outcomes = [LossMass { loss: 5, mass: 10 }];
        let result = worst_case_cvar(
            &[
                ModelLossDistribution { model_id: 9, outcomes: &outcomes },
                ModelLossDistribution { model_id: 3, outcomes: &outcomes },
            ],
            950_000,
        )
        .unwrap();
        assert_eq!(result.worst_case_model_id, 3);
    }

    #[test]
    fn rejects_duplicate_persistent_model_identity() {
        let outcomes = [LossMass { loss: 5, mass: 10 }];
        assert_eq!(
            worst_case_cvar(
                &[
                    ModelLossDistribution { model_id: 1, outcomes: &outcomes },
                    ModelLossDistribution { model_id: 1, outcomes: &outcomes },
                ],
                950_000,
            ),
            Err(TailRiskError::DuplicateModelId(1))
        );
    }
}
