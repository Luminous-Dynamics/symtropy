// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Hard finite-support mission-union feasibility across epistemic scenarios.
//!
//! This gate consumes exact MissionRiskEnvelope values for every frozen
//! scenario × manoeuvre-portfolio cell. A portfolio is eligible only when its
//! pathwise mission-event union is at or below the frozen rational budget in
//! every required scenario.
//!
//! Only after this hard filter does the module calculate a minimax-regret
//! ranking among the eligible portfolio set, using the already simultaneous
//! scenario/pair consequence intervals. The family alpha remains allocated
//! over the full original scenario × pair universe; it is not re-spent after
//! filtering.
//!
//! A passing result means only that every supplied declared finite-support
//! mission-risk envelope is below its frozen budget. It does not prove the
//! underlying physical event inventory or trajectory support is complete,
//! authenticate an issuer, or authorize a maneuver.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::mission_risk_aggregation::{ExactMass, MissionRiskBudgetStatus, MissionRiskEnvelope};
use crate::simultaneous_defensive_selection::SimultaneousDefensivePortfolioReport;

/// Frozen source and policy contract for one scenario × portfolio mission envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortfolioScenarioRiskContract {
    /// Frozen epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Frozen manoeuvre portfolio identity.
    pub portfolio_id: [u8; 32],
    /// Expected mission-support manifest digest for this post-action portfolio.
    pub manifest_digest: [u8; 32],
    /// Target probability measure shared by all portfolios in this scenario.
    pub target_measure_digest: [u8; 32],
    /// Density coordinate/frame/epoch space used by this scenario.
    pub density_space_digest: [u8; 32],
    /// Post-action mission event-inventory digest for this portfolio.
    pub event_inventory_digest: [u8; 32],
    /// Digest of the complete canonical MissionRiskEnvelope contents.
    pub risk_envelope_digest: [u8; 32],
    /// Digest of the complete canonical MissionRiskEnvelope contents.
    pub risk_envelope_digest: [u8; 32],
    /// Exact total integer mass expected for the finite target support.
    pub total_mass_ticks: u64,
    /// Exact rational mission-wide collision-risk budget for this scenario.
    pub risk_budget: ExactMass,
}

/// One supplied exact mission-risk envelope for a frozen scenario/portfolio cell.
#[derive(Debug, Clone, PartialEq)]
pub struct PortfolioScenarioRiskEvidence {
    /// Frozen epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Frozen manoeuvre portfolio identity.
    pub portfolio_id: [u8; 32],
    /// Exact dependence-aware pathwise mission risk output.
    pub envelope: MissionRiskEnvelope,
}

/// Exact hard-gate result for one scenario × portfolio cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortfolioScenarioRiskCheck {
    /// Scenario identity.
    pub scenario_id: [u8; 32],
    /// Portfolio identity.
    pub portfolio_id: [u8; 32],
    /// Exact pathwise mission union mass.
    pub mission_union_mass: ExactMass,
    /// Exact frozen mission-risk budget.
    pub mission_risk_budget: ExactMass,
    /// Recomputed comparison; never trusted from the envelope's status field.
    pub within_declared_finite_support_budget: bool,
}

/// Worst-case regret interval recomputed against only the eligible portfolio set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EligiblePortfolioRegretInterval {
    /// Stable portfolio identity.
    pub portfolio_id: [u8; 32],
    /// Lower endpoint for worst-case regret against eligible alternatives.
    pub lower_worst_case_regret_ticks: f64,
    /// Upper endpoint for worst-case regret against eligible alternatives.
    pub upper_worst_case_regret_ticks: f64,
    /// Scenario attaining the largest lower endpoint.
    pub lower_witness_scenario_id: [u8; 32],
    /// Scenario attaining the largest upper endpoint.
    pub upper_witness_scenario_id: [u8; 32],
}

/// Complete scenario-wide mission-risk feasibility and eligible-regret report.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioMissionRiskGateReport {
    /// Total family-wise confidence error allocated before sampling.
    pub total_family_alpha: f64,
    /// Per-scenario family-wise error allocation.
    pub scenario_family_alpha: f64,
    /// Per-pair error allocation across the full original universe.
    pub contrast_alpha: f64,
    /// Number of frozen epistemic scenarios.
    pub scenario_count: usize,
    /// Number of frozen candidate portfolios.
    pub portfolio_count: usize,
    /// Exact consequence unit / sign convention / tick-scale identity.
    pub consequence_unit_id: [u8; 32],
    /// Every validated scenario × portfolio finite-support risk result.
    pub scenario_portfolio_risk_checks: Vec<PortfolioScenarioRiskCheck>,
    /// Portfolios within mission-risk budget in every required scenario.
    pub eligible_portfolio_ids: Vec<[u8; 32]>,
    /// Portfolios exceeding at least one scenario-specific mission-risk budget.
    pub ineligible_portfolio_ids: Vec<[u8; 32]>,
    /// Regret intervals calculated only against the eligible alternative set.
    pub eligible_portfolio_regret_intervals: Vec<EligiblePortfolioRegretInterval>,
    /// Unique lowest-regret eligible portfolio, only when strict separation holds.
    pub unique_best_eligible_portfolio_id: Option<[u8; 32]>,
    /// Eligible portfolios not proven to have higher regret than another.
    pub minimax_regret_undominated_eligible_portfolio_ids: Vec<[u8; 32]>,
    /// Always false. This gate has no operational authority.
    pub authority_granted: bool,
}

/// Errors validating a complete scenario × portfolio mission-risk evidence matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioMissionRiskGateError {
    /// The total family-wise alpha is outside (0,1).
    InvalidFamilyAlpha,
    /// Scenario or portfolio identity universe is empty, duplicated, invalid or too large.
    InvalidScenarioOrPortfolioUniverse,
    /// The consequence-unit identity is zero.
    InvalidConsequenceUnitIdentity,
    /// The risk-contract matrix is incomplete, duplicated or names an unexpected cell.
    InvalidRiskContractMatrix,
    /// Scenario target, density-space or risk-budget identities disagree across portfolios.
    ScenarioContractMismatch,
    /// A contract has an invalid digest, total mass or rational risk budget.
    InvalidRiskContract,
    /// One or more exact risk envelopes are missing, duplicated or unexpected.
    IncompleteRiskEvidenceMatrix,
    /// A risk envelope's source identities do not match the frozen contract.
    RiskEnvelopeIdentityMismatch,
    /// A risk envelope has an empty or malformed finite support/event inventory.
    InvalidRiskEnvelopeShape,
    /// A rational mass has zero denominator or lies outside [0,1].
    InvalidExactMass,
    /// Union and no-event exact masses do not sum to one.
    UnionComplementMismatch,
    /// The exact union lies outside the reported Fréchet bounds.
    FrechetBoundsMismatch,
    /// The supplied budget status disagrees with exact rational comparison.
    RiskBudgetStatusMismatch,
    /// An envelope claims operational authority.
    UnexpectedAuthorityGrant,
    /// Scenario consequence report matrix is incomplete or duplicated.
    IncompleteScenarioConsequenceReports,
    /// Scenario report identity, alpha or consequence unit differs from frozen contract.
    ConsequenceReportContractMismatch,
    /// One scenario report does not contain the complete pairwise portfolio family.
    IncompleteConsequencePairFamily,
    /// A pair interval has an invalid identity, alpha, sample count or endpoint.
    InvalidConsequencePairInterval,
    /// A rational comparison or regret calculation failed numerically.
    NumericalFailure,
}

impl core::fmt::Display for ScenarioMissionRiskGateError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidFamilyAlpha => "total family-wise alpha must lie in (0,1)",
            Self::InvalidScenarioOrPortfolioUniverse => "scenario or portfolio identity universe is invalid",
            Self::InvalidConsequenceUnitIdentity => "consequence unit identity is zero",
            Self::InvalidRiskContractMatrix => "scenario/portfolio risk contract matrix is incomplete or duplicated",
            Self::ScenarioContractMismatch => "target measure, density space or risk budget varies within a scenario",
            Self::InvalidRiskContract => "scenario/portfolio risk contract is invalid",
            Self::IncompleteRiskEvidenceMatrix => "mission-risk envelope matrix is incomplete or duplicated",
            Self::RiskEnvelopeIdentityMismatch => "mission-risk envelope does not match its frozen contract",
            Self::RiskEnvelopeDigestMismatch => "mission-risk envelope content digest differs from its frozen contract",
            Self::InvalidRiskEnvelopeShape => "mission-risk envelope shape is invalid",
            Self::InvalidExactMass => "exact probability mass is invalid",
            Self::UnionComplementMismatch => "mission union and no-event masses do not sum exactly to one",
            Self::FrechetBoundsMismatch => "mission union lies outside its reported Frechet bounds",
            Self::RiskBudgetStatusMismatch => "mission-risk budget status disagrees with exact mass comparison",
            Self::UnexpectedAuthorityGrant => "mission-risk envelope unexpectedly grants authority",
            Self::IncompleteScenarioConsequenceReports => "scenario consequence report universe is incomplete",
            Self::ConsequenceReportContractMismatch => "scenario consequence report differs from frozen family contract",
            Self::IncompleteConsequencePairFamily => "scenario consequence report omits a required pair",
            Self::InvalidConsequencePairInterval => "scenario pair interval is invalid",
            Self::NumericalFailure => "scenario mission-risk gate calculation failed numerically",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ScenarioMissionRiskGateError {}

/// Frozen cross-scenario mission-risk gate and consequence family.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioMissionRiskGate {
    total_family_alpha: f64,
    scenario_family_alpha: f64,
    contrast_alpha: f64,
    expected_scenario_ids: Vec<[u8; 32]>,
    portfolio_ids: Vec<[u8; 32]>,
    consequence_unit_id: [u8; 32],
    contracts: BTreeMap<([u8; 32], [u8; 32]), PortfolioScenarioRiskContract>,
}

impl ScenarioMissionRiskGate {
    /// Freeze scenario IDs, portfolio IDs, the consequence scale and every
    /// scenario × portfolio mission-risk source/budget contract.
    pub fn new(
        total_family_alpha: f64,
        mut expected_scenario_ids: Vec<[u8; 32]>,
        mut portfolio_ids: Vec<[u8; 32]>,
        consequence_unit_id: [u8; 32],
        contracts: Vec<PortfolioScenarioRiskContract>,
    ) -> Result<Self, ScenarioMissionRiskGateError> {
        if !total_family_alpha.is_finite()
            || total_family_alpha <= 0.0
            || total_family_alpha >= 1.0
        {
            return Err(ScenarioMissionRiskGateError::InvalidFamilyAlpha);
        }
        if consequence_unit_id.iter().all(|b| *b == 0) {
            return Err(ScenarioMissionRiskGateError::InvalidConsequenceUnitIdentity);
        }
        if expected_scenario_ids.is_empty()
            || expected_scenario_ids.len() > 1_024
            || expected_scenario_ids.iter().any(|id| id.iter().all(|b| *b == 0))
            || portfolio_ids.len() < 2
            || portfolio_ids.len() > 32
            || portfolio_ids.iter().any(|id| id.iter().all(|b| *b == 0))
        {
            return Err(ScenarioMissionRiskGateError::InvalidScenarioOrPortfolioUniverse);
        }
        expected_scenario_ids.sort_unstable();
        portfolio_ids.sort_unstable();
        if expected_scenario_ids.windows(2).any(|w| w[0] == w[1])
            || portfolio_ids.windows(2).any(|w| w[0] == w[1])
        {
            return Err(ScenarioMissionRiskGateError::InvalidScenarioOrPortfolioUniverse);
        }

        let expected_contract_count = expected_scenario_ids
            .len()
            .checked_mul(portfolio_ids.len())
            .ok_or(ScenarioMissionRiskGateError::NumericalFailure)?;
        if contracts.len() != expected_contract_count {
            return Err(ScenarioMissionRiskGateError::InvalidRiskContractMatrix);
        }

        let mut contract_map = BTreeMap::new();
        for contract in contracts {
            if !expected_scenario_ids.contains(&contract.scenario_id)
                || !portfolio_ids.contains(&contract.portfolio_id)
                || contract.scenario_id.iter().all(|b| *b == 0)
                || contract.portfolio_id.iter().all(|b| *b == 0)
            {
                return Err(ScenarioMissionRiskGateError::InvalidRiskContractMatrix);
            }
            if contract.manifest_digest.iter().all(|b| *b == 0)
                || contract.target_measure_digest.iter().all(|b| *b == 0)
                || contract.density_space_digest.iter().all(|b| *b == 0)
                || contract.event_inventory_digest.iter().all(|b| *b == 0)
                || contract.risk_envelope_digest.iter().all(|b| *b == 0)
                || contract.total_mass_ticks == 0
                || !valid_probability_mass(contract.risk_budget)
            {
                return Err(ScenarioMissionRiskGateError::InvalidRiskContract);
            }
            let key = (contract.scenario_id, contract.portfolio_id);
            if contract_map.insert(key, contract).is_some() {
                return Err(ScenarioMissionRiskGateError::InvalidRiskContractMatrix);
            }
        }
        if contract_map.len() != expected_contract_count {
            return Err(ScenarioMissionRiskGateError::InvalidRiskContractMatrix);
        }

        for scenario_id in &expected_scenario_ids {
            let first = contract_map
                .get(&(*scenario_id, portfolio_ids[0]))
                .ok_or(ScenarioMissionRiskGateError::InvalidRiskContractMatrix)?;
            for portfolio_id in portfolio_ids.iter().skip(1) {
                let contract = contract_map
                    .get(&(*scenario_id, *portfolio_id))
                    .ok_or(ScenarioMissionRiskGateError::InvalidRiskContractMatrix)?;
                if contract.target_measure_digest != first.target_measure_digest
                    || contract.density_space_digest != first.density_space_digest
                    || compare_exact_mass(contract.risk_budget, first.risk_budget)?
                        != Ordering::Equal
                {
                    return Err(ScenarioMissionRiskGateError::ScenarioContractMismatch);
                }
            }
        }

        let scenario_family_alpha = total_family_alpha / expected_scenario_ids.len() as f64;
        let pair_count = portfolio_ids
            .len()
            .checked_mul(portfolio_ids.len() - 1)
            .and_then(|n| n.checked_div(2))
            .ok_or(ScenarioMissionRiskGateError::NumericalFailure)?;
        let contrast_alpha = scenario_family_alpha / pair_count as f64;
        if !scenario_family_alpha.is_finite()
            || scenario_family_alpha <= 0.0
            || !contrast_alpha.is_finite()
            || contrast_alpha <= 0.0
        {
            return Err(ScenarioMissionRiskGateError::NumericalFailure);
        }

        Ok(Self {
            total_family_alpha,
            scenario_family_alpha,
            contrast_alpha,
            expected_scenario_ids,
            portfolio_ids,
            consequence_unit_id,
            contracts: contract_map,
        })
    }

    /// Exact target-measure family error allocation for each scenario.
    pub fn scenario_family_alpha(&self) -> f64 {
        self.scenario_family_alpha
    }

    /// Exact pairwise error allocation; the denominator remains the full
    /// originally declared K(K-1)/2 family, even after risk filtering.
    pub fn contrast_alpha(&self) -> f64 {
        self.contrast_alpha
    }

    /// Validate all exact scenario/portfolio mission-risk envelopes, filter
    /// ineligible portfolios, then recompute minimax regret among the remaining
    /// eligible alternatives using the original simultaneous pair intervals.
    ///
    /// The full original family is retained for the confidence guarantee.
    /// Every scenario and every original pair is required before risk filtering.
    pub fn evaluate(
        &self,
        risk_evidence: &[PortfolioScenarioRiskEvidence],
        consequence_reports: &[SimultaneousDefensivePortfolioReport],
    ) -> Result<ScenarioMissionRiskGateReport, ScenarioMissionRiskGateError> {
        let expected_cell_count = self
            .expected_scenario_ids
            .len()
            .checked_mul(self.portfolio_ids.len())
            .ok_or(ScenarioMissionRiskGateError::NumericalFailure)?;
        if risk_evidence.len() != expected_cell_count {
            return Err(ScenarioMissionRiskGateError::IncompleteRiskEvidenceMatrix);
        }

        let mut risk_by_cell = BTreeMap::new();
        for evidence in risk_evidence {
            let key = (evidence.scenario_id, evidence.portfolio_id);
            if risk_by_cell.insert(key, &evidence.envelope).is_some() {
                return Err(ScenarioMissionRiskGateError::IncompleteRiskEvidenceMatrix);
            }
        }
        if risk_by_cell.len() != expected_cell_count {
            return Err(ScenarioMissionRiskGateError::IncompleteRiskEvidenceMatrix);
        }

        let mut risk_checks = Vec::with_capacity(expected_cell_count);
        let mut ineligible_scenarios_by_portfolio =
            BTreeMap::<[u8; 32], Vec<[u8; 32]>>::new();
        for scenario_id in &self.expected_scenario_ids {
            for portfolio_id in &self.portfolio_ids {
                let contract = self
                    .contracts
                    .get(&(*scenario_id, *portfolio_id))
                    .ok_or(ScenarioMissionRiskGateError::InvalidRiskContractMatrix)?;
                let envelope = risk_by_cell
                    .get(&(*scenario_id, *portfolio_id))
                    .copied()
                    .ok_or(ScenarioMissionRiskGateError::IncompleteRiskEvidenceMatrix)?;
                let within_budget = validate_envelope(contract, envelope)?;
                if !within_budget {
                    ineligible_scenarios_by_portfolio
                        .entry(*portfolio_id)
                        .or_default()
                        .push(*scenario_id);
                }
                risk_checks.push(PortfolioScenarioRiskCheck {
                    scenario_id: *scenario_id,
                    portfolio_id: *portfolio_id,
                    mission_union_mass: envelope.pathwise_union_mass,
                    mission_risk_budget: envelope.risk_budget,
                    within_declared_finite_support_budget: within_budget,
                });
            }
        }

        let eligible_portfolio_ids = self
            .portfolio_ids
            .iter()
            .filter(|id| !ineligible_scenarios_by_portfolio.contains_key(*id))
            .copied()
            .collect::<Vec<_>>();
        let ineligible_portfolio_ids = self
            .portfolio_ids
            .iter()
            .filter(|id| ineligible_scenarios_by_portfolio.contains_key(*id))
            .copied()
            .collect::<Vec<_>>();

        let scenario_reports = self.validate_consequence_reports(consequence_reports)?;
        let eligible_portfolio_regret_intervals =
            self.regret_intervals_for_eligible(&eligible_portfolio_ids, &scenario_reports)?;
        let mut unique_best = Vec::new();
        let mut undominated = Vec::new();
        for candidate in &eligible_portfolio_regret_intervals {
            let beats_all = eligible_portfolio_regret_intervals.iter().all(|opponent| {
                candidate.portfolio_id == opponent.portfolio_id
                    || candidate.upper_worst_case_regret_ticks
                        < opponent.lower_worst_case_regret_ticks
            });
            let is_dominated = eligible_portfolio_regret_intervals.iter().any(|opponent| {
                candidate.portfolio_id != opponent.portfolio_id
                    && opponent.upper_worst_case_regret_ticks
                        < candidate.lower_worst_case_regret_ticks
            });
            if beats_all {
                unique_best.push(candidate.portfolio_id);
            }
            if !is_dominated {
                undominated.push(candidate.portfolio_id);
            }
        }

        Ok(ScenarioMissionRiskGateReport {
            total_family_alpha: self.total_family_alpha,
            scenario_family_alpha: self.scenario_family_alpha,
            contrast_alpha: self.contrast_alpha,
            scenario_count: self.expected_scenario_ids.len(),
            portfolio_count: self.portfolio_ids.len(),
            consequence_unit_id: self.consequence_unit_id,
            scenario_portfolio_risk_checks: risk_checks,
            eligible_portfolio_ids,
            ineligible_portfolio_ids,
            ineligible_scenarios_by_portfolio,
            eligible_portfolio_regret_intervals,
            unique_best_eligible_portfolio_id: if unique_best.len() == 1 {
                Some(unique_best[0])
            } else {
                None
            },
            minimax_regret_undominated_eligible_portfolio_ids: undominated,
            authority_granted: false,
        })
    }

    fn validate_consequence_reports<'a>(
        &self,
        reports: &'a [SimultaneousDefensivePortfolioReport],
    ) -> Result<BTreeMap<[u8; 32], &'a SimultaneousDefensivePortfolioReport>, ScenarioMissionRiskGateError>
    {
        if reports.len() != self.expected_scenario_ids.len() {
            return Err(ScenarioMissionRiskGateError::IncompleteScenarioConsequenceReports);
        }
        let portfolio_count = self.portfolio_ids.len();
        let pair_count = portfolio_count
            .checked_mul(portfolio_count - 1)
            .and_then(|n| n.checked_div(2))
            .ok_or(ScenarioMissionRiskGateError::NumericalFailure)?;
        let mut by_scenario = BTreeMap::new();
        for report in reports {
            if by_scenario.insert(report.scenario_id, report).is_some() {
                return Err(ScenarioMissionRiskGateError::IncompleteScenarioConsequenceReports);
            }
        }

        for scenario_id in &self.expected_scenario_ids {
            let report = by_scenario
                .get(scenario_id)
                .copied()
                .ok_or(ScenarioMissionRiskGateError::IncompleteScenarioConsequenceReports)?;
            if report.consequence_unit_id != self.consequence_unit_id
                || report.portfolio_count != portfolio_count
                || report.contrast_count != pair_count
                || report.pairwise_intervals.len() != pair_count
                || !close_enough(report.family_alpha, self.scenario_family_alpha)
                || !close_enough(report.contrast_alpha, self.contrast_alpha)
            {
                return Err(ScenarioMissionRiskGateError::ConsequenceReportContractMismatch);
            }
            let scenario_target_measure = self.contracts
                .get(&(*scenario_id, self.portfolio_ids[0]))
                .ok_or(ScenarioMissionRiskGateError::InvalidRiskContractMatrix)?
                .target_measure_digest;
            let scenario_density_space = self.contracts
                .get(&(*scenario_id, self.portfolio_ids[0]))
                .ok_or(ScenarioMissionRiskGateError::InvalidRiskContractMatrix)?
                .density_space_digest;
            if report.target_measure_id != scenario_target_measure
                || report.density_space_id != scenario_density_space
                || report.configuration_digest.iter().all(|byte| *byte == 0)
                || !report.effective_defensive_fraction.is_finite()
                || report.effective_defensive_fraction <= 0.0
                || report.effective_defensive_fraction > 1.0
            {
                return Err(ScenarioMissionRiskGateError::ConsequenceReportContractMismatch);
            }

            let mut intervals = BTreeMap::new();
            for interval in &report.pairwise_intervals {
                if interval.left_portfolio_id >= interval.right_portfolio_id
                    || interval.sample_count != report.draw_count
                    || !close_enough(interval.contrast_alpha, self.contrast_alpha)
                    || !interval.lower_expected_difference_ticks.is_finite()
                    || !interval.upper_expected_difference_ticks.is_finite()
                    || !interval.lower_normalized_expected_difference.is_finite()
                    || !interval.upper_normalized_expected_difference.is_finite()
                    || interval.lower_normalized_expected_difference < 0.0
                    || interval.upper_normalized_expected_difference > 1.0
                    || interval.lower_normalized_expected_difference
                        > interval.upper_normalized_expected_difference
                    || interval.lower_expected_difference_ticks
                        > interval.upper_expected_difference_ticks
                {
                    return Err(ScenarioMissionRiskGateError::InvalidConsequencePairInterval);
                }
                if intervals
                    .insert((interval.left_portfolio_id, interval.right_portfolio_id), interval)
                    .is_some()
                {
                    return Err(ScenarioMissionRiskGateError::InvalidConsequencePairInterval);
                }
            }
            for (left_index, left_id) in self.portfolio_ids.iter().enumerate() {
                for right_id in self.portfolio_ids.iter().skip(left_index + 1) {
                    if !intervals.contains_key(&(*left_id, *right_id)) {
                        return Err(ScenarioMissionRiskGateError::IncompleteConsequencePairFamily);
                    }
                }
            }
        }
        Ok(by_scenario)
    }

    fn regret_intervals_for_eligible(
        &self,
        eligible_portfolio_ids: &[[u8; 32]],
        reports: &BTreeMap<[u8; 32], &SimultaneousDefensivePortfolioReport>,
    ) -> Result<Vec<EligiblePortfolioRegretInterval>, ScenarioMissionRiskGateError> {
        let Some(first_scenario) = self.expected_scenario_ids.first().copied() else {
            return Err(ScenarioMissionRiskGateError::NumericalFailure);
        };
        let mut intervals = Vec::with_capacity(eligible_portfolio_ids.len());
        for candidate in eligible_portfolio_ids {
            let mut lower_regret = 0.0_f64;
            let mut upper_regret = 0.0_f64;
            let mut lower_witness = first_scenario;
            let mut upper_witness = first_scenario;

            for scenario_id in &self.expected_scenario_ids {
                let report = reports
                    .get(scenario_id)
                    .copied()
                    .ok_or(ScenarioMissionRiskGateError::IncompleteScenarioConsequenceReports)?;
                for opponent in eligible_portfolio_ids {
                    if candidate == opponent {
                        continue;
                    }
                    let (left_id, right_id, candidate_is_left) = if candidate < opponent {
                        (*candidate, *opponent, true)
                    } else {
                        (*opponent, *candidate, false)
                    };
                    let interval = report
                        .pairwise_intervals
                        .iter()
                        .find(|interval| {
                            interval.left_portfolio_id == left_id
                                && interval.right_portfolio_id == right_id
                        })
                        .ok_or(ScenarioMissionRiskGateError::IncompleteConsequencePairFamily)?;
                    let (lower, upper) = if candidate_is_left {
                        (
                            interval.lower_expected_difference_ticks,
                            interval.upper_expected_difference_ticks,
                        )
                    } else {
                        (
                            -interval.upper_expected_difference_ticks,
                            -interval.lower_expected_difference_ticks,
                        )
                    };
                    if lower > lower_regret {
                        lower_regret = lower;
                        lower_witness = *scenario_id;
                    }
                    if upper > upper_regret {
                        upper_regret = upper;
                        upper_witness = *scenario_id;
                    }
                }
            }

            if !lower_regret.is_finite() || !upper_regret.is_finite() || lower_regret > upper_regret {
                return Err(ScenarioMissionRiskGateError::NumericalFailure);
            }
            intervals.push(EligiblePortfolioRegretInterval {
                portfolio_id: *candidate,
                lower_worst_case_regret_ticks: lower_regret.max(0.0),
                upper_worst_case_regret_ticks: upper_regret.max(0.0),
                lower_witness_scenario_id: lower_witness,
                upper_witness_scenario_id: upper_witness,
            });
        }
        Ok(intervals)
    }
}


/// Compute a canonical, domain-separated content digest for a mission-risk envelope.
///
/// The digest binds all exact risk summaries, event identifiers and the
/// diagnostic independence approximation. It is a content identity, not an
/// issuer signature or proof that the underlying source universe is complete.
pub fn mission_risk_envelope_digest(envelope: &MissionRiskEnvelope) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.mission-risk-envelope.v0.1\0");
    hasher.update(envelope.manifest_digest);
    hasher.update(envelope.target_measure_digest);
    hasher.update(envelope.event_inventory_digest);
    hasher.update(envelope.total_mass_ticks.to_be_bytes());
    hasher.update((envelope.event_count as u64).to_be_bytes());
    hasher.update((envelope.trajectory_atom_count as u64).to_be_bytes());

    hasher.update((envelope.event_marginals.len() as u64).to_be_bytes());
    for marginal in &envelope.event_marginals {
        hash_string(&mut hasher, &marginal.event_id);
        hash_exact_mass(&mut hasher, marginal.mass);
    }

    hasher.update((envelope.pairwise_intersections.len() as u64).to_be_bytes());
    for pair in &envelope.pairwise_intersections {
        hash_string(&mut hasher, &pair.left_event_id);
        hash_string(&mut hasher, &pair.right_event_id);
        hash_exact_mass(&mut hasher, pair.mass);
    }

    hasher.update((envelope.joint_signatures.len() as u64).to_be_bytes());
    for signature in &envelope.joint_signatures {
        hasher.update((signature.contacting_event_ids.len() as u64).to_be_bytes());
        for event_id in &signature.contacting_event_ids {
            hash_string(&mut hasher, event_id);
        }
        hash_exact_mass(&mut hasher, signature.mass);
    }

    hash_exact_mass(&mut hasher, envelope.pathwise_union_mass);
    hash_exact_mass(&mut hasher, envelope.no_event_mass);
    hasher.update(envelope.marginal_sum_ticks.to_be_bytes());
    hash_exact_mass(&mut hasher, envelope.frechet_union_bounds.lower_bound);
    hash_exact_mass(&mut hasher, envelope.frechet_union_bounds.upper_bound);
    let independence_bits = if envelope.independence_union_approximation == 0.0 {
        0.0_f64.to_bits()
    } else {
        envelope.independence_union_approximation.to_bits()
    };
    hasher.update(independence_bits.to_be_bytes());
    hash_exact_mass(&mut hasher, envelope.risk_budget);
    hasher.update([match envelope.budget_status {
        MissionRiskBudgetStatus::WithinDeclaredFiniteSupportBudget => 0,
        MissionRiskBudgetStatus::ExceedsDeclaredFiniteSupportBudget => 1,
    }]);
    hasher.update([u8::from(envelope.authority_granted)]);
    hasher.finalize().into()
}

fn hash_string(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}

fn hash_exact_mass(hasher: &mut Sha256, mass: ExactMass) {
    hasher.update(mass.numerator.to_be_bytes());
    hasher.update(mass.denominator.to_be_bytes());
}


/// Compute a canonical, domain-separated content digest for a mission-risk envelope.
///
/// The digest binds exact risk summaries, event identifiers and the diagnostic
/// independence approximation. It is a content identity, not an issuer signature
/// or proof that the underlying source universe is complete.
pub fn mission_risk_envelope_digest(envelope: &MissionRiskEnvelope) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.mission-risk-envelope.v0.1\0");
    hasher.update(envelope.manifest_digest);
    hasher.update(envelope.target_measure_digest);
    hasher.update(envelope.event_inventory_digest);
    hasher.update(envelope.total_mass_ticks.to_be_bytes());
    hasher.update((envelope.event_count as u64).to_be_bytes());
    hasher.update((envelope.trajectory_atom_count as u64).to_be_bytes());

    hasher.update((envelope.event_marginals.len() as u64).to_be_bytes());
    for marginal in &envelope.event_marginals {
        hash_string(&mut hasher, &marginal.event_id);
        hash_exact_mass(&mut hasher, marginal.mass);
    }

    hasher.update((envelope.pairwise_intersections.len() as u64).to_be_bytes());
    for pair in &envelope.pairwise_intersections {
        hash_string(&mut hasher, &pair.left_event_id);
        hash_string(&mut hasher, &pair.right_event_id);
        hash_exact_mass(&mut hasher, pair.mass);
    }

    hasher.update((envelope.joint_signatures.len() as u64).to_be_bytes());
    for signature in &envelope.joint_signatures {
        hasher.update((signature.contacting_event_ids.len() as u64).to_be_bytes());
        for event_id in &signature.contacting_event_ids {
            hash_string(&mut hasher, event_id);
        }
        hash_exact_mass(&mut hasher, signature.mass);
    }

    hash_exact_mass(&mut hasher, envelope.pathwise_union_mass);
    hash_exact_mass(&mut hasher, envelope.no_event_mass);
    hasher.update(envelope.marginal_sum_ticks.to_be_bytes());
    hash_exact_mass(&mut hasher, envelope.frechet_union_bounds.lower_bound);
    hash_exact_mass(&mut hasher, envelope.frechet_union_bounds.upper_bound);
    let independence_bits = if envelope.independence_union_approximation == 0.0 {
        0.0_f64.to_bits()
    } else {
        envelope.independence_union_approximation.to_bits()
    };
    hasher.update(independence_bits.to_be_bytes());
    hash_exact_mass(&mut hasher, envelope.risk_budget);
    hasher.update([match envelope.budget_status {
        MissionRiskBudgetStatus::WithinDeclaredFiniteSupportBudget => 0,
        MissionRiskBudgetStatus::ExceedsDeclaredFiniteSupportBudget => 1,
    }]);
    hasher.update([u8::from(envelope.authority_granted)]);
    hasher.finalize().into()
}

fn hash_string(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}

fn hash_exact_mass(hasher: &mut Sha256, mass: ExactMass) {
    hasher.update(mass.numerator.to_be_bytes());
    hasher.update(mass.denominator.to_be_bytes());
}

fn validate_envelope(
    contract: &PortfolioScenarioRiskContract,
    envelope: &MissionRiskEnvelope,
) -> Result<bool, ScenarioMissionRiskGateError> {
    if envelope.manifest_digest != contract.manifest_digest
        || envelope.target_measure_digest != contract.target_measure_digest
        || envelope.event_inventory_digest != contract.event_inventory_digest
        || envelope.total_mass_ticks != contract.total_mass_ticks
    {
        return Err(ScenarioMissionRiskGateError::RiskEnvelopeIdentityMismatch);
    }
    if mission_risk_envelope_digest(envelope) != contract.risk_envelope_digest {
        return Err(ScenarioMissionRiskGateError::RiskEnvelopeDigestMismatch);
    }
    if mission_risk_envelope_digest(envelope) != contract.risk_envelope_digest {
        return Err(ScenarioMissionRiskGateError::RiskEnvelopeDigestMismatch);
    }
    if envelope.authority_granted {
        return Err(ScenarioMissionRiskGateError::UnexpectedAuthorityGrant);
    }
    if envelope.event_count == 0
        || envelope.event_count > 32
        || envelope.trajectory_atom_count == 0
        || envelope.event_marginals.len() != envelope.event_count
        || envelope.pairwise_intersections.len()
            != envelope.event_count * envelope.event_count.saturating_sub(1) / 2
        || envelope.joint_signatures.is_empty()
        || !envelope.independence_union_approximation.is_finite()
        || !(0.0..=1.0).contains(&envelope.independence_union_approximation)
    {
        return Err(ScenarioMissionRiskGateError::InvalidRiskEnvelopeShape);
    }

    for mass in [
        envelope.pathwise_union_mass,
        envelope.no_event_mass,
        envelope.frechet_union_bounds.lower_bound,
        envelope.frechet_union_bounds.upper_bound,
        envelope.risk_budget,
    ] {
        if !valid_probability_mass(mass) {
            return Err(ScenarioMissionRiskGateError::InvalidExactMass);
        }
    }

    // Every event-derived mass must be an exact reduction of the same finite
    // support denominator. Convert it back to integer ticks to independently
    // reconcile the summary rather than adding unrelated reduced fractions.
    let union_ticks = exact_mass_ticks(envelope.pathwise_union_mass, envelope.total_mass_ticks)?;
    let no_event_ticks = exact_mass_ticks(envelope.no_event_mass, envelope.total_mass_ticks)?;
    let frechet_lower_ticks =
        exact_mass_ticks(envelope.frechet_union_bounds.lower_bound, envelope.total_mass_ticks)?;
    let frechet_upper_ticks =
        exact_mass_ticks(envelope.frechet_union_bounds.upper_bound, envelope.total_mass_ticks)?;
    if union_ticks.checked_add(no_event_ticks) != Some(envelope.total_mass_ticks) {
        return Err(ScenarioMissionRiskGateError::UnionComplementMismatch);
    }

    let mut event_ids = Vec::with_capacity(envelope.event_count);
    let mut event_ticks = Vec::with_capacity(envelope.event_count);
    let mut marginal_sum = 0_u128;
    for marginal in &envelope.event_marginals {
        if marginal.event_id.trim().is_empty()
            || event_ids
                .last()
                .is_some_and(|previous: &String| previous >= &marginal.event_id)
        {
            return Err(ScenarioMissionRiskGateError::InvalidRiskEnvelopeShape);
        }
        let ticks = exact_mass_ticks(marginal.mass, envelope.total_mass_ticks)?;
        marginal_sum = marginal_sum
            .checked_add(u128::from(ticks))
            .ok_or(ScenarioMissionRiskGateError::NumericalFailure)?;
        event_ids.push(marginal.event_id.clone());
        event_ticks.push(ticks);
    }
    if marginal_sum != envelope.marginal_sum_ticks {
        return Err(ScenarioMissionRiskGateError::InvalidRiskEnvelopeShape);
    }

    let expected_lower = *event_ticks.iter().max().unwrap_or(&0);
    let expected_upper = marginal_sum.min(u128::from(envelope.total_mass_ticks)) as u64;
    if frechet_lower_ticks != expected_lower || frechet_upper_ticks != expected_upper {
        return Err(ScenarioMissionRiskGateError::FrechetBoundsMismatch);
    }
    if union_ticks < frechet_lower_ticks || union_ticks > frechet_upper_ticks {
        return Err(ScenarioMissionRiskGateError::FrechetBoundsMismatch);
    }

    let mut pair_index = 0;
    for left in 0..event_ids.len() {
        for right in (left + 1)..event_ids.len() {
            let pair = &envelope.pairwise_intersections[pair_index];
            if pair.left_event_id != event_ids[left]
                || pair.right_event_id != event_ids[right]
            {
                return Err(ScenarioMissionRiskGateError::InvalidRiskEnvelopeShape);
            }
            let pair_ticks = exact_mass_ticks(pair.mass, envelope.total_mass_ticks)?;
            if pair_ticks > event_ticks[left] || pair_ticks > event_ticks[right] {
                return Err(ScenarioMissionRiskGateError::InvalidRiskEnvelopeShape);
            }
            pair_index += 1;
        }
    }

    let mut signature_total = 0_u64;
    let mut signature_union = 0_u64;
    let mut signature_no_event = None;
    let mut previous_signature: Option<&[String]> = None;
    for signature in &envelope.joint_signatures {
        if signature
            .contacting_event_ids
            .windows(2)
            .any(|window| window[0] >= window[1])
            || signature
                .contacting_event_ids
                .iter()
                .any(|event_id| event_ids.binary_search(event_id).is_err())
            || previous_signature.is_some_and(|previous| {
                previous >= signature.contacting_event_ids.as_slice()
            })
        {
            return Err(ScenarioMissionRiskGateError::InvalidRiskEnvelopeShape);
        }
        let ticks = exact_mass_ticks(signature.mass, envelope.total_mass_ticks)?;
        signature_total = signature_total
            .checked_add(ticks)
            .ok_or(ScenarioMissionRiskGateError::NumericalFailure)?;
        if signature.contacting_event_ids.is_empty() {
            if signature_no_event.replace(ticks).is_some() {
                return Err(ScenarioMissionRiskGateError::InvalidRiskEnvelopeShape);
            }
        } else {
            signature_union = signature_union
                .checked_add(ticks)
                .ok_or(ScenarioMissionRiskGateError::NumericalFailure)?;
        }
        previous_signature = Some(&signature.contacting_event_ids);
    }
    if signature_total != envelope.total_mass_ticks
        || signature_union != union_ticks
        || signature_no_event.unwrap_or(0) != no_event_ticks
    {
        return Err(ScenarioMissionRiskGateError::InvalidRiskEnvelopeShape);
    }

    let within_budget =
        compare_exact_mass(envelope.pathwise_union_mass, contract.risk_budget)? != Ordering::Greater;
    let expected_status = if within_budget {
        MissionRiskBudgetStatus::WithinDeclaredFiniteSupportBudget
    } else {
        MissionRiskBudgetStatus::ExceedsDeclaredFiniteSupportBudget
    };
    if envelope.budget_status != expected_status
        || compare_exact_mass(envelope.risk_budget, contract.risk_budget)? != Ordering::Equal
    {
        return Err(ScenarioMissionRiskGateError::RiskBudgetStatusMismatch);
    }
    Ok(within_budget)
}

fn exact_mass_ticks(
    mass: ExactMass,
    total_mass_ticks: u64,
) -> Result<u64, ScenarioMissionRiskGateError> {
    if !valid_probability_mass(mass)
        || total_mass_ticks == 0
        || total_mass_ticks % mass.denominator != 0
    {
        return Err(ScenarioMissionRiskGateError::InvalidExactMass);
    }
    let ticks = mass
        .numerator
        .checked_mul(total_mass_ticks / mass.denominator)
        .ok_or(ScenarioMissionRiskGateError::NumericalFailure)?;
    if ticks > total_mass_ticks {
        return Err(ScenarioMissionRiskGateError::InvalidExactMass);
    }
    Ok(ticks)
}

fn valid_probability_mass(mass: ExactMass) -> bool {
    mass.denominator != 0 && mass.numerator <= mass.denominator
}

fn compare_exact_mass(left: ExactMass, right: ExactMass) -> Result<Ordering, ScenarioMissionRiskGateError> {
    if !valid_probability_mass(left) || !valid_probability_mass(right) {
        return Err(ScenarioMissionRiskGateError::InvalidExactMass);
    }
    let left_scaled = u128::from(left.numerator) * u128::from(right.denominator);
    let right_scaled = u128::from(right.numerator) * u128::from(left.denominator);
    Ok(left_scaled.cmp(&right_scaled))
}

fn close_enough(left: f64, right: f64) -> bool {
    left.is_finite()
        && right.is_finite()
        && (left - right).abs() <= 16.0 * f64::EPSILON * (1.0 + left.abs().max(right.abs()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission_risk_aggregation::{
        EventMarginalMass, FrechetUnionBounds, JointEventSignatureMass, PairwiseIntersectionMass,
    };
    use crate::DefensivePortfolioPairInterval;

    const SCENARIO_A: [u8; 32] = [1; 32];
    const SCENARIO_B: [u8; 32] = [2; 32];
    const PORTFOLIO_A: [u8; 32] = [10; 32];
    const PORTFOLIO_B: [u8; 32] = [20; 32];
    const PORTFOLIO_C: [u8; 32] = [30; 32];
    const CONSEQUENCE_UNIT: [u8; 32] = [40; 32];
    const TARGET_A: [u8; 32] = [50; 32];
    const TARGET_B: [u8; 32] = [51; 32];
    const DENSITY_A: [u8; 32] = [60; 32];
    const DENSITY_B: [u8; 32] = [61; 32];

    fn exact(numerator: u64, denominator: u64) -> ExactMass {
        ExactMass { numerator, denominator }
    }

    fn envelope(
        manifest: u8,
        target: [u8; 32],
        inventory: u8,
        union: ExactMass,
        budget: ExactMass,
    ) -> MissionRiskEnvelope {
        let no_event = ExactMass {
            numerator: union.denominator - union.numerator,
            denominator: union.denominator,
        };
        let within = compare_exact_mass(union, budget).unwrap() != Ordering::Greater;
        let mut joint_signatures = Vec::new();
        if no_event.numerator > 0 {
            joint_signatures.push(JointEventSignatureMass {
                contacting_event_ids: Vec::new(),
                mass: no_event,
            });
        }
        if union.numerator > 0 {
            joint_signatures.push(JointEventSignatureMass {
                contacting_event_ids: vec!["event-1".to_owned()],
                mass: union,
            });
        }
        MissionRiskEnvelope {
            manifest_digest: [manifest; 32],
            target_measure_digest: target,
            event_inventory_digest: [inventory; 32],
            total_mass_ticks: 100,
            event_count: 1,
            trajectory_atom_count: 2,
            event_marginals: vec![EventMarginalMass {
                event_id: "event-1".to_owned(),
                mass: union,
            }],
            pairwise_intersections: Vec::<PairwiseIntersectionMass>::new(),
            joint_signatures,
            pathwise_union_mass: union,
            no_event_mass: no_event,
            marginal_sum_ticks: u128::from(union.numerator) * 100 / u128::from(union.denominator),
            frechet_union_bounds: FrechetUnionBounds {
                lower_bound: union,
                upper_bound: union,
            },
            independence_union_approximation: union.numerator as f64 / union.denominator as f64,
            risk_budget: budget,
            budget_status: if within {
                MissionRiskBudgetStatus::WithinDeclaredFiniteSupportBudget
            } else {
                MissionRiskBudgetStatus::ExceedsDeclaredFiniteSupportBudget
            },
            authority_granted: false,
        }
    }

    fn contracts() -> Vec<PortfolioScenarioRiskContract> {
        let budget = exact(1, 5);
        let mut rows = Vec::new();
        for (scenario_id, target, density, base) in [
            (SCENARIO_A, TARGET_A, DENSITY_A, 70_u8),
            (SCENARIO_B, TARGET_B, DENSITY_B, 80_u8),
        ] {
            for (offset, portfolio_id) in [
                (0_u8, PORTFOLIO_A),
                (1_u8, PORTFOLIO_B),
                (2_u8, PORTFOLIO_C),
            ] {
                let union = if scenario_id == SCENARIO_B && portfolio_id == PORTFOLIO_C {
                    exact(1, 2)
                } else {
                    exact(1, 10)
                };
                let manifest_digest = [base + offset; 32];
                let event_inventory_digest = [base + 10 + offset; 32];
                let risk = envelope(
                    manifest_digest[0],
                    target,
                    event_inventory_digest[0],
                    union,
                    budget,
                );
                rows.push(PortfolioScenarioRiskContract {
                    scenario_id,
                    portfolio_id,
                    manifest_digest,
                    target_measure_digest: target,
                    density_space_digest: density,
                    event_inventory_digest,
                    risk_envelope_digest: mission_risk_envelope_digest(&risk),
                    total_mass_ticks: 100,
                    risk_budget: budget,
                });
            }
        }
        rows
    }

    fn risk_evidence() -> Vec<PortfolioScenarioRiskEvidence> {
        let budget = exact(1, 5);
        let mut rows = Vec::new();
        for contract in contracts() {
            let union = if contract.scenario_id == SCENARIO_B
                && contract.portfolio_id == PORTFOLIO_C
            {
                exact(1, 2)
            } else {
                exact(1, 10)
            };
            rows.push(PortfolioScenarioRiskEvidence {
                scenario_id: contract.scenario_id,
                portfolio_id: contract.portfolio_id,
                envelope: envelope(
                    contract.manifest_digest[0],
                    contract.target_measure_digest,
                    contract.event_inventory_digest[0],
                    union,
                    budget,
                ),
            });
        }
        rows
    }

    fn scenario_report(
        scenario_id: [u8; 32],
        target_id: [u8; 32],
        density_id: [u8; 32],
        means: [f64; 3],
    ) -> SimultaneousDefensivePortfolioReport {
        let pairs = [
            (PORTFOLIO_A, PORTFOLIO_B, 0, 1),
            (PORTFOLIO_A, PORTFOLIO_C, 0, 2),
            (PORTFOLIO_B, PORTFOLIO_C, 1, 2),
        ];
        let intervals = pairs
            .iter()
            .map(|(left, right, left_index, right_index)| {
                let difference = means[*left_index] - means[*right_index];
                DefensivePortfolioPairInterval {
                    left_portfolio_id: *left,
                    right_portfolio_id: *right,
                    sample_count: 100,
                    contrast_alpha: 0.025 / 3.0,
                    lower_expected_difference_ticks: difference,
                    upper_expected_difference_ticks: difference,
                    lower_normalized_expected_difference: 0.5,
                    upper_normalized_expected_difference: 0.5,
                }
            })
            .collect::<Vec<_>>();
        SimultaneousDefensivePortfolioReport {
            consequence_unit_id: CONSEQUENCE_UNIT,
            family_alpha: 0.025,
            portfolio_count: 3,
            contrast_count: 3,
            contrast_alpha: 0.025 / 3.0,
            draw_count: 100,
            pairwise_intervals: intervals,
            unique_best_portfolio_id: None,
            statistically_undominated_portfolio_ids: vec![
                PORTFOLIO_A,
                PORTFOLIO_B,
                PORTFOLIO_C,
            ],
            target_measure_id: target_id,
            density_space_id: density_id,
            scenario_id,
            configuration_digest: [scenario_id[0] + 100; 32],
            effective_defensive_fraction: 0.2,
        }
    }

    fn consequence_reports() -> Vec<SimultaneousDefensivePortfolioReport> {
        vec![
            scenario_report(SCENARIO_A, TARGET_A, DENSITY_A, [0.0, 10.0, 20.0]),
            scenario_report(SCENARIO_B, TARGET_B, DENSITY_B, [20.0, 0.0, 10.0]),
        ]
    }

    fn gate_with_contracts(
        mut contracts: Vec<PortfolioScenarioRiskContract>,
    ) -> ScenarioMissionRiskGate {
        contracts.reverse();
        ScenarioMissionRiskGate::new(
            0.05,
            vec![SCENARIO_B, SCENARIO_A],
            vec![PORTFOLIO_C, PORTFOLIO_A, PORTFOLIO_B],
            CONSEQUENCE_UNIT,
            contracts,
        )
        .unwrap()
    }

    fn gate() -> ScenarioMissionRiskGate {
        gate_with_contracts(contracts())
    }

    fn contracts_matching_evidence(
        evidence: &[PortfolioScenarioRiskEvidence],
    ) -> Vec<PortfolioScenarioRiskContract> {
        let mut contract_rows = contracts();
        for contract in &mut contract_rows {
            let row = evidence
                .iter()
                .find(|row| {
                    row.scenario_id == contract.scenario_id
                        && row.portfolio_id == contract.portfolio_id
                })
                .unwrap();
            contract.risk_envelope_digest = mission_risk_envelope_digest(&row.envelope);
        }
        contract_rows
    }

    #[test]
    fn hard_risk_gate_excludes_infeasible_portfolio_before_regret_selection() {
        let report = gate()
            .evaluate(&risk_evidence(), &consequence_reports())
            .unwrap();

        assert_eq!(report.eligible_portfolio_ids, vec![PORTFOLIO_A, PORTFOLIO_B]);
        assert_eq!(report.ineligible_portfolio_ids, vec![PORTFOLIO_C]);
        assert_eq!(
            report.ineligible_scenarios_by_portfolio.get(&PORTFOLIO_C),
            Some(&vec![SCENARIO_B])
        );

        // After C is removed by the hard gate, A/B regrets are A=20 and B=10.
        // B is therefore the unique minimum-regret portfolio among eligible arms.
        assert_eq!(report.unique_best_eligible_portfolio_id, Some(PORTFOLIO_B));
        assert_eq!(
            report.minimax_regret_undominated_eligible_portfolio_ids,
            vec![PORTFOLIO_B]
        );
        assert_eq!(report.eligible_portfolio_regret_intervals[0].portfolio_id, PORTFOLIO_A);
        assert_eq!(
            report.eligible_portfolio_regret_intervals[0].upper_worst_case_regret_ticks,
            20.0
        );
        assert_eq!(report.eligible_portfolio_regret_intervals[1].portfolio_id, PORTFOLIO_B);
        assert_eq!(
            report.eligible_portfolio_regret_intervals[1].upper_worst_case_regret_ticks,
            10.0
        );
        assert!(!report.authority_granted);
    }

    #[test]
    fn exact_budget_boundary_is_eligible_but_any_exceedance_is_not() {
        let budget = exact(1, 5);
        let mut evidence = risk_evidence();
        let row = evidence
            .iter_mut()
            .find(|row| row.scenario_id == SCENARIO_B && row.portfolio_id == PORTFOLIO_C)
            .unwrap();
        let contract = contracts()
            .into_iter()
            .find(|contract| {
                contract.scenario_id == SCENARIO_B && contract.portfolio_id == PORTFOLIO_C
            })
            .unwrap();
        row.envelope = envelope(
            contract.manifest_digest[0],
            TARGET_B,
            contract.event_inventory_digest[0],
            budget,
            budget,
        );

        let report = gate_with_contracts(contracts_matching_evidence(&evidence))
            .evaluate(&evidence, &consequence_reports())
            .unwrap();
        assert_eq!(report.eligible_portfolio_ids, vec![PORTFOLIO_A, PORTFOLIO_B, PORTFOLIO_C]);
        assert!(report.ineligible_portfolio_ids.is_empty());
    }

    #[test]
    fn zero_and_unit_union_masses_allow_absent_zero_mass_signatures() {
        let mut evidence = risk_evidence();
        let contract = contracts()
            .into_iter()
            .find(|contract| {
                contract.scenario_id == SCENARIO_A && contract.portfolio_id == PORTFOLIO_A
            })
            .unwrap();
        let row = evidence
            .iter_mut()
            .find(|row| {
                row.scenario_id == SCENARIO_A && row.portfolio_id == PORTFOLIO_A
            })
            .unwrap();
        row.envelope = envelope(
            contract.manifest_digest[0],
            contract.target_measure_digest,
            contract.event_inventory_digest[0],
            exact(0, 1),
            exact(1, 5),
        );

        let contract = contracts()
            .into_iter()
            .find(|contract| {
                contract.scenario_id == SCENARIO_A && contract.portfolio_id == PORTFOLIO_B
            })
            .unwrap();
        let row = evidence
            .iter_mut()
            .find(|row| {
                row.scenario_id == SCENARIO_A && row.portfolio_id == PORTFOLIO_B
            })
            .unwrap();
        row.envelope = envelope(
            contract.manifest_digest[0],
            contract.target_measure_digest,
            contract.event_inventory_digest[0],
            exact(1, 1),
            exact(1, 5),
        );

        let report = gate_with_contracts(contracts_matching_evidence(&evidence))
            .evaluate(&evidence, &consequence_reports())
            .unwrap();
        assert!(report.eligible_portfolio_ids.contains(&PORTFOLIO_A));
        assert!(report.ineligible_portfolio_ids.contains(&PORTFOLIO_B));
    }

    #[test]
    fn missing_duplicate_or_substituted_risk_cells_fail_closed() {
        let mut rows = risk_evidence();
        rows.pop();
        assert_eq!(
            gate().evaluate(&rows, &consequence_reports()),
            Err(ScenarioMissionRiskGateError::IncompleteRiskEvidenceMatrix)
        );

        let mut rows = risk_evidence();
        rows[0] = rows[1].clone();
        assert_eq!(
            gate().evaluate(&rows, &consequence_reports()),
            Err(ScenarioMissionRiskGateError::IncompleteRiskEvidenceMatrix)
        );

        let mut rows = risk_evidence();
        rows[0].envelope.manifest_digest = [99; 32];
        assert_eq!(
            gate().evaluate(&rows, &consequence_reports()),
            Err(ScenarioMissionRiskGateError::RiskEnvelopeIdentityMismatch)
        );
    }

    #[test]
    fn status_complement_and_authority_inconsistency_are_rejected() {
        let mut rows = risk_evidence();
        rows[0].envelope.budget_status = MissionRiskBudgetStatus::ExceedsDeclaredFiniteSupportBudget;
        assert_eq!(
            gate_with_contracts(contracts_matching_evidence(&rows))
                .evaluate(&rows, &consequence_reports()),
            Err(ScenarioMissionRiskGateError::RiskBudgetStatusMismatch)
        );

        let mut rows = risk_evidence();
        rows[0].envelope.no_event_mass = exact(1, 10);
        assert_eq!(
            gate_with_contracts(contracts_matching_evidence(&rows))
                .evaluate(&rows, &consequence_reports()),
            Err(ScenarioMissionRiskGateError::UnionComplementMismatch)
        );

        let mut rows = risk_evidence();
        rows[0].envelope.authority_granted = true;
        assert_eq!(
            gate_with_contracts(contracts_matching_evidence(&rows))
                .evaluate(&rows, &consequence_reports()),
            Err(ScenarioMissionRiskGateError::UnexpectedAuthorityGrant)
        );
    }

    #[test]
    fn risk_envelope_content_is_bound_even_when_source_ids_are_unchanged() {
        let mut rows = risk_evidence();
        rows[0].envelope.pathwise_union_mass = exact(1, 2);

        assert_eq!(
            gate().evaluate(&rows, &consequence_reports()),
            Err(ScenarioMissionRiskGateError::RiskEnvelopeDigestMismatch)
        );
    }

    #[test]
    fn scenario_target_and_density_spaces_must_be_consistent() {
        let mut contracts = contracts();
        contracts
            .iter_mut()
            .find(|contract| {
                contract.scenario_id == SCENARIO_A && contract.portfolio_id == PORTFOLIO_C
            })
            .unwrap()
            .target_measure_digest = [99; 32];

        assert_eq!(
            ScenarioMissionRiskGate::new(
                0.05,
                vec![SCENARIO_A, SCENARIO_B],
                vec![PORTFOLIO_A, PORTFOLIO_B, PORTFOLIO_C],
                CONSEQUENCE_UNIT,
                contracts,
            )
            .unwrap_err(),
            ScenarioMissionRiskGateError::ScenarioContractMismatch
        );
    }

    #[test]
    fn all_portfolios_can_be_ineligible_without_a_fallback_recommendation() {
        let mut rows = risk_evidence();
        for row in &mut rows {
            let contract = contracts()
                .into_iter()
                .find(|contract| {
                    contract.scenario_id == row.scenario_id && contract.portfolio_id == row.portfolio_id
                })
                .unwrap();
            row.envelope = envelope(
                contract.manifest_digest[0],
                contract.target_measure_digest,
                contract.event_inventory_digest[0],
                exact(1, 2),
                exact(1, 5),
            );
        }

        let report = gate_with_contracts(contracts_matching_evidence(&rows))
            .evaluate(&rows, &consequence_reports())
            .unwrap();
        assert!(report.eligible_portfolio_ids.is_empty());
        assert_eq!(
            report.ineligible_portfolio_ids,
            vec![PORTFOLIO_A, PORTFOLIO_B, PORTFOLIO_C]
        );
        assert!(report.unique_best_eligible_portfolio_id.is_none());
        assert!(report.eligible_portfolio_regret_intervals.is_empty());
    }

    #[test]
    fn input_order_is_canonical_and_u64_cross_products_are_exact() {
        let first = gate().evaluate(&risk_evidence(), &consequence_reports()).unwrap();

        let mut risks = risk_evidence();
        risks.reverse();
        let mut consequence = consequence_reports();
        consequence.reverse();
        let second = gate().evaluate(&risks, &consequence).unwrap();
        assert_eq!(first, second);

        let left = exact(u64::MAX - 1, u64::MAX);
        let right = exact(u64::MAX - 2, u64::MAX - 1);
        assert_eq!(compare_exact_mass(left, right).unwrap(), Ordering::Greater);
    }
}
