// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Validates policy-impact evaluation admissibility declarations. It does not
// estimate treatment effects or authenticate causal identification.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{env, fs, process};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-policy-impact-registration-v0.1.schema.json");
#[cfg(test)]
const FIXTURE_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/pirb-policy-impact-before-after-not-identified-synthetic-v0.1.json");

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn option_value(args: &[String], name: &str) -> Result<Option<String>, String> {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return Ok(None);
    };
    args.get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .map(Some)
        .ok_or_else(|| format!("argument {name} requires a value"))
}

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn is_placeholder_digest(digest: &str) -> bool {
    digest
        .chars()
        .next()
        .is_some_and(|first| digest.chars().all(|ch| ch == first))
}

fn gates_for_independent_review(registration: &Value) -> Vec<&'static str> {
    let mut missing = Vec::new();
    let design = &registration["design"];
    let counterfactual = &registration["counterfactual"];
    let actions = &registration["action_lane"];
    let outcome = &registration["outcome_lane"];
    let governance = &registration["governance"];
    let policy = &registration["policy_comparison"];
    let forecast = &registration["forecast_lane"];

    let design_type = string_at(design, "design_type").unwrap_or_default();
    let assignment = string_at(design, "assignment_mechanism").unwrap_or_default();
    let assignment_compatible = match design_type {
        "randomized_trial" => assignment == "randomized",
        "difference_in_differences" | "synthetic_control" | "interrupted_time_series" => {
            matches!(assignment, "as_if_random" | "observational")
        }
        _ => false,
    };
    if !assignment_compatible {
        missing.push("design and assignment mechanism are not a supported randomized/quasi-experimental pairing");
    }
    if design
        .get("independent_design_review_verified")
        .and_then(Value::as_bool)
        != Some(true)
    {
        missing.push("independent design review is not verified");
    }
    if string_at(design, "analysis_plan_reference").is_none_or(str::is_empty)
        || string_at(design, "analysis_plan_sha256").is_none_or(is_placeholder_digest)
    {
        missing.push("a non-placeholder preregistered analysis plan reference and digest are required");
    }
    if string_at(design, "estimand_reference").is_none_or(str::is_empty) {
        missing.push("a prespecified causal estimand reference is missing");
    }
    if string_at(design, "comparison_strategy").is_none_or(str::is_empty) {
        missing.push("a comparison strategy is missing");
    }
    if counterfactual["counterfactual_defined"].as_bool() != Some(true) {
        missing.push("the counterfactual is not defined");
    }
    if counterfactual["treatment_timing_defined"].as_bool() != Some(true) {
        missing.push("treatment timing is not defined");
    }
    if string_at(counterfactual, "identification_assumptions").is_none_or(str::is_empty)
        || string_at(counterfactual, "diagnostics_plan").is_none_or(str::is_empty)
        || string_at(counterfactual, "sensitivity_analysis_plan").is_none_or(str::is_empty)
    {
        missing.push("identification assumptions, diagnostics, or sensitivity analysis plan is missing");
    }
    if !matches!(
        string_at(design, "interference_assessment"),
        Some("explicitly_modelled" | "bounded_and_tested")
    ) {
        missing.push("interference/spillover is not modelled or bounded and tested");
    }
    if outcome["reporting_shift_assessment"] == "not_assessed" {
        missing.push("outcome reporting shift has not been assessed");
    }
    if string_at(outcome, "outcome_measurement_process").is_none_or(str::is_empty)
        || string_at(outcome, "missingness_policy").is_none_or(str::is_empty)
    {
        missing.push("outcome measurement or missingness policy is missing");
    }
    let issue_cutoff = forecast["issue_cutoff_index"].as_i64().unwrap_or_default();
    let follow_up_end = outcome["follow_up_end_index"].as_i64().unwrap_or_default();
    if follow_up_end <= issue_cutoff {
        missing.push("outcome follow-up must extend beyond forecast issue cutoff");
    }
    if string_at(outcome, "outcome_release_sha256").is_none_or(is_placeholder_digest) {
        missing.push("a non-placeholder frozen outcome release digest is required");
    }
    if string_at(forecast, "forecast_artifact_sha256").is_none_or(is_placeholder_digest) {
        missing.push("a non-placeholder forecast artifact digest is required");
    }
    if actions["offer_assignment_recorded"].as_bool() != Some(true)
        || actions["delivery_recorded"].as_bool() != Some(true)
        || actions["receipt_or_adherence_recorded"].as_bool() != Some(true)
    {
        missing.push("offer assignment, delivered action, and receipt/adherence must be recorded");
    }
    if actions["no_action_baseline_included"].as_bool() != Some(true) {
        missing.push("a no-action baseline is not included");
    }
    let policies = policy["comparison_policies"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    if !policies.iter().any(|value| *value == "no_action") {
        missing.push("comparison policies must include no_action");
    }
    if policy["subgroup_harm_review"].as_bool() != Some(true) {
        missing.push("subgroup benefit/harm review is not declared");
    }
    if policy["thresholds_preregistered"].as_bool() != Some(true) {
        missing.push("decision thresholds must be preregistered");
    }
    if string_at(governance, "data_authority_status") != Some("authorized") {
        missing.push("data authority is not verified as authorized");
    }
    if string_at(governance, "disclosure_review_status") != Some("approved") {
        missing.push("disclosure review is not approved");
    }
    if string_at(governance, "publication_review_status") != Some("approved") {
        missing.push("publication review is not approved");
    }
    if string_at(governance, "data_authority_reference").is_none_or(str::is_empty)
        || string_at(governance, "disclosure_review_reference").is_none_or(str::is_empty)
    {
        missing.push("data-authority or disclosure-review reference is missing");
    }

    missing
}

fn semantic_errors(registration: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let evidence_scope = string_at(registration, "evidence_scope").unwrap_or_default();
    let design_type = string_at(&registration["design"], "design_type").unwrap_or_default();
    let effect_status = string_at(&registration["causal_lane"], "effect_status").unwrap_or_default();
    let eligibility = string_at(
        &registration["causal_lane"],
        "independent_review_eligibility",
    )
    .unwrap_or_default();

    if registration["forecast_lane"]["heldout_scoring_separate"].as_bool() != Some(true)
        || registration["causal_lane"]["forecast_and_impact_lanes_separate"].as_bool()
            != Some(true)
    {
        errors.push("forecast verification and policy-impact evaluation must remain separate lanes".to_owned());
    }
    if registration["causal_lane"]["causal_effect_estimated"].as_bool() != Some(false)
        || !registration["causal_lane"]["effect_estimate"].is_null()
    {
        errors.push("this validator never accepts a claimed causal effect estimate".to_owned());
    }
    if registration["forecast_lane"]["issue_cutoff_index"]
        .as_i64()
        .unwrap_or_default()
        >= registration["outcome_lane"]["follow_up_end_index"]
            .as_i64()
            .unwrap_or_default()
    {
        errors.push("follow-up end index must be later than forecast issue cutoff".to_owned());
    }

    if !registration["policy_comparison"]["comparison_policies"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|policy| policy == "no_action")
    {
        errors.push("policy comparison must include a no_action baseline".to_owned());
    }

    match evidence_scope {
        "synthetic" => {
            if string_at(&registration["governance"], "data_authority_status")
                != Some("not_applicable_synthetic")
                || string_at(&registration["governance"], "disclosure_review_status")
                    != Some("not_applicable_synthetic")
            {
                errors.push("synthetic registrations must identify data governance as not_applicable_synthetic".to_owned());
            }
            if eligibility != "not_identified" {
                errors.push("synthetic evidence cannot qualify a real-world impact evaluation for independent review".to_owned());
            }
            if design_type == "synthetic_known_truth" {
                if effect_status != "synthetic_method_test_only" {
                    errors.push("synthetic_known_truth design must use effect_status=synthetic_method_test_only".to_owned());
                }
            } else if effect_status != "unestimated" {
                errors.push("synthetic before/after or observational data must leave effect_status=unestimated".to_owned());
            }
        }
        "authorized_historical" => {
            if effect_status == "synthetic_method_test_only" {
                errors.push("authorized historical evidence cannot use synthetic_method_test_only status".to_owned());
            }
            if eligibility == "eligible_for_independent_review" {
                let gates = gates_for_independent_review(registration);
                if !gates.is_empty() {
                    errors.push(format!(
                        "registration declares independent-review eligibility without satisfying gates: {}",
                        gates.join("; ")
                    ));
                }
            }
        }
        _ => errors.push("unsupported evidence_scope".to_owned()),
    }

    if eligibility == "eligible_for_independent_review" && evidence_scope != "authorized_historical" {
        errors.push("independent-review eligibility requires authorized_historical evidence".to_owned());
    }
    if !matches!(eligibility, "not_identified" | "eligible_for_independent_review") {
        errors.push("unsupported independent_review_eligibility".to_owned());
    }
    if !matches!(effect_status, "unestimated" | "synthetic_method_test_only") {
        errors.push("unsupported effect_status; this validator does not estimate causal effects".to_owned());
    }

    errors.sort();
    errors
}

fn validate_contract(input: &[u8], source_commit: &str) -> Result<Value, Vec<String>> {
    if source_commit != "UNPINNED_LOCAL_RUN"
        && (source_commit.len() != 40
            || !source_commit
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()))
    {
        return Err(vec![
            "source commit must be a 40-character lowercase Git SHA or UNPINNED_LOCAL_RUN"
                .to_owned(),
        ]);
    }

    let registration: Value = serde_json::from_slice(input)
        .map_err(|error| vec![format!("policy-impact registration JSON is invalid: {error}")])?;
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES).map_err(|error| {
        vec![format!("embedded policy-impact schema JSON is invalid: {error}")]
    })?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!(
            "Draft 2020-12 schema meta-validation failed: {error}"
        )]
    })?;
    let validator = jsonschema::draft202012::new(&schema).map_err(|error| {
        vec![format!("cannot build policy-impact schema validator: {error}")]
    })?;
    let mut errors = validator
        .iter_errors(&registration)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        errors.sort();
        return Err(errors);
    }

    let semantic = semantic_errors(&registration);
    if !semantic.is_empty() {
        return Err(semantic);
    }

    let eligibility = string_at(
        &registration["causal_lane"],
        "independent_review_eligibility",
    )
    .expect("schema validated independent_review_eligibility");
    let gates = if registration["evidence_scope"] == "authorized_historical" {
        gates_for_independent_review(&registration)
    } else {
        vec!["synthetic-only registration cannot establish real-world causal eligibility"]
    };
    let registration_id =
        string_at(&registration, "registration_id").expect("schema validated registration_id");
    let input_sha256 = sha256_hex(input);
    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let source_commit_sha256 = sha256_hex(
        format!(
            "pirb-policy-impact-admissibility-receipt-v0.1\n{registration_id}\n{source_commit}\n{schema_sha256}\n{input_sha256}\n{eligibility}"
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.1",
        "receipt_type": "policy_impact_admissibility",
        "registration_id": registration_id,
        "evidence_scope": registration["evidence_scope"],
        "source_commit": source_commit,
        "source_commit_pinned": source_commit != "UNPINNED_LOCAL_RUN",
        "input_sha256": input_sha256,
        "schema_sha256": schema_sha256,
        "forecast_lane": {
            "heldout_scoring_separate": registration["forecast_lane"]["heldout_scoring_separate"],
            "forecast_artifact_sha256": registration["forecast_lane"]["forecast_artifact_sha256"],
            "issue_cutoff_index": registration["forecast_lane"]["issue_cutoff_index"]
        },
        "action_lane": {
            "offer_assignment_recorded": registration["action_lane"]["offer_assignment_recorded"],
            "delivery_recorded": registration["action_lane"]["delivery_recorded"],
            "receipt_or_adherence_recorded": registration["action_lane"]["receipt_or_adherence_recorded"],
            "no_action_baseline_included": registration["action_lane"]["no_action_baseline_included"]
        },
        "outcome_lane": {
            "outcome_release_sha256": registration["outcome_lane"]["outcome_release_sha256"],
            "follow_up_end_index": registration["outcome_lane"]["follow_up_end_index"],
            "reporting_shift_assessment": registration["outcome_lane"]["reporting_shift_assessment"]
        },
        "design_type": registration["design"]["design_type"],
        "independent_review_eligibility": eligibility,
        "missing_independent_review_gates": gates,
        "forecast_performance_estimated": false,
        "policy_value_estimated": false,
        "causal_effect_estimated": false,
        "effect_estimate": Value::Null,
        "causal_impact_established": false,
        "live_decision_authority": false,
        "claim_ceiling": "contract_and_admissibility_only",
        "receipt_commitment_sha256": source_commit_sha256
    }))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?.ok_or_else(|| {
        "usage: pirb-policy-impact-admissibility --input REGISTRATION.json [--source-commit SHA]"
            .to_owned()
    })?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("cannot read policy-impact registration {input_path}: {error}"))?;
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let receipt =
        validate_contract(&input, &source_commit).map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize policy-impact receipt: {error}"))?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_slice(FIXTURE_BYTES)
            .expect("synthetic before/after policy-impact fixture is valid JSON")
    }

    fn validate(value: &Value) -> Result<Value, Vec<String>> {
        validate_contract(
            &serde_json::to_vec(value).expect("fixture serializes"),
            "0123456789abcdef0123456789abcdef01234567",
        )
    }

    #[test]
    fn naive_before_after_fixture_is_valid_but_impact_remains_unestimated() {
        let receipt = validate(&fixture()).expect("negative-control registration is valid");
        assert_eq!(receipt["design_type"], "before_after");
        assert_eq!(receipt["independent_review_eligibility"], "not_identified");
        assert_eq!(receipt["forecast_performance_estimated"], false);
        assert_eq!(receipt["policy_value_estimated"], false);
        assert_eq!(receipt["causal_effect_estimated"], false);
        assert_eq!(receipt["effect_estimate"], Value::Null);
        assert_eq!(receipt["causal_impact_established"], false);
        assert_eq!(receipt["live_decision_authority"], false);
    }

    #[test]
    fn synthetic_registration_cannot_claim_independent_historical_review_eligibility() {
        let mut value = fixture();
        value["causal_lane"]["independent_review_eligibility"] =
            json!("eligible_for_independent_review");
        let errors = validate(&value).expect_err("synthetic data is not external causal evidence");
        assert!(errors.iter().any(|error| error.contains("cannot qualify a real-world impact evaluation")));
    }

    #[test]
    fn before_after_observational_design_cannot_claim_review_eligibility() {
        let mut value = fixture();
        value["causal_lane"]["independent_review_eligibility"] =
            json!("eligible_for_independent_review");
        value["evidence_scope"] = json!("authorized_historical");
        value["governance"]["data_authority_status"] = json!("authorized");
        value["governance"]["disclosure_review_status"] = json!("approved");
        value["governance"]["publication_review_status"] = json!("approved");
        value["governance"]["data_authority_reference"] = json!("reviewed-authority-record");
        value["governance"]["disclosure_review_reference"] = json!("reviewed-disclosure-record");
        let errors = validate(&value).expect_err("before/after lacks a supported identification design");
        assert!(errors.iter().any(|error| error.contains("without satisfying gates")));
        assert!(errors.iter().any(|error| error.contains("not a supported randomized/quasi-experimental pairing")));
        assert!(errors.iter().any(|error| error.contains("counterfactual is not defined")));
    }

    #[test]
    fn eligible_historical_trial_only_becomes_reviewable_after_all_gates() {
        let mut value = fixture();
        value["evidence_scope"] = json!("authorized_historical");
        value["forecast_lane"]["forecast_artifact_sha256"] =
            json!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        value["outcome_lane"]["outcome_release_sha256"] =
            json!("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");
        value["outcome_lane"]["reporting_shift_assessment"] = json!("assessed_no_material_shift");
        value["design"]["design_type"] = json!("randomized_trial");
        value["design"]["assignment_mechanism"] = json!("randomized");
        value["design"]["analysis_plan_reference"] = json!("registered-analysis-plan");
        value["design"]["analysis_plan_sha256"] =
            json!("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789");
        value["design"]["independent_design_review_verified"] = json!(true);
        value["design"]["interference_assessment"] = json!("bounded_and_tested");
        value["counterfactual"]["counterfactual_defined"] = json!(true);
        value["counterfactual"]["treatment_timing_defined"] = json!(true);
        value["action_lane"]["receipt_or_adherence_recorded"] = json!(true);
        value["policy_comparison"]["subgroup_harm_review"] = json!(true);
        value["causal_lane"]["independent_review_eligibility"] =
            json!("eligible_for_independent_review");
        value["governance"]["data_authority_status"] = json!("authorized");
        value["governance"]["disclosure_review_status"] = json!("approved");
        value["governance"]["publication_review_status"] = json!("approved");
        value["governance"]["data_authority_reference"] = json!("reviewed-authority-record");
        value["governance"]["disclosure_review_reference"] = json!("reviewed-disclosure-record");
        let receipt = validate(&value).expect("complete declarations may enter independent review");
        assert_eq!(receipt["independent_review_eligibility"], "eligible_for_independent_review");
        assert_eq!(receipt["causal_effect_estimated"], false);
        assert_eq!(receipt["effect_estimate"], Value::Null);
        assert_eq!(receipt["causal_impact_established"], false);
    }

    #[test]
    fn missing_adherence_record_blocks_historical_review_eligibility() {
        let mut value = fixture();
        value["evidence_scope"] = json!("authorized_historical");
        value["causal_lane"]["independent_review_eligibility"] =
            json!("eligible_for_independent_review");
        value["design"]["design_type"] = json!("randomized_trial");
        value["design"]["assignment_mechanism"] = json!("randomized");
        value["design"]["analysis_plan_sha256"] =
            json!("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789");
        value["design"]["independent_design_review_verified"] = json!(true);
        value["counterfactual"]["counterfactual_defined"] = json!(true);
        value["counterfactual"]["treatment_timing_defined"] = json!(true);
        value["outcome_lane"]["reporting_shift_assessment"] = json!("assessed_no_material_shift");
        value["action_lane"]["receipt_or_adherence_recorded"] = json!(false);
        let errors = validate(&value).expect_err("assignment alone is not treatment delivery");
        assert!(errors.iter().any(|error| error.contains("without satisfying gates")));
    }

    #[test]
    fn effect_estimates_are_never_accepted_from_input() {
        let mut value = fixture();
        value["causal_lane"]["causal_effect_estimated"] = json!(true);
        value["causal_lane"]["effect_estimate"] = json!(2.3);
        let errors = validate(&value).expect_err("this validator never accepts effect estimates");
        assert!(errors.iter().any(|error| error.contains("never accepts a claimed causal effect estimate")));
    }

    #[test]
    fn missing_no_action_baseline_is_rejected() {
        let mut value = fixture();
        value["action_lane"]["no_action_baseline_included"] = json!(false);
        value["policy_comparison"]["comparison_policies"] =
            json!(["synthetic_current_practice", "intensive_response"]);
        let errors = validate(&value).expect_err("policy regret needs no-action comparator");
        assert!(errors.iter().any(|error| error.contains("no_action baseline")));
    }

    #[test]
    fn followup_must_extend_beyond_forecast_issue_cutoff() {
        let mut value = fixture();
        value["outcome_lane"]["follow_up_end_index"] = json!(10);
        let errors = validate(&value).expect_err("outcome timing must be posterior to forecast issue");
        assert!(errors.iter().any(|error| error.contains("follow-up end index")));
    }
}
