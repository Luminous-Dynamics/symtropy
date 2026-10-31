//! Bounded policy falsification against provenance-labelled candidate worlds.
//!
//! A candidate world may have less observation support than the policy's
//! original family. Its unreachable branches are projected away; a reachable
//! observation without a policy branch is retained as a counterexample.
//! Supported candidates are scored using the robust planner's independent raw
//! forward-mass replay, not by reusing this module's posterior projection.

use super::finite_horizon::{DecisionBranch, DecisionPlan};
use super::robust_finite_horizon::{
    replay_shared_policy_across_worlds, DecisionModelWorld, WorldDecisionObjectives,
};
use super::{DecisionLossTable, ObservationProtocol};
use crate::{
    filter_one_step, ActionRealizationKernel, BeliefMass, ObservationField, PublicObservation,
    Rational,
};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_FALSIFIER_CANDIDATES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateOrigin {
    /// Candidate generated from the falsifier's permitted search/training split.
    SearchTraining,
    /// Reserved for ambiguity-radius calibration; never admitted to search.
    Calibration,
    /// Sealed evaluation; must never be inspected by the falsifier.
    SealedEvaluation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelWorldProvenance {
    pub generator_id: String,
    pub generator_version: String,
    /// Canonical, human-auditable generator parameters and seed receipt.
    ///
    /// This is a descriptive receipt, not a cryptographic digest.
    pub parameter_receipt: String,
    pub origin: CandidateOrigin,
}

#[derive(Clone, Debug)]
pub struct PolicyFalsificationCandidate {
    pub world: DecisionModelWorld,
    pub provenance: ModelWorldProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidatePolicyOutcome {
    /// The candidate supports every observation encountered by the policy.
    /// Metrics were computed by the independent raw-mass replay.
    Supported {
        objectives: WorldDecisionObjectives,
    },
    /// A reachable public observation is not covered by the supplied policy.
    /// The preceding observations are in execution order.
    UncoveredObservation {
        command: usize,
        observation: PublicObservation,
        remaining_horizon: usize,
        preceding_observations: Vec<PublicObservation>,
    },
    /// Candidate kernel was invalid or incompatible with the reference schema.
    InvalidCandidate {
        reason: String,
    },
    /// The plan is malformed or violates the action/horizon/hazard contract.
    InvalidPolicy {
        reason: String,
        preceding_observations: Vec<PublicObservation>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidatePolicyEvaluation {
    pub world_id: String,
    pub provenance: ModelWorldProvenance,
    pub outcome: CandidatePolicyOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyFalsificationReport {
    pub reference_world_id: String,
    pub candidate_count: usize,
    pub evaluations: Vec<CandidatePolicyEvaluation>,
    /// Maximum expected cost plus terminal loss over supported candidates.
    /// Unsupported policies are reported separately and are not assigned an
    /// invented numeric penalty.
    pub worst_supported_world_id: Option<String>,
    pub worst_supported_total_loss: Option<Rational>,
    pub unsupported_world_ids: Vec<String>,
    pub invalid_world_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ProjectionFailure {
    UncoveredObservation {
        command: usize,
        observation: PublicObservation,
        remaining_horizon: usize,
        preceding_observations: Vec<PublicObservation>,
    },
    InvalidPolicy {
        reason: String,
        preceding_observations: Vec<PublicObservation>,
    },
}

fn same_public_schema(
    reference: &ActionRealizationKernel,
    candidate: &ActionRealizationKernel,
) -> bool {
    reference.hypothesis_ids == candidate.hypothesis_ids
        && reference.physical_state_count == candidate.physical_state_count
        && reference.sensor_state_count == candidate.sensor_state_count
        && reference.apparatus_state_count == candidate.apparatus_state_count
        && reference.command_ids == candidate.command_ids
        && reference.realized_action_ids == candidate.realized_action_ids
        && reference.readback_ids == candidate.readback_ids
        && reference.measurement_ids == candidate.measurement_ids
}

fn public_field(value: Option<usize>, observed: bool) -> ObservationField {
    if !observed {
        ObservationField::Any
    } else {
        match value {
            Some(value) => ObservationField::Value(value),
            None => ObservationField::Missing,
        }
    }
}

fn outcome_observation(
    outcome: &crate::TransitionOutcome,
    protocol: ObservationProtocol,
) -> PublicObservation {
    PublicObservation {
        hazard: outcome.hazard,
        readback: public_field(outcome.readback, protocol.observe_readback),
        measurement: public_field(outcome.measurement, protocol.observe_measurement),
    }
}

fn reachable_observations(
    kernel: &ActionRealizationKernel,
    belief: &[BeliefMass],
    command: usize,
    protocol: ObservationProtocol,
) -> Result<BTreeSet<PublicObservation>, String> {
    if command >= kernel.command_ids.len() {
        return Err("policy references a command outside the candidate vocabulary".into());
    }
    let mut observations = BTreeSet::new();
    for point in belief {
        let row = kernel
            .rows
            .iter()
            .find(|row| row.state == point.state && row.command == command)
            .ok_or_else(|| "validated kernel is missing a state-command row".to_string())?;
        for outcome in &row.outcomes {
            observations.insert(outcome_observation(outcome, protocol));
        }
    }
    if observations.is_empty() {
        return Err("candidate model has no reachable public observation".into());
    }
    Ok(observations)
}

/// Project a shared policy onto one candidate world without granting access to
/// that world's identity. Extra branches that are unreachable in this world
/// are removed; every reachable observation must already have a branch.
fn project_policy_to_candidate(
    kernel: &ActionRealizationKernel,
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    plan: &DecisionPlan,
    belief: &[BeliefMass],
    remaining_horizon: usize,
    preceding_observations: &[PublicObservation],
) -> Result<DecisionPlan, ProjectionFailure> {
    match plan {
        DecisionPlan::TerminalDecision { decision } => {
            if decision.is_some_and(|index| index >= table.decision_ids.len()) {
                return Err(ProjectionFailure::InvalidPolicy {
                    reason: "terminal policy references an invalid decision".into(),
                    preceding_observations: preceding_observations.to_vec(),
                });
            }
            Ok(plan.clone())
        }
        DecisionPlan::HazardTerminal => Err(ProjectionFailure::InvalidPolicy {
            reason: "hazard terminal cannot be used before an observed hazard".into(),
            preceding_observations: preceding_observations.to_vec(),
        }),
        DecisionPlan::Experiment { command, branches } => {
            if remaining_horizon == 0 {
                return Err(ProjectionFailure::InvalidPolicy {
                    reason: "policy exceeds its finite horizon".into(),
                    preceding_observations: preceding_observations.to_vec(),
                });
            }
            let observations = reachable_observations(kernel, belief, *command, protocol)
                .map_err(|reason| ProjectionFailure::InvalidPolicy {
                    reason,
                    preceding_observations: preceding_observations.to_vec(),
                })?;
            let branch_map: BTreeMap<_, _> = branches
                .iter()
                .map(|branch| (branch.observation, branch.plan.as_ref()))
                .collect();
            if branch_map.len() != branches.len() {
                return Err(ProjectionFailure::InvalidPolicy {
                    reason: "policy contains duplicate public-observation branches".into(),
                    preceding_observations: preceding_observations.to_vec(),
                });
            }

            let mut projected = Vec::with_capacity(observations.len());
            for observation in observations {
                let Some(child_plan) = branch_map.get(&observation).copied() else {
                    return Err(ProjectionFailure::UncoveredObservation {
                        command: *command,
                        observation,
                        remaining_horizon,
                        preceding_observations: preceding_observations.to_vec(),
                    });
                };
                let filtered = filter_one_step(kernel, belief, *command, observation).map_err(
                    |error| ProjectionFailure::InvalidPolicy {
                        reason: format!("candidate observation update failed: {error}"),
                        preceding_observations: preceding_observations.to_vec(),
                    },
                )?;
                let child = if filtered.terminal_hazard {
                    if *child_plan != DecisionPlan::HazardTerminal {
                        return Err(ProjectionFailure::InvalidPolicy {
                            reason: "candidate hazard observation does not terminate".into(),
                            preceding_observations: preceding_observations.to_vec(),
                        });
                    }
                    DecisionPlan::HazardTerminal
                } else {
                    if *child_plan == DecisionPlan::HazardTerminal {
                        return Err(ProjectionFailure::InvalidPolicy {
                            reason: "safe candidate observation uses a hazard terminal".into(),
                            preceding_observations: preceding_observations.to_vec(),
                        });
                    }
                    let next_belief = filtered
                        .belief_for_next_step()
                        .map_err(|error| ProjectionFailure::InvalidPolicy {
                            reason: format!("candidate next belief failed: {error}"),
                            preceding_observations: preceding_observations.to_vec(),
                        })?;
                    let mut history = preceding_observations.to_vec();
                    history.push(observation);
                    project_policy_to_candidate(
                        kernel,
                        table,
                        protocol,
                        child_plan,
                        &next_belief,
                        remaining_horizon - 1,
                        &history,
                    )?
                };
                projected.push(DecisionBranch {
                    observation,
                    plan: Box::new(child),
                });
            }
            Ok(DecisionPlan::Experiment {
                command: *command,
                branches: projected,
            })
        }
    }
}

/// Evaluate a fixed policy against search-origin candidate worlds.
///
/// The reference world supplies the stable vocabulary/semantics contract.
/// Calibration and sealed-evaluation origins are rejected before any model is
/// inspected. This prevents accidental adaptive holdout leakage. Candidate
/// model generation and physical-plausibility admission remain separate steps.
pub fn falsify_shared_policy_against_candidates(
    reference_world: &DecisionModelWorld,
    candidates: &[PolicyFalsificationCandidate],
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    plan: &DecisionPlan,
    horizon: usize,
) -> Result<PolicyFalsificationReport, String> {
    reference_world.kernel.validate()?;
    table.validate(&reference_world.kernel)?;
    if command_costs.len() != reference_world.kernel.command_ids.len() {
        return Err("one command cost is required for every reference command".into());
    }
    if candidates.is_empty() || candidates.len() > MAX_FALSIFIER_CANDIDATES {
        return Err(format!(
            "falsifier candidate count must be in 1..={MAX_FALSIFIER_CANDIDATES}"
        ));
    }
    if horizon > super::finite_horizon::MAX_DECISION_HORIZON {
        return Err("falsifier horizon exceeds the finite decision horizon".into());
    }
    if matches!(plan, DecisionPlan::HazardTerminal) {
        return Err("campaign cannot begin in a hazard-terminal state".into());
    }

    let mut sorted: Vec<_> = candidates.iter().collect();
    sorted.sort_by(|left, right| left.world.id.cmp(&right.world.id));
    let mut ids = BTreeSet::new();
    for candidate in &sorted {
        if candidate.provenance.origin != CandidateOrigin::SearchTraining {
            return Err(format!(
                "candidate {} is not from the permitted search/training split",
                candidate.world.id
            ));
        }
        if candidate.world.id.trim().is_empty()
            || !ids.insert(candidate.world.id.as_str())
            || candidate.world.id == reference_world.id
        {
            return Err("candidate IDs must be non-empty, unique and distinct from reference ID".into());
        }
        if candidate.provenance.generator_id.trim().is_empty()
            || candidate.provenance.generator_version.trim().is_empty()
            || candidate.provenance.parameter_receipt.trim().is_empty()
        {
            return Err(format!(
                "candidate {} is missing deterministic generator provenance",
                candidate.world.id
            ));
        }
    }

    let mut evaluations = Vec::with_capacity(sorted.len());
    let mut worst_supported: Option<(String, Rational)> = None;
    let mut unsupported_world_ids = Vec::new();
    let mut invalid_world_ids = Vec::new();

    for candidate in sorted {
        let id = candidate.world.id.clone();
        let outcome = match candidate.world.kernel.validate() {
            Err(reason) => CandidatePolicyOutcome::InvalidCandidate { reason },
            Ok(()) if !same_public_schema(&reference_world.kernel, &candidate.world.kernel) => {
                CandidatePolicyOutcome::InvalidCandidate {
                    reason: "candidate state/action/observation vocabulary is incompatible with reference"
                        .into(),
                }
            }
            Ok(()) => match table.validate(&candidate.world.kernel) {
                Err(reason) => CandidatePolicyOutcome::InvalidCandidate { reason },
                Ok(()) => {
                    let initial_belief = &candidate.world.kernel.initial_belief;
                    match project_policy_to_candidate(
                        &candidate.world.kernel,
                        table,
                        protocol,
                        plan,
                        initial_belief,
                        horizon,
                        &[],
                    ) {
                        Err(ProjectionFailure::UncoveredObservation {
                            command,
                            observation,
                            remaining_horizon,
                            preceding_observations,
                        }) => CandidatePolicyOutcome::UncoveredObservation {
                            command,
                            observation,
                            remaining_horizon,
                            preceding_observations,
                        },
                        Err(ProjectionFailure::InvalidPolicy {
                            reason,
                            preceding_observations,
                        }) => CandidatePolicyOutcome::InvalidPolicy {
                            reason,
                            preceding_observations,
                        },
                        Ok(projected_plan) => {
                            match replay_shared_policy_across_worlds(
                                &[candidate.world.clone()],
                                &candidate.world.id,
                                table,
                                protocol,
                                command_costs,
                                hazard_loss,
                                &projected_plan,
                                horizon,
                            ) {
                                Err(reason) => CandidatePolicyOutcome::InvalidPolicy {
                                    reason,
                                    preceding_observations: Vec::new(),
                                },
                                Ok(replay) => CandidatePolicyOutcome::Supported {
                                    objectives: replay.per_world.into_iter().next().ok_or_else(
                                        || "single-world raw replay returned no objective".to_string(),
                                    )?,
                                },
                            }
                        }
                    }
                }
            },
        };

        if let CandidatePolicyOutcome::Supported { objectives } = &outcome {
            let total = objectives.expected_total_loss()?;
            if worst_supported
                .as_ref()
                .is_none_or(|(_, current)| total > *current)
            {
                worst_supported = Some((id.clone(), total));
            }
        } else if matches!(outcome, CandidatePolicyOutcome::UncoveredObservation { .. }) {
            unsupported_world_ids.push(id.clone());
        } else if matches!(outcome, CandidatePolicyOutcome::InvalidCandidate { .. }) {
            invalid_world_ids.push(id.clone());
        }

        evaluations.push(CandidatePolicyEvaluation {
            world_id: id,
            provenance: candidate.provenance.clone(),
            outcome,
        });
    }

    Ok(PolicyFalsificationReport {
        reference_world_id: reference_world.id.clone(),
        candidate_count: evaluations.len(),
        evaluations,
        worst_supported_world_id: worst_supported.as_ref().map(|(id, _)| id.clone()),
        worst_supported_total_loss: worst_supported.map(|(_, loss)| loss),
        unsupported_world_ids,
        invalid_world_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BeliefMass, KernelRow, LatentState, TransitionOutcome};

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn make_world(id: &str, measurement: usize) -> DecisionModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            let state = LatentState {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                apparatus_state: 0,
            };
            rows.push(KernelRow {
                state,
                command: 0,
                outcomes: vec![TransitionOutcome {
                    probability: Rational::one(),
                    next_state: state,
                    realized_action: 0,
                    readback: Some(0),
                    measurement: Some(measurement),
                    hazard: false,
                }],
            });
        }
        DecisionModelWorld {
            id: id.into(),
            kernel: ActionRealizationKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                apparatus_state_count: 1,
                command_ids: vec!["measure".into()],
                realized_action_ids: vec!["performed".into()],
                readback_ids: vec!["ack".into()],
                measurement_ids: vec!["m0".into(), "m1".into()],
                initial_belief: vec![
                    BeliefMass {
                        state: LatentState { hypothesis: 0, physical_state: 0, sensor_state: 0, apparatus_state: 0 },
                        mass: half(),
                    },
                    BeliefMass {
                        state: LatentState { hypothesis: 1, physical_state: 0, sensor_state: 0, apparatus_state: 0 },
                        mass: half(),
                    },
                ],
                rows,
            },
        }
    }

    fn loss_table() -> DecisionLossTable {
        DecisionLossTable {
            decision_ids: vec!["claim-h0".into(), "claim-h1".into()],
            losses: vec![
                vec![
                    vec![vec![vec![Rational::zero()]]],
                    vec![vec![vec![Rational::one()]]],
                ],
                vec![
                    vec![vec![vec![Rational::one()]]],
                    vec![vec![vec![Rational::zero()]]],
                ],
            ],
            abstention_loss: Rational::one(),
        }
    }

    fn provenance(origin: CandidateOrigin, id: &str) -> ModelWorldProvenance {
        ModelWorldProvenance {
            generator_id: "unit-test-generator".into(),
            generator_version: "1".into(),
            parameter_receipt: format!("fixture={id};seed=0"),
            origin,
        }
    }

    fn policy_covering_both_measurements() -> DecisionPlan {
        let branch = |measurement, decision| DecisionBranch {
            observation: PublicObservation {
                hazard: false,
                readback: ObservationField::Value(0),
                measurement: ObservationField::Value(measurement),
            },
            plan: Box::new(DecisionPlan::TerminalDecision {
                decision: Some(decision),
            }),
        };
        DecisionPlan::Experiment {
            command: 0,
            branches: vec![branch(0, 0), branch(1, 1)],
        }
    }

    fn costs() -> Vec<Rational> {
        vec![Rational::new(1, 10).unwrap()]
    }

    #[test]
    fn uncovered_public_observation_is_reported_with_public_trace() {
        let reference = make_world("reference", 0);
        let candidate = PolicyFalsificationCandidate {
            world: make_world("shifted", 1),
            provenance: provenance(CandidateOrigin::SearchTraining, "sign-flip"),
        };
        let policy = DecisionPlan::Experiment {
            command: 0,
            branches: vec![DecisionBranch {
                observation: PublicObservation {
                    hazard: false,
                    readback: ObservationField::Value(0),
                    measurement: ObservationField::Value(0),
                },
                plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(0) }),
            }],
        };
        let report = falsify_shared_policy_against_candidates(
            &reference,
            &[candidate],
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &policy,
            1,
        )
        .unwrap();
        assert_eq!(report.unsupported_world_ids, vec!["shifted".to_string()]);
        assert!(matches!(
            report.evaluations[0].outcome,
            CandidatePolicyOutcome::UncoveredObservation {
                observation: PublicObservation {
                    measurement: ObservationField::Value(1),
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn supported_candidate_ignores_unreachable_policy_branches_and_is_raw_replayed() {
        let reference = make_world("reference", 0);
        let candidate = PolicyFalsificationCandidate {
            world: make_world("same-support", 0),
            provenance: provenance(CandidateOrigin::SearchTraining, "same-support"),
        };
        let report = falsify_shared_policy_against_candidates(
            &reference,
            &[candidate],
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &policy_covering_both_measurements(),
            1,
        )
        .unwrap();
        let CandidatePolicyOutcome::Supported { objectives } = &report.evaluations[0].outcome else {
            panic!("candidate should be supported");
        };
        assert_eq!(objectives.expected_experiment_cost, Rational::new(1, 10).unwrap());
        assert_eq!(objectives.expected_terminal_loss, half());
        assert_eq!(report.worst_supported_total_loss, Some(Rational::new(3, 5).unwrap()));
    }

    #[test]
    fn calibration_and_sealed_worlds_are_rejected_before_evaluation() {
        let reference = make_world("reference", 0);
        for origin in [CandidateOrigin::Calibration, CandidateOrigin::SealedEvaluation] {
            let candidate = PolicyFalsificationCandidate {
                world: make_world("held-out", 0),
                provenance: provenance(origin, "held-out"),
            };
            assert!(falsify_shared_policy_against_candidates(
                &reference,
                &[candidate],
                &loss_table(),
                ObservationProtocol::fully_observed(),
                &costs(),
                Rational::one(),
                &policy_covering_both_measurements(),
                1,
            )
            .is_err());
        }
    }

    #[test]
    fn missing_provenance_and_incompatible_schema_fail_closed() {
        let reference = make_world("reference", 0);
        let mut candidate = PolicyFalsificationCandidate {
            world: make_world("candidate", 0),
            provenance: provenance(CandidateOrigin::SearchTraining, "candidate"),
        };
        candidate.provenance.parameter_receipt.clear();
        assert!(falsify_shared_policy_against_candidates(
            &reference,
            &[candidate],
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &policy_covering_both_measurements(),
            1,
        )
        .is_err());

        let mut incompatible = make_world("other", 0);
        incompatible.kernel.measurement_ids.swap(0, 1);
        let candidate = PolicyFalsificationCandidate {
            world: incompatible,
            provenance: provenance(CandidateOrigin::SearchTraining, "incompatible"),
        };
        let report = falsify_shared_policy_against_candidates(
            &reference,
            &[candidate],
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &policy_covering_both_measurements(),
            1,
        )
        .unwrap();
        assert!(matches!(
            report.evaluations[0].outcome,
            CandidatePolicyOutcome::InvalidCandidate { .. }
        ));
    }
}
