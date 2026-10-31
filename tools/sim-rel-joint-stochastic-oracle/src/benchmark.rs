//! Reproducible policy baseline runner for finite joint stochastic experiments.
//!
//! Baselines are constructed using the nominal model and public observation
//! history only, then evaluated unchanged across every model world.

use super::{
    JointKernel, JointMass, PolicyBranch, PolicyPlan, Rational,
    baseline::rank_by_one_step_eig,
    robust::{ModelWorld, RobustEvaluationReport, evaluate_policy_across_worlds},
    robust_optimizer::{RobustFrontierPoint, RobustSolverConfig, optimize_robust_worlds},
};
use std::collections::BTreeMap;

const MAX_BENCHMARK_HORIZON: usize = 12;
const MAX_POLICY_NODES: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BenchmarkConfig {
    pub horizon: usize,
    pub seed: u64,
    pub max_hazard: Rational,
    pub max_false_identification: Rational,
    pub max_abstention: Rational,
    pub max_memo_states: usize,
    pub max_generated_combinations: usize,
    pub max_frontier_points: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            horizon: 4,
            seed: 0x9e37_79b9_7f4a_7c15,
            max_hazard: Rational::zero(),
            max_false_identification: Rational::zero(),
            max_abstention: Rational::one(),
            max_memo_states: 100_000,
            max_generated_combinations: 1_000_000,
            max_frontier_points: 20_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodResult {
    pub method: String,
    pub evaluation: RobustEvaluationReport,
    pub satisfies_robust_budgets: bool,
    /// Exact additional worst-case cost over the robust optimum, when feasible.
    pub worst_case_cost_regret: Option<Rational>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkReport {
    pub nominal_world_id: String,
    pub horizon: usize,
    pub seed: u64,
    pub exact_robust_optimal: Option<RobustFrontierPoint>,
    pub exact_robust_optimal_worst_case_cost: Option<Rational>,
    pub methods: Vec<MethodResult>,
}

/// Build a fixed action-sequence policy whose next action never depends on
/// observation values. Its terminal MAP claim is based on the nominal posterior.
pub fn build_open_loop_policy(
    kernel: &JointKernel,
    action_sequence: &[usize],
) -> Result<PolicyPlan, String> {
    kernel.validate()?;
    if action_sequence.len() > MAX_BENCHMARK_HORIZON {
        return Err("open-loop sequence exceeds benchmark horizon cap".into());
    }
    if action_sequence.iter().any(|action| *action >= kernel.action_ids.len()) {
        return Err("open-loop sequence contains an invalid action".into());
    }
    let mut initial = kernel.initial_belief.clone();
    initial.sort();
    let mut budget = MAX_POLICY_NODES;
    build_sequence_node(kernel, &initial, action_sequence, &mut budget)
}

/// Build a receding-horizon greedy one-step EIG policy. At every public history
/// it recomputes the nominal joint posterior and selects the highest EIG/cost
/// action; it is deliberately myopic and is not the exact multi-step planner.
pub fn build_greedy_one_step_eig_policy(
    kernel: &JointKernel,
    horizon: usize,
) -> Result<PolicyPlan, String> {
    kernel.validate()?;
    if horizon > MAX_BENCHMARK_HORIZON {
        return Err("greedy EIG horizon exceeds benchmark cap".into());
    }
    let mut initial = kernel.initial_belief.clone();
    initial.sort();
    let mut budget = MAX_POLICY_NODES;
    build_greedy_eig_node(kernel, &initial, horizon, &mut budget)
}

/// Run fixed-order, seeded-random, static EIG-order and receding-horizon greedy
/// EIG baselines, then compare them against the exact robust shared-policy DP.
/// Every constructed policy is evaluated unchanged against the complete model family.
pub fn run_benchmark_suite(
    worlds: &[ModelWorld],
    nominal_world_id: &str,
    config: &BenchmarkConfig,
) -> Result<BenchmarkReport, String> {
    if worlds.is_empty() {
        return Err("benchmark requires at least one model world".into());
    }
    if config.horizon > MAX_BENCHMARK_HORIZON {
        return Err(format!("benchmark horizon exceeds {MAX_BENCHMARK_HORIZON}"));
    }
    validate_budget("hazard", config.max_hazard)?;
    validate_budget("false-identification", config.max_false_identification)?;
    validate_budget("abstention", config.max_abstention)?;

    let nominal = worlds
        .iter()
        .find(|world| world.id == nominal_world_id)
        .ok_or_else(|| "nominal world is not in the benchmark family".to_string())?;

    let optimizer_config = RobustSolverConfig {
        horizon: config.horizon,
        max_memo_states: config.max_memo_states,
        max_generated_combinations: config.max_generated_combinations,
        max_frontier_points: config.max_frontier_points,
    };
    let exact = optimize_robust_worlds(worlds, nominal_world_id, &optimizer_config)?;
    let exact_selected = exact.minimum_worst_case_cost_under(
        config.max_hazard,
        config.max_false_identification,
        config.max_abstention,
    )?;
    let exact_cost = exact_selected.map(RobustFrontierPoint::worst_case_expected_cost);

    let fixed_sequence: Vec<usize> = (0..config.horizon)
        .map(|step| step % nominal.kernel.action_ids.len())
        .collect();
    let random_sequence = seeded_random_sequence(
        nominal.kernel.action_ids.len(),
        config.horizon,
        config.seed,
    );
    let eig_scores = rank_by_one_step_eig(
        &nominal.kernel,
        &nominal.kernel.initial_belief,
    )?;
    let mut eig_order: Vec<usize> = eig_scores
        .iter()
        .map(|score| score.action_index)
        .collect();
    if eig_order.is_empty() {
        return Err("one-step EIG returned no action ranking".into());
    }
    let static_eig_sequence: Vec<usize> = (0..config.horizon)
        .map(|step| eig_order[step % eig_order.len()])
        .collect();
    // Keep the ranking owned by this exact benchmark instance; do not mutate
    // action identity or leak any latent state to the policy.
    eig_order.clear();

    let plans = vec![
        (
            "fixed-order-open-loop".to_string(),
            build_open_loop_policy(&nominal.kernel, &fixed_sequence)?,
        ),
        (
            "seeded-random-open-loop".to_string(),
            build_open_loop_policy(&nominal.kernel, &random_sequence)?,
        ),
        (
            "static-one-step-EIG-order".to_string(),
            build_open_loop_policy(&nominal.kernel, &static_eig_sequence)?,
        ),
        (
            "receding-horizon-greedy-one-step-EIG".to_string(),
            build_greedy_one_step_eig_policy(&nominal.kernel, config.horizon)?,
        ),
    ];

    let mut methods = Vec::with_capacity(plans.len());
    for (method, policy) in plans {
        let evaluation = evaluate_policy_across_worlds(
            worlds,
            nominal_world_id,
            &policy,
            config.horizon,
        )?;
        let satisfies_robust_budgets = evaluation.satisfies_budgets(
            config.max_hazard,
            config.max_false_identification,
            config.max_abstention,
        )?;
        let worst_case_cost_regret = match (satisfies_robust_budgets, exact_cost) {
            (true, Some(optimal)) => Some(
                evaluation
                    .worst_case_expected_cost
                    .checked_sub(optimal)
                    .map_err(|_| "baseline cost is below exact robust optimum".to_string())?,
            ),
            _ => None,
        };
        methods.push(MethodResult {
            method,
            evaluation,
            satisfies_robust_budgets,
            worst_case_cost_regret,
        });
    }

    Ok(BenchmarkReport {
        nominal_world_id: nominal_world_id.to_string(),
        horizon: config.horizon,
        seed: config.seed,
        exact_robust_optimal: exact_selected.cloned(),
        exact_robust_optimal_worst_case_cost: exact_cost,
        methods,
    })
}

fn validate_budget(name: &str, value: Rational) -> Result<(), String> {
    if !value.is_probability() {
        return Err(format!("{name} budget must lie in [0, 1]"));
    }
    Ok(())
}

fn build_sequence_node(
    kernel: &JointKernel,
    belief: &[JointMass],
    sequence: &[usize],
    budget: &mut usize,
) -> Result<PolicyPlan, String> {
    consume_node_budget(budget)?;
    let Some((&action, remaining)) = sequence.split_first() else {
        return Ok(map_claim(belief, kernel.hypothesis_ids.len()));
    };

    let posteriors = observation_posteriors(kernel, belief, action)?;
    let mut branches = Vec::with_capacity(kernel.observation_ids.len());
    for observation in 0..kernel.observation_ids.len() {
        let continuation = match posteriors.get(&observation) {
            Some(posterior) => build_sequence_node(kernel, posterior, remaining, budget)?,
            None => PolicyPlan::Abstain,
        };
        branches.push(PolicyBranch {
            observation,
            plan: Box::new(continuation),
        });
    }
    Ok(PolicyPlan::Action {
        action,
        branches,
    })
}

fn build_greedy_eig_node(
    kernel: &JointKernel,
    belief: &[JointMass],
    remaining: usize,
    budget: &mut usize,
) -> Result<PolicyPlan, String> {
    consume_node_budget(budget)?;
    if remaining == 0 {
        return Ok(map_claim(belief, kernel.hypothesis_ids.len()));
    }
    let scores = rank_by_one_step_eig(kernel, belief)?;
    let action = scores
        .first()
        .map(|score| score.action_index)
        .ok_or_else(|| "greedy one-step EIG found no available action".to_string())?;
    let posteriors = observation_posteriors(kernel, belief, action)?;
    let mut branches = Vec::with_capacity(kernel.observation_ids.len());
    for observation in 0..kernel.observation_ids.len() {
        let continuation = match posteriors.get(&observation) {
            Some(posterior) => {
                build_greedy_eig_node(kernel, posterior, remaining - 1, budget)?
            }
            None => PolicyPlan::Abstain,
        };
        branches.push(PolicyBranch {
            observation,
            plan: Box::new(continuation),
        });
    }
    Ok(PolicyPlan::Action {
        action,
        branches,
    })
}

fn consume_node_budget(budget: &mut usize) -> Result<(), String> {
    if *budget == 0 {
        return Err("baseline policy node budget exceeded".into());
    }
    *budget -= 1;
    Ok(())
}

fn map_claim(belief: &[JointMass], hypothesis_count: usize) -> PolicyPlan {
    let mut masses = vec![Rational::zero(); hypothesis_count];
    for point in belief {
        masses[point.hypothesis] = masses[point.hypothesis]
            .checked_add(point.mass)
            .expect("validated normalized belief rational addition is representable");
    }
    let mut best = None;
    for (hypothesis, mass) in masses.into_iter().enumerate() {
        if best.is_none_or(|(_, best_mass)| mass > best_mass) {
            best = Some((hypothesis, mass));
        }
    }
    best.map_or(PolicyPlan::Abstain, |(hypothesis, _)| {
        PolicyPlan::Claim { hypothesis }
    })
}

fn observation_posteriors(
    kernel: &JointKernel,
    belief: &[JointMass],
    action: usize,
) -> Result<BTreeMap<usize, Vec<JointMass>>, String> {
    let mut by_observation: BTreeMap<
        usize,
        BTreeMap<(usize, usize, usize), Rational>,
    > = BTreeMap::new();
    for point in belief {
        let row_index = kernel.row_index(
            point.hypothesis,
            point.physical_state,
            point.sensor_state,
            action,
        );
        for outcome in &kernel.rows[row_index].outcomes {
            if let Some(observation) = outcome.observation {
                let weighted = point.mass.checked_mul(outcome.probability)?;
                let key = (
                    point.hypothesis,
                    outcome.next_physical_state,
                    outcome.next_sensor_state,
                );
                let states = by_observation.entry(observation).or_default();
                let previous = states.get(&key).copied().unwrap_or(Rational::zero());
                states.insert(key, previous.checked_add(weighted)?);
            }
        }
    }

    let mut result = BTreeMap::new();
    for (observation, states) in by_observation {
        let probability = states.values().try_fold(
            Rational::zero(),
            |sum, mass| sum.checked_add(*mass),
        )?;
        if probability.is_zero() {
            return Err("baseline produced a zero-probability observation posterior".into());
        }
        let posterior = states
            .into_iter()
            .map(|((hypothesis, physical_state, sensor_state), mass)| {
                Ok(JointMass {
                    hypothesis,
                    physical_state,
                    sensor_state,
                    mass: mass.checked_div(probability)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        result.insert(observation, posterior);
    }
    Ok(result)
}

fn seeded_random_sequence(
    action_count: usize,
    horizon: usize,
    seed: u64,
) -> Vec<usize> {
    let mut order: Vec<usize> = (0..action_count).collect();
    let mut state = if seed == 0 { 0xa076_1d64_78bd_642f } else { seed };
    for index in (1..order.len()).rev() {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let random = state.wrapping_mul(0x2545_f491_4f6c_dd1d);
        order.swap(index, (random as usize) % (index + 1));
    }
    if order.is_empty() {
        return Vec::new();
    }
    (0..horizon)
        .map(|step| order[step % order.len()])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        JointOutcome, KernelRow,
        robust_optimizer::RobustSolverConfig,
    };

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn calibration_kernel() -> JointKernel {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            for action in 0..2 {
                for physical in 0..2 {
                    for sensor in 0..2 {
                        let (next_physical, next_sensor, observation) = if action == 0 {
                            (physical, sensor, physical ^ sensor)
                        } else {
                            (physical, 0, 0)
                        };
                        rows.push(KernelRow {
                            hypothesis,
                            physical_state: physical,
                            sensor_state: sensor,
                            action,
                            outcomes: vec![JointOutcome {
                                probability: Rational::one(),
                                next_physical_state: next_physical,
                                next_sensor_state: next_sensor,
                                observation: Some(observation),
                            }],
                        });
                    }
                }
            }
        }
        JointKernel {
            hypothesis_ids: vec!["h0".into(), "h1".into()],
            physical_state_count: 2,
            sensor_state_count: 2,
            action_ids: vec!["measure-combined-signal".into(), "calibrate-sensor".into()],
            observation_ids: vec!["zero".into(), "one".into()],
            action_costs: vec![1, 1],
            initial_belief: vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 1,
                    mass: half(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 1,
                    sensor_state: 0,
                    mass: half(),
                },
            ],
            rows,
        }
    }

    #[test]
    fn open_loop_fixed_order_cannot_value_calibration_before_measurement() {
        let kernel = calibration_kernel();
        let policy = build_open_loop_policy(&kernel, &[0, 1]).unwrap();
        let result = super::super::evaluate_policy(&kernel, &policy, 2).unwrap();
        assert_eq!(result.correct_identification_probability, Rational::zero());
        assert_eq!(result.abstention_probability, Rational::one());
    }

    #[test]
    fn greedy_one_step_eig_prefers_first_action_on_zero_eig_tie() {
        let kernel = calibration_kernel();
        let policy = build_greedy_one_step_eig_policy(&kernel, 2).unwrap();
        match &policy {
            PolicyPlan::Action { action, branches } => {
                assert_eq!(*action, 0);
                let zero = branches.iter().find(|branch| branch.observation == 1).unwrap();
                assert!(matches!(zero.plan.as_ref(), PolicyPlan::Action { action: 0, .. }));
            }
            _ => panic!("expected a greedy action"),
        }
        let result = super::super::evaluate_policy(&kernel, &policy, 2).unwrap();
        assert_eq!(result.abstention_probability, Rational::one());
    }

    #[test]
    fn static_eig_order_and_exact_lookahead_are_separately_evaluated() {
        let kernel = calibration_kernel();
        let worlds = vec![ModelWorld {
            id: "nominal".into(),
            kernel: kernel.clone(),
        }];
        let report = run_benchmark_suite(
            &worlds,
            "nominal",
            &BenchmarkConfig {
                horizon: 2,
                max_hazard: Rational::zero(),
                max_false_identification: Rational::zero(),
                max_abstention: Rational::zero(),
                ..BenchmarkConfig::default()
            },
        ).unwrap();
        let exact = report.exact_robust_optimal.as_ref().unwrap();
        assert_eq!(exact.worst_case_expected_cost(), Rational::from_u64(2));
        let greedy = report.methods.iter()
            .find(|method| method.method == "receding-horizon-greedy-one-step-EIG")
            .unwrap();
        assert!(!greedy.satisfies_robust_budgets);
        assert_eq!(greedy.evaluation.nominal.abstention_probability, Rational::one());
    }

    #[test]
    fn random_action_sequence_is_reproducible_and_seed_sensitive() {
        let a = seeded_random_sequence(7, 12, 1234);
        let b = seeded_random_sequence(7, 12, 1234);
        let c = seeded_random_sequence(7, 12, 4321);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 12);
    }

    #[test]
    fn benchmark_suite_reports_exact_budgeted_regret_when_feasible() {
        let kernel = calibration_kernel();
        let worlds = vec![ModelWorld {
            id: "nominal".into(),
            kernel,
        }];
        let report = run_benchmark_suite(
            &worlds,
            "nominal",
            &BenchmarkConfig {
                horizon: 2,
                max_hazard: Rational::zero(),
                max_false_identification: Rational::zero(),
                max_abstention: Rational::zero(),
                ..BenchmarkConfig::default()
            },
        ).unwrap();
        let optimum = report.exact_robust_optimal_worst_case_cost.unwrap();
        assert_eq!(optimum, Rational::from_u64(2));
        assert!(report.methods.iter().any(|method| {
            method.satisfies_robust_budgets
                && method.worst_case_cost_regret == Some(Rational::zero())
        }));
    }

    #[test]
    fn invalid_benchmark_budgets_horizons_and_action_sequences_fail_closed() {
        let kernel = calibration_kernel();
        assert!(build_open_loop_policy(&kernel, &[99]).is_err());
        assert!(build_greedy_one_step_eig_policy(&kernel, MAX_BENCHMARK_HORIZON + 1).is_err());
        let worlds = vec![ModelWorld { id: "nominal".into(), kernel }];
        assert!(run_benchmark_suite(
            &worlds,
            "nominal",
            &BenchmarkConfig { horizon: MAX_BENCHMARK_HORIZON + 1, ..BenchmarkConfig::default() },
        ).is_err());
        assert!(run_benchmark_suite(
            &worlds,
            "nominal",
            &BenchmarkConfig { max_hazard: Rational::new(2, 1).unwrap(), ..BenchmarkConfig::default() },
        ).is_err());
    }
}
