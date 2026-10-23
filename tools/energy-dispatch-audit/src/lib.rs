//! Independent, fail-closed audit of Sol-Atlas finite dispatch reports.
//!
//! This checker deliberately reimplements the declared energy-accounting
//! invariants instead of calling the dispatch search/transition code. A pass
//! establishes schedule feasibility in the declared integer-quantized model
//! and receipt integrity; it does not independently prove optimality, physical
//! model adequacy, AC-network behavior, safety, or evidence truth.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

mod reference;

const REQUEST_SCHEMA: &str = "energy-dispatch-v0.1";
const ASSESSMENT_SCHEMA: &str = "energy-dispatch-report-v1";
const RECEIPT_SCHEMA: &str = "1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchRequest {
    pub schema_version: String,
    pub case_id: String,
    pub synthetic: bool,
    pub evidence_class: String,
    pub evidence_ref: Option<String>,
    pub evidence_as_of: Option<String>,
    pub energy_quantum_kwh: f64,
    pub max_candidate_expansions: u64,
    pub battery: Battery,
    pub intervals: Vec<IntervalInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Battery {
    pub capacity_quanta: u16,
    pub initial_soc_quanta: u16,
    pub charge_input_limit_quanta: u16,
    pub discharge_withdrawal_limit_quanta: u16,
    pub charge_efficiency_numerator: u16,
    pub charge_efficiency_denominator: u16,
    pub discharge_efficiency_numerator: u16,
    pub discharge_efficiency_denominator: u16,
    pub terminal_reserve_quanta: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntervalInput {
    pub interval_id: String,
    pub start_unix_seconds: i64,
    pub duration_seconds: u64,
    pub grid_outage: bool,
    pub pv_available_quanta: u16,
    pub grid_import_limit_quanta: u16,
    pub essential_demand_quanta: u16,
    pub flexible_demand_quanta: u16,
    pub grid_cost_units_per_quantum: u32,
    pub grid_carbon_grams_per_quantum: u32,
}

// Field order intentionally matches Sol-Atlas energy-dispatch-report-v1
// serialization so the receipt's compact assessment digest can be recomputed.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchAssessment {
    pub assessment_schema_version: String,
    pub case_id: String,
    pub synthetic: bool,
    pub evidence_class: String,
    pub evidence_ref: Option<String>,
    pub evidence_as_of: Option<String>,
    pub claim_ceiling: String,
    pub energy_quantum_kwh: f64,
    pub search_is_exact: bool,
    pub pareto_frontier_complete: bool,
    pub evaluated_action_expansions: u64,
    pub feasible_complete_schedules: u64,
    pub pareto_frontier_count: usize,
    pub pareto_alternatives: Vec<ParetoAlternative>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParetoAlternative {
    pub objective: DispatchObjective,
    pub schedule: Vec<IntervalDecision>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct DispatchObjective {
    pub essential_unserved_quanta: u64,
    pub flexible_unserved_quanta: u64,
    pub operating_cost_units: u64,
    pub grid_carbon_grams: u64,
    pub curtailed_pv_quanta: u64,
    pub terminal_soc_quanta: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntervalDecision {
    pub interval_id: String,
    pub start_unix_seconds: i64,
    pub duration_seconds: u64,
    pub grid_outage: bool,
    pub pv_available_quanta: u16,
    pub pv_used_quanta: u16,
    pub pv_curtailed_quanta: u16,
    pub grid_import_quanta: u16,
    pub battery_soc_before_quanta: u16,
    pub battery_charge_input_quanta: u16,
    pub battery_soc_withdrawal_quanta: u16,
    pub battery_discharge_delivered_quanta: u16,
    pub battery_soc_after_quanta: u16,
    pub essential_demand_quanta: u16,
    pub essential_served_quanta: u16,
    pub essential_unserved_quanta: u16,
    pub flexible_demand_quanta: u16,
    pub flexible_served_quanta: u16,
    pub flexible_unserved_quanta: u16,
    pub grid_cost_units_per_quantum: u32,
    pub grid_carbon_grams_per_quantum: u32,
    pub bus_energy_balance_residual_quanta: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunReceipt {
    pub receipt_schema_version: String,
    pub evaluator_version: String,
    pub digest_algorithm: String,
    pub digest_scope: String,
    pub input_sha256: String,
    pub assessment_sha256: String,
    pub input_bytes: usize,
    pub case_id: String,
    pub synthetic: bool,
    pub claim_ceiling: String,
    pub verification_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchRunReport {
    pub receipt: RunReceipt,
    pub assessment: DispatchAssessment,
}

#[derive(Clone, Debug, Serialize)]
pub struct AuditCheck {
    pub id: String,
    pub status: String,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct IndependentAuditReport {
    pub audit_schema_version: String,
    pub evaluator_version: String,
    pub request_sha256: String,
    pub run_report_sha256: String,
    pub case_id: Option<String>,
    /// Status is about receipt integrity and schedule feasibility, not optimality.
    pub status: String,
    pub solver_reported_search_status: String,
    pub reference_frontier_status: String,
    pub reference_expansions: Option<u64>,
    pub reference_feasible_complete_schedules: Option<u64>,
    pub objective_claim_status: String,
    pub checks: Vec<AuditCheck>,
    pub claim_ceiling: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn check(checks: &mut Vec<AuditCheck>, id: impl Into<String>, passed: bool, detail: impl Into<String>) {
    checks.push(AuditCheck {
        id: id.into(),
        status: if passed { "pass" } else { "fail" }.to_owned(),
        detail: detail.into(),
    });
}

fn invalid_input_report(request_bytes: &[u8], report_bytes: &[u8], id: &str, error: &str) -> IndependentAuditReport {
    IndependentAuditReport {
        audit_schema_version: "energy-independent-audit-v0.1".to_owned(),
        evaluator_version: "symtropy-energy-dispatch-audit-0.1.0".to_owned(),
        request_sha256: sha256_hex(request_bytes),
        run_report_sha256: sha256_hex(report_bytes),
        case_id: None,
        status: "fail".to_owned(),
        solver_reported_search_status: "unknown".to_owned(),
        reference_frontier_status: "not_run_invalid_input".to_owned(),
        reference_expansions: None,
        reference_feasible_complete_schedules: None,
        objective_claim_status: "not_independently_verified".to_owned(),
        checks: vec![AuditCheck {
            id: id.to_owned(),
            status: "fail".to_owned(),
            detail: error.to_owned(),
        }],
        claim_ceiling: "Input/report could not be safely audited; no feasibility claim is supported.".to_owned(),
    }
}

/// Audit the exact request bytes and the run-report bytes emitted by
/// energy_dispatch_assess. The embedded receipt must bind to those exact bytes.
pub fn audit_bytes(request_bytes: &[u8], run_report_bytes: &[u8]) -> IndependentAuditReport {
    let request: DispatchRequest = match serde_json::from_slice(request_bytes) {
        Ok(value) => value,
        Err(error) => return invalid_input_report(request_bytes, run_report_bytes, "request_parse", &error.to_string()),
    };
    let run_report: DispatchRunReport = match serde_json::from_slice(run_report_bytes) {
        Ok(value) => value,
        Err(error) => return invalid_input_report(request_bytes, run_report_bytes, "run_report_parse", &error.to_string()),
    };

    let receipt = &run_report.receipt;
    let assessment = &run_report.assessment;
    let mut checks = Vec::new();

    let request_hash = sha256_hex(request_bytes);
    let assessment_bytes = match serde_json::to_vec(assessment) {
        Ok(bytes) => bytes,
        Err(error) => return invalid_input_report(request_bytes, run_report_bytes, "assessment_serialize", &error.to_string()),
    };
    let assessment_hash = sha256_hex(&assessment_bytes);

    check(
        &mut checks,
        "request_schema_version",
        request.schema_version == REQUEST_SCHEMA,
        format!("expected {REQUEST_SCHEMA}, got {}", request.schema_version),
    );
    check(
        &mut checks,
        "assessment_schema_version",
        assessment.assessment_schema_version == ASSESSMENT_SCHEMA,
        format!("expected {ASSESSMENT_SCHEMA}, got {}", assessment.assessment_schema_version),
    );
    check(
        &mut checks,
        "receipt_schema_version",
        receipt.receipt_schema_version == RECEIPT_SCHEMA
            && receipt.digest_algorithm == "SHA-256"
            && receipt.evaluator_version == "energy-dispatch-v0.1"
            && receipt.digest_scope == "exact_input_bytes_and_compact_assessment_json",
        "receipt schema, evaluator version, digest scope and algorithm must match the supported v0.1 receipt contract",
    );
    check(
        &mut checks,
        "receipt_binds_exact_request_bytes",
        receipt.input_sha256 == request_hash && receipt.input_bytes == request_bytes.len(),
        "receipt input digest and byte count must match the exact dispatch-request byte sequence",
    );
    check(
        &mut checks,
        "receipt_binds_assessment",
        receipt.assessment_sha256 == assessment_hash,
        "receipt assessment digest must match the compact serialized assessment",
    );
    check(
        &mut checks,
        "case_identity",
        !request.case_id.is_empty()
            && request.case_id == assessment.case_id
            && request.case_id == receipt.case_id,
        "request, assessment and receipt must carry the same non-empty case ID",
    );
    check(
        &mut checks,
        "synthetic_identity",
        request.synthetic == assessment.synthetic && assessment.synthetic == receipt.synthetic,
        "synthetic/evidence boundary must agree across request, assessment and receipt",
    );
    check(
        &mut checks,
        "assessment_parameters_match_request",
        assessment.energy_quantum_kwh == request.energy_quantum_kwh
            && assessment.evidence_class == request.evidence_class
            && assessment.evidence_ref == request.evidence_ref
            && assessment.evidence_as_of == request.evidence_as_of,
        "energy quantum and evidence fields must be preserved from request into assessment",
    );
    check(
        &mut checks,
        "receipt_claim_ceiling",
        receipt.claim_ceiling == assessment.claim_ceiling,
        "receipt and assessment claim ceilings must match",
    );

    audit_request_contract(&request, &mut checks);

    check(
        &mut checks,
        "pareto_alternatives_present",
        !assessment.pareto_alternatives.is_empty(),
        "at least one complete schedule is required to audit feasibility",
    );
    check(
        &mut checks,
        "frontier_count_matches",
        assessment.pareto_frontier_count == assessment.pareto_alternatives.len(),
        "declared frontier count must equal the number of returned alternatives",
    );
    check(
        &mut checks,
        "feasible_schedule_count_nonzero",
        assessment.feasible_complete_schedules > 0,
        "assessment must report at least one feasible complete schedule",
    );
    check(
        &mut checks,
        "evaluated_expansions_within_budget",
        assessment.evaluated_action_expansions <= request.max_candidate_expansions,
        "reported action expansions must not exceed the request budget",
    );

    for (index, alternative) in assessment.pareto_alternatives.iter().enumerate() {
        audit_alternative(&request, alternative, index, &mut checks);
    }

    let request_ok = checks.iter().any(|item| item.id == "request_schema_version" && item.status == "pass")
        && checks.iter().any(|item| item.id == "request_internal_invariants" && item.status == "pass");
    let mut reference_frontier_status = "not_run_invalid_request".to_owned();
    let mut reference_expansions = None;
    let mut reference_feasible_complete_schedules = None;
    let mut objective_claim_status = "not_independently_verified".to_owned();

    if request_ok {
        match reference::enumerate(&request, reference::DEFAULT_REFERENCE_EXPANSION_LIMIT) {
            Ok(reference_result) => {
                reference_expansions = Some(reference_result.expansions);
                reference_feasible_complete_schedules = Some(reference_result.feasible_complete_schedules);

                let mut expected_frontier = reference_result.frontier;
                expected_frontier.sort();
                let mut reported_frontier = assessment.pareto_alternatives
                    .iter()
                    .map(|alternative| alternative.objective.clone())
                    .collect::<Vec<_>>();
                reported_frontier.sort();
                let frontier_matches = expected_frontier == reported_frontier;
                check(
                    &mut checks,
                    "independent_reference_frontier",
                    frontier_matches,
                    format!(
                        "independently enumerated {} frontier vectors; received {}; a mismatch fails closed",
                        expected_frontier.len(),
                        reported_frontier.len()
                    ),
                );

                let complete_count_matches = assessment.feasible_complete_schedules
                    == reference_result.feasible_complete_schedules;
                check(
                    &mut checks,
                    "independent_reference_feasible_schedule_count",
                    complete_count_matches,
                    format!(
                        "reference enumerated {} feasible complete schedules; report declares {}",
                        reference_result.feasible_complete_schedules,
                        assessment.feasible_complete_schedules
                    ),
                );

                if frontier_matches && complete_count_matches {
                    reference_frontier_status = "pass".to_owned();
                    objective_claim_status = "verified_for_bounded_model_case".to_owned();
                } else {
                    reference_frontier_status = "fail".to_owned();
                    objective_claim_status = "reference_frontier_mismatch".to_owned();
                }
            }
            Err(reference::ReferenceError::BudgetExceeded { expansions }) => {
                reference_expansions = Some(expansions);
                reference_frontier_status = "not_run_budget_exceeded".to_owned();
                checks.push(AuditCheck {
                    id: "independent_reference_frontier".to_owned(),
                    status: "unknown".to_owned(),
                    detail: format!(
                        "reference enumeration exceeded its independent budget of {}; schedule feasibility checks remain separate",
                        reference::DEFAULT_REFERENCE_EXPANSION_LIMIT
                    ),
                });
            }
            Err(reference::ReferenceError::ArithmeticOverflow) => {
                reference_frontier_status = "not_run_arithmetic_overflow".to_owned();
                checks.push(AuditCheck {
                    id: "independent_reference_frontier".to_owned(),
                    status: "unknown".to_owned(),
                    detail: "reference enumeration stopped on checked arithmetic overflow".to_owned(),
                });
            }
        }
    } else {
        checks.push(AuditCheck {
            id: "independent_reference_frontier".to_owned(),
            status: "unknown".to_owned(),
            detail: "reference enumeration was skipped because the request failed its independent structural checks".to_owned(),
        });
    }

    let has_fail = checks.iter().any(|item| item.status == "fail");
    let has_unknown = checks.iter().any(|item| item.status == "unknown");
    let status = if has_fail { "fail" } else if has_unknown { "unknown" } else { "pass" };
    let solver_reported_search_status = if assessment.search_is_exact && assessment.pareto_frontier_complete {
        "solver_reports_exhaustive_search"
    } else {
        "solver_reports_incomplete_or_unknown_search"
    };

    IndependentAuditReport {
        audit_schema_version: "energy-independent-audit-v0.1".to_owned(),
        evaluator_version: "symtropy-energy-dispatch-audit-0.1.0".to_owned(),
        request_sha256: request_hash,
        run_report_sha256: sha256_hex(run_report_bytes),
        case_id: Some(request.case_id),
        status: status.to_owned(),
        solver_reported_search_status: solver_reported_search_status.to_owned(),
        reference_frontier_status,
        reference_expansions,
        reference_feasible_complete_schedules,
        objective_claim_status,
        checks,
        claim_ceiling: "Independent schedule feasibility and (when bounded enumeration completes) Pareto-frontier verification for the finite integer-quantized electrical model only; not AC/network physics, physical safety, evidence truth, model adequacy or field performance.".to_owned(),
    }
}

fn audit_request_contract(request: &DispatchRequest, checks: &mut Vec<AuditCheck>) {
    let battery = &request.battery;
    let evidence_class_known = matches!(
        request.evidence_class.as_str(),
        "analyst_assumption"
            | "published_model"
            | "vendor_target"
            | "measured"
            | "field_pilot"
            | "operating_plant"
            | "replicated_fleet"
            | "independently_audited"
    );
    let evidence_reference_ok = request.evidence_class == "analyst_assumption"
        || request.evidence_ref.as_deref().is_some_and(|value| !value.trim().is_empty());
    let mut valid = !request.case_id.trim().is_empty()
        && request.energy_quantum_kwh.is_finite()
        && request.energy_quantum_kwh > 0.0
        && request.max_candidate_expansions > 0
        && !request.intervals.is_empty()
        && evidence_class_known
        && evidence_reference_ok
        && battery.initial_soc_quanta <= battery.capacity_quanta
        && battery.terminal_reserve_quanta <= battery.capacity_quanta
        && battery.charge_efficiency_denominator > 0
        && battery.discharge_efficiency_denominator > 0
        && battery.charge_efficiency_numerator > 0
        && battery.discharge_efficiency_numerator > 0
        && battery.charge_efficiency_numerator <= battery.charge_efficiency_denominator
        && battery.discharge_efficiency_numerator <= battery.discharge_efficiency_denominator;

    let mut ids = BTreeSet::new();
    let mut expected_start: Option<i64> = None;
    for interval in &request.intervals {
        if interval.interval_id.trim().is_empty() || !ids.insert(interval.interval_id.as_str()) || interval.duration_seconds == 0 {
            valid = false;
        }
        if let Some(expected) = expected_start {
            if interval.start_unix_seconds != expected {
                valid = false;
            }
        }
        expected_start = i64::try_from(interval.duration_seconds)
            .ok()
            .and_then(|duration| interval.start_unix_seconds.checked_add(duration));
        if expected_start.is_none() {
            valid = false;
        }
    }
    check(
        checks,
        "request_internal_invariants",
        valid,
        "case ID, quantum, interval IDs/chronology, battery bounds and efficiency ratios must be internally valid",
    );
}

fn audit_alternative(request: &DispatchRequest, alternative: &ParetoAlternative, alternative_index: usize, checks: &mut Vec<AuditCheck>) {
    let prefix = format!("alternative_{alternative_index}");
    let schedule = &alternative.schedule;
    let lengths_match = schedule.len() == request.intervals.len();
    check(
        checks,
        format!("{prefix}_schedule_length"),
        lengths_match,
        format!("schedule has {} steps; request has {} intervals", schedule.len(), request.intervals.len()),
    );
    if !lengths_match {
        return;
    }

    let battery = &request.battery;
    let mut expected_soc = battery.initial_soc_quanta;
    let mut essential_unserved = 0_u64;
    let mut flexible_unserved = 0_u64;
    let mut cost_units = 0_u64;
    let mut carbon_grams = 0_u64;
    let mut curtailed_pv = 0_u64;

    for (index, (input, step)) in request.intervals.iter().zip(schedule.iter()).enumerate() {
        let sid = format!("{prefix}_step_{index}");
        let identity_ok = step.interval_id == input.interval_id
            && step.start_unix_seconds == input.start_unix_seconds
            && step.duration_seconds == input.duration_seconds
            && step.grid_outage == input.grid_outage
            && step.pv_available_quanta == input.pv_available_quanta
            && step.essential_demand_quanta == input.essential_demand_quanta
            && step.flexible_demand_quanta == input.flexible_demand_quanta
            && step.grid_cost_units_per_quantum == input.grid_cost_units_per_quantum
            && step.grid_carbon_grams_per_quantum == input.grid_carbon_grams_per_quantum;
        check(checks, format!("{sid}_input_binding"), identity_ok, "step identity, chronology, outage flag and declared demands/rates must match the request");

        let pv_ok = step.pv_used_quanta <= step.pv_available_quanta
            && u32::from(step.pv_used_quanta) + u32::from(step.pv_curtailed_quanta) == u32::from(step.pv_available_quanta);
        check(checks, format!("{sid}_pv_limits"), pv_ok, "PV used must not exceed availability and used plus curtailed must equal available");

        let grid_ok = step.grid_import_quanta <= input.grid_import_limit_quanta
            && (!input.grid_outage || step.grid_import_quanta == 0);
        check(checks, format!("{sid}_grid_limits"), grid_ok, "grid import must obey the interval cap and be zero during an outage");

        let service_ok = step.essential_served_quanta <= step.essential_demand_quanta
            && u32::from(step.essential_served_quanta) + u32::from(step.essential_unserved_quanta) == u32::from(step.essential_demand_quanta)
            && step.flexible_served_quanta <= step.flexible_demand_quanta
            && u32::from(step.flexible_served_quanta) + u32::from(step.flexible_unserved_quanta) == u32::from(step.flexible_demand_quanta);
        check(checks, format!("{sid}_service_accounting"), service_ok, "served plus unserved must equal demand; served energy cannot exceed demand");

        let storage_limits_ok = step.battery_soc_before_quanta == expected_soc
            && step.battery_charge_input_quanta <= battery.charge_input_limit_quanta
            && step.battery_soc_withdrawal_quanta <= battery.discharge_withdrawal_limit_quanta
            && step.battery_soc_withdrawal_quanta <= step.battery_soc_before_quanta
            && !(step.battery_charge_input_quanta > 0 && step.battery_soc_withdrawal_quanta > 0);
        let charge_gain = u64::from(step.battery_charge_input_quanta)
            .saturating_mul(u64::from(battery.charge_efficiency_numerator))
            / u64::from(battery.charge_efficiency_denominator.max(1));
        let discharge_expected = u64::from(step.battery_soc_withdrawal_quanta)
            .saturating_mul(u64::from(battery.discharge_efficiency_numerator))
            / u64::from(battery.discharge_efficiency_denominator.max(1));
        let soc_expected = u64::from(step.battery_soc_before_quanta)
            .saturating_sub(u64::from(step.battery_soc_withdrawal_quanta))
            .saturating_add(charge_gain);
        let state_transition_ok = storage_limits_ok
            && soc_expected == u64::from(step.battery_soc_after_quanta)
            && discharge_expected == u64::from(step.battery_discharge_delivered_quanta)
            && step.battery_soc_after_quanta <= battery.capacity_quanta;
        check(checks, format!("{sid}_storage_transition"), state_transition_ok, "SOC transition, charge/discharge efficiency, non-simultaneous operation, power-energy limits and capacity must agree");

        let bus_supply = u64::from(step.pv_used_quanta)
            + u64::from(step.grid_import_quanta)
            + u64::from(step.battery_discharge_delivered_quanta);
        let bus_use = u64::from(step.essential_served_quanta)
            + u64::from(step.flexible_served_quanta)
            + u64::from(step.battery_charge_input_quanta);
        let derived_residual = i64::try_from(bus_supply).unwrap_or(i64::MAX)
            - i64::try_from(bus_use).unwrap_or(i64::MAX);
        let balance_ok = bus_supply == bus_use
            && step.bus_energy_balance_residual_quanta == 0
            && step.bus_energy_balance_residual_quanta == derived_residual;
        check(checks, format!("{sid}_bus_balance"), balance_ok, "PV + grid + delivered battery energy must equal served load + battery charge; residual must be exactly zero");

        essential_unserved += u64::from(step.essential_unserved_quanta);
        flexible_unserved += u64::from(step.flexible_unserved_quanta);
        cost_units += u64::from(step.grid_import_quanta) * u64::from(input.grid_cost_units_per_quantum);
        carbon_grams += u64::from(step.grid_import_quanta) * u64::from(input.grid_carbon_grams_per_quantum);
        curtailed_pv += u64::from(step.pv_curtailed_quanta);
        expected_soc = step.battery_soc_after_quanta;
    }

    let last_soc = schedule.last().map(|step| step.battery_soc_after_quanta);
    let terminal_ok = last_soc.is_some_and(|soc| soc >= battery.terminal_reserve_quanta)
        && last_soc == Some(alternative.objective.terminal_soc_quanta);
    check(checks, format!("{prefix}_terminal_reserve"), terminal_ok, "terminal SOC must satisfy reserve and match the declared terminal-SOC objective");

    let objective_ok = alternative.objective.essential_unserved_quanta == essential_unserved
        && alternative.objective.flexible_unserved_quanta == flexible_unserved
        && alternative.objective.operating_cost_units == cost_units
        && alternative.objective.grid_carbon_grams == carbon_grams
        && alternative.objective.curtailed_pv_quanta == curtailed_pv;
    check(checks, format!("{prefix}_objective_recomputation"), objective_ok, "reported service, grid cost, carbon and PV-curtailment totals must be recomputable from the schedule");
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUEST: &str = include_str!("../fixtures/pv-outage.request.json");

    fn make_valid_report(request_bytes: &[u8]) -> DispatchRunReport {
        let request: DispatchRequest = serde_json::from_slice(request_bytes).expect("frozen request is valid");
        let inputs = &request.intervals;
        assert_eq!(inputs.len(), 2);

        let first_step = IntervalDecision {
            interval_id: inputs[0].interval_id.clone(),
            start_unix_seconds: inputs[0].start_unix_seconds,
            duration_seconds: inputs[0].duration_seconds,
            grid_outage: inputs[0].grid_outage,
            pv_available_quanta: inputs[0].pv_available_quanta,
            pv_used_quanta: 2,
            pv_curtailed_quanta: 0,
            grid_import_quanta: 0,
            battery_soc_before_quanta: 0,
            battery_charge_input_quanta: 2,
            battery_soc_withdrawal_quanta: 0,
            battery_discharge_delivered_quanta: 0,
            battery_soc_after_quanta: 2,
            essential_demand_quanta: 0,
            essential_served_quanta: 0,
            essential_unserved_quanta: 0,
            flexible_demand_quanta: 0,
            flexible_served_quanta: 0,
            flexible_unserved_quanta: 0,
            grid_cost_units_per_quantum: inputs[0].grid_cost_units_per_quantum,
            grid_carbon_grams_per_quantum: inputs[0].grid_carbon_grams_per_quantum,
            bus_energy_balance_residual_quanta: 0,
        };

        let make_schedule = |discharge: u16| {
            vec![
                first_step.clone(),
                IntervalDecision {
                    interval_id: inputs[1].interval_id.clone(),
                    start_unix_seconds: inputs[1].start_unix_seconds,
                    duration_seconds: inputs[1].duration_seconds,
                    grid_outage: inputs[1].grid_outage,
                    pv_available_quanta: inputs[1].pv_available_quanta,
                    pv_used_quanta: 0,
                    pv_curtailed_quanta: 0,
                    grid_import_quanta: 0,
                    battery_soc_before_quanta: 2,
                    battery_charge_input_quanta: 0,
                    battery_soc_withdrawal_quanta: discharge,
                    battery_discharge_delivered_quanta: discharge,
                    battery_soc_after_quanta: 2 - discharge,
                    essential_demand_quanta: 2,
                    essential_served_quanta: discharge,
                    essential_unserved_quanta: 2 - discharge,
                    flexible_demand_quanta: 0,
                    flexible_served_quanta: 0,
                    flexible_unserved_quanta: 0,
                    grid_cost_units_per_quantum: inputs[1].grid_cost_units_per_quantum,
                    grid_carbon_grams_per_quantum: inputs[1].grid_carbon_grams_per_quantum,
                    bus_energy_balance_residual_quanta: 0,
                },
            ]
        };

        // Exact reference enumeration yields three non-dominated trade-offs:
        // serve 2/1/0 quanta and retain 0/1/2 quanta of terminal battery SOC.
        let alternatives = [2_u16, 1, 0]
            .into_iter()
            .map(|discharge| ParetoAlternative {
                objective: DispatchObjective {
                    essential_unserved_quanta: u64::from(2 - discharge),
                    flexible_unserved_quanta: 0,
                    operating_cost_units: 0,
                    grid_carbon_grams: 0,
                    curtailed_pv_quanta: 0,
                    terminal_soc_quanta: 2 - discharge,
                },
                schedule: make_schedule(discharge),
            })
            .collect::<Vec<_>>();

        let assessment = DispatchAssessment {
            assessment_schema_version: ASSESSMENT_SCHEMA.to_owned(),
            case_id: request.case_id.clone(),
            synthetic: request.synthetic,
            evidence_class: request.evidence_class.clone(),
            evidence_ref: request.evidence_ref.clone(),
            evidence_as_of: request.evidence_as_of.clone(),
            claim_ceiling: "Synthetic finite quantized-model fixture only.".to_owned(),
            energy_quantum_kwh: request.energy_quantum_kwh,
            search_is_exact: true,
            pareto_frontier_complete: true,
            evaluated_action_expansions: 203,
            feasible_complete_schedules: 14,
            pareto_frontier_count: 3,
            pareto_alternatives: alternatives,
        };
        let assessment_sha256 = sha256_hex(&serde_json::to_vec(&assessment).expect("assessment serializes"));
        DispatchRunReport {
            receipt: RunReceipt {
                receipt_schema_version: RECEIPT_SCHEMA.to_owned(),
                evaluator_version: "energy-dispatch-v0.1".to_owned(),
                digest_algorithm: "SHA-256".to_owned(),
                digest_scope: "exact_input_bytes_and_compact_assessment_json".to_owned(),
                input_sha256: sha256_hex(request_bytes),
                assessment_sha256,
                input_bytes: request_bytes.len(),
                case_id: request.case_id,
                synthetic: request.synthetic,
                claim_ceiling: "Synthetic finite quantized-model fixture only.".to_owned(),
                verification_boundary: "Digest binding only; not evidence truth.".to_owned(),
            },
            assessment,
        }
    }

    fn audit_report(request_bytes: &[u8], report: &DispatchRunReport) -> IndependentAuditReport {
        let bytes = serde_json::to_vec(report).expect("report serializes");
        audit_bytes(request_bytes, &bytes)
    }

    #[test]
    fn known_answer_schedule_passes_feasibility_audit() {
        let report = make_valid_report(REQUEST.as_bytes());
        let audited = audit_report(REQUEST.as_bytes(), &report);
        assert_eq!(audited.status, "pass", "{:#?}", audited.checks);
        assert_eq!(audited.reference_frontier_status, "pass");
        assert_eq!(audited.reference_feasible_complete_schedules, Some(14));
        assert_eq!(audited.objective_claim_status, "verified_for_bounded_model_case");
    }

    #[test]
    fn grid_import_during_outage_is_rejected() {
        let mut report = make_valid_report(REQUEST.as_bytes());
        report.assessment.pareto_alternatives[0].schedule[1].grid_import_quanta = 1;
        report.receipt.assessment_sha256 = sha256_hex(&serde_json::to_vec(&report.assessment).unwrap());
        let audited = audit_report(REQUEST.as_bytes(), &report);
        assert_eq!(audited.status, "fail");
        assert!(audited.checks.iter().any(|c| c.id == "alternative_0_step_1_grid_limits" && c.status == "fail"));
    }

    #[test]
    fn forged_state_of_charge_is_rejected() {
        let mut report = make_valid_report(REQUEST.as_bytes());
        report.assessment.pareto_alternatives[0].schedule[0].battery_soc_after_quanta = 1;
        report.receipt.assessment_sha256 = sha256_hex(&serde_json::to_vec(&report.assessment).unwrap());
        let audited = audit_report(REQUEST.as_bytes(), &report);
        assert_eq!(audited.status, "fail");
        assert!(audited.checks.iter().any(|c| c.id == "alternative_0_step_0_storage_transition" && c.status == "fail"));
    }

    #[test]
    fn request_byte_change_breaks_receipt_binding() {
        let report = make_valid_report(REQUEST.as_bytes());
        let changed = REQUEST.replace("synthetic-pv-charge-outage", "synthetic-pv-charge-outage-modified");
        let bytes = serde_json::to_vec(&report).unwrap();
        let audited = audit_bytes(changed.as_bytes(), &bytes);
        assert_eq!(audited.status, "fail");
        assert!(audited.checks.iter().any(|c| c.id == "receipt_binds_exact_request_bytes" && c.status == "fail"));
    }

    #[test]
    fn independent_reference_can_verify_small_frontier_even_if_solver_flags_are_incomplete() {
        let mut report = make_valid_report(REQUEST.as_bytes());
        report.assessment.search_is_exact = false;
        report.assessment.pareto_frontier_complete = false;
        report.receipt.assessment_sha256 = sha256_hex(&serde_json::to_vec(&report.assessment).unwrap());
        let audited = audit_report(REQUEST.as_bytes(), &report);
        assert_eq!(audited.status, "pass");
        assert_eq!(audited.solver_reported_search_status, "solver_reports_incomplete_or_unknown_search");
        assert_eq!(audited.reference_frontier_status, "pass");
        assert_eq!(audited.objective_claim_status, "verified_for_bounded_model_case");
    }

    #[test]
    fn mutated_assessment_without_receipt_refresh_is_rejected() {
        let mut report = make_valid_report(REQUEST.as_bytes());
        report.assessment.pareto_alternatives[0].schedule[1].essential_served_quanta = 1;
        // Deliberately leave the old assessment digest in the receipt.
        let audited = audit_report(REQUEST.as_bytes(), &report);
        assert_eq!(audited.status, "fail");
        assert!(audited.checks.iter().any(|c| c.id == "receipt_binds_assessment" && c.status == "fail"));
    }
}
