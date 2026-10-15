// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Narrow executable checker for the v0.1 synthetic disruption-rehearsal contract.
//!
//! This module evaluates declared abstract input states only. It does not run
//! maritime physics, simulate a vessel, assess safety, or grant authority.

use serde::Serialize;
use std::env;
use std::fs;

const MAX_SCENARIO_BYTES: usize = 1_000_000;
const ROOT_KEYS: &[&str] = &[
    "schema_id",
    "schema_version",
    "fixture_id",
    "fixture_status",
    "source_kind",
    "claim_ceiling",
    "description",
    "assessment",
    "model_scope",
    "candidate_plans",
    "hard_constraints",
    "scenario_cases",
    "expected_global_properties",
    "execution",
];
const AUTHORITY_KEYS: &[&str] = &[
    "engineering_admission",
    "navigation_authorized",
    "physical_truth_claim_authorized",
    "berth_availability_inference_authorized",
    "reservation_or_custody_event_authorized",
    "physical_execution_authorized",
];
const INPUT_KEYS: &[&str] = &[
    "water_level_envelope_state",
    "energy_reserve_state",
    "terminal_window_state",
    "forecast_evidence_state",
    "waiting_resource_state",
    "alternate_capacity_state",
];
const PLAN_IDS: &[&str] = &[
    "continue_original_slot",
    "hold_and_reassess",
    "prepare_alternate_terminal",
];
const CASE_IDS: &[&str] = &[
    "baseline_declared_inputs",
    "adverse_water_level_envelope",
    "shore_power_unavailable",
    "forecast_invalidated_before_decision",
    "common_cause_terminal_and_power_disruption",
];

#[derive(Debug, Serialize, PartialEq, Eq)]
struct DispositionReport {
    case_id: String,
    plan_id: String,
    status: String,
    blockers: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct RehearsalReport {
    status: &'static str,
    schema_id: String,
    fixture_id: String,
    scenario_sha256: String,
    scenario_case_count: usize,
    candidate_plan_count: usize,
    dispositions: Vec<DispositionReport>,
    robustly_feasible_candidate_plans: Vec<String>,
    engineering_admission: &'static str,
    navigation_authorized: bool,
    physical_truth_claim_authorized: bool,
    berth_availability_inference_authorized: bool,
    reservation_or_custody_event_authorized: bool,
    physical_execution_authorized: bool,
    source_commit_binding: String,
    contract_evaluation_performed: bool,
    physical_simulation_performed: bool,
}

#[derive(Debug)]
struct AbstractInputs {
    water: String,
    energy: String,
    terminal: String,
    forecast: String,
    waiting: String,
    alternate_capacity: String,
}

fn text_field(value: &super::JsonValue, key: &str, path: &str) -> Result<String, String> {
    Ok(super::require_nonempty_text(
        super::field(value, key, path)?,
        &format!("{path}.{key}"),
    )?
    .to_owned())
}

fn expect_text_array(value: &super::JsonValue, path: &str, expected: &[&str]) -> Result<(), String> {
    let actual = super::array(value, path)?;
    if actual.len() != expected.len() {
        return Err(format!("{path}: expected {} entries, found {}", expected.len(), actual.len()));
    }
    for (index, (value, expected_text)) in actual.iter().zip(expected).enumerate() {
        super::expect_text(value, &format!("{path}[{index}]"), expected_text)?;
    }
    Ok(())
}

fn validate_static_contract(root: &super::JsonValue) -> Result<(), String> {
    let assessment = super::field(root, "assessment", "rehearsal")?;
    super::exact_keys(assessment, &["assessment_time_utc", "target_time_utc", "time_policy_id", "forecast_input"], "rehearsal.assessment")?;
    for key in ["assessment_time_utc", "target_time_utc"] {
        let value = text_field(assessment, key, "rehearsal.assessment")?;
        if !super::valid_utc_timestamp(&value) || value.len() != 20 {
            return Err(format!("rehearsal.assessment.{key}: expected canonical whole-second UTC timestamp"));
        }
    }
    if text_field(assessment, "time_policy_id", "rehearsal.assessment")? != "synthetic-policy-v0.1" {
        return Err("rehearsal.assessment.time_policy_id is not the pinned v0.1 policy".to_owned());
    }
    let forecast = super::field(assessment, "forecast_input", "rehearsal.assessment")?;
    super::exact_keys(forecast, &[
        "fixture_reference", "source_artifact_sha256", "receipt_sha256", "binding_state",
        "issued_at_utc", "model_initialization_time_utc", "valid_from_utc", "valid_until_utc",
        "max_issue_age_seconds", "max_forecast_lead_seconds",
        "expected_temporal_arithmetic_for_test_design_only", "execution_status", "interpretation"
    ], "rehearsal.assessment.forecast_input")?;
    for key in ["source_artifact_sha256", "receipt_sha256"] {
        if !matches!(super::field(forecast, key, "forecast_input")?, super::JsonValue::Null) {
            return Err(format!("forecast_input.{key} must remain null until exact real bytes are bound"));
        }
    }
    super::expect_text(super::field(forecast, "binding_state", "forecast_input")?, "forecast_input.binding_state", "not_bound_in_this_design_fixture")?;
    super::expect_text(super::field(forecast, "execution_status", "forecast_input")?, "forecast_input.execution_status", "not_executed")?;
    for key in ["issued_at_utc", "model_initialization_time_utc", "valid_from_utc", "valid_until_utc"] {
        let value = text_field(forecast, key, "forecast_input")?;
        if !super::valid_utc_timestamp(&value) || value.len() != 20 {
            return Err(format!("forecast_input.{key}: expected canonical whole-second UTC timestamp"));
        }
    }
    for key in ["max_issue_age_seconds", "max_forecast_lead_seconds"] {
        if super::integer(super::field(forecast, key, "forecast_input")?, &format!("forecast_input.{key}"))? == 0 {
            return Err(format!("forecast_input.{key} must be positive"));
        }
    }
    let arithmetic = super::field(forecast, "expected_temporal_arithmetic_for_test_design_only", "forecast_input")?;
    super::exact_keys(arithmetic, &["issue_age_seconds", "forecast_lead_seconds"], "forecast_input.expected_temporal_arithmetic_for_test_design_only")?;
    for key in ["issue_age_seconds", "forecast_lead_seconds"] {
        if super::integer(super::field(arithmetic, key, "forecast arithmetic")?, &format!("forecast arithmetic.{key}"))? == 0 {
            return Err(format!("forecast arithmetic.{key} must be positive"));
        }
    }

    let scope = super::field(root, "model_scope", "rehearsal")?;
    super::exact_keys(scope, &["execution_profile", "physics_claims", "abstract_inputs_only", "unsupported_if_requested", "missing_or_unsupported_is_never_zero"], "rehearsal.model_scope")?;
    super::expect_text(super::field(scope, "execution_profile", "model_scope")?, "model_scope.execution_profile", "EngineeringDeterministic")?;
    if !super::array(super::field(scope, "physics_claims", "model_scope")?, "model_scope.physics_claims")?.is_empty() {
        return Err("model_scope.physics_claims must stay empty in this abstract evaluator".to_owned());
    }
    expect_text_array(super::field(scope, "abstract_inputs_only", "model_scope")?, "model_scope.abstract_inputs_only", INPUT_KEYS)?;
    expect_text_array(super::field(scope, "unsupported_if_requested", "model_scope")?, "model_scope.unsupported_if_requested", &[
        "hydrodynamics", "under_keel_clearance", "vessel maneuvering", "mooring loads", "shore-power electrical dynamics", "terminal capacity derived from real operations"
    ])?;
    if !super::bool_value(super::field(scope, "missing_or_unsupported_is_never_zero", "model_scope")?, "model_scope.missing_or_unsupported_is_never_zero")? {
        return Err("missing_or_unsupported_is_never_zero must remain true".to_owned());
    }

    let constraints = super::array(super::field(root, "hard_constraints", "rehearsal")?, "rehearsal.hard_constraints")?;
    const IDS: [&str; 6] = [
        "water_level_envelope_supported_and_satisfied", "energy_reserve_minimum_satisfied",
        "terminal_window_open", "forecast_temporal_applicability",
        "designated_waiting_resource_declared_available", "alternate_capacity_confirmed"
    ];
    if constraints.len() != IDS.len() {
        return Err(format!("expected {} v0.1 constraints, found {}", IDS.len(), constraints.len()));
    }
    for (index, (constraint, id)) in constraints.iter().zip(IDS).enumerate() {
        let path = format!("rehearsal.hard_constraints[{index}]");
        let actual_id = text_field(constraint, "constraint_id", &path)?;
        if actual_id != id {
            return Err(format!("{path}.constraint_id: unexpected or reordered ID {actual_id:?}"));
        }
        let status_key = match id {
            "water_level_envelope_supported_and_satisfied" | "terminal_window_open" | "designated_waiting_resource_declared_available" => "missing_state",
            "energy_reserve_minimum_satisfied" => "unsupported_state",
            "forecast_temporal_applicability" => "invalid_or_missing_state",
            "alternate_capacity_confirmed" => "default_state",
            _ => unreachable!(),
        };
        let mut keys = vec!["constraint_id", "kind", "unit", "source", status_key];
        super::exact_keys(constraint, &keys, &path)?;
        let expected_kind = if id == "forecast_temporal_applicability" { "evidence_gate" } else { "hard" };
        super::expect_text(super::field(constraint, "kind", &path)?, &format!("{path}.kind"), expected_kind)?;
        let expected_status = if id == "energy_reserve_minimum_satisfied" { "UNSUPPORTED" } else if id == "alternate_capacity_confirmed" { "INDETERMINATE" } else { "INDETERMINATE" };
        super::expect_text(super::field(constraint, status_key, &path)?, &format!("{path}.{status_key}"), expected_status)?;
        let unit = super::field(constraint, "unit", &path)?;
        if id == "forecast_temporal_applicability" {
            super::expect_text(unit, &format!("{path}.unit"), "seconds")?;
        } else if !matches!(unit, super::JsonValue::Null) {
            return Err(format!("{path}.unit must remain null for abstract state constraints"));
        }
        let _ = text_field(constraint, "source", &path)?;
    }

    let properties = super::array(super::field(root, "expected_global_properties", "rehearsal")?, "rehearsal.expected_global_properties")?;
    if properties.len() != 7 {
        return Err("expected seven global non-escalation/metamorphic properties".to_owned());
    }
    for (index, property) in properties.iter().enumerate() {
        super::require_nonempty_text(property, &format!("rehearsal.expected_global_properties[{index}]"))?;
    }
    Ok(())
}

fn validate_abstract_inputs(value: &super::JsonValue, path: &str) -> Result<AbstractInputs, String> {
    super::exact_keys(value, INPUT_KEYS, path)?;
    let input = AbstractInputs {
        water: text_field(value, "water_level_envelope_state", path)?,
        energy: text_field(value, "energy_reserve_state", path)?,
        terminal: text_field(value, "terminal_window_state", path)?,
        forecast: text_field(value, "forecast_evidence_state", path)?,
        waiting: text_field(value, "waiting_resource_state", path)?,
        alternate_capacity: text_field(value, "alternate_capacity_state", path)?,
    };

    if !["within_declared_limit", "outside_declared_limit", "unknown"]
        .contains(&input.water.as_str())
        || !["adequate", "inadequate_for_declared_plan"].contains(&input.energy.as_str())
        || !["open", "closed"].contains(&input.terminal.as_str())
        || !["temporally_applicable_not_admitted", "invalidated"]
            .contains(&input.forecast.as_str())
        || !["declared_available", "unknown_due_to_shared_dependency"]
            .contains(&input.waiting.as_str())
        || !["unconfirmed", "confirmed"].contains(&input.alternate_capacity.as_str())
    {
        return Err(format!("{path}: unknown abstract input state; refusing to guess"));
    }
    Ok(input)
}

fn disposition(plan_id: &str, input: &AbstractInputs) -> Result<(&'static str, Vec<&'static str>), String> {
    match plan_id {
        "continue_original_slot" => {
            // A known hard-constraint violation takes precedence over missing
            // evidence because this candidate is already infeasible in the case.
            if input.terminal == "closed" {
                Ok(("INFEASIBLE", vec!["terminal_window_open"]))
            } else if input.water == "outside_declared_limit" {
                Ok((
                    "INFEASIBLE",
                    vec!["water_level_envelope_supported_and_satisfied"],
                ))
            } else if input.energy == "inadequate_for_declared_plan" {
                Ok(("INFEASIBLE", vec!["energy_reserve_minimum_satisfied"]))
            } else if input.forecast != "temporally_applicable_not_admitted"
                || input.water == "unknown"
            {
                let mut blockers = Vec::new();
                if input.forecast != "temporally_applicable_not_admitted" {
                    blockers.push("forecast_temporal_applicability");
                }
                if input.water == "unknown" {
                    blockers.push("water_level_envelope_supported_and_satisfied");
                }
                Ok(("INDETERMINATE", blockers))
            } else {
                Ok(("FEASIBLE_WITHIN_DECLARED_MODEL", Vec::new()))
            }
        }
        "hold_and_reassess" => {
            if input.waiting == "unknown_due_to_shared_dependency" {
                Ok((
                    "INDETERMINATE",
                    vec!["designated_waiting_resource_declared_available"],
                ))
            } else if input.energy == "inadequate_for_declared_plan" {
                Ok((
                    "UNSUPPORTED",
                    vec!["energy behaviour while holding is outside the model scope"],
                ))
            } else {
                Ok(("FEASIBLE_WITHIN_DECLARED_MODEL", Vec::new()))
            }
        }
        "prepare_alternate_terminal" => {
            if input.alternate_capacity != "confirmed" {
                Ok(("INDETERMINATE", vec!["alternate_capacity_confirmed"]))
            } else {
                // v0.1 has no admitted alternate-terminal physics. Capacity
                // confirmation alone must not silently qualify the alternative.
                Ok((
                    "UNSUPPORTED",
                    vec!["alternate-terminal feasibility model not admitted"],
                ))
            }
        }
        _ => Err(format!("unknown plan ID {plan_id:?}; refusing to evaluate")),
    }
}

fn evaluate_bytes(bytes: &[u8], source_commit: &str) -> Result<RehearsalReport, String> {
    if bytes.len() > MAX_SCENARIO_BYTES {
        return Err(format!("scenario fixture exceeds {MAX_SCENARIO_BYTES}-byte bound"));
    }
    let root = super::parse_strict(bytes)?;
    super::exact_keys(&root, ROOT_KEYS, "rehearsal")?;
    super::expect_text(
        super::field(&root, "schema_id", "rehearsal")?,
        "rehearsal.schema_id",
        "luminous.symtropy.maritime-disruption-rehearsal",
    )?;
    super::expect_text(
        super::field(&root, "schema_version", "rehearsal")?,
        "rehearsal.schema_version",
        "0.1.0",
    )?;
    super::expect_text(
        super::field(&root, "fixture_status", "rehearsal")?,
        "rehearsal.fixture_status",
        "synthetic_contract_evaluable_physics_not_executed",
    )?;
    super::expect_text(
        super::field(&root, "source_kind", "rehearsal")?,
        "rehearsal.source_kind",
        "synthetic_fixture",
    )?;
    let fixture_id = text_field(&root, "fixture_id", "rehearsal")?;
    validate_static_contract(&root)?;

    let ceiling = super::field(&root, "claim_ceiling", "rehearsal")?;
    super::exact_keys(ceiling, AUTHORITY_KEYS, "rehearsal.claim_ceiling")?;
    super::expect_text(
        super::field(ceiling, "engineering_admission", "claim_ceiling")?,
        "claim_ceiling.engineering_admission",
        "NotAdmitted",
    )?;
    for key in [
        "navigation_authorized",
        "physical_truth_claim_authorized",
        "berth_availability_inference_authorized",
        "reservation_or_custody_event_authorized",
        "physical_execution_authorized",
    ] {
        if super::bool_value(
            super::field(ceiling, key, "claim_ceiling")?,
            &format!("claim_ceiling.{key}"),
        )? {
            return Err(format!("claim_ceiling.{key} must remain false"));
        }
    }

    let execution = super::field(&root, "execution", "rehearsal")?;
    super::exact_keys(
        execution,
        &[
            "executed",
            "rust_build_test_status",
            "independent_oracle_status",
            "canonical_replay_status",
            "real_source_capture_status",
            "expected_status",
        ],
        "rehearsal.execution",
    )?;
    if super::bool_value(
        super::field(execution, "executed", "rehearsal.execution")?,
        "rehearsal.execution.executed",
    )? {
        return Err("design fixture must not claim that the physical scenario executed".to_owned());
    }

    let plans = super::array(
        super::field(&root, "candidate_plans", "rehearsal")?,
        "rehearsal.candidate_plans",
    )?;
    if plans.len() != PLAN_IDS.len() {
        return Err(format!(
            "expected {} fixed v0.1 candidate plans, found {}",
            PLAN_IDS.len(),
            plans.len()
        ));
    }
    for (index, plan) in plans.iter().enumerate() {
        let path = format!("rehearsal.candidate_plans[{index}]");
        let expected_plan_keys: &[&str] = if index == 2 {
            &["plan_id", "kind", "preconditions", "side_effects", "not_a_booking", "preparation_does_not_reserve_capacity"]
        } else {
            &["plan_id", "kind", "preconditions", "side_effects", "not_a_booking"]
        };
        super::exact_keys(plan, expected_plan_keys, &path)?;
        let plan_id = text_field(plan, "plan_id", &path)?;
        if plan_id != PLAN_IDS[index] {
            return Err(format!("{path}.plan_id: unexpected plan ID {plan_id:?}"));
        }
        super::expect_text(
            super::field(plan, "kind", &path)?,
            &format!("{path}.kind"),
            "simulated_intent",
        )?;
        if !super::bool_value(
            super::field(plan, "not_a_booking", &path)?,
            &format!("{path}.not_a_booking"),
        )? {
            return Err(format!("{path}.not_a_booking must be true"));
        }
        let preconditions = super::array(super::field(plan, "preconditions", &path)?, &format!("{path}.preconditions"))?;
        if preconditions.is_empty() {
            return Err(format!("{path}.preconditions cannot be empty"));
        }
        for (precondition_index, precondition) in preconditions.iter().enumerate() {
            super::require_nonempty_text(precondition, &format!("{path}.preconditions[{precondition_index}]"))?;
        }
        if index == 2 && !super::bool_value(super::field(plan, "preparation_does_not_reserve_capacity", &path)?, &format!("{path}.preparation_does_not_reserve_capacity"))? {
            return Err("preparing an alternate terminal must not reserve capacity".to_owned());
        }
        if !super::array(
            super::field(plan, "side_effects", &path)?,
            &format!("{path}.side_effects"),
        )?
        .is_empty()
        {
            return Err(format!("{path}.side_effects must remain empty"));
        }
    }

    let cases = super::array(
        super::field(&root, "scenario_cases", "rehearsal")?,
        "rehearsal.scenario_cases",
    )?;
    if cases.len() != CASE_IDS.len() {
        return Err(format!(
            "expected {} frozen v0.1 scenario cases, found {}",
            CASE_IDS.len(),
            cases.len()
        ));
    }

    let mut dispositions = Vec::with_capacity(cases.len() * plans.len());
    let mut per_plan_all_required_feasible = vec![true; plans.len()];
    for (case_index, case) in cases.iter().enumerate() {
        let case_path = format!("rehearsal.scenario_cases[{case_index}]");
        let case_id = text_field(case, "case_id", &case_path)?;
        let expected_case_keys: &[&str] = match case_id.as_str() {
            "baseline_declared_inputs" | "shore_power_unavailable" =>
                &["case_id", "required", "disturbances", "abstract_inputs", "expected_plan_dispositions", "expected_blockers"],
            "adverse_water_level_envelope" =>
                &["case_id", "required", "disturbances", "abstract_inputs", "expected_plan_dispositions", "expected_blockers", "interpretation"],
            "forecast_invalidated_before_decision" | "common_cause_terminal_and_power_disruption" =>
                &["case_id", "required", "disturbances", "abstract_inputs", "expected_plan_dispositions", "expected_blockers", "metamorphic_property"],
            _ => return Err(format!("{case_path}.case_id: unknown case {case_id:?}")),
        };
        super::exact_keys(case, expected_case_keys, &case_path)?;
        if case_id != CASE_IDS[case_index] {
            return Err(format!("{case_path}.case_id: unexpected case ID {case_id:?}"));
        }
        if !super::bool_value(
            super::field(case, "required", &case_path)?,
            &format!("{case_path}.required"),
        )? {
            return Err(format!(
                "{case_path}.required must remain true in the v0.1 required set"
            ));
        }
        let input = validate_abstract_inputs(
            super::field(case, "abstract_inputs", &case_path)?,
            &format!("{case_path}.abstract_inputs"),
        )?;
        let expected = super::object(
            super::field(case, "expected_plan_dispositions", &case_path)?,
            &format!("{case_path}.expected_plan_dispositions"),
        )?;
        if expected.len() != PLAN_IDS.len() || expected.keys().any(|key| !PLAN_IDS.contains(&key.as_str())) {
            return Err(format!("{case_path}.expected_plan_dispositions must contain exactly the three known plan IDs"));
        }
        let expected_blockers = super::object(
            super::field(case, "expected_blockers", &case_path)?,
            &format!("{case_path}.expected_blockers"),
        )?;
        if expected_blockers.keys().any(|key| !PLAN_IDS.contains(&key.as_str())) {
            return Err(format!("{case_path}.expected_blockers contains an unknown plan ID"));
        }

        for (plan_index, plan_id) in PLAN_IDS.iter().enumerate() {
            let (status, blockers) = disposition(plan_id, &input)?;
            let expected_status = expected
                .get(*plan_id)
                .ok_or_else(|| format!("{case_path}: missing expected status for {plan_id}"))?;
            super::expect_text(
                expected_status,
                &format!("{case_path}.expected_plan_dispositions.{plan_id}"),
                status,
            )?;
            let expected_blocker_values = expected_blockers.get(*plan_id);
            if blockers.is_empty() {
                if expected_blocker_values.is_some() {
                    return Err(format!("{case_path}.expected_blockers.{plan_id} must be absent when no blockers exist"));
                }
            } else {
                let declared = expected_blocker_values.ok_or_else(|| format!("{case_path}.expected_blockers is missing {plan_id}"))?;
                let declared = super::array(declared, &format!("{case_path}.expected_blockers.{plan_id}"))?;
                if declared.len() != blockers.len() {
                    return Err(format!("{case_path}.expected_blockers.{plan_id} count mismatch"));
                }
                for (blocker_index, (value, expected_blocker)) in declared.iter().zip(&blockers).enumerate() {
                    super::expect_text(value, &format!("{case_path}.expected_blockers.{plan_id}[{blocker_index}]"), expected_blocker)?;
                }
            }
            if status != "FEASIBLE_WITHIN_DECLARED_MODEL" {
                per_plan_all_required_feasible[plan_index] = false;
            }
            dispositions.push(DispositionReport {
                case_id: case_id.clone(),
                plan_id: (*plan_id).to_owned(),
                status: status.to_owned(),
                blockers: blockers.into_iter().map(str::to_owned).collect(),
            });
        }
        if expected.keys().any(|key| !PLAN_IDS.contains(&key.as_str())) {
            return Err(format!(
                "{case_path}.expected_plan_dispositions contains an unknown plan ID"
            ));
        }
    }

    let robustly_feasible_candidate_plans = PLAN_IDS
        .iter()
        .zip(per_plan_all_required_feasible)
        .filter(|(_, is_feasible)| **is_feasible)
        .map(|(plan_id, _)| (*plan_id).to_owned())
        .collect();

    Ok(RehearsalReport {
        status: "SYNTHETIC_REHEARSAL_CONTRACT_MATCH_NOT_ADMITTED",
        schema_id: "luminous.symtropy.maritime-disruption-rehearsal".to_owned(),
        fixture_id,
        scenario_sha256: super::sha256_bytes(bytes),
        scenario_case_count: cases.len(),
        candidate_plan_count: plans.len(),
        dispositions,
        robustly_feasible_candidate_plans,
        engineering_admission: "NotAdmitted",
        navigation_authorized: false,
        physical_truth_claim_authorized: false,
        berth_availability_inference_authorized: false,
        reservation_or_custody_event_authorized: false,
        physical_execution_authorized: false,
        source_commit_binding: source_commit.to_owned(),
        contract_evaluation_performed: true,
        physical_simulation_performed: false,
    })
}

pub(super) fn run_cli(scenario_path: &str) -> Result<(), String> {
    let metadata = fs::metadata(scenario_path)
        .map_err(|error| format!("cannot stat scenario fixture {scenario_path}: {error}"))?;
    if metadata.len() > MAX_SCENARIO_BYTES as u64 {
        return Err(format!("scenario fixture exceeds {MAX_SCENARIO_BYTES}-byte bound"));
    }
    let bytes = fs::read(scenario_path)
        .map_err(|error| format!("cannot read scenario fixture {scenario_path}: {error}"))?;
    let source_commit =
        env::var("MARITIME_SOURCE_COMMIT").unwrap_or_else(|_| "UNBOUND_LOCAL_SOURCE".to_owned());
    let report = evaluate_bytes(&bytes, &source_commit)?;
    let output = serde_json::to_string(&report)
        .map_err(|error| format!("cannot serialize rehearsal report: {error}"))?;
    println!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &[u8] =
        include_bytes!("../../../../docs/tech/MARITIME_DISRUPTION_REHEARSAL_V0_1.json");

    // Intentionally separate test oracle: direct serde_json parsing and a distinct
    // classifier. It does not use the production JsonValue parser or disposition().
    fn oracle_status(plan: &str, case: &Value) -> &'static str {
        let input = &case["abstract_inputs"];
        let water = input["water_level_envelope_state"].as_str().unwrap();
        let energy = input["energy_reserve_state"].as_str().unwrap();
        let terminal = input["terminal_window_state"].as_str().unwrap();
        let forecast = input["forecast_evidence_state"].as_str().unwrap();
        let waiting = input["waiting_resource_state"].as_str().unwrap();
        let alternate = input["alternate_capacity_state"].as_str().unwrap();

        match plan {
            "continue_original_slot" if terminal == "closed" => "INFEASIBLE",
            "continue_original_slot" if water == "outside_declared_limit" => "INFEASIBLE",
            "continue_original_slot" if energy == "inadequate_for_declared_plan" => "INFEASIBLE",
            "continue_original_slot"
                if water == "unknown" || forecast != "temporally_applicable_not_admitted" =>
            {
                "INDETERMINATE"
            }
            "continue_original_slot" => "FEASIBLE_WITHIN_DECLARED_MODEL",
            "hold_and_reassess" if waiting != "declared_available" => "INDETERMINATE",
            "hold_and_reassess" if energy != "adequate" => "UNSUPPORTED",
            "hold_and_reassess" => "FEASIBLE_WITHIN_DECLARED_MODEL",
            "prepare_alternate_terminal" if alternate != "confirmed" => "INDETERMINATE",
            "prepare_alternate_terminal" => "UNSUPPORTED",
            _ => "UNSUPPORTED",
        }
    }

    #[test]
    fn frozen_fixture_matches_independent_oracle() {
        let fixture: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON parses");
        let report = evaluate_bytes(FIXTURE, "test-source-commit").expect("fixture validates");
        let cases = fixture["scenario_cases"].as_array().expect("cases are array");
        assert_eq!(report.scenario_case_count, 5);
        assert_eq!(report.candidate_plan_count, 3);
        assert_eq!(report.dispositions.len(), 15);

        for case in cases {
            let case_id = case["case_id"].as_str().unwrap();
            for plan in PLAN_IDS {
                let expected = oracle_status(plan, case);
                let declared = case["expected_plan_dispositions"][plan]
                    .as_str()
                    .expect("declared expectation");
                assert_eq!(declared, expected, "frozen oracle: {case_id} / {plan}");
                let actual = report
                    .dispositions
                    .iter()
                    .find(|d| d.case_id == case_id && d.plan_id == *plan)
                    .expect("production disposition exists");
                assert_eq!(actual.status, expected, "runtime oracle: {case_id} / {plan}");
            }
        }
        assert!(report.robustly_feasible_candidate_plans.is_empty());
        assert_eq!(report.engineering_admission, "NotAdmitted");
        assert!(!report.navigation_authorized);
        assert!(!report.physical_truth_claim_authorized);
        assert!(!report.berth_availability_inference_authorized);
        assert!(!report.reservation_or_custody_event_authorized);
        assert!(!report.physical_execution_authorized);
        assert!(!report.physical_simulation_performed);
    }

    #[test]
    fn identical_inputs_produce_identical_report_bytes() {
        let first = evaluate_bytes(FIXTURE, "fixed-test-source").unwrap();
        let second = evaluate_bytes(FIXTURE, "fixed-test-source").unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }

    #[test]
    fn authority_escalation_is_rejected() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["claim_ceiling"]["navigation_authorized"] = Value::Bool(true);
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(evaluate_bytes(&changed, "test").is_err());
    }

    #[test]
    fn unknown_abstract_state_is_rejected() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["scenario_cases"][0]["abstract_inputs"]["water_level_envelope_state"] =
            Value::String("probably_ok".to_owned());
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(evaluate_bytes(&changed, "test").is_err());
    }

    #[test]
    fn unknown_root_field_is_rejected() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["surprise_authority"] = Value::Bool(true);
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(evaluate_bytes(&changed, "test").is_err());
    }
}
