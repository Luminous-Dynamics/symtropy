//! Joint construction production optioneering and observation-campaign evaluation.
//!
//! This composition layer calls the existing Avoid/Switch/Improve option evaluator and
//! bounded observation-campaign solver. It retains every robust-feasible option even if
//! the option-only evaluator labels it dominated: its option-specific evidence campaign
//! can change the joint Pareto ordering.
//!
//! It is a synthetic planning evaluator, not a design authority, survey engine, sensor
//! model, inspection authority, safety controller, project scheduler or permit system.

use std::collections::{BTreeMap, BTreeSet};

use construction_observation_planning_core::{ObservationAction, WorkNeed};
use construction_observation_portfolio_core::{
    CampaignSearchResult, PortfolioError, plan_observation_campaign,
};
use construction_production_optioneering_core::{
    FunctionalOutcomeContract, OptionCandidate, OptionEvaluation, OptioneeringError,
    OptioneeringReport, ReportDisposition, RobustObjectives, ScenarioCase, StageGateDecision,
    evaluate_option_set,
};
use construction_robot_interop_core::resources::ReservationBook;
use construction_spacetime_occupancy_core::SpatialOccupancyBook;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampaignCostTreatment {
    /// Add the campaign's declared direct cost once to the construction option LCC.
    AdditionalToOptionLifecycleCost,
    /// The option LCC already includes this exact campaign expenditure; do not add twice.
    AlreadyIncludedInOptionLifecycleCost,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionEvidenceCampaign {
    pub option_id: String,
    pub functional_contract_digest: String,
    pub project_context_digest: String,
    pub option_design_digest: String,
    pub floor_id: String,
    pub site_epoch: String,
    pub target_frame_id: String,
    pub clock_domain_id: String,
    pub currency_code: String,
    pub cost_treatment: CampaignCostTreatment,
    /// Accountable reference describing how campaign expenditure is included/excluded.
    pub cost_boundary_ref: String,
    pub work_needs: Vec<WorkNeed>,
    pub actions: Vec<ObservationAction>,
    /// Option-specific snapshots. Planning runs on scratch books inside the portfolio
    /// solver; caller-owned production books are not mutated.
    pub occupancy_book: SpatialOccupancyBook,
    pub reservation_book: ReservationBook,
}

impl OptionEvidenceCampaign {
    fn validate_for(
        &self,
        contract: &FunctionalOutcomeContract,
        option: &OptionCandidate,
    ) -> Result<(), JointEvaluationError> {
        require_non_empty("campaign_option_id", &self.option_id)?;
        require_non_empty("campaign_contract_digest", &self.functional_contract_digest)?;
        require_non_empty("campaign_project_context_digest", &self.project_context_digest)?;
        require_non_empty("campaign_option_design_digest", &self.option_design_digest)?;
        require_non_empty("campaign_floor_id", &self.floor_id)?;
        require_non_empty("campaign_site_epoch", &self.site_epoch)?;
        require_non_empty("campaign_target_frame_id", &self.target_frame_id)?;
        require_non_empty("campaign_clock_domain_id", &self.clock_domain_id)?;
        require_non_empty("campaign_currency_code", &self.currency_code)?;
        require_non_empty("campaign_cost_boundary_ref", &self.cost_boundary_ref)?;

        if self.option_id != option.option_id {
            return Err(JointEvaluationError::CampaignOptionMismatch {
                expected: option.option_id.clone(),
                received: self.option_id.clone(),
            });
        }
        if self.functional_contract_digest != contract.contract_digest {
            return Err(JointEvaluationError::FunctionalContractMismatch(
                self.option_id.clone(),
            ));
        }
        if self.project_context_digest != contract.project_context_digest {
            return Err(JointEvaluationError::ProjectContextMismatch(
                self.option_id.clone(),
            ));
        }
        if self.option_design_digest != option.option_design_digest {
            return Err(JointEvaluationError::OptionDesignMismatch {
                option_id: self.option_id.clone(),
                expected: option.option_design_digest.clone(),
                received: self.option_design_digest.clone(),
            });
        }
        if self.currency_code != contract.currency_code {
            return Err(JointEvaluationError::CampaignCurrencyMismatch {
                option_id: self.option_id.clone(),
                expected: contract.currency_code.clone(),
                received: self.currency_code.clone(),
            });
        }
        validate_currency_code(&self.currency_code)?;

        let occupancy_binding = self.occupancy_book.binding();
        if occupancy_binding.floor_id != self.floor_id
            || occupancy_binding.site_epoch != self.site_epoch
            || occupancy_binding.design_revision_digest != self.option_design_digest
            || occupancy_binding.target_frame_id != self.target_frame_id
        {
            return Err(JointEvaluationError::BaseOccupancyContextMismatch(
                self.option_id.clone(),
            ));
        }
        if self.reservation_book.current_site_epoch() != self.site_epoch {
            return Err(JointEvaluationError::ReservationSiteEpochMismatch(
                self.option_id.clone(),
            ));
        }

        let mut action_ids = BTreeSet::new();
        let mut occupancy_ids = BTreeSet::new();
        let mut reservation_ids = BTreeSet::new();
        for action in &self.actions {
            require_non_empty("action_id", &action.action_id)?;
            require_non_empty("coverage_profile_ref", &action.coverage_profile_ref)?;
            require_non_empty("calibration_ref", &action.calibration_ref)?;
            require_non_empty("action_currency_code", &action.cost_currency_code)?;
            require_non_empty("occupancy_id", &action.occupancy.occupancy_id)?;
            require_non_empty("occupancy_task_id", &action.occupancy.task_id)?;
            require_non_empty(
                "reservation_id",
                &action.occupancy.reservation.reservation_id,
            )?;
            require_non_empty(
                "reservation_resource_id",
                &action.occupancy.reservation.resource_id,
            )?;
            require_non_empty(
                "reservation_owner_id",
                &action.occupancy.reservation.owner_id,
            )?;
            if action.cost_currency_code != self.currency_code {
                return Err(JointEvaluationError::CampaignCurrencyMismatch {
                    option_id: self.option_id.clone(),
                    expected: self.currency_code.clone(),
                    received: action.cost_currency_code.clone(),
                });
            }
            if !action_ids.insert(action.action_id.as_str()) {
                return Err(JointEvaluationError::DuplicateAction(
                    action.action_id.clone(),
                ));
            }
            if !occupancy_ids.insert(action.occupancy.occupancy_id.as_str()) {
                return Err(JointEvaluationError::DuplicateOccupancy(
                    action.occupancy.occupancy_id.clone(),
                ));
            }
            if !reservation_ids.insert(action.occupancy.reservation.reservation_id.as_str()) {
                return Err(JointEvaluationError::DuplicateReservation(
                    action.occupancy.reservation.reservation_id.clone(),
                ));
            }

            let binding = &action.occupancy.binding;
            if binding.floor_id != self.floor_id
                || binding.site_epoch != self.site_epoch
                || binding.design_revision_digest != self.option_design_digest
                || binding.target_frame_id != self.target_frame_id
                || action.occupancy.reservation.site_epoch != self.site_epoch
                || action.occupancy.reservation.clock_domain_id != self.clock_domain_id
            {
                return Err(JointEvaluationError::ObservationContextMismatch {
                    option_id: self.option_id.clone(),
                    action_id: action.action_id.clone(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointObjectives {
    pub construction_lifecycle_cost_minor_units: u64,
    pub observation_campaign_cost_minor_units: u64,
    pub combined_lifecycle_cost_minor_units: u64,
    pub construction_elapsed_minutes: u64,
    pub observation_duration_minutes: u64,
    pub observation_scheduled_span_ms: u64,
    pub total_workflow_crew_minutes: u64,
    pub total_energy_wh: u64,
    pub construction_carbon_gco2e: u64,
    pub construction_material_mass_kg_milli: u64,
    pub construction_rework_units: u64,
}

impl JointObjectives {
    fn build(
        construction: &RobustObjectives,
        campaign: &construction_observation_portfolio_core::ObservationCampaign,
        treatment: CampaignCostTreatment,
    ) -> Result<Self, JointEvaluationError> {
        let combined_lifecycle_cost_minor_units = match treatment {
            CampaignCostTreatment::AdditionalToOptionLifecycleCost => construction
                .worst_lifecycle_cost_minor_units
                .checked_add(campaign.total_cost_minor_units)
                .ok_or(JointEvaluationError::ArithmeticOverflow)?,
            CampaignCostTreatment::AlreadyIncludedInOptionLifecycleCost => {
                construction.worst_lifecycle_cost_minor_units
            }
        };
        let total_workflow_crew_minutes = construction
            .worst_workflow_crew_minutes
            .checked_add(campaign.total_crew_minutes)
            .ok_or(JointEvaluationError::ArithmeticOverflow)?;
        let total_energy_wh = construction
            .worst_energy_wh
            .checked_add(campaign.total_energy_wh)
            .ok_or(JointEvaluationError::ArithmeticOverflow)?;

        Ok(Self {
            construction_lifecycle_cost_minor_units: construction.worst_lifecycle_cost_minor_units,
            observation_campaign_cost_minor_units: campaign.total_cost_minor_units,
            combined_lifecycle_cost_minor_units,
            construction_elapsed_minutes: construction.worst_elapsed_minutes,
            observation_duration_minutes: campaign.total_observation_duration_minutes,
            observation_scheduled_span_ms: campaign.scheduled_span_ms,
            total_workflow_crew_minutes,
            total_energy_wh,
            construction_carbon_gco2e: construction.worst_carbon_gco2e,
            construction_material_mass_kg_milli: construction.worst_material_mass_kg_milli,
            construction_rework_units: construction.worst_rework_units,
        })
    }

    fn dominates(&self, other: &Self) -> bool {
        let no_worse = self.combined_lifecycle_cost_minor_units
            <= other.combined_lifecycle_cost_minor_units
            && self.construction_elapsed_minutes <= other.construction_elapsed_minutes
            && self.observation_duration_minutes <= other.observation_duration_minutes
            && self.observation_scheduled_span_ms <= other.observation_scheduled_span_ms
            && self.total_workflow_crew_minutes <= other.total_workflow_crew_minutes
            && self.total_energy_wh <= other.total_energy_wh
            && self.construction_carbon_gco2e <= other.construction_carbon_gco2e
            && self.construction_material_mass_kg_milli
                <= other.construction_material_mass_kg_milli
            && self.construction_rework_units <= other.construction_rework_units;
        let strictly_better = self.combined_lifecycle_cost_minor_units
            < other.combined_lifecycle_cost_minor_units
            || self.construction_elapsed_minutes < other.construction_elapsed_minutes
            || self.observation_duration_minutes < other.observation_duration_minutes
            || self.observation_scheduled_span_ms < other.observation_scheduled_span_ms
            || self.total_workflow_crew_minutes < other.total_workflow_crew_minutes
            || self.total_energy_wh < other.total_energy_wh
            || self.construction_carbon_gco2e < other.construction_carbon_gco2e
            || self.construction_material_mass_kg_milli
                < other.construction_material_mass_kg_milli
            || self.construction_rework_units < other.construction_rework_units;
        no_worse && strictly_better
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JointPackageStatus {
    /// Construction option passed every scenario; campaign is feasible and proven optimal.
    JointOptimalFeasible,
    /// A complete campaign was found, but the campaign search budget did not prove optimality.
    FeasibleCampaignOptimalityUnproven,
    /// Exhaustive campaign search proved no complete feasible campaign exists.
    CampaignInfeasibleProven,
    /// Search budget ended before campaign feasibility or infeasibility was proven.
    CampaignSearchIncomplete,
    /// Construction alternative failed one or more required hard scenario gates.
    ConstructionNotRobustFeasible,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointOptionEvaluation {
    pub option_id: String,
    pub construction_evaluation: OptionEvaluation,
    pub status: JointPackageStatus,
    pub campaign_search: Option<CampaignSearchResult>,
    pub objectives: Option<JointObjectives>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointOptioneeringReport {
    pub functional_contract_id: String,
    pub functional_contract_digest: String,
    pub unit_class_id: String,
    pub unit_symbol: String,
    pub required_units: u64,
    pub assessment_horizon_months: u64,
    pub evaluated_tier: Option<construction_production_optioneering_core::OptionTier>,
    pub joint_evaluations: Vec<JointOptionEvaluation>,
    /// Only options with complete, optimality-proven campaigns enter this frontier.
    pub proven_frontier_option_ids: Vec<String>,
    /// Complete campaigns found before the search budget ended.
    pub provisional_feasible_option_ids: Vec<String>,
    /// False if any robust-feasible option has an unresolved campaign search.
    pub joint_frontier_proven: bool,
    pub unused_campaign_option_ids: Vec<String>,
    pub deferred_option_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JointEvaluationError {
    EmptyField(&'static str),
    InvalidCurrencyCode(String),
    DuplicateCampaignOption(String),
    DuplicateAction(String),
    DuplicateOccupancy(String),
    DuplicateReservation(String),
    MissingCampaignInput(String),
    UnexpectedCampaignInput(String),
    CampaignOptionMismatch { expected: String, received: String },
    FunctionalContractMismatch(String),
    ProjectContextMismatch(String),
    OptionDesignMismatch { option_id: String, expected: String, received: String },
    CampaignCurrencyMismatch { option_id: String, expected: String, received: String },
    ObservationContextMismatch { option_id: String, action_id: String },
    BaseOccupancyContextMismatch(String),
    ReservationSiteEpochMismatch(String),
    ArithmeticOverflow,
    Optioneering(OptioneeringError),
    Portfolio(PortfolioError),
}

impl From<OptioneeringError> for JointEvaluationError {
    fn from(value: OptioneeringError) -> Self {
        Self::Optioneering(value)
    }
}

impl From<PortfolioError> for JointEvaluationError {
    fn from(value: PortfolioError) -> Self {
        Self::Portfolio(value)
    }
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), JointEvaluationError> {
    if value.trim().is_empty() {
        return Err(JointEvaluationError::EmptyField(name));
    }
    Ok(())
}

fn validate_currency_code(value: &str) -> Result<(), JointEvaluationError> {
    if value.len() != 3 || !value.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err(JointEvaluationError::InvalidCurrencyCode(value.to_owned()));
    }
    Ok(())
}

/// Evaluate design/method alternatives together with their own exact feasible evidence
/// campaigns. Robust-feasible options that are dominated on construction-only objectives
/// are intentionally retained: their campaign burden can change the joint frontier.
///
/// Each option must have an option-specific planning snapshot. The portfolio planner uses
/// scratch clones and does not mutate these supplied books. Search exhaustion is always
/// reported as incomplete or feasible-but-unproven; it never produces a proven frontier.
pub fn evaluate_option_evidence_packages(
    contract: &FunctionalOutcomeContract,
    stage_decisions: &[StageGateDecision],
    scenarios: &[ScenarioCase],
    options: &[OptionCandidate],
    campaigns: &[OptionEvidenceCampaign],
    now_ms: u64,
    node_budget_per_option: usize,
) -> Result<JointOptioneeringReport, JointEvaluationError> {
    let option_report = evaluate_option_set(contract, stage_decisions, scenarios, options)?;

    let mut campaign_by_option = BTreeMap::new();
    for campaign in campaigns {
        if campaign_by_option
            .insert(campaign.option_id.as_str(), campaign)
            .is_some()
        {
            return Err(JointEvaluationError::DuplicateCampaignOption(
                campaign.option_id.clone(),
            ));
        }
    }

    let option_by_id: BTreeMap<&str, &OptionCandidate> = options
        .iter()
        .map(|option| (option.option_id.as_str(), option))
        .collect();
    for campaign_id in campaign_by_option.keys() {
        if !option_by_id.contains_key(campaign_id) {
            return Err(JointEvaluationError::UnexpectedCampaignInput(
                (*campaign_id).to_owned(),
            ));
        }
    }
    let evaluated_ids: BTreeSet<&str> = option_report
        .evaluations
        .iter()
        .map(|evaluation| evaluation.option_id.as_str())
        .collect();
    let mut unused_campaign_option_ids = Vec::new();
    let mut joint_evaluations = Vec::new();
    let mut unresolved_campaign = false;

    for evaluation in &option_report.evaluations {
        let Some(construction_objectives) = evaluation.robust_objectives.as_ref() else {
            joint_evaluations.push(JointOptionEvaluation {
                option_id: evaluation.option_id.clone(),
                construction_evaluation: evaluation.clone(),
                status: JointPackageStatus::ConstructionNotRobustFeasible,
                campaign_search: None,
                objectives: None,
            });
            if campaign_by_option.contains_key(evaluation.option_id.as_str()) {
                unused_campaign_option_ids.push(evaluation.option_id.clone());
            }
            continue;
        };

        let option = option_by_id
            .get(evaluation.option_id.as_str())
            .copied()
            .ok_or_else(|| JointEvaluationError::UnexpectedCampaignInput(
                evaluation.option_id.clone(),
            ))?;
        let campaign = campaign_by_option
            .get(evaluation.option_id.as_str())
            .copied()
            .ok_or_else(|| JointEvaluationError::MissingCampaignInput(
                evaluation.option_id.clone(),
            ))?;
        campaign.validate_for(contract, option)?;

        let search = plan_observation_campaign(
            &campaign.actions,
            &campaign.work_needs,
            &campaign.occupancy_book,
            &campaign.reservation_book,
            now_ms,
            node_budget_per_option,
        )?;

        if let Some(currency) = search.best_campaign.cost_currency_code.as_deref() {
            if currency != campaign.currency_code {
                return Err(JointEvaluationError::CampaignCurrencyMismatch {
                    option_id: evaluation.option_id.clone(),
                    expected: campaign.currency_code.clone(),
                    received: currency.to_owned(),
                });
            }
        }

        let (status, objectives) = if search.optimality_proven
            && search.best_campaign.complete
        {
            (
                JointPackageStatus::JointOptimalFeasible,
                Some(JointObjectives::build(
                    construction_objectives,
                    &search.best_campaign,
                    campaign.cost_treatment,
                )?),
            )
        } else if search.best_campaign.complete {
            unresolved_campaign = true;
            (
                JointPackageStatus::FeasibleCampaignOptimalityUnproven,
                Some(JointObjectives::build(
                    construction_objectives,
                    &search.best_campaign,
                    campaign.cost_treatment,
                )?),
            )
        } else if search.infeasibility_proven {
            (
                JointPackageStatus::CampaignInfeasibleProven,
                None,
            )
        } else {
            unresolved_campaign = true;
            (
                JointPackageStatus::CampaignSearchIncomplete,
                None,
            )
        };

        joint_evaluations.push(JointOptionEvaluation {
            option_id: evaluation.option_id.clone(),
            construction_evaluation: evaluation.clone(),
            status,
            campaign_search: Some(search),
            objectives,
        });
    }

    for campaign_id in campaign_by_option.keys() {
        if !evaluated_ids.contains(campaign_id) {
            unused_campaign_option_ids.push((*campaign_id).to_owned());
        }
    }
    unused_campaign_option_ids.sort();
    unused_campaign_option_ids.dedup();

    let mut proven_frontier_option_ids = Vec::new();
    let proven_indices: Vec<usize> = joint_evaluations
        .iter()
        .enumerate()
        .filter_map(|(index, evaluation)| {
            (evaluation.status == JointPackageStatus::JointOptimalFeasible)
                .then_some(index)
        })
        .collect();

    for index in &proven_indices {
        let current = joint_evaluations[*index]
            .objectives
            .as_ref()
            .expect("proven feasible package has joint objectives");
        let dominated = proven_indices.iter().any(|other_index| {
            if other_index == index {
                return false;
            }
            joint_evaluations[*other_index]
                .objectives
                .as_ref()
                .is_some_and(|other| other.dominates(current))
        });
        if !dominated {
            proven_frontier_option_ids.push(joint_evaluations[*index].option_id.clone());
        }
    }
    proven_frontier_option_ids.sort();

    let mut provisional_feasible_option_ids: Vec<String> = joint_evaluations
        .iter()
        .filter_map(|evaluation| {
            (evaluation.status == JointPackageStatus::FeasibleCampaignOptimalityUnproven)
                .then_some(evaluation.option_id.clone())
        })
        .collect();
    provisional_feasible_option_ids.sort();

    let construction_frontier_resolved = matches!(
        option_report.disposition,
        ReportDisposition::FrontierComputed | ReportDisposition::NoRobustFeasibleCandidates
    );
    let joint_frontier_proven = construction_frontier_resolved && !unresolved_campaign;
    Ok(JointOptioneeringReport {
        functional_contract_id: option_report.functional_contract_id,
        functional_contract_digest: option_report.functional_contract_digest,
        unit_class_id: option_report.unit_class_id,
        unit_symbol: option_report.unit_symbol,
        required_units: option_report.required_units,
        assessment_horizon_months: option_report.assessment_horizon_months,
        evaluated_tier: option_report.evaluated_tier,
        joint_evaluations,
        proven_frontier_option_ids,
        provisional_feasible_option_ids,
        joint_frontier_proven,
        unused_campaign_option_ids,
        deferred_option_ids: option_report.deferred_option_ids,
    })
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use construction_production_optioneering_core::{
        EvidenceDisposition, FunctionalOutcomeContract, MeasurementBasis, OptionCandidate,
        OptionStatus, OptionTier, PerformanceRequirement, ScenarioResult, StageDisposition,
        StageGateDecision,
    };
    use construction_robot_interop_core::resources::ReservationRequest;
    use construction_spacetime_occupancy_core::{
        Bounds3Mm, OccupancyKind, SiteBinding, SpatialOccupancy,
    };

    fn contract() -> FunctionalOutcomeContract {
        FunctionalOutcomeContract {
            contract_id: "partition-m2-v1".to_owned(),
            contract_digest: "git-blob-sha1:partition-contract".to_owned(),
            project_context_digest: "git-blob-sha1:project-context".to_owned(),
            unit_class_id: "non-loadbearing-partition".to_owned(),
            unit_symbol: "m2".to_owned(),
            required_units: 1,
            acceptance_contract_ref: "acceptance:partition-v1".to_owned(),
            currency_code: "USD".to_owned(),
            carbon_assessment_boundary_ref: "lca-boundary:cradle-to-end-of-life".to_owned(),
            assessment_horizon_months: 240,
            required_gate_ids: vec!["design-authority".to_owned()],
            performance_requirements: vec![PerformanceRequirement {
                metric_id: "fire-rating-minutes".to_owned(),
                unit_symbol: "min".to_owned(),
                scale: 1,
                minimum_value: Some(60),
                maximum_value: None,
            }],
        }
    }

    fn stage_decisions() -> Vec<StageGateDecision> {
        vec![StageGateDecision {
            tier: OptionTier::Avoid,
            disposition: StageDisposition::OpenForEvaluation,
            accountable_owner_ref: "client-decision-owner".to_owned(),
            project_context_digest: contract().project_context_digest,
            evidence_refs: vec!["stage-evidence:avoid".to_owned()],
            rationale: "evaluate the function-first avoid alternatives".to_owned(),
        }]
    }

    fn option(
        id: &str,
        design: &str,
        direct_cost: u64,
        crew_minutes: u64,
        elapsed_minutes: u64,
        energy_wh: u64,
        carbon_gco2e: u64,
        acceptance_qualified: bool,
    ) -> OptionCandidate {
        let contract = contract();
        let acceptance = if acceptance_qualified {
            EvidenceDisposition::Qualified {
                authority_ref: format!("authority:{id}"),
                receipt_ref: format!("acceptance-receipt:{id}"),
            }
        } else {
            EvidenceDisposition::Unknown {
                reason_ref: format!("unknown-acceptance:{id}"),
            }
        };
        let result = ScenarioResult {
            scenario_id: "nominal".to_owned(),
            measurement_basis: MeasurementBasis::FullRequiredQuantity,
            option_design_digest: design.to_owned(),
            assessment_horizon_months: contract.assessment_horizon_months,
            acceptance_contract_digest: contract.contract_digest.clone(),
            accepted_option_design_digest: design.to_owned(),
            acceptance,
            hard_gate_results: BTreeMap::from([(
                "design-authority".to_owned(),
                EvidenceDisposition::Qualified {
                    authority_ref: format!("design-authority:{id}"),
                    receipt_ref: format!("design-receipt:{id}"),
                },
            )]),
            accepted_units: 1,
            performance_values: BTreeMap::from([("fire-rating-minutes".to_owned(), 90)]),
            direct_cost_minor_units: direct_cost,
            shared_cost_minor_units: 0,
            maintenance_cost_minor_units: 0,
            repair_cost_minor_units: 0,
            end_of_life_cost_minor_units: 0,
            elapsed_minutes,
            workflow_crew_minutes: crew_minutes,
            energy_wh,
            carbon_gco2e,
            material_mass_kg_milli: 1_000,
            rework_units: 0,
        };

        OptionCandidate {
            option_id: id.to_owned(),
            tier: OptionTier::Avoid,
            functional_contract_id: contract.contract_id,
            functional_contract_digest: contract.contract_digest,
            option_design_digest: design.to_owned(),
            method_profile_digest: format!("method:{design}"),
            scheduler_profile_digest: "scheduler:baseline".to_owned(),
            fleet_profile_digest: "fleet:baseline".to_owned(),
            matched_reference_option_id: None,
            scenario_results: BTreeMap::from([("nominal".to_owned(), result)]),
        }
    }

    fn campaign(
        option: &OptionCandidate,
        cost_minor_units: u64,
        crew_minutes: u64,
        cost_treatment: CampaignCostTreatment,
    ) -> OptionEvidenceCampaign {
        let floor_id = "reference-floor-01".to_owned();
        let site_epoch = "site-epoch-1".to_owned();
        let target_frame_id = "project-grid-1".to_owned();
        let clock_domain_id = "site-clock-1".to_owned();
        let task_id = format!("survey-task-{}", option.option_id);
        let occupancy_id = format!("survey-occupancy-{}", option.option_id);
        let reservation_id = format!("survey-reservation-{}", option.option_id);
        let resource_id = "SURVEY-INSTRUMENT-1".to_owned();
        let binding = SiteBinding {
            floor_id: floor_id.clone(),
            site_epoch: site_epoch.clone(),
            design_revision_digest: option.option_design_digest.clone(),
            target_frame_id: target_frame_id.clone(),
        };
        let occupancy = SpatialOccupancy {
            occupancy_id: occupancy_id.clone(),
            task_id: task_id.clone(),
            kind: OccupancyKind::InspectionAccess,
            binding,
            bounds: Bounds3Mm {
                min: [0, 0, 0],
                max: [1_000, 1_000, 1_000],
            },
            uncertainty_margin_mm: [10, 10, 10],
            clearance_margin_mm: [20, 20, 20],
            reservation: ReservationRequest {
                reservation_id,
                resource_id: resource_id.clone(),
                task_id: task_id.clone(),
                owner_id: format!("survey-owner:{}", option.option_id),
                site_epoch: site_epoch.clone(),
                clock_domain_id: clock_domain_id.clone(),
                clock_uncertainty_ms: 0,
                interval_start_ms: 0,
                interval_end_ms: 60_000,
                expires_at_ms: 120_000,
                units: 1,
            },
        };

        let action = ObservationAction {
            action_id: format!("scan-{}", option.option_id),
            coverage_profile_ref: format!("coverage-profile:{}", option.option_id),
            calibration_ref: format!("calibration:{}", option.option_id),
            covered_region_ids: vec!["region-1".to_owned()],
            occupancy,
            duration_minutes: 10,
            crew_minutes,
            cost_minor_units,
            cost_currency_code: "USD".to_owned(),
            energy_wh: 25,
        };

        let mut reservation_book =
            ReservationBook::new(site_epoch.clone(), clock_domain_id.clone(), 0)
                .expect("reservation book");
        reservation_book
            .register_resource(resource_id, 1)
            .expect("survey resource");
        let occupancy_book =
            SpatialOccupancyBook::new(
                SiteBinding {
                    floor_id: floor_id.clone(),
                    site_epoch: site_epoch.clone(),
                    design_revision_digest: option.option_design_digest.clone(),
                    target_frame_id: target_frame_id.clone(),
                },
                clock_domain_id.clone(),
            )
            .expect("occupancy book");

        OptionEvidenceCampaign {
            option_id: option.option_id.clone(),
            functional_contract_digest: contract().contract_digest,
            project_context_digest: contract().project_context_digest,
            option_design_digest: option.option_design_digest.clone(),
            floor_id,
            site_epoch,
            target_frame_id,
            clock_domain_id,
            currency_code: "USD".to_owned(),
            cost_treatment,
            cost_boundary_ref: format!("cost-boundary:{}", option.option_id),
            work_needs: vec![WorkNeed {
                work_unit_id: format!("accepted-work-{}", option.option_id),
                region_id: "region-1".to_owned(),
                uncertainty_bound_mm: [100, 100, 100],
            }],
            actions: vec![action],
            occupancy_book,
            reservation_book,
        }
    }

    fn evaluate(
        options: &[OptionCandidate],
        campaigns: &[OptionEvidenceCampaign],
        node_budget: usize,
    ) -> Result<JointOptioneeringReport, JointEvaluationError> {
        evaluate_option_evidence_packages(
            &contract(),
            &stage_decisions(),
            &[ScenarioCase {
                scenario_id: "nominal".to_owned(),
            }],
            options,
            campaigns,
            0,
            node_budget,
        )
    }

    #[test]
    fn option_dominated_before_campaign_can_become_joint_frontier_winner() {
        // option-a is strictly dominated by option-b on construction-only objectives:
        // lower cost, labour, elapsed time, energy and carbon belong to option-b.
        let option_a = option(
            "option-a",
            "design-a",
            100,
            100,
            100,
            100,
            100,
            true,
        );
        let option_b = option(
            "option-b",
            "design-b",
            90,
            90,
            100,
            100,
            100,
            true,
        );
        let campaigns = vec![
            campaign(
                &option_a,
                0,
                0,
                CampaignCostTreatment::AdditionalToOptionLifecycleCost,
            ),
            campaign(
                &option_b,
                100,
                100,
                CampaignCostTreatment::AdditionalToOptionLifecycleCost,
            ),
        ];

        let report = evaluate(&[option_a, option_b], &campaigns, 100)
            .expect("joint option/evidence evaluation");

        let construction_a = report
            .joint_evaluations
            .iter()
            .find(|evaluation| evaluation.option_id == "option-a")
            .expect("option-a evaluation");
        assert_eq!(
            construction_a.construction_evaluation.status,
            OptionStatus::RobustFeasibleDominated {
                by_option_ids: vec!["option-b".to_owned()]
            }
        );
        assert_eq!(
            construction_a.status,
            JointPackageStatus::JointOptimalFeasible
        );
        assert_eq!(
            report.proven_frontier_option_ids,
            vec!["option-a".to_owned()]
        );
        assert!(report.joint_frontier_proven);

        let objectives_a = construction_a.objectives.as_ref().expect("joint objectives");
        assert_eq!(objectives_a.construction_lifecycle_cost_minor_units, 100);
        assert_eq!(objectives_a.observation_campaign_cost_minor_units, 0);
        assert_eq!(objectives_a.combined_lifecycle_cost_minor_units, 100);

        let evaluation_b = report
            .joint_evaluations
            .iter()
            .find(|evaluation| evaluation.option_id == "option-b")
            .expect("option-b evaluation");
        let objectives_b = evaluation_b.objectives.as_ref().expect("joint objectives");
        assert_eq!(objectives_b.combined_lifecycle_cost_minor_units, 190);
        assert_eq!(objectives_b.total_workflow_crew_minutes, 190);
    }

    #[test]
    fn observation_cost_boundary_prevents_accidental_double_count() {
        let candidate = option(
            "option-a",
            "design-a",
            100,
            20,
            30,
            50,
            60,
            true,
        );
        let additional = campaign(
            &candidate,
            25,
            5,
            CampaignCostTreatment::AdditionalToOptionLifecycleCost,
        );
        let included = campaign(
            &candidate,
            25,
            5,
            CampaignCostTreatment::AlreadyIncludedInOptionLifecycleCost,
        );

        let additional_report = evaluate(
            std::slice::from_ref(&candidate),
            &[additional],
            100,
        )
        .expect("additional cost evaluation");
        let included_report = evaluate(&[candidate], &[included], 100)
            .expect("included cost evaluation");

        let additional_cost = additional_report.joint_evaluations[0]
            .objectives
            .as_ref()
            .expect("additional objectives")
            .combined_lifecycle_cost_minor_units;
        let included_cost = included_report.joint_evaluations[0]
            .objectives
            .as_ref()
            .expect("included objectives")
            .combined_lifecycle_cost_minor_units;

        assert_eq!(additional_cost, 125);
        assert_eq!(included_cost, 100);
    }

    #[test]
    fn robust_infeasible_option_does_not_consume_campaign_search() {
        let infeasible = option(
            "bad-option",
            "design-bad",
            10,
            10,
            10,
            10,
            10,
            false,
        );
        let report = evaluate(std::slice::from_ref(&infeasible), &[], 100)
            .expect("construction rejection is reported before campaign search");

        assert_eq!(report.joint_evaluations.len(), 1);
        assert_eq!(
            report.joint_evaluations[0].status,
            JointPackageStatus::ConstructionNotRobustFeasible
        );
        assert!(report.joint_evaluations[0].campaign_search.is_none());
        assert!(report.joint_frontier_proven);
        assert!(report.proven_frontier_option_ids.is_empty());
    }

    #[test]
    fn design_revision_mismatch_rejects_campaign_before_search() {
        let candidate = option(
            "option-a",
            "design-a",
            100,
            20,
            30,
            50,
            60,
            true,
        );
        let mut mismatched = campaign(
            &candidate,
            10,
            5,
            CampaignCostTreatment::AdditionalToOptionLifecycleCost,
        );
        mismatched.option_design_digest = "stale-design-revision".to_owned();

        assert_eq!(
            evaluate(&[candidate], &[mismatched], 100),
            Err(JointEvaluationError::OptionDesignMismatch {
                option_id: "option-a".to_owned(),
                expected: "design-a".to_owned(),
                received: "stale-design-revision".to_owned(),
            })
        );
    }

    #[test]
    fn node_budget_exhaustion_never_claims_joint_optimality() {
        let candidate = option(
            "option-a",
            "design-a",
            100,
            20,
            30,
            50,
            60,
            true,
        );
        let plan = campaign(
            &candidate,
            10,
            5,
            CampaignCostTreatment::AdditionalToOptionLifecycleCost,
        );
        let report = evaluate(&[candidate], &[plan], 1)
            .expect("budget-limited search is a report");

        assert_eq!(
            report.joint_evaluations[0].status,
            JointPackageStatus::CampaignSearchIncomplete
        );
        assert!(!report.joint_frontier_proven);
        assert!(report.proven_frontier_option_ids.is_empty());
    }

    #[test]
    fn independent_pareto_oracle_matches_joint_frontier_fixture() {
        let option_a = option(
            "option-a",
            "design-a",
            100,
            100,
            100,
            100,
            100,
            true,
        );
        let option_b = option(
            "option-b",
            "design-b",
            90,
            90,
            100,
            100,
            100,
            true,
        );
        let campaigns = vec![
            campaign(
                &option_a,
                0,
                0,
                CampaignCostTreatment::AdditionalToOptionLifecycleCost,
            ),
            campaign(
                &option_b,
                100,
                100,
                CampaignCostTreatment::AdditionalToOptionLifecycleCost,
            ),
        ];
        let report = evaluate(&[option_a, option_b], &campaigns, 100)
            .expect("joint fixture");

        // Independently calculate the tiny two-package frontier from fixture-visible
        // objective fields. This oracle does not call JointObjectives::dominates.
        let a = report.joint_evaluations
            .iter()
            .find(|item| item.option_id == "option-a")
            .and_then(|item| item.objectives.as_ref())
            .expect("option-a objectives");
        let b = report.joint_evaluations
            .iter()
            .find(|item| item.option_id == "option-b")
            .and_then(|item| item.objectives.as_ref())
            .expect("option-b objectives");
        let b_no_worse_than_a =
            b.combined_lifecycle_cost_minor_units <= a.combined_lifecycle_cost_minor_units
            && b.total_workflow_crew_minutes <= a.total_workflow_crew_minutes
            && b.construction_elapsed_minutes <= a.construction_elapsed_minutes
            && b.total_energy_wh <= a.total_energy_wh
            && b.construction_carbon_gco2e <= a.construction_carbon_gco2e;
        let a_no_worse_than_b =
            a.combined_lifecycle_cost_minor_units <= b.combined_lifecycle_cost_minor_units
            && a.total_workflow_crew_minutes <= b.total_workflow_crew_minutes
            && a.construction_elapsed_minutes <= b.construction_elapsed_minutes
            && a.total_energy_wh <= b.total_energy_wh
            && a.construction_carbon_gco2e <= b.construction_carbon_gco2e;
        let oracle_frontier = match (a_no_worse_than_b, b_no_worse_than_a) {
            (true, false) => vec!["option-a".to_owned()],
            (false, true) => vec!["option-b".to_owned()],
            _ => vec!["option-a".to_owned(), "option-b".to_owned()],
        };
        assert_eq!(report.proven_frontier_option_ids, oracle_frontier);
    }

    #[test]
    fn option_evidence_package_rejects_base_occupancy_book_context_mismatch() {
        let candidate = option(
            "option-a",
            "design-a",
            100,
            20,
            30,
            50,
            60,
            true,
        );
        let mut package = campaign(
            &candidate,
            10,
            5,
            CampaignCostTreatment::AdditionalToOptionLifecycleCost,
        );
        package.occupancy_book = SpatialOccupancyBook::new(
            SiteBinding {
                floor_id: "wrong-floor".to_owned(),
                site_epoch: package.site_epoch.clone(),
                design_revision_digest: package.option_design_digest.clone(),
                target_frame_id: package.target_frame_id.clone(),
            },
            package.clock_domain_id.clone(),
        )
        .expect("wrong-floor occupancy book");

        assert_eq!(
            evaluate(&[candidate], &[package], 100),
            Err(JointEvaluationError::BaseOccupancyContextMismatch(
                "option-a".to_owned()
            ))
        );
    }

    #[test]
    fn missing_campaign_is_typed_for_robust_feasible_option() {
        let candidate = option(
            "option-a",
            "design-a",
            100,
            20,
            30,
            50,
            60,
            true,
        );
        assert_eq!(
            evaluate(&[candidate], &[], 100),
            Err(JointEvaluationError::MissingCampaignInput(
                "option-a".to_owned()
            ))
        );
    }
}
