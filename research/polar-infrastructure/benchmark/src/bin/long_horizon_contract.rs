use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::io;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: String,
    artifact_kind: String,
    contract_id: String,
    status: String,
    evidence_class: String,
    study_id: String,
    evidence_boundary: EvidenceBoundary,
    horizon: Horizon,
    scenario_families: Vec<ScenarioFamily>,
    persistent_state_variables: Vec<String>,
    adaptation_decisions: Vec<AdaptationDecision>,
    comparison_design: ComparisonDesign,
    required_metric_ids: Vec<String>,
    admission_gates: AdmissionGates,
    decision_policy: DecisionPolicy,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceBoundary {
    environmental_trace_admitted: bool,
    physical_model_verified: bool,
    controller_result_measured: bool,
    probability_basis_verified: bool,
    scenario_weights: Option<Vec<f64>>,
    rule: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Horizon {
    start_year: u32,
    end_year_exclusive: u32,
    duration_years: u32,
    planning_resolution: String,
    carry_state_between_events: bool,
    allow_state_reset_between_events: bool,
    preserve_temporal_order: bool,
    allow_silent_gap_imputation: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioFamily {
    scenario_id: String,
    scenario_kind: String,
    climate_regime: String,
    nonstationary: bool,
    environmental_parameters_included: bool,
    source_admission_status: String,
    scenario_weight: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdaptationDecision {
    decision_id: String,
    decision_frequency: String,
    decision_costs_admitted: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ComparisonDesign {
    arms: Vec<ComparisonArm>,
    paired_exogenous_trajectories: bool,
    same_initial_state_per_pair: bool,
    controller_visible_information_must_be_declared: bool,
    matched_information_budget_or_explicit_ablation: bool,
    compute_budget_must_be_recorded: bool,
    locked_holdout_frozen_before_tuning: bool,
    whole_site_year_or_episode_blocks: bool,
    locked_holdout_used_for_tuning: bool,
    reset_state_between_events: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ComparisonArm {
    arm_id: String,
    state_carryover: bool,
    #[serde(default)]
    adaptation_enabled: Option<bool>,
    role: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdmissionGates {
    source_trace_and_usage_terms_admitted: bool,
    climate_scenarios_have_declared_evidence_class: bool,
    state_carryover_semantics_verified: bool,
    plant_model_verified_for_declared_use: bool,
    adaptation_cost_data_admitted_or_cost_metric_disabled: bool,
    paired_information_and_compute_budget_reviewed: bool,
    locked_holdout_frozen: bool,
    negative_controls_passed: bool,
}

impl AdmissionGates {
    fn all_closed(&self) -> bool {
        !self.source_trace_and_usage_terms_admitted
            && !self.climate_scenarios_have_declared_evidence_class
            && !self.state_carryover_semantics_verified
            && !self.plant_model_verified_for_declared_use
            && !self.adaptation_cost_data_admitted_or_cost_metric_disabled
            && !self.paired_information_and_compute_budget_reviewed
            && !self.locked_holdout_frozen
            && !self.negative_controls_passed
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DecisionPolicy {
    result_kind: String,
    single_scalar_score_allowed: bool,
    probability_weighted_risk_allowed_without_verified_basis: bool,
    event_reset_baseline_may_be_called_sota: bool,
    failed_or_missing_service_windows_may_be_dropped: bool,
    cross_site_or_future_climate_claim_requires_independent_holdout: bool,
}

#[derive(Debug, Serialize)]
struct Report {
    report_kind: &'static str,
    schema_version: &'static str,
    contract_id: String,
    contract_valid: bool,
    candidate_for_execution: bool,
    evidence_level: &'static str,
    scenario_family_count: usize,
    nonstationary_scenario_count: usize,
    persistent_state_variable_count: usize,
    required_metric_count: usize,
    admission_gates_closed: bool,
    result_scope: &'static str,
    warnings: Vec<String>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn unique_nonempty(values: &[String], name: &str) -> Result<(), String> {
    if values.iter().any(|v| !nonempty(v)) {
        return Err(format!("{name} contains an empty identifier"));
    }
    if values.iter().collect::<BTreeSet<_>>().len() != values.len() {
        return Err(format!("{name} contains duplicate identifiers"));
    }
    Ok(())
}

fn validate(input: &Input) -> Result<(), String> {
    if input.schema_version != "0.1.0"
        || input.artifact_kind != "symtropy-long-horizon-resilience-contract"
        || input.contract_id != "long_horizon_climate_adaptation_v0"
        || input.status != "proposed_not_executed"
        || input.evidence_class != "test_design_only"
    {
        return Err("only the declared proposed v0.1.0 design contract is accepted".into());
    }
    if !nonempty(&input.study_id) || !nonempty(&input.evidence_boundary.rule) {
        return Err("study identifier and evidence boundary rule must be non-empty".into());
    }

    let h = &input.horizon;
    if h.end_year_exclusive <= h.start_year
        || h.duration_years != h.end_year_exclusive - h.start_year
        || !nonempty(&h.planning_resolution)
    {
        return Err("horizon years, duration and planning resolution are inconsistent".into());
    }
    if !h.carry_state_between_events
        || h.allow_state_reset_between_events
        || !h.preserve_temporal_order
        || h.allow_silent_gap_imputation
        || input.comparison_design.reset_state_between_events
    {
        return Err("long-horizon evaluation requires chronological state carry-over, no reset, and no silent gap imputation".into());
    }

    if input.scenario_families.len() < 3 {
        return Err("at least three distinct scenario families are required".into());
    }
    let scenario_ids: Vec<String> = input.scenario_families.iter().map(|s| s.scenario_id.clone()).collect();
    unique_nonempty(&scenario_ids, "scenario IDs")?;
    let nonstationary_count = input.scenario_families.iter().filter(|s| s.nonstationary).count();
    if nonstationary_count < 2 {
        return Err("at least two nonstationary scenario families are required for contrast".into());
    }
    for s in &input.scenario_families {
        if !nonempty(&s.scenario_kind) || !nonempty(&s.climate_regime) {
            return Err(format!("scenario {} has incomplete classification", s.scenario_id));
        }
        if s.environmental_parameters_included {
            return Err(format!("scenario {} claims parameters although this fixture is design-only", s.scenario_id));
        }
        if s.source_admission_status != "not_admitted" {
            return Err(format!("scenario {} must remain not_admitted until its source is reviewed", s.scenario_id));
        }
        if let Some(weight) = s.scenario_weight {
            if !input.evidence_boundary.probability_basis_verified
                || !weight.is_finite()
                || weight <= 0.0
            {
                return Err("scenario weights require a verified probability basis and finite positive weights".into());
            }
        }
    }
    if input.evidence_boundary.probability_basis_verified {
        let weights: Vec<f64> = input.scenario_families.iter().filter_map(|s| s.scenario_weight).collect();
        if weights.len() != input.scenario_families.len()
            || (weights.iter().sum::<f64>() - 1.0).abs() > 1e-9
        {
            return Err("verified probability basis requires one scenario weight per family summing to one".into());
        }
    } else if input.evidence_boundary.scenario_weights.is_some()
        || input.scenario_families.iter().any(|s| s.scenario_weight.is_some())
    {
        return Err("probability weights are forbidden without a verified probability basis".into());
    }
    if input.evidence_boundary.environmental_trace_admitted
        || input.evidence_boundary.physical_model_verified
        || input.evidence_boundary.controller_result_measured
    {
        return Err("design fixture cannot escalate source, model, or controller evidence".into());
    }

    let required_states = [
        "storage_state_of_charge_and_health",
        "fuel_and_consumables_inventory",
        "equipment_age_and_degradation",
        "maintenance_and_repair_queue",
        "spares_inventory_and_replenishment_delay",
        "cumulative_service_deficit_debt",
        "controller_mode_and_recovery_state",
    ];
    unique_nonempty(&input.persistent_state_variables, "persistent state variables")?;
    for state in required_states {
        if !input.persistent_state_variables.iter().any(|s| s == state) {
            return Err(format!("required path-dependent state is missing: {state}"));
        }
    }

    unique_nonempty(
        &input.adaptation_decisions.iter().map(|d| d.decision_id.clone()).collect::<Vec<_>>(),
        "adaptation decision IDs",
    )?;
    if input.adaptation_decisions.is_empty()
        || input.adaptation_decisions.iter().any(|d| !nonempty(&d.decision_frequency))
    {
        return Err("adaptation decisions must be present and have explicit decision frequencies".into());
    }

    let c = &input.comparison_design;
    if c.arms.len() < 3
        || !c.paired_exogenous_trajectories
        || !c.same_initial_state_per_pair
        || !c.controller_visible_information_must_be_declared
        || !c.matched_information_budget_or_explicit_ablation
        || !c.compute_budget_must_be_recorded
        || !c.locked_holdout_frozen_before_tuning
        || !c.whole_site_year_or_episode_blocks
        || c.locked_holdout_used_for_tuning
    {
        return Err("paired comparison, fair information/compute budget, and frozen locked holdout are required".into());
    }
    let arm_ids = c.arms.iter().map(|a| a.arm_id.clone()).collect::<Vec<_>>();
    unique_nonempty(&arm_ids, "comparison arm IDs")?;
    let find_arm = |id: &str| c.arms.iter().find(|a| a.arm_id == id);
    if !find_arm("event_reset_baseline").is_some_and(|a| !a.state_carryover)
        || !find_arm("stateful_no_adaptation").is_some_and(|a| a.state_carryover)
        || !find_arm("stateful_adaptive_candidate").is_some_and(|a| a.state_carryover && a.adaptation_enabled == Some(true))
    {
        return Err("comparison must include event-reset, stateful non-adaptive, and stateful adaptive arms".into());
    }

    let required_metrics = [
        "cumulative_critical_service_deficit_by_service",
        "time_to_first_critical_service_floor_breach",
        "state_debt_after_each_event_and_after_recovery",
        "worst_case_by_declared_scenario_family",
        "empirical_stress_set_p95_with_nonprobabilistic_label",
        "locked_holdout_degradation",
    ];
    unique_nonempty(&input.required_metric_ids, "required metric IDs")?;
    for metric in required_metrics {
        if !input.required_metric_ids.iter().any(|m| m == metric) {
            return Err(format!("required long-horizon metric is missing: {metric}"));
        }
    }

    if !input.admission_gates.all_closed() {
        return Err("all admission gates must remain closed in a design-only, unexecuted contract".into());
    }
    let d = &input.decision_policy;
    if d.result_kind != "contract_validation_only_no_performance_results"
        || d.single_scalar_score_allowed
        || d.probability_weighted_risk_allowed_without_verified_basis
        || d.event_reset_baseline_may_be_called_sota
        || d.failed_or_missing_service_windows_may_be_dropped
        || !d.cross_site_or_future_climate_claim_requires_independent_holdout
    {
        return Err("decision policy permits an unsupported claim or omits a required holdout".into());
    }
    Ok(())
}

fn aggregate(input: &Input) -> Result<Report, String> {
    validate(input)?;
    let nonstationary_count = input.scenario_families.iter().filter(|s| s.nonstationary).count();
    Ok(Report {
        report_kind: "symtropy-long-horizon-resilience-contract-report",
        schema_version: "0.1.0",
        contract_id: input.contract_id.clone(),
        contract_valid: true,
        candidate_for_execution: false,
        evidence_level: "test_design_only",
        scenario_family_count: input.scenario_families.len(),
        nonstationary_scenario_count: nonstationary_count,
        persistent_state_variable_count: input.persistent_state_variables.len(),
        required_metric_count: input.required_metric_ids.len(),
        admission_gates_closed: input.admission_gates.all_closed(),
        result_scope: "design_contract_validation_only_no_environment_or_controller_performance",
        warnings: vec![
            "No environmental source or physical model is admitted by this design contract.".into(),
            "Unweighted stress-set quantiles are descriptive only, not calibrated real-world probabilities.".into(),
            "The event-reset arm is a diagnostic baseline and cannot establish stateful-controller superiority by itself.".into(),
        ],
    })
}

fn run() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "usage: long_horizon_contract <contract.json>")
    })?;
    let raw = fs::read_to_string(path)?;
    let input: Input = serde_json::from_str(&raw)?;
    let report = aggregate(&input).map_err(|message| {
        io::Error::new(io::ErrorKind::InvalidData, message)
    })?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("long_horizon_contract: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Input {
        serde_json::from_str(include_str!("../../fixtures/long-horizon-resilience-contract-2026.json"))
            .expect("long-horizon contract fixture must be valid JSON")
    }

    #[test]
    fn accepts_design_only_long_horizon_contract_without_escalating_evidence() {
        let input = fixture();
        let report = aggregate(&input).expect("design contract should validate");
        assert_eq!(report.scenario_family_count, 4);
        assert_eq!(report.nonstationary_scenario_count, 3);
        assert!(report.admission_gates_closed);
        assert!(!report.candidate_for_execution);
        assert_eq!(report.evidence_level, "test_design_only");
    }

    #[test]
    fn rejects_reset_between_events() {
        let mut input = fixture();
        input.horizon.allow_state_reset_between_events = true;
        assert!(validate(&input).unwrap_err().contains("state carry-over"));
    }

    #[test]
    fn rejects_probability_weights_without_probability_basis() {
        let mut input = fixture();
        input.scenario_families[0].scenario_weight = Some(0.25);
        assert!(validate(&input).unwrap_err().contains("verified probability basis"));
    }

    #[test]
    fn rejects_locked_holdout_tuning() {
        let mut input = fixture();
        input.comparison_design.locked_holdout_used_for_tuning = true;
        assert!(validate(&input).unwrap_err().contains("locked holdout"));
    }

    #[test]
    fn rejects_missing_path_dependent_state() {
        let mut input = fixture();
        input.persistent_state_variables.retain(|s| s != "spares_inventory_and_replenishment_delay");
        assert!(validate(&input).unwrap_err().contains("spares_inventory_and_replenishment_delay"));
    }

    #[test]
    fn rejects_open_admission_gate_in_unexecuted_contract() {
        let mut input = fixture();
        input.admission_gates.plant_model_verified_for_declared_use = true;
        assert!(validate(&input).unwrap_err().contains("all admission gates"));
    }

    #[test]
    fn rejects_claim_that_missing_windows_may_be_dropped() {
        let mut input = fixture();
        input.decision_policy.failed_or_missing_service_windows_may_be_dropped = true;
        assert!(validate(&input).unwrap_err().contains("unsupported claim"));
    }
}
