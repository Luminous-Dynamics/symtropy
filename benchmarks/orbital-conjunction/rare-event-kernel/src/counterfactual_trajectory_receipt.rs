// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Structural receipts for shared-exogenous orbital counterfactual trajectories.
//!
//! A paired comparison requires more than equal row IDs: every portfolio arm
//! must be evaluated from the same frozen pre-action state, uncertainty atom,
//! and exogenous environment realization. Each intervention may then produce
//! its own post-action state, trajectory and encounter inventory.
//!
//! This module validates receipt structure and cross-links it to the upstream
//! hard mission-risk gate. It cannot prove that an external propagator actually
//! ran, authenticate receipt issuers, or validate the physical dynamics.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::paired_objective_pareto::validate_and_digest_gate;
use crate::scenario_mission_risk_gate::ScenarioMissionRiskGateReport;

const MAX_SCENARIOS: usize = 64;
const MAX_PORTFOLIOS: usize = 32;
const MAX_TOTAL_ATOMS: usize = 65_536;
const MAX_TRAJECTORY_RECEIPTS: usize = 2_000_000;

/// Scenario-level physical and numerical model contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioTrajectoryContract {
    /// Frozen epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Caller-supplied physical-world/force-model configuration identity.
    pub scenario_physics_configuration_digest: [u8; 32],
    /// Target probability measure used by every portfolio in this scenario.
    pub target_measure_digest: [u8; 32],
    /// Density coordinates, frame and epoch semantics.
    pub density_space_digest: [u8; 32],
    /// Reference frame identity for state vectors and covariance products.
    pub reference_frame_digest: [u8; 32],
    /// Time-scale and time-conversion semantics.
    pub time_scale_digest: [u8; 32],
    /// Reference epoch identity.
    pub reference_epoch_digest: [u8; 32],
    /// Exact propagator implementation/configuration identity.
    pub propagator_configuration_digest: [u8; 32],
    /// Exact integrator implementation/configuration identity.
    pub integrator_configuration_digest: [u8; 32],
    /// Complete finite latent atom universe for this epistemic scenario.
    pub atom_ids: Vec<[u8; 32]>,
}

/// One shared exogenous world realization for one scenario/latent atom.
///
/// These values are common inputs to every portfolio's counterfactual
/// intervention. This receipt intentionally excludes all post-action values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedExogenousWorldReceipt {
    /// Epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Latent uncertainty atom identity.
    pub atom_id: [u8; 32],
    /// Exact integer target-measure mass assigned to this atom.
    pub atom_mass_ticks: u64,
    /// Pre-manoeuvre state snapshot digest.
    pub initial_state_digest: [u8; 32],
    /// Pre-manoeuvre covariance snapshot digest.
    pub initial_covariance_digest: [u8; 32],
    /// Exact latent uncertainty draw/atom digest.
    pub latent_uncertainty_digest: [u8; 32],
    /// Environment realization digest (e.g. atmosphere/space-weather inputs).
    pub environment_realization_digest: [u8; 32],
    /// Exogenous process-noise realization shared across all arms.
    pub process_noise_realization_digest: [u8; 32],
    /// Target measure identity.
    pub target_measure_digest: [u8; 32],
    /// Density-space identity.
    pub density_space_digest: [u8; 32],
    /// Reference frame identity.
    pub reference_frame_digest: [u8; 32],
    /// Time-scale identity.
    pub time_scale_digest: [u8; 32],
    /// Reference epoch identity.
    pub reference_epoch_digest: [u8; 32],
}

impl SharedExogenousWorldReceipt {
    /// Return the canonical SHA-256 digest of this shared pre-action world receipt.
    ///
    /// This is a content identity, not an issuer signature or proof of execution.
    pub fn canonical_digest(&self) -> [u8; 32] {
        digest_shared_world(self)
    }
}

/// Definite pathwise result of the complete post-action mission event inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionUnionContactDisposition {
    /// At least one declared mission event contacts on this trajectory.
    DefiniteContact,
    /// Every declared mission event is proven not to contact on this trajectory.
    DefiniteNoContact,
    /// The event inventory cannot classify this trajectory completely.
    Ambiguous,
}

/// One portfolio's post-intervention trajectory and mission-risk binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortfolioTrajectoryReceipt {
    /// Epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Latent uncertainty atom identity.
    pub atom_id: [u8; 32],
    /// Hard-risk-eligible portfolio identity.
    pub portfolio_id: [u8; 32],
    /// Digest of the complete shared-exogenous receipt for this atom.
    pub shared_exogenous_receipt_digest: [u8; 32],
    /// Manoeuvre-plan/intervention digest. A baseline arm uses an explicit no-op plan.
    pub maneuver_plan_digest: [u8; 32],
    /// Portfolio-specific manoeuvre execution/actuation uncertainty realization.
    ///
    /// This is distinct from the shared exogenous process-noise realization.
    /// It may differ across portfolios; equal values are permitted only when
    /// the physical experiment explicitly declares a common actuation error.
    /// A deterministic/no-noise arm must still bind an explicit nonzero
    /// deterministic execution-noise identity.
    pub maneuver_execution_noise_digest: [u8; 32],
    /// State digest immediately after the portfolio's intervention.
    pub post_action_initial_state_digest: [u8; 32],
    /// Physical-world/force-model identity matching the scenario contract.
    pub scenario_physics_configuration_digest: [u8; 32],
    /// Propagator implementation/configuration matching the scenario contract.
    pub propagator_configuration_digest: [u8; 32],
    /// Integrator implementation/configuration matching the scenario contract.
    pub integrator_configuration_digest: [u8; 32],
    /// Digest of the complete propagated post-action trajectory.
    pub propagated_trajectory_digest: [u8; 32],
    /// Post-action mission-support manifest digest from the hard-risk gate.
    pub mission_manifest_digest: [u8; 32],
    /// Post-action encounter inventory digest from the hard-risk gate.
    pub post_action_event_inventory_digest: [u8; 32],
    /// Approving-agent post-action screening horizon digest.
    pub post_action_horizon_digest: [u8; 32],
    /// Collision-risk policy digest.
    pub risk_policy_digest: [u8; 32],
    /// Exact risk-envelope digest for this scenario/portfolio.
    pub risk_envelope_digest: [u8; 32],
    /// Pathwise mission-wide union contact classification on this atom.
    pub mission_union_contact: MissionUnionContactDisposition,
}

/// One complete shared-world row and its portfolio-specific interventions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterfactualTrajectoryAtom {
    /// Single shared pre-action world realization.
    pub shared_world: SharedExogenousWorldReceipt,
    /// Exactly one post-action trajectory receipt for each eligible portfolio.
    pub portfolio_trajectories: Vec<PortfolioTrajectoryReceipt>,
}

/// Canonical scenario/atom identity retained in the validated report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScenarioAtomIdentity {
    /// Scenario identity.
    pub scenario_id: [u8; 32],
    /// Latent atom identity.
    pub atom_id: [u8; 32],
}

/// Complete structurally validated trajectory-coupling report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterfactualTrajectoryReceiptReport {
    /// Canonical scenario identities.
    pub scenario_ids: Vec<[u8; 32]>,
    /// Frozen scenario physics/propagator/integrator identities.
    pub scenario_contracts: Vec<ScenarioTrajectoryContract>,
    /// Exact hard-risk-eligible portfolio universe.
    pub eligible_portfolio_ids: Vec<[u8; 32]>,
    /// Portfolios excluded by the hard mission-risk gate.
    pub ineligible_portfolio_ids: Vec<[u8; 32]>,
    /// Complete canonical scenario/atom set.
    pub scenario_atom_ids: Vec<ScenarioAtomIdentity>,
    /// Complete canonical shared-world and portfolio trajectory receipts.
    pub canonical_atoms: Vec<CounterfactualTrajectoryAtom>,
    /// Digest of the exact upstream hard-risk gate.
    pub mission_risk_gate_digest: [u8; 32],
    /// Digest of the frozen trajectory campaign contract.
    pub frozen_input_digest: [u8; 32],
    /// Digest of all canonical receipts and validated identities.
    pub result_digest: [u8; 32],
    /// Always false; receipt validation grants no manoeuvre authority.
    pub authority_granted: bool,
}

/// Construction, identity and replay-receipt validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterfactualTrajectoryReceiptError {
    /// The upstream hard-risk gate is invalid or claims authority.
    InvalidMissionRiskGate,
    /// Scenario contracts are incomplete, duplicated or inconsistent with the gate.
    InvalidScenarioUniverse,
    /// A scenario contract is missing a nonzero model/input identity.
    InvalidScenarioContract,
    /// Scenario target measure or density space disagrees with the hard-risk gate.
    ScenarioMeasureBindingMismatch,
    /// A scenario has an empty, duplicated or invalid atom universe.
    InvalidScenarioAtomUniverse,
    /// The eligible/ineligible portfolio partition is invalid.
    InvalidPortfolioUniverse,
    /// Frozen campaign cardinality exceeds the bounded exact receipt workload.
    CardinalityOrWorkLimit,
    /// The supplied scenario/atom rows are incomplete or contain extras.
    IncompleteScenarioAtomUniverse,
    /// A shared exogenous receipt has a zero, duplicate or unknown identity.
    InvalidSharedWorldIdentity,
    /// Shared exogenous inputs do not match the frozen scenario contract.
    SharedWorldContractMismatch,
    /// A shared exogenous receipt digest is invalid or does not match an arm receipt.
    SharedWorldReceiptMismatch,
    /// The portfolio receipt matrix is incomplete or contains duplicates.
    IncompletePortfolioTrajectoryMatrix,
    /// A portfolio identity is unknown, ineligible or duplicated.
    InvalidPortfolioTrajectoryIdentity,
    /// Portfolio trajectory fields disagree with the scenario or risk gate.
    PortfolioTrajectoryContractMismatch,
    /// A required trajectory or intervention digest is zero.
    MissingTrajectoryEvidence,
    /// The exact finite target mass assigned to a world is zero or inconsistent.
    InvalidWorldMass,
    /// Per-atom contact classifications do not reproduce the upstream portfolio risk.
    MissionUnionMassMismatch,
    /// A pathwise mission contact outcome is ambiguous.
    AmbiguousMissionUnionClassification,
}

impl core::fmt::Display for CounterfactualTrajectoryReceiptError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidMissionRiskGate => "upstream hard mission-risk gate is invalid",
            Self::InvalidScenarioUniverse => "scenario contracts are incomplete or inconsistent with the risk gate",
            Self::InvalidScenarioContract => "scenario trajectory contract contains a zero identity",
            Self::ScenarioMeasureBindingMismatch => "scenario measure or density space differs from the hard-risk gate",
            Self::InvalidScenarioAtomUniverse => "scenario latent atom universe is invalid",
            Self::InvalidPortfolioUniverse => "hard-risk portfolio partition is invalid",
            Self::CardinalityOrWorkLimit => "trajectory receipt matrix exceeds the exact finite workload limit",
            Self::IncompleteScenarioAtomUniverse => "trajectory rows do not cover the frozen scenario/atom universe",
            Self::InvalidSharedWorldIdentity => "shared exogenous world identity is invalid or duplicated",
            Self::SharedWorldContractMismatch => "shared exogenous inputs differ from the scenario contract",
            Self::SharedWorldReceiptMismatch => "portfolio receipt does not reference the exact shared-world receipt",
            Self::IncompletePortfolioTrajectoryMatrix => "portfolio trajectory receipt matrix is incomplete",
            Self::InvalidPortfolioTrajectoryIdentity => "portfolio receipt identity is unknown or duplicated",
            Self::PortfolioTrajectoryContractMismatch => "portfolio trajectory receipt differs from the hard-risk gate or scenario contract",
            Self::MissingTrajectoryEvidence => "required intervention or trajectory evidence digest is zero",
            Self::InvalidWorldMass => "shared-world target mass is zero or inconsistent with the frozen support",
            Self::MissionUnionMassMismatch => "per-atom mission contacts do not reproduce the hard-risk gate union mass",
            Self::AmbiguousMissionUnionClassification => "ambiguous mission contact cannot be used for exact policy risk",

        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CounterfactualTrajectoryReceiptError {}

/// Frozen receipt campaign built from the hard mission-risk gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterfactualTrajectoryReceiptCampaign {
    scenario_contracts: Vec<ScenarioTrajectoryContract>,
    eligible_portfolio_ids: Vec<[u8; 32]>,
    ineligible_portfolio_ids: Vec<[u8; 32]>,
    gate_checks: BTreeMap<([u8; 32], [u8; 32]), crate::scenario_mission_risk_gate::PortfolioScenarioRiskCheck>,
    total_atom_count: usize,
    expected_trajectory_receipt_count: usize,
    mission_risk_gate_digest: [u8; 32],
    frozen_input_digest: [u8; 32],
}

impl CounterfactualTrajectoryReceiptCampaign {
    /// Freeze scenario physical models, target measures, coordinate/time semantics,
    /// atom universes and the hard-risk eligibility partition.
    pub fn new(
        gate: &ScenarioMissionRiskGateReport,
        mut scenario_contracts: Vec<ScenarioTrajectoryContract>,
    ) -> Result<Self, CounterfactualTrajectoryReceiptError> {
        let mission_risk_gate_digest =
            validate_and_digest_gate(gate).map_err(|_| {
                CounterfactualTrajectoryReceiptError::InvalidMissionRiskGate
            })?;
        if scenario_contracts.is_empty() || scenario_contracts.len() > MAX_SCENARIOS {
            return Err(CounterfactualTrajectoryReceiptError::InvalidScenarioUniverse);
        }

        scenario_contracts.sort_by_key(|contract| contract.scenario_id);
        let mut total_atom_count = 0_usize;
        for contract in &mut scenario_contracts {
            if is_zero(&contract.scenario_id)
                || is_zero(&contract.scenario_physics_configuration_digest)
                || is_zero(&contract.target_measure_digest)
                || is_zero(&contract.density_space_digest)
                || is_zero(&contract.reference_frame_digest)
                || is_zero(&contract.time_scale_digest)
                || is_zero(&contract.reference_epoch_digest)
                || is_zero(&contract.propagator_configuration_digest)
                || is_zero(&contract.integrator_configuration_digest)
            {
                return Err(CounterfactualTrajectoryReceiptError::InvalidScenarioContract);
            }
            contract.atom_ids.sort_unstable();
            if contract.atom_ids.is_empty()
                || contract.atom_ids.iter().any(is_zero)
                || contract.atom_ids.windows(2).any(|pair| pair[0] == pair[1])
            {
                return Err(CounterfactualTrajectoryReceiptError::InvalidScenarioAtomUniverse);
            }
            total_atom_count = total_atom_count
                .checked_add(contract.atom_ids.len())
                .ok_or(CounterfactualTrajectoryReceiptError::CardinalityOrWorkLimit)?;
        }
        if scenario_contracts
            .windows(2)
            .any(|pair| pair[0].scenario_id == pair[1].scenario_id)
            || total_atom_count == 0
            || total_atom_count > MAX_TOTAL_ATOMS
        {
            return Err(CounterfactualTrajectoryReceiptError::InvalidScenarioUniverse);
        }

        let gate_scenarios = gate
            .scenario_portfolio_risk_checks
            .iter()
            .map(|check| check.scenario_id)
            .collect::<BTreeSet<_>>();
        let contract_scenarios = scenario_contracts
            .iter()
            .map(|contract| contract.scenario_id)
            .collect::<BTreeSet<_>>();
        if gate_scenarios.len() != gate.scenario_count
            || gate_scenarios != contract_scenarios
        {
            return Err(CounterfactualTrajectoryReceiptError::InvalidScenarioUniverse);
        }

        let mut target_by_scenario = BTreeMap::new();
        let mut density_by_scenario = BTreeMap::new();
        let mut gate_checks = BTreeMap::new();
        for check in &gate.scenario_portfolio_risk_checks {
            if target_by_scenario
                .insert(check.scenario_id, check.target_measure_digest)
                .is_some_and(|existing| existing != check.target_measure_digest)
                || density_by_scenario
                    .insert(check.scenario_id, check.density_space_digest)
                    .is_some_and(|existing| existing != check.density_space_digest)
            {
                return Err(CounterfactualTrajectoryReceiptError::ScenarioMeasureBindingMismatch);
            }
            if gate_checks
                .insert((check.scenario_id, check.portfolio_id), *check)
                .is_some()
            {
                return Err(CounterfactualTrajectoryReceiptError::InvalidMissionRiskGate);
            }
        }
        for contract in &scenario_contracts {
            if target_by_scenario.get(&contract.scenario_id)
                != Some(&contract.target_measure_digest)
                || density_by_scenario.get(&contract.scenario_id)
                    != Some(&contract.density_space_digest)
            {
                return Err(CounterfactualTrajectoryReceiptError::ScenarioMeasureBindingMismatch);
            }
        }

        let mut eligible_portfolio_ids = gate.eligible_portfolio_ids.clone();
        let mut ineligible_portfolio_ids = gate.ineligible_portfolio_ids.clone();
        eligible_portfolio_ids.sort_unstable();
        ineligible_portfolio_ids.sort_unstable();
        if eligible_portfolio_ids.len() > MAX_PORTFOLIOS
            || eligible_portfolio_ids.iter().any(is_zero)
            || ineligible_portfolio_ids.iter().any(is_zero)
            || eligible_portfolio_ids.windows(2).any(|pair| pair[0] == pair[1])
            || ineligible_portfolio_ids.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(CounterfactualTrajectoryReceiptError::InvalidPortfolioUniverse);
        }
        let eligible_set = eligible_portfolio_ids.iter().copied().collect::<BTreeSet<_>>();
        let ineligible_set = ineligible_portfolio_ids.iter().copied().collect::<BTreeSet<_>>();
        if !eligible_set.is_disjoint(&ineligible_set) {
            return Err(CounterfactualTrajectoryReceiptError::InvalidPortfolioUniverse);
        }

        let expected_trajectory_receipt_count = total_atom_count
            .checked_mul(eligible_portfolio_ids.len())
            .ok_or(CounterfactualTrajectoryReceiptError::CardinalityOrWorkLimit)?;
        if expected_trajectory_receipt_count > MAX_TRAJECTORY_RECEIPTS {
            return Err(CounterfactualTrajectoryReceiptError::CardinalityOrWorkLimit);
        }

        let frozen_input_digest = digest_frozen_inputs(
            mission_risk_gate_digest,
            &scenario_contracts,
            &eligible_portfolio_ids,
            &ineligible_portfolio_ids,
        );
        Ok(Self {
            scenario_contracts,
            eligible_portfolio_ids,
            ineligible_portfolio_ids,
            gate_checks,
            total_atom_count,
            expected_trajectory_receipt_count,
            mission_risk_gate_digest,
            frozen_input_digest,
        })
    }

    /// Expected number of scenario/atom rows in a complete campaign.
    pub fn expected_atom_count(&self) -> usize {
        self.total_atom_count
    }

    /// Expected number of per-portfolio trajectory receipts.
    pub fn expected_trajectory_receipt_count(&self) -> usize {
        self.expected_trajectory_receipt_count
    }

    /// Digest of the frozen gate, scenario and portfolio universe.
    pub fn frozen_input_digest(&self) -> [u8; 32] {
        self.frozen_input_digest
    }

    /// Validate a complete set of shared-world and portfolio trajectory receipts.
    ///
    /// This verifies structural identity, common-exogenous references and
    /// hard-risk provenance. It does not cryptographically authenticate the
    /// source or establish that a propagator executed the declared model.
    pub fn validate(
        &self,
        mut atoms: Vec<CounterfactualTrajectoryAtom>,
    ) -> Result<CounterfactualTrajectoryReceiptReport, CounterfactualTrajectoryReceiptError> {
        if atoms.len() != self.total_atom_count {
            return Err(CounterfactualTrajectoryReceiptError::IncompleteScenarioAtomUniverse);
        }

        let contracts = self
            .scenario_contracts
            .iter()
            .map(|contract| (contract.scenario_id, contract))
            .collect::<BTreeMap<_, _>>();
        let eligible = self
            .eligible_portfolio_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let mut atom_map = BTreeMap::new();

        for atom in &mut atoms {
            atom.portfolio_trajectories
                .sort_unstable_by_key(|receipt| receipt.portfolio_id);
            let shared = atom.shared_world;
            if is_zero(&shared.scenario_id)
                || is_zero(&shared.atom_id)
                || shared.atom_mass_ticks == 0
                || is_zero(&shared.initial_state_digest)
                || is_zero(&shared.initial_covariance_digest)
                || is_zero(&shared.latent_uncertainty_digest)
                || is_zero(&shared.environment_realization_digest)
                || is_zero(&shared.process_noise_realization_digest)
                || is_zero(&shared.target_measure_digest)
                || is_zero(&shared.density_space_digest)
                || is_zero(&shared.reference_frame_digest)
                || is_zero(&shared.time_scale_digest)
                || is_zero(&shared.reference_epoch_digest)
            {
                return Err(CounterfactualTrajectoryReceiptError::InvalidSharedWorldIdentity);
            }
            let contract = contracts
                .get(&shared.scenario_id)
                .ok_or(CounterfactualTrajectoryReceiptError::InvalidSharedWorldIdentity)?;
            if contract.atom_ids.binary_search(&shared.atom_id).is_err() {
                return Err(CounterfactualTrajectoryReceiptError::InvalidSharedWorldIdentity);
            }
            if shared.target_measure_digest != contract.target_measure_digest
                || shared.density_space_digest != contract.density_space_digest
                || shared.reference_frame_digest != contract.reference_frame_digest
                || shared.time_scale_digest != contract.time_scale_digest
                || shared.reference_epoch_digest != contract.reference_epoch_digest
            {
                return Err(CounterfactualTrajectoryReceiptError::SharedWorldContractMismatch);
            }

            if atom.portfolio_trajectories.len() != self.eligible_portfolio_ids.len() {
                return Err(CounterfactualTrajectoryReceiptError::IncompletePortfolioTrajectoryMatrix);
            }
            if atom.portfolio_trajectories
                .windows(2)
                .any(|pair| pair[0].portfolio_id == pair[1].portfolio_id)
            {
                return Err(CounterfactualTrajectoryReceiptError::InvalidPortfolioTrajectoryIdentity);
            }

            let shared_digest = digest_shared_world(&shared);
            for receipt in &atom.portfolio_trajectories {
                if is_zero(&receipt.portfolio_id)
                    || !eligible.contains(&receipt.portfolio_id)
                    || receipt.scenario_id != shared.scenario_id
                    || receipt.atom_id != shared.atom_id
                {
                    return Err(CounterfactualTrajectoryReceiptError::InvalidPortfolioTrajectoryIdentity);
                }
                if is_zero(&receipt.shared_exogenous_receipt_digest)
                    || is_zero(&receipt.maneuver_plan_digest)
                    || is_zero(&receipt.maneuver_execution_noise_digest)
                    || is_zero(&receipt.post_action_initial_state_digest)
                    || is_zero(&receipt.propagated_trajectory_digest)
                    || is_zero(&receipt.mission_manifest_digest)
                    || is_zero(&receipt.post_action_event_inventory_digest)
                    || is_zero(&receipt.post_action_horizon_digest)
                    || is_zero(&receipt.risk_policy_digest)
                    || is_zero(&receipt.risk_envelope_digest)
                {
                    return Err(CounterfactualTrajectoryReceiptError::MissingTrajectoryEvidence);
                }
                if receipt.shared_exogenous_receipt_digest != shared_digest {
                    return Err(CounterfactualTrajectoryReceiptError::SharedWorldReceiptMismatch);
                }
                if receipt.mission_union_contact == MissionUnionContactDisposition::Ambiguous {
                    return Err(
                        CounterfactualTrajectoryReceiptError::AmbiguousMissionUnionClassification,
                    );
                }
                if receipt.scenario_physics_configuration_digest
                    != contract.scenario_physics_configuration_digest
                    || receipt.propagator_configuration_digest
                        != contract.propagator_configuration_digest
                    || receipt.integrator_configuration_digest
                        != contract.integrator_configuration_digest
                {
                    return Err(CounterfactualTrajectoryReceiptError::PortfolioTrajectoryContractMismatch);
                }
                let gate_check = self
                    .gate_checks
                    .get(&(receipt.scenario_id, receipt.portfolio_id))
                    .ok_or(CounterfactualTrajectoryReceiptError::InvalidPortfolioTrajectoryIdentity)?;
                if receipt.mission_manifest_digest != gate_check.manifest_digest
                    || receipt.post_action_event_inventory_digest
                        != gate_check.event_inventory_digest
                    || receipt.post_action_horizon_digest
                        != gate_check.post_action_horizon_digest
                    || receipt.risk_policy_digest != gate_check.risk_policy_digest
                    || receipt.risk_envelope_digest != gate_check.risk_envelope_digest
                {
                    return Err(CounterfactualTrajectoryReceiptError::PortfolioTrajectoryContractMismatch);
                }
            }
            if atom
                .portfolio_trajectories
                .iter()
                .map(|receipt| receipt.portfolio_id)
                .ne(self.eligible_portfolio_ids.iter().copied())
            {
                return Err(CounterfactualTrajectoryReceiptError::IncompletePortfolioTrajectoryMatrix);
            }
            if atom_map
                .insert((shared.scenario_id, shared.atom_id), atom.clone())
                .is_some()
            {
                return Err(CounterfactualTrajectoryReceiptError::InvalidSharedWorldIdentity);
            }
        }

        let mut expected_atom_ids = Vec::with_capacity(self.total_atom_count);
        for contract in &self.scenario_contracts {
            for atom_id in &contract.atom_ids {
                expected_atom_ids.push(ScenarioAtomIdentity {
                    scenario_id: contract.scenario_id,
                    atom_id: *atom_id,
                });
            }
        }
        expected_atom_ids.sort_unstable();
        let supplied_atom_ids = atom_map
            .keys()
            .map(|(scenario_id, atom_id)| ScenarioAtomIdentity {
                scenario_id: *scenario_id,
                atom_id: *atom_id,
            })
            .collect::<Vec<_>>();
        if supplied_atom_ids != expected_atom_ids {
            return Err(CounterfactualTrajectoryReceiptError::IncompleteScenarioAtomUniverse);
        }

        let canonical_atoms = atom_map.into_values().collect::<Vec<_>>();
        validate_pathwise_risk_masses(&self.scenario_contracts, &self.eligible_portfolio_ids, &self.gate_checks, &canonical_atoms)?;
        let scenario_atom_ids = expected_atom_ids;
        let mut report = CounterfactualTrajectoryReceiptReport {
            scenario_ids: self
                .scenario_contracts
                .iter()
                .map(|contract| contract.scenario_id)
                .collect(),
            scenario_contracts: self.scenario_contracts.clone(),
            eligible_portfolio_ids: self.eligible_portfolio_ids.clone(),
            ineligible_portfolio_ids: self.ineligible_portfolio_ids.clone(),
            scenario_atom_ids,
            canonical_atoms,
            mission_risk_gate_digest: self.mission_risk_gate_digest,
            frozen_input_digest: self.frozen_input_digest,
            result_digest: [0; 32],
            authority_granted: false,
        };
        report.result_digest = digest_receipt_report(&report);
        Ok(report)
    }
}

fn digest_frozen_inputs(
    gate_digest: [u8; 32],
    contracts: &[ScenarioTrajectoryContract],
    eligible: &[[u8; 32]],
    ineligible: &[[u8; 32]],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.counterfactual-trajectory.frozen.v0.1\0");
    hasher.update(gate_digest);
    hasher.update((contracts.len() as u64).to_be_bytes());
    for contract in contracts {
        hasher.update(contract.scenario_id);
        hasher.update(contract.scenario_physics_configuration_digest);
        hasher.update(contract.target_measure_digest);
        hasher.update(contract.density_space_digest);
        hasher.update(contract.reference_frame_digest);
        hasher.update(contract.time_scale_digest);
        hasher.update(contract.reference_epoch_digest);
        hasher.update(contract.propagator_configuration_digest);
        hasher.update(contract.integrator_configuration_digest);
        hash_ids(&mut hasher, &contract.atom_ids);
    }
    hash_ids(&mut hasher, eligible);
    hash_ids(&mut hasher, ineligible);
    hasher.finalize().into()
}

fn digest_shared_world(shared: &SharedExogenousWorldReceipt) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.counterfactual-trajectory.shared-world.v0.1\0");
    hasher.update(shared.scenario_id);
    hasher.update(shared.atom_id);
    hasher.update(shared.atom_mass_ticks.to_be_bytes());
    hasher.update(shared.initial_state_digest);
    hasher.update(shared.initial_covariance_digest);
    hasher.update(shared.latent_uncertainty_digest);
    hasher.update(shared.environment_realization_digest);
    hasher.update(shared.process_noise_realization_digest);
    hasher.update(shared.target_measure_digest);
    hasher.update(shared.density_space_digest);
    hasher.update(shared.reference_frame_digest);
    hasher.update(shared.time_scale_digest);
    hasher.update(shared.reference_epoch_digest);
    hasher.finalize().into()
}

pub(crate) fn digest_receipt_report(report: &CounterfactualTrajectoryReceiptReport) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.counterfactual-trajectory.report.v0.1\0");
    hasher.update(report.mission_risk_gate_digest);
    hasher.update(report.frozen_input_digest);
    hash_ids(&mut hasher, &report.scenario_ids);
    hash_ids(&mut hasher, &report.eligible_portfolio_ids);
    hash_ids(&mut hasher, &report.ineligible_portfolio_ids);
    hasher.update((report.scenario_atom_ids.len() as u64).to_be_bytes());
    for identity in &report.scenario_atom_ids {
        hasher.update(identity.scenario_id);
        hasher.update(identity.atom_id);
    }
    hasher.update((report.canonical_atoms.len() as u64).to_be_bytes());
    for atom in &report.canonical_atoms {
        hasher.update(digest_shared_world(&atom.shared_world));
        hasher.update((atom.portfolio_trajectories.len() as u64).to_be_bytes());
        for receipt in &atom.portfolio_trajectories {
            hasher.update(receipt.scenario_id);
            hasher.update(receipt.atom_id);
            hasher.update(receipt.portfolio_id);
            hasher.update(receipt.shared_exogenous_receipt_digest);
            hasher.update(receipt.maneuver_plan_digest);
            hasher.update(receipt.maneuver_execution_noise_digest);
            hasher.update(receipt.post_action_initial_state_digest);
            hasher.update(receipt.scenario_physics_configuration_digest);
            hasher.update(receipt.propagator_configuration_digest);
            hasher.update(receipt.integrator_configuration_digest);
            hasher.update(receipt.propagated_trajectory_digest);
            hasher.update(receipt.mission_manifest_digest);
            hasher.update(receipt.post_action_event_inventory_digest);
            hasher.update(receipt.post_action_horizon_digest);
            hasher.update(receipt.risk_policy_digest);
            hasher.update(receipt.risk_envelope_digest);
            hasher.update([match receipt.mission_union_contact {
                MissionUnionContactDisposition::DefiniteContact => 1,
                MissionUnionContactDisposition::DefiniteNoContact => 0,
                MissionUnionContactDisposition::Ambiguous => 2,
            }]);
        }
    }
    hasher.finalize().into()
}

fn validate_pathwise_risk_masses(
    scenario_contracts: &[ScenarioTrajectoryContract],
    eligible_portfolio_ids: &[[u8; 32]],
    gate_checks: &BTreeMap<
        ([u8; 32], [u8; 32]),
        crate::scenario_mission_risk_gate::PortfolioScenarioRiskCheck,
    >,
    atoms: &[CounterfactualTrajectoryAtom],
) -> Result<(), CounterfactualTrajectoryReceiptError> {
    for contract in scenario_contracts {
        let scenario_atoms = atoms
            .iter()
            .filter(|atom| atom.shared_world.scenario_id == contract.scenario_id)
            .collect::<Vec<_>>();
        let total_mass_ticks = scenario_atoms.iter().try_fold(0_u128, |sum, atom| {
            sum.checked_add(u128::from(atom.shared_world.atom_mass_ticks))
        }).ok_or(CounterfactualTrajectoryReceiptError::InvalidWorldMass)?;
        let total_mass_ticks = u64::try_from(total_mass_ticks)
            .map_err(|_| CounterfactualTrajectoryReceiptError::InvalidWorldMass)?;
        if scenario_atoms.len() != contract.atom_ids.len() || total_mass_ticks == 0 {
            return Err(CounterfactualTrajectoryReceiptError::InvalidWorldMass);
        }

        for portfolio_id in eligible_portfolio_ids {
            let check = gate_checks
                .get(&(contract.scenario_id, *portfolio_id))
                .ok_or(CounterfactualTrajectoryReceiptError::InvalidPortfolioTrajectoryIdentity)?;
            if check.total_mass_ticks != total_mass_ticks {
                return Err(CounterfactualTrajectoryReceiptError::InvalidWorldMass);
            }

            let mut contacting_mass_ticks = 0_u128;
            for atom in &scenario_atoms {
                let receipt = atom
                    .portfolio_trajectories
                    .iter()
                    .find(|receipt| receipt.portfolio_id == *portfolio_id)
                    .ok_or(CounterfactualTrajectoryReceiptError::IncompletePortfolioTrajectoryMatrix)?;
                if receipt.mission_union_contact == MissionUnionContactDisposition::DefiniteContact {
                    contacting_mass_ticks = contacting_mass_ticks
                        .checked_add(u128::from(atom.shared_world.atom_mass_ticks))
                        .ok_or(CounterfactualTrajectoryReceiptError::InvalidWorldMass)?;
                }
            }

            let numerator = contacting_mass_ticks
                .checked_mul(u128::from(check.mission_union_mass.denominator))
                .ok_or(CounterfactualTrajectoryReceiptError::MissionUnionMassMismatch)?;
            let expected = u128::from(check.mission_union_mass.numerator)
                .checked_mul(u128::from(total_mass_ticks))
                .ok_or(CounterfactualTrajectoryReceiptError::MissionUnionMassMismatch)?;
            if numerator != expected {
                return Err(CounterfactualTrajectoryReceiptError::MissionUnionMassMismatch);
            }
        }
    }
    Ok(())
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
    use crate::mission_risk_aggregation::ExactMass;
    use crate::scenario_mission_risk_gate::PortfolioScenarioRiskCheck;

    const SCENARIO_A: [u8; 32] = [1; 32];
    const SCENARIO_B: [u8; 32] = [2; 32];
    const PORTFOLIO_A: [u8; 32] = [10; 32];
    const PORTFOLIO_B: [u8; 32] = [20; 32];
    const PORTFOLIO_C: [u8; 32] = [30; 32];

    fn risk_check(
        scenario_id: [u8; 32],
        portfolio_id: [u8; 32],
    ) -> PortfolioScenarioRiskCheck {
        let mission_union_mass = match (scenario_id, portfolio_id) {
            (_, id) if id == PORTFOLIO_A => ExactMass {
                numerator: 3,
                denominator: 4,
            },
            (_, id) if id == PORTFOLIO_C => ExactMass {
                numerator: 1,
                denominator: 4,
            },
            (SCENARIO_A, _) => ExactMass {
                numerator: 3,
                denominator: 4,
            },
            (SCENARIO_B, _) => ExactMass {
                numerator: 1,
                denominator: 1,
            },
            _ => unreachable!(),
        };
        let within_budget = u128::from(mission_union_mass.numerator) * 5
            <= 4 * u128::from(mission_union_mass.denominator);
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
            mission_union_mass,
            mission_risk_budget: ExactMass {
                numerator: 4,
                denominator: 5,
            },
            within_declared_finite_support_budget: within_budget,
        }
    }

    fn gate() -> ScenarioMissionRiskGateReport {
        let checks = vec![
            risk_check(SCENARIO_A, PORTFOLIO_A),
            risk_check(SCENARIO_A, PORTFOLIO_B),
            risk_check(SCENARIO_A, PORTFOLIO_C),
            risk_check(SCENARIO_B, PORTFOLIO_A),
            risk_check(SCENARIO_B, PORTFOLIO_B),
            risk_check(SCENARIO_B, PORTFOLIO_C),
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
                atom_ids: vec![[1; 32], [2; 32]],
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
                atom_ids: vec![[1; 32], [2; 32]],
            },
        ]
    }

    fn shared_world(
        scenario_id: [u8; 32],
        atom_id: [u8; 32],
    ) -> SharedExogenousWorldReceipt {
        let contract = contracts()
            .into_iter()
            .find(|contract| contract.scenario_id == scenario_id)
            .unwrap();
        SharedExogenousWorldReceipt {
            scenario_id,
            atom_id,
            atom_mass_ticks: if atom_id == [1; 32] { 75 } else { 25 },
            initial_state_digest: [120; 32],
            initial_covariance_digest: [121; 32],
            latent_uncertainty_digest: [122 + atom_id[0]; 32],
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
        atom_id: [u8; 32],
        shared_digest: [u8; 32],
    ) -> Vec<PortfolioTrajectoryReceipt> {
        let contract = contracts()
            .into_iter()
            .find(|contract| contract.scenario_id == scenario_id)
            .unwrap();
        [PORTFOLIO_A, PORTFOLIO_C]
            .into_iter()
            .enumerate()
            .map(|(index, portfolio_id)| {
                let check = gate()
                    .scenario_portfolio_risk_checks
                    .into_iter()
                    .find(|check| {
                        check.scenario_id == scenario_id && check.portfolio_id == portfolio_id
                    })
                    .unwrap();
                PortfolioTrajectoryReceipt {
                    scenario_id,
                    atom_id,
                    portfolio_id,
                    shared_exogenous_receipt_digest: shared_digest,
                    maneuver_plan_digest: [130 + portfolio_id[0]; 32],
                    maneuver_execution_noise_digest: [131 + portfolio_id[0]; 32],
                    post_action_initial_state_digest: [140 + portfolio_id[0]; 32],
                    scenario_physics_configuration_digest: contract
                        .scenario_physics_configuration_digest,
                    propagator_configuration_digest: contract.propagator_configuration_digest,
                    integrator_configuration_digest: contract.integrator_configuration_digest,
                    propagated_trajectory_digest: [150 + portfolio_id[0] + index as u8; 32],
                    mission_manifest_digest: check.manifest_digest,
                    post_action_event_inventory_digest: check.event_inventory_digest,
                    post_action_horizon_digest: check.post_action_horizon_digest,
                    risk_policy_digest: check.risk_policy_digest,
                    risk_envelope_digest: check.risk_envelope_digest,
                    mission_union_contact: if portfolio_id == PORTFOLIO_A {
                        if atom_id == [1; 32] {
                            MissionUnionContactDisposition::DefiniteContact
                        } else {
                            MissionUnionContactDisposition::DefiniteNoContact
                        }
                    } else if atom_id == [2; 32] {
                        MissionUnionContactDisposition::DefiniteContact
                    } else {
                        MissionUnionContactDisposition::DefiniteNoContact
                    },
                }
            })
            .collect()
    }

    fn atom(
        scenario_id: [u8; 32],
        atom_id: [u8; 32],
    ) -> CounterfactualTrajectoryAtom {
        let shared_world = shared_world(scenario_id, atom_id);
        let shared_digest = digest_shared_world(&shared_world);
        CounterfactualTrajectoryAtom {
            shared_world,
            portfolio_trajectories: trajectory_receipts(
                scenario_id,
                atom_id,
                shared_digest,
            ),
        }
    }

    fn campaign() -> CounterfactualTrajectoryReceiptCampaign {
        CounterfactualTrajectoryReceiptCampaign::new(&gate(), contracts()).unwrap()
    }

    fn complete_atoms() -> Vec<CounterfactualTrajectoryAtom> {
        vec![
            atom(SCENARIO_A, [1; 32]),
            atom(SCENARIO_A, [2; 32]),
            atom(SCENARIO_B, [1; 32]),
            atom(SCENARIO_B, [2; 32]),
        ]
    }

    #[test]
    fn common_exogenous_world_allows_distinct_post_action_trajectories_and_inventories() {
        let report = campaign().validate(complete_atoms()).unwrap();
        assert_eq!(report.scenario_atom_ids.len(), 4);
        assert_eq!(report.eligible_portfolio_ids, vec![PORTFOLIO_A, PORTFOLIO_C]);
        assert_eq!(report.ineligible_portfolio_ids, vec![PORTFOLIO_B]);
        assert!(!report.authority_granted);

        let first = &report.canonical_atoms[0];
        assert_ne!(
            first.portfolio_trajectories[0].post_action_initial_state_digest,
            first.portfolio_trajectories[1].post_action_initial_state_digest
        );
        assert_ne!(
            first.portfolio_trajectories[0].propagated_trajectory_digest,
            first.portfolio_trajectories[1].propagated_trajectory_digest
        );
        assert_ne!(
            first.portfolio_trajectories[0].post_action_event_inventory_digest,
            first.portfolio_trajectories[1].post_action_event_inventory_digest
        );
        assert_eq!(
            first.portfolio_trajectories[0].shared_exogenous_receipt_digest,
            first.portfolio_trajectories[1].shared_exogenous_receipt_digest
        );
        assert_ne!(
            first.portfolio_trajectories[0].maneuver_execution_noise_digest,
            first.portfolio_trajectories[1].maneuver_execution_noise_digest
        );
    }

    #[test]
    fn mismatched_shared_exogenous_receipt_reference_is_rejected() {
        let mut atoms = complete_atoms();
        atoms[0].portfolio_trajectories[0].shared_exogenous_receipt_digest = [199; 32];
        assert_eq!(
            campaign().validate(atoms),
            Err(CounterfactualTrajectoryReceiptError::SharedWorldReceiptMismatch)
        );
    }

    #[test]
    fn mismatched_model_target_density_frame_or_epoch_is_rejected() {
        let mut atoms = complete_atoms();
        atoms[0].shared_world.environment_realization_digest = [198; 32];
        // The shared world is a single source of truth, so its changed value is
        // bound by the computed receipt digest. The portfolio links are stale.
        assert_eq!(
            campaign().validate(atoms),
            Err(CounterfactualTrajectoryReceiptError::SharedWorldReceiptMismatch)
        );

        let mut atoms = complete_atoms();
        atoms[0].shared_world.reference_frame_digest = [197; 32];
        assert_eq!(
            campaign().validate(atoms),
            Err(CounterfactualTrajectoryReceiptError::SharedWorldContractMismatch)
        );

        let mut atoms = complete_atoms();
        atoms[0].portfolio_trajectories[0].scenario_physics_configuration_digest = [196; 32];
        assert_eq!(
            campaign().validate(atoms),
            Err(CounterfactualTrajectoryReceiptError::PortfolioTrajectoryContractMismatch)
        );
    }

    #[test]
    fn missing_duplicate_unknown_and_ineligible_portfolio_receipts_are_rejected() {
        let mut atoms = complete_atoms();
        atoms[0].portfolio_trajectories.pop();
        assert_eq!(
            campaign().validate(atoms),
            Err(CounterfactualTrajectoryReceiptError::IncompletePortfolioTrajectoryMatrix)
        );

        let mut atoms = complete_atoms();
        atoms[0].portfolio_trajectories[1] = atoms[0].portfolio_trajectories[0];
        assert_eq!(
            campaign().validate(atoms),
            Err(CounterfactualTrajectoryReceiptError::InvalidPortfolioTrajectoryIdentity)
        );

        let mut atoms = complete_atoms();
        atoms[0].portfolio_trajectories[0].portfolio_id = PORTFOLIO_B;
        assert_eq!(
            campaign().validate(atoms),
            Err(CounterfactualTrajectoryReceiptError::InvalidPortfolioTrajectoryIdentity)
        );
    }

    #[test]
    fn portfolio_specific_mission_manifest_is_bound_without_forcing_equality() {
        let mut atoms = complete_atoms();
        let first = &atoms[0].portfolio_trajectories;
        assert_ne!(
            first[0].mission_manifest_digest,
            first[1].mission_manifest_digest
        );
        let report = campaign().validate(atoms).unwrap();
        assert_ne!(
            report.canonical_atoms[0].portfolio_trajectories[0].mission_manifest_digest,
            report.canonical_atoms[0].portfolio_trajectories[1].mission_manifest_digest
        );
    }

    #[test]
    fn scenario_and_atom_order_do_not_change_the_receipt_digest() {
        let atoms = complete_atoms();
        let first = campaign().validate(atoms.clone()).unwrap();
        let mut reversed = atoms;
        reversed.reverse();
        for atom in &mut reversed {
            atom.portfolio_trajectories.reverse();
        }
        let second = campaign().validate(reversed).unwrap();
        assert_eq!(first.frozen_input_digest, second.frozen_input_digest);
        assert_eq!(first.result_digest, second.result_digest);
    }

    #[test]
    fn action_execution_noise_is_portfolio_specific_and_bound_into_the_digest() {
        let atoms = complete_atoms();
        let first = campaign().validate(atoms.clone()).unwrap();
        let mut changed = atoms;
        changed[0].portfolio_trajectories[0].maneuver_execution_noise_digest = [194; 32];
        let changed_report = campaign().validate(changed).unwrap();
        assert_ne!(first.result_digest, changed_report.result_digest);
    }

    #[test]
    fn zero_action_execution_noise_identity_is_rejected() {
        let mut atoms = complete_atoms();
        atoms[0].portfolio_trajectories[0].maneuver_execution_noise_digest = [0; 32];
        assert_eq!(
            campaign().validate(atoms),
            Err(CounterfactualTrajectoryReceiptError::MissingTrajectoryEvidence)
        );
    }

    #[test]
    fn trajectory_replacement_changes_receipt_digest() {
        let atoms = complete_atoms();
        let first = campaign().validate(atoms.clone()).unwrap();
        let mut changed = atoms;
        changed[0].portfolio_trajectories[0].propagated_trajectory_digest = [195; 32];
        let changed_report = campaign().validate(changed).unwrap();
        assert_ne!(first.result_digest, changed_report.result_digest);
    }

    #[test]
    fn all_ineligible_portfolios_yield_empty_trajectory_receipt_rows() {
        let mut hard_gate = gate();
        for check in &mut hard_gate.scenario_portfolio_risk_checks {
            check.within_declared_finite_support_budget = false;
            check.mission_union_mass = ExactMass {
                numerator: 1,
                denominator: 1,
            };
        }
        hard_gate.eligible_portfolio_ids.clear();
        hard_gate.ineligible_portfolio_ids = vec![PORTFOLIO_A, PORTFOLIO_B, PORTFOLIO_C];
        hard_gate.ineligible_scenarios_by_portfolio = BTreeMap::from([
            (PORTFOLIO_A, vec![SCENARIO_A, SCENARIO_B]),
            (PORTFOLIO_B, vec![SCENARIO_A, SCENARIO_B]),
            (PORTFOLIO_C, vec![SCENARIO_A, SCENARIO_B]),
        ]);
        hard_gate.minimax_regret_undominated_eligible_portfolio_ids.clear();

        let process =
            CounterfactualTrajectoryReceiptCampaign::new(&hard_gate, contracts()).unwrap();
        let rows = contracts()
            .into_iter()
            .flat_map(|contract| {
                let scenario_id = contract.scenario_id;
                contract.atom_ids.into_iter().map(move |atom_id| {
                    CounterfactualTrajectoryAtom {
                        shared_world: shared_world(scenario_id, atom_id),
                        portfolio_trajectories: Vec::new(),
                    }
                })
            })
            .collect::<Vec<_>>();
        let report = process.validate(rows).unwrap();
        assert!(report.eligible_portfolio_ids.is_empty());
        assert_eq!(
            report.ineligible_portfolio_ids,
            vec![PORTFOLIO_A, PORTFOLIO_B, PORTFOLIO_C]
        );
        assert_eq!(report.canonical_atoms.len(), 4);
        assert!(
            report
                .canonical_atoms
                .iter()
                .all(|atom| atom.portfolio_trajectories.is_empty())
        );
    }
}
