// SPDX-License-Identifier: AGPL-3.0-or-later

//! Score declared action losses across a registered set of alternative models.
//!
//! Example input schema:
//! {
//!   "schema_version": "ANIMAL-MODEL-DECISION-OUTCOMES-V1",
//!   "cvar_alpha": 0.9,
//!   "models": [{
//!     "model_id": "structure_a",
//!     "weight": 0.5,
//!     "action_losses": { "wait": 3.0 },
//!     "admissible_actions": ["wait"]
//!   }]
//! }
//!
//! This evaluates the declared numbers. It does not validate ecological realism,
//! the empirical basis for model weights, or the normative choice of an action.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::process;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use symtropy_animal_model_uncertainty::decision::{
    ActionPreferenceRange, ActionRisk, ModelOutcome, ModelWeightInterval, WeightSensitivity,
    compare_actions_under_weight_uncertainty, score_model_ensemble, score_weight_sensitivity,
};

const SYNTHETIC_EXAMPLE: &str =
    include_str!("../../../../../docs/living-world/ANIMAL_MODEL_DECISION_OUTCOMES_EXAMPLE_V1.json");
const EMBEDDED_INPUT_SCHEMA: &str =
    include_str!("../../../../../docs/living-world/ANIMAL_MODEL_DECISION_OUTCOMES_V1.schema.json");
const EMBEDDED_REPORT_SCHEMA: &str = include_str!(
    "../../../../../docs/living-world/ANIMAL_MODEL_DECISION_RISK_REPORT_V1.schema.json"
);

fn ensure_only_fields(value: &Value, allowed: &[&str], context: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context} must be an object"))?;
    if let Some(field) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("{context} contains unknown field '{field}'"));
    }
    Ok(())
}

type ParsedInput = (
    Vec<ModelOutcome>,
    f64,
    Option<Vec<ModelWeightInterval>>,
    f64,
);

fn parse_input(root: &Value) -> Result<ParsedInput, String> {
    ensure_only_fields(
        root,
        &[
            "schema_version",
            "status",
            "cvar_alpha",
            "practical_loss_difference_margin",
            "scenario",
            "models",
            "model_weight_intervals",
        ],
        "input",
    )?;
    if root.get("schema_version").and_then(Value::as_str)
        != Some("ANIMAL-MODEL-DECISION-OUTCOMES-V1")
    {
        return Err("schema_version must be ANIMAL-MODEL-DECISION-OUTCOMES-V1".to_owned());
    }
    let status = root
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| "status must be a string".to_owned())?;
    if !matches!(
        status,
        "synthetic_fixture_not_ecological_validation" | "declared_model_outcomes_not_validated"
    ) {
        return Err("status must state that these outcomes are not validated".to_owned());
    }
    let _scenario = root
        .get("scenario")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "scenario must be a non-empty string".to_owned())?;
    let cvar_alpha = root
        .get("cvar_alpha")
        .and_then(Value::as_f64)
        .ok_or_else(|| "cvar_alpha must be a number".to_owned())?;
    let practical_loss_difference_margin = root
        .get("practical_loss_difference_margin")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| {
            "practical_loss_difference_margin must be finite and non-negative".to_owned()
        })?;
    let model_values = root
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| "models must be an array".to_owned())?;

    let mut models = Vec::with_capacity(model_values.len());
    for (index, value) in model_values.iter().enumerate() {
        let prefix = format!("models[{index}]");
        ensure_only_fields(
            value,
            &["model_id", "weight", "action_losses", "admissible_actions"],
            &prefix,
        )?;
        let model_id = value
            .get("model_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{prefix}.model_id must be a string"))?
            .to_owned();
        let weight = value
            .get("weight")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("{prefix}.weight must be a number"))?;

        let loss_values = value
            .get("action_losses")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{prefix}.action_losses must be an object"))?;
        let mut action_losses = BTreeMap::new();
        for (action, loss_value) in loss_values {
            let loss = loss_value
                .as_f64()
                .ok_or_else(|| format!("{prefix}.action_losses.{action} must be a number"))?;
            action_losses.insert(action.clone(), loss);
        }

        let action_values = value
            .get("admissible_actions")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{prefix}.admissible_actions must be an array"))?;
        let mut admissible_actions = BTreeSet::new();
        for (action_index, action_value) in action_values.iter().enumerate() {
            let action = action_value.as_str().ok_or_else(|| {
                format!("{prefix}.admissible_actions[{action_index}] must be a string")
            })?;
            admissible_actions.insert(action.to_owned());
        }

        models.push(ModelOutcome {
            model_id,
            weight,
            action_losses,
            admissible_actions,
        });
    }
    let model_weight_intervals = match root.get("model_weight_intervals") {
        None => None,
        Some(Value::Array(values)) => {
            let mut intervals = Vec::with_capacity(values.len());
            for (index, value) in values.iter().enumerate() {
                let prefix = format!("model_weight_intervals[{index}]");
                ensure_only_fields(value, &["model_id", "lower", "upper"], &prefix)?;
                let model_id = value
                    .get("model_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("{prefix}.model_id must be a string"))?
                    .to_owned();
                let lower = value
                    .get("lower")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| format!("{prefix}.lower must be a number"))?;
                let upper = value
                    .get("upper")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| format!("{prefix}.upper must be a number"))?;
                intervals.push(ModelWeightInterval {
                    model_id,
                    lower,
                    upper,
                });
            }
            Some(intervals)
        }
        Some(_) => return Err("model_weight_intervals must be an array when present".to_owned()),
    };

    Ok((
        models,
        cvar_alpha,
        model_weight_intervals,
        practical_loss_difference_margin,
    ))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn score_action_preferences(
    models: &[ModelOutcome],
    scores: &[ActionRisk],
    intervals: Option<&[ModelWeightInterval]>,
    practical_loss_difference_margin: f64,
) -> Result<Vec<ActionPreferenceRange>, Vec<String>> {
    let Some(intervals) = intervals else {
        return Ok(Vec::new());
    };
    let mut comparisons = Vec::new();
    for (index, first) in scores.iter().enumerate() {
        for second in scores.iter().skip(index + 1) {
            comparisons.push(compare_actions_under_weight_uncertainty(
                models,
                &first.action,
                &second.action,
                intervals,
                practical_loss_difference_margin,
            )?);
        }
    }
    Ok(comparisons)
}

struct ReportComponents<'a> {
    cvar_alpha: f64,
    practical_loss_difference_margin: f64,
    scores: &'a [ActionRisk],
    weight_sensitivity: &'a [WeightSensitivity],
    action_preferences: &'a [ActionPreferenceRange],
}

fn build_report(
    raw_input: &str,
    root: &Value,
    models: &[ModelOutcome],
    components: ReportComponents<'_>,
) -> Value {
    json!({
        "schema_version": "ANIMAL-MODEL-DECISION-RISK-REPORT-V1",
        "status": "computed_risk_summary_not_model_validation",
        "cvar_alpha": components.cvar_alpha,
        "practical_loss_difference_margin": components.practical_loss_difference_margin,
        "input_sha256": sha256_hex(raw_input.as_bytes()),
        "scenario": root.get("scenario").and_then(Value::as_str).unwrap_or("unspecified"),
        "model_ids": models.iter().map(|model| model.model_id.as_str()).collect::<Vec<_>>(),
        "model_weights": models.iter().map(|model| json!({ "model_id": model.model_id, "weight": model.weight })).collect::<Vec<Value>>(),
        "model_count": models.len(),
        "scores": components.scores.iter().map(|score| json!({
            "action": score.action,
            "expected_loss": score.expected_loss,
            "worst_case_loss": score.worst_case_loss,
            "cvar_loss": score.cvar_loss,
            "per_model_regret": score.per_model_regret,
            "max_regret": score.max_regret,
            "admissible_across_all_models": score.admissible_across_all_models
        })).collect::<Vec<Value>>(),
        "weight_sensitivity": components.weight_sensitivity.iter().map(|result| json!({
            "action": result.action,
            "minimum_expected_loss": result.minimum_expected_loss,
            "maximum_expected_loss": result.maximum_expected_loss,
            "minimizing_weights": result.minimizing_weights,
            "maximizing_weights": result.maximizing_weights
        })).collect::<Vec<Value>>(),
        "action_preference": components.action_preferences.iter().map(|result| json!({
            "first_action": result.first_action,
            "second_action": result.second_action,
            "first_admissible_across_all_models": result.first_admissible_across_all_models,
            "second_admissible_across_all_models": result.second_admissible_across_all_models,
            "minimum_loss_difference": result.minimum_loss_difference,
            "maximum_loss_difference": result.maximum_loss_difference,
            "minimizing_weights": result.minimizing_weights,
            "maximizing_weights": result.maximizing_weights,
            "stability": result.stability.as_str()
        })).collect::<Vec<Value>>()
    })
}

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!(
            "usage: score_animal_model_ensemble <outcomes.json> (options: --help, --example, --input-schema, --report-schema)"
        );
        process::exit(2);
    };
    if arguments.next().is_some() {
        eprintln!(
            "usage: score_animal_model_ensemble <outcomes.json> (options: --help, --example, --input-schema, --report-schema)"
        );
        process::exit(2);
    }

    match path.to_str() {
        Some("--help" | "-h") => {
            println!(
                r#"Score declared action losses across registered ecological models.
Usage: score_animal_model_ensemble <outcomes.json>
--example       print a synthetic input example
--input-schema  print the expected input schema
--report-schema print the risk report schema

Risk scoring is conditional on the declared model set. This CLI does not validate model-set adequacy."#
            );
            return;
        }
        Some("--example") => {
            print!("{SYNTHETIC_EXAMPLE}");
            return;
        }
        Some("--input-schema") => {
            print!("{EMBEDDED_INPUT_SCHEMA}");
            return;
        }
        Some("--report-schema") => {
            print!("{EMBEDDED_REPORT_SCHEMA}");
            return;
        }
        _ => {}
    }

    let raw = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!(
                "could not read model outcomes '{}': {error}",
                path.to_string_lossy()
            );
            process::exit(2);
        }
    };
    let root: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid JSON: {error}");
            process::exit(2);
        }
    };
    let (models, cvar_alpha, model_weight_intervals, practical_loss_difference_margin) =
        match parse_input(&root) {
            Ok(parsed) => parsed,
            Err(error) => {
                eprintln!("invalid model outcome input: {error}");
                process::exit(2);
            }
        };
    let scores = match score_model_ensemble(&models, cvar_alpha) {
        Ok(scores) => scores,
        Err(errors) => {
            for error in errors {
                eprintln!("risk scoring rejected input: {error}");
            }
            process::exit(1);
        }
    };

    let mut weight_sensitivity = Vec::new();
    if let Some(intervals) = &model_weight_intervals {
        for score in &scores {
            match score_weight_sensitivity(&models, &score.action, intervals) {
                Ok(result) => weight_sensitivity.push(result),
                Err(errors) => {
                    for error in errors {
                        eprintln!("weight-sensitivity analysis rejected input: {error}");
                    }
                    process::exit(1);
                }
            }
        }
    }

    let action_preferences = match score_action_preferences(
        &models,
        &scores,
        model_weight_intervals.as_deref(),
        practical_loss_difference_margin,
    ) {
        Ok(comparisons) => comparisons,
        Err(errors) => {
            for error in errors {
                eprintln!("action-preference analysis rejected input: {error}");
            }
            process::exit(1);
        }
    };

    let output = build_report(
        &raw,
        &root,
        &models,
        ReportComponents {
            cvar_alpha,
            practical_loss_difference_margin,
            scores: &scores,
            weight_sensitivity: &weight_sensitivity,
            action_preferences: &action_preferences,
        },
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("JSON risk report should serialize")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_committed_two_model_synthetic_example() {
        let input: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("committed example should parse");
        let (models, alpha, _, _practical_margin) =
            parse_input(&input).expect("valid synthetic model set");
        let scores = score_model_ensemble(&models, alpha).expect("valid synthetic losses");
        let graze = scores
            .iter()
            .find(|score| score.action == "graze")
            .expect("graze");
        let wait = scores
            .iter()
            .find(|score| score.action == "wait")
            .expect("wait");
        assert!((graze.expected_loss - 4.8).abs() < 1e-9);
        assert!(!graze.admissible_across_all_models);
        assert_eq!(wait.worst_case_loss, 3.0);
    }

    #[test]
    fn decision_input_schema_matches_example_contract() {
        let example: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON should parse");
        let schema: Value =
            serde_json::from_str(EMBEDDED_INPUT_SCHEMA).expect("input schema should parse");
        assert_eq!(
            example["schema_version"].as_str(),
            schema["properties"]["schema_version"]["const"].as_str()
        );
        assert_eq!(schema["properties"]["models"]["minItems"].as_u64(), Some(1));
        assert_eq!(
            schema["properties"]["practical_loss_difference_margin"]["minimum"].as_f64(),
            Some(0.0)
        );
        assert_eq!(
            schema["properties"]["model_weight_intervals"]["items"]["required"],
            json!(["model_id", "lower", "upper"])
        );
        assert!(
            example["models"]
                .as_array()
                .is_some_and(|models| !models.is_empty())
        );
    }

    #[test]
    fn risk_report_is_bound_to_input_and_matches_report_contract() {
        let root: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON should parse");
        let (models, alpha, intervals, practical_margin) =
            parse_input(&root).expect("input should parse");
        let scores = score_model_ensemble(&models, alpha).expect("scores should be valid");
        let mut weight_sensitivity = Vec::new();
        for score in &scores {
            weight_sensitivity.push(
                score_weight_sensitivity(
                    &models,
                    &score.action,
                    intervals
                        .as_deref()
                        .expect("example declares weight intervals"),
                )
                .expect("valid model-weight intervals"),
            );
        }
        let preferences =
            score_action_preferences(&models, &scores, intervals.as_deref(), practical_margin)
                .expect("valid pairwise model-weight comparisons");
        let report = build_report(
            SYNTHETIC_EXAMPLE,
            &root,
            &models,
            ReportComponents {
                cvar_alpha: alpha,
                practical_loss_difference_margin: practical_margin,
                scores: &scores,
                weight_sensitivity: &weight_sensitivity,
                action_preferences: &preferences,
            },
        );
        let schema: Value =
            serde_json::from_str(EMBEDDED_REPORT_SCHEMA).expect("report schema should parse");
        assert_eq!(
            report["schema_version"].as_str(),
            schema["properties"]["schema_version"]["const"].as_str()
        );
        assert_eq!(
            report["input_sha256"].as_str(),
            Some(sha256_hex(SYNTHETIC_EXAMPLE.as_bytes()).as_str())
        );
        assert_eq!(report["model_count"].as_u64(), Some(2));
        assert_eq!(
            report["practical_loss_difference_margin"].as_f64(),
            Some(practical_margin)
        );
        assert_eq!(report["model_weights"][0]["weight"].as_f64(), Some(0.6));
        assert_eq!(report["model_ids"].as_array().map(Vec::len), Some(2));
        assert_eq!(
            report["scores"].as_array().map(Vec::len),
            root["models"][0]["action_losses"]
                .as_object()
                .map(|actions| actions.len())
        );
        assert_eq!(
            report["weight_sensitivity"].as_array().map(Vec::len),
            Some(3)
        );
        let graze = report["weight_sensitivity"]
            .as_array()
            .expect("weight sensitivity array")
            .iter()
            .find(|item| item["action"] == "graze")
            .expect("graze sensitivity");
        assert!((graze["minimum_expected_loss"].as_f64().expect("minimum") - 3.4).abs() < 1e-9);
        assert!((graze["maximum_expected_loss"].as_f64().expect("maximum") - 6.2).abs() < 1e-9);
        assert_eq!(
            report["action_preference"].as_array().map(Vec::len),
            Some(3)
        );
        let graze_retreat = report["action_preference"]
            .as_array()
            .expect("action preference array")
            .iter()
            .find(|item| item["first_action"] == "graze" && item["second_action"] == "retreat")
            .expect("graze/retreat comparison");
        assert_eq!(
            graze_retreat["stability"].as_str(),
            Some("ranking_can_flip")
        );
        assert!(
            (graze_retreat["minimum_loss_difference"]
                .as_f64()
                .expect("min diff")
                + 1.4)
                .abs()
                < 1e-9
        );
        assert!(
            (graze_retreat["maximum_loss_difference"]
                .as_f64()
                .expect("max diff")
                - 1.8)
                .abs()
                < 1e-9
        );
        let minimum_weights = &graze_retreat["minimizing_weights"];
        let maximum_weights = &graze_retreat["maximizing_weights"];
        assert!(
            (minimum_weights["predator_coupled"]
                .as_f64()
                .expect("weight")
                - 0.8)
                .abs()
                < 1e-9
        );
        assert!((minimum_weights["predator_weak"].as_f64().expect("weight") - 0.2).abs() < 1e-9);
        assert!(
            (maximum_weights["predator_coupled"]
                .as_f64()
                .expect("weight")
                - 0.4)
                .abs()
                < 1e-9
        );
        assert!((maximum_weights["predator_weak"].as_f64().expect("weight") - 0.6).abs() < 1e-9);
        for weights in [minimum_weights, maximum_weights] {
            let sum = weights
                .as_object()
                .expect("witness weight object")
                .values()
                .map(|value| value.as_f64().expect("numeric witness weight"))
                .sum::<f64>();
            assert!((sum - 1.0).abs() < 1e-9);
        }
        assert_eq!(
            graze_retreat["first_admissible_across_all_models"].as_bool(),
            Some(false)
        );
        assert_eq!(
            graze_retreat["second_admissible_across_all_models"].as_bool(),
            Some(true)
        );
        let retreat_wait = report["action_preference"]
            .as_array()
            .expect("action preference array")
            .iter()
            .find(|item| item["first_action"] == "retreat" && item["second_action"] == "wait")
            .expect("retreat/wait comparison");
        assert_eq!(
            retreat_wait["stability"].as_str(),
            Some("second_strictly_better")
        );
        assert_eq!(
            schema["properties"]["weight_sensitivity"]["type"].as_str(),
            Some("array")
        );
        for field in [
            "first_admissible_across_all_models",
            "second_admissible_across_all_models",
            "minimizing_weights",
            "maximizing_weights",
            "stability",
        ] {
            assert!(
                schema["properties"]["action_preference"]["items"]["required"]
                    .as_array()
                    .is_some_and(|required| required
                        .iter()
                        .any(|value| value.as_str() == Some(field))),
                "pairwise schema must require {field}"
            );
        }
    }

    #[test]
    fn sha256_digest_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn rejects_missing_status_or_scenario() {
        let mut input: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON should parse");
        input.as_object_mut().expect("root object").remove("status");
        assert!(
            parse_input(&input)
                .expect_err("missing status must fail")
                .contains("status")
        );

        let mut input: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON should parse");
        input
            .as_object_mut()
            .expect("root object")
            .remove("scenario");
        assert!(
            parse_input(&input)
                .expect_err("missing scenario must fail")
                .contains("scenario")
        );
    }

    #[test]
    fn rejects_negative_practical_loss_difference_margin() {
        let mut input: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON should parse");
        input["practical_loss_difference_margin"] = json!(-0.01);
        assert!(
            parse_input(&input)
                .expect_err("negative practical margin must fail")
                .contains("practical_loss_difference_margin")
        );
    }

    #[test]
    fn rejects_unknown_root_fields() {
        let mut input: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON should parse");
        input["unreviewed_field"] = Value::Bool(true);
        assert!(
            parse_input(&input)
                .expect_err("unknown field must fail")
                .contains("unknown field")
        );
    }

    #[test]
    fn point_weight_mode_remains_available_without_intervals() {
        let mut input: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON should parse");
        input
            .as_object_mut()
            .expect("root object")
            .remove("model_weight_intervals");
        let (_, _, intervals, _) = parse_input(&input).expect("point-weight input remains valid");
        assert!(intervals.is_none());
    }

    #[test]
    fn rejects_unknown_model_weight_interval_fields() {
        let mut input: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON should parse");
        input["model_weight_intervals"][0]["oracle_truth"] = Value::Bool(true);
        assert!(
            parse_input(&input)
                .expect_err("unknown interval field must fail")
                .contains("unknown field")
        );
    }

    #[test]
    fn rejects_unknown_model_fields() {
        let mut input: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON should parse");
        input["models"][0]["oracle_truth"] = Value::String("hidden".to_owned());
        assert!(
            parse_input(&input)
                .expect_err("unknown model field must fail")
                .contains("unknown field")
        );
    }

    #[test]
    fn rejects_wrong_schema_version() {
        let input = json!({
            "schema_version": "wrong",
            "cvar_alpha": 0.9,
            "models": []
        });
        assert!(
            parse_input(&input)
                .expect_err("wrong schema must fail")
                .contains("schema_version")
        );
    }
}
