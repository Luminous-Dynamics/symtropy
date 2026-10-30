//! Exact replay of shared adaptive policies across finite stateful scientific worlds.
//!
//! Unlike static experiment-selection oracles, a safe experiment advances a hidden
//! physical state. Repeated actions therefore read and update the current state;
//! they are not assumed to be IID observations. This crate evaluates supplied policy
//! trees but does not synthesize or optimize them.
//!
//! The policy sees only binary observations and never receives a true hypothesis,
//! latent state, or world identifier.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::collections::BTreeSet;

const MAX_POLICY_NODES: usize = 8_192;
const MAX_POLICY_STEPS: usize = 128;
const MAX_ACTIONS: usize = 64;

/// A reduced non-negative rational number.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fraction {
    pub numerator: u128,
    pub denominator: u128,
}

impl Fraction {
    pub fn new(numerator: u128, denominator: u128) -> Result<Self, String> {
        if denominator == 0 {
            return Err("fraction denominator must be positive".into());
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_ratios(
            self.numerator,
            self.denominator,
            other.numerator,
            other.denominator,
        )
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// Compare non-negative rational values without cross-multiplication overflow.
fn compare_ratios(
    mut left_num: u128,
    mut left_den: u128,
    mut right_num: u128,
    mut right_den: u128,
) -> Ordering {
    let mut reversed = false;
    loop {
        let left_quotient = left_num / left_den;
        let right_quotient = right_num / right_den;
        if left_quotient != right_quotient {
            return if reversed {
                right_quotient.cmp(&left_quotient)
            } else {
                left_quotient.cmp(&right_quotient)
            };
        }

        let left_remainder = left_num % left_den;
        let right_remainder = right_num % right_den;
        match (left_remainder == 0, right_remainder == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => {
                return if reversed {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
            }
            (false, true) => {
                return if reversed {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            (false, false) => {}
        }

        left_num = left_den;
        left_den = left_remainder;
        right_num = right_den;
        right_den = right_remainder;
        reversed = !reversed;
    }
}

/// A single physical transition for one action in one latent state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Transition {
    /// The experiment is safe, returns an observable bit, and changes physical state.
    Safe {
        observation: bool,
        next_state: usize,
    },
    /// The experiment causes a campaign-ending hazard; no observation is returned.
    Hazard,
}

/// Finite-state dynamics associated with one semantic mechanism hypothesis.
///
/// The transition table is indexed by action, then current hidden physical state.
/// Every action row must enumerate the same non-empty state space for this hypothesis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HypothesisMachine {
    pub initial_state: usize,
    pub transitions: Vec<Vec<Transition>>,
}

/// A complete coherent stateful world.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct World {
    /// World identity, used for reporting only; never supplied to the policy.
    pub name: String,
    /// Stable semantic hypothesis IDs shared by all worlds.
    pub hypothesis_ids: Vec<String>,
    /// Stable action IDs shared by all worlds.
    pub action_ids: Vec<String>,
    /// Prior mass for each semantic hypothesis.
    pub prior_weights: Vec<u64>,
    /// Positive, shared cost of each action.
    pub action_costs: Vec<u64>,
    /// One finite-state transducer per hypothesis.
    pub machines: Vec<HypothesisMachine>,
}

/// One observable-history policy node.
///
/// Repeated action IDs are allowed. Whether a repeat is informative or hazardous
/// depends on the current hidden state reached along that hypothesis' trajectory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyNode {
    Action {
        action: usize,
        on_zero: usize,
        on_one: usize,
    },
    Claim {
        hypothesis: usize,
    },
    Abstain,
}

/// A finite rooted policy tree.
///
/// max_steps bounds the number of experiments on any path. The tree must be acyclic,
/// contain no shared child nodes, and contain no unreachable nodes. A repeated action
/// is valid if it occurs at a later node in the same path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyTree {
    pub nodes: Vec<PolicyNode>,
    pub root: usize,
    pub max_steps: usize,
}

/// Exact metrics for one complete stateful world.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldMetrics {
    pub world: String,
    pub expected_cost: Fraction,
    pub hazard_probability: Fraction,
    pub identification_probability: Fraction,
    pub false_identification_probability: Fraction,
    pub abstention_probability: Fraction,
}

/// Per-world results plus conservative summaries across the supplied world family.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationReport {
    pub nominal_world: String,
    pub nominal_expected_cost: Fraction,
    pub worst_case_expected_cost: Fraction,
    pub worst_case_hazard_probability: Fraction,
    pub worst_case_identification_probability: Fraction,
    pub worst_case_false_identification_probability: Fraction,
    pub worst_case_abstention_probability: Fraction,
    pub per_world: Vec<WorldMetrics>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Terminal {
    Hazard,
    CorrectClaim,
    FalseClaim,
    Abstain,
}

impl World {
    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("world name must not be empty".into());
        }

        let hypothesis_count = self.hypothesis_ids.len();
        let action_count = self.action_ids.len();
        if hypothesis_count == 0 || self.prior_weights.len() != hypothesis_count {
            return Err(format!(
                "world {} has an empty or inconsistent hypothesis identity/prior table",
                self.name
            ));
        }
        if self.machines.len() != hypothesis_count {
            return Err(format!(
                "world {} machine count does not match its hypothesis count",
                self.name
            ));
        }
        if action_count == 0
            || action_count > MAX_ACTIONS
            || self.action_costs.len() != action_count
        {
            return Err(format!(
                "world {} must have 1..={MAX_ACTIONS} action IDs and matching costs",
                self.name
            ));
        }
        if self.action_costs.contains(&0) {
            return Err(format!("world {} contains a zero action cost", self.name));
        }
        validate_unique_ids(&self.hypothesis_ids, "hypothesis")?;
        validate_unique_ids(&self.action_ids, "action")?;

        let total_prior = self.prior_weights.iter().try_fold(0_u128, |total, weight| {
            total
                .checked_add(u128::from(*weight))
                .ok_or_else(|| format!("world {} prior mass overflowed u128", self.name))
        })?;
        if total_prior == 0 {
            return Err(format!("world {} has zero total prior mass", self.name));
        }

        for (hypothesis, machine) in self.machines.iter().enumerate() {
            if machine.transitions.len() != action_count {
                return Err(format!(
                    "world {} hypothesis {hypothesis} has an invalid action-row count",
                    self.name
                ));
            }
            let state_count = machine
                .transitions
                .first()
                .map(Vec::len)
                .ok_or_else(|| format!("world {} has no state rows", self.name))?;
            if state_count == 0 || machine.initial_state >= state_count {
                return Err(format!(
                    "world {} hypothesis {hypothesis} has an invalid state space or initial state",
                    self.name
                ));
            }

            for (action, row) in machine.transitions.iter().enumerate() {
                if row.len() != state_count {
                    return Err(format!(
                        "world {} hypothesis {hypothesis} action {action} has an incomplete state row",
                        self.name
                    ));
                }
                for (state, transition) in row.iter().enumerate() {
                    if let Transition::Safe { next_state, .. } = transition {
                        if *next_state >= state_count {
                            return Err(format!(
                                "world {} hypothesis {hypothesis} action {action} state {state} has an out-of-range next state",
                                self.name
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn validate_unique_ids(ids: &[String], kind: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(format!("{kind} ID must not be empty"));
        }
        if !seen.insert(id.as_str()) {
            return Err(format!("duplicate {kind} ID {id}"));
        }
    }
    Ok(())
}

impl PolicyTree {
    fn validate(&self, hypothesis_count: usize, action_count: usize) -> Result<(), String> {
        if self.nodes.is_empty() || self.nodes.len() > MAX_POLICY_NODES {
            return Err(format!(
                "policy must contain between 1 and {MAX_POLICY_NODES} nodes"
            ));
        }
        if self.root >= self.nodes.len() {
            return Err(format!("policy root {} is out of range", self.root));
        }
        if self.max_steps > MAX_POLICY_STEPS {
            return Err(format!(
                "policy horizon exceeds maximum of {MAX_POLICY_STEPS} steps"
            ));
        }

        let mut active = vec![false; self.nodes.len()];
        let mut reachable = vec![false; self.nodes.len()];
        let mut parent_counts = vec![0_usize; self.nodes.len()];
        self.validate_node(
            self.root,
            0,
            hypothesis_count,
            action_count,
            &mut active,
            &mut reachable,
            &mut parent_counts,
        )?;

        if parent_counts[self.root] != 0 {
            return Err("policy root must not have a parent".into());
        }
        for (index, (is_reachable, parents)) in reachable
            .iter()
            .zip(parent_counts.iter())
            .enumerate()
        {
            if !is_reachable {
                return Err(format!("policy contains unreachable node {index}"));
            }
            if index != self.root && *parents != 1 {
                return Err(format!(
                    "policy node {index} must have exactly one observable-history parent"
                ));
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_node(
        &self,
        index: usize,
        depth: usize,
        hypothesis_count: usize,
        action_count: usize,
        active: &mut [bool],
        reachable: &mut [bool],
        parent_counts: &mut [usize],
    ) -> Result<(), String> {
        if index >= self.nodes.len() {
            return Err(format!("policy references missing node {index}"));
        }
        if active[index] {
            return Err(format!("policy contains a cycle through node {index}"));
        }
        if reachable[index] {
            return Err(format!("policy node {index} is shared by multiple histories"));
        }
        active[index] = true;
        reachable[index] = true;

        let result = match &self.nodes[index] {
            PolicyNode::Action {
                action,
                on_zero,
                on_one,
            } => {
                if *action >= action_count {
                    Err(format!("policy references missing action {action}"))
                } else if depth >= self.max_steps {
                    Err(format!(
                        "policy exceeds declared finite horizon of {} steps",
                        self.max_steps
                    ))
                } else {
                    for child in [*on_zero, *on_one] {
                        if child >= self.nodes.len() {
                            return Err(format!("policy references missing node {child}"));
                        }
                        parent_counts[child] = parent_counts[child]
                            .checked_add(1)
                            .ok_or_else(|| "policy parent count overflowed".to_string())?;
                        if parent_counts[child] > 1 {
                            return Err(format!(
                                "policy node {child} is shared by multiple histories"
                            ));
                        }
                    }
                    self.validate_node(
                        *on_zero,
                        depth + 1,
                        hypothesis_count,
                        action_count,
                        active,
                        reachable,
                        parent_counts,
                    )?;
                    self.validate_node(
                        *on_one,
                        depth + 1,
                        hypothesis_count,
                        action_count,
                        active,
                        reachable,
                        parent_counts,
                    )
                }
            }
            PolicyNode::Claim { hypothesis } => {
                if *hypothesis >= hypothesis_count {
                    Err(format!("policy claims missing hypothesis {hypothesis}"))
                } else {
                    Ok(())
                }
            }
            PolicyNode::Abstain => Ok(()),
        };

        active[index] = false;
        result
    }
}

/// Evaluate the same observable-history policy under every supplied stateful world.
///
/// Physical state is advanced separately for each world/hypothesis trajectory. The policy
/// receives only the public observation bit and no hypothesis ID, latent state, or model
/// index. Repeated actions are valid and use the current state. Action cost is paid before
/// the transition, including on a hazard.
pub fn evaluate_shared_policy(
    policy: &PolicyTree,
    worlds: &[World],
    nominal_world_index: usize,
) -> Result<EvaluationReport, String> {
    if worlds.is_empty() {
        return Err("at least one complete world is required".into());
    }
    if nominal_world_index >= worlds.len() {
        return Err("nominal world index is out of range".into());
    }

    let first = &worlds[0];
    first.validate()?;
    for world in worlds.iter().skip(1) {
        world.validate()?;
        if world.hypothesis_ids != first.hypothesis_ids
            || world.action_ids != first.action_ids
            || world.action_costs != first.action_costs
        {
            return Err(
                "all worlds must share ordered hypothesis IDs, action IDs, and action costs"
                    .into(),
            );
        }
    }
    let mut world_names = BTreeSet::new();
    for world in worlds {
        if !world_names.insert(world.name.as_str()) {
            return Err(format!("duplicate world name {}", world.name));
        }
    }

    policy.validate(first.hypothesis_ids.len(), first.action_ids.len())?;

    let mut per_world = Vec::with_capacity(worlds.len());
    for world in worlds {
        per_world.push(evaluate_one_world(policy, world)?);
    }

    let nominal = &per_world[nominal_world_index];
    let worst_case_expected_cost = per_world
        .iter()
        .map(|metrics| metrics.expected_cost)
        .max()
        .ok_or_else(|| "world evaluation unexpectedly returned no metrics".to_string())?;
    let worst_case_hazard_probability = per_world
        .iter()
        .map(|metrics| metrics.hazard_probability)
        .max()
        .ok_or_else(|| "world evaluation unexpectedly returned no metrics".to_string())?;
    let worst_case_identification_probability = per_world
        .iter()
        .map(|metrics| metrics.identification_probability)
        .min()
        .ok_or_else(|| "world evaluation unexpectedly returned no metrics".to_string())?;
    let worst_case_false_identification_probability = per_world
        .iter()
        .map(|metrics| metrics.false_identification_probability)
        .max()
        .ok_or_else(|| "world evaluation unexpectedly returned no metrics".to_string())?;
    let worst_case_abstention_probability = per_world
        .iter()
        .map(|metrics| metrics.abstention_probability)
        .max()
        .ok_or_else(|| "world evaluation unexpectedly returned no metrics".to_string())?;

    Ok(EvaluationReport {
        nominal_world: nominal.world.clone(),
        nominal_expected_cost: nominal.expected_cost,
        worst_case_expected_cost,
        worst_case_hazard_probability,
        worst_case_identification_probability,
        worst_case_false_identification_probability,
        worst_case_abstention_probability,
        per_world,
    })
}

fn evaluate_one_world(policy: &PolicyTree, world: &World) -> Result<WorldMetrics, String> {
    let total_mass = world.prior_weights.iter().try_fold(0_u128, |total, weight| {
        total
            .checked_add(u128::from(*weight))
            .ok_or_else(|| format!("world {} prior mass overflowed u128", world.name))
    })?;

    let mut weighted_cost = 0_u128;
    let mut hazard_mass = 0_u128;
    let mut identification_mass = 0_u128;
    let mut false_identification_mass = 0_u128;
    let mut abstention_mass = 0_u128;

    for (hypothesis, weight) in world.prior_weights.iter().copied().enumerate() {
        if weight == 0 {
            continue;
        }
        let (cost, terminal) = replay_hypothesis(policy, world, hypothesis)?;
        let weighted = u128::from(weight);
        let cost_contribution = weighted
            .checked_mul(cost)
            .ok_or_else(|| format!("world {} expected-cost numerator overflowed", world.name))?;
        weighted_cost = weighted_cost
            .checked_add(cost_contribution)
            .ok_or_else(|| format!("world {} expected-cost sum overflowed", world.name))?;

        match terminal {
            Terminal::Hazard => {
                hazard_mass = hazard_mass
                    .checked_add(weighted)
                    .ok_or_else(|| format!("world {} hazard mass overflowed", world.name))?;
            }
            Terminal::CorrectClaim => {
                identification_mass = identification_mass
                    .checked_add(weighted)
                    .ok_or_else(|| format!("world {} identification mass overflowed", world.name))?;
            }
            Terminal::FalseClaim => {
                false_identification_mass = false_identification_mass
                    .checked_add(weighted)
                    .ok_or_else(|| format!("world {} false-claim mass overflowed", world.name))?;
            }
            Terminal::Abstain => {
                abstention_mass = abstention_mass
                    .checked_add(weighted)
                    .ok_or_else(|| format!("world {} abstention mass overflowed", world.name))?;
            }
        }
    }

    let terminal_mass = hazard_mass
        .checked_add(identification_mass)
        .and_then(|value| value.checked_add(false_identification_mass))
        .and_then(|value| value.checked_add(abstention_mass))
        .ok_or_else(|| format!("world {} terminal mass overflowed", world.name))?;
    if terminal_mass != total_mass {
        return Err(format!(
            "world {} terminal outcomes do not account for all prior mass",
            world.name
        ));
    }

    Ok(WorldMetrics {
        world: world.name.clone(),
        expected_cost: Fraction::new(weighted_cost, total_mass)?,
        hazard_probability: Fraction::new(hazard_mass, total_mass)?,
        identification_probability: Fraction::new(identification_mass, total_mass)?,
        false_identification_probability: Fraction::new(
            false_identification_mass,
            total_mass,
        )?,
        abstention_probability: Fraction::new(abstention_mass, total_mass)?,
    })
}

fn replay_hypothesis(
    policy: &PolicyTree,
    world: &World,
    hypothesis: usize,
) -> Result<(u128, Terminal), String> {
    let machine = &world.machines[hypothesis];
    let mut state = machine.initial_state;
    let mut node_index = policy.root;
    let mut cost = 0_u128;
    let mut steps = 0_usize;

    loop {
        let node = policy
            .nodes
            .get(node_index)
            .ok_or_else(|| format!("policy references missing node {node_index}"))?;
        match node {
            PolicyNode::Action {
                action,
                on_zero,
                on_one,
            } => {
                if steps >= policy.max_steps {
                    return Err("policy replay exceeded declared finite horizon".into());
                }
                let action_cost = *world
                    .action_costs
                    .get(*action)
                    .ok_or_else(|| format!("policy references missing action {action}"))?;
                cost = cost
                    .checked_add(u128::from(action_cost))
                    .ok_or_else(|| "realized policy cost overflowed u128".to_string())?;
                steps += 1;

                let transition = machine
                    .transitions
                    .get(*action)
                    .and_then(|row| row.get(state))
                    .ok_or_else(|| {
                        format!(
                            "world {} hypothesis {hypothesis} has no transition for action {action}, state {state}",
                            world.name
                        )
                    })?;
                match transition {
                    Transition::Hazard => return Ok((cost, Terminal::Hazard)),
                    Transition::Safe {
                        observation,
                        next_state,
                    } => {
                        state = *next_state;
                        node_index = if *observation { *on_one } else { *on_zero };
                    }
                }
            }
            PolicyNode::Claim {
                hypothesis: claimed,
            } => {
                let terminal = if *claimed == hypothesis {
                    Terminal::CorrectClaim
                } else {
                    Terminal::FalseClaim
                };
                return Ok((cost, terminal));
            }
            PolicyNode::Abstain => return Ok((cost, Terminal::Abstain)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order_effect_world(name: &str, reverse_signal: bool) -> World {
        let h0 = HypothesisMachine {
            initial_state: 0,
            transitions: vec![
                vec![
                    Transition::Safe {
                        observation: false,
                        next_state: 1,
                    },
                    Transition::Safe {
                        observation: false,
                        next_state: 1,
                    },
                ],
                vec![
                    Transition::Safe {
                        observation: false,
                        next_state: 0,
                    },
                    Transition::Safe {
                        observation: false,
                        next_state: 1,
                    },
                ],
            ],
        };
        let h1 = HypothesisMachine {
            initial_state: 0,
            transitions: vec![
                vec![
                    Transition::Safe {
                        observation: false,
                        next_state: 1,
                    },
                    Transition::Safe {
                        observation: false,
                        next_state: 1,
                    },
                ],
                vec![
                    Transition::Safe {
                        observation: false,
                        next_state: 0,
                    },
                    Transition::Safe {
                        observation: !reverse_signal,
                        next_state: 1,
                    },
                ],
            ],
        };
        World {
            name: name.into(),
            hypothesis_ids: vec!["h0".into(), "h1".into()],
            action_ids: vec!["condition".into(), "measure".into()],
            prior_weights: vec![1, 1],
            action_costs: vec![1, 1],
            machines: vec![h0, h1],
        }
    }

    fn condition_then_measure_policy() -> PolicyTree {
        PolicyTree {
            nodes: vec![
                PolicyNode::Action {
                    action: 0,
                    on_zero: 1,
                    on_one: 2,
                },
                PolicyNode::Action {
                    action: 1,
                    on_zero: 3,
                    on_one: 4,
                },
                PolicyNode::Abstain,
                PolicyNode::Claim { hypothesis: 0 },
                PolicyNode::Claim { hypothesis: 1 },
            ],
            root: 0,
            max_steps: 2,
        }
    }

    fn measure_then_condition_policy() -> PolicyTree {
        PolicyTree {
            nodes: vec![
                PolicyNode::Action {
                    action: 1,
                    on_zero: 1,
                    on_one: 2,
                },
                PolicyNode::Action {
                    action: 0,
                    on_zero: 3,
                    on_one: 4,
                },
                PolicyNode::Abstain,
                PolicyNode::Abstain,
                PolicyNode::Abstain,
            ],
            root: 0,
            max_steps: 2,
        }
    }

    #[test]
    fn exact_fraction_order_avoids_cross_product_overflow() {
        let left = Fraction::new(u128::MAX - 1, u128::MAX).unwrap();
        let right = Fraction::new(u128::MAX - 2, u128::MAX - 1).unwrap();
        assert!(left > right);
        assert_eq!(Fraction::new(50, 100).unwrap(), Fraction::new(1, 2).unwrap());
        assert!(Fraction::new(1, 0).is_err());
    }

    #[test]
    fn conditioning_then_measurement_identifies_but_reversing_order_does_not() {
        let world = order_effect_world("order-effect", false);
        let optimal_order =
            evaluate_shared_policy(&condition_then_measure_policy(), &[world.clone()], 0).unwrap();
        assert_eq!(
            optimal_order.per_world[0].identification_probability,
            Fraction::new(1, 1).unwrap()
        );
        assert_eq!(
            optimal_order.per_world[0].expected_cost,
            Fraction::new(2, 1).unwrap()
        );

        let reverse =
            evaluate_shared_policy(&measure_then_condition_policy(), &[world], 0).unwrap();
        assert_eq!(
            reverse.per_world[0].identification_probability,
            Fraction::new(0, 1).unwrap()
        );
        assert_eq!(
            reverse.per_world[0].abstention_probability,
            Fraction::new(1, 1).unwrap()
        );
    }

    #[test]
    fn same_policy_is_replayed_under_a_different_transition_world() {
        let worlds = vec![
            order_effect_world("signal", false),
            order_effect_world("no-signal", true),
        ];
        let report =
            evaluate_shared_policy(&condition_then_measure_policy(), &worlds, 0).unwrap();
        assert_eq!(
            report.per_world[0].identification_probability,
            Fraction::new(1, 1).unwrap()
        );
        assert_eq!(
            report.per_world[1].false_identification_probability,
            Fraction::new(1, 2).unwrap()
        );
        assert_eq!(
            report.worst_case_identification_probability,
            Fraction::new(1, 2).unwrap()
        );
        assert_eq!(
            report.worst_case_false_identification_probability,
            Fraction::new(1, 2).unwrap()
        );
    }

    #[test]
    fn repeated_action_uses_the_advanced_physical_state() {
        let machine = HypothesisMachine {
            initial_state: 0,
            transitions: vec![vec![
                Transition::Safe {
                    observation: false,
                    next_state: 1,
                },
                Transition::Safe {
                    observation: true,
                    next_state: 1,
                },
            ]],
        };
        let world = World {
            name: "repeat".into(),
            hypothesis_ids: vec!["only".into()],
            action_ids: vec!["pulse".into()],
            prior_weights: vec![1],
            action_costs: vec![2],
            machines: vec![machine],
        };
        let policy = PolicyTree {
            nodes: vec![
                PolicyNode::Action {
                    action: 0,
                    on_zero: 1,
                    on_one: 2,
                },
                PolicyNode::Action {
                    action: 0,
                    on_zero: 3,
                    on_one: 4,
                },
                PolicyNode::Claim { hypothesis: 0 },
                PolicyNode::Claim { hypothesis: 0 },
                PolicyNode::Claim { hypothesis: 0 },
            ],
            root: 0,
            max_steps: 2,
        };
        let report = evaluate_shared_policy(&policy, &[world], 0).unwrap();
        assert_eq!(
            report.per_world[0].expected_cost,
            Fraction::new(4, 1).unwrap()
        );
    }

    #[test]
    fn explicit_reset_is_charged_and_changes_the_next_observation() {
        let machine = HypothesisMachine {
            initial_state: 0,
            transitions: vec![
                vec![
                    Transition::Safe {
                        observation: false,
                        next_state: 1,
                    },
                    Transition::Safe {
                        observation: false,
                        next_state: 1,
                    },
                ],
                vec![
                    Transition::Safe {
                        observation: false,
                        next_state: 0,
                    },
                    Transition::Safe {
                        observation: false,
                        next_state: 0,
                    },
                ],
                vec![
                    Transition::Safe {
                        observation: false,
                        next_state: 0,
                    },
                    Transition::Safe {
                        observation: true,
                        next_state: 1,
                    },
                ],
            ],
        };
        let world = World {
            name: "reset".into(),
            hypothesis_ids: vec!["h0".into()],
            action_ids: vec!["condition".into(), "reset".into(), "measure".into()],
            prior_weights: vec![1],
            action_costs: vec![1, 3, 2],
            machines: vec![machine],
        };
        let policy = PolicyTree {
            nodes: vec![
                PolicyNode::Action {
                    action: 0,
                    on_zero: 1,
                    on_one: 2,
                },
                PolicyNode::Action {
                    action: 1,
                    on_zero: 3,
                    on_one: 4,
                },
                PolicyNode::Abstain,
                PolicyNode::Action {
                    action: 2,
                    on_zero: 5,
                    on_one: 6,
                },
                PolicyNode::Abstain,
                PolicyNode::Claim { hypothesis: 0 },
                PolicyNode::Claim { hypothesis: 0 },
            ],
            root: 0,
            max_steps: 3,
        };
        let report = evaluate_shared_policy(&policy, &[world], 0).unwrap();
        assert_eq!(
            report.per_world[0].expected_cost,
            Fraction::new(6, 1).unwrap()
        );
    }

    #[test]
    fn hazard_terminates_before_any_observation_or_state_update() {
        let machine = HypothesisMachine {
            initial_state: 0,
            transitions: vec![vec![Transition::Hazard]],
        };
        let world = World {
            name: "hazard".into(),
            hypothesis_ids: vec!["h0".into()],
            action_ids: vec!["break".into()],
            prior_weights: vec![1],
            action_costs: vec![7],
            machines: vec![machine],
        };
        let policy = PolicyTree {
            nodes: vec![
                PolicyNode::Action {
                    action: 0,
                    on_zero: 1,
                    on_one: 2,
                },
                PolicyNode::Claim { hypothesis: 0 },
                PolicyNode::Abstain,
            ],
            root: 0,
            max_steps: 1,
        };
        let report = evaluate_shared_policy(&policy, &[world], 0).unwrap();
        assert_eq!(
            report.per_world[0].hazard_probability,
            Fraction::new(1, 1).unwrap()
        );
        assert_eq!(
            report.per_world[0].expected_cost,
            Fraction::new(7, 1).unwrap()
        );
        assert_eq!(
            report.per_world[0].identification_probability,
            Fraction::new(0, 1).unwrap()
        );
    }

    #[test]
    fn rejects_incomplete_or_out_of_range_machine_transitions() {
        let mut world = order_effect_world("bad", false);
        world.machines[0].transitions[0].pop();
        assert!(evaluate_shared_policy(&condition_then_measure_policy(), &[world], 0).is_err());

        let mut world = order_effect_world("bad-next", false);
        world.machines[0].transitions[0][0] = Transition::Safe {
            observation: false,
            next_state: 99,
        };
        assert!(evaluate_shared_policy(&condition_then_measure_policy(), &[world], 0).is_err());
    }

    #[test]
    fn rejects_policy_horizon_cycles_and_unreachable_nodes() {
        let mut over_horizon = condition_then_measure_policy();
        over_horizon.max_steps = 1;
        assert!(evaluate_shared_policy(
            &over_horizon,
            &[order_effect_world("horizon", false)],
            0
        )
        .unwrap_err()
        .contains("horizon"));

        let cyclic = PolicyTree {
            nodes: vec![
                PolicyNode::Action {
                    action: 0,
                    on_zero: 0,
                    on_one: 1,
                },
                PolicyNode::Abstain,
            ],
            root: 0,
            max_steps: 4,
        };
        assert!(evaluate_shared_policy(
            &cyclic,
            &[order_effect_world("cycle", false)],
            0
        )
        .unwrap_err()
        .contains("cycle"));

        let unreachable = PolicyTree {
            nodes: vec![
                PolicyNode::Claim { hypothesis: 0 },
                PolicyNode::Claim { hypothesis: 1 },
            ],
            root: 0,
            max_steps: 0,
        };
        assert!(evaluate_shared_policy(
            &unreachable,
            &[order_effect_world("unreachable", false)],
            0
        )
        .unwrap_err()
        .contains("unreachable"));
    }

    #[test]
    fn rejects_shared_nodes_and_inconsistent_world_action_identity() {
        let shared = PolicyTree {
            nodes: vec![
                PolicyNode::Action {
                    action: 0,
                    on_zero: 1,
                    on_one: 1,
                },
                PolicyNode::Abstain,
            ],
            root: 0,
            max_steps: 1,
        };
        assert!(evaluate_shared_policy(
            &shared,
            &[order_effect_world("shared", false)],
            0
        )
        .unwrap_err()
        .contains("shared"));

        let mut other = order_effect_world("other", false);
        other.action_ids[1] = "different-action".into();
        assert!(evaluate_shared_policy(
            &condition_then_measure_policy(),
            &[order_effect_world("base", false), other],
            0
        )
        .unwrap_err()
        .contains("action IDs"));
    }

    #[test]
    fn abstention_is_reported_separately_from_hazard_and_false_claims() {
        let world = order_effect_world("abstain", false);
        let policy = PolicyTree {
            nodes: vec![PolicyNode::Abstain],
            root: 0,
            max_steps: 0,
        };
        let report = evaluate_shared_policy(&policy, &[world], 0).unwrap();
        assert_eq!(
            report.per_world[0].abstention_probability,
            Fraction::new(1, 1).unwrap()
        );
        assert_eq!(
            report.per_world[0].identification_probability,
            Fraction::new(0, 1).unwrap()
        );
    }
}
