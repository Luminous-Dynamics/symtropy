//! Deterministic counterexample-ready scientific experiment benchmark generator.
//! Every emitted task must pass the exact optimizer, one-step EIG baseline,
//! and all-horizon identifiability analysis before it is admitted.

use sim_rel_joint_stochastic_oracle::{
    JointKernel, JointMass, JointOutcome, KernelRow, PolicyPlan, Rational,
    SolverConfig, optimize_joint_kernel,
    baseline::{OneStepEigScore, rank_by_one_step_eig},
    identifiability::{IdentifiabilityConfig, IdentifiabilityReport, analyze_identifiability},
};
use std::collections::BTreeSet;

pub const GENERATOR_VERSION: &str = "sim-rel-018-v0.2.0";
pub const TASK_HORIZON: usize = 2;
pub const MAX_TASKS_PER_FAMILY: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TaskFamily {
    SensorPhysicalConfounding,
    NoisySensorConfounding,
    PhysicalCarryover,
    NonIdentifiable,
}

impl TaskFamily {
    pub const ALL: [Self; 4] = [
        Self::SensorPhysicalConfounding,
        Self::NoisySensorConfounding,
        Self::PhysicalCarryover,
        Self::NonIdentifiable,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::SensorPhysicalConfounding => "sensor-physical-confounding",
            Self::NoisySensorConfounding => "noisy-sensor-confounding",
            Self::PhysicalCarryover => "physical-carryover",
            Self::NonIdentifiable => "non-identifiable",
        }
    }

    const fn code(self) -> u64 {
        match self {
            Self::SensorPhysicalConfounding => 0x51A7_0001,
            Self::NoisySensorConfounding => 0x51A7_0004,
            Self::PhysicalCarryover => 0x51A7_0002,
            Self::NonIdentifiable => 0x51A7_0003,
        }
    }

    const fn requires_identification(self) -> bool {
        !matches!(self, Self::NonIdentifiable)
    }

    fn max_false_identification_probability(self) -> Result<Rational, String> {
        match self {
            Self::NoisySensorConfounding => Rational::new(1, 4),
            _ => Ok(Rational::zero()),
        }
    }
}

/// Small deterministic generator with no external RNG or hidden entropy.
#[derive(Clone, Debug)]
struct SeededRng {
    state: u64,
}

impl SeededRng {
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

    fn between(&mut self, minimum: u64, maximum: u64) -> u64 {
        minimum + self.next_u64() % (maximum - minimum + 1)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkReceipt {
    pub generator_version: String,
    pub task_id: String,
    pub seed: u64,
    pub family: TaskFamily,
    pub horizon: usize,
    pub exact_frontier_points: usize,
    pub budget_feasible_expected_cost: Option<Rational>,
    pub budget_feasible_policy: Option<PolicyPlan>,
    pub one_step_baseline: Vec<OneStepEigScore>,
    pub equivalence_classes: Vec<Vec<String>>,
    pub equivalence_certificate_count: usize,
    pub distinguishing_witness_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedTask {
    pub receipt: BenchmarkReceipt,
    pub kernel: JointKernel,
    pub identifiability: IdentifiabilityReport,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedSuite {
    pub generator_version: String,
    pub root_seed: u64,
    pub tasks: Vec<GeneratedTask>,
}

/// Generate one task and reject it unless all declared scientific properties hold.
pub fn generate_task(family: TaskFamily, seed: u64) -> Result<GeneratedTask, String> {
    let mut rng = SeededRng::new(seed ^ family.code());
    let left_weight = u128::from(rng.between(1, 4));
    let right_weight = u128::from(rng.between(1, 4));
    let total_weight = left_weight
        .checked_add(right_weight)
        .ok_or_else(|| "generated prior mass overflow".to_string())?;
    let prior = if family == TaskFamily::NoisySensorConfounding {
        // Keep the prior balanced so the 1/4 false-claim budget cannot be
        // satisfied by a zero-cost prior-only guess; the calibrated noisy
        // measurement is the actual information-bearing sequence.
        [Rational::new(1, 2)?, Rational::new(1, 2)?]
    } else {
        [
            Rational::new(left_weight, total_weight)?,
            Rational::new(right_weight, total_weight)?,
        ]
    };
    let primary_cost = rng.between(1, 4);
    let secondary_cost = rng.between(1, 4);
    let kernel = build_kernel(family, prior, primary_cost, secondary_cost)?;
    kernel.validate()?;

    let optimization = optimize_joint_kernel(
        &kernel,
        &SolverConfig { horizon: TASK_HORIZON, ..SolverConfig::default() },
    )?;
    if optimization.frontier.is_empty() {
        return Err("exact optimizer returned an empty task frontier".into());
    }

    let false_identification_budget = family.max_false_identification_probability()?;
    let feasible = optimization
        .minimum_cost_under(Rational::zero(), false_identification_budget, Rational::zero())?
        .cloned();
    if family.requires_identification() != feasible.is_some() {
        return Err(format!("family {} violates its budget-feasibility contract", family.id()));
    }

    let eig = rank_by_one_step_eig(&kernel, &kernel.initial_belief)?;
    if eig.len() != kernel.action_ids.len() {
        return Err("one-step EIG baseline did not score every action".into());
    }
    if family.requires_identification()
        && eig.iter().any(|score| score.expected_information_gain_nats.abs() > 1e-12)
    {
        return Err("sequential challenge unexpectedly has immediate one-step information".into());
    }

    let identifiability = analyze_identifiability(&kernel, &IdentifiabilityConfig::default())?;
    let all_equivalent = identifiability.equivalence_classes.len() == 1;
    if all_equivalent == family.requires_identification() {
        return Err(format!("family {} violates its identifiability contract", family.id()));
    }

    let equivalence_certificate_count = identifiability.pairwise.iter().filter(|pair| {
        matches!(
            &pair.classification,
            sim_rel_joint_stochastic_oracle::identifiability::PairClassification::Equivalent(_)
        )
    }).count();
    let distinguishing_witness_count = identifiability.pairwise.len()
        - equivalence_certificate_count;

    let task_id = format!("sim-rel-018-{}-{:016x}", family.id(), seed);
    let receipt = BenchmarkReceipt {
        generator_version: GENERATOR_VERSION.to_string(),
        task_id,
        seed,
        family,
        horizon: TASK_HORIZON,
        exact_frontier_points: optimization.frontier.len(),
        budget_feasible_expected_cost: feasible.as_ref().map(|point| point.expected_experiment_cost),
        budget_feasible_policy: feasible.map(|point| point.policy),
        one_step_baseline: eig,
        equivalence_classes: identifiability.equivalence_classes.clone(),
        equivalence_certificate_count,
        distinguishing_witness_count,
    };
    let task = GeneratedTask { receipt, kernel, identifiability };
    verify_generated_task(&task)?;
    Ok(task)
}

/// Generate a deterministic, balanced suite with one reproducible seed per task.
pub fn generate_suite(root_seed: u64, per_family: usize) -> Result<GeneratedSuite, String> {
    if per_family == 0 || per_family > MAX_TASKS_PER_FAMILY {
        return Err(format!("per-family task count must be in 1..={MAX_TASKS_PER_FAMILY}"));
    }
    let mut tasks = Vec::with_capacity(TaskFamily::ALL.len() * per_family);
    let mut task_ids = BTreeSet::new();
    for family in TaskFamily::ALL {
        for index in 0..per_family {
            let seed = derive_seed(root_seed, family, index);
            let task = generate_task(family, seed)?;
            if !task_ids.insert(task.receipt.task_id.clone()) {
                return Err("generator emitted a duplicate task ID".into());
            }
            tasks.push(task);
        }
    }
    Ok(GeneratedSuite {
        generator_version: GENERATOR_VERSION.to_string(),
        root_seed,
        tasks,
    })
}

/// Recompute exact receipts and reject tasks that have been altered.
pub fn verify_generated_task(task: &GeneratedTask) -> Result<(), String> {
    let receipt = &task.receipt;
    if receipt.generator_version != GENERATOR_VERSION
        || receipt.horizon != TASK_HORIZON
        || receipt.task_id != format!("sim-rel-018-{}-{:016x}", receipt.family.id(), receipt.seed)
    {
        return Err("task receipt version, ID or horizon mismatch".into());
    }
    task.kernel.validate()?;

    let optimization = optimize_joint_kernel(
        &task.kernel,
        &SolverConfig { horizon: receipt.horizon, ..SolverConfig::default() },
    )?;
    if optimization.frontier.len() != receipt.exact_frontier_points {
        return Err("exact frontier receipt did not replay".into());
    }
    let false_identification_budget = receipt.family.max_false_identification_probability()?;
    let feasible = optimization
        .minimum_cost_under(Rational::zero(), false_identification_budget, Rational::zero())?
        .cloned();
    if feasible.as_ref().map(|point| point.expected_experiment_cost)
        != receipt.budget_feasible_expected_cost
        || feasible.as_ref().map(|point| point.policy.clone()) != receipt.budget_feasible_policy
    {
        return Err("budget-feasibility receipt did not replay".into());
    }

    let eig = rank_by_one_step_eig(&task.kernel, &task.kernel.initial_belief)?;
    if eig != receipt.one_step_baseline {
        return Err("one-step baseline receipt did not replay".into());
    }

    let ident = analyze_identifiability(&task.kernel, &IdentifiabilityConfig::default())?;
    if ident != task.identifiability
        || ident.equivalence_classes != receipt.equivalence_classes
    {
        return Err("identifiability receipt did not replay".into());
    }
    let feasible_is_identifiable = feasible.is_some();
    if receipt.family.requires_identification() != feasible_is_identifiable {
        return Err("task family budget-feasibility contract did not replay".into());
    }
    if receipt.family.requires_identification()
        && eig.iter().any(|score| score.expected_information_gain_nats.abs() > 1e-12)
    {
        return Err("sequential task family has immediate one-step information".into());
    }
    let certificate_count = ident.pairwise.iter().filter(|pair| {
        matches!(
            &pair.classification,
            sim_rel_joint_stochastic_oracle::identifiability::PairClassification::Equivalent(_)
        )
    }).count();
    let witness_count = ident.pairwise.len() - certificate_count;
    if certificate_count != receipt.equivalence_certificate_count
        || witness_count != receipt.distinguishing_witness_count
    {
        return Err("identifiability receipt counts did not replay".into());
    }
    Ok(())
}

fn derive_seed(root_seed: u64, family: TaskFamily, index: usize) -> u64 {
    let mixed = root_seed
        ^ family.code()
        ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    SeededRng::new(mixed).next_u64()
}

fn build_kernel(
    family: TaskFamily,
    prior: [Rational; 2],
    primary_cost: u64,
    secondary_cost: u64,
) -> Result<JointKernel, String> {
    let (physical_state_count, sensor_state_count, action_ids, observation_ids, action_costs) =
        match family {
            TaskFamily::SensorPhysicalConfounding | TaskFamily::NoisySensorConfounding => (
                2, 2, vec!["measure", "calibrate"], vec!["zero", "one"],
                vec![primary_cost, secondary_cost],
            ),
            TaskFamily::PhysicalCarryover => (
                2, 1, vec!["read", "condition"], vec!["zero", "one"],
                vec![primary_cost, secondary_cost],
            ),
            TaskFamily::NonIdentifiable => (
                2, 2, vec!["read", "cycle"], vec!["constant"],
                vec![primary_cost, secondary_cost],
            ),
        };

    let initial_belief = match family {
        TaskFamily::SensorPhysicalConfounding | TaskFamily::NoisySensorConfounding => vec![
            JointMass { hypothesis: 0, physical_state: 0, sensor_state: 1, mass: prior[0] },
            JointMass { hypothesis: 1, physical_state: 1, sensor_state: 0, mass: prior[1] },
        ],
        TaskFamily::PhysicalCarryover => vec![
            JointMass { hypothesis: 0, physical_state: 0, sensor_state: 0, mass: prior[0] },
            JointMass { hypothesis: 1, physical_state: 0, sensor_state: 0, mass: prior[1] },
        ],
        TaskFamily::NonIdentifiable => vec![
            JointMass { hypothesis: 0, physical_state: 0, sensor_state: 0, mass: prior[0] },
            JointMass { hypothesis: 1, physical_state: 1, sensor_state: 1, mass: prior[1] },
        ],
    };

    let mut rows = Vec::new();
    for hypothesis in 0..2 {
        for physical_state in 0..physical_state_count {
            for sensor_state in 0..sensor_state_count {
                for action in 0..action_ids.len() {
                    rows.push(KernelRow {
                        hypothesis,
                        physical_state,
                        sensor_state,
                        action,
                        outcomes: outcomes_for(
                            family, hypothesis, physical_state, sensor_state, action,
                        ),
                    });
                }
            }
        }
    }

    Ok(JointKernel {
        hypothesis_ids: vec!["mechanism-a".into(), "mechanism-b".into()],
        physical_state_count,
        sensor_state_count,
        action_ids: action_ids.into_iter().map(str::to_string).collect(),
        observation_ids: observation_ids.into_iter().map(str::to_string).collect(),
        action_costs,
        initial_belief,
        rows,
    })
}

fn outcomes_for(
    family: TaskFamily,
    hypothesis: usize,
    physical_state: usize,
    sensor_state: usize,
    action: usize,
) -> Vec<JointOutcome> {
    match family {
        TaskFamily::SensorPhysicalConfounding if action == 0 => vec![safe(
            Rational::one(), physical_state, sensor_state, physical_state ^ sensor_state,
        )],
        TaskFamily::SensorPhysicalConfounding => vec![safe(
            Rational::one(), physical_state, 0, 0,
        )],
        TaskFamily::NoisySensorConfounding if action == 0 => {
            let latent_signal = physical_state ^ sensor_state;
            let correct = Rational::new(3, 4).expect("fixed 3/4 probability is valid");
            let error = Rational::new(1, 4).expect("fixed 1/4 probability is valid");
            if latent_signal == 0 {
                vec![
                    safe(correct, physical_state, sensor_state, 0),
                    safe(error, physical_state, sensor_state, 1),
                ]
            } else {
                vec![
                    safe(error, physical_state, sensor_state, 0),
                    safe(correct, physical_state, sensor_state, 1),
                ]
            }
        }
        TaskFamily::NoisySensorConfounding => vec![safe(
            Rational::one(), physical_state, 0, 0,
        )],
        TaskFamily::PhysicalCarryover if action == 0 => vec![safe(
            Rational::one(), physical_state, sensor_state, physical_state,
        )],
        TaskFamily::PhysicalCarryover => vec![safe(
            Rational::one(), hypothesis, sensor_state, 0,
        )],
        TaskFamily::NonIdentifiable if action == 0 => vec![safe(
            Rational::one(), physical_state, sensor_state, 0,
        )],
        TaskFamily::NonIdentifiable => vec![safe(
            Rational::one(), 1 - physical_state, 1 - sensor_state, 0,
        )],
    }
}

fn safe(
    probability: Rational,
    next_physical_state: usize,
    next_sensor_state: usize,
    observation: usize,
) -> JointOutcome {
    JointOutcome {
        probability,
        next_physical_state,
        next_sensor_state,
        observation: Some(observation),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy_actions(policy: &PolicyPlan, kernel: &JointKernel) -> Vec<String> {
        let mut actions = Vec::new();
        let mut node = policy;
        while let PolicyPlan::Action { action, branches } = node {
            actions.push(kernel.action_ids[*action].clone());
            let Some(branch) = branches.first() else { break };
            node = &branch.plan;
        }
        actions
    }

    #[test]
    fn sensor_confounding_requires_calibration_then_measurement() {
        for seed in 0..16 {
            let task = generate_task(TaskFamily::SensorPhysicalConfounding, seed).unwrap();
            let policy = task.receipt.budget_feasible_policy.as_ref().unwrap();
            assert_eq!(policy_actions(policy, &task.kernel), vec!["calibrate".to_string(), "measure".to_string()]);
            assert!(task.receipt.one_step_baseline.iter().all(|score| {
                score.expected_information_gain_nats.abs() < 1e-12
            }));
        }
    }


    #[test]
    fn stochastic_sensor_confounding_has_exact_bounded_false_claim_risk() {
        for seed in 0..16 {
            let task = generate_task(TaskFamily::NoisySensorConfounding, seed).unwrap();
            let policy = task.receipt.budget_feasible_policy.as_ref().unwrap();
            assert_eq!(
                policy_actions(policy, &task.kernel),
                vec!["calibrate".to_string(), "measure".to_string()]
            );
            let selected = optimize_joint_kernel(
                &task.kernel,
                &SolverConfig { horizon: TASK_HORIZON, ..SolverConfig::default() },
            )
            .unwrap()
            .minimum_cost_under(
                Rational::zero(),
                Rational::new(1, 4).unwrap(),
                Rational::zero(),
            )
            .unwrap()
            .unwrap();
            assert_eq!(selected.false_identification_probability, Rational::new(1, 4).unwrap());
            assert_eq!(selected.correct_identification_probability, Rational::new(3, 4).unwrap());
            assert!(task.receipt.one_step_baseline.iter().all(|score| {
                score.expected_information_gain_nats.abs() < 1e-12
            }));
        }
    }

    #[test]
    fn physical_carryover_requires_condition_then_read() {
        for seed in 0..16 {
            let task = generate_task(TaskFamily::PhysicalCarryover, seed).unwrap();
            let policy = task.receipt.budget_feasible_policy.as_ref().unwrap();
            assert_eq!(policy_actions(policy, &task.kernel), vec!["condition".to_string(), "read".to_string()]);
            assert!(task.receipt.one_step_baseline.iter().all(|score| {
                score.expected_information_gain_nats.abs() < 1e-12
            }));
        }
    }

    #[test]
    fn nonidentifiable_controls_have_equivalence_certificates_and_no_strict_policy() {
        for seed in 0..16 {
            let task = generate_task(TaskFamily::NonIdentifiable, seed).unwrap();
            assert!(task.receipt.budget_feasible_policy.is_none());
            assert_eq!(
                task.identifiability.equivalence_classes,
                vec![vec!["mechanism-a".to_string(), "mechanism-b".to_string()]],
            );
            assert_eq!(task.receipt.equivalence_certificate_count, 1);
        }
    }

    #[test]
    fn suite_is_seed_deterministic_and_all_receipts_replay() {
        let first = generate_suite(0xA11C_E55, 4).unwrap();
        let second = generate_suite(0xA11C_E55, 4).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.tasks.len(), TaskFamily::ALL.len() * 4);
        for task in &first.tasks {
            verify_generated_task(task).unwrap();
        }
    }

    #[test]
    fn suite_size_caps_and_seed_extremes_fail_safely() {
        assert!(generate_suite(1, 0).is_err());
        assert!(generate_suite(1, MAX_TASKS_PER_FAMILY + 1).is_err());
        assert!(generate_task(TaskFamily::NonIdentifiable, u64::MAX).is_ok());
    }

    #[test]
    fn accepted_task_families_produce_nontrivial_exact_frontiers() {
        let suite = generate_suite(0x1234_5678, 8).unwrap();
        assert!(suite.tasks.iter().all(|task| task.receipt.exact_frontier_points >= 2));
    }
}
