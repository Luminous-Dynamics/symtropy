// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact finite-support, contrast-first Pareto oracle for orbital portfolios.
//!
//! Each scenario/latent-atom row contains the directly evaluated objective
//! outcomes for every portfolio admitted by the upstream hard mission-risk
//! gate. Pairwise differences are computed from those same joint rows.
//! Epistemic scenarios are alternatives and are never averaged.
//!
//! This is an exhaustive equally weighted finite-support oracle. It provides
//! exact population means, variances and covariance diagnostics, not statistical
//! confidence intervals or inference for continuous/adaptive orbital measures.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::mission_risk_aggregation::ExactMass;
use crate::paired_counterfactual::{
    PairedCounterfactualAtom, PairedCounterfactualError, PairingEffect, SignedRational,
    compare_paired_counterfactual_support,
};
use crate::scenario_mission_risk_gate::ScenarioMissionRiskGateReport;
use crate::space_time_partition::Rational;

const MAX_SCENARIOS: usize = 64;
const MAX_PORTFOLIOS: usize = 32;
const MAX_OBJECTIVES: usize = 32;
const MAX_TOTAL_ATOMS: usize = 65_536;
const MAX_OUTCOME_CELLS: usize = 8_000_000;
const MAX_CONTRAST_WORK: usize = 8_000_000;

/// Direction in which an orbital mission objective is optimized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairedObjectiveDirection {
    /// Lower values are preferred (e.g. propellant, downtime, residual risk).
    Minimize,
    /// Higher values are preferred (e.g. delivered mission capability).
    Maximize,
}

/// Frozen semantic identity for one objective shared across scenarios/portfolios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairedObjectiveSpec {
    /// Stable objective identity.
    pub objective_id: [u8; 32],
    /// Stable physical or operational unit identity.
    pub unit_id: [u8; 32],
    /// Stable scale/tick interpretation identity.
    pub scale_id: [u8; 32],
    /// Stable model implementation/configuration identity.
    pub model_configuration_digest: [u8; 32],
    /// Explicit optimization direction.
    pub direction: PairedObjectiveDirection,
}

/// Exact latent atom identities admitted for one epistemic scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioAtomUniverse {
    /// Frozen epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Physical-world/force-model manifest digest for this scenario.
    ///
    /// This is scenario-specific and must match the manifest identity in every
    /// upstream hard-risk check for this scenario.
    pub scenario_model_manifest_digest: [u8; 32],
    /// Complete finite set of equally weighted latent uncertainty atoms.
    pub atom_ids: Vec<[u8; 32]>,
}

/// Frozen per-portfolio, per-objective integer bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairedObjectivePortfolioBound {
    /// Portfolio identity, which must pass the hard mission-risk gate.
    pub portfolio_id: [u8; 32],
    /// Objective identity.
    pub objective_id: [u8; 32],
    /// Inclusive lower outcome bound in objective ticks.
    pub lower_ticks: i32,
    /// Inclusive upper outcome bound in objective ticks.
    pub upper_ticks: i32,
}

/// One directly evaluated objective outcome for one portfolio on one atom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairedObjectiveOutcome {
    /// Portfolio identity.
    pub portfolio_id: [u8; 32],
    /// Objective identity.
    pub objective_id: [u8; 32],
    /// Directly evaluated post-action objective value.
    pub value_ticks: i32,
}

/// One joint trajectory realization for every eligible portfolio/objective.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedObjectiveAtom {
    /// Epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Latent uncertainty atom identity, shared by every portfolio in this row.
    pub atom_id: [u8; 32],
    /// Exactly one outcome for every eligible portfolio × objective cell.
    pub outcomes: Vec<PairedObjectiveOutcome>,
}

/// Exact paired objective contrast for one scenario, portfolio pair and objective.
///
/// The sign convention is left minus right. For minimized objectives, a
/// nonpositive mean favors the left portfolio. For maximized objectives, a
/// nonnegative mean favors the left portfolio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairedObjectiveContrast {
    /// Epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Lexicographically smaller portfolio identity.
    pub left_portfolio_id: [u8; 32],
    /// Lexicographically larger portfolio identity.
    pub right_portfolio_id: [u8; 32],
    /// Objective identity.
    pub objective_id: [u8; 32],
    /// Objective direction.
    pub direction: PairedObjectiveDirection,
    /// Number of equally weighted joint trajectory atoms in this scenario.
    pub atom_count: usize,
    /// Exact population mean of left objective minus right objective.
    pub mean_left_minus_right_ticks: SignedRational,
    /// Exact population variance of the shared-atom contrast.
    pub paired_difference_population_variance: Rational,
    /// Exact variance if the same marginal outcomes were independently paired.
    pub independent_arm_difference_population_variance: Rational,
    /// Exact covariance between left and right objective outcomes on shared atoms.
    pub portfolio_covariance: SignedRational,
    /// Paired versus independent variance relation.
    pub pairing_effect: PairingEffect,
    /// Conservative contrast lower bound derived from frozen arm bounds.
    pub declared_difference_lower_ticks: i64,
    /// Conservative contrast upper bound derived from frozen arm bounds.
    pub declared_difference_upper_ticks: i64,
}

/// Scenario identity bound to its physical-world/force-model manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenarioModelBinding {
    /// Epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Physical-world/force-model manifest digest.
    pub scenario_model_manifest_digest: [u8; 32],
}

/// Exact per-scenario sign witness for one robust Pareto comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenarioObjectiveContrastWitness {
    /// Epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Exact mean of dominating portfolio minus dominated portfolio.
    pub mean_dominating_minus_dominated_ticks: SignedRational,
    /// True when this scenario satisfies the objective's no-worse relation.
    pub no_worse: bool,
    /// True when this scenario establishes strict improvement.
    pub strictly_better: bool,
}

/// Per-objective components of a contrast-first dominance proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedObjectiveDominanceWitness {
    /// Objective identity.
    pub objective_id: [u8; 32],
    /// Objective direction.
    pub direction: PairedObjectiveDirection,
    /// Exact scenario-level contrast witnesses.
    pub scenario_witnesses: Vec<ScenarioObjectiveContrastWitness>,
}

/// Proof that one portfolio robustly dominates another across every scenario/objective.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedPortfolioDominanceWitness {
    /// Portfolio proven no worse on every objective in every scenario and strictly
    /// better on at least one scenario/objective.
    pub dominating_portfolio_id: [u8; 32],
    /// Portfolio proven dominated.
    pub dominated_portfolio_id: [u8; 32],
    /// Per-objective and per-scenario exact sign witnesses.
    pub objective_witnesses: Vec<PairedObjectiveDominanceWitness>,
}

/// Full exact contrast-first Pareto oracle report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedObjectiveParetoReport {
    /// Digest of the upstream hard mission-risk gate.
    pub mission_risk_gate_digest: [u8; 32],
    /// Canonical epistemic scenario universe.
    pub scenario_ids: Vec<[u8; 32]>,
    /// Scenario-to-physical-model manifest bindings.
    pub scenario_model_bindings: Vec<ScenarioModelBinding>,
    /// Canonical eligible portfolio universe.
    pub eligible_portfolio_ids: Vec<[u8; 32]>,
    /// Portfolios excluded by the upstream hard-risk gate.
    pub ineligible_portfolio_ids: Vec<[u8; 32]>,
    /// Frozen objective specifications.
    pub objective_specs: Vec<PairedObjectiveSpec>,
    /// Canonical, complete joint trajectory/objective evidence.
    pub canonical_atoms: Vec<PairedObjectiveAtom>,
    /// Complete scenario × portfolio-pair × objective contrast family.
    pub pairwise_contrasts: Vec<PairedObjectiveContrast>,
    /// Exact conservative dominance proofs.
    pub dominance_witnesses: Vec<PairedPortfolioDominanceWitness>,
    /// Feasible portfolios not proven dominated by another feasible portfolio.
    pub pareto_frontier_portfolio_ids: Vec<[u8; 32]>,
    /// Digest of the frozen campaign universe and hard-risk gate.
    pub frozen_input_digest: [u8; 32],
    /// Digest of the complete canonical evidence and oracle result.
    pub result_digest: [u8; 32],
    /// Always false; this arithmetic oracle has no manoeuvre authority.
    pub authority_granted: bool,
}

/// Fail-closed errors for the exact paired objective Pareto oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairedObjectiveParetoError {
    /// The upstream mission-risk gate is malformed or claims authority.
    InvalidMissionRiskGate,
    /// Scenario universe is empty, too large, duplicated or inconsistent with the gate.
    InvalidScenarioUniverse,
    /// Scenario physical-model manifest does not match the upstream risk gate.
    ScenarioModelBindingMismatch,
    /// An admitted scenario has an empty or invalid latent atom universe.
    InvalidScenarioAtomUniverse,
    /// Eligible portfolio identities are invalid, duplicated or exceed the bound.
    InvalidPortfolioUniverse,
    /// Objective identities, units, scales, model IDs or directions are invalid.
    InvalidObjectiveUniverse,
    /// The portfolio × objective bound matrix is incomplete or duplicated.
    IncompleteObjectiveBoundMatrix,
    /// A frozen objective bound has reversed endpoints.
    InvalidObjectiveBound,
    /// Exact cell/contrast work cardinality overflowed or exceeded its guard.
    CardinalityOrWorkLimit,
    /// The supplied scenario/atom matrix is incomplete or contains extras.
    IncompleteScenarioAtomEvidence,
    /// Scenario or latent atom identity is zero, unknown or duplicated.
    InvalidScenarioAtomIdentity,
    /// An outcome names an unknown/zero portfolio or objective.
    InvalidObjectiveOutcomeIdentity,
    /// A portfolio/objective outcome is duplicated or omitted.
    IncompleteObjectiveOutcomeMatrix,
    /// An objective outcome falls outside its frozen per-arm bound.
    ObjectiveOutcomeOutsideBound,
    /// Exact paired moment arithmetic failed or overflowed.
    ArithmeticFailure,
}

impl core::fmt::Display for PairedObjectiveParetoError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidMissionRiskGate => "upstream mission-risk gate is invalid or grants authority",
            Self::InvalidScenarioUniverse => "scenario universe is invalid or inconsistent with the risk gate",
            Self::ScenarioModelBindingMismatch => "scenario physical-model manifest differs from the hard-risk gate",
            Self::InvalidScenarioAtomUniverse => "scenario latent-atom universe is empty or invalid",
            Self::InvalidPortfolioUniverse => "eligible portfolio universe is invalid",
            Self::InvalidObjectiveUniverse => "objective specification universe is invalid",
            Self::IncompleteObjectiveBoundMatrix => "portfolio/objective bound matrix is incomplete or duplicated",
            Self::InvalidObjectiveBound => "a frozen objective bound has reversed endpoints",
            Self::CardinalityOrWorkLimit => "paired objective oracle exceeds its exact finite-work limit",
            Self::IncompleteScenarioAtomEvidence => "scenario/latent-atom evidence matrix is incomplete or contains extras",
            Self::InvalidScenarioAtomIdentity => "scenario or latent atom identity is invalid or duplicated",
            Self::InvalidObjectiveOutcomeIdentity => "objective outcome identity is unknown or zero",
            Self::IncompleteObjectiveOutcomeMatrix => "joint atom does not contain exactly one outcome per portfolio/objective",
            Self::ObjectiveOutcomeOutsideBound => "objective outcome is outside its frozen per-portfolio bound",
            Self::ArithmeticFailure => "exact paired contrast arithmetic failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PairedObjectiveParetoError {}

/// Frozen exact finite-support campaign built from a completed hard-risk gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedObjectiveParetoCampaign {
    scenario_supports: Vec<ScenarioAtomUniverse>,
    eligible_portfolio_ids: Vec<[u8; 32]>,
    ineligible_portfolio_ids: Vec<[u8; 32]>,
    objective_specs: Vec<PairedObjectiveSpec>,
    objective_bounds: BTreeMap<([u8; 32], [u8; 32]), PairedObjectivePortfolioBound>,
    total_atom_count: usize,
    expected_outcome_count: usize,
    frozen_input_digest: [u8; 32],
    mission_risk_gate_digest: [u8; 32],
}

impl PairedObjectiveParetoCampaign {
    /// Freeze scenario supports, hard-risk eligibility, objective semantics and
    /// exact per-arm bounds before observing objective outcomes.
    ///
    /// The upstream report is rechecked for exact finite-support risk
    /// classifications and complete scenario/portfolio identity coverage.
    pub fn new(
        gate: &ScenarioMissionRiskGateReport,
        mut scenario_supports: Vec<ScenarioAtomUniverse>,
        mut objective_specs: Vec<PairedObjectiveSpec>,
        objective_bounds: Vec<PairedObjectivePortfolioBound>,
    ) -> Result<Self, PairedObjectiveParetoError> {
        let gate_digest = validate_and_digest_gate(gate)?;
        if scenario_supports.is_empty() || scenario_supports.len() > MAX_SCENARIOS {
            return Err(PairedObjectiveParetoError::InvalidScenarioUniverse);
        }
        if objective_specs.is_empty() || objective_specs.len() > MAX_OBJECTIVES {
            return Err(PairedObjectiveParetoError::InvalidObjectiveUniverse);
        }

        scenario_supports.sort_by_key(|support| support.scenario_id);
        let mut total_atom_count = 0_usize;
        for support in &mut scenario_supports {
            if is_zero(&support.scenario_id)
                || is_zero(&support.scenario_model_manifest_digest)
                || support.atom_ids.is_empty()
            {
                return Err(PairedObjectiveParetoError::InvalidScenarioAtomUniverse);
            }
            support.atom_ids.sort_unstable();
            if support.atom_ids.iter().any(is_zero)
                || support.atom_ids.windows(2).any(|pair| pair[0] == pair[1])
            {
                return Err(PairedObjectiveParetoError::InvalidScenarioAtomUniverse);
            }
            total_atom_count = total_atom_count
                .checked_add(support.atom_ids.len())
                .ok_or(PairedObjectiveParetoError::CardinalityOrWorkLimit)?;
        }
        if scenario_supports
            .windows(2)
            .any(|pair| pair[0].scenario_id == pair[1].scenario_id)
            || total_atom_count == 0
            || total_atom_count > MAX_TOTAL_ATOMS
        {
            return Err(PairedObjectiveParetoError::InvalidScenarioUniverse);
        }

        let gate_scenarios = gate
            .scenario_portfolio_risk_checks
            .iter()
            .map(|check| check.scenario_id)
            .collect::<BTreeSet<_>>();
        let supplied_scenarios = scenario_supports
            .iter()
            .map(|support| support.scenario_id)
            .collect::<BTreeSet<_>>();
        if gate_scenarios.len() != gate.scenario_count || gate_scenarios != supplied_scenarios {
            return Err(PairedObjectiveParetoError::InvalidScenarioUniverse);
        }
        let mut gate_manifest_by_scenario = BTreeMap::new();
        for check in &gate.scenario_portfolio_risk_checks {
            if gate_manifest_by_scenario
                .insert(check.scenario_id, check.manifest_digest)
                .is_some_and(|existing| existing != check.manifest_digest)
            {
                return Err(PairedObjectiveParetoError::InvalidMissionRiskGate);
            }
        }
        if scenario_supports.iter().any(|support| {
            gate_manifest_by_scenario.get(&support.scenario_id)
                != Some(&support.scenario_model_manifest_digest)
        }) {
            return Err(PairedObjectiveParetoError::ScenarioModelBindingMismatch);
        }

        let mut eligible_portfolio_ids = gate.eligible_portfolio_ids.clone();
        let mut ineligible_portfolio_ids = gate.ineligible_portfolio_ids.clone();
        eligible_portfolio_ids.sort_unstable();
        ineligible_portfolio_ids.sort_unstable();
        let eligible_set = eligible_portfolio_ids.iter().copied().collect::<BTreeSet<_>>();
        let ineligible_set = ineligible_portfolio_ids.iter().copied().collect::<BTreeSet<_>>();
        if eligible_portfolio_ids.iter().any(is_zero)
            || ineligible_portfolio_ids.iter().any(is_zero)
            || eligible_portfolio_ids.windows(2).any(|pair| pair[0] == pair[1])
            || ineligible_portfolio_ids.windows(2).any(|pair| pair[0] == pair[1])
            || !eligible_set.is_disjoint(&ineligible_set)
            || eligible_portfolio_ids.len() > MAX_PORTFOLIOS
        {
            return Err(PairedObjectiveParetoError::InvalidPortfolioUniverse);
        }

        objective_specs.sort_by_key(|spec| spec.objective_id);
        if objective_specs.iter().any(|spec| {
            is_zero(&spec.objective_id)
                || is_zero(&spec.unit_id)
                || is_zero(&spec.scale_id)
                || is_zero(&spec.model_configuration_digest)
        }) || objective_specs
            .windows(2)
            .any(|pair| pair[0].objective_id == pair[1].objective_id)
        {
            return Err(PairedObjectiveParetoError::InvalidObjectiveUniverse);
        }

        let expected_bound_count = eligible_portfolio_ids
            .len()
            .checked_mul(objective_specs.len())
            .ok_or(PairedObjectiveParetoError::CardinalityOrWorkLimit)?;
        if objective_bounds.len() != expected_bound_count {
            return Err(PairedObjectiveParetoError::IncompleteObjectiveBoundMatrix);
        }
        let mut bound_map = BTreeMap::new();
        for bound in objective_bounds {
            if is_zero(&bound.portfolio_id) || is_zero(&bound.objective_id) {
                return Err(PairedObjectiveParetoError::InvalidObjectiveOutcomeIdentity);
            }
            if bound.lower_ticks > bound.upper_ticks {
                return Err(PairedObjectiveParetoError::InvalidObjectiveBound);
            }
            if eligible_portfolio_ids.binary_search(&bound.portfolio_id).is_err()
                || objective_specs
                    .binary_search_by_key(&bound.objective_id, |spec| spec.objective_id)
                    .is_err()
            {
                return Err(PairedObjectiveParetoError::InvalidObjectiveOutcomeIdentity);
            }
            if bound_map
                .insert((bound.portfolio_id, bound.objective_id), bound)
                .is_some()
            {
                return Err(PairedObjectiveParetoError::IncompleteObjectiveBoundMatrix);
            }
        }

        let pair_count = eligible_portfolio_ids
            .len()
            .checked_mul(eligible_portfolio_ids.len().saturating_sub(1))
            .and_then(|count| count.checked_div(2))
            .ok_or(PairedObjectiveParetoError::CardinalityOrWorkLimit)?;
        let outcome_count = total_atom_count
            .checked_mul(eligible_portfolio_ids.len())
            .and_then(|count| count.checked_mul(objective_specs.len()))
            .ok_or(PairedObjectiveParetoError::CardinalityOrWorkLimit)?;
        let contrast_work = total_atom_count
            .checked_mul(pair_count)
            .and_then(|count| count.checked_mul(objective_specs.len()))
            .ok_or(PairedObjectiveParetoError::CardinalityOrWorkLimit)?;
        if outcome_count > MAX_OUTCOME_CELLS || contrast_work > MAX_CONTRAST_WORK {
            return Err(PairedObjectiveParetoError::CardinalityOrWorkLimit);
        }

        let frozen_input_digest = digest_frozen_inputs(
            gate_digest,
            &scenario_supports,
            &eligible_portfolio_ids,
            &ineligible_portfolio_ids,
            &objective_specs,
            &bound_map,
        );
        Ok(Self {
            scenario_supports,
            eligible_portfolio_ids,
            ineligible_portfolio_ids,
            objective_specs,
            objective_bounds: bound_map,
            total_atom_count,
            expected_outcome_count: outcome_count,
            frozen_input_digest,
            mission_risk_gate_digest: gate_digest,
        })
    }

    /// Number of unique scenario/latent-atom rows required by the frozen campaign.
    pub fn expected_atom_count(&self) -> usize {
        self.total_atom_count
    }

    /// Number of portfolio/objective values required across the complete campaign.
    pub fn expected_outcome_count(&self) -> usize {
        self.expected_outcome_count
    }

    /// Identity of the upstream risk gate and frozen comparison universe.
    pub fn frozen_input_digest(&self) -> [u8; 32] {
        self.frozen_input_digest
    }

    /// Validate complete joint rows, calculate exact paired contrasts and
    /// establish robust Pareto dominance without averaging epistemic scenarios.
    pub fn evaluate(
        &self,
        atoms: &[PairedObjectiveAtom],
    ) -> Result<PairedObjectiveParetoReport, PairedObjectiveParetoError> {
        if atoms.len() != self.total_atom_count {
            return Err(PairedObjectiveParetoError::IncompleteScenarioAtomEvidence);
        }

        let support_by_scenario = self
            .scenario_supports
            .iter()
            .map(|support| {
                (
                    support.scenario_id,
                    support.atom_ids.iter().copied().collect::<BTreeSet<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();

        let mut rows = BTreeMap::<
            ([u8; 32], [u8; 32]),
            BTreeMap<([u8; 32], [u8; 32]), i32>,
        >::new();
        let expected_cells_per_atom = self
            .eligible_portfolio_ids
            .len()
            .checked_mul(self.objective_specs.len())
            .ok_or(PairedObjectiveParetoError::CardinalityOrWorkLimit)?;

        for atom in atoms {
            if is_zero(&atom.scenario_id) || is_zero(&atom.atom_id) {
                return Err(PairedObjectiveParetoError::InvalidScenarioAtomIdentity);
            }
            let allowed_atoms = support_by_scenario
                .get(&atom.scenario_id)
                .ok_or(PairedObjectiveParetoError::InvalidScenarioAtomIdentity)?;
            if !allowed_atoms.contains(&atom.atom_id) {
                return Err(PairedObjectiveParetoError::InvalidScenarioAtomIdentity);
            }
            if atom.outcomes.len() != expected_cells_per_atom {
                return Err(PairedObjectiveParetoError::IncompleteObjectiveOutcomeMatrix);
            }

            let mut outcomes = BTreeMap::new();
            for outcome in &atom.outcomes {
                if is_zero(&outcome.portfolio_id) || is_zero(&outcome.objective_id) {
                    return Err(PairedObjectiveParetoError::InvalidObjectiveOutcomeIdentity);
                }
                if self
                    .eligible_portfolio_ids
                    .binary_search(&outcome.portfolio_id)
                    .is_err()
                    || self
                        .objective_specs
                        .binary_search_by_key(&outcome.objective_id, |spec| spec.objective_id)
                        .is_err()
                {
                    return Err(PairedObjectiveParetoError::InvalidObjectiveOutcomeIdentity);
                }
                let bound = self
                    .objective_bounds
                    .get(&(outcome.portfolio_id, outcome.objective_id))
                    .ok_or(PairedObjectiveParetoError::IncompleteObjectiveBoundMatrix)?;
                if outcome.value_ticks < bound.lower_ticks || outcome.value_ticks > bound.upper_ticks {
                    return Err(PairedObjectiveParetoError::ObjectiveOutcomeOutsideBound);
                }
                if outcomes
                    .insert((outcome.portfolio_id, outcome.objective_id), outcome.value_ticks)
                    .is_some()
                {
                    return Err(PairedObjectiveParetoError::IncompleteObjectiveOutcomeMatrix);
                }
            }
            if outcomes.len() != expected_cells_per_atom {
                return Err(PairedObjectiveParetoError::IncompleteObjectiveOutcomeMatrix);
            }
            if rows
                .insert((atom.scenario_id, atom.atom_id), outcomes)
                .is_some()
            {
                return Err(PairedObjectiveParetoError::InvalidScenarioAtomIdentity);
            }
        }

        if rows.len() != self.total_atom_count {
            return Err(PairedObjectiveParetoError::IncompleteScenarioAtomEvidence);
        }

        let mut canonical_atoms = Vec::with_capacity(self.total_atom_count);
        for ((scenario_id, atom_id), outcomes) in &rows {
            let mut canonical_outcomes = Vec::with_capacity(expected_cells_per_atom);
            for portfolio_id in &self.eligible_portfolio_ids {
                for spec in &self.objective_specs {
                    let value_ticks = outcomes
                        .get(&(*portfolio_id, spec.objective_id))
                        .copied()
                        .ok_or(PairedObjectiveParetoError::IncompleteObjectiveOutcomeMatrix)?;
                    canonical_outcomes.push(PairedObjectiveOutcome {
                        portfolio_id: *portfolio_id,
                        objective_id: spec.objective_id,
                        value_ticks,
                    });
                }
            }
            canonical_atoms.push(PairedObjectiveAtom {
                scenario_id: *scenario_id,
                atom_id: *atom_id,
                outcomes: canonical_outcomes,
            });
        }

        let mut pairwise_contrasts = Vec::new();
        for support in &self.scenario_supports {
            for (left_index, left_portfolio_id) in
                self.eligible_portfolio_ids.iter().enumerate()
            {
                for right_portfolio_id in self
                    .eligible_portfolio_ids
                    .iter()
                    .skip(left_index + 1)
                {
                    for spec in &self.objective_specs {
                        let mut pair_atoms = Vec::with_capacity(support.atom_ids.len());
                        for atom_id in &support.atom_ids {
                            let outcomes = rows
                                .get(&(support.scenario_id, *atom_id))
                                .ok_or(PairedObjectiveParetoError::IncompleteScenarioAtomEvidence)?;
                            let left_value = outcomes
                                .get(&(*left_portfolio_id, spec.objective_id))
                                .copied()
                                .ok_or(PairedObjectiveParetoError::IncompleteObjectiveOutcomeMatrix)?;
                            let right_value = outcomes
                                .get(&(*right_portfolio_id, spec.objective_id))
                                .copied()
                                .ok_or(PairedObjectiveParetoError::IncompleteObjectiveOutcomeMatrix)?;
                            pair_atoms.push(PairedCounterfactualAtom {
                                atom_id: *atom_id,
                                baseline_consequence_ticks: left_value,
                                portfolio_consequence_ticks: right_value,
                            });
                        }

                        let exact = compare_paired_counterfactual_support(&pair_atoms)
                            .map_err(|_: PairedCounterfactualError| {
                                PairedObjectiveParetoError::ArithmeticFailure
                            })?;
                        let left_bound = self
                            .objective_bounds
                            .get(&(*left_portfolio_id, spec.objective_id))
                            .ok_or(PairedObjectiveParetoError::IncompleteObjectiveBoundMatrix)?;
                        let right_bound = self
                            .objective_bounds
                            .get(&(*right_portfolio_id, spec.objective_id))
                            .ok_or(PairedObjectiveParetoError::IncompleteObjectiveBoundMatrix)?;
                        let declared_difference_lower_ticks =
                            i64::from(left_bound.lower_ticks) - i64::from(right_bound.upper_ticks);
                        let declared_difference_upper_ticks =
                            i64::from(left_bound.upper_ticks) - i64::from(right_bound.lower_ticks);
                        if declared_difference_lower_ticks > declared_difference_upper_ticks {
                            return Err(PairedObjectiveParetoError::InvalidObjectiveBound);
                        }

                        pairwise_contrasts.push(PairedObjectiveContrast {
                            scenario_id: support.scenario_id,
                            left_portfolio_id: *left_portfolio_id,
                            right_portfolio_id: *right_portfolio_id,
                            objective_id: spec.objective_id,
                            direction: spec.direction,
                            atom_count: support.atom_ids.len(),
                            mean_left_minus_right_ticks: exact.mean_consequence_avoided_ticks,
                            paired_difference_population_variance: exact
                                .paired_difference_population_variance,
                            independent_arm_difference_population_variance: exact
                                .independent_arm_difference_population_variance,
                            portfolio_covariance: exact.baseline_portfolio_covariance,
                            pairing_effect: exact.pairing_effect,
                            declared_difference_lower_ticks,
                            declared_difference_upper_ticks,
                        });
                    }
                }
            }
        }

        let mut dominance_witnesses = Vec::new();
        let mut dominated_portfolios = BTreeSet::new();
        for (left_index, left_id) in self.eligible_portfolio_ids.iter().enumerate() {
            for right_id in self.eligible_portfolio_ids.iter().skip(left_index + 1) {
                if let Some(witness) =
                    self.dominance_witness(*left_id, *right_id, &pairwise_contrasts)?
                {
                    dominated_portfolios.insert(witness.dominated_portfolio_id);
                    dominance_witnesses.push(witness);
                } else if let Some(witness) =
                    self.dominance_witness(*right_id, *left_id, &pairwise_contrasts)?
                {
                    dominated_portfolios.insert(witness.dominated_portfolio_id);
                    dominance_witnesses.push(witness);
                }
            }
        }

        let pareto_frontier_portfolio_ids = self
            .eligible_portfolio_ids
            .iter()
            .filter(|id| !dominated_portfolios.contains(*id))
            .copied()
            .collect::<Vec<_>>();

        let mut report = PairedObjectiveParetoReport {
            mission_risk_gate_digest: self.mission_risk_gate_digest,
            scenario_ids: self
                .scenario_supports
                .iter()
                .map(|support| support.scenario_id)
                .collect(),
            scenario_model_bindings: self
                .scenario_supports
                .iter()
                .map(|support| ScenarioModelBinding {
                    scenario_id: support.scenario_id,
                    scenario_model_manifest_digest: support.scenario_model_manifest_digest,
                })
                .collect(),
            eligible_portfolio_ids: self.eligible_portfolio_ids.clone(),
            ineligible_portfolio_ids: self.ineligible_portfolio_ids.clone(),
            objective_specs: self.objective_specs.clone(),
            canonical_atoms,
            pairwise_contrasts,
            dominance_witnesses,
            pareto_frontier_portfolio_ids,
            frozen_input_digest: self.frozen_input_digest,
            result_digest: [0; 32],
            authority_granted: false,
        };
        report.result_digest = digest_report(&report);
        Ok(report)
    }

    fn dominance_witness(
        &self,
        dominating_id: [u8; 32],
        dominated_id: [u8; 32],
        contrasts: &[PairedObjectiveContrast],
    ) -> Result<Option<PairedPortfolioDominanceWitness>, PairedObjectiveParetoError> {
        let mut all_no_worse = true;
        let mut any_strict = false;
        let mut objective_witnesses = Vec::with_capacity(self.objective_specs.len());

        for spec in &self.objective_specs {
            let mut scenario_witnesses = Vec::with_capacity(self.scenario_supports.len());
            for support in &self.scenario_supports {
                let contrast = contrasts
                    .iter()
                    .find(|contrast| {
                        contrast.scenario_id == support.scenario_id
                            && contrast.objective_id == spec.objective_id
                            && ((contrast.left_portfolio_id == dominating_id
                                && contrast.right_portfolio_id == dominated_id)
                                || (contrast.left_portfolio_id == dominated_id
                                    && contrast.right_portfolio_id == dominating_id))
                    })
                    .ok_or(PairedObjectiveParetoError::IncompleteScenarioAtomEvidence)?;

                let mean = if contrast.left_portfolio_id == dominating_id {
                    contrast.mean_left_minus_right_ticks
                } else {
                    SignedRational {
                        numerator: contrast
                            .mean_left_minus_right_ticks
                            .numerator
                            .checked_neg()
                            .ok_or(PairedObjectiveParetoError::ArithmeticFailure)?,
                        denominator: contrast.mean_left_minus_right_ticks.denominator,
                    }
                };
                let (no_worse, strictly_better) = match spec.direction {
                    PairedObjectiveDirection::Minimize => {
                        (mean.numerator <= 0, mean.numerator < 0)
                    }
                    PairedObjectiveDirection::Maximize => {
                        (mean.numerator >= 0, mean.numerator > 0)
                    }
                };
                all_no_worse &= no_worse;
                any_strict |= strictly_better;
                scenario_witnesses.push(ScenarioObjectiveContrastWitness {
                    scenario_id: support.scenario_id,
                    mean_dominating_minus_dominated_ticks: mean,
                    no_worse,
                    strictly_better,
                });
            }
            objective_witnesses.push(PairedObjectiveDominanceWitness {
                objective_id: spec.objective_id,
                direction: spec.direction,
                scenario_witnesses,
            });
        }

        if all_no_worse && any_strict {
            Ok(Some(PairedPortfolioDominanceWitness {
                dominating_portfolio_id: dominating_id,
                dominated_portfolio_id: dominated_id,
                objective_witnesses,
            }))
        } else {
            Ok(None)
        }
    }
}

fn validate_and_digest_gate(
    gate: &ScenarioMissionRiskGateReport,
) -> Result<[u8; 32], PairedObjectiveParetoError> {
    if gate.authority_granted
        || gate.scenario_count == 0
        || gate.portfolio_count == 0
        || gate.scenario_portfolio_risk_checks.len()
            != gate
                .scenario_count
                .checked_mul(gate.portfolio_count)
                .ok_or(PairedObjectiveParetoError::InvalidMissionRiskGate)?
        || !gate.total_family_alpha.is_finite()
        || gate.total_family_alpha <= 0.0
        || gate.total_family_alpha >= 1.0
    {
        return Err(PairedObjectiveParetoError::InvalidMissionRiskGate);
    }

    let mut scenario_ids = BTreeSet::new();
    let mut portfolio_ids = BTreeSet::new();
    let mut scenario_manifests = BTreeMap::new();
    let mut checks = BTreeMap::new();
    let mut derived_ineligible = BTreeMap::<[u8; 32], Vec<[u8; 32]>>::new();
    for check in &gate.scenario_portfolio_risk_checks {
        if is_zero(&check.scenario_id)
            || is_zero(&check.portfolio_id)
            || is_zero(&check.manifest_digest)
            || is_zero(&check.target_measure_digest)
            || is_zero(&check.density_space_digest)
            || is_zero(&check.event_inventory_digest)
            || is_zero(&check.risk_envelope_digest)
            || is_zero(&check.post_action_horizon_digest)
            || is_zero(&check.risk_policy_digest)
            || check.total_mass_ticks == 0
            || !valid_mass(check.mission_union_mass)
            || !valid_mass(check.mission_risk_budget)
        {
            return Err(PairedObjectiveParetoError::InvalidMissionRiskGate);
        }
        let within = mass_leq(check.mission_union_mass, check.mission_risk_budget);
        if within != check.within_declared_finite_support_budget {
            return Err(PairedObjectiveParetoError::InvalidMissionRiskGate);
        }
        if !within {
            derived_ineligible
                .entry(check.portfolio_id)
                .or_default()
                .push(check.scenario_id);
        }
        if scenario_manifests
            .insert(check.scenario_id, check.manifest_digest)
            .is_some_and(|existing| existing != check.manifest_digest)
        {
            return Err(PairedObjectiveParetoError::InvalidMissionRiskGate);
        }
        scenario_ids.insert(check.scenario_id);
        portfolio_ids.insert(check.portfolio_id);
        if checks
            .insert((check.scenario_id, check.portfolio_id), *check)
            .is_some()
        {
            return Err(PairedObjectiveParetoError::InvalidMissionRiskGate);
        }
    }
    if scenario_ids.len() != gate.scenario_count || portfolio_ids.len() != gate.portfolio_count {
        return Err(PairedObjectiveParetoError::InvalidMissionRiskGate);
    }

    let mut expected_eligible = portfolio_ids
        .iter()
        .filter(|id| !derived_ineligible.contains_key(*id))
        .copied()
        .collect::<Vec<_>>();
    let mut expected_ineligible = derived_ineligible.keys().copied().collect::<Vec<_>>();
    expected_eligible.sort_unstable();
    expected_ineligible.sort_unstable();

    let mut supplied_eligible = gate.eligible_portfolio_ids.clone();
    let mut supplied_ineligible = gate.ineligible_portfolio_ids.clone();
    supplied_eligible.sort_unstable();
    supplied_ineligible.sort_unstable();
    let supplied_map = normalize_ineligible_map(&gate.ineligible_scenarios_by_portfolio);
    let expected_map = normalize_ineligible_map(&derived_ineligible);

    if supplied_eligible != expected_eligible
        || supplied_ineligible != expected_ineligible
        || supplied_map != expected_map
    {
        return Err(PairedObjectiveParetoError::InvalidMissionRiskGate);
    }

    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.paired-objective-pareto.risk-gate.v0.1\0");
    hasher.update(gate.total_family_alpha.to_bits().to_be_bytes());
    hasher.update((gate.scenario_count as u64).to_be_bytes());
    hasher.update((gate.portfolio_count as u64).to_be_bytes());
    hasher.update(gate.consequence_unit_id);
    hasher.update((checks.len() as u64).to_be_bytes());
    for ((scenario_id, portfolio_id), check) in checks {
        hasher.update(scenario_id);
        hasher.update(portfolio_id);
        hasher.update(check.manifest_digest);
        hasher.update(check.target_measure_digest);
        hasher.update(check.density_space_digest);
        hasher.update(check.event_inventory_digest);
        hasher.update(check.risk_envelope_digest);
        hasher.update(check.post_action_horizon_digest);
        hasher.update(check.risk_policy_digest);
        hash_mass(&mut hasher, check.mission_union_mass);
        hash_mass(&mut hasher, check.mission_risk_budget);
        hasher.update([u8::from(check.within_declared_finite_support_budget)]);
    }
    Ok(hasher.finalize().into())
}

fn normalize_ineligible_map(
    map: &BTreeMap<[u8; 32], Vec<[u8; 32]>>,
) -> BTreeMap<[u8; 32], Vec<[u8; 32]>> {
    map.iter()
        .map(|(portfolio_id, scenarios)| {
            let mut scenarios = scenarios.clone();
            scenarios.sort_unstable();
            (*portfolio_id, scenarios)
        })
        .collect()
}

fn digest_frozen_inputs(
    gate_digest: [u8; 32],
    scenario_supports: &[ScenarioAtomUniverse],
    eligible_portfolios: &[[u8; 32]],
    ineligible_portfolios: &[[u8; 32]],
    objectives: &[PairedObjectiveSpec],
    bounds: &BTreeMap<([u8; 32], [u8; 32]), PairedObjectivePortfolioBound>,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.paired-objective-pareto.frozen.v0.1\0");
    hasher.update(gate_digest);
    hasher.update((scenario_supports.len() as u64).to_be_bytes());
    for support in scenario_supports {
        hasher.update(support.scenario_id);
        hasher.update(support.scenario_model_manifest_digest);
        hash_ids(&mut hasher, &support.atom_ids);
    }
    hash_ids(&mut hasher, eligible_portfolios);
    hash_ids(&mut hasher, ineligible_portfolios);
    hasher.update((objectives.len() as u64).to_be_bytes());
    for spec in objectives {
        hasher.update(spec.objective_id);
        hasher.update(spec.unit_id);
        hasher.update(spec.scale_id);
        hasher.update(spec.model_configuration_digest);
        hasher.update([direction_byte(spec.direction)]);
    }
    hasher.update((bounds.len() as u64).to_be_bytes());
    for ((portfolio_id, objective_id), bound) in bounds {
        hasher.update(portfolio_id);
        hasher.update(objective_id);
        hasher.update(bound.lower_ticks.to_be_bytes());
        hasher.update(bound.upper_ticks.to_be_bytes());
    }
    hasher.finalize().into()
}

fn digest_report(report: &PairedObjectiveParetoReport) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.paired-objective-pareto.report.v0.1\0");
    hasher.update(report.mission_risk_gate_digest);
    hasher.update(report.frozen_input_digest);
    hash_ids(&mut hasher, &report.scenario_ids);
    hasher.update((report.scenario_model_bindings.len() as u64).to_be_bytes());
    for binding in &report.scenario_model_bindings {
        hasher.update(binding.scenario_id);
        hasher.update(binding.scenario_model_manifest_digest);
    }
    hash_ids(&mut hasher, &report.eligible_portfolio_ids);
    hash_ids(&mut hasher, &report.ineligible_portfolio_ids);
    hasher.update((report.objective_specs.len() as u64).to_be_bytes());
    for spec in &report.objective_specs {
        hasher.update(spec.objective_id);
        hasher.update(spec.unit_id);
        hasher.update(spec.scale_id);
        hasher.update(spec.model_configuration_digest);
        hasher.update([direction_byte(spec.direction)]);
    }
    hasher.update((report.canonical_atoms.len() as u64).to_be_bytes());
    for atom in &report.canonical_atoms {
        hasher.update(atom.scenario_id);
        hasher.update(atom.atom_id);
        hasher.update((atom.outcomes.len() as u64).to_be_bytes());
        for outcome in &atom.outcomes {
            hasher.update(outcome.portfolio_id);
            hasher.update(outcome.objective_id);
            hasher.update(outcome.value_ticks.to_be_bytes());
        }
    }
    hasher.update((report.pairwise_contrasts.len() as u64).to_be_bytes());
    for contrast in &report.pairwise_contrasts {
        hasher.update(contrast.scenario_id);
        hasher.update(contrast.left_portfolio_id);
        hasher.update(contrast.right_portfolio_id);
        hasher.update(contrast.objective_id);
        hasher.update([direction_byte(contrast.direction)]);
        hasher.update((contrast.atom_count as u64).to_be_bytes());
        hash_signed(&mut hasher, contrast.mean_left_minus_right_ticks);
        hash_rational(&mut hasher, contrast.paired_difference_population_variance);
        hash_rational(
            &mut hasher,
            contrast.independent_arm_difference_population_variance,
        );
        hash_signed(&mut hasher, contrast.portfolio_covariance);
        hasher.update([pairing_effect_byte(contrast.pairing_effect)]);
        hasher.update(contrast.declared_difference_lower_ticks.to_be_bytes());
        hasher.update(contrast.declared_difference_upper_ticks.to_be_bytes());
    }
    hasher.update((report.dominance_witnesses.len() as u64).to_be_bytes());
    for witness in &report.dominance_witnesses {
        hasher.update(witness.dominating_portfolio_id);
        hasher.update(witness.dominated_portfolio_id);
        for objective in &witness.objective_witnesses {
            hasher.update(objective.objective_id);
            hasher.update([direction_byte(objective.direction)]);
            for scenario in &objective.scenario_witnesses {
                hasher.update(scenario.scenario_id);
                hash_signed(
                    &mut hasher,
                    scenario.mean_dominating_minus_dominated_ticks,
                );
                hasher.update([u8::from(scenario.no_worse)]);
                hasher.update([u8::from(scenario.strictly_better)]);
            }
        }
    }
    hash_ids(&mut hasher, &report.pareto_frontier_portfolio_ids);
    hasher.finalize().into()
}

fn hash_ids(hasher: &mut Sha256, ids: &[[u8; 32]]) {
    hasher.update((ids.len() as u64).to_be_bytes());
    for id in ids {
        hasher.update(id);
    }
}

fn hash_mass(hasher: &mut Sha256, mass: ExactMass) {
    hasher.update(mass.numerator.to_be_bytes());
    hasher.update(mass.denominator.to_be_bytes());
}

fn hash_rational(hasher: &mut Sha256, value: Rational) {
    hasher.update(value.numerator().to_be_bytes());
    hasher.update(value.denominator().to_be_bytes());
}

fn hash_signed(hasher: &mut Sha256, value: SignedRational) {
    hasher.update(value.numerator.to_be_bytes());
    hasher.update(value.denominator.to_be_bytes());
}

fn valid_mass(mass: ExactMass) -> bool {
    mass.denominator > 0 && mass.numerator <= mass.denominator
}

fn mass_leq(left: ExactMass, right: ExactMass) -> bool {
    u128::from(left.numerator) * u128::from(right.denominator)
        <= u128::from(right.numerator) * u128::from(left.denominator)
}

fn is_zero(id: &[u8; 32]) -> bool {
    id.iter().all(|byte| *byte == 0)
}

fn direction_byte(direction: PairedObjectiveDirection) -> u8 {
    match direction {
        PairedObjectiveDirection::Minimize => 0,
        PairedObjectiveDirection::Maximize => 1,
    }
}

fn pairing_effect_byte(effect: PairingEffect) -> u8 {
    match effect {
        PairingEffect::VarianceReduction => 0,
        PairingEffect::NoVarianceChange => 1,
        PairingEffect::VarianceInflation => 2,
        PairingEffect::ZeroIndependentVariance => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENARIO_A: [u8; 32] = [1; 32];
    const SCENARIO_B: [u8; 32] = [2; 32];
    const PORTFOLIO_A: [u8; 32] = [10; 32];
    const PORTFOLIO_B: [u8; 32] = [20; 32];
    const PORTFOLIO_C: [u8; 32] = [30; 32];
    const OBJECTIVE_FUEL: [u8; 32] = [40; 32];
    const OBJECTIVE_SERVICE: [u8; 32] = [41; 32];

    fn risk_check(
        scenario_id: [u8; 32],
        portfolio_id: [u8; 32],
        within_budget: bool,
    ) -> crate::scenario_mission_risk_gate::PortfolioScenarioRiskCheck {
        crate::scenario_mission_risk_gate::PortfolioScenarioRiskCheck {
            scenario_id,
            portfolio_id,
            manifest_digest: if scenario_id == SCENARIO_A { [50; 32] } else { [70; 32] },
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
            event_inventory_digest: [55; 32],
            risk_envelope_digest: [56 + portfolio_id[0]; 32],
            total_mass_ticks: 100,
            post_action_horizon_digest: [57; 32],
            risk_policy_digest: [58; 32],
            mission_union_mass: if within_budget {
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
            within_declared_finite_support_budget: within_budget,
        }
    }

    fn gate_report() -> ScenarioMissionRiskGateReport {
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
            consequence_unit_id: [59; 32],
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

    fn specs() -> Vec<PairedObjectiveSpec> {
        vec![
            PairedObjectiveSpec {
                objective_id: OBJECTIVE_FUEL,
                unit_id: [60; 32],
                scale_id: [61; 32],
                model_configuration_digest: [62; 32],
                direction: PairedObjectiveDirection::Minimize,
            },
            PairedObjectiveSpec {
                objective_id: OBJECTIVE_SERVICE,
                unit_id: [63; 32],
                scale_id: [64; 32],
                model_configuration_digest: [65; 32],
                direction: PairedObjectiveDirection::Maximize,
            },
        ]
    }

    fn campaign(
        supports: Vec<ScenarioAtomUniverse>,
        objective_specs: Vec<PairedObjectiveSpec>,
        bounds: Vec<PairedObjectivePortfolioBound>,
    ) -> PairedObjectiveParetoCampaign {
        PairedObjectiveParetoCampaign::new(&gate_report(), supports, objective_specs, bounds)
            .unwrap()
    }

    fn standard_supports() -> Vec<ScenarioAtomUniverse> {
        vec![
            ScenarioAtomUniverse {
                scenario_id: SCENARIO_A,
                scenario_model_manifest_digest: [50; 32],
                atom_ids: vec![[101; 32], [102; 32], [103; 32], [104; 32]],
            },
            ScenarioAtomUniverse {
                scenario_id: SCENARIO_B,
                scenario_model_manifest_digest: [70; 32],
                atom_ids: vec![[101; 32], [102; 32], [103; 32], [104; 32]],
            },
        ]
    }

    fn bound(
        portfolio_id: [u8; 32],
        objective_id: [u8; 32],
        lower_ticks: i32,
        upper_ticks: i32,
    ) -> PairedObjectivePortfolioBound {
        PairedObjectivePortfolioBound {
            portfolio_id,
            objective_id,
            lower_ticks,
            upper_ticks,
        }
    }

    fn standard_bounds() -> Vec<PairedObjectivePortfolioBound> {
        vec![
            bound(PORTFOLIO_A, OBJECTIVE_FUEL, 0, 200),
            bound(PORTFOLIO_C, OBJECTIVE_FUEL, 0, 200),
            bound(PORTFOLIO_A, OBJECTIVE_SERVICE, 0, 200),
            bound(PORTFOLIO_C, OBJECTIVE_SERVICE, 0, 200),
        ]
    }

    fn atom(
        scenario_id: [u8; 32],
        atom_id: [u8; 32],
        outcomes: &[(u8, i32, i32)],
    ) -> PairedObjectiveAtom {
        let mut values = Vec::new();
        for (portfolio, fuel, service) in outcomes {
            let portfolio_id = match *portfolio {
                10 => PORTFOLIO_A,
                20 => PORTFOLIO_B,
                30 => PORTFOLIO_C,
                _ => [*portfolio; 32],
            };
            values.push(PairedObjectiveOutcome {
                portfolio_id,
                objective_id: OBJECTIVE_FUEL,
                value_ticks: *fuel,
            });
            values.push(PairedObjectiveOutcome {
                portfolio_id,
                objective_id: OBJECTIVE_SERVICE,
                value_ticks: *service,
            });
        }
        PairedObjectiveAtom {
            scenario_id,
            atom_id,
            outcomes: values,
        }
    }

    fn contrast_atoms_shifted() -> Vec<PairedObjectiveAtom> {
        vec![
            atom(SCENARIO_A, [101; 32], &[(10, 1, 10), (30, 2, 12)]),
            atom(SCENARIO_A, [102; 32], &[(10, 2, 11), (30, 3, 13)]),
            atom(SCENARIO_A, [103; 32], &[(10, 3, 12), (30, 4, 14)]),
            atom(SCENARIO_A, [104; 32], &[(10, 4, 13), (30, 5, 15)]),
            atom(SCENARIO_B, [101; 32], &[(10, 101, 10), (30, 103, 12)]),
            atom(SCENARIO_B, [102; 32], &[(10, 102, 11), (30, 104, 13)]),
            atom(SCENARIO_B, [103; 32], &[(10, 103, 12), (30, 105, 14)]),
            atom(SCENARIO_B, [104; 32], &[(10, 104, 13), (30, 106, 15)]),
        ]
    }

    fn fuel_only_atoms() -> Vec<PairedObjectiveAtom> {
        contrast_atoms_shifted()
            .into_iter()
            .map(|mut atom| {
                atom.outcomes
                    .retain(|outcome| outcome.objective_id == OBJECTIVE_FUEL);
                atom
            })
            .collect()
    }

    #[test]
    fn paired_contrasts_prove_dominance_when_marginal_worst_case_envelopes_do_not() {
        let process = campaign(
            standard_supports(),
            vec![specs()[0]],
            vec![
                bound(PORTFOLIO_A, OBJECTIVE_FUEL, 0, 200),
                bound(PORTFOLIO_C, OBJECTIVE_FUEL, 0, 200),
            ],
        );
        let report = process.evaluate(&fuel_only_atoms()).unwrap();

        assert_eq!(report.pairwise_contrasts.len(), 2);
        assert_eq!(
            report.pairwise_contrasts[0].mean_left_minus_right_ticks,
            SignedRational {
                numerator: -1,
                denominator: 1,
            }
        );
        assert_eq!(
            report.pairwise_contrasts[1].mean_left_minus_right_ticks,
            SignedRational {
                numerator: -2,
                denominator: 1,
            }
        );

        // Marginal worst-case envelopes overlap: max(A) = 104 while min(C) = 2.
        assert!(104 > 2);
        assert_eq!(report.dominance_witnesses.len(), 1);
        assert_eq!(report.dominance_witnesses[0].dominating_portfolio_id, PORTFOLIO_A);
        assert_eq!(report.dominance_witnesses[0].dominated_portfolio_id, PORTFOLIO_C);
        assert_eq!(report.pareto_frontier_portfolio_ids, vec![PORTFOLIO_A]);
        assert!(!report.authority_granted);
    }

    #[test]
    fn a_winner_that_changes_across_scenarios_is_not_robustly_dominant() {
        let process = campaign(
            standard_supports(),
            vec![specs()[0]],
            vec![
                bound(PORTFOLIO_A, OBJECTIVE_FUEL, 0, 200),
                bound(PORTFOLIO_C, OBJECTIVE_FUEL, 0, 200),
            ],
        );
        let mut rows = fuel_only_atoms();
        for row in rows.iter_mut().filter(|row| row.scenario_id == SCENARIO_B) {
            for outcome in &mut row.outcomes {
                if outcome.portfolio_id == PORTFOLIO_A {
                    outcome.value_ticks += 10;
                }
            }
        }
        let report = process.evaluate(&rows).unwrap();
        assert!(report.dominance_witnesses.is_empty());
        assert_eq!(
            report.pareto_frontier_portfolio_ids,
            vec![PORTFOLIO_A, PORTFOLIO_C]
        );
    }

    #[test]
    fn multiobjective_tradeoffs_remain_on_the_frontier() {
        let process = campaign(standard_supports(), specs(), standard_bounds());
        let report = process.evaluate(&contrast_atoms_shifted()).unwrap();
        assert!(report.dominance_witnesses.is_empty());
        assert_eq!(
            report.pareto_frontier_portfolio_ids,
            vec![PORTFOLIO_A, PORTFOLIO_C]
        );
    }

    #[test]
    fn paired_variance_diagnostics_retain_covariance_instead_of_assuming_crn_helps() {
        let process = campaign(
            standard_supports(),
            vec![specs()[0]],
            vec![
                bound(PORTFOLIO_A, OBJECTIVE_FUEL, 0, 200),
                bound(PORTFOLIO_C, OBJECTIVE_FUEL, 0, 200),
            ],
        );
        let mut rows = fuel_only_atoms();
        for (index, row) in rows.iter_mut().enumerate() {
            if index < 4 {
                for outcome in &mut row.outcomes {
                    if outcome.portfolio_id == PORTFOLIO_C
                        && outcome.objective_id == OBJECTIVE_FUEL
                    {
                        outcome.value_ticks = [4, 3, 2, 1][index];
                    }
                }
            }
        }
        let report = process.evaluate(&rows).unwrap();
        assert_eq!(report.pairwise_contrasts.len(), 2);
        assert_eq!(
            report.pairwise_contrasts[0].pairing_effect,
            PairingEffect::VarianceInflation
        );
        assert_eq!(
            report.pairwise_contrasts[1].pairing_effect,
            PairingEffect::VarianceReduction
        );
    }

    #[test]
    fn missing_duplicate_or_substituted_scenario_rows_are_rejected() {
        let process = campaign(
            standard_supports(),
            vec![specs()[0]],
            vec![
                bound(PORTFOLIO_A, OBJECTIVE_FUEL, 0, 200),
                bound(PORTFOLIO_C, OBJECTIVE_FUEL, 0, 200),
            ],
        );
        let mut rows = fuel_only_atoms();
        rows.pop();
        assert_eq!(
            process.evaluate(&rows),
            Err(PairedObjectiveParetoError::IncompleteScenarioAtomEvidence)
        );

        let mut rows = fuel_only_atoms();
        rows[1] = rows[0].clone();
        assert_eq!(
            process.evaluate(&rows),
            Err(PairedObjectiveParetoError::InvalidScenarioAtomIdentity)
        );

        let mut rows = contrast_atoms_shifted();
        rows[0].scenario_id = [99; 32];
        assert_eq!(
            process.evaluate(&rows),
            Err(PairedObjectiveParetoError::InvalidScenarioAtomIdentity)
        );
    }

    #[test]
    fn missing_duplicate_and_substituted_objective_cells_are_rejected() {
        let process = campaign(standard_supports(), specs(), standard_bounds());
        let mut rows = contrast_atoms_shifted();
        rows[0].outcomes.pop();
        assert_eq!(
            process.evaluate(&rows),
            Err(PairedObjectiveParetoError::IncompleteObjectiveOutcomeMatrix)
        );

        let mut rows = contrast_atoms_shifted();
        rows[0].outcomes.push(rows[0].outcomes[0]);
        assert_eq!(
            process.evaluate(&rows),
            Err(PairedObjectiveParetoError::IncompleteObjectiveOutcomeMatrix)
        );

        let mut rows = contrast_atoms_shifted();
        rows[0].outcomes[0].portfolio_id = PORTFOLIO_B;
        assert_eq!(
            process.evaluate(&rows),
            Err(PairedObjectiveParetoError::InvalidObjectiveOutcomeIdentity)
        );
    }

    #[test]
    fn objective_values_outside_frozen_arm_bounds_are_rejected() {
        let process = campaign(
            standard_supports(),
            vec![specs()[0]],
            vec![
                bound(PORTFOLIO_A, OBJECTIVE_FUEL, 0, 5),
                bound(PORTFOLIO_C, OBJECTIVE_FUEL, 0, 6),
            ],
        );
        assert_eq!(
            process.evaluate(&fuel_only_atoms()),
            Err(PairedObjectiveParetoError::ObjectiveOutcomeOutsideBound)
        );
    }

    #[test]
    fn objective_bound_matrix_must_be_complete_and_ordered() {
        let mut bounds = standard_bounds();
        bounds.pop();
        assert_eq!(
            PairedObjectiveParetoCampaign::new(
                &gate_report(),
                standard_supports(),
                specs(),
                bounds,
            ),
            Err(PairedObjectiveParetoError::IncompleteObjectiveBoundMatrix)
        );

        let mut bounds = standard_bounds();
        bounds[0].lower_ticks = 10;
        bounds[0].upper_ticks = 0;
        assert_eq!(
            PairedObjectiveParetoCampaign::new(
                &gate_report(),
                standard_supports(),
                specs(),
                bounds,
            ),
            Err(PairedObjectiveParetoError::InvalidObjectiveBound)
        );
    }

    #[test]
    fn portfolio_failing_one_scenario_is_never_reintroduced() {
        let process = campaign(
            standard_supports(),
            vec![specs()[0]],
            vec![
                bound(PORTFOLIO_A, OBJECTIVE_FUEL, 0, 200),
                bound(PORTFOLIO_C, OBJECTIVE_FUEL, 0, 200),
            ],
        );
        let report = process.evaluate(&fuel_only_atoms()).unwrap();
        assert_eq!(report.ineligible_portfolio_ids, vec![PORTFOLIO_B]);
        assert!(!report.pareto_frontier_portfolio_ids.contains(&PORTFOLIO_B));
    }

    #[test]
    fn campaign_and_result_digests_are_order_invariant_and_bind_values() {
        let process = campaign(standard_supports(), specs(), standard_bounds());
        let rows = contrast_atoms_shifted();
        let first = process.evaluate(&rows).unwrap();
        let mut reversed = rows.clone();
        reversed.reverse();
        for atom in &mut reversed {
            atom.outcomes.reverse();
        }
        let second = process.evaluate(&reversed).unwrap();
        assert_eq!(first.result_digest, second.result_digest);
        assert_eq!(first.frozen_input_digest, second.frozen_input_digest);

        let mut changed = rows;
        changed[0].outcomes[0].value_ticks += 1;
        let third = process.evaluate(&changed).unwrap();
        assert_ne!(first.result_digest, third.result_digest);
    }

    #[test]
    fn gate_scenario_universe_and_authority_are_verified() {
        let mut gate = gate_report();
        gate.authority_granted = true;
        assert_eq!(
            PairedObjectiveParetoCampaign::new(
                &gate,
                standard_supports(),
                specs(),
                standard_bounds(),
            ),
            Err(PairedObjectiveParetoError::InvalidMissionRiskGate)
        );

        let mut supports = standard_supports();
        supports[0].scenario_id = [99; 32];
        assert_eq!(
            PairedObjectiveParetoCampaign::new(
                &gate_report(),
                supports,
                specs(),
                standard_bounds(),
            ),
            Err(PairedObjectiveParetoError::InvalidScenarioUniverse)
        );

        let mut supports = standard_supports();
        supports[1].scenario_model_manifest_digest = [99; 32];
        assert_eq!(
            PairedObjectiveParetoCampaign::new(
                &gate_report(),
                supports,
                specs(),
                standard_bounds(),
            ),
            Err(PairedObjectiveParetoError::ScenarioModelBindingMismatch)
        );
    }

    #[test]
    fn pairwise_contrast_bounds_are_derived_from_arm_bounds() {
        let process = campaign(
            standard_supports(),
            vec![specs()[0]],
            vec![
                bound(PORTFOLIO_A, OBJECTIVE_FUEL, 2, 8),
                bound(PORTFOLIO_C, OBJECTIVE_FUEL, 3, 9),
            ],
        );
        let rows = fuel_only_atoms()
            .into_iter()
            .map(|mut atom| {
                for outcome in &mut atom.outcomes {
                    if outcome.objective_id == OBJECTIVE_FUEL {
                        outcome.value_ticks = outcome.value_ticks.clamp(
                            if outcome.portfolio_id == PORTFOLIO_A { 2 } else { 3 },
                            if outcome.portfolio_id == PORTFOLIO_A { 8 } else { 9 },
                        );
                    }
                }
                atom
            })
            .collect::<Vec<_>>();
        let report = process.evaluate(&rows).unwrap();
        assert_eq!(report.pairwise_contrasts[0].declared_difference_lower_ticks, -7);
        assert_eq!(report.pairwise_contrasts[0].declared_difference_upper_ticks, 5);
    }

    #[test]
    fn exact_ties_do_not_create_a_dominance_witness() {
        let process = campaign(
            standard_supports(),
            vec![specs()[0]],
            vec![
                bound(PORTFOLIO_A, OBJECTIVE_FUEL, 0, 200),
                bound(PORTFOLIO_C, OBJECTIVE_FUEL, 0, 200),
            ],
        );
        let mut rows = fuel_only_atoms();
        for row in &mut rows {
            let left = row
                .outcomes
                .iter()
                .find(|outcome| {
                    outcome.portfolio_id == PORTFOLIO_A
                        && outcome.objective_id == OBJECTIVE_FUEL
                })
                .unwrap()
                .value_ticks;
            for outcome in &mut row.outcomes {
                if outcome.objective_id == OBJECTIVE_FUEL {
                    outcome.value_ticks = left;
                }
            }
        }
        let report = process.evaluate(&rows).unwrap();
        assert!(report.dominance_witnesses.is_empty());
        assert_eq!(
            report.pareto_frontier_portfolio_ids,
            vec![PORTFOLIO_A, PORTFOLIO_C]
        );
    }
}
