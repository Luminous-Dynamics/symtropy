//! Evidence-bound construction production optioneering.
//!
//! This core compares alternatives against one homogeneous functional outcome,
//! applies explicit Avoid -> Switch -> Improve stage dispositions, requires every
//! frozen scenario to pass hard acceptance/evidence gates, and computes a robust
//! multiobjective Pareto frontier. It intentionally does not assign probabilities
//! or collapse safety, quality, cost, time and carbon into one score.
//!
//! It is not a structural designer, safety authority, code-compliance engine,
//! procurement system, scheduler, sensor model or professional approval process.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptionTier {
    Avoid,
    Switch,
    Improve,
}

impl OptionTier {
    pub const ORDERED: [Self; 3] = [Self::Avoid, Self::Switch, Self::Improve];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerformanceRequirement {
    pub metric_id: String,
    pub unit_symbol: String,
    /// Fixed-point integer value in the scale declared by the contract.
    pub minimum_value: Option<i64>,
    pub maximum_value: Option<i64>,
}

impl PerformanceRequirement {
    fn validate(&self) -> Result<(), OptioneeringError> {
        require_non_empty("performance_metric_id", &self.metric_id)?;
        require_non_empty("performance_unit_symbol", &self.unit_symbol)?;
        if self.scale == 0 {
            return Err(OptioneeringError::ZeroPerformanceScale(self.metric_id.clone()));
        }
        if self.minimum_value.is_none() && self.maximum_value.is_none() {
            return Err(OptioneeringError::UnboundedPerformanceRequirement(
                self.metric_id.clone(),
            ));
        }
        if let (Some(minimum), Some(maximum)) = (self.minimum_value, self.maximum_value) {
            if minimum > maximum {
                return Err(OptioneeringError::InvalidPerformanceRange(
                    self.metric_id.clone(),
                ));
            }
        }
        Ok(())
    }

    fn accepts(&self, value: i64) -> bool {
        self.minimum_value.is_none_or(|minimum| value >= minimum)
            && self.maximum_value.is_none_or(|maximum| value <= maximum)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionalOutcomeContract {
    pub contract_id: String,
    /// Caller-supplied immutable reference; this crate does not calculate/authenticate it.
    pub contract_digest: String,
    pub project_context_digest: String,
    pub unit_class_id: String,
    pub unit_symbol: String,
    pub required_units: u64,
    pub acceptance_contract_ref: String,
    pub currency_code: String,
    pub carbon_assessment_boundary_ref: String,
    pub assessment_horizon_months: u64,
    pub required_gate_ids: Vec<String>,
    pub performance_requirements: Vec<PerformanceRequirement>,
}

impl FunctionalOutcomeContract {
    fn validate(&self) -> Result<(), OptioneeringError> {
        require_non_empty("contract_id", &self.contract_id)?;
        require_non_empty("contract_digest", &self.contract_digest)?;
        require_non_empty("project_context_digest", &self.project_context_digest)?;
        require_non_empty("unit_class_id", &self.unit_class_id)?;
        require_non_empty("unit_symbol", &self.unit_symbol)?;
        require_non_empty("acceptance_contract_ref", &self.acceptance_contract_ref)?;
        require_non_empty("currency_code", &self.currency_code)?;
        require_non_empty(
            "carbon_assessment_boundary_ref",
            &self.carbon_assessment_boundary_ref,
        )?;
        if self.required_units == 0 {
            return Err(OptioneeringError::ZeroRequiredUnits);
        }
        if self.assessment_horizon_months == 0 {
            return Err(OptioneeringError::ZeroAssessmentHorizon);
        }

        let mut gates = BTreeSet::new();
        for gate_id in &self.required_gate_ids {
            require_non_empty("required_gate_id", gate_id)?;
            if !gates.insert(gate_id.as_str()) {
                return Err(OptioneeringError::DuplicateRequiredGate(gate_id.clone()));
            }
        }
        let mut metrics = BTreeSet::new();
        for requirement in &self.performance_requirements {
            requirement.validate()?;
            if !metrics.insert(requirement.metric_id.as_str()) {
                return Err(OptioneeringError::DuplicatePerformanceMetric(
                    requirement.metric_id.clone(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceDisposition {
    Qualified {
        authority_ref: String,
        receipt_ref: String,
    },
    Rejected {
        authority_ref: String,
        finding_ref: String,
    },
    Unknown {
        reason_ref: String,
    },
}

impl EvidenceDisposition {
    fn validate(&self) -> Result<(), OptioneeringError> {
        match self {
            Self::Qualified {
                authority_ref,
                receipt_ref,
            } => {
                require_non_empty("authority_ref", authority_ref)?;
                require_non_empty("receipt_ref", receipt_ref)
            }
            Self::Rejected {
                authority_ref,
                finding_ref,
            } => {
                require_non_empty("authority_ref", authority_ref)?;
                require_non_empty("finding_ref", finding_ref)
            }
            Self::Unknown { reason_ref } => require_non_empty("unknown_reason_ref", reason_ref),
        }
    }

    fn is_qualified(&self) -> bool {
        matches!(self, Self::Qualified { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageDisposition {
    /// The stage has been assessed and candidates at this tier may be compared.
    OpenForEvaluation,
    /// Assessment found no viable option in this tier; evidence is mandatory to advance.
    ClosedNoViableAlternative,
    /// This tier cannot apply to the declared need; evidence is mandatory to advance.
    NotApplicable,
    /// Information is insufficient; the hierarchy must stop here.
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageGateDecision {
    pub tier: OptionTier,
    pub disposition: StageDisposition,
    pub accountable_owner_ref: String,
    pub project_context_digest: String,
    pub evidence_refs: Vec<String>,
    pub rationale: String,
}

impl StageGateDecision {
    fn validate(
        &self,
        contract: &FunctionalOutcomeContract,
    ) -> Result<(), OptioneeringError> {
        require_non_empty("stage_owner_ref", &self.accountable_owner_ref)?;
        require_non_empty("stage_rationale", &self.rationale)?;
        if self.project_context_digest != contract.project_context_digest {
            return Err(OptioneeringError::StaleProjectContext(self.tier));
        }
        if self.evidence_refs.is_empty() {
            return Err(OptioneeringError::MissingStageEvidence(self.tier));
        }
        for evidence_ref in &self.evidence_refs {
            require_non_empty("stage_evidence_ref", evidence_ref)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioCase {
    pub scenario_id: String,
}

impl ScenarioCase {
    fn validate(&self) -> Result<(), OptioneeringError> {
        require_non_empty("scenario_id", &self.scenario_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub option_design_digest: String,
    pub assessment_horizon_months: u64,
    /// Independent acceptance/inspection receipt; executor completion is not sufficient.
    pub acceptance_contract_digest: String,
    pub accepted_option_design_digest: String,
    pub acceptance: EvidenceDisposition,
    /// Required hard gates such as design disposition or authority sign-off.
    pub hard_gate_results: BTreeMap<String, EvidenceDisposition>,
    pub accepted_units: u64,
    /// Values use each metric's fixed-point scale in the functional contract.
    pub performance_values: BTreeMap<String, i64>,
    pub direct_cost_minor_units: u64,
    pub shared_cost_minor_units: u64,
    pub maintenance_cost_minor_units: u64,
    pub repair_cost_minor_units: u64,
    pub end_of_life_cost_minor_units: u64,
    pub elapsed_minutes: u64,
    pub workflow_crew_minutes: u64,
    pub energy_wh: u64,
    pub carbon_gco2e: u64,
    /// Material mass in thousandths of a kilogram to preserve integer arithmetic.
    pub material_mass_kg_milli: u64,
    pub rework_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionCandidate {
    pub option_id: String,
    pub tier: OptionTier,
    pub functional_contract_id: String,
    pub functional_contract_digest: String,
    /// Each alternative may have a different design/method revision.
    pub option_design_digest: String,
    pub scenario_results: BTreeMap<String, ScenarioResult>,
}

impl OptionCandidate {
    fn validate_identity(
        &self,
        contract: &FunctionalOutcomeContract,
        expected_scenarios: &BTreeSet<&str>,
    ) -> Result<(), OptioneeringError> {
        require_non_empty("option_id", &self.option_id)?;
        require_non_empty("option_design_digest", &self.option_design_digest)?;
        if self.functional_contract_id != contract.contract_id
            || self.functional_contract_digest != contract.contract_digest
        {
            return Err(OptioneeringError::FunctionalContractMismatch(
                self.option_id.clone(),
            ));
        }
        let actual_scenarios: BTreeSet<&str> =
            self.scenario_results.keys().map(String::as_str).collect();
        if &actual_scenarios != expected_scenarios {
            return Err(OptioneeringError::ScenarioSetMismatch(
                self.option_id.clone(),
            ));
        }
        for (scenario_id, result) in &self.scenario_results {
            if result.scenario_id != *scenario_id {
                return Err(OptioneeringError::ScenarioIdentityMismatch(
                    self.option_id.clone(),
                    scenario_id.clone(),
                ));
            }
            if result.option_design_digest != self.option_design_digest
                || result.accepted_option_design_digest != self.option_design_digest
            {
                return Err(OptioneeringError::OptionDesignMismatch(
                    self.option_id.clone(),
                    scenario_id.clone(),
                ));
            }
            if result.acceptance_contract_digest != contract.contract_digest {
                return Err(OptioneeringError::AcceptanceContractMismatch(
                    self.option_id.clone(),
                    scenario_id.clone(),
                ));
            }
            if result.assessment_horizon_months != contract.assessment_horizon_months {
                return Err(OptioneeringError::AssessmentHorizonMismatch(
                    self.option_id.clone(),
                    scenario_id.clone(),
                ));
            }
            result.acceptance.validate()?;
            for evidence in result.hard_gate_results.values() {
                evidence.validate()?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RobustObjectives {
    /// Direct + shared + maintenance + repair + end-of-life cost over the same horizon.
    pub worst_lifecycle_cost_minor_units: u64,
    pub worst_direct_cost_minor_units: u64,
    pub worst_shared_cost_minor_units: u64,
    pub worst_maintenance_cost_minor_units: u64,
    pub worst_repair_cost_minor_units: u64,
    pub worst_end_of_life_cost_minor_units: u64,
    pub worst_elapsed_minutes: u64,
    pub worst_workflow_crew_minutes: u64,
    pub worst_energy_wh: u64,
    pub worst_carbon_gco2e: u64,
    pub worst_material_mass_kg_milli: u64,
    pub worst_rework_units: u64,
}

impl RobustObjectives {
    fn from_scenarios(
        results: &BTreeMap<String, ScenarioResult>,
    ) -> Result<Self, OptioneeringError> {
        let mut robust = Self {
            worst_lifecycle_cost_minor_units: 0,
            worst_direct_cost_minor_units: 0,
            worst_shared_cost_minor_units: 0,
            worst_maintenance_cost_minor_units: 0,
            worst_repair_cost_minor_units: 0,
            worst_end_of_life_cost_minor_units: 0,
            worst_elapsed_minutes: 0,
            worst_workflow_crew_minutes: 0,
            worst_energy_wh: 0,
            worst_carbon_gco2e: 0,
            worst_material_mass_kg_milli: 0,
            worst_rework_units: 0,
        };

        for result in results.values() {
            let lifecycle_cost = result
                .direct_cost_minor_units
                .checked_add(result.shared_cost_minor_units)
                .and_then(|value| value.checked_add(result.maintenance_cost_minor_units))
                .and_then(|value| value.checked_add(result.repair_cost_minor_units))
                .and_then(|value| value.checked_add(result.end_of_life_cost_minor_units))
                .ok_or_else(|| OptioneeringError::ArithmeticOverflow(result.scenario_id.clone()))?;

            robust.worst_lifecycle_cost_minor_units =
                robust.worst_lifecycle_cost_minor_units.max(lifecycle_cost);
            robust.worst_direct_cost_minor_units =
                robust.worst_direct_cost_minor_units.max(result.direct_cost_minor_units);
            robust.worst_shared_cost_minor_units =
                robust.worst_shared_cost_minor_units.max(result.shared_cost_minor_units);
            robust.worst_maintenance_cost_minor_units = robust
                .worst_maintenance_cost_minor_units
                .max(result.maintenance_cost_minor_units);
            robust.worst_repair_cost_minor_units =
                robust.worst_repair_cost_minor_units.max(result.repair_cost_minor_units);
            robust.worst_end_of_life_cost_minor_units =
                robust.worst_end_of_life_cost_minor_units.max(result.end_of_life_cost_minor_units);
            robust.worst_elapsed_minutes =
                robust.worst_elapsed_minutes.max(result.elapsed_minutes);
            robust.worst_workflow_crew_minutes = robust
                .worst_workflow_crew_minutes
                .max(result.workflow_crew_minutes);
            robust.worst_energy_wh = robust.worst_energy_wh.max(result.energy_wh);
            robust.worst_carbon_gco2e =
                robust.worst_carbon_gco2e.max(result.carbon_gco2e);
            robust.worst_material_mass_kg_milli = robust
                .worst_material_mass_kg_milli
                .max(result.material_mass_kg_milli);
            robust.worst_rework_units = robust.worst_rework_units.max(result.rework_units);
        }

        Ok(robust)
    }

    fn dominates(&self, other: &Self) -> bool {
        let no_worse = self.worst_lifecycle_cost_minor_units
            <= other.worst_lifecycle_cost_minor_units
            && self.worst_elapsed_minutes <= other.worst_elapsed_minutes
            && self.worst_workflow_crew_minutes <= other.worst_workflow_crew_minutes
            && self.worst_energy_wh <= other.worst_energy_wh
            && self.worst_carbon_gco2e <= other.worst_carbon_gco2e
            && self.worst_material_mass_kg_milli <= other.worst_material_mass_kg_milli
            && self.worst_rework_units <= other.worst_rework_units;
        let strictly_better = self.worst_lifecycle_cost_minor_units
            < other.worst_lifecycle_cost_minor_units
            || self.worst_elapsed_minutes < other.worst_elapsed_minutes
            || self.worst_workflow_crew_minutes < other.worst_workflow_crew_minutes
            || self.worst_energy_wh < other.worst_energy_wh
            || self.worst_carbon_gco2e < other.worst_carbon_gco2e
            || self.worst_material_mass_kg_milli < other.worst_material_mass_kg_milli
            || self.worst_rework_units < other.worst_rework_units;
        no_worse && strictly_better
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScenarioFailure {
    AcceptanceNotQualified,
    InsufficientAcceptedUnits { required: u64, actual: u64 },
    MissingHardGateResult(String),
    HardGateNotQualified(String),
    MissingPerformanceMetric(String),
    PerformanceBelowMinimum { metric_id: String, minimum: i64, actual: i64 },
    PerformanceAboveMaximum { metric_id: String, maximum: i64, actual: i64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioFailureRecord {
    pub scenario_id: String,
    pub failures: Vec<ScenarioFailure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OptionStatus {
    RobustFeasibleFrontier,
    RobustFeasibleDominated { by_option_ids: Vec<String> },
    NotRobustFeasible { failures: Vec<ScenarioFailureRecord> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionEvaluation {
    pub option_id: String,
    pub tier: OptionTier,
    pub robust_objectives: Option<RobustObjectives>,
    pub status: OptionStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportDisposition {
    FrontierComputed,
    NoCandidatesAtOpenStage,
    NoRobustFeasibleCandidates,
    BlockedAtUnknownStage,
    AllStagesClosed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptioneeringReport {
    pub functional_contract_id: String,
    pub functional_contract_digest: String,
    pub unit_class_id: String,
    pub unit_symbol: String,
    pub required_units: u64,
    pub assessment_horizon_months: u64,
    pub evaluated_tier: Option<OptionTier>,
    pub disposition: ReportDisposition,
    pub evaluations: Vec<OptionEvaluation>,
    pub frontier_option_ids: Vec<String>,
    /// Candidates at later tiers are intentionally not evaluated until the current
    /// stage has an evidence-bearing close disposition.
    pub deferred_option_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OptioneeringError {
    EmptyField(&'static str),
    ZeroRequiredUnits,
    ZeroAssessmentHorizon,
    UnboundedPerformanceRequirement(String),
    ZeroPerformanceScale(String),
    InvalidPerformanceRange(String),
    DuplicateRequiredGate(String),
    DuplicatePerformanceMetric(String),
    MissingStageDecision(OptionTier),
    DuplicateStageDecision(OptionTier),
    StageOrderInvalid,
    MissingStageEvidence(OptionTier),
    StaleProjectContext(OptionTier),
    DuplicateScenario(String),
    NoScenarios,
    DuplicateOption(String),
    FunctionalContractMismatch(String),
    ScenarioSetMismatch(String),
    ScenarioIdentityMismatch(String, String),
    OptionDesignMismatch(String, String),
    AcceptanceContractMismatch(String, String),
    AssessmentHorizonMismatch(String, String),
    ArithmeticOverflow(String),
}

impl OptioneeringError {
    fn scenario_failures(
        contract: &FunctionalOutcomeContract,
        result: &ScenarioResult,
    ) -> Vec<ScenarioFailure> {
        let mut failures = Vec::new();
        if !result.acceptance.is_qualified() {
            failures.push(ScenarioFailure::AcceptanceNotQualified);
        }
        if result.accepted_units < contract.required_units {
            failures.push(ScenarioFailure::InsufficientAcceptedUnits {
                required: contract.required_units,
                actual: result.accepted_units,
            });
        }

        for gate_id in &contract.required_gate_ids {
            match result.hard_gate_results.get(gate_id) {
                None => failures.push(ScenarioFailure::MissingHardGateResult(gate_id.clone())),
                Some(evidence) if !evidence.is_qualified() => {
                    failures.push(ScenarioFailure::HardGateNotQualified(gate_id.clone()));
                }
                Some(_) => {}
            }
        }

        for requirement in &contract.performance_requirements {
            match result.performance_values.get(&requirement.metric_id) {
                None => failures.push(ScenarioFailure::MissingPerformanceMetric(
                    requirement.metric_id.clone(),
                )),
                Some(actual) => {
                    if let Some(minimum) = requirement.minimum_value {
                        if actual < &minimum {
                            failures.push(ScenarioFailure::PerformanceBelowMinimum {
                                metric_id: requirement.metric_id.clone(),
                                minimum,
                                actual: *actual,
                            });
                        }
                    }
                    if let Some(maximum) = requirement.maximum_value {
                        if actual > &maximum {
                            failures.push(ScenarioFailure::PerformanceAboveMaximum {
                                metric_id: requirement.metric_id.clone(),
                                maximum,
                                actual: *actual,
                            });
                        }
                    }
                }
            }
        }

        failures
    }
}

pub fn evaluate_option_set(
    contract: &FunctionalOutcomeContract,
    stage_decisions: &[StageGateDecision],
    scenarios: &[ScenarioCase],
    options: &[OptionCandidate],
) -> Result<OptioneeringReport, OptioneeringError> {
    contract.validate()?;
    if scenarios.is_empty() {
        return Err(OptioneeringError::NoScenarios);
    }

    let mut scenario_ids = BTreeSet::new();
    for scenario in scenarios {
        scenario.validate()?;
        if !scenario_ids.insert(scenario.scenario_id.as_str()) {
            return Err(OptioneeringError::DuplicateScenario(
                scenario.scenario_id.clone(),
            ));
        }
    }
    let expected_scenarios = scenario_ids;

    let mut option_ids = BTreeSet::new();
    for option in options {
        if !option_ids.insert(option.option_id.as_str()) {
            return Err(OptioneeringError::DuplicateOption(option.option_id.clone()));
        }
        option.validate_identity(contract, &expected_scenarios)?;
    }

    let mut decisions_by_tier = BTreeMap::new();
    for (index, decision) in stage_decisions.iter().enumerate() {
        if OptionTier::ORDERED.get(index).copied() != Some(decision.tier) {
            return Err(OptioneeringError::StageOrderInvalid);
        }
        decision.validate(contract)?;
        if decisions_by_tier.insert(decision.tier, decision).is_some() {
            return Err(OptioneeringError::DuplicateStageDecision(decision.tier));
        }
    }

    let mut evaluated_tier = None;
    let mut disposition = ReportDisposition::AllStagesClosed;
    for tier in OptionTier::ORDERED {
        let decision = decisions_by_tier
            .get(&tier)
            .ok_or(OptioneeringError::MissingStageDecision(tier))?;
        match decision.disposition {
            StageDisposition::ClosedNoViableAlternative | StageDisposition::NotApplicable => {
                continue;
            }
            StageDisposition::Unknown => {
                disposition = ReportDisposition::BlockedAtUnknownStage;
                evaluated_tier = Some(tier);
                break;
            }
            StageDisposition::OpenForEvaluation => {
                evaluated_tier = Some(tier);
                let mut tier_options: Vec<&OptionCandidate> =
                    options.iter().filter(|option| option.tier == tier).collect();
                tier_options.sort_by(|left, right| left.option_id.cmp(&right.option_id));
                if tier_options.is_empty() {
                    disposition = ReportDisposition::NoCandidatesAtOpenStage;
                } else {
                    disposition = ReportDisposition::FrontierComputed;
                }
                break;
            }
        }
    }

    let Some(tier) = evaluated_tier else {
        let mut deferred_option_ids: Vec<String> = options
            .iter()
            .map(|option| option.option_id.clone())
            .collect();
        deferred_option_ids.sort();
        return Ok(OptioneeringReport {
            functional_contract_id: contract.contract_id.clone(),
            functional_contract_digest: contract.contract_digest.clone(),
            unit_class_id: contract.unit_class_id.clone(),
            unit_symbol: contract.unit_symbol.clone(),
            required_units: contract.required_units,
            assessment_horizon_months: contract.assessment_horizon_months,
            evaluated_tier: None,
            disposition,
            evaluations: Vec::new(),
            frontier_option_ids: Vec::new(),
            deferred_option_ids,
        });
    };

    if disposition == ReportDisposition::BlockedAtUnknownStage
        || disposition == ReportDisposition::NoCandidatesAtOpenStage
    {
        return Ok(OptioneeringReport {
            functional_contract_id: contract.contract_id.clone(),
            functional_contract_digest: contract.contract_digest.clone(),
            unit_class_id: contract.unit_class_id.clone(),
            unit_symbol: contract.unit_symbol.clone(),
            required_units: contract.required_units,
            assessment_horizon_months: contract.assessment_horizon_months,
            evaluated_tier: Some(tier),
            disposition,
            evaluations: Vec::new(),
            frontier_option_ids: Vec::new(),
            deferred_option_ids: sorted_option_ids(options, tier),
        });
    }

    let mut evaluations = Vec::new();
    for option in options.iter().filter(|option| option.tier == tier) {
        let mut scenario_failures = Vec::new();
        for scenario in scenarios {
            let result = option
                .scenario_results
                .get(&scenario.scenario_id)
                .expect("scenario-set identity was validated");
            let failures = OptioneeringError::scenario_failures(contract, result);
            if !failures.is_empty() {
                scenario_failures.push(ScenarioFailureRecord {
                    scenario_id: scenario.scenario_id.clone(),
                    failures,
                });
            }
        }

        if scenario_failures.is_empty() {
            let robust_objectives =
                RobustObjectives::from_scenarios(&option.scenario_results)?;
            evaluations.push(OptionEvaluation {
                option_id: option.option_id.clone(),
                tier: option.tier,
                robust_objectives: Some(robust_objectives),
                status: OptionStatus::RobustFeasibleFrontier,
            });
        } else {
            evaluations.push(OptionEvaluation {
                option_id: option.option_id.clone(),
                tier: option.tier,
                robust_objectives: None,
                status: OptionStatus::NotRobustFeasible {
                    failures: scenario_failures,
                },
            });
        }
    }

    let feasible: Vec<usize> = evaluations
        .iter()
        .enumerate()
        .filter_map(|(index, evaluation)| {
            evaluation.robust_objectives.as_ref().map(|_| index)
        })
        .collect();

    let mut frontier_option_ids = Vec::new();
    for index in &feasible {
        let objective = evaluations[*index]
            .robust_objectives
            .as_ref()
            .expect("feasible option has objectives");
        let dominators: Vec<String> = feasible
            .iter()
            .filter(|other_index| **other_index != *index)
            .filter_map(|other_index| {
                let other = evaluations[*other_index]
                    .robust_objectives
                    .as_ref()
                    .expect("feasible option has objectives");
                other
                    .dominates(objective)
                    .then(|| evaluations[*other_index].option_id.clone())
            })
            .collect();

        if dominators.is_empty() {
            frontier_option_ids.push(evaluations[*index].option_id.clone());
            evaluations[*index].status = OptionStatus::RobustFeasibleFrontier;
        } else {
            evaluations[*index].status = OptionStatus::RobustFeasibleDominated {
                by_option_ids: dominators,
            };
        }
    }

    frontier_option_ids.sort();
    if frontier_option_ids.is_empty() {
        disposition = ReportDisposition::NoRobustFeasibleCandidates;
    }

    evaluations.sort_by(|left, right| left.option_id.cmp(&right.option_id));
    Ok(OptioneeringReport {
        functional_contract_id: contract.contract_id.clone(),
        functional_contract_digest: contract.contract_digest.clone(),
        unit_class_id: contract.unit_class_id.clone(),
        unit_symbol: contract.unit_symbol.clone(),
        required_units: contract.required_units,
        assessment_horizon_months: contract.assessment_horizon_months,
        evaluated_tier: Some(tier),
        disposition,
        evaluations,
        frontier_option_ids,
        deferred_option_ids: sorted_option_ids(options, tier),
    })
}

fn sorted_option_ids(options: &[OptionCandidate], current_tier: OptionTier) -> Vec<String> {
    let mut ids: Vec<String> = options
        .iter()
        .filter(|option| option.tier > current_tier)
        .map(|option| option.option_id.clone())
        .collect();
    ids.sort();
    ids
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), OptioneeringError> {
    if value.trim().is_empty() {
        return Err(OptioneeringError::EmptyField(name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> FunctionalOutcomeContract {
        FunctionalOutcomeContract {
            contract_id: "partition-accepted-m2-v1".to_owned(),
            contract_digest: "git-blob-sha1:partition-contract".to_owned(),
            project_context_digest: "git-blob-sha1:project-need-context".to_owned(),
            unit_class_id: "non-loadbearing-partition".to_owned(),
            unit_symbol: "m2".to_owned(),
            required_units: 100,
            acceptance_contract_ref: "acceptance:partition-fire-acoustic-interface".to_owned(),
            currency_code: "USD".to_owned(),
            carbon_assessment_boundary_ref: "lca-boundary:cradle-to-end-of-life-v1".to_owned(),
            assessment_horizon_months: 240,
            required_gate_ids: vec!["design-authority".to_owned(), "fire-performance".to_owned()],
            performance_requirements: vec![
                PerformanceRequirement {
                    metric_id: "fire-rating-minutes".to_owned(),
                    unit_symbol: "min",
                    scale: 1,
                    minimum_value: Some(60),
                    maximum_value: None,
                },
                PerformanceRequirement {
                    metric_id: "acoustic-insulation".to_owned(),
                    unit_symbol: "dB",
                    scale: 1,
                    minimum_value: Some(45),
                    maximum_value: None,
                },
            ],
        }
    }

    fn qualified(ref_id: &str) -> EvidenceDisposition {
        EvidenceDisposition::Qualified {
            authority_ref: format!("authority:{ref_id}"),
            receipt_ref: format!("receipt:{ref_id}"),
        }
    }

    fn result(
        scenario_id: &str,
        design: &str,
        direct: u64,
        shared: u64,
        maintenance: u64,
        elapsed: u64,
        crew: u64,
        energy: u64,
        carbon: u64,
        rework: u64,
    ) -> ScenarioResult {
        ScenarioResult {
            scenario_id: scenario_id.to_owned(),
            option_design_digest: design.to_owned(),
            assessment_horizon_months: 240,
            acceptance_contract_digest: contract().contract_digest,
            accepted_option_design_digest: design.to_owned(),
            acceptance: qualified(&format!("acceptance-{scenario_id}")),
            hard_gate_results: BTreeMap::from([
                ("design-authority".to_owned(), qualified("design")),
                ("fire-performance".to_owned(), qualified("fire")),
            ]),
            accepted_units: 100,
            performance_values: BTreeMap::from([
                ("fire-rating-minutes".to_owned(), 90),
                ("acoustic-insulation".to_owned(), 50),
            ]),
            direct_cost_minor_units: direct,
            shared_cost_minor_units: shared,
            maintenance_cost_minor_units: maintenance,
            repair_cost_minor_units: 0,
            end_of_life_cost_minor_units: 0,
            elapsed_minutes: elapsed,
            workflow_crew_minutes: crew,
            energy_wh: energy,
            carbon_gco2e: carbon,
            material_mass_kg_milli: 150_000,
            rework_units: rework,
        }
    }

    fn option(
        id: &str,
        tier: OptionTier,
        design: &str,
        scenarios: Vec<ScenarioResult>,
    ) -> OptionCandidate {
        OptionCandidate {
            option_id: id.to_owned(),
            tier,
            functional_contract_id: contract().contract_id,
            functional_contract_digest: contract().contract_digest,
            option_design_digest: design.to_owned(),
            scenario_results: scenarios
                .into_iter()
                .map(|scenario| (scenario.scenario_id.clone(), scenario))
                .collect(),
        }
    }

    fn stage(tier: OptionTier, disposition: StageDisposition) -> StageGateDecision {
        StageGateDecision {
            tier,
            disposition,
            accountable_owner_ref: format!("decision-owner:{tier:?}"),
            project_context_digest: contract().project_context_digest,
            evidence_refs: vec![format!("stage-evidence:{tier:?}")],
            rationale: format!("explicit {tier:?} stage decision"),
        }
    }

    fn scenarios() -> Vec<ScenarioCase> {
        vec![
            ScenarioCase {
                scenario_id: "nominal".to_owned(),
            },
            ScenarioCase {
                scenario_id: "outage-rework".to_owned(),
            },
        ]
    }

    #[test]
    fn avoids_stage_is_evaluated_before_switch_and_improve() {
        let options = vec![
            option(
                "reuse-existing",
                OptionTier::Avoid,
                "design-reuse",
                vec![
                    result("nominal", "design-reuse", 7_000, 500, 100, 400, 300, 1_000, 600, 0),
                    result(
                        "outage-rework",
                        "design-reuse",
                        8_000,
                        500,
                        100,
                        450,
                        320,
                        1_100,
                        650,
                        1,
                    ),
                ],
            ),
            option(
                "prefabricated-panel",
                OptionTier::Switch,
                "design-prefab",
                vec![
                    result("nominal", "design-prefab", 9_000, 300, 50, 350, 260, 900, 500, 0),
                    result(
                        "outage-rework",
                        "design-prefab",
                        10_000,
                        300,
                        50,
                        420,
                        280,
                        950,
                        520,
                        1,
                    ),
                ],
            ),
            option(
                "robot-improved-cip",
                OptionTier::Improve,
                "design-cip-robot",
                vec![
                    result("nominal", "design-cip-robot", 10_000, 400, 80, 320, 240, 850, 550, 0),
                    result(
                        "outage-rework",
                        "design-cip-robot",
                        11_000,
                        400,
                        80,
                        480,
                        300,
                        1_000,
                        600,
                        2,
                    ),
                ],
            ),
        ];
        let report = evaluate_option_set(
            &contract(),
            &[stage(
                OptionTier::Avoid,
                StageDisposition::OpenForEvaluation,
            )],
            &scenarios(),
            &options,
        )
        .expect("avoid options evaluate");

        assert_eq!(report.evaluated_tier, Some(OptionTier::Avoid));
        assert_eq!(report.frontier_option_ids, vec!["reuse-existing"]);
        assert_eq!(report.deferred_option_ids, vec!["prefabricated-panel", "robot-improved-cip"]);
    }

    #[test]
    fn switch_stage_requires_evidence_bearing_avoid_closure() {
        let options = vec![option(
            "prefabricated-panel",
            OptionTier::Switch,
            "design-prefab",
            vec![
                result("nominal", "design-prefab", 9_000, 300, 50, 350, 260, 900, 500, 0),
                result("outage-rework", "design-prefab", 10_000, 300, 50, 420, 280, 950, 520, 1),
            ],
        )];
        let report = evaluate_option_set(
            &contract(),
            &[
                stage(
                    OptionTier::Avoid,
                    StageDisposition::ClosedNoViableAlternative,
                ),
                stage(
                    OptionTier::Switch,
                    StageDisposition::OpenForEvaluation,
                ),
            ],
            &scenarios(),
            &options,
        )
        .expect("switch options evaluate");

        assert_eq!(report.evaluated_tier, Some(OptionTier::Switch));
        assert_eq!(report.frontier_option_ids, vec!["prefabricated-panel"]);
    }

    #[test]
    fn unknown_avoid_stage_blocks_switch_and_improve() {
        let options = vec![option(
            "prefabricated-panel",
            OptionTier::Switch,
            "design-prefab",
            vec![
                result("nominal", "design-prefab", 9_000, 300, 50, 350, 260, 900, 500, 0),
                result("outage-rework", "design-prefab", 10_000, 300, 50, 420, 280, 950, 520, 1),
            ],
        )];
        let report = evaluate_option_set(
            &contract(),
            &[stage(OptionTier::Avoid, StageDisposition::Unknown)],
            &scenarios(),
            &options,
        )
        .expect("unknown stage is a report");

        assert_eq!(report.disposition, ReportDisposition::BlockedAtUnknownStage);
        assert_eq!(report.evaluations.len(), 0);
        assert_eq!(report.deferred_option_ids, vec!["prefabricated-panel"]);
    }

    #[test]
    fn closed_stage_requires_a_nonempty_evidence_reference() {
        let mut decision = stage(
            OptionTier::Avoid,
            StageDisposition::ClosedNoViableAlternative,
        );
        decision.evidence_refs.clear();
        assert_eq!(
            evaluate_option_set(&contract(), &[decision], &scenarios(), &[]),
            Err(OptioneeringError::MissingStageEvidence(OptionTier::Avoid))
        );
    }

    #[test]
    fn every_scenario_must_pass_independent_acceptance() {
        let mut rejected = result(
            "outage-rework",
            "design-reuse",
            8_000,
            500,
            100,
            450,
            320,
            1_100,
            650,
            1,
        );
        rejected.acceptance = EvidenceDisposition::Unknown {
            reason_ref: "unknown:inspection-not-returned".to_owned(),
        };
        let options = vec![option(
            "reuse-existing",
            OptionTier::Avoid,
            "design-reuse",
            vec![
                result("nominal", "design-reuse", 7_000, 500, 100, 400, 300, 1_000, 600, 0),
                rejected,
            ],
        )];
        let report = evaluate_option_set(
            &contract(),
            &[stage(OptionTier::Avoid, StageDisposition::OpenForEvaluation)],
            &scenarios(),
            &options,
        )
        .expect("scenario failure is a report");
        assert_eq!(
            report.disposition,
            ReportDisposition::NoRobustFeasibleCandidates
        );
        assert!(matches!(
            report.evaluations[0].status,
            OptionStatus::NotRobustFeasible { .. }
        ));
    }

    #[test]
    fn one_missing_hard_gate_disqualifies_robust_option() {
        let mut failed = result(
            "outage-rework",
            "design-reuse",
            8_000,
            500,
            100,
            450,
            320,
            1_100,
            650,
            1,
        );
        failed.hard_gate_results.remove("fire-performance");
        let options = vec![option(
            "reuse-existing",
            OptionTier::Avoid,
            "design-reuse",
            vec![
                result("nominal", "design-reuse", 7_000, 500, 100, 400, 300, 1_000, 600, 0),
                failed,
            ],
        )];
        let report = evaluate_option_set(
            &contract(),
            &[stage(OptionTier::Avoid, StageDisposition::OpenForEvaluation)],
            &scenarios(),
            &options,
        )
        .expect("missing gate is an exclusion");
        assert!(matches!(
            &report.evaluations[0].status,
            OptionStatus::NotRobustFeasible { failures }
                if failures[0].failures.contains(&ScenarioFailure::MissingHardGateResult("fire-performance".to_owned()))
        ));
    }

    #[test]
    fn performance_requirements_are_hard_bounds_not_weighted_scores() {
        let mut bad = result(
            "outage-rework",
            "design-reuse",
            8_000,
            500,
            100,
            450,
            320,
            1_100,
            650,
            1,
        );
        bad.performance_values
            .insert("acoustic-insulation".to_owned(), 40);
        let options = vec![option(
            "reuse-existing",
            OptionTier::Avoid,
            "design-reuse",
            vec![
                result("nominal", "design-reuse", 7_000, 500, 100, 400, 300, 1_000, 600, 0),
                bad,
            ],
        )];
        let report = evaluate_option_set(
            &contract(),
            &[stage(OptionTier::Avoid, StageDisposition::OpenForEvaluation)],
            &scenarios(),
            &options,
        )
        .expect("performance failure is visible");
        assert!(matches!(
            &report.evaluations[0].status,
            OptionStatus::NotRobustFeasible { failures }
                if failures[0].failures.contains(&ScenarioFailure::PerformanceBelowMinimum {
                    metric_id: "acoustic-insulation".to_owned(),
                    minimum: 45,
                    actual: 40
                })
        ));
    }

    #[test]
    fn pareto_frontier_preserves_cost_time_tradeoffs() {
        let options = vec![
            option(
                "cheap-slow",
                OptionTier::Switch,
                "design-cheap",
                vec![
                    result("nominal", "design-cheap", 5_000, 100, 0, 600, 500, 900, 400, 0),
                    result("outage-rework", "design-cheap", 5_500, 100, 0, 650, 520, 950, 420, 1),
                ],
            ),
            option(
                "expensive-fast",
                OptionTier::Switch,
                "design-fast",
                vec![
                    result("nominal", "design-fast", 9_000, 200, 0, 300, 250, 900, 500, 0),
                    result("outage-rework", "design-fast", 9_500, 200, 0, 320, 270, 950, 520, 1),
                ],
            ),
            option(
                "dominated",
                OptionTier::Switch,
                "design-dominated",
                vec![
                    result("nominal", "design-dominated", 10_000, 300, 50, 700, 600, 1_200, 700, 2),
                    result("outage-rework", "design-dominated", 11_000, 300, 50, 750, 620, 1_250, 720, 3),
                ],
            ),
        ];
        let report = evaluate_option_set(
            &contract(),
            &[
                stage(
                    OptionTier::Avoid,
                    StageDisposition::ClosedNoViableAlternative,
                ),
                stage(OptionTier::Switch, StageDisposition::OpenForEvaluation),
            ],
            &scenarios(),
            &options,
        )
        .expect("pareto options evaluate");

        assert_eq!(
            report.frontier_option_ids,
            vec!["cheap-slow", "expensive-fast"]
        );
        let dominated = report
            .evaluations
            .iter()
            .find(|item| item.option_id == "dominated")
            .expect("dominated option present");
        assert!(matches!(
            &dominated.status,
            OptionStatus::RobustFeasibleDominated { by_option_ids }
                if by_option_ids.contains(&"cheap-slow".to_owned())
                    && by_option_ids.contains(&"expensive-fast".to_owned())
        ));
    }

    #[test]
    fn worst_case_objectives_cover_all_scenario_cost_components() {
        let options = vec![option(
            "reuse-existing",
            OptionTier::Avoid,
            "design-reuse",
            vec![
                result("nominal", "design-reuse", 7_000, 500, 100, 400, 300, 1_000, 600, 0),
                result("outage-rework", "design-reuse", 8_000, 500, 250, 450, 320, 1_100, 650, 1),
            ],
        )];
        let report = evaluate_option_set(
            &contract(),
            &[stage(OptionTier::Avoid, StageDisposition::OpenForEvaluation)],
            &scenarios(),
            &options,
        )
        .expect("cost rollup");
        let objectives = report.evaluations[0]
            .robust_objectives
            .as_ref()
            .expect("robust feasible");
        assert_eq!(objectives.worst_lifecycle_cost_minor_units, 8_750);
        assert_eq!(objectives.worst_shared_cost_minor_units, 500);
        assert_eq!(objectives.worst_maintenance_cost_minor_units, 250);
        assert_eq!(objectives.worst_elapsed_minutes, 450);
        assert_eq!(objectives.worst_workflow_crew_minutes, 320);
    }

    #[test]
    fn scenario_probabilities_are_not_present_in_the_contract() {
        let report = evaluate_option_set(
            &contract(),
            &[stage(OptionTier::Avoid, StageDisposition::OpenForEvaluation)],
            &scenarios(),
            &[option(
                "reuse-existing",
                OptionTier::Avoid,
                "design-reuse",
                vec![
                    result("nominal", "design-reuse", 7_000, 500, 100, 400, 300, 1_000, 600, 0),
                    result("outage-rework", "design-reuse", 8_000, 500, 100, 450, 320, 1_100, 650, 1),
                ],
            )],
        )
        .expect("deterministic robust evaluation");
        assert_eq!(report.frontier_option_ids, vec!["reuse-existing"]);
    }

    #[test]
    fn independent_exhaustive_frontier_oracle_matches_implementation() {
        let vectors = [
            RobustObjectives {
                worst_lifecycle_cost_minor_units: 10,
                worst_direct_cost_minor_units: 10,
                worst_shared_cost_minor_units: 0,
                worst_maintenance_cost_minor_units: 0,
                worst_repair_cost_minor_units: 0,
                worst_end_of_life_cost_minor_units: 0,
                worst_elapsed_minutes: 10,
                worst_workflow_crew_minutes: 10,
                worst_energy_wh: 10,
                worst_carbon_gco2e: 10,
                worst_material_mass_kg_milli: 10,
                worst_rework_units: 0,
            },
            RobustObjectives {
                worst_lifecycle_cost_minor_units: 8,
                worst_direct_cost_minor_units: 8,
                worst_shared_cost_minor_units: 0,
                worst_maintenance_cost_minor_units: 0,
                worst_repair_cost_minor_units: 0,
                worst_end_of_life_cost_minor_units: 0,
                worst_elapsed_minutes: 12,
                worst_workflow_crew_minutes: 8,
                worst_energy_wh: 10,
                worst_carbon_gco2e: 10,
                worst_material_mass_kg_milli: 10,
                worst_rework_units: 0,
            },
            RobustObjectives {
                worst_lifecycle_cost_minor_units: 10,
                worst_direct_cost_minor_units: 10,
                worst_shared_cost_minor_units: 0,
                worst_maintenance_cost_minor_units: 0,
                worst_repair_cost_minor_units: 0,
                worst_end_of_life_cost_minor_units: 0,
                worst_elapsed_minutes: 8,
                worst_workflow_crew_minutes: 8,
                worst_energy_wh: 8,
                worst_carbon_gco2e: 8,
                worst_material_mass_kg_milli: 8,
                worst_rework_units: 0,
            },
            RobustObjectives {
                worst_lifecycle_cost_minor_units: 14,
                worst_direct_cost_minor_units: 14,
                worst_shared_cost_minor_units: 0,
                worst_maintenance_cost_minor_units: 0,
                worst_repair_cost_minor_units: 0,
                worst_end_of_life_cost_minor_units: 0,
                worst_elapsed_minutes: 14,
                worst_workflow_crew_minutes: 14,
                worst_energy_wh: 14,
                worst_carbon_gco2e: 14,
                worst_material_mass_kg_milli: 14,
                worst_rework_units: 4,
            },
        ];

        let oracle: Vec<usize> = (0..vectors.len())
            .filter(|candidate| {
                !(0..vectors.len()).any(|other| {
                    if other == *candidate {
                        return false;
                    }
                    let a = &vectors[other];
                    let b = &vectors[*candidate];
                    a.worst_lifecycle_cost_minor_units <= b.worst_lifecycle_cost_minor_units
                        && a.worst_elapsed_minutes <= b.worst_elapsed_minutes
                        && a.worst_workflow_crew_minutes <= b.worst_workflow_crew_minutes
                        && a.worst_energy_wh <= b.worst_energy_wh
                        && a.worst_carbon_gco2e <= b.worst_carbon_gco2e
                        && a.worst_material_mass_kg_milli <= b.worst_material_mass_kg_milli
                        && a.worst_rework_units <= b.worst_rework_units
                        && (a.worst_lifecycle_cost_minor_units < b.worst_lifecycle_cost_minor_units
                            || a.worst_elapsed_minutes < b.worst_elapsed_minutes
                            || a.worst_workflow_crew_minutes < b.worst_workflow_crew_minutes
                            || a.worst_energy_wh < b.worst_energy_wh
                            || a.worst_carbon_gco2e < b.worst_carbon_gco2e
                            || a.worst_material_mass_kg_milli < b.worst_material_mass_kg_milli
                            || a.worst_rework_units < b.worst_rework_units)
                })
            })
            .collect();

        let actual: Vec<usize> = (0..vectors.len())
            .filter(|candidate| {
                !(0..vectors.len()).any(|other| {
                    other != *candidate && vectors[other].dominates(&vectors[*candidate])
                })
            })
            .collect();
        assert_eq!(actual, oracle);
    }

    #[test]
    fn missing_scenario_result_is_typed_not_assumed_to_pass() {
        let incomplete = option(
            "reuse-existing",
            OptionTier::Avoid,
            "design-reuse",
            vec![result(
                "nominal",
                "design-reuse",
                7_000,
                500,
                100,
                400,
                300,
                1_000,
                600,
                0,
            )],
        );
        assert_eq!(
            evaluate_option_set(
                &contract(),
                &[stage(
                    OptionTier::Avoid,
                    StageDisposition::OpenForEvaluation
                )],
                &scenarios(),
                &[incomplete]
            ),
            Err(OptioneeringError::ScenarioSetMismatch(
                "reuse-existing".to_owned()
            ))
        );
    }

    #[test]
    fn duplicate_option_and_scenario_ids_are_rejected() {
        let candidate = option(
            "same-option",
            OptionTier::Avoid,
            "design-reuse",
            vec![
                result("nominal", "design-reuse", 7_000, 500, 100, 400, 300, 1_000, 600, 0),
                result("outage-rework", "design-reuse", 8_000, 500, 100, 450, 320, 1_100, 650, 1),
            ],
        );
        assert_eq!(
            evaluate_option_set(
                &contract(),
                &[stage(
                    OptionTier::Avoid,
                    StageDisposition::OpenForEvaluation
                )],
                &scenarios(),
                &[candidate.clone(), candidate]
            ),
            Err(OptioneeringError::DuplicateOption(
                "same-option".to_owned()
            ))
        );
        assert_eq!(
            evaluate_option_set(
                &contract(),
                &[stage(
                    OptionTier::Avoid,
                    StageDisposition::OpenForEvaluation
                )],
                &[
                    ScenarioCase {
                        scenario_id: "nominal".to_owned()
                    },
                    ScenarioCase {
                        scenario_id: "nominal".to_owned()
                    }
                ],
                &[]
            ),
            Err(OptioneeringError::DuplicateScenario(
                "nominal".to_owned()
            ))
        );
    }

    #[test]
    fn alternatives_cannot_change_functional_unit_contract() {
        let mut candidate = option(
            "wrong-contract",
            OptionTier::Avoid,
            "design-reuse",
            vec![
                result("nominal", "design-reuse", 7_000, 500, 100, 400, 300, 1_000, 600, 0),
                result("outage-rework", "design-reuse", 8_000, 500, 100, 450, 320, 1_100, 650, 1),
            ],
        );
        candidate.functional_contract_id = "different-unit-class".to_owned();
        assert_eq!(
            evaluate_option_set(
                &contract(),
                &[stage(
                    OptionTier::Avoid,
                    StageDisposition::OpenForEvaluation
                )],
                &scenarios(),
                &[candidate]
            ),
            Err(OptioneeringError::FunctionalContractMismatch(
                "wrong-contract".to_owned()
            ))
        );
    }

    #[test]
    fn design_revisions_may_differ_but_scenario_result_must_bind_exact_option() {
        let mut candidate = option(
            "prefabricated",
            OptionTier::Switch,
            "design-prefab-v2",
            vec![
                result("nominal", "design-prefab-v2", 9_000, 300, 50, 350, 260, 900, 500, 0),
                result(
                    "outage-rework",
                    "design-prefab-v2",
                    10_000,
                    300,
                    50,
                    420,
                    280,
                    950,
                    520,
                    1,
                ),
            ],
        );
        candidate.scenario_results.get_mut("outage-rework").unwrap().option_design_digest =
            "stale-design-prefab-v1".to_owned();
        assert_eq!(
            evaluate_option_set(
                &contract(),
                &[
                    stage(
                        OptionTier::Avoid,
                        StageDisposition::ClosedNoViableAlternative,
                    ),
                    stage(OptionTier::Switch, StageDisposition::OpenForEvaluation),
                ],
                &scenarios(),
                &[candidate]
            ),
            Err(OptioneeringError::OptionDesignMismatch(
                "prefabricated".to_owned(),
                "outage-rework".to_owned()
            ))
        );
    }

    #[test]
    fn acceptance_receipt_must_bind_exact_option_design_and_functional_contract() {
        let mut candidate = option(
            "prefabricated",
            OptionTier::Switch,
            "design-prefab-v2",
            vec![
                result("nominal", "design-prefab-v2", 9_000, 300, 50, 350, 260, 900, 500, 0),
                result(
                    "outage-rework",
                    "design-prefab-v2",
                    10_000,
                    300,
                    50,
                    420,
                    280,
                    950,
                    520,
                    1,
                ),
            ],
        );
        candidate
            .scenario_results
            .get_mut("outage-rework")
            .expect("scenario exists")
            .acceptance_contract_digest = "fixture:stale-contract".to_owned();
        assert_eq!(
            evaluate_option_set(
                &contract(),
                &[
                    stage(
                        OptionTier::Avoid,
                        StageDisposition::ClosedNoViableAlternative,
                    ),
                    stage(OptionTier::Switch, StageDisposition::OpenForEvaluation),
                ],
                &scenarios(),
                &[candidate]
            ),
            Err(OptioneeringError::AcceptanceContractMismatch(
                "prefabricated".to_owned(),
                "outage-rework".to_owned()
            ))
        );
    }

    #[test]
    fn stage_decisions_must_be_a_contiguous_hierarchy_prefix() {
        assert_eq!(
            evaluate_option_set(
                &contract(),
                &[
                    stage(
                        OptionTier::Avoid,
                        StageDisposition::ClosedNoViableAlternative,
                    ),
                    stage(
                        OptionTier::Improve,
                        StageDisposition::OpenForEvaluation,
                    ),
                ],
                &scenarios(),
                &[]
            ),
            Err(OptioneeringError::StageOrderInvalid)
        );
    }

    #[test]
    fn fixed_point_performance_scale_must_be_positive() {
        let mut invalid = contract();
        invalid.performance_requirements[0].scale = 0;
        assert_eq!(
            evaluate_option_set(
                &invalid,
                &[stage(
                    OptionTier::Avoid,
                    StageDisposition::OpenForEvaluation
                )],
                &scenarios(),
                &[]
            ),
            Err(OptioneeringError::ZeroPerformanceScale(
                "fire-rating-minutes".to_owned()
            ))
        );
    }

    #[test]
    fn closed_and_not_applicable_stages_must_be_a_prefix() {
        assert_eq!(
            evaluate_option_set(
                &contract(),
                &[stage(
                    OptionTier::Switch,
                    StageDisposition::ClosedNoViableAlternative
                )],
                &scenarios(),
                &[]
            ),
            Err(OptioneeringError::MissingStageDecision(OptionTier::Avoid))
        );
    }
}
