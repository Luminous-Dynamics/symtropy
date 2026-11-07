//! Paired-seed synthetic episode runner for sequential intervention policies.
//!
//! The runner compares adaptive selectors with static and balanced schedules on
//! the same synthetic ground-truth scenarios. It reports software benchmark
//! quantities only; it is not a biological simulator or a SOTA claim.

use std::collections::BTreeSet;

use super::scenario_tree::{
    ScenarioTreeConfig, ScenarioTreeError, TransitionConstraints, plan_first_action,
};
use super::{
    AbstainReason, Decision, DecisionConfig, DecisionState, EngineError, EvidenceTier,
    ExperimentRecord, InterventionCandidate, ModelAdequacy, Posterior, PosteriorUpdate,
    SelectionPolicy, StopReason, condition_on_outcome, entropy, expected_information_gain,
    reject_candidate, splitmix64, validate_candidate, validate_posterior,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BenchmarkPolicy {
    /// Rank all actions once under the initial posterior, then cycle through
    /// that frozen ranking without looking at later outcomes.
    StaticOneShotRanking,
    /// Recompute one-step EIG at every look.
    GreedySequentialEig,
    /// Recompute EIG divided by the shared comparable cost unit.
    CostNormalizedSequentialEig,
    /// Recompute the worst-case EIG over the declared ambiguity posterior set.
    RobustMaximinSequentialEig,
    /// Deterministic seeded selection from the feasible candidate set.
    RandomFeasible,
    /// Fixed canonical candidate-ID round-robin schedule.
    BalancedRoundRobin,
    /// Receding-horizon exact categorical scenario-tree planner.
    ScenarioTreeLookahead,
    /// Synthetic clairvoyant one-step upper bound using the true model identity.
    /// This is not a deployable policy or a finite-horizon optimality claim.
    ClairvoyantMyopicUpperBound,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticScenario {
    pub scenario_id: String,
    pub initial_posterior: Posterior,
    pub ambiguity_posteriors: Vec<Posterior>,
    pub true_model_id: String,
    pub candidates: Vec<InterventionCandidate>,
    pub horizon: u32,
    pub cost_budget: f64,
    /// A classification-score threshold, not a frequentist significance level.
    pub confident_model_probability_threshold: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EpisodeTermination {
    HorizonComplete,
    BudgetExhausted,
    NoPermittedCandidate,
    PolicyStop(StopReason),
    PolicyAbstain(AbstainReason),
    ModelInadequate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeSummary {
    pub scenario_id: String,
    pub policy: BenchmarkPolicy,
    pub seed: u64,
    pub selected_candidate_ids: Vec<String>,
    pub observed_outcome_ids: Vec<String>,
    pub steps_completed: u32,
    pub cost_spent: f64,
    pub cumulative_expected_information_gain_nats: f64,
    pub final_model_entropy_nats: f64,
    pub true_model_posterior: f64,
    pub most_probable_model_id: String,
    pub false_confident_selection: bool,
    /// Sum of per-step gaps to a clairvoyant *myopic* KL-per-cost selector.
    /// This is not finite-horizon regret or an optimality gap.
    pub myopic_oracle_regret_nats_per_cost: f64,
    pub termination: EpisodeTermination,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BenchmarkError {
    InvalidScenario(&'static str),
    UnknownTrueModel(String),
    TrueModelHasZeroPrior(String),
    CostUnitMismatch,
    Candidate(EngineError),
    ScenarioTree(ScenarioTreeError),
}

impl From<EngineError> for BenchmarkError {
    fn from(value: EngineError) -> Self {
        Self::Candidate(value)
    }
}

impl From<ScenarioTreeError> for BenchmarkError {
    fn from(value: ScenarioTreeError) -> Self {
        Self::ScenarioTree(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairedComparison {
    pub scenario_id: String,
    pub seed: u64,
    pub results: Vec<EpisodeSummary>,
}

fn validate_scenario(scenario: &SyntheticScenario) -> Result<(), BenchmarkError> {
    if scenario.scenario_id.trim().is_empty() {
        return Err(BenchmarkError::InvalidScenario("empty scenario ID"));
    }
    validate_posterior(&scenario.initial_posterior)?;
    if !scenario
        .initial_posterior
        .contains_key(&scenario.true_model_id)
    {
        return Err(BenchmarkError::UnknownTrueModel(
            scenario.true_model_id.clone(),
        ));
    }
    if scenario.initial_posterior[&scenario.true_model_id] <= 0.0 {
        return Err(BenchmarkError::TrueModelHasZeroPrior(
            scenario.true_model_id.clone(),
        ));
    }
    if scenario.candidates.is_empty() {
        return Err(BenchmarkError::InvalidScenario("empty candidate set"));
    }
    if scenario.horizon == 0 {
        return Err(BenchmarkError::InvalidScenario("zero experiment horizon"));
    }
    if !scenario.cost_budget.is_finite() || scenario.cost_budget < 0.0 {
        return Err(BenchmarkError::InvalidScenario("invalid cost budget"));
    }
    if !scenario.confident_model_probability_threshold.is_finite()
        || !(0.0..=1.0).contains(&scenario.confident_model_probability_threshold)
    {
        return Err(BenchmarkError::InvalidScenario(
            "invalid confident-classification threshold",
        ));
    }
    let model_ids: BTreeSet<String> = scenario.initial_posterior.keys().cloned().collect();
    let mut candidate_ids = BTreeSet::new();
    let mut cost_units = BTreeSet::new();
    for candidate in &scenario.candidates {
        validate_candidate(candidate, &model_ids)?;
        cost_units.insert(candidate.cost_unit_id.clone());
        if !candidate_ids.insert(candidate.candidate_id.clone()) {
            return Err(BenchmarkError::InvalidScenario(
                "duplicate intervention candidate ID",
            ));
        }
    }
    if cost_units.len() > 1 {
        return Err(BenchmarkError::CostUnitMismatch);
    }
    for posterior in &scenario.ambiguity_posteriors {
        validate_posterior(posterior)?;
        if posterior.keys().ne(scenario.initial_posterior.keys()) {
            return Err(BenchmarkError::InvalidScenario(
                "ambiguity posterior model set mismatch",
            ));
        }
    }
    Ok(())
}

fn candidate_by_id<'a>(
    candidates: &'a [InterventionCandidate],
    id: &str,
) -> &'a InterventionCandidate {
    candidates
        .iter()
        .find(|candidate| candidate.candidate_id == id)
        .expect("candidate ID was selected from the validated scenario manifest")
}

fn eligible_candidates<'a>(
    state: &DecisionState,
    candidates: &'a [InterventionCandidate],
    config: &DecisionConfig,
) -> Vec<&'a InterventionCandidate> {
    let mut eligible: Vec<&InterventionCandidate> = candidates
        .iter()
        .filter(|candidate| reject_candidate(candidate, config, state).is_none())
        .collect();
    eligible.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    eligible
}

fn static_one_shot_order(
    state: &DecisionState,
    candidates: &[InterventionCandidate],
    config: &DecisionConfig,
) -> Result<Vec<String>, BenchmarkError> {
    let mut scored = eligible_candidates(state, candidates, config)
        .into_iter()
        .map(|candidate| {
            Ok((
                candidate.candidate_id.clone(),
                expected_information_gain(&state.posterior, candidate)? / candidate.cost,
            ))
        })
        .collect::<Result<Vec<_>, EngineError>>()?;
    scored.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    Ok(scored.into_iter().map(|(id, _)| id).collect())
}

fn outcome_kl_under_true_model(
    posterior: &Posterior,
    true_model_id: &str,
    candidate: &InterventionCandidate,
) -> f64 {
    let true_mass = posterior[true_model_id];
    let alternative_mass = (1.0 - true_mass).max(0.0);
    if alternative_mass == 0.0 {
        return 0.0;
    }
    candidate
        .outcomes
        .iter()
        .map(|outcome| {
            let p_true = outcome.model_probabilities[true_model_id];
            if p_true == 0.0 {
                return 0.0;
            }
            let p_alternative: f64 = posterior
                .iter()
                .filter(|(model_id, _)| model_id.as_str() != true_model_id)
                .map(|(model_id, probability)| {
                    probability / alternative_mass * outcome.model_probabilities[model_id]
                })
                .sum();
            if p_alternative == 0.0 {
                f64::INFINITY
            } else {
                p_true * (p_true / p_alternative).ln()
            }
        })
        .sum()
}

fn clairvoyant_candidate_id(
    state: &DecisionState,
    candidates: &[InterventionCandidate],
    config: &DecisionConfig,
    true_model_id: &str,
) -> Option<String> {
    eligible_candidates(state, candidates, config)
        .into_iter()
        .map(|candidate| {
            (
                candidate.candidate_id.clone(),
                outcome_kl_under_true_model(&state.posterior, true_model_id, candidate)
                    / candidate.cost,
            )
        })
        .max_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| right.0.cmp(&left.0))
        })
        .map(|(id, _)| id)
}

struct PolicySelectionContext<'a> {
    state: &'a DecisionState,
    candidates: &'a [InterventionCandidate],
    config: &'a DecisionConfig,
    static_order: &'a [String],
    true_model_id: &'a str,
}

fn choose_for_policy(
    policy: BenchmarkPolicy,
    context: &PolicySelectionContext<'_>,
    step: u32,
    seed: u64,
) -> Result<Result<String, EpisodeTermination>, BenchmarkError> {
    let state = context.state;
    let candidates = context.candidates;
    let config = context.config;
    let static_order = context.static_order;
    let true_model_id = context.true_model_id;
    match policy {
        BenchmarkPolicy::StaticOneShotRanking | BenchmarkPolicy::BalancedRoundRobin => {
            let order: Vec<String> = if policy == BenchmarkPolicy::StaticOneShotRanking {
                static_order.to_vec()
            } else {
                let mut ids: Vec<String> = candidates
                    .iter()
                    .map(|candidate| candidate.candidate_id.clone())
                    .collect();
                ids.sort();
                ids
            };
            if order.is_empty() {
                return Ok(Err(EpisodeTermination::NoPermittedCandidate));
            }
            let start_index = (step as usize) % order.len();
            for id in order.iter().cycle().skip(start_index).take(order.len()) {
                let candidate = candidate_by_id(candidates, id);
                if reject_candidate(candidate, config, state).is_none() {
                    return Ok(Ok(id.clone()));
                }
            }
            Ok(Err(EpisodeTermination::NoPermittedCandidate))
        }
        BenchmarkPolicy::ClairvoyantMyopicUpperBound => {
            Ok(
                clairvoyant_candidate_id(state, candidates, config, true_model_id)
                    .ok_or(EpisodeTermination::NoPermittedCandidate),
            )
        }
        BenchmarkPolicy::ScenarioTreeLookahead => {
            let remaining_horizon = config
                .maximum_experiments
                .saturating_sub(state.completed_experiments)
                .clamp(1, 4);
            let plan = plan_first_action(
                &state.posterior,
                candidates,
                state.remaining_cost_budget,
                config,
                &ScenarioTreeConfig {
                    horizon: remaining_horizon,
                    maximum_tree_nodes: 100_000,
                    discount_factor: 0.95,
                    transitions: TransitionConstraints::default(),
                },
            )?;
            Ok(Ok(plan.first_candidate_id))
        }
        adaptive => {
            let selection_policy = match adaptive {
                BenchmarkPolicy::GreedySequentialEig => SelectionPolicy::GreedyEig,
                BenchmarkPolicy::CostNormalizedSequentialEig => SelectionPolicy::CostNormalizedEig,
                BenchmarkPolicy::RobustMaximinSequentialEig => SelectionPolicy::RobustMaximin,
                BenchmarkPolicy::RandomFeasible => SelectionPolicy::DeterministicRandom {
                    seed: seed ^ u64::from(step).wrapping_mul(0x9e37_79b9_7f4a_7c15),
                },
                BenchmarkPolicy::StaticOneShotRanking
                | BenchmarkPolicy::BalancedRoundRobin
                | BenchmarkPolicy::ScenarioTreeLookahead
                | BenchmarkPolicy::ClairvoyantMyopicUpperBound => unreachable!(),
            };
            let decision = super::choose_next(
                state,
                candidates,
                &DecisionConfig {
                    policy: selection_policy,
                    ..config.clone()
                },
            )?;
            match decision {
                Decision::Select(selection) => Ok(Ok(selection.candidate_id)),
                Decision::Stop { reason, .. } => Ok(Err(EpisodeTermination::PolicyStop(reason))),
                Decision::Abstain { reason, .. } => {
                    Ok(Err(EpisodeTermination::PolicyAbstain(reason)))
                }
            }
        }
    }
}

fn sample_outcome_id(candidate: &InterventionCandidate, true_model_id: &str, seed: u64) -> String {
    let draw = splitmix64(seed);
    let unit_interval = (draw >> 11) as f64 / ((1_u64 << 53) as f64);
    let mut outcomes: Vec<&super::OutcomeLikelihood> = candidate.outcomes.iter().collect();
    outcomes.sort_by(|left, right| left.outcome_id.cmp(&right.outcome_id));
    let mut cumulative = 0.0;
    for outcome in &outcomes {
        cumulative += outcome.model_probabilities[true_model_id];
        if unit_interval < cumulative {
            return outcome.outcome_id.clone();
        }
    }
    outcomes
        .last()
        .expect("candidate outcome set is non-empty after validation")
        .outcome_id
        .clone()
}

/// Run one synthetic episode with a deterministic, paired-comparison seed.
///
/// Each generated observation represents a new independent replication unit.
/// Repeated within-animal/time-series observations require a conditional
/// trajectory likelihood and are intentionally outside this V0 episode runner.
pub fn run_synthetic_episode(
    scenario: &SyntheticScenario,
    policy: BenchmarkPolicy,
    seed: u64,
) -> Result<EpisodeSummary, BenchmarkError> {
    validate_scenario(scenario)?;
    let mut state = DecisionState {
        posterior: scenario.initial_posterior.clone(),
        ambiguity_posteriors: scenario.ambiguity_posteriors.clone(),
        remaining_cost_budget: scenario.cost_budget,
        completed_experiments: 0,
        model_adequacy: ModelAdequacy::Adequate,
        history: Vec::new(),
    };
    let config = DecisionConfig {
        policy: match policy {
            BenchmarkPolicy::GreedySequentialEig => SelectionPolicy::GreedyEig,
            BenchmarkPolicy::RobustMaximinSequentialEig => SelectionPolicy::RobustMaximin,
            _ => SelectionPolicy::CostNormalizedEig,
        },
        maximum_experiments: scenario.horizon,
        ..DecisionConfig::default()
    };
    let static_order = static_one_shot_order(&state, &scenario.candidates, &config)?;
    let mut selected_candidate_ids = Vec::new();
    let mut observed_outcome_ids = Vec::new();
    let mut cumulative_expected_information_gain_nats = 0.0;
    let mut cost_spent = 0.0;
    let mut myopic_oracle_regret_nats_per_cost = 0.0;
    let mut termination = EpisodeTermination::HorizonComplete;

    for step in 0..scenario.horizon {
        if state.remaining_cost_budget <= 0.0 {
            termination = EpisodeTermination::BudgetExhausted;
            break;
        }
        let candidate_id = match choose_for_policy(
            policy,
            &PolicySelectionContext {
                state: &state,
                candidates: &scenario.candidates,
                config: &config,
                static_order: &static_order,
                true_model_id: &scenario.true_model_id,
            },
            step,
            seed,
        )? {
            Ok(id) => id,
            Err(reason) => {
                termination = reason;
                break;
            }
        };
        let candidate = candidate_by_id(&scenario.candidates, &candidate_id);
        let current_eig = expected_information_gain(&state.posterior, candidate)?;
        let eligible = eligible_candidates(&state, &scenario.candidates, &config);
        let maximum_oracle_utility = eligible
            .iter()
            .map(|item| {
                outcome_kl_under_true_model(&state.posterior, &scenario.true_model_id, item)
                    / item.cost
            })
            .fold(0.0_f64, f64::max);
        let selected_oracle_utility =
            outcome_kl_under_true_model(&state.posterior, &scenario.true_model_id, candidate)
                / candidate.cost;
        let step_regret = if maximum_oracle_utility.is_infinite() {
            if selected_oracle_utility.is_infinite() {
                0.0
            } else {
                f64::INFINITY
            }
        } else {
            (maximum_oracle_utility - selected_oracle_utility).max(0.0)
        };
        myopic_oracle_regret_nats_per_cost += step_regret;

        let outcome_id = sample_outcome_id(
            candidate,
            &scenario.true_model_id,
            seed ^ u64::from(step).wrapping_mul(0xd1b5_4a32_d192_ed03),
        );
        let record = ExperimentRecord {
            sample_id: format!("{}-{seed}-sample-{step}", scenario.scenario_id),
            evidence_tier: EvidenceTier::SyntheticFixture,
            assigned_candidate_id: candidate.candidate_id.clone(),
            delivered_protocol_id: candidate.protocol_id.clone(),
            outcome_id: outcome_id.clone(),
            replication_unit_id: format!("{}-{seed}-animal-{step}", scenario.scenario_id),
            session_id: format!("{}-{seed}-session-{step}", scenario.scenario_id),
            washout_cleared: true,
        };
        match condition_on_outcome(
            &state.posterior,
            candidate,
            &record,
            &state.history,
            config.minimum_delivery_fidelity,
        )? {
            PosteriorUpdate::Updated { posterior, .. } => state.posterior = posterior,
            PosteriorUpdate::ModelInadequate { .. } => {
                termination = EpisodeTermination::ModelInadequate;
                break;
            }
        }
        state.history.push(record);
        state.completed_experiments += 1;
        state.remaining_cost_budget = (state.remaining_cost_budget - candidate.cost).max(0.0);
        cost_spent += candidate.cost;
        cumulative_expected_information_gain_nats += current_eig;
        selected_candidate_ids.push(candidate.candidate_id.clone());
        observed_outcome_ids.push(outcome_id);
    }

    let true_model_posterior = state.posterior[&scenario.true_model_id];
    let (most_probable_model_id, maximum_probability) = state
        .posterior
        .iter()
        .max_by(|left, right| left.1.total_cmp(right.1))
        .map(|(id, probability)| (id.clone(), *probability))
        .expect("validated posterior is non-empty");
    Ok(EpisodeSummary {
        scenario_id: scenario.scenario_id.clone(),
        policy,
        seed,
        selected_candidate_ids,
        observed_outcome_ids,
        steps_completed: state.completed_experiments,
        cost_spent,
        cumulative_expected_information_gain_nats,
        final_model_entropy_nats: entropy(&state.posterior),
        true_model_posterior,
        most_probable_model_id: most_probable_model_id.clone(),
        false_confident_selection: maximum_probability
            >= scenario.confident_model_probability_threshold
            && most_probable_model_id != scenario.true_model_id,
        myopic_oracle_regret_nats_per_cost,
        termination,
    })
}

/// Run all supplied policies on the same scenario and seed. This supplies a
/// paired-randomness comparison surface; population summaries and confidence
/// intervals should aggregate many scenarios and seeds outside this function.
pub fn run_paired_comparison(
    scenario: &SyntheticScenario,
    seed: u64,
    policies: &[BenchmarkPolicy],
) -> Result<PairedComparison, BenchmarkError> {
    let mut unique = BTreeSet::new();
    for policy in policies {
        if !unique.insert(*policy) {
            return Err(BenchmarkError::InvalidScenario(
                "duplicate policy in paired comparison",
            ));
        }
    }
    let results = policies
        .iter()
        .map(|policy| run_synthetic_episode(scenario, *policy, seed))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PairedComparison {
        scenario_id: scenario.scenario_id.clone(),
        seed,
        results,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::super::Feasibility;
    use super::*;

    fn posterior(p: f64) -> Posterior {
        BTreeMap::from([("model-a".to_owned(), p), ("model-b".to_owned(), 1.0 - p)])
    }

    fn candidate(id: &str, p_a_yes: f64, p_b_yes: f64, cost: f64) -> InterventionCandidate {
        InterventionCandidate {
            candidate_id: id.to_owned(),
            protocol_id: format!("protocol-{id}"),
            cost_unit_id: "experiment-credit".to_owned(),
            cost,
            feasibility: Feasibility::Permitted,
            causal_validity: super::super::CausalValidity {
                assignment_design_valid: true,
                delivery_fidelity: 1.0,
                delivery_identity_verified: true,
                interference_controlled: true,
                carryover_cleared: true,
            },
            outcomes: vec![
                super::super::OutcomeLikelihood {
                    outcome_id: "yes".to_owned(),
                    model_probabilities: BTreeMap::from([
                        ("model-a".to_owned(), p_a_yes),
                        ("model-b".to_owned(), p_b_yes),
                    ]),
                },
                super::super::OutcomeLikelihood {
                    outcome_id: "no".to_owned(),
                    model_probabilities: BTreeMap::from([
                        ("model-a".to_owned(), 1.0 - p_a_yes),
                        ("model-b".to_owned(), 1.0 - p_b_yes),
                    ]),
                },
            ],
        }
    }

    fn scenario(regime: u32, variant: u32) -> SyntheticScenario {
        let discriminating_probability = 0.60 + f64::from(variant) * 0.075;
        let weak_probability = 0.52 + f64::from(regime % 4) * 0.025;
        SyntheticScenario {
            scenario_id: format!("regime-{regime}-variant-{variant}"),
            initial_posterior: posterior(0.25 + f64::from(variant) * 0.10),
            ambiguity_posteriors: vec![posterior(0.5)],
            true_model_id: if regime.is_multiple_of(2) {
                "model-a".to_owned()
            } else {
                "model-b".to_owned()
            },
            candidates: vec![
                candidate(
                    "intervention-a",
                    discriminating_probability,
                    1.0 - discriminating_probability,
                    1.0,
                ),
                candidate(
                    "intervention-b",
                    weak_probability,
                    1.0 - weak_probability,
                    0.5,
                ),
            ],
            horizon: 8,
            cost_budget: 8.0,
            confident_model_probability_threshold: 0.95,
        }
    }

    #[test]
    fn paired_comparison_is_deterministic_and_respects_budget() {
        let policies = [
            BenchmarkPolicy::StaticOneShotRanking,
            BenchmarkPolicy::GreedySequentialEig,
            BenchmarkPolicy::CostNormalizedSequentialEig,
            BenchmarkPolicy::RobustMaximinSequentialEig,
            BenchmarkPolicy::RandomFeasible,
            BenchmarkPolicy::BalancedRoundRobin,
            BenchmarkPolicy::ClairvoyantMyopicUpperBound,
            BenchmarkPolicy::ScenarioTreeLookahead,
        ];
        for regime in 0..8 {
            for variant in 0..5 {
                let s = scenario(regime, variant);
                let left = run_paired_comparison(&s, 0x1234_5678, &policies).unwrap();
                let right = run_paired_comparison(&s, 0x1234_5678, &policies).unwrap();
                assert_eq!(left, right);
                for result in left.results {
                    assert!(result.steps_completed <= s.horizon);
                    assert!(result.cost_spent <= s.cost_budget + 1.0e-12);
                    assert_eq!(
                        result.selected_candidate_ids.len(),
                        result.steps_completed as usize
                    );
                    assert_eq!(
                        result.observed_outcome_ids.len(),
                        result.steps_completed as usize
                    );
                    assert!(result.true_model_posterior.is_finite());
                    assert!((0.0..=1.0).contains(&result.true_model_posterior));
                }
            }
        }
    }

    #[test]
    fn clairvoyant_myopic_upper_bound_has_zero_myopic_regret() {
        let s = scenario(0, 3);
        let result =
            run_synthetic_episode(&s, BenchmarkPolicy::ClairvoyantMyopicUpperBound, 77).unwrap();
        assert_eq!(result.myopic_oracle_regret_nats_per_cost, 0.0);
    }

    #[test]
    fn adaptive_selector_and_static_ranking_are_distinct_policies() {
        let s = scenario(0, 2);
        let static_run =
            run_synthetic_episode(&s, BenchmarkPolicy::StaticOneShotRanking, 0xabc).unwrap();
        let adaptive_run =
            run_synthetic_episode(&s, BenchmarkPolicy::GreedySequentialEig, 0xabc).unwrap();
        assert_eq!(static_run.seed, adaptive_run.seed);
        assert_eq!(static_run.policy, BenchmarkPolicy::StaticOneShotRanking);
        assert_eq!(adaptive_run.policy, BenchmarkPolicy::GreedySequentialEig);
    }

    #[test]
    fn mismatched_cost_units_are_rejected() {
        let mut s = scenario(0, 0);
        s.candidates[1].cost_unit_id = "seconds".to_owned();
        assert_eq!(
            run_synthetic_episode(&s, BenchmarkPolicy::GreedySequentialEig, 1),
            Err(BenchmarkError::CostUnitMismatch)
        );
    }

    #[test]
    fn scenario_tree_policy_runs_in_paired_comparison() {
        let s = scenario(2, 3);
        let policies = [
            BenchmarkPolicy::GreedySequentialEig,
            BenchmarkPolicy::ScenarioTreeLookahead,
        ];
        let result = run_paired_comparison(&s, 0x51a7, &policies).unwrap();
        assert_eq!(result.results.len(), 2);
        assert_eq!(
            result.results[1].policy,
            BenchmarkPolicy::ScenarioTreeLookahead
        );
        assert!(result.results[1].steps_completed <= s.horizon);
        assert!(result.results[1].cost_spent <= s.cost_budget + 1.0e-12);
    }

    #[test]
    fn duplicate_policy_ids_are_rejected_in_paired_comparison() {
        let s = scenario(0, 0);
        assert_eq!(
            run_paired_comparison(
                &s,
                1,
                &[
                    BenchmarkPolicy::GreedySequentialEig,
                    BenchmarkPolicy::GreedySequentialEig,
                ]
            ),
            Err(BenchmarkError::InvalidScenario(
                "duplicate policy in paired comparison"
            ))
        );
    }
}
