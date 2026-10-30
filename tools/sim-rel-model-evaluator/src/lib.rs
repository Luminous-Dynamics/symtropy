//! Independent exact evaluation of one adaptive policy across coherent finite worlds.
//!
//! This crate deliberately does not synthesize or optimize policies. It replays the same
//! observable-history policy under every supplied world and reports exact rational metrics.
//! The evaluator receives no true-world or evaluator-target argument.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::collections::BTreeSet;

const MAX_POLICY_NODES: usize = 4_096;
const MAX_ACTIONS: usize = 64;

/// An exact non-negative rational number with a reduced numerator and denominator.
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

/// One node in a deterministic policy tree.
///
/// The action selected at a node depends only on the observable history encoded by
/// the path through the tree. There is no model-index or evaluator-target input.
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

/// A shared observable-history policy tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyTree {
    pub nodes: Vec<PolicyNode>,
    pub root: usize,
}

/// A complete coherent world: a prior, outcomes and hazards over shared hypothesis IDs.
///
/// Rows in outcomes and hazards are indexed by action; columns are indexed by hypothesis.
/// A hazard terminates the campaign before its outcome bit is observed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct World {
    pub name: String,
    pub prior_weights: Vec<u64>,
    pub action_costs: Vec<u64>,
    pub outcomes: Vec<Vec<bool>>,
    pub hazards: Vec<Vec<bool>>,
}

/// Exact metrics for one fixed complete world.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldMetrics {
    pub world: String,
    pub expected_cost: Fraction,
    pub hazard_probability: Fraction,
    pub identification_probability: Fraction,
    pub false_identification_probability: Fraction,
    pub abstention_probability: Fraction,
}

/// Per-world results plus robust summaries across the supplied finite model family.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationReport {
    pub nominal_world: String,
    pub nominal_expected_cost: Fraction,
    pub worst_case_expected_cost: Fraction,
    pub worst_case_hazard_probability: Fraction,
    pub worst_case_identification_probability: Fraction,
    pub worst_case_false_identification_probability: Fraction,
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
    fn validate(&self) -> Result<usize, String> {
        if self.name.trim().is_empty() {
            return Err("world name must not be empty".into());
        }
        let hypothesis_count = self.prior_weights.len();
        if hypothesis_count == 0 {
            return Err(format!("world {} has no hypotheses", self.name));
        }
        let action_count = self.action_costs.len();
        if action_count == 0 || action_count > MAX_ACTIONS {
            return Err(format!(
                "world {} must have between 1 and {MAX_ACTIONS} actions",
                self.name
            ));
        }
        if self.prior_weights.iter().all(|weight| *weight == 0) {
            return Err(format!("world {} has zero total prior mass", self.name));
        }
        if self.action_costs.contains(&0) {
            return Err(format!("world {} contains a non-positive action cost", self.name));
        }
        if self.outcomes.len() != action_count || self.hazards.len() != action_count {
            return Err(format!(
                "world {} action table count does not match action costs",
                self.name
            ));
        }
        for (action, (outcomes, hazards)) in
            self.outcomes.iter().zip(self.hazards.iter()).enumerate()
        {
            if outcomes.len() != hypothesis_count || hazards.len() != hypothesis_count {
                return Err(format!(
                    "world {} action {action} has an invalid hypothesis-column count",
                    self.name
                ));
            }
        }
        self.prior_weights.iter().try_fold(0_u128, |total, weight| {
            total
                .checked_add(u128::from(*weight))
                .ok_or_else(|| format!("world {} prior mass overflowed u128", self.name))
        })?;
        Ok(hypothesis_count)
    }
}

impl PolicyTree {
    fn validate(&self, hypothesis_count: usize, action_count: usize) -> Result<(), String> {
        if self.nodes.is_empty() || self.nodes.len() > MAX_POLICY_NODES {
            return Err(format!(
                "policy must contain between 1 and {MAX_POLICY_NODES} nodes"
            ));
        }
        if action_count == 0 || action_count > MAX_ACTIONS {
            return Err(format!("policy action count must be in 1..={MAX_ACTIONS}"));
        }

        let mut stack = Vec::new();
        let mut reachable = BTreeSet::new();
        self.validate_node(
            self.root,
            0,
            hypothesis_count,
            action_count,
            &mut stack,
            &mut reachable,
        )?;
        if reachable.len() != self.nodes.len() {
            return Err("policy contains unreachable nodes".into());
        }
        Ok(())
    }

    fn validate_node(
        &self,
        index: usize,
        used_actions: u64,
        hypothesis_count: usize,
        action_count: usize,
        stack: &mut Vec<usize>,
        reachable: &mut BTreeSet<usize>,
    ) -> Result<(), String> {
        if index >= self.nodes.len() {
            return Err(format!("policy references missing node {index}"));
        }
        if stack.contains(&index) {
            return Err(format!("policy contains a cycle through node {index}"));
        }
        reachable.insert(index);
        stack.push(index);

        let result = match &self.nodes[index] {
            PolicyNode::Action {
                action,
                on_zero,
                on_one,
            } => {
                if *action >= action_count {
                    Err(format!("policy references missing action {action}"))
                } else {
                    let bit = 1_u64
                        .checked_shl(*action as u32)
                        .ok_or_else(|| "action mask shift overflowed".to_string())?;
                    if used_actions & bit != 0 {
                        Err(format!("policy repeats action {action} on one path"))
                    } else {
                        let next_used = used_actions | bit;
                        self.validate_node(
                            *on_zero,
                            next_used,
                            hypothesis_count,
                            action_count,
                            stack,
                            reachable,
                        )?;
                        self.validate_node(
                            *on_one,
                            next_used,
                            hypothesis_count,
                            action_count,
                            stack,
                            reachable,
                        )
                    }
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

        stack.pop();
        result
    }
}

/// Evaluate one fixed policy under every supplied complete world.
///
/// The same policy tree is replayed in every world. Costs are paid when an action is
/// attempted, including when that action immediately causes a hazard. Zero prior weights
/// are permitted, but each world must have positive total mass. All reported probabilities
/// and expected costs are exact reduced fractions.
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

    let hypothesis_count = worlds[0].validate()?;
    let action_costs = &worlds[0].action_costs;
    let mut names = BTreeSet::new();
    for world in worlds {
        let count = world.validate()?;
        if count != hypothesis_count || world.action_costs.as_slice() != action_costs.as_slice() {
            return Err("all worlds must share hypothesis IDs and action costs".into());
        }
        if !names.insert(world.name.as_str()) {
            return Err(format!("duplicate world name {}", world.name));
        }
    }
    policy.validate(hypothesis_count, action_costs.len())?;

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

    Ok(EvaluationReport {
        nominal_world: nominal.world.clone(),
        nominal_expected_cost: nominal.expected_cost,
        worst_case_expected_cost,
        worst_case_hazard_probability,
        worst_case_identification_probability,
        worst_case_false_identification_probability,
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

    Ok(WorldMetrics {
        world: world.name.clone(),
        expected_cost: Fraction::new(weighted_cost, total_mass)?,
        hazard_probability: Fraction::new(hazard_mass, total_mass)?,
        identification_probability: Fraction::new(identification_mass, total_mass)?,
        false_identification_probability: Fraction::new(false_identification_mass, total_mass)?,
        abstention_probability: Fraction::new(abstention_mass, total_mass)?,
    })
}

fn replay_hypothesis(
    policy: &PolicyTree,
    world: &World,
    hypothesis: usize,
) -> Result<(u128, Terminal), String> {
    let mut node_index = policy.root;
    let mut cost = 0_u128;

    for _ in 0..=world.action_costs.len() {
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
                let action_cost = *world
                    .action_costs
                    .get(*action)
                    .ok_or_else(|| format!("policy references missing action {action}"))?;
                cost = cost
                    .checked_add(u128::from(action_cost))
                    .ok_or_else(|| "realized policy cost overflowed u128".to_string())?;
                if world.hazards[*action][hypothesis] {
                    return Ok((cost, Terminal::Hazard));
                }
                node_index = if world.outcomes[*action][hypothesis] {
                    *on_one
                } else {
                    *on_zero
                };
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

    Err("policy exceeded the maximum number of distinct actions on a path".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_world(name: &str, weights: [u64; 4]) -> World {
        World {
            name: name.into(),
            prior_weights: weights.to_vec(),
            action_costs: vec![1, 3, 1],
            outcomes: vec![
                vec![false, false, true, false],
                vec![false, false, true, true],
                vec![false, true, false, true],
            ],
            hazards: vec![
                vec![false, false, false, true],
                vec![false, false, false, false],
                vec![false, false, false, false],
            ],
        }
    }

    fn two_worlds() -> Vec<World> {
        vec![
            fixture_world("nominal", [35, 30, 30, 5]),
            fixture_world("prior-shift", [20, 20, 30, 30]),
        ]
    }

    fn risky_policy() -> PolicyTree {
        PolicyTree {
            nodes: vec![
                PolicyNode::Action {
                    action: 0,
                    on_zero: 1,
                    on_one: 2,
                },
                PolicyNode::Action {
                    action: 2,
                    on_zero: 3,
                    on_one: 4,
                },
                PolicyNode::Claim { hypothesis: 2 },
                PolicyNode::Claim { hypothesis: 0 },
                PolicyNode::Claim { hypothesis: 1 },
            ],
            root: 0,
        }
    }

    fn safe_policy() -> PolicyTree {
        PolicyTree {
            nodes: vec![
                PolicyNode::Action {
                    action: 2,
                    on_zero: 1,
                    on_one: 4,
                },
                PolicyNode::Action {
                    action: 0,
                    on_zero: 2,
                    on_one: 3,
                },
                PolicyNode::Claim { hypothesis: 0 },
                PolicyNode::Claim { hypothesis: 2 },
                PolicyNode::Action {
                    action: 1,
                    on_zero: 5,
                    on_one: 6,
                },
                PolicyNode::Claim { hypothesis: 1 },
                PolicyNode::Claim { hypothesis: 3 },
            ],
            root: 0,
        }
    }

    #[test]
    fn exact_fraction_order_does_not_overflow_cross_products() {
        let left = Fraction::new(u128::MAX - 1, u128::MAX).unwrap();
        let right = Fraction::new(u128::MAX - 2, u128::MAX - 1).unwrap();
        assert!(left > right);
        assert_eq!(Fraction::new(50, 100).unwrap(), Fraction::new(1, 2).unwrap());
        assert!(Fraction::new(1, 0).is_err());
    }

    #[test]
    fn nominally_safe_policy_is_rejected_by_robust_risk_summary() {
        let report = evaluate_shared_policy(&risky_policy(), &two_worlds(), 0).unwrap();
        assert_eq!(report.nominal_expected_cost, Fraction::new(165, 100).unwrap());
        assert_eq!(report.worst_case_expected_cost, Fraction::new(165, 100).unwrap());
        assert_eq!(
            report.per_world[0].hazard_probability,
            Fraction::new(5, 100).unwrap()
        );
        assert_eq!(
            report.per_world[1].hazard_probability,
            Fraction::new(30, 100).unwrap()
        );
        assert_eq!(
            report.worst_case_hazard_probability,
            Fraction::new(3, 10).unwrap()
        );
        assert_eq!(
            report.worst_case_identification_probability,
            Fraction::new(7, 10).unwrap()
        );
    }

    #[test]
    fn one_safe_policy_tree_is_replayed_under_every_world() {
        let report = evaluate_shared_policy(&safe_policy(), &two_worlds(), 0).unwrap();
        assert_eq!(report.nominal_expected_cost, Fraction::new(27, 10).unwrap());
        assert_eq!(report.worst_case_expected_cost, Fraction::new(3, 1).unwrap());
        assert_eq!(report.worst_case_hazard_probability, Fraction::new(0, 1).unwrap());
        assert_eq!(
            report.worst_case_identification_probability,
            Fraction::new(1, 1).unwrap()
        );
        assert!(report.per_world.iter().all(|metric| {
            metric.false_identification_probability == Fraction::new(0, 1).unwrap()
                && metric.abstention_probability == Fraction::new(0, 1).unwrap()
        }));
    }

    #[test]
    fn incorrect_leaf_claims_are_not_counted_as_identification() {
        let world = World {
            name: "wrong-claim".into(),
            prior_weights: vec![1, 3],
            action_costs: vec![1],
            outcomes: vec![vec![false, true]],
            hazards: vec![vec![false, false]],
        };
        let policy = PolicyTree {
            nodes: vec![PolicyNode::Claim { hypothesis: 0 }],
            root: 0,
        };
        let report = evaluate_shared_policy(&policy, &[world], 0).unwrap();
        assert_eq!(
            report.per_world[0].identification_probability,
            Fraction::new(1, 4).unwrap()
        );
        assert_eq!(
            report.per_world[0].false_identification_probability,
            Fraction::new(3, 4).unwrap()
        );
    }

    #[test]
    fn action_cost_is_paid_before_an_immediate_hazard() {
        let worlds = two_worlds();
        let report = evaluate_shared_policy(&risky_policy(), &worlds, 0).unwrap();
        assert_eq!(report.per_world[0].expected_cost, Fraction::new(165, 100).unwrap());
        assert_eq!(report.per_world[1].expected_cost, Fraction::new(140, 100).unwrap());
    }

    #[test]
    fn rejects_repeated_action_on_a_policy_path() {
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
                PolicyNode::Claim { hypothesis: 1 },
                PolicyNode::Claim { hypothesis: 0 },
                PolicyNode::Claim { hypothesis: 1 },
            ],
            root: 0,
        };
        let error = evaluate_shared_policy(&policy, &two_worlds(), 0).unwrap_err();
        assert!(error.contains("repeats action"));
    }

    #[test]
    fn rejects_cycles_and_unreachable_nodes() {
        let cyclic = PolicyTree {
            nodes: vec![PolicyNode::Action {
                action: 0,
                on_zero: 0,
                on_one: 1,
            }, PolicyNode::Claim { hypothesis: 0 }],
            root: 0,
        };
        assert!(evaluate_shared_policy(&cyclic, &two_worlds(), 0)
            .unwrap_err()
            .contains("cycle"));

        let unreachable = PolicyTree {
            nodes: vec![
                PolicyNode::Claim { hypothesis: 0 },
                PolicyNode::Claim { hypothesis: 1 },
            ],
            root: 0,
        };
        assert!(evaluate_shared_policy(&unreachable, &two_worlds(), 0)
            .unwrap_err()
            .contains("unreachable"));
    }

    #[test]
    fn rejects_incoherent_world_dimensions_and_different_action_costs() {
        let mut malformed = fixture_world("malformed", [35, 30, 30, 5]);
        malformed.outcomes.pop();
        assert!(evaluate_shared_policy(&safe_policy(), &[malformed], 0).is_err());

        let mut worlds = two_worlds();
        worlds[1].action_costs[0] = 2;
        assert!(evaluate_shared_policy(&safe_policy(), &worlds, 0)
            .unwrap_err()
            .contains("share hypothesis IDs and action costs"));
    }

    #[test]
    fn rejects_zero_total_prior_and_duplicate_world_names() {
        let mut world = fixture_world("empty", [0, 0, 0, 0]);
        assert!(world.validate().is_err());

        let duplicate = vec![
            fixture_world("same", [1, 1, 1, 1]),
            fixture_world("same", [2, 2, 2, 2]),
        ];
        assert!(evaluate_shared_policy(&safe_policy(), &duplicate, 0)
            .unwrap_err()
            .contains("duplicate world name"));
    }

    #[test]
    fn abstention_is_reported_separately_from_failure_and_hazard() {
        let world = World {
            name: "abstain".into(),
            prior_weights: vec![2, 2],
            action_costs: vec![1],
            outcomes: vec![vec![false, true]],
            hazards: vec![vec![false, false]],
        };
        let policy = PolicyTree {
            nodes: vec![PolicyNode::Abstain],
            root: 0,
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
