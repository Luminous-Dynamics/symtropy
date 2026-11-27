// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact finite verifier for observation-conditioned, non-anticipative policies.
//!
//! v0.1 supports one frozen decision epoch after a declared observation window.
//! A policy maps only the measurement content available by that epoch to a
//! hard-risk-eligible portfolio. Hidden scenario/world IDs and future
//! observations are excluded from the policy key.
//!
//! Candidate action trajectories come from the shared-exogenous trajectory
//! receipt report. This verifies finite identity and non-anticipativity of the
//! supplied policy mapping; it does not establish physical observation
//! completeness, out-of-sample policy integrity, or statistical coverage.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::counterfactual_trajectory_receipt::{
    CounterfactualTrajectoryReceiptReport, ScenarioAtomIdentity, digest_receipt_report,
};
use crate::paired_objective_pareto::validate_and_digest_gate;
use crate::scenario_mission_risk_gate::ScenarioMissionRiskGateReport;

const MAX_WORLDS: usize = 65_536;
const MAX_OBSERVATIONS_PER_WORLD: usize = 65_536;
const MAX_POLICY_BRANCHES: usize = 1_000_000;

/// One measurement that could be available to the policy at the frozen epoch.
///
/// source_evidence_digest is integrity provenance for the verifier. It is
/// deliberately excluded from the policy-visible history digest: the policy
/// observes the sensor channel, sequence, delivery time, value and declared
/// quality, not hidden-world receipt metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservableMeasurement {
    /// Observable sensor/channel identity.
    pub sensor_id: [u8; 32],
    /// Observable monotonically increasing sample sequence for this channel.
    pub sample_sequence: u64,
    /// First decision epoch at which this sample is available to the policy.
    pub available_at_epoch: u32,
    /// Digest of the measurement value and its public representation.
    pub value_digest: [u8; 32],
    /// Digest of the observable quality/uncertainty representation.
    pub quality_digest: [u8; 32],
    /// Source receipt for integrity/audit; not a policy branching variable.
    pub source_evidence_digest: [u8; 32],
}

/// Complete declared measurement inventory for one scenario/latent world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyWorldObservationTrace {
    /// Epistemic scenario identity (hidden from the policy mapping).
    pub scenario_id: [u8; 32],
    /// Latent world / trajectory atom identity (hidden from the policy mapping).
    pub world_id: [u8; 32],
    /// Digest of the complete canonical measurement inventory as supplied.
    pub observation_inventory_digest: [u8; 32],
    /// Every declared measurement, including those arriving after the decision.
    pub measurements: Vec<ObservableMeasurement>,
}

/// One node in the finite policy map.
///
/// The node is indexed only by the digest of the observable history at the
/// frozen decision epoch. It contains no scenario or latent-world identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonAnticipativePolicyNode {
    /// Canonical digest of the policy-visible measurement history.
    pub observable_history_digest: [u8; 32],
    /// Selected prequalified manoeuvre portfolio for this history.
    pub selected_portfolio_id: [u8; 32],
}

/// Frozen finite policy table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonAnticipativePolicyPlan {
    /// Stable nonzero policy identity.
    pub policy_id: [u8; 32],
    /// Digest of the policy implementation/configuration frozen by the caller.
    pub policy_code_digest: [u8; 32],
    /// Exactly one node for every observable history in the evaluation corpus.
    pub nodes: Vec<NonAnticipativePolicyNode>,
}

/// One evaluated policy decision for one hidden world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldPolicyDecision {
    /// Epistemic scenario identity; not part of the decision function.
    pub scenario_id: [u8; 32],
    /// Hidden latent world identity; not part of the decision function.
    pub world_id: [u8; 32],
    /// Decision epoch index within the frozen time schedule.
    pub decision_epoch: u32,
    /// Digest of only the measurements available by the decision epoch.
    pub observable_history_digest: [u8; 32],
    /// Action selected by the non-anticipative policy.
    pub selected_portfolio_id: [u8; 32],
    /// Exact trajectory digest for this portfolio in this scenario/world.
    pub selected_trajectory_digest: [u8; 32],
    /// Number of observations visible to the policy.
    pub visible_measurement_count: usize,
    /// Number of supplied observations that arrived after the decision.
    pub future_measurement_count: usize,
}

/// Complete finite policy verification report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonAnticipativePolicyReport {
    /// Stable policy identity.
    pub policy_id: [u8; 32],
    /// Policy implementation/configuration digest.
    pub policy_code_digest: [u8; 32],
    /// Frozen decision epoch index within the declared schedule.
    pub decision_epoch: u32,
    /// Digest mapping logical epoch indices to absolute instants and time-scale semantics.
    pub decision_schedule_digest: [u8; 32],
    /// Digest of the exact hard-risk gate.
    pub mission_risk_gate_digest: [u8; 32],
    /// Digest of the validated shared-exogenous trajectory receipt report.
    pub trajectory_receipt_digest: [u8; 32],
    /// Canonical scenario universe.
    pub scenario_ids: Vec<[u8; 32]>,
    /// Hard-risk-eligible action universe.
    pub eligible_portfolio_ids: Vec<[u8; 32]>,
    /// Actions excluded by the hard-risk gate.
    pub ineligible_portfolio_ids: Vec<[u8; 32]>,
    /// Canonical decisions for every scenario × latent-world row.
    pub world_decisions: Vec<WorldPolicyDecision>,
    /// Number of distinct observable histories in the evaluation corpus.
    pub unique_observable_history_count: usize,
    /// Number of hidden scenario/world rows covered exactly once.
    pub covered_world_count: usize,
    /// Digest of the frozen scenario/world/policy universe.
    pub frozen_input_digest: [u8; 32],
    /// Digest of all canonical observation inventories and policy decisions.
    pub result_digest: [u8; 32],
    /// Always false; a policy receipt has no manoeuvre authority.
    pub authority_granted: bool,
}

/// Fail-closed errors for the finite non-anticipative policy verifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonAnticipativePolicyError {
    /// Hard mission-risk gate is invalid or claims authority.
    InvalidMissionRiskGate,
    /// Trajectory report digest, gate identity or eligibility partition mismatches.
    InvalidTrajectoryReceiptReport,
    /// Decision epoch is zero or otherwise invalid.
    InvalidDecisionEpoch,
    /// The decision time-schedule identity is zero.
    InvalidDecisionSchedule,
    /// Policy identity or implementation digest is zero.
    InvalidPolicyIdentity,
    /// Frozen world universe is empty, too large, duplicated or malformed.
    InvalidWorldUniverse,
    /// Trace world is unknown, duplicated or missing.
    IncompleteWorldObservationMatrix,
    /// Measurement identity, value, time or source receipt is invalid.
    InvalidObservation,
    /// Two measurements reuse one observable sensor sequence identity.
    DuplicateMeasurementIdentity,
    /// Declared observation inventory digest does not match supplied contents.
    ObservationInventoryDigestMismatch,
    /// A policy branch identity is zero or duplicated.
    DuplicateOrInvalidPolicyBranch,
    /// No policy branch exists for an observable history in the corpus.
    MissingPolicyBranch,
    /// Policy contains a branch not used by any admitted observable history.
    UnusedPolicyBranch,
    /// Selected action is not in the hard-risk-eligible action set.
    IneligiblePolicyAction,
    /// Trajectory receipt for the selected action is missing or mismatched.
    PolicyActionTrajectoryMismatch,
}

impl core::fmt::Display for NonAnticipativePolicyError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidMissionRiskGate => "hard mission-risk gate is invalid or grants authority",
            Self::InvalidTrajectoryReceiptReport => {
                "trajectory receipt report does not match the hard-risk gate"
            }
            Self::InvalidDecisionEpoch => "policy decision epoch must be positive",
            Self::InvalidDecisionSchedule => {
                "policy decision schedule identity must be nonzero"
            },
            Self::InvalidPolicyIdentity => "policy identity or implementation digest is zero",
            Self::InvalidWorldUniverse => "frozen scenario/world universe is invalid",
            Self::IncompleteWorldObservationMatrix => {
                "policy observation traces do not cover every frozen world exactly once"
            }
            Self::InvalidObservation => "observation identity, time or provenance is invalid",
            Self::DuplicateMeasurementIdentity => {
                "measurement reuses a sensor sequence identity"
            }
            Self::ObservationInventoryDigestMismatch => {
                "observation inventory digest does not match canonical contents"
            }
            Self::DuplicateOrInvalidPolicyBranch => {
                "policy branch identity is zero or duplicated"
            }
            Self::MissingPolicyBranch => "policy has no action for an observed history",
            Self::UnusedPolicyBranch => {
                "policy includes a branch outside the frozen observable-history corpus"
            }
            Self::IneligiblePolicyAction => {
                "policy selected a portfolio outside the hard-risk-eligible set"
            }
            Self::PolicyActionTrajectoryMismatch => {
                "selected portfolio has no matching trajectory receipt for this world"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for NonAnticipativePolicyError {}

/// Frozen campaign that couples observation traces to an independently checked
/// hard-risk gate and shared-exogenous trajectory receipt report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonAnticipativePolicyCampaign {
    decision_epoch: u32,
    decision_schedule_digest: [u8; 32],
    mission_risk_gate_digest: [u8; 32],
    trajectory_receipt_digest: [u8; 32],
    scenario_ids: Vec<[u8; 32]>,
    world_ids: Vec<ScenarioAtomIdentity>,
    eligible_portfolio_ids: Vec<[u8; 32]>,
    ineligible_portfolio_ids: Vec<[u8; 32]>,
    trajectory_digest_by_world_portfolio: BTreeMap<([u8; 32], [u8; 32], [u8; 32]), [u8; 32]>,
    frozen_input_digest: [u8; 32],
}

impl NonAnticipativePolicyCampaign {
    /// Freeze the observable decision epoch and exact hard-risk/trajectory universe.
    ///
    /// The hard-risk gate is independently revalidated. The trajectory report's
    /// result digest is recomputed from its public contents before its selected
    /// portfolio trajectories are used as action evidence.
    pub fn new(
        gate: &ScenarioMissionRiskGateReport,
        trajectory_report: &CounterfactualTrajectoryReceiptReport,
        decision_epoch: u32,
        decision_schedule_digest: [u8; 32],
    ) -> Result<Self, NonAnticipativePolicyError> {
        if decision_epoch == 0 {
            return Err(NonAnticipativePolicyError::InvalidDecisionEpoch);
        }
        if is_zero(&decision_schedule_digest) {
            return Err(NonAnticipativePolicyError::InvalidDecisionSchedule);
        }
        let mission_risk_gate_digest = validate_and_digest_gate(gate)
            .map_err(|_| NonAnticipativePolicyError::InvalidMissionRiskGate)?;

        let mut gate_eligible = gate.eligible_portfolio_ids.clone();
        let mut gate_ineligible = gate.ineligible_portfolio_ids.clone();
        gate_eligible.sort_unstable();
        gate_ineligible.sort_unstable();

        if trajectory_report.authority_granted
            || trajectory_report.mission_risk_gate_digest != mission_risk_gate_digest
            || is_zero(&trajectory_report.result_digest)
            || digest_receipt_report(trajectory_report) != trajectory_report.result_digest
            || trajectory_report.scenario_ids.is_empty()
            || trajectory_report.scenario_ids.len() != gate.scenario_count
            || trajectory_report.eligible_portfolio_ids != gate_eligible
            || trajectory_report.ineligible_portfolio_ids != gate_ineligible
        {
            return Err(NonAnticipativePolicyError::InvalidTrajectoryReceiptReport);
        }

        let gate_scenarios = gate
            .scenario_portfolio_risk_checks
            .iter()
            .map(|check| check.scenario_id)
            .collect::<BTreeSet<_>>();
        let supplied_scenarios = trajectory_report
            .scenario_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if supplied_scenarios.len() != trajectory_report.scenario_ids.len()
            || supplied_scenarios != gate_scenarios
        {
            return Err(NonAnticipativePolicyError::InvalidTrajectoryReceiptReport);
        }

        let mut derived_world_ids = Vec::new();
        let mut trajectory_digest_by_world_portfolio = BTreeMap::new();
        for atom in &trajectory_report.canonical_atoms {
            derived_world_ids.push(ScenarioAtomIdentity {
                scenario_id: atom.shared_world.scenario_id,
                atom_id: atom.shared_world.atom_id,
            });
            for receipt in &atom.portfolio_trajectories {
                if trajectory_digest_by_world_portfolio
                    .insert(
                        (receipt.scenario_id, receipt.atom_id, receipt.portfolio_id),
                        receipt.propagated_trajectory_digest,
                    )
                    .is_some()
                {
                    return Err(NonAnticipativePolicyError::InvalidTrajectoryReceiptReport);
                }
            }
        }
        derived_world_ids.sort_unstable();
        if derived_world_ids.is_empty()
            || derived_world_ids.len() > MAX_WORLDS
            || derived_world_ids.windows(2).any(|pair| pair[0] == pair[1])
            || derived_world_ids != trajectory_report.scenario_atom_ids
        {
            return Err(NonAnticipativePolicyError::InvalidTrajectoryReceiptReport);
        }

        let eligible_set = trajectory_report
            .eligible_portfolio_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let world_set = derived_world_ids.iter().copied().collect::<BTreeSet<_>>();
        let expected_trajectory_count = derived_world_ids
            .len()
            .checked_mul(eligible_set.len())
            .ok_or(NonAnticipativePolicyError::InvalidTrajectoryReceiptReport)?;
        let trajectory_keys_are_exact = trajectory_digest_by_world_portfolio
            .keys()
            .all(|(scenario_id, world_id, portfolio_id)| {
                world_set.contains(&ScenarioAtomIdentity {
                    scenario_id: *scenario_id,
                    atom_id: *world_id,
                }) && eligible_set.contains(portfolio_id)
            }) && derived_world_ids.iter().all(|world| {
            eligible_set.iter().all(|portfolio_id| {
                trajectory_digest_by_world_portfolio.contains_key(&(
                    world.scenario_id,
                    world.atom_id,
                    *portfolio_id,
                ))
            })
        });
        if trajectory_digest_by_world_portfolio.len() != expected_trajectory_count
            || trajectory_digest_by_world_portfolio.values().any(is_zero)
            || !trajectory_keys_are_exact
        {
            return Err(NonAnticipativePolicyError::InvalidTrajectoryReceiptReport);
        }

        let mut scenario_ids = trajectory_report.scenario_ids.clone();
        scenario_ids.sort_unstable();
        let mut eligible_portfolio_ids = trajectory_report.eligible_portfolio_ids.clone();
        eligible_portfolio_ids.sort_unstable();
        let mut ineligible_portfolio_ids = trajectory_report.ineligible_portfolio_ids.clone();
        ineligible_portfolio_ids.sort_unstable();

        let frozen_input_digest = digest_frozen_input(&FrozenPolicyInput {
            decision_epoch,
            decision_schedule_digest,
            gate_digest: mission_risk_gate_digest,
            trajectory_receipt_digest: trajectory_report.result_digest,
            scenarios: &scenario_ids,
            worlds: &derived_world_ids,
            eligible: &eligible_portfolio_ids,
            ineligible: &ineligible_portfolio_ids,
        });

        Ok(Self {
            decision_epoch,
            decision_schedule_digest,
            mission_risk_gate_digest,
            trajectory_receipt_digest: trajectory_report.result_digest,
            scenario_ids,
            world_ids: derived_world_ids,
            eligible_portfolio_ids,
            ineligible_portfolio_ids,
            trajectory_digest_by_world_portfolio,
            frozen_input_digest,
        })
    }

    /// Return the number of hidden scenario/world traces required.
    pub fn expected_world_count(&self) -> usize {
        self.world_ids.len()
    }

    /// Return the exact decision epoch frozen before policy evaluation.
    pub fn decision_epoch(&self) -> u32 {
        self.decision_epoch
    }

    /// Return the digest of the frozen gate/world/action universe.
    pub fn frozen_input_digest(&self) -> [u8; 32] {
        self.frozen_input_digest
    }

    /// Verify that a finite policy maps observation-equivalent worlds to the
    /// same eligible portfolio, independent of hidden scenario/world IDs.
    pub fn evaluate(
        &self,
        plan: &NonAnticipativePolicyPlan,
        traces: &[PolicyWorldObservationTrace],
    ) -> Result<NonAnticipativePolicyReport, NonAnticipativePolicyError> {
        if is_zero(&plan.policy_id) || is_zero(&plan.policy_code_digest) {
            return Err(NonAnticipativePolicyError::InvalidPolicyIdentity);
        }
        if traces.len() != self.world_ids.len() || traces.len() > MAX_WORLDS {
            return Err(NonAnticipativePolicyError::IncompleteWorldObservationMatrix);
        }
        if plan.nodes.is_empty() || plan.nodes.len() > MAX_POLICY_BRANCHES {
            return Err(NonAnticipativePolicyError::InvalidPolicyIdentity);
        }

        let mut policy_nodes = BTreeMap::new();
        for node in &plan.nodes {
            if is_zero(&node.observable_history_digest)
                || is_zero(&node.selected_portfolio_id)
                || policy_nodes
                    .insert(node.observable_history_digest, node.selected_portfolio_id)
                    .is_some()
            {
                return Err(NonAnticipativePolicyError::DuplicateOrInvalidPolicyBranch);
            }
            if self
                .eligible_portfolio_ids
                .binary_search(&node.selected_portfolio_id)
                .is_err()
            {
                return Err(NonAnticipativePolicyError::IneligiblePolicyAction);
            }
        }

        let expected_worlds = self.world_ids.iter().copied().collect::<BTreeSet<_>>();
        let mut seen_worlds = BTreeSet::new();
        let mut world_decisions = Vec::with_capacity(traces.len());
        let mut used_policy_nodes = BTreeSet::new();
        let mut observation_inventory_digests = BTreeMap::new();

        for trace in traces {
            let identity = ScenarioAtomIdentity {
                scenario_id: trace.scenario_id,
                atom_id: trace.world_id,
            };
            if !expected_worlds.contains(&identity) || !seen_worlds.insert(identity) {
                return Err(NonAnticipativePolicyError::IncompleteWorldObservationMatrix);
            }
            if trace.measurements.len() > MAX_OBSERVATIONS_PER_WORLD
                || is_zero(&trace.observation_inventory_digest)
            {
                return Err(NonAnticipativePolicyError::InvalidObservation);
            }

            let mut measurements = trace.measurements.clone();
            measurements.sort_unstable_by_key(|measurement| {
                (
                    measurement.available_at_epoch,
                    measurement.sensor_id,
                    measurement.sample_sequence,
                )
            });
            if measurements.iter().any(|measurement| {
                is_zero(&measurement.sensor_id)
                    || measurement.sample_sequence == 0
                    || is_zero(&measurement.value_digest)
                    || is_zero(&measurement.quality_digest)
                    || is_zero(&measurement.source_evidence_digest)
            }) {
                return Err(NonAnticipativePolicyError::InvalidObservation);
            }
            if measurements.windows(2).any(|pair| {
                pair[0].sensor_id == pair[1].sensor_id
                    && pair[0].sample_sequence == pair[1].sample_sequence
            }) {
                return Err(NonAnticipativePolicyError::DuplicateMeasurementIdentity);
            }

            let inventory_digest = digest_observation_inventory(&measurements);
            if inventory_digest != trace.observation_inventory_digest {
                return Err(NonAnticipativePolicyError::ObservationInventoryDigestMismatch);
            }

            let visible_measurements = measurements
                .iter()
                .filter(|measurement| measurement.available_at_epoch <= self.decision_epoch)
                .copied()
                .collect::<Vec<_>>();
            let observable_history_digest = digest_observable_history(
                self.decision_epoch,
                self.decision_schedule_digest,
                &visible_measurements,
            );
            let selected_portfolio_id = policy_nodes
                .get(&observable_history_digest)
                .copied()
                .ok_or(NonAnticipativePolicyError::MissingPolicyBranch)?;
            let selected_trajectory_digest = self
                .trajectory_digest_by_world_portfolio
                .get(&(trace.scenario_id, trace.world_id, selected_portfolio_id))
                .copied()
                .ok_or(NonAnticipativePolicyError::PolicyActionTrajectoryMismatch)?;

            used_policy_nodes.insert(observable_history_digest);
            observation_inventory_digests.insert(identity, inventory_digest);

            world_decisions.push(WorldPolicyDecision {
                scenario_id: trace.scenario_id,
                world_id: trace.world_id,
                decision_epoch: self.decision_epoch,
                observable_history_digest,
                selected_portfolio_id,
                selected_trajectory_digest,
                visible_measurement_count: visible_measurements.len(),
                future_measurement_count: measurements.len() - visible_measurements.len(),
            });
        }

        if seen_worlds != expected_worlds {
            return Err(NonAnticipativePolicyError::IncompleteWorldObservationMatrix);
        }
        if used_policy_nodes != policy_nodes.keys().copied().collect::<BTreeSet<_>>() {
            return Err(NonAnticipativePolicyError::UnusedPolicyBranch);
        }

        world_decisions.sort_unstable_by_key(|decision| {
            (decision.scenario_id, decision.world_id)
        });
        let unique_observable_history_count = used_policy_nodes.len();
        let mut report = NonAnticipativePolicyReport {
            policy_id: plan.policy_id,
            policy_code_digest: plan.policy_code_digest,
            decision_epoch: self.decision_epoch,
            decision_schedule_digest: self.decision_schedule_digest,
            mission_risk_gate_digest: self.mission_risk_gate_digest,
            trajectory_receipt_digest: self.trajectory_receipt_digest,
            scenario_ids: self.scenario_ids.clone(),
            eligible_portfolio_ids: self.eligible_portfolio_ids.clone(),
            ineligible_portfolio_ids: self.ineligible_portfolio_ids.clone(),
            world_decisions,
            unique_observable_history_count,
            covered_world_count: seen_worlds.len(),
            frozen_input_digest: self.frozen_input_digest,
            result_digest: [0; 32],
            authority_granted: false,
        };
        report.result_digest = digest_policy_report(&report, &observation_inventory_digests);
        Ok(report)
    }
}

struct FrozenPolicyInput<'a> {
    decision_epoch: u32,
    decision_schedule_digest: [u8; 32],
    gate_digest: [u8; 32],
    trajectory_receipt_digest: [u8; 32],
    scenarios: &'a [[u8; 32]],
    worlds: &'a [ScenarioAtomIdentity],
    eligible: &'a [[u8; 32]],
    ineligible: &'a [[u8; 32]],
}

fn digest_frozen_input(input: &FrozenPolicyInput<'_>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.non-anticipative-policy.frozen.v0.1\0");
    hasher.update(input.decision_epoch.to_be_bytes());
    hasher.update(input.decision_schedule_digest);
    hasher.update(input.gate_digest);
    hasher.update(input.trajectory_receipt_digest);
    hash_ids(&mut hasher, input.scenarios);
    hasher.update((input.worlds.len() as u64).to_be_bytes());
    for world in input.worlds {
        hasher.update(world.scenario_id);
        hasher.update(world.atom_id);
    }
    hash_ids(&mut hasher, input.eligible);
    hash_ids(&mut hasher, input.ineligible);
    hasher.finalize().into()
}

fn digest_observation_inventory(measurements: &[ObservableMeasurement]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.non-anticipative-policy.observation-inventory.v0.1\0");
    hasher.update((measurements.len() as u64).to_be_bytes());
    for measurement in measurements {
        hasher.update(measurement.sensor_id);
        hasher.update(measurement.sample_sequence.to_be_bytes());
        hasher.update(measurement.available_at_epoch.to_be_bytes());
        hasher.update(measurement.value_digest);
        hasher.update(measurement.quality_digest);
        hasher.update(measurement.source_evidence_digest);
    }
    hasher.finalize().into()
}

/// Digest only policy-visible evidence. Hidden world/scenario IDs, future
/// measurements and source receipt IDs are absent.
fn digest_observable_history(
    decision_epoch: u32,
    decision_schedule_digest: [u8; 32],
    visible_measurements: &[ObservableMeasurement],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.non-anticipative-policy.observable-history.v0.1\0");
    hasher.update(decision_epoch.to_be_bytes());
    hasher.update(decision_schedule_digest);
    hasher.update((visible_measurements.len() as u64).to_be_bytes());
    for measurement in visible_measurements {
        hasher.update(measurement.sensor_id);
        hasher.update(measurement.sample_sequence.to_be_bytes());
        hasher.update(measurement.available_at_epoch.to_be_bytes());
        hasher.update(measurement.value_digest);
        hasher.update(measurement.quality_digest);
    }
    hasher.finalize().into()
}

fn digest_policy_report(
    report: &NonAnticipativePolicyReport,
    observation_inventories: &BTreeMap<ScenarioAtomIdentity, [u8; 32]>,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.non-anticipative-policy.report.v0.1\0");
    hasher.update(report.policy_id);
    hasher.update(report.policy_code_digest);
    hasher.update(report.decision_epoch.to_be_bytes());
    hasher.update(report.decision_schedule_digest);
    hasher.update(report.mission_risk_gate_digest);
    hasher.update(report.trajectory_receipt_digest);
    hasher.update(report.frozen_input_digest);
    hash_ids(&mut hasher, &report.scenario_ids);
    hash_ids(&mut hasher, &report.eligible_portfolio_ids);
    hash_ids(&mut hasher, &report.ineligible_portfolio_ids);
    hasher.update((report.world_decisions.len() as u64).to_be_bytes());
    for decision in &report.world_decisions {
        hasher.update(decision.scenario_id);
        hasher.update(decision.world_id);
        hasher.update(decision.decision_epoch.to_be_bytes());
        hasher.update(decision.observable_history_digest);
        hasher.update(decision.selected_portfolio_id);
        hasher.update(decision.selected_trajectory_digest);
        hasher.update((decision.visible_measurement_count as u64).to_be_bytes());
        hasher.update((decision.future_measurement_count as u64).to_be_bytes());
        if let Some(inventory_digest) =
            observation_inventories.get(&ScenarioAtomIdentity {
                scenario_id: decision.scenario_id,
                atom_id: decision.world_id,
            })
        {
            hasher.update(inventory_digest);
        }
    }
    hasher.update((report.unique_observable_history_count as u64).to_be_bytes());
    hasher.update((report.covered_world_count as u64).to_be_bytes());
    hasher.finalize().into()
}

fn hash_ids(hasher: &mut Sha256, ids: &[[u8; 32]]) {
    hasher.update((ids.len() as u64).to_be_bytes());
    for id in ids {
        hasher.update(id);
    }
}

fn is_zero(id: &[u8; 32]) -> bool {
    id.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counterfactual_trajectory_receipt::{
        CounterfactualTrajectoryAtom, CounterfactualTrajectoryReceiptCampaign,
        PortfolioTrajectoryReceipt, ScenarioTrajectoryContract, SharedExogenousWorldReceipt,
    };
    use crate::mission_risk_aggregation::ExactMass;
    use crate::scenario_mission_risk_gate::PortfolioScenarioRiskCheck;

    const SCENARIO_A: [u8; 32] = [1; 32];
    const SCENARIO_B: [u8; 32] = [2; 32];
    const WORLD_1: [u8; 32] = [3; 32];
    const WORLD_2: [u8; 32] = [4; 32];
    const TEST_DECISION_SCHEDULE: [u8; 32] = [210; 32];
    const PORTFOLIO_A: [u8; 32] = [10; 32];
    const PORTFOLIO_B: [u8; 32] = [20; 32];
    const PORTFOLIO_C: [u8; 32] = [30; 32];

    fn risk_check(
        scenario_id: [u8; 32],
        portfolio_id: [u8; 32],
        eligible: bool,
    ) -> PortfolioScenarioRiskCheck {
        PortfolioScenarioRiskCheck {
            scenario_id,
            portfolio_id,
            manifest_digest: [80 + portfolio_id[0]; 32],
            target_measure_digest: if scenario_id == SCENARIO_A {
                [51; 32]
            } else {
                [52; 32]
            },
            density_space_digest: if scenario_id == SCENARIO_A {
                [53; 32]
            } else {
                [54; 32]
            },
            event_inventory_digest: [90 + portfolio_id[0]; 32],
            risk_envelope_digest: [100 + portfolio_id[0]; 32],
            total_mass_ticks: 100,
            post_action_horizon_digest: [111; 32],
            risk_policy_digest: [112; 32],
            mission_union_mass: if eligible {
                ExactMass {
                    numerator: 1,
                    denominator: 100,
                }
            } else {
                ExactMass {
                    numerator: 2,
                    denominator: 100,
                }
            },
            mission_risk_budget: ExactMass {
                numerator: 1,
                denominator: 100,
            },
            within_declared_finite_support_budget: eligible,
        }
    }

    fn gate() -> ScenarioMissionRiskGateReport {
        let checks = vec![
            risk_check(SCENARIO_A, PORTFOLIO_A, true),
            risk_check(SCENARIO_A, PORTFOLIO_B, true),
            risk_check(SCENARIO_A, PORTFOLIO_C, true),
            risk_check(SCENARIO_B, PORTFOLIO_A, true),
            risk_check(SCENARIO_B, PORTFOLIO_B, false),
            risk_check(SCENARIO_B, PORTFOLIO_C, true),
        ];
        ScenarioMissionRiskGateReport {
            total_family_alpha: 0.05,
            scenario_family_alpha: 0.025,
            contrast_alpha: 0.008_333_333_333_333_333,
            scenario_count: 2,
            portfolio_count: 3,
            consequence_unit_id: [113; 32],
            scenario_portfolio_risk_checks: checks,
            eligible_portfolio_ids: vec![PORTFOLIO_A, PORTFOLIO_C],
            ineligible_portfolio_ids: vec![PORTFOLIO_B],
            ineligible_scenarios_by_portfolio: BTreeMap::from([(PORTFOLIO_B, vec![SCENARIO_B])]),
            eligible_portfolio_regret_intervals: Vec::new(),
            unique_best_eligible_portfolio_id: None,
            minimax_regret_undominated_eligible_portfolio_ids: vec![PORTFOLIO_A, PORTFOLIO_C],
            authority_granted: false,
        }
    }

    fn contracts() -> Vec<ScenarioTrajectoryContract> {
        vec![
            ScenarioTrajectoryContract {
                scenario_id: SCENARIO_A,
                scenario_physics_configuration_digest: [60; 32],
                target_measure_digest: [51; 32],
                density_space_digest: [53; 32],
                reference_frame_digest: [61; 32],
                time_scale_digest: [62; 32],
                reference_epoch_digest: [63; 32],
                propagator_configuration_digest: [64; 32],
                integrator_configuration_digest: [65; 32],
                atom_ids: vec![WORLD_1, WORLD_2],
            },
            ScenarioTrajectoryContract {
                scenario_id: SCENARIO_B,
                scenario_physics_configuration_digest: [70; 32],
                target_measure_digest: [52; 32],
                density_space_digest: [54; 32],
                reference_frame_digest: [71; 32],
                time_scale_digest: [72; 32],
                reference_epoch_digest: [73; 32],
                propagator_configuration_digest: [74; 32],
                integrator_configuration_digest: [75; 32],
                atom_ids: vec![WORLD_1, WORLD_2],
            },
        ]
    }

    fn shared_world(
        scenario_id: [u8; 32],
        world_id: [u8; 32],
    ) -> SharedExogenousWorldReceipt {
        let contract = contracts()
            .into_iter()
            .find(|contract| contract.scenario_id == scenario_id)
            .unwrap();
        SharedExogenousWorldReceipt {
            scenario_id,
            atom_id: world_id,
            initial_state_digest: [120 + world_id[0]; 32],
            initial_covariance_digest: [121; 32],
            latent_uncertainty_digest: [122 + world_id[0]; 32],
            environment_realization_digest: [124; 32],
            process_noise_realization_digest: [125; 32],
            target_measure_digest: contract.target_measure_digest,
            density_space_digest: contract.density_space_digest,
            reference_frame_digest: contract.reference_frame_digest,
            time_scale_digest: contract.time_scale_digest,
            reference_epoch_digest: contract.reference_epoch_digest,
        }
    }

    fn trajectory_receipts(
        scenario_id: [u8; 32],
        world_id: [u8; 32],
        shared_digest: [u8; 32],
    ) -> Vec<PortfolioTrajectoryReceipt> {
        let contract = contracts()
            .into_iter()
            .find(|contract| contract.scenario_id == scenario_id)
            .unwrap();
        [PORTFOLIO_A, PORTFOLIO_C]
            .into_iter()
            .map(|portfolio_id| {
                let check = gate()
                    .scenario_portfolio_risk_checks
                    .into_iter()
                    .find(|check| {
                        check.scenario_id == scenario_id && check.portfolio_id == portfolio_id
                    })
                    .unwrap();
                PortfolioTrajectoryReceipt {
                    scenario_id,
                    atom_id: world_id,
                    portfolio_id,
                    shared_exogenous_receipt_digest: shared_digest,
                    maneuver_plan_digest: [130 + portfolio_id[0]; 32],
                    maneuver_execution_noise_digest: [131 + portfolio_id[0]; 32],
                    post_action_initial_state_digest: [140 + portfolio_id[0]; 32],
                    scenario_physics_configuration_digest: contract
                        .scenario_physics_configuration_digest,
                    propagator_configuration_digest: contract.propagator_configuration_digest,
                    integrator_configuration_digest: contract.integrator_configuration_digest,
                    propagated_trajectory_digest: [150 + portfolio_id[0]; 32],
                    mission_manifest_digest: check.manifest_digest,
                    post_action_event_inventory_digest: check.event_inventory_digest,
                    post_action_horizon_digest: check.post_action_horizon_digest,
                    risk_policy_digest: check.risk_policy_digest,
                    risk_envelope_digest: check.risk_envelope_digest,
                }
            })
            .collect()
    }

    fn trajectory_report() -> CounterfactualTrajectoryReceiptReport {
        let hard_gate = gate();
        let scenario_contracts = contracts();
        let receipt_campaign =
            CounterfactualTrajectoryReceiptCampaign::new(&hard_gate, scenario_contracts.clone())
                .unwrap();
        let mut rows = Vec::new();
        for contract in scenario_contracts {
            for world_id in &contract.atom_ids {
                let shared = shared_world(contract.scenario_id, *world_id);
                let shared_digest = shared.canonical_digest();
                rows.push(CounterfactualTrajectoryAtom {
                    shared_world: shared,
                    portfolio_trajectories: trajectory_receipts(
                        contract.scenario_id,
                        *world_id,
                        shared_digest,
                    ),
                });
            }
        }
        receipt_campaign.validate(rows).unwrap()
    }

    fn measurement(
        sensor: u8,
        sequence: u64,
        available_at_epoch: u32,
        value: u8,
        source: u8,
    ) -> ObservableMeasurement {
        ObservableMeasurement {
            sensor_id: [sensor; 32],
            sample_sequence: sequence,
            available_at_epoch,
            value_digest: [value; 32],
            quality_digest: [value + 1; 32],
            source_evidence_digest: [source; 32],
        }
    }

    fn trace(
        scenario_id: [u8; 32],
        world_id: [u8; 32],
        measurements: Vec<ObservableMeasurement>,
    ) -> PolicyWorldObservationTrace {
        let mut measurements = measurements;
        measurements.sort_unstable_by_key(|measurement| {
            (
                measurement.available_at_epoch,
                measurement.sensor_id,
                measurement.sample_sequence,
            )
        });
        let observation_inventory_digest = digest_observation_inventory(&measurements);
        PolicyWorldObservationTrace {
            scenario_id,
            world_id,
            observation_inventory_digest,
            measurements,
        }
    }

    fn campaign() -> NonAnticipativePolicyCampaign {
        NonAnticipativePolicyCampaign::new(
            &gate(),
            &trajectory_report(),
            2,
            TEST_DECISION_SCHEDULE,
        )
        .unwrap()
    }

    fn policy_node(
        observations: &[ObservableMeasurement],
        selected_portfolio_id: [u8; 32],
    ) -> NonAnticipativePolicyNode {
        let mut visible = observations
            .iter()
            .filter(|measurement| measurement.available_at_epoch <= 2)
            .copied()
            .collect::<Vec<_>>();
        visible.sort_unstable_by_key(|measurement| {
            (
                measurement.available_at_epoch,
                measurement.sensor_id,
                measurement.sample_sequence,
            )
        });
        NonAnticipativePolicyNode {
            observable_history_digest: digest_observable_history(
                2,
                TEST_DECISION_SCHEDULE,
                &visible,
            ),
            selected_portfolio_id,
        }
    }

    fn complete_traces(two_histories: bool) -> Vec<PolicyWorldObservationTrace> {
        let second_value = if two_histories { 60 } else { 50 };
        vec![
            trace(SCENARIO_A, WORLD_1, vec![measurement(7, 1, 1, 50, 101)]),
            trace(
                SCENARIO_A,
                WORLD_2,
                vec![measurement(7, 1, 1, second_value, 102)],
            ),
            trace(SCENARIO_B, WORLD_1, vec![measurement(7, 1, 1, 50, 103)]),
            trace(
                SCENARIO_B,
                WORLD_2,
                vec![measurement(7, 1, 1, second_value, 104)],
            ),
        ]
    }

    #[test]
    fn decision_schedule_identity_is_required_and_changes_the_frozen_campaign() {
        assert_eq!(
            NonAnticipativePolicyCampaign::new(
                &gate(),
                &trajectory_report(),
                2,
                [0; 32],
            ),
            Err(NonAnticipativePolicyError::InvalidDecisionSchedule)
        );
        let first =
            NonAnticipativePolicyCampaign::new(
                &gate(),
                &trajectory_report(),
                2,
                TEST_DECISION_SCHEDULE,
            )
                .unwrap();
        let second =
            NonAnticipativePolicyCampaign::new(&gate(), &trajectory_report(), 2, [211; 32])
                .unwrap();
        assert_ne!(first.frozen_input_digest(), second.frozen_input_digest());
    }

    #[test]
    fn same_observable_history_uses_one_action_across_hidden_scenarios_and_worlds() {
        let traces = complete_traces(false);
        let node = policy_node(&traces[0].measurements, PORTFOLIO_A);
        let plan = NonAnticipativePolicyPlan {
            policy_id: [180; 32],
            policy_code_digest: [181; 32],
            nodes: vec![node],
        };
        let report = campaign().evaluate(&plan, &traces).unwrap();
        assert_eq!(report.covered_world_count, 4);
        assert_eq!(report.unique_observable_history_count, 1);
        assert!(
            report
                .world_decisions
                .iter()
                .all(|decision| decision.selected_portfolio_id == PORTFOLIO_A)
        );
        assert!(!report.authority_granted);
    }

    #[test]
    fn different_observable_histories_may_select_different_eligible_actions() {
        let traces = complete_traces(true);
        let nodes = vec![
            policy_node(&traces[0].measurements, PORTFOLIO_A),
            policy_node(&traces[1].measurements, PORTFOLIO_C),
        ];
        let plan = NonAnticipativePolicyPlan {
            policy_id: [182; 32],
            policy_code_digest: [183; 32],
            nodes,
        };
        let report = campaign().evaluate(&plan, &traces).unwrap();
        assert_eq!(report.unique_observable_history_count, 2);
        assert_eq!(report.world_decisions[0].selected_portfolio_id, PORTFOLIO_A);
        assert_eq!(report.world_decisions[1].selected_portfolio_id, PORTFOLIO_C);
    }

    #[test]
    fn future_observations_do_not_change_the_current_policy_action() {
        let mut traces = complete_traces(false);
        traces[1].measurements.push(measurement(7, 2, 3, 90, 102));
        traces[1].measurements.sort_unstable_by_key(|measurement| {
            (
                measurement.available_at_epoch,
                measurement.sensor_id,
                measurement.sample_sequence,
            )
        });
        traces[1].observation_inventory_digest =
            digest_observation_inventory(&traces[1].measurements);
        let plan = NonAnticipativePolicyPlan {
            policy_id: [194; 32],
            policy_code_digest: [195; 32],
            nodes: vec![policy_node(&traces[0].measurements, PORTFOLIO_A)],
        };
        let report = campaign().evaluate(&plan, &traces).unwrap();
        assert!(
            report
                .world_decisions
                .iter()
                .all(|decision| decision.selected_portfolio_id == PORTFOLIO_A)
        );
        assert_eq!(
            report
                .world_decisions
                .iter()
                .filter(|decision| decision.future_measurement_count > 0)
                .count(),
            1
        );
    }

    #[test]
    fn hidden_world_ids_and_source_receipt_ids_are_not_policy_keys() {
        let traces = complete_traces(false);
        let visible_histories = traces
            .iter()
            .map(|trace| digest_observable_history(
                    2,
                    TEST_DECISION_SCHEDULE,
                    &trace
                        .measurements
                        .iter()
                        .filter(|measurement| measurement.available_at_epoch <= 2)
                        .copied()
                        .collect::<Vec<_>>(),
                ))
            .collect::<BTreeSet<_>>();
        assert_eq!(visible_histories.len(), 1);
        let inventory_ids = traces
            .iter()
            .map(|trace| trace.observation_inventory_digest)
            .collect::<BTreeSet<_>>();
        assert_eq!(inventory_ids.len(), 4);
    }

    #[test]
    fn a_second_action_for_the_same_hidden_world_independent_history_is_rejected() {
        let traces = complete_traces(false);
        let digest = policy_node(&traces[0].measurements, PORTFOLIO_A)
            .observable_history_digest;
        let plan = NonAnticipativePolicyPlan {
            policy_id: [184; 32],
            policy_code_digest: [185; 32],
            nodes: vec![
                NonAnticipativePolicyNode {
                    observable_history_digest: digest,
                    selected_portfolio_id: PORTFOLIO_A,
                },
                NonAnticipativePolicyNode {
                    observable_history_digest: digest,
                    selected_portfolio_id: PORTFOLIO_C,
                },
            ],
        };
        assert_eq!(
            campaign().evaluate(&plan, &traces),
            Err(NonAnticipativePolicyError::DuplicateOrInvalidPolicyBranch)
        );
    }

    #[test]
    fn policy_cannot_add_a_branch_for_a_hidden_world_without_an_observable_history() {
        let traces = complete_traces(false);
        let digest = policy_node(&traces[0].measurements, PORTFOLIO_A)
            .observable_history_digest;
        let plan = NonAnticipativePolicyPlan {
            policy_id: [196; 32],
            policy_code_digest: [197; 32],
            nodes: vec![
                NonAnticipativePolicyNode {
                    observable_history_digest: digest,
                    selected_portfolio_id: PORTFOLIO_A,
                },
                NonAnticipativePolicyNode {
                    observable_history_digest: [198; 32],
                    selected_portfolio_id: PORTFOLIO_C,
                },
            ],
        };
        assert_eq!(
            campaign().evaluate(&plan, &traces),
            Err(NonAnticipativePolicyError::UnusedPolicyBranch)
        );
    }

    #[test]
    fn missing_policy_branch_and_ineligible_action_fail_closed() {
        let traces = complete_traces(true);
        let missing = NonAnticipativePolicyPlan {
            policy_id: [187; 32],
            policy_code_digest: [188; 32],
            nodes: vec![policy_node(&traces[0].measurements, PORTFOLIO_A)],
        };
        assert_eq!(
            campaign().evaluate(&missing, &traces),
            Err(NonAnticipativePolicyError::MissingPolicyBranch)
        );

        let ineligible = NonAnticipativePolicyPlan {
            policy_id: [187; 32],
            policy_code_digest: [188; 32],
            nodes: vec![
                policy_node(&traces[0].measurements, PORTFOLIO_A),
                policy_node(&traces[1].measurements, PORTFOLIO_B),
            ],
        };
        assert_eq!(
            campaign().evaluate(&ineligible, &traces),
            Err(NonAnticipativePolicyError::IneligiblePolicyAction)
        );
    }

    #[test]
    fn duplicated_measurements_and_observation_inventory_substitution_are_rejected() {
        let mut traces = complete_traces(false);
        traces[0].measurements.push(measurement(7, 1, 1, 60, 201));
        assert_eq!(
            campaign().evaluate(
                &NonAnticipativePolicyPlan {
                    policy_id: [189; 32],
                    policy_code_digest: [190; 32],
                    nodes: vec![policy_node(&traces[0].measurements, PORTFOLIO_A)],
                },
                &traces,
            ),
            Err(NonAnticipativePolicyError::DuplicateMeasurementIdentity)
        );

        let mut traces = complete_traces(false);
        traces[0].observation_inventory_digest = [191; 32];
        let plan = NonAnticipativePolicyPlan {
            policy_id: [189; 32],
            policy_code_digest: [190; 32],
            nodes: vec![policy_node(&traces[0].measurements, PORTFOLIO_A)],
        };
        assert_eq!(
            campaign().evaluate(&plan, &traces),
            Err(NonAnticipativePolicyError::ObservationInventoryDigestMismatch)
        );
    }

    #[test]
    fn missing_world_trace_is_not_interpreted_as_a_safe_policy_result() {
        let mut traces = complete_traces(false);
        traces.pop();
        let plan = NonAnticipativePolicyPlan {
            policy_id: [199; 32],
            policy_code_digest: [200; 32],
            nodes: vec![policy_node(
                &complete_traces(false)[0].measurements,
                PORTFOLIO_A,
            )],
        };
        assert_eq!(
            campaign().evaluate(&plan, &traces),
            Err(NonAnticipativePolicyError::IncompleteWorldObservationMatrix)
        );
    }

    #[test]
    fn row_permutation_preserves_policy_report_digest() {
        let traces = complete_traces(true);
        let nodes = vec![
            policy_node(&traces[0].measurements, PORTFOLIO_A),
            policy_node(&traces[1].measurements, PORTFOLIO_C),
        ];
        let plan = NonAnticipativePolicyPlan {
            policy_id: [192; 32],
            policy_code_digest: [193; 32],
            nodes,
        };
        let first = campaign().evaluate(&plan, &traces).unwrap();
        let mut reversed = traces.clone();
        reversed.reverse();
        let second = campaign().evaluate(&plan, &reversed).unwrap();
        assert_eq!(first.result_digest, second.result_digest);
    }
}
