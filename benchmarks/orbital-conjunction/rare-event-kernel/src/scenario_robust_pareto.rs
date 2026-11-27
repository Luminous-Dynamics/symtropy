// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Conservative multi-objective selection after exact mission-risk feasibility.
//!
//! The upstream scenario mission-risk gate is the eligibility boundary. This
//! module cannot re-admit an ineligible portfolio, even when its operational
//! objectives look attractive. For eligible portfolios it constructs
//! worst-scenario objective intervals and proves only interval-separated
//! Pareto dominance. Incomparable alternatives remain on the frontier.
//!
//! Every objective interval is bound to one frozen objective identity, unit,
//! scale and model configuration across the complete scenario/portfolio matrix.
//! No weighted sum, implied preference, source authentication, physical-model
//! validation, or operational authority is introduced here.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::mission_risk_aggregation::ExactMass;
use crate::scenario_mission_risk_gate::{
    PortfolioScenarioRiskCheck, ScenarioMissionRiskGateReport,
};

const MAX_OBJECTIVES: usize = 32;
const MAX_SCENARIOS: usize = 1_024;
const MAX_PORTFOLIOS: usize = 64;

/// Direction in which an objective is optimized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveDirection {
    /// Lower values are preferred.
    Minimize,
    /// Higher values are preferred.
    Maximize,
}

/// Frozen semantic definition of one objective.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RobustParetoObjectiveSpec {
    /// Stable semantic identity of the objective.
    pub objective_id: [u8; 32],
    /// Stable identity of the physical or operational unit.
    pub unit_id: [u8; 32],
    /// Stable identity of the scale and tick interpretation.
    pub scale_id: [u8; 32],
    /// Frozen identity of the model implementation and configuration.
    pub model_configuration_digest: [u8; 32],
    /// Explicit optimization direction.
    pub direction: ObjectiveDirection,
}

/// One confidence interval for a scenario × portfolio × objective cell.
///
/// The evaluator verifies its frozen allocation and provenance bindings. It
/// assumes the producer has independently established the interval coverage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenarioPortfolioObjectiveEvidence {
    /// Frozen epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Portfolio identity, which must have passed the hard mission-risk gate.
    pub portfolio_id: [u8; 32],
    /// Frozen objective identity.
    pub objective_id: [u8; 32],
    /// Objective unit; must equal the frozen objective specification.
    pub unit_id: [u8; 32],
    /// Objective scale; must equal the frozen objective specification.
    pub scale_id: [u8; 32],
    /// Lower simultaneous confidence endpoint.
    pub lower_bound: f64,
    /// Upper simultaneous confidence endpoint.
    pub upper_bound: f64,
    /// Number of observations supporting this interval.
    pub sample_count: u64,
    /// Per-cell error allocation fixed before examining the interval.
    pub cell_alpha: f64,
    /// Digest of the source evidence used to produce the interval.
    pub source_evidence_digest: [u8; 32],
    /// Digest of the objective model and configuration.
    pub model_configuration_digest: [u8; 32],
    /// Mission manifest digest inherited from the hard-risk gate.
    pub mission_manifest_digest: [u8; 32],
    /// Mission-risk envelope digest inherited from the hard-risk gate.
    pub mission_risk_envelope_digest: [u8; 32],
    /// Post-action screening horizon digest inherited from the hard-risk gate.
    pub post_action_horizon_digest: [u8; 32],
    /// Mission-risk policy digest inherited from the hard-risk gate.
    pub risk_policy_digest: [u8; 32],
    /// Scenario target-measure identity inherited from the hard-risk gate.
    pub target_measure_digest: [u8; 32],
    /// Scenario density-space identity inherited from the hard-risk gate.
    pub density_space_digest: [u8; 32],
}

/// Conservative worst-scenario objective interval for one eligible portfolio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RobustPortfolioObjectiveEnvelope {
    /// Portfolio identity.
    pub portfolio_id: [u8; 32],
    /// Objective identity.
    pub objective_id: [u8; 32],
    /// Objective unit identity.
    pub unit_id: [u8; 32],
    /// Objective scale identity.
    pub scale_id: [u8; 32],
    /// Explicit objective direction.
    pub direction: ObjectiveDirection,
    /// Conservative lower endpoint across all epistemic scenarios.
    pub lower_bound: f64,
    /// Conservative upper endpoint across all epistemic scenarios.
    pub upper_bound: f64,
    /// Scenario attaining the lower endpoint.
    pub lower_witness_scenario_id: [u8; 32],
    /// Scenario attaining the upper endpoint.
    pub upper_witness_scenario_id: [u8; 32],
}

/// Exact hard-risk reason for excluding one portfolio in one scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IneligiblePortfolioRiskReason {
    /// Excluded portfolio identity.
    pub portfolio_id: [u8; 32],
    /// Scenario in which the risk budget was exceeded.
    pub scenario_id: [u8; 32],
    /// Exact finite-support mission-union probability mass.
    pub mission_union_mass: ExactMass,
    /// Exact scenario-specific mission-risk budget.
    pub mission_risk_budget: ExactMass,
    /// Event-inventory identity used by the hard-risk gate.
    pub event_inventory_digest: [u8; 32],
    /// Mission-risk envelope identity used by the hard-risk gate.
    pub risk_envelope_digest: [u8; 32],
    /// Post-action screening horizon identity.
    pub post_action_horizon_digest: [u8; 32],
    /// Mission-risk policy identity.
    pub risk_policy_digest: [u8; 32],
}

/// One per-objective inequality supporting a conservative dominance proof.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectiveDominanceWitness {
    /// Objective identity.
    pub objective_id: [u8; 32],
    /// Explicit objective direction.
    pub direction: ObjectiveDirection,
    /// Conservative bound for the dominating portfolio.
    pub dominating_bound: f64,
    /// Opposing conservative bound for the dominated portfolio.
    pub dominated_bound: f64,
    /// Whether this objective establishes strict improvement.
    pub strictly_better: bool,
}

/// Proof witness that one portfolio dominates another on every objective.
#[derive(Debug, Clone, PartialEq)]
pub struct PortfolioDominanceWitness {
    /// Portfolio proven no worse on every objective and strictly better on one.
    pub dominating_portfolio_id: [u8; 32],
    /// Portfolio proven dominated.
    pub dominated_portfolio_id: [u8; 32],
    /// Per-objective interval inequalities.
    pub objective_witnesses: Vec<ObjectiveDominanceWitness>,
}

/// Complete uncertainty-preserving robust Pareto result.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioRobustParetoReport {
    /// Family-wise error allocation for the objective interval family.
    pub objective_family_alpha: f64,
    /// Bonferroni allocation per scenario × eligible portfolio × objective cell.
    pub cell_alpha: f64,
    /// Number of frozen epistemic scenarios.
    pub scenario_count: usize,
    /// Number of candidates before hard-risk filtering.
    pub original_portfolio_count: usize,
    /// Number of portfolios admitted by the hard-risk gate.
    pub eligible_portfolio_count: usize,
    /// Number of declared objectives.
    pub objective_count: usize,
    /// Canonical scenario universe.
    pub scenario_ids: Vec<[u8; 32]>,
    /// Portfolios admitted by the hard-risk gate.
    pub eligible_portfolio_ids: Vec<[u8; 32]>,
    /// Portfolios excluded by the hard-risk gate.
    pub ineligible_portfolio_ids: Vec<[u8; 32]>,
    /// All scenario-specific hard-risk exclusion reasons.
    pub ineligible_risk_reasons: Vec<IneligiblePortfolioRiskReason>,
    /// Frozen objective definitions.
    pub objective_specs: Vec<RobustParetoObjectiveSpec>,
    /// Complete objective evidence matrix for eligible portfolios.
    pub objective_evidence: Vec<ScenarioPortfolioObjectiveEvidence>,
    /// Worst-scenario envelopes for each eligible portfolio and objective.
    pub objective_envelopes: Vec<RobustPortfolioObjectiveEnvelope>,
    /// Pairwise conservative dominance proofs.
    pub dominance_witnesses: Vec<PortfolioDominanceWitness>,
    /// Eligible portfolios not proven dominated by another eligible portfolio.
    pub pareto_frontier_portfolio_ids: Vec<[u8; 32]>,
    /// Digest of the frozen gate and objective universe.
    pub frozen_input_digest: [u8; 32],
    /// Digest of the complete canonical result.
    pub result_digest: [u8; 32],
    /// Always false; this evaluator has no operational authority.
    pub authority_granted: bool,
}

/// Errors from validating the hard-risk gate, objective matrix, or frontier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioRobustParetoError {
    /// Objective family alpha must lie strictly between zero and one.
    InvalidObjectiveFamilyAlpha,
    /// The upstream gate is malformed or claims operational authority.
    InvalidMissionRiskGateReport,
    /// The upstream scenario × portfolio risk matrix is incomplete or invalid.
    InvalidMissionRiskCheckMatrix,
    /// Eligible and ineligible portfolios do not partition the original universe.
    InvalidEligibilityPartition,
    /// The gate's ineligible-scenario map disagrees with exact risk checks.
    IneligibleScenarioMapMismatch,
    /// Objective identities, units, scales, or cardinality are invalid.
    InvalidObjectiveUniverse,
    /// A scenario, portfolio, or objective cell count overflowed.
    CellCountOverflow,
    /// Objective evidence does not exactly cover the frozen cell universe.
    IncompleteObjectiveEvidenceMatrix,
    /// An objective evidence identity is invalid or unexpected.
    InvalidObjectiveEvidenceIdentity,
    /// An objective interval, count, or provenance digest is invalid.
    InvalidObjectiveEvidence,
    /// An evidence row changes the frozen unit, scale, or model configuration.
    ObjectiveSemanticsMismatch,
    /// Per-cell alpha differs from the frozen allocation.
    ObjectiveAlphaMismatch,
    /// Objective evidence is not bound to the matching hard-risk cell.
    RiskEvidenceBindingMismatch,
    /// A rational mission-risk mass has an invalid denominator or range.
    InvalidExactMass,
    /// An envelope or digest calculation failed numerically.
    NumericalFailure,
}

impl core::fmt::Display for ScenarioRobustParetoError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidObjectiveFamilyAlpha => "objective family alpha must lie in (0,1)",
            Self::InvalidMissionRiskGateReport => {
                "upstream mission-risk gate report is invalid or grants authority"
            }
            Self::InvalidMissionRiskCheckMatrix => {
                "upstream scenario/portfolio risk-check matrix is incomplete or invalid"
            }
            Self::InvalidEligibilityPartition => {
                "eligible and ineligible portfolios do not partition the original universe"
            }
            Self::IneligibleScenarioMapMismatch => {
                "ineligible-scenario map disagrees with exact mission-risk checks"
            }
            Self::InvalidObjectiveUniverse => "objective specification universe is invalid",
            Self::CellCountOverflow => "scenario/portfolio/objective cell count overflowed",
            Self::IncompleteObjectiveEvidenceMatrix => {
                "objective evidence matrix is incomplete, duplicated, or unexpected"
            }
            Self::InvalidObjectiveEvidenceIdentity => {
                "objective evidence identity is zero or outside the frozen universe"
            }
            Self::InvalidObjectiveEvidence => {
                "objective interval, sample count, or provenance is invalid"
            }
            Self::ObjectiveSemanticsMismatch => {
                "objective evidence changes its frozen unit, scale, or model configuration"
            }
            Self::ObjectiveAlphaMismatch => {
                "objective interval alpha differs from the frozen per-cell allocation"
            }
            Self::RiskEvidenceBindingMismatch => {
                "objective evidence is not bound to its corresponding hard-risk evidence"
            }
            Self::InvalidExactMass => "exact mission-risk probability mass is invalid",
            Self::NumericalFailure => "robust Pareto calculation or digest failed numerically",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ScenarioRobustParetoError {}

/// Frozen robust Pareto evaluator constructed from a completed hard-risk gate.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioRobustParetoFrontier {
    objective_family_alpha: f64,
    cell_alpha: f64,
    scenario_ids: Vec<[u8; 32]>,
    eligible_portfolio_ids: Vec<[u8; 32]>,
    ineligible_portfolio_ids: Vec<[u8; 32]>,
    ineligible_risk_reasons: Vec<IneligiblePortfolioRiskReason>,
    objective_specs: Vec<RobustParetoObjectiveSpec>,
    risk_checks: BTreeMap<([u8; 32], [u8; 32]), PortfolioScenarioRiskCheck>,
    original_portfolio_count: usize,
    frozen_input_digest: [u8; 32],
}

impl ScenarioRobustParetoFrontier {
    /// Freeze the objective universe and eligible scope from the hard-risk gate.
    ///
    /// Objective alpha is separate from collision-risk and consequence-regret
    /// alpha. It is allocated uniformly before objective intervals are supplied.
    pub fn new(
        objective_family_alpha: f64,
        gate: &ScenarioMissionRiskGateReport,
        mut objective_specs: Vec<RobustParetoObjectiveSpec>,
    ) -> Result<Self, ScenarioRobustParetoError> {
        if !objective_family_alpha.is_finite()
            || objective_family_alpha <= 0.0
            || objective_family_alpha >= 1.0
        {
            return Err(ScenarioRobustParetoError::InvalidObjectiveFamilyAlpha);
        }
        if gate.authority_granted
            || !gate.total_family_alpha.is_finite()
            || gate.total_family_alpha <= 0.0
            || gate.total_family_alpha >= 1.0
            || gate.scenario_count == 0
            || gate.portfolio_count == 0
        {
            return Err(ScenarioRobustParetoError::InvalidMissionRiskGateReport);
        }
        if objective_specs.is_empty()
            || objective_specs.len() > MAX_OBJECTIVES
            || objective_specs.iter().any(|spec| {
                is_zero(&spec.objective_id)
                    || is_zero(&spec.unit_id)
                    || is_zero(&spec.scale_id)
                    || is_zero(&spec.model_configuration_digest)
            })
        {
            return Err(ScenarioRobustParetoError::InvalidObjectiveUniverse);
        }
        objective_specs.sort_by_key(|spec| spec.objective_id);
        if objective_specs
            .windows(2)
            .any(|pair| pair[0].objective_id == pair[1].objective_id)
        {
            return Err(ScenarioRobustParetoError::InvalidObjectiveUniverse);
        }

        let expected_check_count = gate
            .scenario_count
            .checked_mul(gate.portfolio_count)
            .ok_or(ScenarioRobustParetoError::CellCountOverflow)?;
        if gate.scenario_count > MAX_SCENARIOS
            || gate.portfolio_count > MAX_PORTFOLIOS
            || gate.scenario_portfolio_risk_checks.len() != expected_check_count
        {
            return Err(ScenarioRobustParetoError::InvalidMissionRiskCheckMatrix);
        }

        let mut scenarios = BTreeSet::new();
        let mut original_portfolios = BTreeSet::new();
        let mut risk_checks = BTreeMap::new();
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
                return Err(ScenarioRobustParetoError::InvalidMissionRiskCheckMatrix);
            }
            let exact_within = mass_leq(check.mission_union_mass, check.mission_risk_budget);
            if exact_within != check.within_declared_finite_support_budget {
                return Err(ScenarioRobustParetoError::InvalidMissionRiskCheckMatrix);
            }
            if !exact_within {
                derived_ineligible
                    .entry(check.portfolio_id)
                    .or_default()
                    .push(check.scenario_id);
            }
            scenarios.insert(check.scenario_id);
            original_portfolios.insert(check.portfolio_id);
            if risk_checks
                .insert((check.scenario_id, check.portfolio_id), *check)
                .is_some()
            {
                return Err(ScenarioRobustParetoError::InvalidMissionRiskCheckMatrix);
            }
        }

        if scenarios.len() != gate.scenario_count
            || original_portfolios.len() != gate.portfolio_count
        {
            return Err(ScenarioRobustParetoError::InvalidMissionRiskCheckMatrix);
        }
        let scenario_ids = scenarios.into_iter().collect::<Vec<_>>();
        let original_portfolio_ids = original_portfolios.into_iter().collect::<Vec<_>>();

        let mut eligible_portfolio_ids = gate.eligible_portfolio_ids.clone();
        let mut ineligible_portfolio_ids = gate.ineligible_portfolio_ids.clone();
        eligible_portfolio_ids.sort_unstable();
        ineligible_portfolio_ids.sort_unstable();
        if eligible_portfolio_ids.iter().any(is_zero)
            || ineligible_portfolio_ids.iter().any(is_zero)
            || eligible_portfolio_ids
                .windows(2)
                .any(|pair| pair[0] == pair[1])
            || ineligible_portfolio_ids
                .windows(2)
                .any(|pair| pair[0] == pair[1])
        {
            return Err(ScenarioRobustParetoError::InvalidEligibilityPartition);
        }
        let eligible_set = eligible_portfolio_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let ineligible_set = ineligible_portfolio_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let original_set = original_portfolio_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if eligible_set.len() + ineligible_set.len() != original_set.len()
            || !eligible_set.is_disjoint(&ineligible_set)
            || eligible_set
                .union(&ineligible_set)
                .copied()
                .collect::<BTreeSet<_>>()
                != original_set
        {
            return Err(ScenarioRobustParetoError::InvalidEligibilityPartition);
        }

        let normalize_map = |map: &BTreeMap<[u8; 32], Vec<[u8; 32]>>| {
            map.iter()
                .map(|(portfolio_id, scenario_ids)| {
                    let mut ids = scenario_ids.clone();
                    ids.sort_unstable();
                    (*portfolio_id, ids)
                })
                .collect::<BTreeMap<_, _>>()
        };
        let expected_map = normalize_map(&derived_ineligible);
        let supplied_map = normalize_map(&gate.ineligible_scenarios_by_portfolio);
        if expected_map != supplied_map
            || eligible_portfolio_ids
                != original_portfolio_ids
                    .iter()
                    .filter(|id| !expected_map.contains_key(*id))
                    .copied()
                    .collect::<Vec<_>>()
            || ineligible_portfolio_ids != expected_map.keys().copied().collect::<Vec<_>>()
        {
            return Err(ScenarioRobustParetoError::IneligibleScenarioMapMismatch);
        }

        let mut ineligible_risk_reasons = Vec::new();
        for check in gate
            .scenario_portfolio_risk_checks
            .iter()
            .filter(|check| !check.within_declared_finite_support_budget)
        {
            ineligible_risk_reasons.push(IneligiblePortfolioRiskReason {
                portfolio_id: check.portfolio_id,
                scenario_id: check.scenario_id,
                mission_union_mass: check.mission_union_mass,
                mission_risk_budget: check.mission_risk_budget,
                event_inventory_digest: check.event_inventory_digest,
                risk_envelope_digest: check.risk_envelope_digest,
                post_action_horizon_digest: check.post_action_horizon_digest,
                risk_policy_digest: check.risk_policy_digest,
            });
        }
        ineligible_risk_reasons.sort_by_key(|reason| (reason.portfolio_id, reason.scenario_id));

        let cell_count = scenario_ids
            .len()
            .checked_mul(eligible_portfolio_ids.len())
            .and_then(|count| count.checked_mul(objective_specs.len()))
            .ok_or(ScenarioRobustParetoError::CellCountOverflow)?;
        let cell_alpha = if cell_count == 0 {
            0.0
        } else {
            objective_family_alpha / cell_count as f64
        };
        if !cell_alpha.is_finite() || (cell_count > 0 && cell_alpha <= 0.0) {
            return Err(ScenarioRobustParetoError::NumericalFailure);
        }

        let frozen_input_digest = digest_frozen_inputs(FrozenParetoDigestInputs {
            objective_family_alpha,
            cell_alpha,
            scenario_ids: &scenario_ids,
            original_portfolio_ids: &original_portfolio_ids,
            eligible_portfolio_ids: &eligible_portfolio_ids,
            ineligible_portfolio_ids: &ineligible_portfolio_ids,
            objective_specs: &objective_specs,
            risk_checks: &risk_checks,
        });
        Ok(Self {
            objective_family_alpha,
            cell_alpha,
            scenario_ids,
            eligible_portfolio_ids,
            ineligible_portfolio_ids,
            ineligible_risk_reasons,
            objective_specs,
            risk_checks,
            original_portfolio_count: gate.portfolio_count,
            frozen_input_digest,
        })
    }

    /// Per-cell alpha allocated across the frozen objective family.
    pub fn cell_alpha(&self) -> f64 {
        self.cell_alpha
    }

    /// Number of objective cells required by the frozen eligible scope.
    pub fn expected_cell_count(&self) -> usize {
        self.scenario_ids.len() * self.eligible_portfolio_ids.len() * self.objective_specs.len()
    }

    /// Digest of the frozen risk-gate and objective-specification inputs.
    pub fn frozen_input_digest(&self) -> [u8; 32] {
        self.frozen_input_digest
    }

    /// Validate the complete objective matrix and compute the robust frontier.
    pub fn evaluate(
        &self,
        evidence: &[ScenarioPortfolioObjectiveEvidence],
    ) -> Result<ScenarioRobustParetoReport, ScenarioRobustParetoError> {
        let expected_count = self.expected_cell_count();
        if evidence.len() != expected_count {
            return Err(ScenarioRobustParetoError::IncompleteObjectiveEvidenceMatrix);
        }

        let spec_map = self
            .objective_specs
            .iter()
            .map(|spec| (spec.objective_id, *spec))
            .collect::<BTreeMap<_, _>>();
        let mut cells = BTreeMap::new();
        for row in evidence {
            if is_zero(&row.scenario_id) || is_zero(&row.portfolio_id) || is_zero(&row.objective_id)
            {
                return Err(ScenarioRobustParetoError::InvalidObjectiveEvidenceIdentity);
            }
            if !self.scenario_ids.contains(&row.scenario_id)
                || !self.eligible_portfolio_ids.contains(&row.portfolio_id)
                || !spec_map.contains_key(&row.objective_id)
            {
                return Err(ScenarioRobustParetoError::InvalidObjectiveEvidenceIdentity);
            }
            if cells
                .insert((row.scenario_id, row.portfolio_id, row.objective_id), *row)
                .is_some()
            {
                return Err(ScenarioRobustParetoError::IncompleteObjectiveEvidenceMatrix);
            }
        }
        if cells.len() != expected_count {
            return Err(ScenarioRobustParetoError::IncompleteObjectiveEvidenceMatrix);
        }

        for scenario_id in &self.scenario_ids {
            for portfolio_id in &self.eligible_portfolio_ids {
                let check = self
                    .risk_checks
                    .get(&(*scenario_id, *portfolio_id))
                    .ok_or(ScenarioRobustParetoError::InvalidMissionRiskCheckMatrix)?;
                for spec in &self.objective_specs {
                    let row = cells
                        .get(&(*scenario_id, *portfolio_id, spec.objective_id))
                        .ok_or(ScenarioRobustParetoError::IncompleteObjectiveEvidenceMatrix)?;
                    self.validate_objective_row(row, spec, check)?;
                }
            }
        }

        let objective_evidence = cells.values().copied().collect::<Vec<_>>();
        let mut objective_envelopes =
            Vec::with_capacity(self.eligible_portfolio_ids.len() * self.objective_specs.len());
        for portfolio_id in &self.eligible_portfolio_ids {
            for spec in &self.objective_specs {
                let mut lower = match spec.direction {
                    ObjectiveDirection::Minimize => f64::NEG_INFINITY,
                    ObjectiveDirection::Maximize => f64::INFINITY,
                };
                let mut upper = lower;
                let mut lower_witness = self.scenario_ids[0];
                let mut upper_witness = self.scenario_ids[0];
                for scenario_id in &self.scenario_ids {
                    let row = cells
                        .get(&(*scenario_id, *portfolio_id, spec.objective_id))
                        .ok_or(ScenarioRobustParetoError::IncompleteObjectiveEvidenceMatrix)?;
                    match spec.direction {
                        ObjectiveDirection::Minimize => {
                            if row.lower_bound > lower {
                                lower = row.lower_bound;
                                lower_witness = *scenario_id;
                            }
                            if row.upper_bound > upper {
                                upper = row.upper_bound;
                                upper_witness = *scenario_id;
                            }
                        }
                        ObjectiveDirection::Maximize => {
                            if row.lower_bound < lower {
                                lower = row.lower_bound;
                                lower_witness = *scenario_id;
                            }
                            if row.upper_bound < upper {
                                upper = row.upper_bound;
                                upper_witness = *scenario_id;
                            }
                        }
                    }
                }
                if !lower.is_finite() || !upper.is_finite() || lower > upper {
                    return Err(ScenarioRobustParetoError::NumericalFailure);
                }
                objective_envelopes.push(RobustPortfolioObjectiveEnvelope {
                    portfolio_id: *portfolio_id,
                    objective_id: spec.objective_id,
                    unit_id: spec.unit_id,
                    scale_id: spec.scale_id,
                    direction: spec.direction,
                    lower_bound: lower,
                    upper_bound: upper,
                    lower_witness_scenario_id: lower_witness,
                    upper_witness_scenario_id: upper_witness,
                });
            }
        }

        let mut dominance_witnesses = Vec::new();
        let mut dominated = BTreeSet::new();
        for (index, left_id) in self.eligible_portfolio_ids.iter().enumerate() {
            for right_id in self.eligible_portfolio_ids.iter().skip(index + 1) {
                if let Some(witness) = dominance_witness(
                    *left_id,
                    *right_id,
                    &self.objective_specs,
                    &objective_envelopes,
                )? {
                    dominated.insert(witness.dominated_portfolio_id);
                    dominance_witnesses.push(witness);
                } else if let Some(witness) = dominance_witness(
                    *right_id,
                    *left_id,
                    &self.objective_specs,
                    &objective_envelopes,
                )? {
                    dominated.insert(witness.dominated_portfolio_id);
                    dominance_witnesses.push(witness);
                }
            }
        }

        let pareto_frontier_portfolio_ids = self
            .eligible_portfolio_ids
            .iter()
            .filter(|id| !dominated.contains(*id))
            .copied()
            .collect::<Vec<_>>();
        let mut report = ScenarioRobustParetoReport {
            objective_family_alpha: self.objective_family_alpha,
            cell_alpha: self.cell_alpha,
            scenario_count: self.scenario_ids.len(),
            original_portfolio_count: self.original_portfolio_count,
            eligible_portfolio_count: self.eligible_portfolio_ids.len(),
            objective_count: self.objective_specs.len(),
            scenario_ids: self.scenario_ids.clone(),
            eligible_portfolio_ids: self.eligible_portfolio_ids.clone(),
            ineligible_portfolio_ids: self.ineligible_portfolio_ids.clone(),
            ineligible_risk_reasons: self.ineligible_risk_reasons.clone(),
            objective_specs: self.objective_specs.clone(),
            objective_evidence,
            objective_envelopes,
            dominance_witnesses,
            pareto_frontier_portfolio_ids,
            frozen_input_digest: self.frozen_input_digest,
            result_digest: [0; 32],
            authority_granted: false,
        };
        report.result_digest = digest_report(&report);
        Ok(report)
    }

    fn validate_objective_row(
        &self,
        row: &ScenarioPortfolioObjectiveEvidence,
        spec: &RobustParetoObjectiveSpec,
        check: &PortfolioScenarioRiskCheck,
    ) -> Result<(), ScenarioRobustParetoError> {
        if !row.lower_bound.is_finite()
            || !row.upper_bound.is_finite()
            || row.lower_bound > row.upper_bound
            || row.sample_count == 0
            || is_zero(&row.source_evidence_digest)
            || is_zero(&row.model_configuration_digest)
            || is_zero(&row.mission_manifest_digest)
            || is_zero(&row.mission_risk_envelope_digest)
            || is_zero(&row.post_action_horizon_digest)
            || is_zero(&row.risk_policy_digest)
            || is_zero(&row.target_measure_digest)
            || is_zero(&row.density_space_digest)
        {
            return Err(ScenarioRobustParetoError::InvalidObjectiveEvidence);
        }
        if !close_enough(row.cell_alpha, self.cell_alpha) {
            return Err(ScenarioRobustParetoError::ObjectiveAlphaMismatch);
        }
        if row.unit_id != spec.unit_id
            || row.scale_id != spec.scale_id
            || row.model_configuration_digest != spec.model_configuration_digest
        {
            return Err(ScenarioRobustParetoError::ObjectiveSemanticsMismatch);
        }
        if row.mission_manifest_digest != check.manifest_digest
            || row.mission_risk_envelope_digest != check.risk_envelope_digest
            || row.post_action_horizon_digest != check.post_action_horizon_digest
            || row.risk_policy_digest != check.risk_policy_digest
            || row.target_measure_digest != check.target_measure_digest
            || row.density_space_digest != check.density_space_digest
        {
            return Err(ScenarioRobustParetoError::RiskEvidenceBindingMismatch);
        }
        Ok(())
    }
}

fn dominance_witness(
    dominating_id: [u8; 32],
    dominated_id: [u8; 32],
    specs: &[RobustParetoObjectiveSpec],
    envelopes: &[RobustPortfolioObjectiveEnvelope],
) -> Result<Option<PortfolioDominanceWitness>, ScenarioRobustParetoError> {
    let mut all_no_worse = true;
    let mut any_strict = false;
    let mut witnesses = Vec::with_capacity(specs.len());
    for spec in objective_specs {
        let better = envelopes
            .iter()
            .find(|row| row.portfolio_id == dominating_id && row.objective_id == spec.objective_id)
            .ok_or(ScenarioRobustParetoError::NumericalFailure)?;
        let worse = envelopes
            .iter()
            .find(|row| row.portfolio_id == dominated_id && row.objective_id == spec.objective_id)
            .ok_or(ScenarioRobustParetoError::NumericalFailure)?;
        let (no_worse, strictly_better, better_bound, worse_bound) = match spec.direction {
            ObjectiveDirection::Minimize => (
                better.upper_bound <= worse.lower_bound,
                better.upper_bound < worse.lower_bound,
                better.upper_bound,
                worse.lower_bound,
            ),
            ObjectiveDirection::Maximize => (
                better.lower_bound >= worse.upper_bound,
                better.lower_bound > worse.upper_bound,
                better.lower_bound,
                worse.upper_bound,
            ),
        };
        all_no_worse &= no_worse;
        any_strict |= strictly_better;
        witnesses.push(ObjectiveDominanceWitness {
            objective_id: spec.objective_id,
            direction: spec.direction,
            dominating_bound: better_bound,
            dominated_bound: worse_bound,
            strictly_better,
        });
    }
    if all_no_worse && any_strict {
        Ok(Some(PortfolioDominanceWitness {
            dominating_portfolio_id: dominating_id,
            dominated_portfolio_id: dominated_id,
            objective_witnesses: witnesses,
        }))
    } else {
        Ok(None)
    }
}

struct FrozenParetoDigestInputs<'a> {
    objective_family_alpha: f64,
    cell_alpha: f64,
    scenario_ids: &'a [[u8; 32]],
    original_portfolio_ids: &'a [[u8; 32]],
    eligible_portfolio_ids: &'a [[u8; 32]],
    ineligible_portfolio_ids: &'a [[u8; 32]],
    objective_specs: &'a [RobustParetoObjectiveSpec],
    risk_checks: &'a BTreeMap<([u8; 32], [u8; 32]), PortfolioScenarioRiskCheck>,
}

fn digest_frozen_inputs(inputs: FrozenParetoDigestInputs<'_>) -> [u8; 32] {
    let FrozenParetoDigestInputs {
        objective_family_alpha,
        cell_alpha,
        scenario_ids,
        original_portfolio_ids,
        eligible_portfolio_ids,
        ineligible_portfolio_ids,
        objective_specs,
        risk_checks,
    } = inputs;
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.scenario-robust-pareto.frozen.v0.1\0");
    hasher.update(objective_family_alpha.to_bits().to_be_bytes());
    hasher.update(cell_alpha.to_bits().to_be_bytes());
    hash_ids(&mut hasher, scenario_ids);
    hash_ids(&mut hasher, original_portfolio_ids);
    hash_ids(&mut hasher, eligible_portfolio_ids);
    hash_ids(&mut hasher, ineligible_portfolio_ids);
    hasher.update((objective_specs.len() as u64).to_be_bytes());
    for spec in objective_specs {
        hasher.update(spec.objective_id);
        hasher.update(spec.unit_id);
        hasher.update(spec.scale_id);
        hasher.update(spec.model_configuration_digest);
        hasher.update([direction_byte(spec.direction)]);
    }
    hasher.update((risk_checks.len() as u64).to_be_bytes());
    for ((scenario_id, portfolio_id), check) in risk_checks {
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
    hasher.finalize().into()
}

fn digest_report(report: &ScenarioRobustParetoReport) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.scenario-robust-pareto.report.v0.1\0");
    hasher.update(report.frozen_input_digest);
    hasher.update(report.objective_family_alpha.to_bits().to_be_bytes());
    hasher.update(report.cell_alpha.to_bits().to_be_bytes());
    hasher.update((report.objective_evidence.len() as u64).to_be_bytes());
    for row in &report.objective_evidence {
        hasher.update(row.scenario_id);
        hasher.update(row.portfolio_id);
        hasher.update(row.objective_id);
        hasher.update(row.unit_id);
        hasher.update(row.scale_id);
        hasher.update(row.lower_bound.to_bits().to_be_bytes());
        hasher.update(row.upper_bound.to_bits().to_be_bytes());
        hasher.update(row.sample_count.to_be_bytes());
        hasher.update(row.cell_alpha.to_bits().to_be_bytes());
        hasher.update(row.source_evidence_digest);
        hasher.update(row.model_configuration_digest);
        hasher.update(row.mission_manifest_digest);
        hasher.update(row.mission_risk_envelope_digest);
        hasher.update(row.post_action_horizon_digest);
        hasher.update(row.risk_policy_digest);
        hasher.update(row.target_measure_digest);
        hasher.update(row.density_space_digest);
    }
    hasher.update((report.objective_envelopes.len() as u64).to_be_bytes());
    for row in &report.objective_envelopes {
        hasher.update(row.portfolio_id);
        hasher.update(row.objective_id);
        hasher.update(row.unit_id);
        hasher.update(row.scale_id);
        hasher.update([direction_byte(row.direction)]);
        hasher.update(row.lower_bound.to_bits().to_be_bytes());
        hasher.update(row.upper_bound.to_bits().to_be_bytes());
        hasher.update(row.lower_witness_scenario_id);
        hasher.update(row.upper_witness_scenario_id);
    }
    hasher.update((report.dominance_witnesses.len() as u64).to_be_bytes());
    for witness in &report.dominance_witnesses {
        hasher.update(witness.dominating_portfolio_id);
        hasher.update(witness.dominated_portfolio_id);
        for objective in &witness.objective_witnesses {
            hasher.update(objective.objective_id);
            hasher.update([direction_byte(objective.direction)]);
            hasher.update(objective.dominating_bound.to_bits().to_be_bytes());
            hasher.update(objective.dominated_bound.to_bits().to_be_bytes());
            hasher.update([u8::from(objective.strictly_better)]);
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

fn direction_byte(direction: ObjectiveDirection) -> u8 {
    match direction {
        ObjectiveDirection::Minimize => 0,
        ObjectiveDirection::Maximize => 1,
    }
}

fn is_zero(id: &[u8; 32]) -> bool {
    id.iter().all(|byte| *byte == 0)
}

fn valid_mass(mass: ExactMass) -> bool {
    mass.denominator > 0 && mass.numerator <= mass.denominator
}

fn mass_leq(left: ExactMass, right: ExactMass) -> bool {
    u128::from(left.numerator) * u128::from(right.denominator)
        <= u128::from(right.numerator) * u128::from(left.denominator)
}

fn close_enough(left: f64, right: f64) -> bool {
    (left - right).abs() <= 64.0 * f64::EPSILON * left.abs().max(right.abs()).max(1.0)
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
    const OBJECTIVE_DOWNTIME: [u8; 32] = [41; 32];
    const OBJECTIVE_PERFORMANCE: [u8; 32] = [42; 32];

    fn risk_check(
        scenario_id: [u8; 32],
        portfolio_id: [u8; 32],
        within_budget: bool,
    ) -> PortfolioScenarioRiskCheck {
        PortfolioScenarioRiskCheck {
            scenario_id,
            portfolio_id,
            manifest_digest: [50; 32],
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

    fn specs() -> Vec<RobustParetoObjectiveSpec> {
        vec![
            RobustParetoObjectiveSpec {
                objective_id: OBJECTIVE_FUEL,
                unit_id: [60; 32],
                scale_id: [61; 32],
                model_configuration_digest: [67; 32],
                direction: ObjectiveDirection::Minimize,
            },
            RobustParetoObjectiveSpec {
                objective_id: OBJECTIVE_DOWNTIME,
                unit_id: [62; 32],
                scale_id: [63; 32],
                model_configuration_digest: [67; 32],
                direction: ObjectiveDirection::Minimize,
            },
            RobustParetoObjectiveSpec {
                objective_id: OBJECTIVE_PERFORMANCE,
                unit_id: [64; 32],
                scale_id: [65; 32],
                model_configuration_digest: [67; 32],
                direction: ObjectiveDirection::Maximize,
            },
        ]
    }

    fn row(
        scenario_id: [u8; 32],
        portfolio_id: [u8; 32],
        objective_id: [u8; 32],
        lower_bound: f64,
        upper_bound: f64,
        cell_alpha: f64,
    ) -> ScenarioPortfolioObjectiveEvidence {
        let check = risk_check(
            scenario_id,
            portfolio_id,
            !(portfolio_id == PORTFOLIO_B && scenario_id == SCENARIO_B),
        );
        let (unit_id, scale_id) = match objective_id {
            OBJECTIVE_FUEL => ([60; 32], [61; 32]),
            OBJECTIVE_DOWNTIME => ([62; 32], [63; 32]),
            OBJECTIVE_PERFORMANCE => ([64; 32], [65; 32]),
            _ => ([0; 32], [0; 32]),
        };
        ScenarioPortfolioObjectiveEvidence {
            scenario_id,
            portfolio_id,
            objective_id,
            unit_id,
            scale_id,
            lower_bound,
            upper_bound,
            sample_count: 100,
            cell_alpha,
            source_evidence_digest: [66; 32],
            model_configuration_digest: [67; 32],
            mission_manifest_digest: check.manifest_digest,
            mission_risk_envelope_digest: check.risk_envelope_digest,
            post_action_horizon_digest: check.post_action_horizon_digest,
            risk_policy_digest: check.risk_policy_digest,
            target_measure_digest: check.target_measure_digest,
            density_space_digest: check.density_space_digest,
        }
    }

    fn evidence(process: &ScenarioRobustParetoFrontier) -> Vec<ScenarioPortfolioObjectiveEvidence> {
        let alpha = process.cell_alpha();
        let mut rows = Vec::new();
        for (scenario_id, portfolio_id, fuel, downtime, performance) in [
            (SCENARIO_A, PORTFOLIO_A, (2.0, 2.0), (2.0, 2.0), (7.0, 8.0)),
            (SCENARIO_B, PORTFOLIO_A, (4.0, 4.0), (5.0, 5.0), (8.0, 9.0)),
            (SCENARIO_A, PORTFOLIO_C, (3.5, 3.5), (4.0, 4.0), (6.0, 7.0)),
            (SCENARIO_B, PORTFOLIO_C, (3.5, 3.5), (4.0, 4.0), (6.0, 7.0)),
        ] {
            rows.push(row(
                scenario_id,
                portfolio_id,
                OBJECTIVE_FUEL,
                fuel.0,
                fuel.1,
                alpha,
            ));
            rows.push(row(
                scenario_id,
                portfolio_id,
                OBJECTIVE_DOWNTIME,
                downtime.0,
                downtime.1,
                alpha,
            ));
            rows.push(row(
                scenario_id,
                portfolio_id,
                OBJECTIVE_PERFORMANCE,
                performance.0,
                performance.1,
                alpha,
            ));
        }
        rows
    }

    #[test]
    fn alpha_is_allocated_over_scenarios_eligible_portfolios_and_objectives() {
        let process = ScenarioRobustParetoFrontier::new(0.06, &gate_report(), specs()).unwrap();
        assert_eq!(process.expected_cell_count(), 12);
        assert!((process.cell_alpha() - 0.005).abs() < 1.0e-15);
    }

    #[test]
    fn hard_risk_ineligible_portfolio_never_reenters_the_frontier() {
        let process = ScenarioRobustParetoFrontier::new(0.05, &gate_report(), specs()).unwrap();
        let report = process.evaluate(&evidence(&process)).unwrap();
        assert_eq!(
            report.eligible_portfolio_ids,
            vec![PORTFOLIO_A, PORTFOLIO_C]
        );
        assert_eq!(report.ineligible_portfolio_ids, vec![PORTFOLIO_B]);
        assert_eq!(report.ineligible_risk_reasons.len(), 1);
        assert_eq!(report.ineligible_risk_reasons[0].scenario_id, SCENARIO_B);
        assert!(!report.pareto_frontier_portfolio_ids.contains(&PORTFOLIO_B));
    }

    #[test]
    fn worst_scenario_envelopes_do_not_average_epistemic_alternatives() {
        let process = ScenarioRobustParetoFrontier::new(0.05, &gate_report(), specs()).unwrap();
        let report = process.evaluate(&evidence(&process)).unwrap();
        let fuel_a = report
            .objective_envelopes
            .iter()
            .find(|row| row.portfolio_id == PORTFOLIO_A && row.objective_id == OBJECTIVE_FUEL)
            .unwrap();
        let performance_a = report
            .objective_envelopes
            .iter()
            .find(|row| {
                row.portfolio_id == PORTFOLIO_A && row.objective_id == OBJECTIVE_PERFORMANCE
            })
            .unwrap();
        assert_eq!(fuel_a.lower_bound, 4.0);
        assert_eq!(fuel_a.upper_bound, 4.0);
        assert_eq!(fuel_a.lower_witness_scenario_id, SCENARIO_B);
        assert_eq!(performance_a.lower_bound, 7.0);
        assert_eq!(performance_a.upper_bound, 8.0);
        assert_eq!(performance_a.lower_witness_scenario_id, SCENARIO_A);
        assert_eq!(performance_a.upper_witness_scenario_id, SCENARIO_A);
    }

    #[test]
    fn incomparable_tradeoffs_remain_on_the_frontier() {
        let process = ScenarioRobustParetoFrontier::new(0.05, &gate_report(), specs()).unwrap();
        let report = process.evaluate(&evidence(&process)).unwrap();
        assert_eq!(
            report.pareto_frontier_portfolio_ids,
            vec![PORTFOLIO_A, PORTFOLIO_C]
        );
        assert!(report.dominance_witnesses.is_empty());
    }

    #[test]
    fn missing_duplicate_or_substituted_cells_are_rejected() {
        let process = ScenarioRobustParetoFrontier::new(0.05, &gate_report(), specs()).unwrap();
        let mut rows = evidence(&process);
        rows.pop();
        assert_eq!(
            process.evaluate(&rows),
            Err(ScenarioRobustParetoError::IncompleteObjectiveEvidenceMatrix)
        );
        let mut rows = evidence(&process);
        rows.push(rows[0]);
        assert_eq!(
            process.evaluate(&rows),
            Err(ScenarioRobustParetoError::IncompleteObjectiveEvidenceMatrix)
        );
        let mut rows = evidence(&process);
        rows[0].portfolio_id = PORTFOLIO_B;
        assert_eq!(
            process.evaluate(&rows),
            Err(ScenarioRobustParetoError::InvalidObjectiveEvidenceIdentity)
        );
    }

    #[test]
    fn objective_alpha_and_risk_source_bindings_are_enforced() {
        let process = ScenarioRobustParetoFrontier::new(0.05, &gate_report(), specs()).unwrap();
        let mut rows = evidence(&process);
        rows[0].cell_alpha *= 0.5;
        assert_eq!(
            process.evaluate(&rows),
            Err(ScenarioRobustParetoError::ObjectiveAlphaMismatch)
        );
        let mut rows = evidence(&process);
        rows[0].risk_policy_digest = [99; 32];
        assert_eq!(
            process.evaluate(&rows),
            Err(ScenarioRobustParetoError::RiskEvidenceBindingMismatch)
        );

        let mut rows = evidence(&process);
        rows[0].unit_id = [98; 32];
        assert_eq!(
            process.evaluate(&rows),
            Err(ScenarioRobustParetoError::ObjectiveSemanticsMismatch)
        );

        let mut rows = evidence(&process);
        rows[0].scale_id = [97; 32];
        assert_eq!(
            process.evaluate(&rows),
            Err(ScenarioRobustParetoError::ObjectiveSemanticsMismatch)
        );

        let mut rows = evidence(&process);
        rows[0].model_configuration_digest = [96; 32];
        assert_eq!(
            process.evaluate(&rows),
            Err(ScenarioRobustParetoError::ObjectiveSemanticsMismatch)
        );
    }

    #[test]
    fn conservative_dominance_requires_a_strict_gap_on_one_objective() {
        let process = ScenarioRobustParetoFrontier::new(
            0.05,
            &gate_report(),
            vec![RobustParetoObjectiveSpec {
                objective_id: OBJECTIVE_FUEL,
                unit_id: [60; 32],
                scale_id: [61; 32],
                model_configuration_digest: [67; 32],
                direction: ObjectiveDirection::Minimize,
            }],
        )
        .unwrap();
        let alpha = process.cell_alpha();
        let rows = vec![
            row(SCENARIO_A, PORTFOLIO_A, OBJECTIVE_FUEL, 1.0, 2.0, alpha),
            row(SCENARIO_B, PORTFOLIO_A, OBJECTIVE_FUEL, 1.0, 2.0, alpha),
            row(SCENARIO_A, PORTFOLIO_C, OBJECTIVE_FUEL, 3.0, 4.0, alpha),
            row(SCENARIO_B, PORTFOLIO_C, OBJECTIVE_FUEL, 3.0, 4.0, alpha),
        ];
        let report = process.evaluate(&rows).unwrap();
        assert_eq!(report.pareto_frontier_portfolio_ids, vec![PORTFOLIO_A]);
        assert_eq!(report.dominance_witnesses.len(), 1);
        assert_eq!(
            report.dominance_witnesses[0].dominating_portfolio_id,
            PORTFOLIO_A
        );
        assert_eq!(
            report.dominance_witnesses[0].dominated_portfolio_id,
            PORTFOLIO_C
        );
        assert!(report.dominance_witnesses[0].objective_witnesses[0].strictly_better);
    }

    #[test]
    fn touching_intervals_are_not_called_strictly_dominant() {
        let process = ScenarioRobustParetoFrontier::new(
            0.05,
            &gate_report(),
            vec![RobustParetoObjectiveSpec {
                objective_id: OBJECTIVE_FUEL,
                unit_id: [60; 32],
                scale_id: [61; 32],
                model_configuration_digest: [67; 32],
                direction: ObjectiveDirection::Minimize,
            }],
        )
        .unwrap();
        let alpha = process.cell_alpha();
        let rows = vec![
            row(SCENARIO_A, PORTFOLIO_A, OBJECTIVE_FUEL, 1.0, 2.0, alpha),
            row(SCENARIO_B, PORTFOLIO_A, OBJECTIVE_FUEL, 1.0, 2.0, alpha),
            row(SCENARIO_A, PORTFOLIO_C, OBJECTIVE_FUEL, 2.0, 3.0, alpha),
            row(SCENARIO_B, PORTFOLIO_C, OBJECTIVE_FUEL, 2.0, 3.0, alpha),
        ];
        let report = process.evaluate(&rows).unwrap();
        assert!(report.dominance_witnesses.is_empty());
        assert_eq!(
            report.pareto_frontier_portfolio_ids,
            vec![PORTFOLIO_A, PORTFOLIO_C]
        );
    }

    #[test]
    fn all_hard_risk_ineligible_portfolios_produce_an_empty_frontier() {
        let mut gate = gate_report();
        gate.eligible_portfolio_ids.clear();
        gate.ineligible_portfolio_ids = vec![PORTFOLIO_A, PORTFOLIO_B, PORTFOLIO_C];
        gate.ineligible_scenarios_by_portfolio = BTreeMap::from([
            (PORTFOLIO_A, vec![SCENARIO_A]),
            (PORTFOLIO_B, vec![SCENARIO_B]),
            (PORTFOLIO_C, vec![SCENARIO_A]),
        ]);
        for check in &mut gate.scenario_portfolio_risk_checks {
            let excluded = (check.portfolio_id == PORTFOLIO_A && check.scenario_id == SCENARIO_A)
                || (check.portfolio_id == PORTFOLIO_B && check.scenario_id == SCENARIO_B)
                || (check.portfolio_id == PORTFOLIO_C && check.scenario_id == SCENARIO_A);
            if excluded {
                check.within_declared_finite_support_budget = false;
                check.mission_union_mass = ExactMass {
                    numerator: 2,
                    denominator: 100,
                };
            }
        }
        let process = ScenarioRobustParetoFrontier::new(0.05, &gate, specs()).unwrap();
        assert_eq!(process.expected_cell_count(), 0);
        let report = process.evaluate(&[]).unwrap();
        assert!(report.pareto_frontier_portfolio_ids.is_empty());
        assert!(report.objective_envelopes.is_empty());
        assert_eq!(report.ineligible_risk_reasons.len(), 3);
    }

    #[test]
    fn objective_identities_must_be_unique() {
        let mut invalid = specs();
        invalid.push(invalid[0]);
        assert_eq!(
            ScenarioRobustParetoFrontier::new(0.05, &gate_report(), invalid),
            Err(ScenarioRobustParetoError::InvalidObjectiveUniverse)
        );
    }

    #[test]
    fn invalid_mass_or_gate_authority_is_rejected() {
        let mut gate = gate_report();
        gate.authority_granted = true;
        assert_eq!(
            ScenarioRobustParetoFrontier::new(0.05, &gate, specs()),
            Err(ScenarioRobustParetoError::InvalidMissionRiskGateReport)
        );
        let mut gate = gate_report();
        gate.scenario_portfolio_risk_checks[0]
            .mission_risk_budget
            .denominator = 0;
        assert_eq!(
            ScenarioRobustParetoFrontier::new(0.05, &gate, specs()),
            Err(ScenarioRobustParetoError::InvalidMissionRiskCheckMatrix)
        );
    }

    #[test]
    fn result_digest_is_order_invariant_and_binds_source_evidence() {
        let process = ScenarioRobustParetoFrontier::new(0.05, &gate_report(), specs()).unwrap();
        let rows = evidence(&process);
        let first = process.evaluate(&rows).unwrap();
        let mut reversed = rows.clone();
        reversed.reverse();
        let second = process.evaluate(&reversed).unwrap();
        assert_eq!(first.result_digest, second.result_digest);
        assert_ne!(first.result_digest, [0; 32]);
        let mut changed = rows;
        changed[0].source_evidence_digest = [101; 32];
        let third = process.evaluate(&changed).unwrap();
        assert_ne!(first.result_digest, third.result_digest);
    }
}
