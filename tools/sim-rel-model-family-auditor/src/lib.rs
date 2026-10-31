//! Exact public-trace support audit across persistent joint stochastic worlds.
//!
//! Exact zero likelihood eliminates a world. A rare nonzero rational likelihood
//! remains supported. The model set is declared out-of-family only when every
//! supplied complete world is eliminated by an observed prefix.

use sim_rel_joint_stochastic_oracle::{
    JointMass, PolicyPlan, Rational,
    robust::{ModelWorld, evaluate_policy_across_worlds},
};
use std::collections::BTreeMap;

pub const MAX_TRACE_STEPS: usize = 4096;
pub const MAX_MODEL_WORLDS: usize = 256;
pub const MAX_POSTERIOR_TUPLES: usize = 65_536;
pub const MAX_TRACE_KERNEL_OUTCOME_APPLICATIONS: usize = 10_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservedOutcome {
    Observation { id: String },
    Hazard,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedStep {
    pub action_id: String,
    pub outcome: ObservedOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldTraceAudit {
    pub world_id: String,
    pub trace_likelihood: Rational,
    pub conditional_step_probabilities: Vec<Rational>,
    pub prefix_likelihoods: Vec<Rational>,
    pub first_zero_prefix: Option<usize>,
    pub terminal_hazard: bool,
    pub final_joint_posterior: Option<Vec<JointMass>>,
}

impl WorldTraceAudit {
    pub const fn supports_trace(&self) -> bool {
        self.first_zero_prefix.is_none()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FamilyTraceStatus {
    WithinDeclaredFamily,
    OutOfDeclaredFamily { first_empty_prefix: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelFamilyAudit {
    pub status: FamilyTraceStatus,
    pub observed_steps: usize,
    pub supported_world_ids: Vec<String>,
    pub eliminated_world_ids: Vec<String>,
    pub per_world: Vec<WorldTraceAudit>,
}

pub fn audit_trace_against_worlds(
    worlds: &[ModelWorld],
    trace: &[ObservedStep],
) -> Result<ModelFamilyAudit, String> {
    if worlds.is_empty() || worlds.len() > MAX_MODEL_WORLDS {
        return Err(format!("model-world count must be in 1..={MAX_MODEL_WORLDS}"));
    }
    if trace.len() > MAX_TRACE_STEPS {
        return Err(format!("trace exceeds the {MAX_TRACE_STEPS}-step cap"));
    }

    // Reuse the canonical SIM-REL-015 validation for world identity and semantics.
    evaluate_policy_across_worlds(worlds, &worlds[0].id, &PolicyPlan::Abstain, 0)?;

    let mut kernel_outcome_applications = 0_usize;
    let mut per_world = Vec::with_capacity(worlds.len());
    let mut supported_world_ids = Vec::new();
    let mut eliminated_world_ids = Vec::new();
    for world in worlds {
        let result = replay_world(world, trace, &mut kernel_outcome_applications)?;
        if result.supports_trace() {
            supported_world_ids.push(world.id.clone());
        } else {
            eliminated_world_ids.push(world.id.clone());
        }
        per_world.push(result);
    }

    let status = if supported_world_ids.is_empty() {
        let first_empty_prefix = per_world
            .iter()
            .filter_map(|result| result.first_zero_prefix)
            .max()
            .ok_or_else(|| "empty family has no falsification prefix".to_string())?;
        FamilyTraceStatus::OutOfDeclaredFamily { first_empty_prefix }
    } else {
        FamilyTraceStatus::WithinDeclaredFamily
    };

    Ok(ModelFamilyAudit {
        status,
        observed_steps: trace.len(),
        supported_world_ids,
        eliminated_world_ids,
        per_world,
    })
}

fn replay_world(
    world: &ModelWorld,
    trace: &[ObservedStep],
    kernel_outcome_applications: &mut usize,
) -> Result<WorldTraceAudit, String> {
    let kernel = &world.kernel;
    let mut posterior = kernel.initial_belief.clone();
    posterior.sort();
    if posterior.len() > MAX_POSTERIOR_TUPLES {
        return Err("initial joint posterior exceeds tuple cap".into());
    }

    let mut trace_likelihood = Rational::one();
    let mut conditional_step_probabilities = Vec::with_capacity(trace.len());
    let mut prefix_likelihoods = Vec::with_capacity(trace.len());
    let mut first_zero_prefix = None;
    let mut terminal_hazard = false;

    for (step_index, step) in trace.iter().enumerate() {
        let prefix = step_index + 1;
        if terminal_hazard {
            first_zero_prefix = Some(prefix);
            trace_likelihood = Rational::zero();
            conditional_step_probabilities.push(Rational::zero());
            prefix_likelihoods.push(Rational::zero());
            break;
        }

        let action = kernel.action_ids.iter().position(|id| id == &step.action_id)
            .ok_or_else(|| format!("unknown action ID {}", step.action_id))?;
        let observation = match &step.outcome {
            ObservedOutcome::Observation { id } => Some(
                kernel.observation_ids.iter().position(|known| known == id)
                    .ok_or_else(|| format!("unknown observation ID {id}"))?
            ),
            ObservedOutcome::Hazard => None,
        };
        let is_hazard = matches!(&step.outcome, ObservedOutcome::Hazard);

        let mut next_states: BTreeMap<(usize, usize, usize), Rational> = BTreeMap::new();
        let mut event_probability = Rational::zero();
        for point in &posterior {
            let row_index = (((point.hypothesis * kernel.physical_state_count
                + point.physical_state)
                * kernel.sensor_state_count
                + point.sensor_state)
                * kernel.action_ids.len())
                + action;
            for outcome in &kernel.rows[row_index].outcomes {
                *kernel_outcome_applications = kernel_outcome_applications
                    .checked_add(1)
                    .ok_or_else(|| "trace kernel-outcome counter overflow".to_string())?;
                if *kernel_outcome_applications > MAX_TRACE_KERNEL_OUTCOME_APPLICATIONS {
                    return Err("trace kernel-outcome application budget exceeded".into());
                }
                if outcome.observation != observation {
                    continue;
                }
                let path_mass = point.mass.checked_mul(outcome.probability)?;
                event_probability = event_probability.checked_add(path_mass)?;
                if observation.is_some() {
                    let key = (
                        point.hypothesis,
                        outcome.next_physical_state,
                        outcome.next_sensor_state,
                    );
                    let prior = next_states.get(&key).copied().unwrap_or(Rational::zero());
                    next_states.insert(key, prior.checked_add(path_mass)?);
                }
            }
        }

        conditional_step_probabilities.push(event_probability);
        if event_probability.is_zero() {
            first_zero_prefix = Some(prefix);
            trace_likelihood = Rational::zero();
            prefix_likelihoods.push(Rational::zero());
            posterior.clear();
            break;
        }

        trace_likelihood = trace_likelihood.checked_mul(event_probability)?;
        prefix_likelihoods.push(trace_likelihood);
        if is_hazard {
            terminal_hazard = true;
            posterior.clear();
        } else {
            if next_states.len() > MAX_POSTERIOR_TUPLES {
                return Err("posterior joint-state tuple cap exceeded".into());
            }
            posterior = next_states.into_iter()
                .map(|((hypothesis, physical_state, sensor_state), mass)| {
                    Ok(JointMass {
                        hypothesis,
                        physical_state,
                        sensor_state,
                        mass: mass.checked_div(event_probability)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            if posterior.is_empty() {
                return Err("nonzero observation likelihood produced empty posterior".into());
            }
        }
    }

    let final_joint_posterior = if first_zero_prefix.is_none() && !terminal_hazard {
        Some(posterior)
    } else {
        None
    };
    Ok(WorldTraceAudit {
        world_id: world.id.clone(),
        trace_likelihood,
        conditional_step_probabilities,
        prefix_likelihoods,
        first_zero_prefix,
        terminal_hazard,
        final_joint_posterior,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_rel_joint_stochastic_oracle::{
        JointKernel, JointOutcome, KernelRow, Rational,
    };

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn simple_world(id: &str, reverse: bool) -> ModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            for action in 0..2 {
                let observation = match (action, reverse) {
                    (0, false) | (1, true) => 0,
                    (0, true) | (1, false) => 1,
                    _ => unreachable!("fixture has exactly two actions"),
                };
                rows.push(KernelRow {
                    hypothesis,
                    physical_state: 0,
                    sensor_state: 0,
                    action,
                    outcomes: vec![JointOutcome {
                        probability: Rational::one(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(observation),
                    }],
                });
            }
        }
        ModelWorld {
            id: id.into(),
            kernel: JointKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                action_ids: vec!["read".into(), "confirm".into()],
                observation_ids: vec!["zero".into(), "one".into()],
                action_costs: vec![1, 1],
                initial_belief: vec![
                    JointMass {
                        hypothesis: 0,
                        physical_state: 0,
                        sensor_state: 0,
                        mass: half(),
                    },
                    JointMass {
                        hypothesis: 1,
                        physical_state: 0,
                        sensor_state: 0,
                        mass: half(),
                    },
                ],
                rows,
            },
        }
    }

    fn step(action: &str, observation: &str) -> ObservedStep {
        ObservedStep {
            action_id: action.into(),
            outcome: ObservedOutcome::Observation { id: observation.into() },
        }
    }

    #[derive(Clone)]
    struct BruteTrajectory {
        hypothesis: usize,
        physical_state: usize,
        sensor_state: usize,
        mass: Rational,
        terminal: bool,
    }

    /// Independent unnormalized trajectory enumeration; does not use posterior
    /// updates or any production auditor helper.
    fn brute_trace_likelihood(world: &ModelWorld, trace: &[ObservedStep]) -> Rational {
        let kernel = &world.kernel;
        let mut paths: Vec<BruteTrajectory> = kernel
            .initial_belief
            .iter()
            .map(|point| BruteTrajectory {
                hypothesis: point.hypothesis,
                physical_state: point.physical_state,
                sensor_state: point.sensor_state,
                mass: point.mass,
                terminal: false,
            })
            .collect();

        for step in trace {
            let action = kernel.action_ids.iter().position(|id| id == &step.action_id).unwrap();
            let observation = match &step.outcome {
                ObservedOutcome::Observation { id } => Some(
                    kernel.observation_ids.iter().position(|known| known == id).unwrap(),
                ),
                ObservedOutcome::Hazard => None,
            };
            let mut next_paths = Vec::new();
            for path in &paths {
                if path.terminal {
                    continue;
                }
                let row_index = (((path.hypothesis * kernel.physical_state_count
                    + path.physical_state)
                    * kernel.sensor_state_count
                    + path.sensor_state)
                    * kernel.action_ids.len())
                    + action;
                for outcome in &kernel.rows[row_index].outcomes {
                    if outcome.observation != observation {
                        continue;
                    }
                    let mass = path.mass.checked_mul(outcome.probability).unwrap();
                    next_paths.push(BruteTrajectory {
                        hypothesis: path.hypothesis,
                        physical_state: outcome.next_physical_state,
                        sensor_state: outcome.next_sensor_state,
                        mass,
                        terminal: observation.is_none(),
                    });
                }
            }
            paths = next_paths;
        }
        paths.iter().try_fold(
            Rational::zero(),
            |sum, path| sum.checked_add(path.mass),
        ).unwrap()
    }

    #[test]
    fn surviving_world_is_not_declared_true() {
        let worlds = vec![simple_world("nominal", false), simple_world("reversed", true)];
        let audit = audit_trace_against_worlds(&worlds, &[step("read", "zero")]).unwrap();
        assert_eq!(audit.status, FamilyTraceStatus::WithinDeclaredFamily);
        assert_eq!(audit.supported_world_ids, vec!["nominal".to_string()]);
        assert_eq!(audit.eliminated_world_ids, vec!["reversed".to_string()]);
        assert_eq!(audit.per_world[0].trace_likelihood, Rational::one());
        assert_eq!(audit.per_world[0].final_joint_posterior.as_ref().unwrap().len(), 2);
        assert_eq!(audit.per_world[1].first_zero_prefix, Some(1));
        assert!(audit.per_world[1].final_joint_posterior.is_none());
    }

    #[test]
    fn reports_first_prefix_where_the_entire_family_becomes_empty() {
        let mut first = simple_world("first", false);
        let mut second = simple_world("second", false);
        for row in first.kernel.rows.iter_mut().filter(|row| row.action == 1) {
            for outcome in &mut row.outcomes {
                outcome.observation = Some(0);
            }
        }
        for row in second.kernel.rows.iter_mut().filter(|row| row.action == 0) {
            for outcome in &mut row.outcomes {
                outcome.observation = Some(1);
            }
        }
        let audit = audit_trace_against_worlds(
            &[first, second],
            &[step("read", "zero"), step("confirm", "one")],
        ).unwrap();
        assert_eq!(
            audit.status,
            FamilyTraceStatus::OutOfDeclaredFamily { first_empty_prefix: 2 }
        );
        assert_eq!(audit.per_world[0].first_zero_prefix, Some(2));
        assert_eq!(audit.per_world[1].first_zero_prefix, Some(1));
        assert!(audit.supported_world_ids.is_empty());
    }

    #[test]
    fn tiny_nonzero_likelihood_is_never_rounded_to_zero() {
        let epsilon = Rational::new(1, u128::MAX).unwrap();
        let complement = Rational::one().checked_sub(epsilon).unwrap();
        let mut world = simple_world("rare", false);
        for row in &mut world.kernel.rows {
            row.outcomes = vec![
                JointOutcome {
                    probability: complement,
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(0),
                },
                JointOutcome {
                    probability: epsilon,
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(1),
                },
            ];
        }
        let audit = audit_trace_against_worlds(&[world], &[step("read", "one")]).unwrap();
        assert_eq!(audit.status, FamilyTraceStatus::WithinDeclaredFamily);
        assert_eq!(audit.per_world[0].trace_likelihood, epsilon);
        assert_eq!(audit.per_world[0].first_zero_prefix, None);
    }

    #[test]
    fn hazard_is_terminal_and_later_actions_eliminate_that_world() {
        let mut world = simple_world("hazard", false);
        for row in &mut world.kernel.rows {
            row.outcomes = vec![JointOutcome {
                probability: Rational::one(),
                next_physical_state: 0,
                next_sensor_state: 0,
                observation: None,
            }];
        }
        let trace = vec![
            ObservedStep { action_id: "read".into(), outcome: ObservedOutcome::Hazard },
            step("confirm", "zero"),
        ];
        let audit = audit_trace_against_worlds(&[world], &trace).unwrap();
        assert_eq!(
            audit.status,
            FamilyTraceStatus::OutOfDeclaredFamily { first_empty_prefix: 2 }
        );
        assert!(audit.per_world[0].terminal_hazard);
        assert_eq!(audit.per_world[0].first_zero_prefix, Some(2));
    }

    #[test]
    fn unknown_ids_and_incompatible_worlds_fail_closed() {
        let worlds = vec![simple_world("a", false), simple_world("b", true)];
        let unknown = ObservedStep {
            action_id: "read".into(),
            outcome: ObservedOutcome::Observation { id: "unknown".into() },
        };
        assert!(audit_trace_against_worlds(&worlds, &[unknown]).is_err());
        let mut incompatible = simple_world("changed", false);
        incompatible.kernel.action_costs[0] = 5;
        assert!(audit_trace_against_worlds(&[worlds[0].clone(), incompatible], &[]).is_err());
    }


    #[test]
    fn prefix_likelihoods_match_independent_unnormalized_trajectory_enumeration() {
        let mut rare = simple_world("rare", false);
        let epsilon = Rational::new(1, u128::MAX).unwrap();
        let complement = Rational::one().checked_sub(epsilon).unwrap();
        for row in &mut rare.kernel.rows {
            row.outcomes = vec![
                JointOutcome {
                    probability: complement,
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(0),
                },
                JointOutcome {
                    probability: epsilon,
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(1),
                },
            ];
        }

        let worlds = vec![
            simple_world("normal", false),
            simple_world("reverse", true),
            rare,
        ];
        let traces = vec![
            vec![step("read", "zero")],
            vec![step("read", "one"), step("confirm", "zero")],
            vec![
                ObservedStep {
                    action_id: "read".into(),
                    outcome: ObservedOutcome::Hazard,
                },
                step("confirm", "one"),
            ],
            Vec::new(),
        ];

        for world in &worlds {
            for trace in &traces {
                let audit = audit_trace_against_worlds(
                    std::slice::from_ref(world),
                    trace,
                ).unwrap();
                assert_eq!(
                    audit.per_world[0].trace_likelihood,
                    brute_trace_likelihood(world, trace),
                    "world={} trace_len={}",
                    world.id,
                    trace.len()
                );
            }
        }
    }

    #[test]
    fn supported_joint_posterior_is_exactly_normalized() {
        let audit = audit_trace_against_worlds(
            &[simple_world("a", false)],
            &[step("read", "zero")],
        ).unwrap();
        let posterior = audit.per_world[0].final_joint_posterior.as_ref().unwrap();
        let total = posterior.iter().try_fold(
            Rational::zero(),
            |sum, point| sum.checked_add(point.mass),
        ).unwrap();
        assert_eq!(total, Rational::one());
    }
}
