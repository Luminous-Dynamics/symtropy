//! Exact model-indexed replay over a family of joint stochastic worlds.
//!
//! One complete kernel is used for an entire campaign. This module evaluates
//! the same public-observation policy independently in every supplied world.
//! It never lets a policy inspect the world ID and never rectangularizes model
//! identity by switching kernels between action steps.

use super::{evaluate_policy, FrontierPoint, JointKernel, PolicyPlan, Rational};
use std::collections::BTreeSet;

const MAX_MODEL_WORLDS: usize = 256;

/// One complete, persistent model-world alternative.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelWorld {
    /// Stable reporting identifier; never exposed to the policy.
    pub id: String,
    /// Complete joint stochastic model used for this entire replay.
    pub kernel: JointKernel,
}

/// Exact policy result in one fixed model world.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldEvaluation {
    pub world_id: String,
    pub expected_cost: Rational,
    pub hazard_probability: Rational,
    pub false_identification_probability: Rational,
    pub abstention_probability: Rational,
    pub correct_identification_probability: Rational,
}

/// Per-world replay plus exact worst-case summaries over the finite model family.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RobustEvaluationReport {
    pub nominal_world_id: String,
    pub nominal: WorldEvaluation,
    pub worst_case_expected_cost: Rational,
    pub worst_case_hazard_probability: Rational,
    pub worst_case_false_identification_probability: Rational,
    pub worst_case_abstention_probability: Rational,
    pub worst_case_correct_identification_probability: Rational,
    pub per_world: Vec<WorldEvaluation>,
}

impl RobustEvaluationReport {
    /// Check explicit worst-case hazard, false-claim, and abstention budgets.
    ///
    /// All three budgets must be probabilities in [0, 1]. Returns false when
    /// any supplied complete world violates any budget.
    pub fn satisfies_budgets(
        &self,
        max_hazard: Rational,
        max_false_identification: Rational,
        max_abstention: Rational,
    ) -> Result<bool, String> {
        for (name, budget) in [
            ("hazard", max_hazard),
            ("false-identification", max_false_identification),
            ("abstention", max_abstention),
        ] {
            if !budget.is_probability() {
                return Err(format!("{name} budget must lie in [0, 1]"));
            }
        }
        Ok(self.worst_case_hazard_probability <= max_hazard
            && self.worst_case_false_identification_probability <= max_false_identification
            && self.worst_case_abstention_probability <= max_abstention)
    }
}

/// Evaluate the exact same policy under every supplied complete model world.
///
/// Compatibility requires the same hypothesis/action/observation vocabularies,
/// state dimensions, and action costs. Initial joint beliefs may differ by world,
/// because these are model-indexed alternatives; each is validated and replayed
/// as supplied. Kernel identity is held fixed through each complete trajectory.
pub fn evaluate_policy_across_worlds(
    worlds: &[ModelWorld],
    nominal_world_id: &str,
    policy: &PolicyPlan,
    max_steps: usize,
) -> Result<RobustEvaluationReport, String> {
    if worlds.is_empty() || worlds.len() > MAX_MODEL_WORLDS {
        return Err(format!("model-world count must be in 1..={MAX_MODEL_WORLDS}"));
    }
    if nominal_world_id.trim().is_empty() {
        return Err("nominal world ID must not be empty".into());
    }

    let mut ids = BTreeSet::new();
    for world in worlds {
        if world.id.trim().is_empty() || !ids.insert(world.id.as_str()) {
            return Err("model-world IDs must be non-empty and unique".into());
        }
        world.kernel.validate()?;
    }
    let nominal_index = worlds
        .iter()
        .position(|world| world.id == nominal_world_id)
        .ok_or_else(|| "nominal world ID is not present in the supplied family".to_string())?;

    let reference = &worlds[0].kernel;
    for world in worlds.iter().skip(1) {
        validate_compatible(reference, &world.kernel, &world.id)?;
    }

    let mut per_world = Vec::with_capacity(worlds.len());
    for world in worlds {
        let result: FrontierPoint = evaluate_policy(&world.kernel, policy, max_steps)?;
        per_world.push(WorldEvaluation {
            world_id: world.id.clone(),
            expected_cost: result.expected_cost,
            hazard_probability: result.hazard_probability,
            false_identification_probability: result.false_identification_probability,
            abstention_probability: result.abstention_probability,
            correct_identification_probability: result.correct_identification_probability,
        });
    }

    let nominal = per_world[nominal_index].clone();
    let worst_case_expected_cost = per_world.iter()
        .map(|result| result.expected_cost)
        .max()
        .ok_or_else(|| "internal empty model-world evaluation".to_string())?;
    let worst_case_hazard_probability = per_world.iter()
        .map(|result| result.hazard_probability)
        .max()
        .ok_or_else(|| "internal empty model-world evaluation".to_string())?;
    let worst_case_false_identification_probability = per_world.iter()
        .map(|result| result.false_identification_probability)
        .max()
        .ok_or_else(|| "internal empty model-world evaluation".to_string())?;
    let worst_case_abstention_probability = per_world.iter()
        .map(|result| result.abstention_probability)
        .max()
        .ok_or_else(|| "internal empty model-world evaluation".to_string())?;
    let worst_case_correct_identification_probability = per_world.iter()
        .map(|result| result.correct_identification_probability)
        .min()
        .ok_or_else(|| "internal empty model-world evaluation".to_string())?;

    Ok(RobustEvaluationReport {
        nominal_world_id: nominal_world_id.to_string(),
        nominal,
        worst_case_expected_cost,
        worst_case_hazard_probability,
        worst_case_false_identification_probability,
        worst_case_abstention_probability,
        worst_case_correct_identification_probability,
        per_world,
    })
}

fn validate_compatible(
    reference: &JointKernel,
    alternative: &JointKernel,
    world_id: &str,
) -> Result<(), String> {
    if reference.hypothesis_ids != alternative.hypothesis_ids {
        return Err(format!("world {world_id} changes the semantic hypothesis vocabulary or ordering"));
    }
    if reference.physical_state_count != alternative.physical_state_count
        || reference.sensor_state_count != alternative.sensor_state_count
    {
        return Err(format!("world {world_id} changes the declared physical or sensor state dimensions"));
    }
    if reference.physical_state_ids != alternative.physical_state_ids {
        return Err(format!("world {world_id} changes the semantic physical-state vocabulary or ordering"));
    }
    if reference.sensor_state_ids != alternative.sensor_state_ids {
        return Err(format!("world {world_id} changes the semantic sensor-state vocabulary or ordering"));
    }
    if reference.action_ids != alternative.action_ids {
        return Err(format!("world {world_id} changes the shared action vocabulary or ordering"));
    }
    if reference.observation_ids != alternative.observation_ids {
        return Err(format!("world {world_id} changes the shared observation vocabulary or ordering"));
    }
    if reference.action_costs != alternative.action_costs {
        return Err(format!("world {world_id} changes the shared action-cost contract"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JointMass, JointOutcome, KernelRow};

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn make_world(id: &str, reverse: bool) -> ModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            let observation = if (hypothesis == 0) == reverse { 1 } else { 0 };
            rows.push(KernelRow {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes: vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(observation),
                }],
            });
        }
        ModelWorld {
            id: id.to_string(),
            kernel: JointKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                physical_state_ids: vec!["specimen-state-0".into()],
                sensor_state_ids: vec!["sensor-state-0".into()],
                action_ids: vec!["measure".into()],
                observation_ids: vec!["zero".into(), "one".into()],
                action_costs: vec![2],
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

    fn identifying_policy() -> PolicyPlan {
        PolicyPlan::Action {
            action: 0,
            branches: vec![
                crate::PolicyBranch {
                    observation: 0,
                    plan: Box::new(PolicyPlan::Claim { hypothesis: 0 }),
                },
                crate::PolicyBranch {
                    observation: 1,
                    plan: Box::new(PolicyPlan::Claim { hypothesis: 1 }),
                },
            ],
        }
    }

    #[test]
    fn nominally_perfect_policy_can_be_fully_wrong_under_an_alternative_kernel() {
        let worlds = vec![make_world("nominal", false), make_world("shifted", true)];
        let report = evaluate_policy_across_worlds(
            &worlds,
            "nominal",
            &identifying_policy(),
            1,
        ).unwrap();

        assert_eq!(report.nominal.correct_identification_probability, Rational::one());
        assert_eq!(report.nominal.false_identification_probability, Rational::zero());
        assert_eq!(report.worst_case_correct_identification_probability, Rational::zero());
        assert_eq!(report.worst_case_false_identification_probability, Rational::one());
        assert_eq!(report.worst_case_expected_cost, Rational::from_u64(2));
        assert!(!report.satisfies_budgets(
            Rational::zero(),
            Rational::zero(),
            Rational::zero(),
        ).unwrap());
        assert!(report.satisfies_budgets(
            Rational::zero(),
            Rational::one(),
            Rational::zero(),
        ).unwrap());
    }

    #[test]
    fn shifted_kernel_hazard_is_not_hidden_by_a_safe_nominal_world() {
        let nominal = make_world("nominal", false);
        let mut shifted = make_world("hazard-shift", false);
        shifted.kernel.rows[1].outcomes[0].observation = None;

        let report = evaluate_policy_across_worlds(
            &[nominal, shifted],
            "nominal",
            &identifying_policy(),
            1,
        ).unwrap();
        assert_eq!(report.nominal.hazard_probability, Rational::zero());
        assert_eq!(report.worst_case_hazard_probability, half());
        assert!(!report.satisfies_budgets(
            Rational::new(1, 4).unwrap(),
            Rational::zero(),
            Rational::zero(),
        ).unwrap());
    }

    #[test]
    fn semantic_vocabulary_and_cost_drift_are_rejected() {
        let nominal = make_world("nominal", false);
        let mut changed = make_world("changed", false);
        changed.kernel.action_ids[0] = "different-operation".into();
        assert!(evaluate_policy_across_worlds(
            &[nominal.clone(), changed],
            "nominal",
            &identifying_policy(),
            1,
        ).unwrap_err().contains("action vocabulary"));

        let mut changed_cost = make_world("changed-cost", false);
        changed_cost.kernel.action_costs[0] = 3;
        assert!(evaluate_policy_across_worlds(
            &[nominal.clone()],
            "nominal",
            &identifying_policy(),
            1,
        ).is_ok());
        assert!(evaluate_policy_across_worlds(
            &[nominal, changed_cost],
            "nominal",
            &identifying_policy(),
            1,
        ).unwrap_err().contains("action-cost"));
    }

    #[test]
    fn duplicate_world_ids_and_missing_nominal_fail_closed() {
        let first = make_world("same", false);
        let second = make_world("same", true);
        assert!(evaluate_policy_across_worlds(
            &[first.clone(), second],
            "same",
            &identifying_policy(),
            1,
        ).unwrap_err().contains("unique"));
        assert!(evaluate_policy_across_worlds(
            &[first],
            "absent",
            &identifying_policy(),
            1,
        ).unwrap_err().contains("not present"));
    }
}
