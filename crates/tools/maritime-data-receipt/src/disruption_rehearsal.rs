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
        "design_fixture_not_yet_executable",
    )?;
    super::expect_text(
        super::field(&root, "source_kind", "rehearsal")?,
        "rehearsal.source_kind",
        "synthetic_fixture",
    )?;
    let fixture_id = text_field(&root, "fixture_id", "rehearsal")?;

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
        .filter_map(|(plan_id, is_feasible)| is_feasible.then(|| (*plan_id).to_owned()))
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
