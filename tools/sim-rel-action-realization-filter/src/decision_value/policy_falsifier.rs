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
pub const MAX_PERTURBATION_SCENARIOS: usize = 128;
pub const MAX_PERTURBATIONS_PER_SCENARIO: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KernelPerturbation {
    TransferOutcomeProbability {
        state: crate::LatentState,
        command: usize,
        donor_outcome: usize,
        receiver_outcome: usize,
        amount: Rational,
    },
    SuppressReadback {
        state: crate::LatentState,
        command: usize,
        outcome: usize,
    },
    SwapMeasurementLabels {
        state: crate::LatentState,
        command: usize,
        left_outcome: usize,
        right_outcome: usize,
    },
    RemapRealizedAction {
        state: crate::LatentState,
        command: usize,
        outcome: usize,
        realized_action: usize,
    },
    PromoteSafeOutcomeToHazard {
        state: crate::LatentState,
        command: usize,
        outcome: usize,
    },
    SetNextApparatusState {
        state: crate::LatentState,
        command: usize,
        outcome: usize,
        apparatus_state: usize,
    },
    /// Ordered, bounded edits applied atomically to one candidate clone.
    Sequence(Vec<KernelPerturbation>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PerturbationScenario {
    pub scenario_id: String,
    pub generator_version: String,
    /// Canonical parameter and seed receipt supplied by the scenario author.
    pub parameter_receipt: String,
    pub perturbation: KernelPerturbation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelWorldProvenance {
    pub parent_world_id: String,
    pub generator_id: String,
    pub generator_version: String,
    /// Canonical, human-auditable generator parameters and seed receipt.
    ///
    /// This is a descriptive receipt, not a cryptographic digest.
    pub parameter_receipt: String,
    pub origin: CandidateOrigin,
}

fn checked_sub_probability(left: Rational, right: Rational) -> Result<Rational, String> {
    if right > left {
        return Err("probability transfer exceeds donor outcome mass".into());
    }
    let mut a = left.denominator();
    let mut b = right.denominator();
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    let gcd = a.max(1);
    let left_scale = right.denominator() / gcd;
    let right_scale = left.denominator() / gcd;
    let numerator_left = left
        .numerator()
        .checked_mul(left_scale)
        .ok_or_else(|| "probability subtraction numerator overflow".to_string())?;
    let numerator_right = right
        .numerator()
        .checked_mul(right_scale)
        .ok_or_else(|| "probability subtraction numerator overflow".to_string())?;
    let denominator = left
        .denominator()
        .checked_mul(left_scale)
        .ok_or_else(|| "probability subtraction denominator overflow".to_string())?;
    Rational::new(numerator_left - numerator_right, denominator)
}

fn find_outcome_mut(
    kernel: &mut ActionRealizationKernel,
    state: crate::LatentState,
    command: usize,
    outcome: usize,
) -> Result<&mut crate::TransitionOutcome, String> {
    let row = kernel
        .rows
        .iter_mut()
        .find(|row| row.state == state && row.command == command)
        .ok_or_else(|| "perturbation references an unknown state-command row".to_string())?;
    row.outcomes
        .get_mut(outcome)
        .ok_or_else(|| "perturbation references an unknown outcome index".to_string())
}

/// Apply one bounded structural perturbation while preserving kernel validity.
///
/// This operator intentionally does not assert physical plausibility. It
/// constructs transparent synthetic stress worlds; a separate, domain-specific
/// admission check must decide which candidates are physically admissible.
fn apply_kernel_perturbation(
    kernel: &mut ActionRealizationKernel,
    perturbation: &KernelPerturbation,
) -> Result<(), String> {
    match perturbation {
        KernelPerturbation::TransferOutcomeProbability {
            state,
            command,
            donor_outcome,
            receiver_outcome,
            amount,
        } => {
            if donor_outcome == receiver_outcome || amount.is_zero() || !amount.is_probability() {
                return Err("probability transfer requires distinct outcomes and positive probability".into());
            }
            let row = kernel
                .rows
                .iter_mut()
                .find(|row| row.state == *state && row.command == *command)
                .ok_or_else(|| "probability transfer references an unknown row".to_string())?;
            if *donor_outcome >= row.outcomes.len() || *receiver_outcome >= row.outcomes.len() {
                return Err("probability transfer outcome index is out of range".into());
            }
            let donor = row.outcomes[*donor_outcome].probability;
            let receiver = row.outcomes[*receiver_outcome].probability;
            let donor_after = checked_sub_probability(donor, *amount)?;
            let receiver_after = receiver.checked_add(*amount)?;
            if receiver_after > Rational::one() {
                return Err("probability transfer exceeds receiver simplex capacity".into());
            }
            row.outcomes[*donor_outcome].probability = donor_after;
            row.outcomes[*receiver_outcome].probability = receiver_after;
        }
        KernelPerturbation::SuppressReadback {
            state,
            command,
            outcome,
        } => {
            let target = find_outcome_mut(kernel, *state, *command, *outcome)?;
            if target.readback.is_none() {
                return Err("readback suppression target is already missing".into());
            }
            target.readback = None;
        }
        KernelPerturbation::SwapMeasurementLabels {
            state,
            command,
            left_outcome,
            right_outcome,
        } => {
            if left_outcome == right_outcome {
                return Err("measurement swap requires distinct outcomes".into());
            }
            let row = kernel
                .rows
                .iter_mut()
                .find(|row| row.state == *state && row.command == *command)
                .ok_or_else(|| "measurement swap references an unknown row".to_string())?;
            if *left_outcome >= row.outcomes.len() || *right_outcome >= row.outcomes.len() {
                return Err("measurement swap outcome index is out of range".into());
            }
            let left = row.outcomes[*left_outcome].measurement;
            let right = row.outcomes[*right_outcome].measurement;
            if left.is_none() || right.is_none() {
                return Err("measurement swap requires two safe measured outcomes".into());
            }
            if left == right {
                return Err("measurement swap would not change the model".into());
            }
            row.outcomes[*left_outcome].measurement = right;
            row.outcomes[*right_outcome].measurement = left;
        }
        KernelPerturbation::RemapRealizedAction {
            state,
            command,
            outcome,
            realized_action,
        } => {
            if *realized_action >= kernel.realized_action_ids.len() {
                return Err("realized-action remap target is out of range".into());
            }
            let target = find_outcome_mut(kernel, *state, *command, *outcome)?;
            if target.realized_action == *realized_action {
                return Err("realized-action remap would not change the model".into());
            }
            target.realized_action = *realized_action;
        }
        KernelPerturbation::PromoteSafeOutcomeToHazard {
            state,
            command,
            outcome,
        } => {
            let target = find_outcome_mut(kernel, *state, *command, *outcome)?;
            if target.hazard {
                return Err("hazard promotion target is already hazardous".into());
            }
            target.hazard = true;
            target.measurement = None;
        }
        KernelPerturbation::SetNextApparatusState {
            state,
            command,
            outcome,
            apparatus_state,
        } => {
            if *apparatus_state >= kernel.apparatus_state_count {
                return Err("apparatus transition target is out of range".into());
            }
            let target = find_outcome_mut(kernel, *state, *command, *outcome)?;
            if target.next_state.apparatus_state == *apparatus_state {
                return Err("apparatus transition would not change the model".into());
            }
            target.next_state.apparatus_state = *apparatus_state;
        }
        KernelPerturbation::Sequence(edits) => {
            if edits.is_empty() || edits.len() > MAX_PERTURBATIONS_PER_SCENARIO {
                return Err(format!(
                    "perturbation sequence length must be in 1..={MAX_PERTURBATIONS_PER_SCENARIO}"
                ));
            }
            for edit in edits {
                if matches!(edit, KernelPerturbation::Sequence(_)) {
                    return Err("nested perturbation sequences are not permitted".into());
                }
                apply_kernel_perturbation(kernel, edit)?;
            }
        }
    }
    Ok(())
}

/// Generate deterministic single-edit synthetic candidate worlds from a base
/// kernel. Scenarios are sorted by ID, IDs are namespaced under the parent
/// world, and every emitted kernel must pass full kernel validation.
pub fn generate_search_perturbation_candidates(
    parent: &DecisionModelWorld,
    scenarios: &[PerturbationScenario],
) -> Result<Vec<PolicyFalsificationCandidate>, String> {
    parent.kernel.validate()?;
    if parent.id.trim().is_empty() {
        return Err("parent model-world ID must be non-empty".into());
    }
    if scenarios.is_empty() || scenarios.len() > MAX_PERTURBATION_SCENARIOS {
        return Err(format!(
            "perturbation scenario count must be in 1..={MAX_PERTURBATION_SCENARIOS}"
        ));
    }
    let mut sorted: Vec<_> = scenarios.iter().collect();
    sorted.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    let mut ids = BTreeSet::new();
    let mut generated = Vec::with_capacity(sorted.len());
    for scenario in sorted {
        if scenario.scenario_id.trim().is_empty()
            || !ids.insert(scenario.scenario_id.as_str())
            || scenario.generator_version.trim().is_empty()
            || scenario.parameter_receipt.trim().is_empty()
        {
            return Err("perturbation IDs, versions and parameter receipts must be non-empty and unique".into());
        }
        let world_id = format!("{}::{}", parent.id, scenario.scenario_id);
        let mut world = DecisionModelWorld {
            id: world_id,
            kernel: parent.kernel.clone(),
        };
        apply_kernel_perturbation(&mut world.kernel, &scenario.perturbation)?;
        world.kernel.validate()?;
        generated.push(PolicyFalsificationCandidate {
            world,
            provenance: ModelWorldProvenance {
                parent_world_id: parent.id.clone(),
                generator_id: "sim-rel-029-single-edit-kernel-perturbations".into(),
                generator_version: scenario.generator_version.clone(),
                parameter_receipt: scenario.parameter_receipt.clone(),
                origin: CandidateOrigin::SearchTraining,
            },
        });
    }
    Ok(generated)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateOrigin {
    /// Candidate generated from the falsifier's permitted search/training split.
    SearchTraining,
    /// Reserved for ambiguity-radius calibration; never admitted to search.
    Calibration,
    /// Sealed evaluation; must never be inspected by the falsifier.
    SealedEvaluation,
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
    use crate::{KernelRow, LatentState, TransitionOutcome};

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

    fn make_two_apparatus_state_world(id: &str) -> DecisionModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            for apparatus_state in 0..2 {
                let state = LatentState {
                    hypothesis,
                    physical_state: 0,
                    sensor_state: 0,
                    apparatus_state,
                };
                rows.push(KernelRow {
                    state,
                    command: 0,
                    outcomes: vec![TransitionOutcome {
                        probability: Rational::one(),
                        next_state: state,
                        realized_action: 0,
                        readback: Some(0),
                        measurement: Some(0),
                        hazard: false,
                    }],
                });
            }
        }
        DecisionModelWorld {
            id: id.into(),
            kernel: ActionRealizationKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                apparatus_state_count: 2,
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

    fn provenance(origin: CandidateOrigin, id: &str) -> ModelWorldProvenance {
        ModelWorldProvenance {
            parent_world_id: "reference".into(),
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

    fn make_two_outcome_world(id: &str) -> DecisionModelWorld {
        let mut world = make_world(id, 0);
        for row in &mut world.kernel.rows {
            let state = row.state;
            row.outcomes = vec![
                TransitionOutcome {
                    probability: half(),
                    next_state: state,
                    realized_action: 0,
                    readback: Some(0),
                    measurement: Some(0),
                    hazard: false,
                },
                TransitionOutcome {
                    probability: half(),
                    next_state: state,
                    realized_action: 0,
                    readback: Some(0),
                    measurement: Some(1),
                    hazard: false,
                },
            ];
        }
        world
    }

    #[test]
    fn single_edit_generator_emits_valid_provenance_linked_worlds_deterministically() {
        let mut parent = make_world("parent", 0);
        parent.kernel.realized_action_ids.push("not-performed".into());
        let state_h0 = parent.kernel.initial_belief[0].state;
        let scenarios = vec![
            PerturbationScenario {
                scenario_id: "readback-loss".into(),
                generator_version: "1".into(),
                parameter_receipt: "state=h0;command=0;outcome=0".into(),
                perturbation: KernelPerturbation::SuppressReadback {
                    state: state_h0,
                    command: 0,
                    outcome: 0,
                },
            },
            PerturbationScenario {
                scenario_id: "action-drop".into(),
                generator_version: "1".into(),
                parameter_receipt: "state=h0;command=0;outcome=0;action=1".into(),
                perturbation: KernelPerturbation::RemapRealizedAction {
                    state: state_h0,
                    command: 0,
                    outcome: 0,
                    realized_action: 1,
                },
            },
            PerturbationScenario {
                scenario_id: "hazard-promotion".into(),
                generator_version: "1".into(),
                parameter_receipt: "state=h0;command=0;outcome=0".into(),
                perturbation: KernelPerturbation::PromoteSafeOutcomeToHazard {
                    state: state_h0,
                    command: 0,
                    outcome: 0,
                },
            },
        ];
        let generated = generate_search_perturbation_candidates(&parent, &scenarios).unwrap();
        assert_eq!(
            generated.iter().map(|candidate| candidate.world.id.as_str()).collect::<Vec<_>>(),
            vec![
                "parent::action-drop",
                "parent::hazard-promotion",
                "parent::readback-loss",
            ]
        );
        for candidate in &generated {
            candidate.world.kernel.validate().unwrap();
            assert_eq!(candidate.provenance.parent_world_id, "parent");
            assert_eq!(candidate.provenance.origin, CandidateOrigin::SearchTraining);
            assert_eq!(
                candidate.provenance.generator_id,
                "sim-rel-029-single-edit-kernel-perturbations"
            );
        }
        assert_eq!(generated[0].world.kernel.rows[0].outcomes[0].realized_action, 1);
        assert!(generated[1].world.kernel.rows[0].outcomes[0].hazard);
        assert_eq!(generated[2].world.kernel.rows[0].outcomes[0].readback, None);
    }

    #[test]
    fn perturbation_generator_transfers_exact_probability_mass_and_swaps_measurements() {
        let parent = make_two_outcome_world("stochastic");
        let state = parent.kernel.initial_belief[0].state;
        let scenarios = vec![
            PerturbationScenario {
                scenario_id: "probability-shift".into(),
                generator_version: "1".into(),
                parameter_receipt: "donor=0;receiver=1;amount=1/4".into(),
                perturbation: KernelPerturbation::TransferOutcomeProbability {
                    state,
                    command: 0,
                    donor_outcome: 0,
                    receiver_outcome: 1,
                    amount: Rational::new(1, 4).unwrap(),
                },
            },
            PerturbationScenario {
                scenario_id: "measurement-swap".into(),
                generator_version: "1".into(),
                parameter_receipt: "left=0;right=1".into(),
                perturbation: KernelPerturbation::SwapMeasurementLabels {
                    state,
                    command: 0,
                    left_outcome: 0,
                    right_outcome: 1,
                },
            },
        ];
        let generated = generate_search_perturbation_candidates(&parent, &scenarios).unwrap();
        let shifted = &generated[1].world.kernel.rows[0].outcomes;
        assert_eq!(shifted[0].probability, Rational::new(1, 4).unwrap());
        assert_eq!(shifted[1].probability, Rational::new(3, 4).unwrap());
        let swapped = &generated[0].world.kernel.rows[0].outcomes;
        assert_eq!(swapped[0].measurement, Some(1));
        assert_eq!(swapped[1].measurement, Some(0));
    }

    #[test]
    fn invalid_perturbations_fail_closed_without_emitting_partial_candidates() {
        let parent = make_two_outcome_world("stochastic");
        let state = parent.kernel.initial_belief[0].state;
        let scenarios = vec![
            PerturbationScenario {
                scenario_id: "a-valid-first".into(),
                generator_version: "1".into(),
                parameter_receipt: "swap".into(),
                perturbation: KernelPerturbation::SwapMeasurementLabels {
                    state,
                    command: 0,
                    left_outcome: 0,
                    right_outcome: 1,
                },
            },
            PerturbationScenario {
                scenario_id: "z-invalid-second".into(),
                generator_version: "1".into(),
                parameter_receipt: "too-much".into(),
                perturbation: KernelPerturbation::TransferOutcomeProbability {
                    state,
                    command: 0,
                    donor_outcome: 0,
                    receiver_outcome: 1,
                    amount: Rational::one(),
                },
            },
        ];
        assert!(generate_search_perturbation_candidates(&parent, &scenarios).is_err());
    }

    #[test]
    fn apparatus_degradation_perturbation_changes_only_the_next_persistent_state() {
        let parent = make_two_apparatus_state_world("apparatus");
        let state = parent.kernel.initial_belief[0].state;
        let generated = generate_search_perturbation_candidates(
            &parent,
            &[PerturbationScenario {
                scenario_id: "degrade-apparatus".into(),
                generator_version: "1".into(),
                parameter_receipt: "next_apparatus_state=1".into(),
                perturbation: KernelPerturbation::SetNextApparatusState {
                    state,
                    command: 0,
                    outcome: 0,
                    apparatus_state: 1,
                },
            }],
        )
        .unwrap();
        assert_eq!(
            generated[0].world.kernel.rows[0].outcomes[0].next_state.apparatus_state,
            1
        );
        assert_eq!(
            generated[0].world.kernel.initial_belief,
            parent.kernel.initial_belief
        );
        generated[0].world.kernel.validate().unwrap();
    }

    #[test]
    fn bounded_multi_factor_sequence_captures_probability_concentrated_on_hazard() {
        let parent = make_two_outcome_world("interaction");
        let state = parent.kernel.initial_belief[0].state;
        let generated = generate_search_perturbation_candidates(
            &parent,
            &[PerturbationScenario {
                scenario_id: "probability-to-hazard".into(),
                generator_version: "1".into(),
                parameter_receipt: "transfer=1/4;promote=outcome1".into(),
                perturbation: KernelPerturbation::Sequence(vec![
                    KernelPerturbation::TransferOutcomeProbability {
                        state,
                        command: 0,
                        donor_outcome: 0,
                        receiver_outcome: 1,
                        amount: Rational::new(1, 4).unwrap(),
                    },
                    KernelPerturbation::PromoteSafeOutcomeToHazard {
                        state,
                        command: 0,
                        outcome: 1,
                    },
                ]),
            }],
        )
        .unwrap();
        let outcomes = &generated[0].world.kernel.rows[0].outcomes;
        assert_eq!(outcomes[0].probability, Rational::new(1, 4).unwrap());
        assert_eq!(outcomes[1].probability, Rational::new(3, 4).unwrap());
        assert!(outcomes[1].hazard);
        assert_eq!(outcomes[1].measurement, None);
        generated[0].world.kernel.validate().unwrap();
    }

    #[test]
    fn perturbation_sequences_fail_closed_on_empty_nested_and_over_budget_edits() {
        let parent = make_world("parent", 0);
        let scenarios = vec![
            PerturbationScenario {
                scenario_id: "empty".into(),
                generator_version: "1".into(),
                parameter_receipt: "empty".into(),
                perturbation: KernelPerturbation::Sequence(Vec::new()),
            },
        ];
        assert!(generate_search_perturbation_candidates(&parent, &scenarios).is_err());

        let nested = PerturbationScenario {
            scenario_id: "nested".into(),
            generator_version: "1".into(),
            parameter_receipt: "nested".into(),
            perturbation: KernelPerturbation::Sequence(vec![
                KernelPerturbation::Sequence(vec![
                    KernelPerturbation::PromoteSafeOutcomeToHazard {
                        state: parent.kernel.initial_belief[0].state,
                        command: 0,
                        outcome: 0,
                    },
                ]),
            ]),
        };
        assert!(generate_search_perturbation_candidates(&parent, &[nested]).is_err());

        let over_budget = PerturbationScenario {
            scenario_id: "over-budget".into(),
            generator_version: "1".into(),
            parameter_receipt: "nine-edits".into(),
            perturbation: KernelPerturbation::Sequence(vec![
                KernelPerturbation::PromoteSafeOutcomeToHazard {
                    state: parent.kernel.initial_belief[0].state,
                    command: 0,
                    outcome: 0,
                };
                MAX_PERTURBATIONS_PER_SCENARIO + 1
            ]),
        };
        assert!(generate_search_perturbation_candidates(&parent, &[over_budget]).is_err());
    }

    #[test]
    fn no_op_perturbations_are_rejected() {
        let parent = make_world("parent", 0);
        let state = parent.kernel.initial_belief[0].state;
        let once = generate_search_perturbation_candidates(
            &parent,
            &[PerturbationScenario {
                scenario_id: "suppress".into(),
                generator_version: "1".into(),
                parameter_receipt: "suppress-readback".into(),
                perturbation: KernelPerturbation::SuppressReadback {
                    state,
                    command: 0,
                    outcome: 0,
                },
            }],
        )
        .unwrap();
        let second_suppression = PerturbationScenario {
            scenario_id: "suppress-again".into(),
            generator_version: "1".into(),
            parameter_receipt: "suppress-already-missing-readback".into(),
            perturbation: KernelPerturbation::SuppressReadback {
                state,
                command: 0,
                outcome: 0,
            },
        };
        assert!(generate_search_perturbation_candidates(
            &once[0].world,
            &[second_suppression]
        )
        .is_err());
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
