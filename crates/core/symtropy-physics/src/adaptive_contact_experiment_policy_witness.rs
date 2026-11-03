// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact, test-only adaptive experiment-policy synthesis over a finite set of
//! certified contact-branch observation intervals.
//!
//! This witness deliberately consumes pre-certified finite-step prediction
//! intervals and instrument-noise intervals. It does not generate physical
//! experiments, certify branch tubes, or infer probabilities. Its purpose is
//! to verify that a safe finite experiment catalog can be turned into an exact
//! worst-case decision policy without eliminating a branch whose possible
//! measurement interval still contains the observed value.

use num_bigint::BigInt;
use num_rational::BigRational;
use std::collections::{HashMap, HashSet};

const MAX_HYPOTHESES: usize = 8;
const MAX_EXPERIMENTS: usize = 12;

type PolicyState = (u16, u16);

fn rational(value: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(value))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RationalInterval {
    lower: BigRational,
    upper: BigRational,
}

impl RationalInterval {
    fn new(lower: BigRational, upper: BigRational) -> Option<Self> {
        (lower <= upper).then_some(Self { lower, upper })
    }

    fn point(value: i64) -> Self {
        Self {
            lower: rational(value),
            upper: rational(value),
        }
    }

    /// Minkowski sum with the instrument-noise interval.
    fn expand_by(&self, noise: &Self) -> Self {
        Self {
            lower: &self.lower + &noise.lower,
            upper: &self.upper + &noise.upper,
        }
    }

    fn contains(&self, observation: &BigRational) -> bool {
        &self.lower <= observation && observation <= &self.upper
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Experiment {
    /// Positive worst-case cost, in one common exact unit for the catalog.
    cost: BigRational,
    /// One certified scalar prediction interval per hypothesis.
    prediction_intervals: Vec<RationalInterval>,
    /// Closed interval containing every instrument error admitted by P0.
    measurement_noise: RationalInterval,
    /// Must be backed by an independent finite-step physical safety receipt.
    finite_step_safety_certified: bool,
}

impl Experiment {
    fn expanded_predictions(&self) -> Vec<RationalInterval> {
        self.prediction_intervals
            .iter()
            .map(|prediction| prediction.expand_by(&self.measurement_noise))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PolicyError {
    InvalidHypothesisCount,
    InvalidExperimentCount,
    InvalidTargetAmbiguity,
    InvalidExperimentCatalog,
    StateBudgetExceeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PolicyValue {
    /// Maximum number of hypotheses remaining at any reachable leaf.
    worst_residual_hypotheses: usize,
    /// Maximum accumulated experiment cost over all observation paths.
    worst_case_cost: BigRational,
}

fn mask_population(mask: u16) -> usize {
    mask.count_ones() as usize
}

fn policy_value_is_better(
    candidate: &PolicyValue,
    incumbent: &PolicyValue,
    target_ambiguity: usize,
) -> bool {
    let candidate_qualified = candidate.worst_residual_hypotheses <= target_ambiguity;
    let incumbent_qualified = incumbent.worst_residual_hypotheses <= target_ambiguity;

    match (candidate_qualified, incumbent_qualified) {
        (true, false) => true,
        (false, true) => false,
        (true, true) => candidate.worst_case_cost < incumbent.worst_case_cost,
        (false, false) => {
            candidate.worst_residual_hypotheses < incumbent.worst_residual_hypotheses
                || (candidate.worst_residual_hypotheses == incumbent.worst_residual_hypotheses
                    && candidate.worst_case_cost < incumbent.worst_case_cost)
        }
    }
}

/// Partition the complete set of possible scalar observations into cells on
/// which the surviving-hypothesis mask is constant.
///
/// Closed interval endpoints are included as separate representatives. Between
/// consecutive distinct endpoints, a rational midpoint represents the entire
/// open cell because no prediction endpoint lies inside it. Empty observation
/// regions are omitted. This prevents a measure-zero boundary overlap from
/// being silently ignored.
fn surviving_hypotheses_for_observation(
    experiment: &Experiment,
    current_hypotheses: u16,
    observation: &BigRational,
) -> u16 {
    experiment
        .expanded_predictions()
        .iter()
        .enumerate()
        .fold(0_u16, |survivors, (index, interval)| {
            if current_hypotheses & (1_u16 << index) != 0 && interval.contains(observation) {
                survivors | (1_u16 << index)
            } else {
                survivors
            }
        })
}

fn observation_outcome_masks(experiment: &Experiment, current_hypotheses: u16) -> Vec<u16> {
    let expanded = experiment.expanded_predictions();
    let mut endpoints = Vec::new();

    for (index, interval) in expanded.iter().enumerate() {
        if current_hypotheses & (1_u16 << index) != 0 {
            endpoints.push(interval.lower.clone());
            endpoints.push(interval.upper.clone());
        }
    }

    endpoints.sort();
    endpoints.dedup();

    let mut representatives = endpoints.clone();
    for pair in endpoints.windows(2) {
        if pair[0] < pair[1] {
            representatives.push((&pair[0] + &pair[1]) / rational(2));
        }
    }
    representatives.sort();
    representatives.dedup();

    let mut outcomes = Vec::new();
    for observation in representatives {
        let mut survivors = 0_u16;
        for (index, interval) in expanded.iter().enumerate() {
            if current_hypotheses & (1_u16 << index) != 0 && interval.contains(&observation) {
                survivors |= 1_u16 << index;
            }
        }
        if survivors != 0 && !outcomes.contains(&survivors) {
            outcomes.push(survivors);
        }
    }

    outcomes.sort_unstable();
    outcomes
}

struct PolicySynthesizer<'a> {
    experiments: &'a [Experiment],
    target_ambiguity: usize,
    state_budget: usize,
    memo: HashMap<PolicyState, PolicyValue>,
    selected_actions: HashMap<PolicyState, usize>,
}

impl<'a> PolicySynthesizer<'a> {
    fn solve_state(&mut self, state: PolicyState) -> Result<PolicyValue, PolicyError> {
        if let Some(value) = self.memo.get(&state) {
            return Ok(value.clone());
        }
        if self.memo.len() >= self.state_budget {
            return Err(PolicyError::StateBudgetExceeded);
        }

        let (hypotheses, available_experiments) = state;
        let hypothesis_count = mask_population(hypotheses);
        if hypothesis_count <= self.target_ambiguity || available_experiments == 0 {
            let terminal = PolicyValue {
                worst_residual_hypotheses: hypothesis_count,
                worst_case_cost: rational(0),
            };
            self.memo.insert(state, terminal.clone());
            return Ok(terminal);
        }

        // Abstention is always available. An experiment is selected only when
        // it improves the worst-case ambiguity or achieves the target.
        let mut best = PolicyValue {
            worst_residual_hypotheses: hypothesis_count,
            worst_case_cost: rational(0),
        };
        let mut best_action = None;

        for experiment_index in 0..self.experiments.len() {
            let experiment_bit = 1_u16 << experiment_index;
            if available_experiments & experiment_bit == 0 {
                continue;
            }
            let experiment = &self.experiments[experiment_index];
            if !experiment.finite_step_safety_certified {
                continue;
            }

            let outcomes = observation_outcome_masks(experiment, hypotheses);
            if outcomes.is_empty() {
                continue;
            }

            let next_available = available_experiments & !experiment_bit;
            let mut worst_child_ambiguity = 0_usize;
            let mut worst_child_cost = rational(0);

            for child_hypotheses in outcomes {
                let child = self.solve_state((child_hypotheses, next_available))?;
                worst_child_ambiguity = worst_child_ambiguity.max(child.worst_residual_hypotheses);
                worst_child_cost = worst_child_cost.max(child.worst_case_cost);
            }

            let candidate = PolicyValue {
                worst_residual_hypotheses: worst_child_ambiguity,
                worst_case_cost: &experiment.cost + worst_child_cost,
            };

            if policy_value_is_better(&candidate, &best, self.target_ambiguity) {
                best = candidate;
                best_action = Some(experiment_index);
            }
        }

        if let Some(action) = best_action {
            self.selected_actions.insert(state, action);
        }
        self.memo.insert(state, best.clone());
        Ok(best)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SynthesizedPolicy {
    root_state: PolicyState,
    target_ambiguity: usize,
    value: PolicyValue,
    /// Freeze the exact experiment catalog used during synthesis so runtime
    /// execution cannot accidentally replay actions against reordered inputs.
    experiments: Vec<Experiment>,
    /// Maps each reachable remaining-hypotheses/unused-experiments state to the
    /// next experiment. Unmapped states are terminal/abstention leaves.
    selected_actions: HashMap<PolicyState, usize>,
}

impl SynthesizedPolicy {
    fn next_experiment(&self, state: PolicyState) -> Option<usize> {
        self.selected_actions.get(&state).copied()
    }

    /// Apply one actual scalar reading to a reachable policy state. A reading
    /// outside every certified interval returns None rather than manufacturing
    /// an empty/identified branch state.
    fn advance_after_observation(
        &self,
        state: PolicyState,
        experiment_index: usize,
        observation: &BigRational,
    ) -> Option<PolicyState> {
        if self.next_experiment(state) != Some(experiment_index) {
            return None;
        }
        let (hypotheses, available_experiments) = state;
        let experiment_bit = 1_u16.checked_shl(experiment_index as u32)?;
        if available_experiments & experiment_bit == 0 {
            return None;
        }
        let experiment = self.experiments.get(experiment_index)?;
        let survivors =
            surviving_hypotheses_for_observation(experiment, hypotheses, observation);
        (survivors != 0).then_some((survivors, available_experiments & !experiment_bit))
    }
}

fn replay_policy_state(
    policy: &SynthesizedPolicy,
    state: PolicyState,
    memo: &mut HashMap<PolicyState, PolicyValue>,
    reachable: &mut HashSet<PolicyState>,
) -> Option<PolicyValue> {
    if let Some(value) = memo.get(&state) {
        return Some(value.clone());
    }
    reachable.insert(state);

    let (hypotheses, available_experiments) = state;
    let hypothesis_count = mask_population(hypotheses);
    if hypothesis_count <= policy.target_ambiguity || available_experiments == 0 {
        if policy.selected_actions.contains_key(&state) {
            return None;
        }
        let terminal = PolicyValue {
            worst_residual_hypotheses: hypothesis_count,
            worst_case_cost: rational(0),
        };
        memo.insert(state, terminal.clone());
        return Some(terminal);
    }

    // An unmapped state is an explicit abstention leaf.
    let Some(experiment_index) = policy.selected_actions.get(&state).copied() else {
        let terminal = PolicyValue {
            worst_residual_hypotheses: hypothesis_count,
            worst_case_cost: rational(0),
        };
        memo.insert(state, terminal.clone());
        return Some(terminal);
    };

    let experiment_bit = 1_u16.checked_shl(experiment_index as u32)?;
    if available_experiments & experiment_bit == 0 {
        return None;
    }
    let experiment = policy.experiments.get(experiment_index)?;
    if !experiment.finite_step_safety_certified {
        return None;
    }

    let outcomes = observation_outcome_masks(experiment, hypotheses);
    if outcomes.is_empty() {
        return None;
    }

    let next_available = available_experiments & !experiment_bit;
    let mut worst_child_ambiguity = 0_usize;
    let mut worst_child_cost = rational(0);
    for child_hypotheses in outcomes {
        let child = replay_policy_state(
            policy,
            (child_hypotheses, next_available),
            memo,
            reachable,
        )?;
        worst_child_ambiguity =
            worst_child_ambiguity.max(child.worst_residual_hypotheses);
        worst_child_cost = worst_child_cost.max(child.worst_case_cost);
    }

    let value = PolicyValue {
        worst_residual_hypotheses: worst_child_ambiguity,
        worst_case_cost: &experiment.cost + worst_child_cost,
    };
    memo.insert(state, value.clone());
    Some(value)
}

/// Independently replay the selected decision tree and recompute its exact
/// worst-case value. This verifies feasibility, safety gating and all reachable
/// leaves; it does not claim optimality against policies outside this tree.
fn verify_synthesized_policy(policy: &SynthesizedPolicy) -> bool {
    let mut memo = HashMap::new();
    let mut reachable = HashSet::new();
    let Some(replayed_value) = replay_policy_state(
        policy,
        policy.root_state,
        &mut memo,
        &mut reachable,
    ) else {
        return false;
    };

    replayed_value == policy.value
        && policy
            .selected_actions
            .keys()
            .all(|state| reachable.contains(state))
}

fn synthesize_policy(
    hypothesis_count: usize,
    experiments: &[Experiment],
    target_ambiguity: usize,
    state_budget: usize,
) -> Result<SynthesizedPolicy, PolicyError> {
    if hypothesis_count == 0 || hypothesis_count > MAX_HYPOTHESES {
        return Err(PolicyError::InvalidHypothesisCount);
    }
    if experiments.is_empty() || experiments.len() > MAX_EXPERIMENTS {
        return Err(PolicyError::InvalidExperimentCount);
    }
    if target_ambiguity == 0 || target_ambiguity > hypothesis_count {
        return Err(PolicyError::InvalidTargetAmbiguity);
    }
    if state_budget == 0
        || experiments.iter().any(|experiment| {
            experiment.cost <= rational(0)
                || experiment.prediction_intervals.len() != hypothesis_count
                || experiment
                    .prediction_intervals
                    .iter()
                    .any(|interval| interval.lower > interval.upper)
                || experiment.measurement_noise.lower > experiment.measurement_noise.upper
        })
    {
        return Err(PolicyError::InvalidExperimentCatalog);
    }

    let root_hypotheses = (1_u16 << hypothesis_count) - 1;
    let safe_experiment_mask = experiments
        .iter()
        .enumerate()
        .filter(|(_, experiment)| experiment.finite_step_safety_certified)
        .fold(0_u16, |mask, (index, _)| mask | (1_u16 << index));
    let root_state = (root_hypotheses, safe_experiment_mask);

    let mut synthesizer = PolicySynthesizer {
        experiments,
        target_ambiguity,
        state_budget,
        memo: HashMap::new(),
        selected_actions: HashMap::new(),
    };
    let value = synthesizer.solve_state(root_state)?;

    Ok(SynthesizedPolicy {
        root_state,
        target_ambiguity,
        value,
        experiments: experiments.to_vec(),
        selected_actions: synthesizer.selected_actions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn experiment(cost: i64, predictions: &[i64], noise: (i64, i64), safe: bool) -> Experiment {
        Experiment {
            cost: rational(cost),
            prediction_intervals: predictions
                .iter()
                .map(|value| RationalInterval::point(*value))
                .collect(),
            measurement_noise: RationalInterval::new(rational(noise.0), rational(noise.1))
                .expect("the noise fixture is ordered"),
            finite_step_safety_certified: safe,
        }
    }

    #[test]
    fn adaptive_policy_uniquely_identifies_three_branches_when_no_single_probe_can() {
        let experiments = [
            experiment(1, &[0, 0, 2], (0, 0), true),
            experiment(1, &[0, 2, 2], (0, 0), true),
        ];

        // Either experiment alone leaves an observationally identical pair.
        for candidate in &experiments {
            let outcomes = observation_outcome_masks(candidate, 0b111);
            assert!(
                outcomes.iter().any(|mask| mask_population(*mask) > 1),
                "no single experiment separates all hypotheses"
            );
        }

        let policy = synthesize_policy(3, &experiments, 1, 10_000)
            .expect("the finite adaptive policy is within the exact state budget");
        assert_eq!(policy.root_state, (0b111, 0b11));
        assert_eq!(policy.value.worst_residual_hypotheses, 1);
        assert_eq!(policy.value.worst_case_cost, rational(2));
        assert!(verify_synthesized_policy(&policy));

        // The first experiment splits {h0,h1} from {h2}; only the first child
        // needs the second measurement.
        assert_eq!(policy.next_experiment((0b111, 0b11)), Some(0));
        assert_eq!(policy.next_experiment((0b011, 0b10)), Some(1));
        assert_eq!(policy.next_experiment((0b100, 0b10)), None);

        // Execute the synthesized policy with actual scalar readings. The
        // first read leaves {h0,h1}; the next read isolates h1. A reading
        // outside the certified outcome set is rejected rather than pruning
        // all branches and pretending that identification succeeded.
        let after_first = policy
            .advance_after_observation(
                policy.root_state,
                0,
                &rational(0),
            )
            .expect("the first observation belongs to h0 and h1");
        assert_eq!(after_first, (0b011, 0b10));
        let after_second = policy
            .advance_after_observation(after_first, 1, &rational(2))
            .expect("the second observation identifies h1");
        assert_eq!(after_second, (0b010, 0));
        assert!(
            policy
                .advance_after_observation(policy.root_state, 0, &rational(9))
                .is_none(),
            "an impossible observation must fail closed"
        );
        let mut corrupted_value = policy.clone();
        corrupted_value.value.worst_case_cost += rational(1);
        assert!(!verify_synthesized_policy(&corrupted_value));
    }

    #[test]
    fn closed_interval_endpoints_are_not_discarded_as_measure_zero_events() {
        let candidate = experiment(1, &[0, 1], (0, 0), true);
        let outcomes = observation_outcome_masks(&candidate, 0b11);
        // The exact endpoint at zero selects h0, the exact endpoint at one
        // selects h1, while the midpoint is impossible and is ignored.
        assert_eq!(outcomes, vec![0b01, 0b10]);
    }

    #[test]
    fn interval_noise_boundary_overlap_preserves_both_hypotheses() {
        let experiments = [experiment(1, &[0, 1], (-1, 1), true)];
        let policy = synthesize_policy(2, &experiments, 1, 100)
            .expect("the two-hypothesis policy is within budget");

        // The expanded closed intervals [-1,1] and [0,2] overlap. The
        // observation at zero is possible under both; a unique claim is false.
        assert_eq!(policy.value.worst_residual_hypotheses, 2);
        assert_eq!(policy.value.worst_case_cost, rational(0));
        assert_eq!(policy.next_experiment(policy.root_state), None);
    }

    #[test]
    fn cheaper_equally_informative_experiment_is_selected() {
        let experiments = [
            experiment(3, &[0, 1], (0, 0), true),
            experiment(1, &[0, 2], (0, 0), true),
        ];
        let policy = synthesize_policy(2, &experiments, 1, 100)
            .expect("the finite catalog is within budget");
        assert_eq!(policy.value.worst_residual_hypotheses, 1);
        assert_eq!(policy.value.worst_case_cost, rational(1));
        assert_eq!(policy.next_experiment(policy.root_state), Some(1));
    }

    #[test]
    fn uncertified_experiment_is_never_selected_even_when_it_separates_every_branch() {
        let experiments = [experiment(1, &[0, 1], (0, 0), false)];
        let policy = synthesize_policy(2, &experiments, 1, 100)
            .expect("unsafe experiments are excluded, not executed");
        assert_eq!(policy.value.worst_residual_hypotheses, 2);
        assert_eq!(policy.next_experiment(policy.root_state), None);
        assert!(verify_synthesized_policy(&policy));

        let catalog = [
            experiment(1, &[0, 1], (0, 0), false),
            experiment(2, &[0, 0], (0, 0), true),
        ];
        let safe_policy = synthesize_policy(2, &catalog, 1, 100)
            .expect("the safe-only catalog state is valid");
        let mut tampered_action = safe_policy.clone();
        tampered_action
            .selected_actions
            .insert(tampered_action.root_state, 0);
        assert!(!verify_synthesized_policy(&tampered_action));
    }

    #[test]
    fn state_budget_exhaustion_is_inconclusive_not_infeasibility() {
        let experiments = [experiment(1, &[0, 1, 2], (0, 0), true)];
        assert_eq!(
            synthesize_policy(3, &experiments, 1, 1),
            Err(PolicyError::StateBudgetExceeded)
        );
    }

    #[test]
    fn malformed_interval_catalog_is_rejected_before_search() {
        let mut invalid = experiment(1, &[0, 1], (0, 0), true);
        invalid.prediction_intervals[1] = RationalInterval {
            lower: rational(2),
            upper: rational(1),
        };
        assert_eq!(
            synthesize_policy(2, &[invalid], 1, 100),
            Err(PolicyError::InvalidExperimentCatalog)
        );
    }
}
